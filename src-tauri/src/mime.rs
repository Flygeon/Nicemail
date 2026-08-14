//! 用 mail-parser 0.10 把原始 RFC822 / MIME 消息解析为内部存储模型。
//! 头部解析:parse_header;全文解析:parse_full。
//! 失败时降级:至少返回 subject/from/date。

use std::collections::{HashMap, HashSet};

use base64::{engine::general_purpose, Engine as _};
use chrono::TimeZone;
use mail_parser::{
    Address, ContentType, DateTime, GetHeader, HeaderName, Message, MessageParser, PartType,
};

use crate::models::{AttachmentMeta, StoredMessage};

/// 仅解析头部(IMAP HEADER.FIELDS 增量同步时使用)。
pub fn parse_header(
    account_id: &str,
    folder: &str,
    uid: u32,
    flags: &[String],
    header_bytes: &[u8],
) -> StoredMessage {
    let parser = MessageParser::default();
    let msg = parser.parse_headers(header_bytes).unwrap_or_default();

    let subject = msg.subject().unwrap_or("").to_string();
    let from = extract_addresses(msg.from());
    let to: Vec<String> = extract_addresses(msg.to()).into_iter().map(|(_, e)| e).collect();
    let cc: Vec<String> = extract_addresses(msg.cc()).into_iter().map(|(_, e)| e).collect();
    let date = msg.date().map(dt_to_epoch).unwrap_or(0);
    let (from_name, from_email) = from.first().cloned().unwrap_or_default();

    StoredMessage {
        id: None,
        account_id: account_id.to_string(),
        folder: folder.to_string(),
        uid,
        flags: flags.to_vec(),
        subject,
        from_name,
        from_email,
        to_emails: to,
        cc_emails: cc,
        date,
        has_attachments: false,
        preview: String::new(),
        html: None,
        text: None,
        attachments: Vec::new(),
        embedded: HashMap::new(),
        raw: None,
    }
}

/// 解析完整消息,生成正文 / 附件 / 内嵌图片 / 预览。
pub fn parse_full(
    account_id: &str,
    folder: &str,
    uid: u32,
    flags: &[String],
    raw: &[u8],
) -> StoredMessage {
    let parser = MessageParser::default();
    let msg = parser.parse(raw).unwrap_or_default();

    let subject = msg.subject().unwrap_or("").to_string();
    let from = extract_addresses(msg.from());
    let to: Vec<String> = extract_addresses(msg.to()).into_iter().map(|(_, e)| e).collect();
    let cc: Vec<String> = extract_addresses(msg.cc()).into_iter().map(|(_, e)| e).collect();
    let date = msg.date().map(dt_to_epoch).unwrap_or(0);
    let (from_name, from_email) = from.first().cloned().unwrap_or_default();

    let (html, text) = extract_bodies(&msg);
    let (attachments, embedded) = extract_attachments(&msg, html.as_deref());
    let preview = text.as_deref().map(preview_from_text).unwrap_or_default();
    let has_attachments = !attachments.is_empty();

    StoredMessage {
        id: None,
        account_id: account_id.to_string(),
        folder: folder.to_string(),
        uid,
        flags: flags.to_vec(),
        subject,
        from_name,
        from_email,
        to_emails: to,
        cc_emails: cc,
        date,
        has_attachments,
        preview,
        html,
        text,
        attachments,
        embedded,
        raw: Some(raw.to_vec()),
    }
}

/// 按附件顺序提取某个附件的字节(与 parse_full 的附件索引一致)。
pub fn extract_attachment(raw: &[u8], index: usize) -> Option<Vec<u8>> {
    let parser = MessageParser::default();
    let msg = parser.parse(raw)?;
    let html = extract_bodies(&msg).0;
    let cids = collect_cids(html.as_deref().unwrap_or(""));

    let mut att_index = 0usize;
    for &part_id in &msg.attachments {
        let part = msg.parts.get(part_id)?;
        let content_id = part
            .headers
            .header_value(&HeaderName::ContentId)
            .and_then(|v| v.as_text())
            .map(clean_cid);
        let disposition = part
            .headers
            .header_value(&HeaderName::ContentDisposition)
            .and_then(|v| v.as_content_type());
        let is_inline = disposition.map(|d| d.is_inline()).unwrap_or(false);
        let referenced = content_id
            .as_ref()
            .map(|c| cids.contains(c))
            .unwrap_or(false);

        if (is_inline && content_id.is_some()) || referenced {
            continue;
        }
        if att_index == index {
            return Some(part_body_bytes(&part.body));
        }
        att_index += 1;
    }
    None
}

pub fn preview_from_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect()
}

// ── 内部辅助 ──

/// 截断字符串到最大字符数(在字符边界切,避免破坏 UTF-8)。
fn cap_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        s[..end].to_string()
    }
}

fn extract_bodies(msg: &Message<'_>) -> (Option<String>, Option<String>) {
    let mut html: Option<String> = None;
    let mut text: Option<String> = None;

    for &pid in &msg.html_body {
        if html.is_some() {
            break;
        }
        if let Some(part) = msg.parts.get(pid) {
            if let PartType::Html(s) = &part.body {
                html = Some(cap_str(s, 500_000));
            }
        }
    }
    for &pid in &msg.text_body {
        if text.is_some() {
            break;
        }
        if let Some(part) = msg.parts.get(pid) {
            match &part.body {
                PartType::Text(s) => {
                    text = Some(cap_str(s, 500_000));
                }
                PartType::Html(s) if html.is_none() => {
                    html = Some(cap_str(s, 500_000));
                }
                _ => {}
            }
        }
    }
    if html.is_none() {
        for part in &msg.parts {
            if let PartType::Html(s) = &part.body {
                html = Some(cap_str(s, 500_000));
                break;
            }
        }
    }
    (html, text)
}

fn extract_attachments(
    msg: &Message<'_>,
    html: Option<&str>,
) -> (Vec<AttachmentMeta>, HashMap<String, String>) {
    let cids = collect_cids(html.unwrap_or(""));
    let mut attachments = Vec::new();
    let mut embedded = HashMap::new();
    let mut total_embedded = 0usize;
    let mut index = 0usize;

    for &part_id in &msg.attachments {
        let Some(part) = msg.parts.get(part_id) else {
            continue;
        };
        let ctype = part
            .headers
            .header_value(&HeaderName::ContentType)
            .and_then(|v| v.as_content_type());
        let mime = ctype
            .map(ct_to_mime)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let filename = part
            .headers
            .header_value(&HeaderName::ContentDisposition)
            .and_then(|v| v.as_content_type())
            .and_then(|cd| cd.attribute("filename"))
            .map(|s| s.to_string())
            .or_else(|| {
                ctype
                    .and_then(|ct| ct.attribute("name"))
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| format!("attachment-{}", part_id + 1));
        let content_id = part
            .headers
            .header_value(&HeaderName::ContentId)
            .and_then(|v| v.as_text())
            .map(clean_cid);
        let disposition = part
            .headers
            .header_value(&HeaderName::ContentDisposition)
            .and_then(|v| v.as_content_type());
        let is_inline = disposition.map(|d| d.is_inline()).unwrap_or(false);
        let referenced = content_id
            .as_ref()
            .map(|c| cids.contains(c))
            .unwrap_or(false);

        let bytes = part_body_bytes(&part.body);
        let size = bytes.len();

        if (is_inline && content_id.is_some()) || referenced {
            if let Some(cid) = content_id {
                // 超大/过多内嵌图不做 base64 编码,避免响应体暴涨卡死(前端显示为缺失图)
                if bytes.len() < 1_500_000 && total_embedded + bytes.len() < 4_000_000 {
                    let b64 = general_purpose::STANDARD.encode(&bytes);
                    total_embedded += bytes.len();
                    embedded.insert(cid, format!("data:{mime};base64,{b64}"));
                }
            }
        } else {
            attachments.push(AttachmentMeta {
                index,
                filename,
                mime,
                size,
                content_id,
                is_inline,
            });
            index += 1;
        }
    }
    (attachments, embedded)
}

fn extract_addresses(addr: Option<&Address<'_>>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(addr) = addr {
        match addr {
            Address::List(list) => {
                for a in list {
                    out.push((
                        a.name.as_deref().unwrap_or("").to_string(),
                        a.address.as_deref().unwrap_or("").to_string(),
                    ));
                }
            }
            Address::Group(groups) => {
                for g in groups {
                    for a in &g.addresses {
                        out.push((
                            a.name.as_deref().unwrap_or("").to_string(),
                            a.address.as_deref().unwrap_or("").to_string(),
                        ));
                    }
                }
            }
        }
    }
    out
}

fn dt_to_epoch(dt: &DateTime) -> i64 {
    let sign = if dt.tz_before_gmt { -1i32 } else { 1i32 };
    let offset_secs = sign * (dt.tz_hour as i32 * 3600 + dt.tz_minute as i32 * 60);
    let offset = chrono::FixedOffset::east_opt(offset_secs)
        .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).expect("offset 0 is always valid"));
    let naive = chrono::NaiveDate::from_ymd_opt(dt.year as i32, dt.month as u32, dt.day as u32)
        .and_then(|d| d.and_hms_opt(dt.hour as u32, dt.minute as u32, dt.second as u32));
    naive.and_then(|n| offset.from_local_datetime(&n).single())
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
}

fn ct_to_mime(ct: &ContentType<'_>) -> String {
    match ct.subtype() {
        Some(sub) => format!("{}/{}", ct.ctype(), sub),
        None => ct.ctype().to_string(),
    }
}

fn clean_cid(s: &str) -> String {
    s.trim().trim_matches('<').trim_matches('>').to_string()
}

fn part_body_bytes(body: &PartType<'_>) -> Vec<u8> {
    match body {
        PartType::Text(s) | PartType::Html(s) => s.as_bytes().to_vec(),
        PartType::Binary(b) | PartType::InlineBinary(b) => b.to_vec(),
        PartType::Message(m) => m.raw_message.as_ref().to_vec(),
        _ => Vec::new(),
    }
}

/// 从 HTML 中收集 `cid:` 引用。
fn collect_cids(html: &str) -> HashSet<String> {
    let mut cids = HashSet::new();
    let lower = html.to_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        if &bytes[i..i + 4] == b"cid:" {
            let mut j = i + 4;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || b"-._=@".contains(&bytes[j]))
            {
                j += 1;
            }
            let cid = lower[i + 4..j]
                .trim_matches('>')
                .trim_matches('"')
                .to_string();
            if !cid.is_empty() {
                cids.insert(cid);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    cids
}
