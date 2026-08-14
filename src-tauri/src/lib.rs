mod commands;
mod db;
mod error;
mod imap;
mod keyring;
mod mime;
mod models;
mod oauth;
mod smtp;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tauri::Manager;

use crate::db::Db;

/// app data dir(在 setup 中初始化)。
pub static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> &'static Path {
    DATA_DIR.get().expect("DATA_DIR 尚未初始化")
}

/// 从本地 DB secrets 表读取一个秘密(keyring 的回退)。
pub fn secret_db_get(key: &str) -> Option<String> {
    let db = Db::open(&data_dir().join("nicemail.db")).ok()?;
    db.get_secret(key).ok().flatten()
}

/// 写入一个秘密到本地 DB secrets 表(keyring 的回退)。
pub fn secret_db_set(key: &str, value: &str) {
    if let Ok(db) = Db::open(&data_dir().join("nicemail.db")) {
        let _ = db.set_secret(key, value);
    }
}

/// panic 也写一份到 app data dir/panic.log,方便定位闪退。
fn setup_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        if let Some(dir) = DATA_DIR.get() {
            use std::io::Write;
            let path = dir.join("panic.log");
            let msg = format!("{}\n", info);
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut f| f.write_all(msg.as_bytes()));
        }
    }));
}

/// 确保 oauth_config.json 占位文件存在。
fn ensure_oauth_config() {
    let path = data_dir().join("oauth_config.json");
    if !path.exists() {
        let _ = std::fs::write(
            &path,
            r#"{"google":{"clientId":""},"outlook":{"clientId":""}}"#,
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    setup_panic_hook();
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("nicemail".into()),
                    }),
                ])
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let _ = DATA_DIR.set(dir);
            ensure_oauth_config();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::account_list,
            commands::account_add,
            commands::account_update,
            commands::account_delete,
            commands::account_test,
            commands::oauth_config,
            commands::oauth_start,
            commands::oauth_finish,
            commands::mailbox_list,
            commands::mail_sync,
            commands::mail_list,
            commands::mail_get,
            commands::mail_set_flag,
            commands::mail_move,
            commands::mail_delete,
            commands::mail_search,
            commands::mail_attachment_save,
            commands::mail_send,
            commands::mail_save_draft,
            commands::settings_get,
            commands::settings_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
