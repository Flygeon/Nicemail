// Nicemail — Rust ⇄ TypeScript 桥接契约
// 所有类型名/字段名与 src-tauri 侧 serde 结构体一一对应,勿单方面改名。
// invoke 的命令名以 src-tauri/src/commands.rs 的 #[tauri::command] 为准。

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/* ─────────────────────────── 模型 ─────────────────────────── */

export type Provider = '163' | '126' | 'qq' | 'gmail' | 'outlook' | 'custom';

export type AuthKind = 'password' | 'oauth2';

export interface AccountConfig {
  id: string;
  provider: Provider;
  /** 显示名称(如 "张三") */
  name: string;
  /** 完整邮箱地址 */
  email: string;
  imapHost: string;
  imapPort: number;
  imapSsl: boolean;
  smtpHost: string;
  smtpPort: number;
  smtpSsl: boolean;
  auth: AuthKind;
  /** 同步间隔秒;0 = 仅手动 */
  pollSeconds: number;
  color: string;
  signature: string;
  /** 最后成功同步时间(epoch ms),无则为 null */
  lastSyncAt: number | null;
}

export interface AccountDraft {
  provider: Provider;
  name: string;
  email: string;
  auth: AuthKind;
  imapHost: string;
  imapPort: number;
  imapSsl: boolean;
  smtpHost: string;
  smtpPort: number;
  smtpSsl: boolean;
  /** 密码/授权码。OAuth 提供方可为空串(走登录流程) */
  password: string;
  /** OAuth 用:provider + 一次性授权 */
  useOAuth: boolean;
}

export interface Folder {
  /** IMAP 完整路径,如 "INBOX"、"[Gmail]/已发送邮件" */
  fullName: string;
  name: string;
  delimiter: string;
  flags: string[];
  selectable: boolean;
  unreadCount: number;
  totalCount: number;
}

export type FolderKind = 'inbox' | 'drafts' | 'sent' | 'trash' | 'flagged' | 'archive' | 'junk' | 'other';

export interface MessageSummary {
  id: number;
  accountId: string;
  folder: string;
  uid: number;
  flags: string[];
  subject: string;
  fromName: string;
  fromEmail: string;
  toEmails: string[];
  date: number; // epoch ms
  hasAttachments: boolean;
  preview: string;
  unread: boolean;
  starred: boolean;
}

export interface AttachmentMeta {
  index: number;
  filename: string;
  mime: string;
  size: number;
  contentId: string | null;
  isInline: boolean;
}

export interface MessageDetail {
  id: number;
  accountId: string;
  folder: string;
  uid: number;
  subject: string;
  fromName: string;
  fromEmail: string;
  toEmails: string[];
  ccEmails: string[];
  date: number;
  flags: string[];
  html: string | null;
  text: string | null;
  attachments: AttachmentMeta[];
  /** 内嵌图片(Content-ID → data URL),用于 HTML 渲染 */
  embedded: Record<string, string>;
}

export interface SendAttachment {
  /** 本地绝对路径 */
  path: string;
  filename: string;
  mime: string | null;
}

export interface SendRequest {
  accountId: string;
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  bodyHtml: string;
  bodyText: string;
  attachments: SendAttachment[];
}

export interface SyncResult {
  accountId: string;
  folder: string;
  added: number;
  updated: number;
  removed: number;
}

export interface OAuthConfig {
  google: { configured: boolean };
  outlook: { configured: boolean };
}

export interface TestResult {
  ok: boolean;
  message: string;
}

/* ─────────────────────────── 命令 ─────────────────────────── */

// ── 账号 ──
export const accountList = () => invoke<AccountConfig[]>('account_list');
export const accountAdd = (draft: AccountDraft) => invoke<AccountConfig>('account_add', { draft });
export const accountUpdate = (account: AccountConfig, password?: string) =>
  invoke<AccountConfig>('account_update', { account, password: password ?? null });
export const accountDelete = (id: string) => invoke<void>('account_delete', { id });
export const accountTest = (draft: AccountDraft) => invoke<TestResult>('account_test', { draft });

// ── OAuth ──
export const oauthConfig = () => invoke<OAuthConfig>('oauth_config');
export const oauthStart = (provider: Extract<Provider, 'gmail' | 'outlook'>) =>
  invoke<{ authUrl: string }>('oauth_start', { provider });
export const oauthFinish = (provider: Extract<Provider, 'gmail' | 'outlook'>, code: string, state: string) =>
  invoke<{ email: string }>('oauth_finish', { provider, code, state });

// ── 文件夹与邮件 ──
export const mailboxList = (accountId: string) => invoke<Folder[]>('mailbox_list', { accountId });
export const mailSync = (accountId: string, folder: string) =>
  invoke<SyncResult>('mail_sync', { accountId, folder });
export const mailList = (accountId: string, folder: string, offset: number, limit: number) =>
  invoke<MessageSummary[]>('mail_list', { accountId, folder, offset, limit });
export const mailGet = (accountId: string, folder: string, uid: number) =>
  invoke<MessageDetail>('mail_get', { accountId, folder, uid });
export const mailSetFlag = (accountId: string, folder: string, uids: number[], flag: string, value: boolean) =>
  invoke<void>('mail_set_flag', { accountId, folder, uids, flag, value });
export const mailMove = (accountId: string, folder: string, uids: number[], destFolder: string) =>
  invoke<void>('mail_move', { accountId, folder, uids, destFolder });
export const mailDelete = (accountId: string, folder: string, uids: number[]) =>
  invoke<void>('mail_delete', { accountId, folder, uids });
export const mailSearch = (accountId: string, query: string, folder?: string) =>
  invoke<MessageSummary[]>('mail_search', { accountId, query, folder: folder ?? null });
export const mailAttachmentSave = (accountId: string, folder: string, uid: number, index: number, destPath: string) =>
  invoke<void>('mail_attachment_save', { accountId, folder, uid, index, destPath });

// ── 发送 ──
export const mailSend = (request: SendRequest) => invoke<{ messageId: string }>('mail_send', { request });
export const mailSaveDraft = (accountId: string, request: SendRequest) =>
  invoke<void>('mail_save_draft', { accountId, request });

// ── 设置 ──
export const settingsGet = () => invoke<Record<string, string>>('settings_get');
export const settingsSet = (key: string, value: string) => invoke<void>('settings_set', { key, value });

/* ─────────────────────────── 事件 ─────────────────────────── */

export interface SyncProgressEvent { accountId: string; folder: string; processed: number; total: number }
export interface SyncDoneEvent { accountId: string; folder: string; ok: boolean; message: string }
export interface MailChangedEvent { accountId: string; folder: string }
export interface OAuthReadyEvent { provider: 'gmail' | 'outlook'; email: string }
export interface OAuthErrorEvent { provider: 'gmail' | 'outlook'; message: string }

export function onSyncProgress(handler: (e: SyncProgressEvent) => void): Promise<UnlistenFn> {
  return listen<SyncProgressEvent>('sync://progress', (e) => handler(e.payload));
}
export function onSyncDone(handler: (e: SyncDoneEvent) => void): Promise<UnlistenFn> {
  return listen<SyncDoneEvent>('sync://done', (e) => handler(e.payload));
}
export function onMailChanged(handler: (e: MailChangedEvent) => void): Promise<UnlistenFn> {
  return listen<MailChangedEvent>('mail://changed', (e) => handler(e.payload));
}
export function onOAuthReady(handler: (e: OAuthReadyEvent) => void): Promise<UnlistenFn> {
  return listen<OAuthReadyEvent>('oauth://ready', (e) => handler(e.payload));
}
export function onOAuthError(handler: (e: OAuthErrorEvent) => void): Promise<UnlistenFn> {
  return listen<OAuthErrorEvent>('oauth://error', (e) => handler(e.payload));
}

/* ─────────────────────────── 工具 ─────────────────────────── */

export function folderKind(folder: Folder): FolderKind {
  const f = folder.flags.map((x) => x.toLowerCase());
  if (f.includes('\\inbox')) return 'inbox';
  if (f.includes('\\drafts')) return 'drafts';
  if (f.includes('\\sent')) return 'sent';
  if (f.includes('\\trash')) return 'trash';
  if (f.includes('\\flagged')) return 'flagged';
  if (f.includes('\\archive')) return 'archive';
  if (f.includes('\\junk') || f.includes('\\spam')) return 'junk';
  return 'other';
}
