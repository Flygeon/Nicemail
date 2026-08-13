//! lettre SMTP 发送。
//! 465 端口 → 立即 TLS;587 → STARTTLS;其它端口且 smtpSsl=false → 明文。
//! password 模式用 LOGIN;oauth2 模式用 XOAUTH2(secret 传 access token)。

use std::time::Duration;

use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, Message, MultiPart};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::SmtpTransport;
use lettre::Transport;

use crate::error::Error;
use crate::models::{AccountConfig, AuthKind, SendRequest};

/// 构建邮件并发送,返回 Message-ID。
pub fn send(
    account: &AccountConfig,
    secret: &str,
    request: &SendRequest,
) -> Result<String, Error> {
    let msg = build_message(account, request)?;
    let msg_id = extract_message_id(&msg, account);
    let transport = build_transport(account, secret)?;
    transport.send(&msg).map_err(Error::Smtp)?;
    Ok(msg_id)
}

/// 构建草稿原文(不发送),用于 APPEND 到服务器草稿箱。
pub fn build_draft_raw(account: &AccountConfig, request: &SendRequest) -> Result<Vec<u8>, Error> {
    let msg = build_message(account, request)?;
    Ok(msg.formatted())
}

/// 连接测试(只建链 + 认证,不发送)。
pub fn test_connection(account: &AccountConfig, secret: &str) -> Result<(), Error> {
    let transport = build_transport(account, secret)?;
    transport.test_connection().map_err(Error::Smtp)?;
    Ok(())
}

fn build_message(account: &AccountConfig, request: &SendRequest) -> Result<Message, Error> {
    let sender = Mailbox::new(
        Some(account.name.clone()),
        account
            .email
            .parse::<lettre::Address>()
            .map_err(|e| Error::InvalidInput(format!("发件人邮箱无效 '{}': {}", account.email, e)))?,
    );
    let mut builder = Message::builder().from(sender).subject(request.subject.clone());
    for to in &request.to {
        builder = builder.to(parse_mailbox(to)?);
    }
    for cc in &request.cc {
        builder = builder.cc(parse_mailbox(cc)?);
    }
    for bcc in &request.bcc {
        builder = builder.bcc(parse_mailbox(bcc)?);
    }

    let alt = MultiPart::alternative_plain_html(request.body_text.clone(), request.body_html.clone());
    // MultiPart::mixed() 返回 MultiPartBuilder,multipart(alt) 一步转成 MultiPart,
    // 之后循环里的 singlepart 用的就是 MultiPart::singlepart(返回 Self)。
    let mut mixed = MultiPart::mixed().multipart(alt);

    for att in &request.attachments {
        let data = std::fs::read(&att.path).map_err(Error::Io)?;
        let mime = att
            .mime
            .clone()
            .unwrap_or_else(|| guess_mime(&att.filename));
        let ct = ContentType::parse(&mime).unwrap_or(ContentType::TEXT_PLAIN);
        mixed = mixed.singlepart(Attachment::new(att.filename.clone()).body(data, ct));
    }

    builder
        .multipart(mixed)
        .map_err(|e| Error::Mime(e.to_string()))
}

fn build_transport(account: &AccountConfig, secret: &str) -> Result<SmtpTransport, Error> {
    let host = account.smtp_host.clone();
    let port = account.smtp_port;
    let mut builder = SmtpTransport::builder_dangerous(host.clone())
        .port(port)
        // 关键:lettre 默认无连接超时,服务器接受 TCP 但不回 greeting 时会无限挂起,
        // 导致 account_test/发送永远不返回、前端按钮卡死。给 25s 超时。
        .timeout(Some(Duration::from_secs(25)));

    if port == 465 {
        let params = TlsParameters::new(host.clone()).map_err(Error::Smtp)?;
        builder = builder.tls(Tls::Wrapper(params));
    } else if port == 587 || account.smtp_ssl {
        let params = TlsParameters::new(host.clone()).map_err(Error::Smtp)?;
        builder = builder.tls(Tls::Required(params));
    } else {
        builder = builder.tls(Tls::None);
    }

    let mechanisms = if account.auth == AuthKind::Oauth2 {
        vec![Mechanism::Xoauth2]
    } else {
        vec![Mechanism::Login]
    };
    builder = builder
        .credentials(Credentials::new(account.email.clone(), secret.to_string()))
        .authentication(mechanisms);
    Ok(builder.build())
}

fn parse_mailbox(s: &str) -> Result<Mailbox, Error> {
    s.parse::<Mailbox>()
        .map_err(|e| Error::InvalidInput(format!("收件人地址无效 '{s}': {e}")))
}

fn extract_message_id(msg: &Message, account: &AccountConfig) -> String {
    if let Some(id) = msg.headers().get::<lettre::message::header::MessageId>() {
        id.as_ref().to_string()
    } else {
        let domain = account
            .email
            .split('@')
            .nth(1)
            .unwrap_or("nicemail.app");
        format!("<{}@{}>", uuid::Uuid::new_v4().simple(), domain)
    }
}

fn guess_mime(filename: &str) -> String {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "json" => "application/json",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "md" => "text/markdown",
        "ics" => "text/calendar",
        _ => "application/octet-stream",
    }
    .to_string()
}
