//! 与前端 src/mail/api.ts 一一对应的 serde 结构体。
//! 字段名采用 snake_case + #[serde(rename_all = "camelCase")],
//! 序列化后正好与 TS 类型的 camelCase 字段名匹配。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 邮件服务商。序列化值与 TS `Provider` 联合类型一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    #[serde(rename = "163")]
    _163,
    #[serde(rename = "126")]
    _126,
    #[serde(rename = "qq")]
    Qq,
    #[serde(rename = "gmail")]
    Gmail,
    #[serde(rename = "outlook")]
    Outlook,
    #[serde(rename = "custom")]
    Custom,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::_163 => "163",
            Provider::_126 => "126",
            Provider::Qq => "qq",
            Provider::Gmail => "gmail",
            Provider::Outlook => "outlook",
            Provider::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Provider {
        match s {
            "163" => Provider::_163,
            "126" => Provider::_126,
            "qq" => Provider::Qq,
            "gmail" => Provider::Gmail,
            "outlook" => Provider::Outlook,
            _ => Provider::Custom,
        }
    }
}

/// 认证方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthKind {
    Password,
    Oauth2,
}

impl AuthKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthKind::Password => "password",
            AuthKind::Oauth2 => "oauth2",
        }
    }

    pub fn from_str(s: &str) -> AuthKind {
        match s {
            "oauth2" => AuthKind::Oauth2,
            _ => AuthKind::Password,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountConfig {
    pub id: String,
    pub provider: Provider,
    pub name: String,
    pub email: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_ssl: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_ssl: bool,
    pub auth: AuthKind,
    pub poll_seconds: i64,
    pub color: String,
    pub signature: String,
    pub last_sync_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDraft {
    pub provider: Provider,
    pub name: String,
    pub email: String,
    pub auth: AuthKind,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_ssl: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_ssl: bool,
    pub password: String,
    pub use_oauth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub full_name: String,
    pub name: String,
    pub delimiter: String,
    pub flags: Vec<String>,
    pub selectable: bool,
    pub unread_count: i64,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSummary {
    pub id: i64,
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub flags: Vec<String>,
    pub subject: String,
    pub from_name: String,
    pub from_email: String,
    pub to_emails: Vec<String>,
    pub date: i64,
    pub has_attachments: bool,
    pub preview: String,
    pub unread: bool,
    pub starred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
    pub index: usize,
    pub filename: String,
    pub mime: String,
    pub size: usize,
    pub content_id: Option<String>,
    pub is_inline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDetail {
    pub id: i64,
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub subject: String,
    pub from_name: String,
    pub from_email: String,
    pub to_emails: Vec<String>,
    pub cc_emails: Vec<String>,
    pub date: i64,
    pub flags: Vec<String>,
    pub html: Option<String>,
    pub text: Option<String>,
    pub attachments: Vec<AttachmentMeta>,
    pub embedded: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAttachment {
    pub path: String,
    pub filename: String,
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub account_id: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub attachments: Vec<SendAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub account_id: String,
    pub folder: String,
    pub added: i64,
    pub updated: i64,
    pub removed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderConfig {
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConfig {
    pub google: OAuthProviderConfig,
    pub outlook: OAuthProviderConfig,
}

// ── 事件负载 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub account_id: String,
    pub folder: String,
    pub processed: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDone {
    pub account_id: String,
    pub folder: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailChanged {
    pub account_id: String,
    pub folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthReady {
    pub provider: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthError {
    pub provider: String,
    pub message: String,
}

// ── 命令返回值(api.ts 中内联的返回对象) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStartResponse {
    pub auth_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFinishResponse {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailSendResponse {
    pub message_id: String,
}

// ── 内部存储模型(不入库为契约,仅内部使用) ──

#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub id: Option<i64>,
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub flags: Vec<String>,
    pub subject: String,
    pub from_name: String,
    pub from_email: String,
    pub to_emails: Vec<String>,
    pub cc_emails: Vec<String>,
    pub date: i64,
    pub has_attachments: bool,
    pub preview: String,
    pub html: Option<String>,
    pub text: Option<String>,
    pub attachments: Vec<AttachmentMeta>,
    pub embedded: HashMap<String, String>,
    pub raw: Option<Vec<u8>>,
}

impl StoredMessage {
    pub fn to_summary(&self) -> MessageSummary {
        let unread = !self.flags.iter().any(|f| f.eq_ignore_ascii_case("\\seen"));
        let starred = self.flags.iter().any(|f| f.eq_ignore_ascii_case("\\flagged"));
        MessageSummary {
            id: self.id.unwrap_or(0),
            account_id: self.account_id.clone(),
            folder: self.folder.clone(),
            uid: self.uid,
            flags: self.flags.clone(),
            subject: self.subject.clone(),
            from_name: self.from_name.clone(),
            from_email: self.from_email.clone(),
            to_emails: self.to_emails.clone(),
            date: self.date,
            has_attachments: self.has_attachments,
            preview: self.preview.clone(),
            unread,
            starred,
        }
    }

    pub fn to_detail(&self) -> MessageDetail {
        MessageDetail {
            id: self.id.unwrap_or(0),
            account_id: self.account_id.clone(),
            folder: self.folder.clone(),
            uid: self.uid,
            subject: self.subject.clone(),
            from_name: self.from_name.clone(),
            from_email: self.from_email.clone(),
            to_emails: self.to_emails.clone(),
            cc_emails: self.cc_emails.clone(),
            date: self.date,
            flags: self.flags.clone(),
            html: self.html.clone(),
            text: self.text.clone(),
            attachments: self.attachments.clone(),
            embedded: self.embedded.clone(),
        }
    }
}
