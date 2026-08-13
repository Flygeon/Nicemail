//! 密码 / OAuth refresh token 的本地安全存储封装。
//! service 名固定为 "nicemail",username = 账号 id。
//! OAuth refresh token 以 username = "oauth:<provider>" 存放。

use keyring::Entry;

use crate::error::Error;

const SERVICE: &str = "nicemail";

/// 保存密码或 OAuth refresh token。已存在则覆盖。
pub fn set_password(account_id: &str, secret: &str) -> Result<(), Error> {
    let entry = Entry::new(SERVICE, account_id)?;
    entry.set_password(secret)?;
    Ok(())
}

/// 读取密码或 refresh token。不存在 / 读取失败返回 None。
pub fn get_password(account_id: &str) -> Option<String> {
    match Entry::new(SERVICE, account_id) {
        Ok(entry) => entry.get_password().ok(),
        Err(_) => None,
    }
}

/// 删除保存的凭据。
pub fn delete_password(account_id: &str) -> Result<(), Error> {
    let entry = Entry::new(SERVICE, account_id)?;
    entry.delete_credential()?;
    Ok(())
}
