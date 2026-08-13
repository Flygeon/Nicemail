//! rusqlite 数据库连接管理与 CRUD。
//! 数据库位于 app data dir 下的 nicemail.db。
//! 用 PRAGMA user_version 做迁移,当前版本 1。

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Error;
use crate::models::{
    AccountConfig, AuthKind, MessageSummary, Provider, StoredMessage,
};

const MESSAGE_COLUMNS: &str = "id, account_id, folder, uid, flags, subject, from_name, \
     from_email, to_emails, cc_emails, date, has_attachments, preview, html, text, \
     attachments_json, embedded_json, raw";

pub struct Db {
    pub conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        let db = Db { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), Error> {
        let v: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if v < 1 {
            self.conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS accounts (
                    id TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    name TEXT NOT NULL DEFAULT '',
                    email TEXT NOT NULL,
                    imap_host TEXT NOT NULL,
                    imap_port INTEGER NOT NULL,
                    imap_ssl INTEGER NOT NULL DEFAULT 1,
                    smtp_host TEXT NOT NULL,
                    smtp_port INTEGER NOT NULL,
                    smtp_ssl INTEGER NOT NULL DEFAULT 1,
                    auth TEXT NOT NULL DEFAULT 'password',
                    poll_seconds INTEGER NOT NULL DEFAULT 0,
                    color TEXT NOT NULL DEFAULT '',
                    signature TEXT NOT NULL DEFAULT '',
                    last_sync_at INTEGER
                );

                CREATE TABLE IF NOT EXISTS messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    account_id TEXT NOT NULL,
                    folder TEXT NOT NULL,
                    uid INTEGER NOT NULL,
                    flags TEXT NOT NULL DEFAULT '[]',
                    subject TEXT NOT NULL DEFAULT '',
                    from_name TEXT NOT NULL DEFAULT '',
                    from_email TEXT NOT NULL DEFAULT '',
                    to_emails TEXT NOT NULL DEFAULT '[]',
                    cc_emails TEXT NOT NULL DEFAULT '[]',
                    date INTEGER NOT NULL DEFAULT 0,
                    has_attachments INTEGER NOT NULL DEFAULT 0,
                    preview TEXT NOT NULL DEFAULT '',
                    html TEXT,
                    text TEXT,
                    attachments_json TEXT,
                    embedded_json TEXT,
                    raw BLOB,
                    UNIQUE(account_id, folder, uid)
                );

                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL DEFAULT ''
                );

                CREATE TABLE IF NOT EXISTS sync_state (
                    account_id TEXT NOT NULL,
                    folder TEXT NOT NULL,
                    uid_validity INTEGER NOT NULL DEFAULT 0,
                    last_sync INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY(account_id, folder)
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                    subject, from_email, to_emails, body, message_id UNINDEXED
                );

                CREATE INDEX IF NOT EXISTS idx_messages_acct_folder ON messages(account_id, folder, date DESC);
                CREATE INDEX IF NOT EXISTS idx_messages_acct_folder_uid ON messages(account_id, folder, uid);

                PRAGMA user_version = 1;
                "#,
            )?;
        }
        if v < 2 {
            // v2:secrets 表 —— 密码/授权码的本地回退存储。
            // 打包环境下系统 keyring 可能不可靠(凭据写不入 Windows 凭据管理器),
            // 因此同时落一份到本地 SQLite,account_secret 先查 keyring 再回退 DB。
            self.conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS secrets (
                    account_id TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                PRAGMA user_version = 2;
                "#,
            )?;
        }
        Ok(())
    }

    // ── 秘密(密码/授权码,keyring 的回退) ──

    pub fn set_secret(&self, account_id: &str, value: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO secrets (account_id, value) VALUES (?1,?2) \
                 ON CONFLICT(account_id) DO UPDATE SET value=excluded.value",
                params![account_id, value],
            )?;
        Ok(())
    }

    pub fn get_secret(&self, account_id: &str) -> Result<Option<String>, Error> {
        self.conn
            .query_row(
                "SELECT value FROM secrets WHERE account_id=?1",
                [account_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(Error::Db)
    }

    pub fn delete_secret(&self, account_id: &str) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM secrets WHERE account_id=?1", [account_id])?;
        Ok(())
    }

    // ── 账号 ──

    pub fn list_accounts(&self) -> Result<Vec<AccountConfig>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, provider, name, email, imap_host, imap_port, imap_ssl, smtp_host, \
             smtp_port, smtp_ssl, auth, poll_seconds, color, signature, last_sync_at \
             FROM accounts ORDER BY name, email",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AccountConfig {
                id: r.get(0)?,
                provider: Provider::from_str(&r.get::<_, String>(1)?),
                name: r.get(2)?,
                email: r.get(3)?,
                imap_host: r.get(4)?,
                imap_port: r.get::<_, i64>(5)? as u16,
                imap_ssl: r.get::<_, i64>(6)? != 0,
                smtp_host: r.get(7)?,
                smtp_port: r.get::<_, i64>(8)? as u16,
                smtp_ssl: r.get::<_, i64>(9)? != 0,
                auth: AuthKind::from_str(&r.get::<_, String>(10)?),
                poll_seconds: r.get(11)?,
                color: r.get(12)?,
                signature: r.get(13)?,
                last_sync_at: r.get(14)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_account(&self, id: &str) -> Result<Option<AccountConfig>, Error> {
        self.conn
            .query_row(
                "SELECT id, provider, name, email, imap_host, imap_port, imap_ssl, smtp_host, \
                 smtp_port, smtp_ssl, auth, poll_seconds, color, signature, last_sync_at \
                 FROM accounts WHERE id=?1",
                [id],
                |r| {
                    Ok(AccountConfig {
                        id: r.get(0)?,
                        provider: Provider::from_str(&r.get::<_, String>(1)?),
                        name: r.get(2)?,
                        email: r.get(3)?,
                        imap_host: r.get(4)?,
                        imap_port: r.get::<_, i64>(5)? as u16,
                        imap_ssl: r.get::<_, i64>(6)? != 0,
                        smtp_host: r.get(7)?,
                        smtp_port: r.get::<_, i64>(8)? as u16,
                        smtp_ssl: r.get::<_, i64>(9)? != 0,
                        auth: AuthKind::from_str(&r.get::<_, String>(10)?),
                        poll_seconds: r.get(11)?,
                        color: r.get(12)?,
                        signature: r.get(13)?,
                        last_sync_at: r.get(14)?,
                    })
                },
            )
            .optional()
            .map_err(Error::Db)
    }

    pub fn insert_account(&self, a: &AccountConfig) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO accounts (id, provider, name, email, imap_host, imap_port, imap_ssl, \
                 smtp_host, smtp_port, smtp_ssl, auth, poll_seconds, color, signature, last_sync_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    a.id,
                    a.provider.as_str(),
                    a.name,
                    a.email,
                    a.imap_host,
                    a.imap_port as i64,
                    a.imap_ssl as i64,
                    a.smtp_host,
                    a.smtp_port as i64,
                    a.smtp_ssl as i64,
                    a.auth.as_str(),
                    a.poll_seconds,
                    a.color,
                    a.signature,
                    a.last_sync_at
                ],
            )?;
        Ok(())
    }

    pub fn update_account(&self, a: &AccountConfig) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE accounts SET provider=?2, name=?3, email=?4, imap_host=?5, imap_port=?6, \
                 imap_ssl=?7, smtp_host=?8, smtp_port=?9, smtp_ssl=?10, auth=?11, poll_seconds=?12, \
                 color=?13, signature=?14, last_sync_at=?15 WHERE id=?1",
                params![
                    a.id,
                    a.provider.as_str(),
                    a.name,
                    a.email,
                    a.imap_host,
                    a.imap_port as i64,
                    a.imap_ssl as i64,
                    a.smtp_host,
                    a.smtp_port as i64,
                    a.smtp_ssl as i64,
                    a.auth.as_str(),
                    a.poll_seconds,
                    a.color,
                    a.signature,
                    a.last_sync_at
                ],
            )?;
        Ok(())
    }

    pub fn delete_account(&self, id: &str) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM accounts WHERE id=?1", [id])?;
        // 顺带清理该账号的邮件与同步状态
        self.conn
            .execute("DELETE FROM messages WHERE account_id=?1", [id])?;
        self.conn
            .execute("DELETE FROM sync_state WHERE account_id=?1", [id])?;
        Ok(())
    }

    pub fn set_last_sync(&self, account_id: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "UPDATE accounts SET last_sync_at=?1 WHERE id=?2",
                params![chrono::Utc::now().timestamp_millis(), account_id],
            )?;
        Ok(())
    }

    // ── 同步状态 ──

    pub fn get_uid_validity(&self, account_id: &str, folder: &str) -> Result<i64, Error> {
        self.conn
            .query_row(
                "SELECT uid_validity FROM sync_state WHERE account_id=?1 AND folder=?2",
                params![account_id, folder],
                |r| r.get(0),
            )
            .optional()
            .map(|v| v.unwrap_or(0))
            .map_err(Error::Db)
    }

    pub fn set_uid_validity(&self, account_id: &str, folder: &str, v: i64) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO sync_state (account_id, folder, uid_validity, last_sync) \
                 VALUES (?1,?2,?3,?4) \
                 ON CONFLICT(account_id, folder) DO UPDATE SET \
                 uid_validity=excluded.uid_validity, last_sync=excluded.last_sync",
                params![account_id, folder, v, chrono::Utc::now().timestamp()],
            )?;
        Ok(())
    }

    // ── 邮件 ──

    pub fn list_messages(
        &self,
        account_id: &str,
        folder: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<MessageSummary>, Error> {
        let sql = format!(
            "SELECT {MESSAGE_COLUMNS} FROM messages WHERE account_id=?1 AND folder=?2 \
             ORDER BY date DESC LIMIT ?3 OFFSET ?4"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![account_id, folder, limit, offset], row_to_stored)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?.to_summary());
        }
        Ok(out)
    }

    pub fn get_message_by_uid(
        &self,
        account_id: &str,
        folder: &str,
        uid: u32,
    ) -> Result<Option<StoredMessage>, Error> {
        let sql = format!(
            "SELECT {MESSAGE_COLUMNS} FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3"
        );
        self.conn
            .query_row(&sql, params![account_id, folder, uid], row_to_stored)
            .optional()
            .map_err(Error::Db)
    }

    pub fn get_flags(
        &self,
        account_id: &str,
        folder: &str,
        uid: u32,
    ) -> Result<Vec<String>, Error> {
        Ok(self
            .get_message_by_uid(account_id, folder, uid)?
            .map(|m| m.flags)
            .unwrap_or_default())
    }

    pub fn upsert_message(&self, m: &StoredMessage) -> Result<(), Error> {
        let flags = serde_json::to_string(&m.flags).unwrap_or_else(|_| "[]".into());
        let to_emails = serde_json::to_string(&m.to_emails).unwrap_or_else(|_| "[]".into());
        let cc_emails = serde_json::to_string(&m.cc_emails).unwrap_or_else(|_| "[]".into());
        let attachments_json = serde_json::to_string(&m.attachments).ok();
        let embedded_json = serde_json::to_string(&m.embedded).ok();

        self.conn.execute(
            "INSERT INTO messages (account_id, folder, uid, flags, subject, from_name, from_email, \
             to_emails, cc_emails, date, has_attachments, preview, html, text, attachments_json, \
             embedded_json, raw) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17) \
             ON CONFLICT(account_id, folder, uid) DO UPDATE SET \
             flags=excluded.flags, subject=excluded.subject, from_name=excluded.from_name, \
             from_email=excluded.from_email, to_emails=excluded.to_emails, \
             cc_emails=excluded.cc_emails, date=excluded.date, \
             has_attachments=excluded.has_attachments, preview=excluded.preview, \
             html=excluded.html, text=excluded.text, \
             attachments_json=excluded.attachments_json, embedded_json=excluded.embedded_json, \
             raw=CASE WHEN excluded.raw IS NULL THEN messages.raw ELSE excluded.raw END",
            params![
                m.account_id,
                m.folder,
                m.uid,
                flags,
                m.subject,
                m.from_name,
                m.from_email,
                to_emails,
                cc_emails,
                m.date,
                m.has_attachments as i64,
                m.preview,
                m.html,
                m.text,
                attachments_json,
                embedded_json,
                m.raw
            ],
        )?;

        let rowid: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
                params![m.account_id, m.folder, m.uid],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(rowid) = rowid {
            let body = m
                .text
                .clone()
                .unwrap_or_else(|| m.preview.clone());
            self.conn
                .execute("DELETE FROM messages_fts WHERE rowid=?1", params![rowid])?;
            self.conn
                .execute(
                    "INSERT INTO messages_fts (rowid, subject, from_email, to_emails, body, message_id) \
                     VALUES (?1,?2,?3,?4,?5,?6)",
                    params![rowid, m.subject, m.from_email, to_emails, body, rowid],
                )?;
        }
        Ok(())
    }

    pub fn update_flags(
        &self,
        account_id: &str,
        folder: &str,
        uid: u32,
        flags: Vec<String>,
    ) -> Result<(), Error> {
        let flags_json = serde_json::to_string(&flags).unwrap_or_else(|_| "[]".into());
        self.conn
            .execute(
                "UPDATE messages SET flags=?1 WHERE account_id=?2 AND folder=?3 AND uid=?4",
                params![flags_json, account_id, folder, uid],
            )?;
        Ok(())
    }

    pub fn max_uid(&self, account_id: &str, folder: &str) -> Result<Option<u32>, Error> {
        self.conn
            .query_row(
                "SELECT MAX(uid) FROM messages WHERE account_id=?1 AND folder=?2",
                params![account_id, folder],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map(|v| v.map(|x| x as u32))
            .map_err(Error::Db)
    }

    pub fn count_messages(&self, account_id: &str, folder: &str) -> Result<i64, Error> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE account_id=?1 AND folder=?2",
                params![account_id, folder],
                |r| r.get(0),
            )
            .map_err(Error::Db)
    }

    pub fn count_unread(&self, account_id: &str, folder: &str) -> Result<i64, Error> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE account_id=?1 AND folder=?2 \
                 AND flags NOT LIKE '%\\Seen%'",
                params![account_id, folder],
                |r| r.get(0),
            )
            .map_err(Error::Db)
    }

    pub fn delete_folder_messages(&self, account_id: &str, folder: &str) -> Result<(), Error> {
        let rowids: Vec<i64> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM messages WHERE account_id=?1 AND folder=?2")?;
            let rows = stmt.query_map(params![account_id, folder], |r| r.get(0))?;
            let mut v = Vec::new();
            for r in rows {
                v.push(r?);
            }
            v
        };
        self.conn
            .execute(
                "DELETE FROM messages WHERE account_id=?1 AND folder=?2",
                params![account_id, folder],
            )?;
        for rid in rowids {
            self.conn
                .execute("DELETE FROM messages_fts WHERE rowid=?1", params![rid])?;
        }
        Ok(())
    }

    pub fn delete_uids(
        &self,
        account_id: &str,
        folder: &str,
        uids: &[u32],
    ) -> Result<(), Error> {
        for uid in uids {
            let rowid: Option<i64> = self
                .conn
                .query_row(
                    "SELECT id FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
                    params![account_id, folder, uid],
                    |r| r.get(0),
                )
                .optional()?;
            self.conn
                .execute(
                    "DELETE FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
                    params![account_id, folder, uid],
                )?;
            if let Some(rid) = rowid {
                self.conn
                    .execute("DELETE FROM messages_fts WHERE rowid=?1", params![rid])?;
            }
        }
        Ok(())
    }

    pub fn search_messages(
        &self,
        account_id: &str,
        query: &str,
        folder: Option<&str>,
    ) -> Result<Vec<MessageSummary>, Error> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        // 将用户输入作为短语交给 FTS5,转义内部双引号
        let q = format!("\"{}\"", query.replace('"', "\"\""));
        let sql = format!(
            "SELECT {MESSAGE_COLUMNS} FROM messages m \
             WHERE m.id IN (SELECT rowid FROM messages_fts WHERE messages_fts MATCH ?1) \
             AND m.account_id=?2 AND (?3 IS NULL OR m.folder=?3) \
             ORDER BY m.date DESC LIMIT 200"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![q, account_id, folder], row_to_stored)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?.to_summary());
        }
        Ok(out)
    }

    // ── 设置 ──

    pub fn settings_get(&self, key: &str) -> Result<Option<String>, Error> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key=?1",
                [key],
                |r| r.get(0),
            )
            .optional()
            .map_err(Error::Db)
    }

    pub fn settings_get_all(&self) -> Result<HashMap<String, String>, Error> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut out = HashMap::new();
        for row in rows {
            let (k, v) = row?;
            out.insert(k, v);
        }
        Ok(out)
    }

    pub fn settings_set(&self, key: &str, value: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1,?2) \
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )?;
        Ok(())
    }
}

fn row_to_stored(r: &rusqlite::Row<'_>) -> rusqlite::Result<StoredMessage> {
    let flags: String = r.get(4)?;
    let to_emails: String = r.get(8)?;
    let cc_emails: String = r.get(9)?;
    let has_att: i64 = r.get(11)?;
    let att_json: Option<String> = r.get(15)?;
    let emb_json: Option<String> = r.get(16)?;
    Ok(StoredMessage {
        id: Some(r.get(0)?),
        account_id: r.get(1)?,
        folder: r.get(2)?,
        uid: r.get::<_, i64>(3)? as u32,
        flags: serde_json::from_str(&flags).unwrap_or_default(),
        subject: r.get(5)?,
        from_name: r.get(6)?,
        from_email: r.get(7)?,
        to_emails: serde_json::from_str(&to_emails).unwrap_or_default(),
        cc_emails: serde_json::from_str(&cc_emails).unwrap_or_default(),
        date: r.get(10)?,
        has_attachments: has_att != 0,
        preview: r.get(12)?,
        html: r.get(13)?,
        text: r.get(14)?,
        attachments: att_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default(),
        embedded: emb_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default(),
        raw: r.get(17)?,
    })
}
