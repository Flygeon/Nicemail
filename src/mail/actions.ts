// 前端数据动作层:面板组件只调用这些函数,不直接碰 Tauri API。
import * as api from './api';
import {
  accounts, folders, messages, messageDetail, selectedMessageId,
  selection, view, composeOpen, loadingMessages, loadingDetail, syncing,
  resetMailView, showToast,
} from './state';

let activeSyncToken = 0;

/** 应用启动:拉账号、定默认选区 */
export async function init(): Promise<void> {
  accounts.value = await api.accountList();
  if (accounts.value.length === 0) return;
  const first = accounts.value[0];
  await selectAccount(first.id);
}

export async function selectAccount(accountId: string): Promise<void> {
  const acc = accounts.value.find((a) => a.id === accountId);
  if (!acc) return;
  selection.accountId = accountId;
  resetMailView();
  try {
    const list = await api.mailboxList(accountId);
    folders.value[accountId] = list;
    const inbox = list.find((f) => f.flags.some((x) => x.toLowerCase() === '\\inbox'))
      ?? list.find((f) => f.selectable)
      ?? list[0];
    if (inbox) await selectFolder(accountId, inbox.fullName);
  } catch (err) {
    // 邮箱不可达时保留空文件夹树
    folders.value[accountId] = [];
  }
}

export async function selectFolder(accountId: string, folder: string): Promise<void> {
  selection.accountId = accountId;
  selection.folder = folder;
  resetMailView();
  await loadMessages(accountId, folder, 0);
  // 本地为空(首次进入/刚添加账号)→ 自动同步一次,让收件箱立即有内容
  if (messages.value.length === 0) {
    await refreshCurrentFolder();
  }
}

/** 分页拉取邮件列表(每次 100 条,按时间倒序) */
export async function loadMessages(accountId: string, folder: string, offset: number): Promise<void> {
  loadingMessages.value = true;
  try {
    const list = await api.mailList(accountId, folder, offset, 100);
    if (offset === 0) {
      messages.value = list;
    } else {
      messages.value = [...messages.value, ...list];
    }
  } finally {
    loadingMessages.value = false;
  }
}

export async function loadMoreMessages(): Promise<void> {
  if (loadingMessages.value || !selection.accountId || !selection.folder) return;
  await loadMessages(selection.accountId, selection.folder, messages.value.length);
}

/** 给 Promise 加超时,避免网络问题导致按钮卡死 */
function withTimeout<T>(promise: Promise<T>, ms: number, message: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(message)), ms);
    promise.then(
      (v) => { clearTimeout(timer); resolve(v); },
      (e) => { clearTimeout(timer); reject(e); },
    );
  });
}

/** 手动/自动同步当前文件夹 */
export async function refreshCurrentFolder(): Promise<void> {
  if (!selection.accountId || !selection.folder) return;
  syncing.value = true;
  const token = ++activeSyncToken;
  try {
    await withTimeout(api.mailSync(selection.accountId, selection.folder), 60000, '同步超时');
    if (token !== activeSyncToken) return;
    await loadMessages(selection.accountId, selection.folder, 0);
    await refreshUnreadCounts(selection.accountId);
  } catch (err) {
    console.error('同步失败:', err);
    showToast(`同步失败: ${String(err)}`);
  } finally {
    if (token === activeSyncToken) syncing.value = false;
  }
}

export async function refreshUnreadCounts(accountId: string): Promise<void> {
  try {
    const list = await api.mailboxList(accountId);
    folders.value[accountId] = list;
  } catch { /* 忽略 */ }
}

/** 添加账号(会先测试连接);成功后加入列表并选中 */
export async function addAccount(draft: api.AccountDraft): Promise<api.AccountConfig> {
  const acc = await api.accountAdd(draft);
  accounts.value = [...accounts.value, acc];
  await selectAccount(acc.id);
  return acc;
}

/** 删除账号(移除本地缓存),若删的是当前账号则切到下一个 */
export async function removeAccount(accountId: string): Promise<void> {
  await api.accountDelete(accountId);
  accounts.value = accounts.value.filter((a) => a.id !== accountId);
  delete folders.value[accountId];
  if (selection.accountId === accountId) {
    resetMailView();
    if (accounts.value.length > 0) await selectAccount(accounts.value[0].id);
  }
}

export async function selectMessage(summary: { id: number }): Promise<void> {
  selectedMessageId.value = summary.id;
  if (!selection.accountId || !selection.folder) return;
  loadingDetail.value = true;
  try {
    const summary2 = messages.value.find((m) => m.id === summary.id);
    if (!summary2) return;
    const detail = await api.mailGet(selection.accountId, selection.folder, summary2.uid);
    messageDetail.value = detail;
    if (summary2.unread) {
      // 标记已读(本地 + 服务器)
      await api.mailSetFlag(selection.accountId, selection.folder, [summary2.uid], '\\Seen', true);
      summary2.unread = false;
      summary2.flags = summary2.flags.includes('\\Seen') ? summary2.flags : [...summary2.flags, '\\Seen'];
    }
  } finally {
    loadingDetail.value = false;
  }
}

export async function toggleStar(summary: { id: number }): Promise<void> {
  const m = messages.value.find((x) => x.id === summary.id);
  if (!m || !selection.accountId || !selection.folder) return;
  const next = !m.starred;
  m.starred = next;
  try {
    await api.mailSetFlag(selection.accountId, selection.folder, [m.uid], '\\Flagged', next);
  } catch { /* 回滚本地 */ m.starred = !next; }
}

export async function deleteSelected(): Promise<void> {
  if (!selection.accountId || !selection.folder || selectedMessageId.value == null) return;
  const m = messages.value.find((x) => x.id === selectedMessageId.value);
  if (!m) return;
  const folder = selection.folder;
  try {
    await api.mailDelete(selection.accountId, folder, [m.uid]);
    messages.value = messages.value.filter((x) => x.id !== m.id);
    selectedMessageId.value = null;
    messageDetail.value = null;
  } catch { /* ignore */ }
}

export async function runSearch(query: string, folder?: string): Promise<void> {
  if (!selection.accountId) return;
  loadingMessages.value = true;
  try {
    const result = await api.mailSearch(selection.accountId, query, folder);
    messages.value = result;
    selectedMessageId.value = null;
    messageDetail.value = null;
  } finally {
    loadingMessages.value = false;
  }
}

export function openCompose(): void {
  composeOpen.value = true;
}

export function closeCompose(): void {
  composeOpen.value = false;
}
