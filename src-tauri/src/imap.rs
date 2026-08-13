//! 同步 IMAP 客户端(imap 2.x + native-tls)。
//! 短连接模型:每个操作 connect → login → 操作 → logout。
//! 阻塞式 API,由 commands 层用 spawn_blocking 包裹。

use std::io::{Read, Write};
use std::net::{ToSocketAddrs, TcpStream};
use std::time::Duration;

use base64::{engine::general_purpose, Engine as _};
use imap::types::{NameAttribute, StatusAttribute, UnsolicitedResponse};
use imap::{Client, Session};
use native_tls::{TlsConnector, TlsStream};

use crate::db::Db;
use crate::error::Error;
use crate::mime;
use crate::models::{AccountConfig, AuthKind, Folder, SyncResult};

/// 连接结果:TLS 或明文,统一后续处理。
pub enum ImapClient {
    Tls(Client<TlsStream<TcpStream>>),
    Plain(Client<TcpStream>),
}

/// XOAUTH2 认证器:SASL 串为 `user=<email>\x01auth=Bearer <token>\x01\x01`。
pub struct XOAuth2 {
    pub user: String,
    pub access_token: String,
}

impl imap::Authenticator for XOAuth2 {
    type Response = String;
    fn process(&self, _challenge: &[u8]) -> String {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

/// 建立连接。ssl=true 直连 TLS;ssl=false 尝试 STARTTLS,失败则回退明文。
/// 解析主机名 + 带超时的 TCP 连接,并设置读写超时。
fn connect_tcp(host: &str, port: u16) -> Result<TcpStream, Error> {
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(Error::Io)?
        .next()
        .ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("无法解析主机名 {host}"),
            ))
        })?;
    let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(20))?;
    tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(30)))?;
    Ok(tcp)
}

/// 逐字节读一行(避免 BufReader 过度缓冲,导致 crate 接手连接时丢掉多余字节)。
fn read_line_bytes<R: Read>(r: &mut R) -> Result<Vec<u8>, Error> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match r.read(&mut byte) {
            Ok(0) => return Ok(line), // EOF
            Ok(_) => {
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(line);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Error::Io(e)),
        }
    }
}

/// 手动完成:读 greeting + 发送 ID 命令。
/// imap_proto 解析不了 163.com 等服务器的 `* ID NIL` 未标记响应,若用 crate 的
/// run_command_and_check_ok 发 ID,会在 tag 断言处 panic 并把连接搞错位。
/// 因此必须在 crate 接管连接之前手动把 greeting 和 ID 做完。
fn handshake_with_id<T: Read + Write>(mut stream: T) -> Result<T, Error> {
    // 读 greeting
    let _greeting = read_line_bytes(&mut stream)?;
    // 发 ID(非认证状态即可;163 要求 SELECT 前发过 ID)
    let cmd = b"a000 ID (\"name\" \"Nicemail\" \"version\" \"0.1.0\" \"vendor\" \"Nicemail\")\r\n";
    stream.write_all(cmd).map_err(Error::Io)?;
    // 读到 tagged done(a000 OK/NO)为止,吞掉 `* ID ...` 未标记行
    loop {
        let line = read_line_bytes(&mut stream)?;
        if line.is_empty() {
            break;
        }
        if line.starts_with(b"a000 ") {
            break;
        }
    }
    Ok(stream)
}

pub fn connect(account: &AccountConfig) -> Result<ImapClient, Error> {
    let host = account.imap_host.clone();
    let port = account.imap_port;
    let tls = TlsConnector::builder().build()?;
    let tcp = connect_tcp(&host, port)?;

    if account.imap_ssl {
        let stream = tls
            .connect(&host, tcp)
            .map_err(|e| match e {
                native_tls::HandshakeError::Failure(err) => Error::Tls(err),
                native_tls::HandshakeError::WouldBlock(_) => Error::Io(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "TLS 握手被中断(WouldBlock)",
                )),
            })?;
        let stream = handshake_with_id(stream)?;
        // greeting 已手动读,直接交给 crate(不再 read_greeting)
        Ok(ImapClient::Tls(Client::new(stream)))
    } else {
        // 手动 greeting + ID 后,尝试 STARTTLS;失败则回退明文重连
        let stream = handshake_with_id(tcp)?;
        let client = Client::new(stream);
        match client.secure(&host, &tls) {
            Ok(sec) => Ok(ImapClient::Tls(sec)),
            Err(_) => {
                let tcp2 = connect_tcp(&host, port)?;
                let stream2 = handshake_with_id(tcp2)?;
                Ok(ImapClient::Plain(Client::new(stream2)))
            }
        }
    }
}

/// 登录。password 模式用 LOGIN;oauth2 模式用 XOAUTH2(自动刷新 access token)。
pub fn login<T: Read + Write>(
    client: Client<T>,
    account: &AccountConfig,
    secret: &str,
) -> Result<Session<T>, Error> {
    if account.auth == AuthKind::Oauth2 {
        let token = crate::oauth::access_token_for(account.provider.as_str())?;
        let auth = XOAuth2 {
            user: account.email.clone(),
            access_token: token,
        };
        client
            .authenticate("XOAUTH2", &auth)
            .map_err(|(e, _)| Error::Imap(e))
    } else {
        client
            .login(&account.email, secret)
            .map_err(|(e, _)| Error::Imap(e))
    }
}

/// 统一分发:登录后把 Session 交给 `$f`,操作完 logout。
/// 注意:`$f` 只能借用外部变量,不能 move(宏会把同一段表达式展开到两个分支)。
macro_rules! dispatch {
    ($client:expr, $account:expr, $secret:expr, $f:expr) => {{
        match $client {
            ImapClient::Tls(c) => {
                let mut session = login(c, $account, $secret)?;
                let res = ($f)(&mut session);
                let _ = session.logout();
                res
            }
            ImapClient::Plain(c) => {
                let mut session = login(c, $account, $secret)?;
                let res = ($f)(&mut session);
                let _ = session.logout();
                res
            }
        }
    }};
}

/// 连接测试(仅登录)。
pub fn test_connection(account: &AccountConfig, secret: &str) -> Result<(), Error> {
    let client = connect(account)?;
    dispatch!(client, account, secret, |_session| Ok(()))
}

/// 列出文件夹。
pub fn list_folders(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
) -> Result<Vec<Folder>, Error> {
    dispatch!(client, account, secret, |session| list_folders_session(session))
}

fn list_folders_session<T: Read + Write>(session: &mut Session<T>) -> Result<Vec<Folder>, Error> {
    let names = session.list(None, Some("*")).map_err(Error::Imap)?;
    let mut folders = Vec::new();
    for name in names.iter() {
        let full_name = name.name().to_string();
        let delimiter = name.delimiter().unwrap_or("").to_string();
        let folder_name = if delimiter.is_empty() {
            full_name.clone()
        } else {
            full_name
                .rsplit(delimiter.as_str())
                .next()
                .unwrap_or(&full_name)
                .to_string()
        };
        // IMAP 非 ASCII 文件夹名用 modified-UTF-7 编码,解码成可读中文(如"收件箱")
        let display_name = decode_utf7(&folder_name);
        let flags: Vec<String> = name.attributes().iter().map(attr_to_string).collect();
        let selectable = !name
            .attributes()
            .iter()
            .any(|a| matches!(a, NameAttribute::NoSelect));
        let (total, unread) = if selectable {
            folder_counts(session, &full_name)
        } else {
            (0, 0)
        };
        folders.push(Folder {
            full_name,
            name: display_name,
            delimiter,
            flags,
            selectable,
            unread_count: unread,
            total_count: total,
        });
    }
    Ok(folders)
}

/// 增量同步一个文件夹。
pub fn sync_folder<F>(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
    folder: &str,
    db: &Db,
    progress: F,
) -> Result<SyncResult, Error>
where
    F: Fn(i64, i64),
{
    dispatch!(client, account, secret, |session| sync_session(
        session,
        account,
        folder,
        db,
        &progress,
    ))
}

fn sync_session<T, F>(
    session: &mut Session<T>,
    account: &AccountConfig,
    folder: &str,
    db: &Db,
    progress: &F,
) -> Result<SyncResult, Error>
where
    T: Read + Write,
    F: Fn(i64, i64),
{
    let mailbox = session.select(folder).map_err(Error::Imap)?;
    let uid_validity = mailbox.uid_validity.unwrap_or(0) as i64;

    // UIDVALIDITY 变化 → 清空重同步
    let stored_validity = db.get_uid_validity(&account.id, folder)?;
    let mut removed = 0i64;
    if stored_validity != 0 && stored_validity != uid_validity {
        removed = db.count_messages(&account.id, folder)?;
        db.delete_folder_messages(&account.id, folder)?;
    }
    db.set_uid_validity(&account.id, folder, uid_validity)?;

    let last_uid = db.max_uid(&account.id, folder)?.unwrap_or(0);
    let start = last_uid + 1;
    let mut added = 0i64;
    let mut updated = 0i64;
    let mut new_uids: Vec<u32> = Vec::new();

    if mailbox.exists > 0 {
        let set = format!("{start}:*");
        let query = "(UID FLAGS BODY.PEEK[HEADER.FIELDS (FROM TO CC SUBJECT DATE MESSAGE-ID)])";
        // 头部拉取失败必须显式报错,否则同步"成功"却 0 封邮件,前端无感知
        let fetches = session.uid_fetch(&set, query).map_err(Error::Imap)?;
        let total = fetches.len() as i64;
        for (i, f) in fetches.iter().enumerate() {
            if let Some(uid) = f.uid {
                let flags: Vec<String> = f.flags().iter().map(|x| x.to_string()).collect();
                let header = f.header().unwrap_or_default();
                let msg = mime::parse_header(&account.id, folder, uid, &flags, header);
                let existed = db.get_message_by_uid(&account.id, folder, uid)?.is_some();
                db.upsert_message(&msg)?;
                if existed {
                    updated += 1;
                } else {
                    added += 1;
                }
                new_uids.push(uid);
            }
            progress(i as i64 + 1, total);
        }
    }

    // 对最新 15 封拉取正文(截断 3MB),以生成预览;避免大附件卡住同步
    new_uids.sort_unstable_by(|a, b| b.cmp(a));
    let top: Vec<String> = new_uids.iter().take(15).map(|u| u.to_string()).collect();
    if !top.is_empty() {
        let set = top.join(",");
        if let Ok(fetches) = session.uid_fetch(&set, "(UID FLAGS BODY.PEEK[]<0.3000000>)") {
            for f in fetches.iter() {
                if let Some(uid) = f.uid {
                    if let Some(body) = f.body() {
                        let flags: Vec<String> = f.flags().iter().map(|x| x.to_string()).collect();
                        let msg = mime::parse_full(&account.id, folder, uid, &flags, body);
                        let _ = db.upsert_message(&msg);
                    }
                }
            }
        }
    }

    Ok(SyncResult {
        account_id: account.id.clone(),
        folder: folder.to_string(),
        added,
        updated,
        removed,
    })
}

/// 拉取单封邮件的原文与 flags。
pub fn fetch_message(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
    folder: &str,
    uid: u32,
) -> Result<(Vec<u8>, Vec<String>), Error> {
    dispatch!(client, account, secret, |session| fetch_message_session(
        session, folder, uid,
    ))
}

fn fetch_message_session<T: Read + Write>(
    session: &mut Session<T>,
    folder: &str,
    uid: u32,
) -> Result<(Vec<u8>, Vec<String>), Error> {
    session.select(folder).map_err(Error::Imap)?;
    // 只拉前 3MB:正文/文字通常在前面,避免大附件邮件整封加载导致内存暴涨/卡死。
    // 附件下载用 fetch_message_full 全量拉取。
    let fetches = session
        .uid_fetch(&uid.to_string(), "(UID FLAGS BODY.PEEK[]<0.3000000>)")
        .map_err(Error::Imap)?;
    let fetch = fetches
        .iter()
        .find(|f| f.uid == Some(uid))
        .ok_or_else(|| Error::Imap(imap::Error::No("消息不存在".into())))?;
    let body = fetch
        .body()
        .ok_or_else(|| Error::Imap(imap::Error::No("消息体为空".into())))?;
    let flags: Vec<String> = fetch.flags().iter().map(|f| f.to_string()).collect();
    Ok((body.to_vec(), flags))
}

/// 附件下载用:全量拉取整封邮件(可能很大,仅在用户显式下载时调用)。
pub fn fetch_message_full(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
    folder: &str,
    uid: u32,
) -> Result<(Vec<u8>, Vec<String>), Error> {
    dispatch!(client, account, secret, |session| -> Result<(Vec<u8>, Vec<String>), Error> {
        session.select(folder).map_err(Error::Imap)?;
        let fetches = session
            .uid_fetch(&uid.to_string(), "(UID FLAGS BODY.PEEK[])")
            .map_err(Error::Imap)?;
        let fetch = fetches
            .iter()
            .find(|f| f.uid == Some(uid))
            .ok_or_else(|| Error::Imap(imap::Error::No("消息不存在".into())))?;
        let body = fetch
            .body()
            .ok_or_else(|| Error::Imap(imap::Error::No("消息体为空".into())))?;
        let flags: Vec<String> = fetch.flags().iter().map(|f| f.to_string()).collect();
        Ok((body.to_vec(), flags))
    })
}

/// 设置 flags(+/-)。同时更新本地 DB。
pub fn set_flags(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
    folder: &str,
    uids: &[u32],
    flag: &str,
    value: bool,
    db: &Db,
) -> Result<(), Error> {
    let account_id = account.id.clone();
    dispatch!(client, account, secret, |session| set_flags_session(
        session,
        &account_id,
        folder,
        uids,
        flag,
        value,
        db,
    ))
}

fn set_flags_session<T: Read + Write>(
    session: &mut Session<T>,
    account_id: &str,
    folder: &str,
    uids: &[u32],
    flag: &str,
    value: bool,
    db: &Db,
) -> Result<(), Error> {
    session.select(folder).map_err(Error::Imap)?;
    let set = uids_to_set(uids);
    let imap_flag = normalize_flag(flag);
    let query = if value {
        format!("+FLAGS.SILENT ({imap_flag})")
    } else {
        format!("-FLAGS.SILENT ({imap_flag})")
    };
    let _ = session.uid_store(&set, &query).map_err(Error::Imap)?;

    for &uid in uids {
        let mut flags = db.get_flags(account_id, folder, uid)?;
        if value {
            if !flags.iter().any(|f| f.eq_ignore_ascii_case(&imap_flag)) {
                flags.push(imap_flag.clone());
            }
        } else {
            flags.retain(|f| !f.eq_ignore_ascii_case(&imap_flag));
        }
        db.update_flags(account_id, folder, uid, flags)?;
    }
    Ok(())
}

/// 移动邮件。优先 UID MOVE,不支持则 COPY + STORE \Deleted + EXPUNGE。
pub fn move_messages(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
    folder: &str,
    uids: &[u32],
    dest: &str,
    db: &Db,
) -> Result<(), Error> {
    let account_id = account.id.clone();
    dispatch!(client, account, secret, |session| move_session(
        session,
        &account_id,
        folder,
        uids,
        dest,
        db,
    ))
}

fn move_session<T: Read + Write>(
    session: &mut Session<T>,
    account_id: &str,
    folder: &str,
    uids: &[u32],
    dest: &str,
    db: &Db,
) -> Result<(), Error> {
    session.select(folder).map_err(Error::Imap)?;
    let set = uids_to_set(uids);
    match session.uid_mv(&set, dest) {
        Ok(()) => {}
        Err(_) => {
            session.uid_copy(&set, dest).map_err(Error::Imap)?;
            session
                .uid_store(&set, "+FLAGS (\\Deleted)")
                .map_err(Error::Imap)?;
            session.expunge().map_err(Error::Imap)?;
        }
    }
    db.delete_uids(account_id, folder, uids)?;
    Ok(())
}

/// 删除邮件:移到 Trash(找不到则 \Deleted + EXPUNGE)。
pub fn delete_messages(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
    folder: &str,
    uids: &[u32],
    db: &Db,
) -> Result<(), Error> {
    let account_id = account.id.clone();
    dispatch!(client, account, secret, |session| delete_session(
        session,
        &account_id,
        folder,
        uids,
        db,
    ))
}

fn delete_session<T: Read + Write>(
    session: &mut Session<T>,
    account_id: &str,
    folder: &str,
    uids: &[u32],
    db: &Db,
) -> Result<(), Error> {
    session.select(folder).map_err(Error::Imap)?;
    let set = uids_to_set(uids);
    if let Some(trash) = find_folder(session, "\\Trash") {
        match session.uid_mv(&set, &trash) {
            Ok(()) => {}
            Err(_) => {
                session.uid_copy(&set, &trash).map_err(Error::Imap)?;
                session
                    .uid_store(&set, "+FLAGS (\\Deleted)")
                    .map_err(Error::Imap)?;
                session.expunge().map_err(Error::Imap)?;
            }
        }
    } else {
        session
            .uid_store(&set, "+FLAGS (\\Deleted)")
            .map_err(Error::Imap)?;
        session.expunge().map_err(Error::Imap)?;
    }
    db.delete_uids(account_id, folder, uids)?;
    Ok(())
}

/// 把草稿原文 APPEND 到 \Drafts 文件夹(找不到则 INBOX)。
pub fn append_draft(
    client: ImapClient,
    account: &AccountConfig,
    secret: &str,
    raw: &[u8],
) -> Result<(), Error> {
    dispatch!(client, account, secret, |session| append_draft_session(
        session, raw,
    ))
}

fn append_draft_session<T: Read + Write>(session: &mut Session<T>, raw: &[u8]) -> Result<(), Error> {
    let drafts = find_folder(session, "\\Drafts");
    let folder = drafts.unwrap_or_else(|| "INBOX".to_string());
    session.append(&folder, raw).map_err(Error::Imap)?;
    Ok(())
}

// ── 内部辅助 ──

fn folder_counts<T: Read + Write>(session: &mut Session<T>, folder: &str) -> (i64, i64) {
    let mut total = 0i64;
    let mut unread = 0i64;
    if let Ok(_mb) = session.status(folder, "(MESSAGES UNSEEN)") {
        while let Ok(resp) = session.unsolicited_responses.try_recv() {
            if let UnsolicitedResponse::Status { attributes, .. } = resp {
                for attr in attributes {
                    match attr {
                        StatusAttribute::Messages(n) => total = n as i64,
                        StatusAttribute::Unseen(n) => unread = n as i64,
                        _ => {}
                    }
                }
            }
        }
    }
    (total, unread)
}

fn find_folder<T: Read + Write>(session: &mut Session<T>, flag: &str) -> Option<String> {
    if let Ok(names) = session.list(None, Some("*")) {
        for n in names.iter() {
            if n.attributes().iter().any(|a| attr_has_flag(a, flag)) {
                return Some(n.name().to_string());
            }
        }
    }
    None
}

fn attr_has_flag(a: &NameAttribute, flag: &str) -> bool {
    match a {
        NameAttribute::Custom(s) => s.eq_ignore_ascii_case(flag),
        _ => false,
    }
}

fn attr_to_string(a: &NameAttribute) -> String {
    match a {
        NameAttribute::NoInferiors => "\\NoInferiors".to_string(),
        NameAttribute::NoSelect => "\\Noselect".to_string(),
        NameAttribute::Marked => "\\Marked".to_string(),
        NameAttribute::Unmarked => "\\Unmarked".to_string(),
        NameAttribute::Custom(s) => s.to_string(),
    }
}

/// 解码 IMAP modified-UTF-7 文件夹名(中文文件夹在协议层是 `&xxxx-` 形式)。
fn decode_utf7(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c != '&' {
            out.push(c);
            i += 1;
            continue;
        }
        // '&' 开头,收集到 '-' 或下一个 '&'
        i += 1;
        let mut b64 = String::new();
        while i < chars.len() {
            let nc = chars[i];
            if nc == '&' {
                break;
            }
            i += 1;
            if nc == '-' {
                break;
            }
            b64.push(nc);
        }
        if b64.is_empty() {
            // "&-" 表示字面 '&'
            out.push('&');
        } else {
            // modified base64:',' 代替 '/',无填充 → 归一化后用 STANDARD 解
            let normalized = b64.replace(',', "/");
            let padded = match normalized.len() % 4 {
                2 => format!("{normalized}=="),
                3 => format!("{normalized}="),
                _ => normalized,
            };
            if let Ok(bytes) = general_purpose::STANDARD.decode(padded) {
                let mut decoded = String::new();
                for chunk in bytes.chunks_exact(2) {
                    let u = u16::from_be_bytes([chunk[0], chunk[1]]);
                    if let Some(ch) = char::from_u32(u as u32) {
                        decoded.push(ch);
                    }
                }
                out.push_str(&decoded);
            } else {
                // 解码失败回退原文
                out.push_str(&b64);
            }
        }
    }
    out
}

fn normalize_flag(flag: &str) -> String {
    if flag.starts_with('\\') {
        flag.to_string()
    } else {
        match flag.to_lowercase().as_str() {
            "seen" => "\\Seen".to_string(),
            "flagged" => "\\Flagged".to_string(),
            "answered" => "\\Answered".to_string(),
            "deleted" => "\\Deleted".to_string(),
            "draft" => "\\Draft".to_string(),
            _ => format!("\\{flag}"),
        }
    }
}

fn uids_to_set(uids: &[u32]) -> String {
    uids.iter()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
