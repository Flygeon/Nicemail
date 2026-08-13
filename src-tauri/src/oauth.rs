//! OAuth2 授权码 + PKCE,回调用本地回环服务器(tiny_http,固定端口 52999)。
//! 凭据文件 oauth_config.json 位于 app data dir,只要求 clientId(public client)。

use std::collections::HashMap;
use std::io::Read;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use sha2::Digest;
use tauri::Emitter;

use crate::error::Error;
use crate::keyring;
use crate::models::{OAuthConfig, OAuthError, OAuthProviderConfig, OAuthReady};

const REDIRECT_URI: &str = "http://127.0.0.1:52999/callback";
const PORT: u16 = 52999;

struct PendingOAuth {
    state: String,
    verifier: String,
    client_id: String,
}

static PENDING: LazyLock<Mutex<HashMap<String, PendingOAuth>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static RESULTS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 读取 oauth_config.json,返回各 provider 是否已配置 clientId。
pub fn oauth_config() -> OAuthConfig {
    let json = read_config();
    let google = json
        .get("google")
        .and_then(|v| v.get("clientId"))
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let outlook = json
        .get("outlook")
        .and_then(|v| v.get("clientId"))
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    OAuthConfig {
        google: OAuthProviderConfig {
            configured: google,
        },
        outlook: OAuthProviderConfig {
            configured: outlook,
        },
    }
}

/// 启动 OAuth:生成 PKCE + state,启动回环服务器,返回浏览器授权 URL。
pub fn start_oauth(app: tauri::AppHandle, provider: &str) -> Result<String, Error> {
    let client_id = client_id_for(provider);
    if client_id.is_empty() {
        return Err(Error::Config(format!(
            "{provider} 的 OAuth clientId 未配置,请填写应用数据目录下的 oauth_config.json"
        )));
    }
    let (auth_url_base, token_url, scope) = endpoints(provider)?;

    let state = random_string(32);
    let verifier = random_string(64);
    let challenge = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(verifier.as_bytes());
        general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
    };
    {
        let mut pending = PENDING.lock().unwrap();
        pending.insert(
            provider.to_string(),
            PendingOAuth {
                state: state.clone(),
                verifier,
                client_id: client_id.clone(),
            },
        );
    }

    let mut url = url::Url::parse(auth_url_base).map_err(|e| Error::OAuth(e.to_string()))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("response_type", "code");
        pairs.append_pair("client_id", &client_id);
        pairs.append_pair("redirect_uri", REDIRECT_URI);
        pairs.append_pair("scope", scope);
        pairs.append_pair("state", &state);
        pairs.append_pair("code_challenge", &challenge);
        pairs.append_pair("code_challenge_method", "S256");
        if provider == "gmail" {
            pairs.append_pair("access_type", "offline");
        }
        pairs.append_pair("prompt", "consent");
    }
    let auth_url = url.to_string();

    // 先绑定端口(占位失败立刻报错),再把 server 交给后台线程收回调
    let addr = format!("127.0.0.1:{PORT}");
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| Error::OAuth(format!("启动本地回环服务器失败: {e}")))?;
    let provider_owned = provider.to_string();
    let _ = std::thread::spawn(move || {
        loopback_serve(app, server, provider_owned, token_url.to_string(), client_id);
    });

    Ok(auth_url)
}

/// 兜底:若回环服务器已自动完成则直接返回邮箱;否则用传入的 code/state 手动兑换。
pub fn finish(provider: &str, code: &str, state: &str) -> Result<String, Error> {
    if let Some(email) = RESULTS.lock().unwrap().get(provider).cloned() {
        return Ok(email);
    }
    let client_id = client_id_for(provider);
    if client_id.is_empty() {
        return Err(Error::Config(format!(
            "{provider} 的 OAuth clientId 未配置,请填写 oauth_config.json"
        )));
    }
    let (_auth, token_url, _scope) = endpoints(provider)?;
    exchange_code(provider, token_url, client_id.as_str(), code, state)
}

/// 供 IMAP/SMTP XOAUTH2 使用:读取 refresh token,刷新出 access token。
pub fn access_token_for(provider: &str) -> Result<String, Error> {
    let refresh_token = keyring::get_password(&format!("oauth:{provider}"))
        .ok_or_else(|| Error::OAuth(format!("{provider} 尚未完成 OAuth 授权,请先添加授权")))?;
    let client_id = client_id_for(provider);
    if client_id.is_empty() {
        return Err(Error::Config(format!(
            "{provider} 的 OAuth clientId 未配置,请填写 oauth_config.json"
        )));
    }
    let (_auth, token_url, _scope) = endpoints(provider)?;
    let form = [
        ("refresh_token", refresh_token.as_str()),
        ("client_id", client_id.as_str()),
        ("grant_type", "refresh_token"),
    ];
    let resp = ureq::post(token_url)
        .send_form(form)
        .map_err(|e| Error::OAuth(format!("刷新 token 失败: {e}")))?;
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::OAuth(format!("读取刷新响应失败: {e}")))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::OAuth(format!("解析刷新响应失败: {e}")))?;
    json["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::OAuth(format!("刷新 access_token 失败: {body}")))
}

// ── 内部 ──

fn loopback_serve(
    app: tauri::AppHandle,
    server: tiny_http::Server,
    provider: String,
    token_url: String,
    client_id: String,
) {
    for _ in 0..8 {
        let request = match server.recv_timeout(Duration::from_secs(60)) {
            Ok(Some(req)) => req,
            Ok(None) => continue,
            Err(_) => break,
        };
        let path = request.url().to_string();
        if path.starts_with("/callback") {
            let query = path.splitn(2, '?').nth(1).unwrap_or("");
            let params: HashMap<String, String> =
                url::form_urlencoded::parse(query.as_bytes()).into_owned().collect();

            if let Some(err) = params.get("error") {
                let _ = app.emit(
                    "oauth://error",
                    OAuthError {
                        provider: provider.clone(),
                        message: err.clone(),
                    },
                );
                let _ = request.respond(
                    tiny_http::Response::from_string(format!(
                        "<h1>授权失败</h1><p>{err}</p>"
                    ))
                    .with_status_code(400),
                );
            } else if let (Some(code), Some(state)) =
                (params.get("code").cloned(), params.get("state").cloned())
            {
                match exchange_code(&provider, &token_url, &client_id, &code, &state) {
                    Ok(email) => {
                        let _ = app.emit(
                            "oauth://ready",
                            OAuthReady {
                                provider: provider.clone(),
                                email: email.clone(),
                            },
                        );
                        let _ = request.respond(tiny_http::Response::from_string(
                            "<h1>授权成功</h1><p>已授权成功,请关闭此页面返回 Nicemail。</p>",
                        ));
                    }
                    Err(e) => {
                        let _ = app.emit(
                            "oauth://error",
                            OAuthError {
                                provider: provider.clone(),
                                message: e.to_string(),
                            },
                        );
                        let _ = request.respond(
                            tiny_http::Response::from_string(format!(
                                "<h1>授权失败</h1><p>{e}</p>"
                            ))
                            .with_status_code(400),
                        );
                    }
                }
            } else {
                let _ = request.respond(
                    tiny_http::Response::from_string("<h1>缺少必要参数</h1>").with_status_code(400),
                );
            }
            break;
        } else {
            let _ = request.respond(tiny_http::Response::from_string("Nicemail OAuth 回环服务器"));
        }
    }
}

fn exchange_code(
    provider: &str,
    token_url: &str,
    client_id: &str,
    code: &str,
    state: &str,
) -> Result<String, Error> {
    let verifier = {
        let pending = PENDING.lock().unwrap();
        let p = pending
            .get(provider)
            .ok_or_else(|| Error::OAuth("OAuth 会话不存在或已过期,请重新发起授权".into()))?;
        if p.state != state {
            return Err(Error::OAuth("state 校验失败,请重新发起授权".into()));
        }
        p.verifier.clone()
    };

    let form = [
        ("code", code),
        ("client_id", client_id),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", verifier.as_str()),
        ("grant_type", "authorization_code"),
    ];
    let resp = ureq::post(token_url)
        .send_form(form)
        .map_err(|e| Error::OAuth(format!("换取 token 失败: {e}")))?;
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::OAuth(format!("读取 token 响应失败: {e}")))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::OAuth(format!("解析 token 响应失败: {e}")))?;

    let refresh_token = json["refresh_token"].as_str().ok_or_else(|| {
        Error::OAuth(format!("未获取到 refresh_token(可能未勾选离线权限): {body}"))
    })?;
    keyring::set_password(&format!("oauth:{provider}"), refresh_token)?;

    let email = extract_email(&json);
    if email.is_empty() {
        return Err(Error::OAuth("未能从 token 中解析邮箱".into()));
    }
    RESULTS
        .lock()
        .unwrap()
        .insert(provider.to_string(), email.clone());
    Ok(email)
}

fn extract_email(json: &serde_json::Value) -> String {
    if let Some(id_token) = json["id_token"].as_str() {
        if let Some(payload) = id_token.split('.').nth(1) {
            if let Ok(decoded) = general_purpose::URL_SAFE_NO_PAD.decode(payload) {
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&decoded) {
                    if let Some(e) = v["email"].as_str() {
                        return e.to_string();
                    }
                }
            }
        }
    }
    json["email"].as_str().unwrap_or("").to_string()
}

fn read_config() -> serde_json::Value {
    let path = crate::data_dir().join("oauth_config.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn client_id_for(provider: &str) -> String {
    read_config()
        .get(provider)
        .and_then(|v| v.get("clientId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn endpoints(provider: &str) -> Result<(&'static str, &'static str, &'static str), Error> {
    match provider {
        "gmail" => Ok((
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "openid email https://www.googleapis.com/auth/gmail.imap \
             https://www.googleapis.com/auth/gmail.send \
             https://www.googleapis.com/auth/gmail.modify",
        )),
        "outlook" => Ok((
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            "openid email offline_access https://outlook.office.com/IMAP.AccessAsUser.All \
             https://outlook.office.com/SMTP.Send",
        )),
        _ => Err(Error::InvalidInput(format!(
            "不支持的 OAuth provider: {provider}"
        ))),
    }
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
