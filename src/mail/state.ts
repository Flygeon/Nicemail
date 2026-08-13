// Nicemail 全局响应式状态
import { reactive, ref } from 'vue';
import type {
  AccountConfig,
  Folder,
  FolderKind,
  MessageDetail,
  MessageSummary,
} from './api';

export type View = 'mail' | 'settings';

export interface CurrentSelection {
  accountId: string | null;
  folder: string | null;
}

/** 当前文件夹下的邮件(已按时间倒序) */
export const messages = ref<MessageSummary[]>([]);
export const selectedMessageId = ref<number | null>(null);
export const messageDetail = ref<MessageDetail | null>(null);
export const loadingMessages = ref(false);
export const loadingDetail = ref(false);
export const syncing = ref(false);

export const accounts = ref<AccountConfig[]>([]);
/** accountId → 文件夹列表 */
export const folders = ref<Record<string, Folder[]>>({});
/** 当前选中:账号 + 文件夹 */
export const selection = reactive<CurrentSelection>({ accountId: null, folder: null });

export const view = ref<View>('mail');
export const composeOpen = ref(false);
export const searchOpen = ref(false);

/** 写信预填信息(回复/转发/新建时由 ReadingPane 或外部写入,ComposePane onMounted 读取后清空) */
export interface ComposeInitial {
  to?: string;
  cc?: string;
  bcc?: string;
  subject?: string;
  body?: string;
  accountId?: string;
}

export const composeInitial = ref<ComposeInitial | null>(null);

/* ── 全局轻提示(toast,发送成功等) ── */
export const toastVisible = ref(false);
export const toastMessage = ref('');
let toastTimer: ReturnType<typeof setTimeout> | null = null;

export function showToast(message: string, duration = 3200): void {
  toastMessage.value = message;
  toastVisible.value = true;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { toastVisible.value = false; }, duration);
}

/** 账号头像底色(按账号名稳定派生) */
export function accountColor(account: AccountConfig | null | undefined): string {
  const palette = ['#0078D4', '#0F7B0F', '#9D5D00', '#C42B1C', '#744DA9', '#038387', '#E74856', '#10893E'];
  const key = account?.email ?? '';
  let hash = 0;
  for (let i = 0; i < key.length; i++) hash = (hash * 31 + key.charCodeAt(i)) | 0;
  return palette[Math.abs(hash) % palette.length];
}

/** 从 flags 里挑出分类,用于图标映射 */
export function kindOf(folder: Folder | undefined): FolderKind | null {
  if (!folder) return null;
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

export const currentFolder = () =>
  selection.accountId ? (folders.value[selection.accountId] ?? []).find((f) => f.fullName === selection.folder) : undefined;

export const currentAccount = () =>
  accounts.value.find((a) => a.id === selection.accountId) ?? null;

export function resetMailView() {
  messages.value = [];
  selectedMessageId.value = null;
  messageDetail.value = null;
}
