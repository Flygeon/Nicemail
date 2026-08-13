//! 所有 #[tauri::command]。函数名与 src/mail/api.ts 的 invoke 一一对应。
//! 阻塞的 IMAP/SMTP/DB 操作统一包在 spawn_blocking 中执行。

use std::collections::HashMap;
use std::path::PathBuf;

use tauri::Emitter;

use crate::db::Db;
use crate::error::Error;
use crate::models::*;

fn db_path() -> PathBuf {
    crate::data_dir().join("nicemail.db")
}

/// 在阻塞线程池中执行同步代码,并把 Error 转成 String。
async fn spawn<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce() -> Result<R, Error> + Send + 'static,
    R: Send + 'static,
{
    let r: Result<R, Error> = tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?;
    r.map_err(|e| e.to_string())
}

/// 取账号凭据:oauth2 → 刷新 access token;password → keyring 里的密码。
fn account_secret(account: &AccountConfig) -> Result<String, Error> {
    if account.auth == AuthKind::Oauth2 {
        crate::oauth::access_token_for(account.provider.as_str())
    } else {
        crate::keyring::get_password(&account.id).ok_or_else(|| {
            Error::Config("未找到该账号的密码/授权码,请重新添加账号".into())
        })
    }
}

// ── 账号 ──

#[tauri::command]
pub async fn account_list() -> Result<Vec<AccountConfig>, String> {
    let path = db_path();
    spawn(move || {
        let db = Db::open(&path)?;
        db.list_accounts()
    })
    .await
}

#[tauri::command]
pub async fn account_add(draft: AccountDraft) -> Result<AccountConfig, String> {
    spawn(move || account_add_impl(&draft)).await
}

fn account_add_impl(draft: &AccountDraft) -> Result<AccountConfig, Error> {
    let test = account_test_impl(draft);
    if !test.ok {
        return Err(Error::InvalidInput(test.message));
    }
    let db = Db::open(&db_path())?;
    let id = uuid::Uuid::new_v4().to_string();
    let account = AccountConfig {
        id: id.clone(),
        provider: draft.provider,
        name: draft.name.clone(),
        email: draft.email.clone(),
        imap_host: draft.imap_host.clone(),
        imap_port: draft.imap_port,
        imap_ssl: draft.imap_ssl,
        smtp_host: draft.smtp_host.clone(),
        smtp_port: draft.smtp_port,
        smtp_ssl: draft.smtp_ssl,
        auth: if draft.use_oauth {
            AuthKind::Oauth2
        } else {
            draft.auth
        },
        poll_seconds: 0,
        color: String::new(),
        signature: String::new(),
        last_sync_at: None,
    };
    db.insert_account(&account)?;
    if !draft.use_oauth && !draft.password.is_empty() {
        crate::keyring::set_password(&id, &draft.password)?;
    }
    Ok(account)
}

#[tauri::command]
pub async fn account_update(
    account: AccountConfig,
    password: Option<String>,
) -> Result<AccountConfig, String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        db.update_account(&account)?;
        if let Some(p) = password {
            if !p.is_empty() {
                crate::keyring::set_password(&account.id, &p)?;
            }
        }
        Ok(account)
    })
    .await
}

#[tauri::command]
pub async fn account_delete(id: String) -> Result<(), String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        db.delete_account(&id)?;
        let _ = crate::keyring::delete_password(&id);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn account_test(draft: AccountDraft) -> Result<TestResult, String> {
    spawn(move || Ok(account_test_impl(&draft))).await
}

fn account_test_impl(draft: &AccountDraft) -> TestResult {
    let auth = if draft.use_oauth {
        AuthKind::Oauth2
    } else {
        draft.auth
    };
    let account = AccountConfig {
        id: String::new(),
        provider: draft.provider,
        name: draft.name.clone(),
        email: draft.email.clone(),
        imap_host: draft.imap_host.clone(),
        imap_port: draft.imap_port,
        imap_ssl: draft.imap_ssl,
        smtp_host: draft.smtp_host.clone(),
        smtp_port: draft.smtp_port,
        smtp_ssl: draft.smtp_ssl,
        auth,
        poll_seconds: 0,
        color: String::new(),
        signature: String::new(),
        last_sync_at: None,
    };

    let secret = if auth == AuthKind::Oauth2 {
        match crate::oauth::access_token_for(draft.provider.as_str()) {
            Ok(t) => t,
            Err(e) => {
                return TestResult {
                    ok: false,
                    message: format!("OAuth 未授权: {e}"),
                }
            }
        }
    } else if draft.password.is_empty() {
        return TestResult {
            ok: false,
            message: "密码/授权码不能为空".into(),
        };
    } else {
        draft.password.clone()
    };

    if let Err(e) = crate::imap::test_connection(&account, &secret) {
        return TestResult {
            ok: false,
            message: format!("IMAP 连接失败: {e}"),
        };
    }
    if let Err(e) = crate::smtp::test_connection(&account, &secret) {
        return TestResult {
            ok: false,
            message: format!("SMTP 连接失败: {e}"),
        };
    }
    TestResult {
        ok: true,
        message: "连接成功".into(),
    }
}

// ── OAuth ──

#[tauri::command]
pub async fn oauth_config() -> Result<OAuthConfig, String> {
    Ok(crate::oauth::oauth_config())
}

#[tauri::command]
pub async fn oauth_start(app: tauri::AppHandle, provider: String) -> Result<OAuthStartResponse, String> {
    spawn(move || {
        let auth_url = crate::oauth::start_oauth(app, &provider)?;
        Ok(OAuthStartResponse { auth_url })
    })
    .await
}

#[tauri::command]
pub async fn oauth_finish(
    provider: String,
    code: String,
    state: String,
) -> Result<OAuthFinishResponse, String> {
    spawn(move || {
        let email = crate::oauth::finish(&provider, &code, &state)?;
        Ok(OAuthFinishResponse { email })
    })
    .await
}

// ── 文件夹与邮件 ──

#[tauri::command]
pub async fn mailbox_list(account_id: String) -> Result<Vec<Folder>, String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let secret = account_secret(&account)?;
        let client = crate::imap::connect(&account)?;
        crate::imap::list_folders(client, &account, &secret)
    })
    .await
}

#[tauri::command]
pub async fn mail_sync(
    app: tauri::AppHandle,
    account_id: String,
    folder: String,
) -> Result<SyncResult, String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let secret = account_secret(&account)?;
        let client = crate::imap::connect(&account)?;

        let progress = |processed: i64, total: i64| {
            let _ = app.emit(
                "sync://progress",
                SyncProgress {
                    account_id: account_id.clone(),
                    folder: folder.clone(),
                    processed,
                    total,
                },
            );
        };
        let result = crate::imap::sync_folder(client, &account, &secret, &folder, &db, progress)?;
        let _ = app.emit(
            "sync://done",
            SyncDone {
                account_id: account_id.clone(),
                folder: folder.clone(),
                ok: true,
                message: "同步完成".into(),
            },
        );
        let _ = db.set_last_sync(&account_id);
        let _ = app.emit(
            "mail://changed",
            MailChanged {
                account_id: account_id.clone(),
                folder: folder.clone(),
            },
        );
        Ok(result)
    })
    .await
}

#[tauri::command]
pub async fn mail_list(
    account_id: String,
    folder: String,
    offset: i64,
    limit: i64,
) -> Result<Vec<MessageSummary>, String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        db.list_messages(&account_id, &folder, offset, limit)
    })
    .await
}

#[tauri::command]
pub async fn mail_get(
    account_id: String,
    folder: String,
    uid: u32,
) -> Result<MessageDetail, String> {
    spawn(move || mail_get_impl(&account_id, &folder, uid)).await
}

fn mail_get_impl(account_id: &str, folder: &str, uid: u32) -> Result<MessageDetail, Error> {
    let db = Db::open(&db_path())?;
    let account = db
        .get_account(account_id)?
        .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
    let stored = match db.get_message_by_uid(account_id, folder, uid)? {
        Some(m) if m.raw.is_some() => m,
        _ => {
            let secret = account_secret(&account)?;
            let client = crate::imap::connect(&account)?;
            let (raw, flags) = crate::imap::fetch_message(client, &account, &secret, folder, uid)?;
            let msg = crate::mime::parse_full(account_id, folder, uid, &flags, &raw);
            db.upsert_message(&msg)?;
            msg
        }
    };
    Ok(stored.to_detail())
}

#[tauri::command]
pub async fn mail_set_flag(
    account_id: String,
    folder: String,
    uids: Vec<u32>,
    flag: String,
    value: bool,
) -> Result<(), String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let secret = account_secret(&account)?;
        let client = crate::imap::connect(&account)?;
        crate::imap::set_flags(client, &account, &secret, &folder, &uids, &flag, value, &db)
    })
    .await
}

#[tauri::command]
pub async fn mail_move(
    account_id: String,
    folder: String,
    uids: Vec<u32>,
    dest_folder: String,
) -> Result<(), String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let secret = account_secret(&account)?;
        let client = crate::imap::connect(&account)?;
        crate::imap::move_messages(client, &account, &secret, &folder, &uids, &dest_folder, &db)
    })
    .await
}

#[tauri::command]
pub async fn mail_delete(account_id: String, folder: String, uids: Vec<u32>) -> Result<(), String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let secret = account_secret(&account)?;
        let client = crate::imap::connect(&account)?;
        crate::imap::delete_messages(client, &account, &secret, &folder, &uids, &db)
    })
    .await
}

#[tauri::command]
pub async fn mail_search(
    account_id: String,
    query: String,
    folder: Option<String>,
) -> Result<Vec<MessageSummary>, String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        db.search_messages(&account_id, &query, folder.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn mail_attachment_save(
    account_id: String,
    folder: String,
    uid: u32,
    index: usize,
    dest_path: String,
) -> Result<(), String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let raw = match db
            .get_message_by_uid(&account_id, &folder, uid)?
            .and_then(|m| m.raw)
        {
            Some(raw) => raw,
            None => {
                let secret = account_secret(&account)?;
                let client = crate::imap::connect(&account)?;
                let (raw, flags) =
                    crate::imap::fetch_message(client, &account, &secret, &folder, uid)?;
                let msg = crate::mime::parse_full(&account_id, &folder, uid, &flags, &raw);
                db.upsert_message(&msg)?;
                raw
            }
        };
        let data = crate::mime::extract_attachment(&raw, index)
            .ok_or_else(|| Error::InvalidInput("附件不存在或索引无效".into()))?;
        std::fs::write(&dest_path, data).map_err(Error::Io)?;
        Ok(())
    })
    .await
}

// ── 发送 ──

#[tauri::command]
pub async fn mail_send(request: SendRequest) -> Result<MailSendResponse, String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&request.account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let secret = account_secret(&account)?;
        let message_id = crate::smtp::send(&account, &secret, &request)?;
        Ok(MailSendResponse { message_id })
    })
    .await
}

#[tauri::command]
pub async fn mail_save_draft(account_id: String, request: SendRequest) -> Result<(), String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        let account = db
            .get_account(&account_id)?
            .ok_or_else(|| Error::InvalidInput("账号不存在".into()))?;
        let raw = crate::smtp::build_draft_raw(&account, &request)?;
        let secret = account_secret(&account)?;
        let client = crate::imap::connect(&account)?;
        crate::imap::append_draft(client, &account, &secret, &raw)
    })
    .await
}

// ── 设置 ──

#[tauri::command]
pub async fn settings_get() -> Result<HashMap<String, String>, String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        db.settings_get_all()
    })
    .await
}

#[tauri::command]
pub async fn settings_set(key: String, value: String) -> Result<(), String> {
    spawn(move || {
        let db = Db::open(&db_path())?;
        db.settings_set(&key, &value)
    })
    .await
}
