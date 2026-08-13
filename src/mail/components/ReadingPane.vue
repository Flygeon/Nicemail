<template>
  <section class="reading-pane">
    <template v-if="detail">
      <!-- 操作条 -->
      <header class="rp-toolbar">
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :title="t('nav.starred')"
          @Click="onToggleStar">
          <span v-if="isStarred" class="icon-glyph is-on" aria-hidden="true">&#xE734;</span>
          <span v-else class="icon-glyph" aria-hidden="true">&#xE735;</span>
        </WinButton>
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :title="t('action.delete')"
          @Click="onDelete">
          <span class="icon-glyph" aria-hidden="true">&#xE74D;</span>
        </WinButton>
        <div class="rp-toolbar-sep" aria-hidden="true"></div>
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :title="t('action.reply')"
          @Click="onReply">
          <span class="icon-glyph" aria-hidden="true">&#xE97A;</span>
        </WinButton>
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :title="t('action.replyAll')"
          @Click="onReplyAll">
          <span class="icon-glyph" aria-hidden="true">&#xE8C2;</span>
        </WinButton>
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :title="t('action.forward')"
          @Click="onForward">
          <span class="icon-glyph" aria-hidden="true">&#xE8AB;</span>
        </WinButton>
      </header>

      <div ref="scrollEl" class="rp-scroll">
        <!-- 发件人卡片 -->
        <div class="rp-sender-card">
          <div class="rp-avatar" :style="{ backgroundColor: avatarColor }">{{ avatarLetter }}</div>
          <div class="rp-sender-info">
            <div class="rp-from-name">{{ detail.fromName || detail.fromEmail }}</div>
            <div class="rp-from-email">{{ detail.fromEmail }}</div>
          </div>
        </div>

        <!-- 元信息 -->
        <div class="rp-meta">
          <div v-if="detail.toEmails.length" class="rp-meta-line">
            <span class="rp-meta-label">{{ t('mail.to') }}:</span>
            <span class="rp-meta-value">{{ detail.toEmails.join(', ') }}</span>
          </div>
          <div v-if="detail.ccEmails && detail.ccEmails.length" class="rp-meta-line">
            <span class="rp-meta-label">{{ t('mail.cc') }}:</span>
            <span class="rp-meta-value">{{ detail.ccEmails.join(', ') }}</span>
          </div>
          <div class="rp-meta-line">
            <span class="rp-meta-label">{{ t('mail.date') }}:</span>
            <span class="rp-meta-value">{{ formatFullDate(detail.date) }}</span>
          </div>
        </div>

        <!-- 远程图片拦截横幅 -->
        <div v-if="hasRemoteBlocked" class="rp-remote-banner">
          <span class="icon-glyph rp-remote-icon" aria-hidden="true">&#xE7B3;</span>
          <span class="rp-remote-text">{{ t('mail.remoteBlocked') }}</span>
          <WinButton
            Style="{StaticResource SubtleButtonStyle}"
            @Click="showRemoteImages">
            {{ t('mail.showRemote') }}
          </WinButton>
        </div>

        <!-- 正文 -->
        <div
          v-if="detail.html"
          ref="bodyRef"
          class="rp-body"
          :class="{ 'remote-blocked': hasRemoteBlocked }"
          v-html="bodyHtml"
          @click="onBodyClick"></div>
        <div v-else-if="detail.text" class="rp-body rp-body-text">{{ detail.text }}</div>
        <div v-else class="rp-body rp-body-text">{{ t('mail.noBody') }}</div>

        <!-- 附件 -->
        <div v-if="detail.attachments.length" class="rp-attachments">
          <h3 class="rp-attachments-title">{{ t('mail.attachments', { count: detail.attachments.length }) }}</h3>
          <div class="rp-attachment-list">
            <button
              v-for="att in detail.attachments"
              :key="att.index"
              class="rp-attachment"
              type="button"
              :title="t('mail.downloadAttachment')"
              @click="onSaveAttachment(att)">
              <span class="icon-glyph rp-att-icon" aria-hidden="true">&#xE8B9;</span>
              <span class="rp-att-name">{{ att.filename }}</span>
              <span class="rp-att-size">{{ formatSize(att.size) }}</span>
            </button>
          </div>
        </div>
      </div>

      <WinInfoBar
        v-model:IsOpen="infoOpen"
        :Title="infoTitle"
        :Message="infoMessage"
        :Severity="infoSeverity"
        @Close="infoOpen = false" />
    </template>

    <!-- 空态 -->
    <div v-else class="mail-empty">
      <div class="mail-empty-inner">
        <span class="mail-empty-icon" aria-hidden="true">&#xE8D7;</span>
        <p>{{ t('empty.selectMessage') }}</p>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { useI18n } from '../../components/i18n/index';
import * as state from '../state';
import * as actions from '../actions';
import * as api from '../api';
import { sanitizeHtml } from '../sanitize';
import type { AttachmentMeta } from '../api';
import { save } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';

const { t, locale } = useI18n();

const detail = computed(() => state.messageDetail.value);
const avatarColor = computed(() => state.accountColor(state.currentAccount()));
const avatarLetter = computed(() => {
  const d = detail.value;
  if (!d) return '';
  const name = d.fromName || d.fromEmail;
  return name.trim().charAt(0).toUpperCase();
});

const isStarred = computed(() => {
  const d = detail.value;
  if (!d) return false;
  return state.messages.value.find((m) => m.id === d.id)?.starred ?? false;
});

/* ── 信息条 ── */
const infoOpen = ref(false);
const infoTitle = ref('');
const infoMessage = ref('');
const infoSeverity = ref<'Informational' | 'Success' | 'Warning' | 'Error'>('Informational');

function showInfo(title: string, message: string, severity: 'Informational' | 'Success' | 'Warning' | 'Error'): void {
  infoTitle.value = title;
  infoMessage.value = message;
  infoSeverity.value = severity;
  if (infoOpen.value) {
    infoOpen.value = false;
    void nextTick(() => { infoOpen.value = true; });
  } else {
    infoOpen.value = true;
  }
}

/* ── 正文处理:净化 + 内嵌图 + 远程图片拦截 ── */
const bodyRef = ref<HTMLElement | null>(null);
const remoteBlockedCount = ref(0);
const hasRemoteBlocked = computed(() => remoteBlockedCount.value > 0);

const bodyHtml = computed(() => {
  const d = detail.value;
  if (!d || !d.html) return '';
  let html = sanitizeHtml(d.html);
  // 把 cid:xxx 替换成内嵌 data URL
  for (const [cid, dataUrl] of Object.entries(d.embedded ?? {})) {
    html = html.split(`cid:${cid}`).join(dataUrl);
  }
  // 用 DOM 精确拦截 http(s) 远程图片:src 移到 data-remote
  const wrapper = document.createElement('div');
  wrapper.innerHTML = html;
  remoteBlockedCount.value = 0;
  const imgs = Array.from(wrapper.querySelectorAll('img'));
  for (const img of imgs) {
    const src = img.getAttribute('src') ?? '';
    if (/^https?:\/\//i.test(src)) {
      img.setAttribute('data-remote', src);
      img.removeAttribute('src');
      remoteBlockedCount.value++;
    }
  }
  return wrapper.innerHTML;
});

function showRemoteImages(): void {
  const el = bodyRef.value;
  if (el) {
    el.querySelectorAll('img[data-remote]').forEach((img) => {
      const src = img.getAttribute('data-remote');
      if (src) img.setAttribute('src', src);
      img.removeAttribute('data-remote');
    });
  }
  remoteBlockedCount.value = 0;
}

/* ── 正文链接拦截,交给系统浏览器 ── */
function onBodyClick(e: MouseEvent): void {
  const target = e.target as HTMLElement;
  const anchor = target.closest?.('a');
  if (!anchor) return;
  const href = anchor.getAttribute('href');
  if (!href) return;
  if (/^(https?:|mailto:)/i.test(href)) {
    e.preventDefault();
    void openUrl(href).catch(() => { /* 打开失败静默 */ });
  }
}

/* ── 操作 ── */
function onToggleStar(): void {
  const d = detail.value;
  if (d) void actions.toggleStar(d);
}

function onDelete(): void {
  void actions.deleteSelected();
}

function onReply(): void {
  const d = detail.value;
  if (!d) return;
  state.composeInitial.value = {
    to: formatRecipient(d.fromEmail, d.fromName),
    subject: t('mail.replySubject', { subject: d.subject || t('mail.noSubject') }),
    accountId: d.accountId,
  };
  actions.openCompose();
}

function onReplyAll(): void {
  const d = detail.value;
  if (!d) return;
  const ownEmail = state.currentAccount()?.email ?? '';
  const toList: string[] = [formatRecipient(d.fromEmail, d.fromName)];
  for (const addr of d.toEmails ?? []) {
    if (addr.toLowerCase() !== ownEmail.toLowerCase() && !toList.includes(addr)) toList.push(addr);
  }
  const ccList = (d.ccEmails ?? []).filter((a) => a.toLowerCase() !== ownEmail.toLowerCase());
  state.composeInitial.value = {
    to: toList.join(', '),
    cc: ccList.join(', '),
    subject: t('mail.replySubject', { subject: d.subject || t('mail.noSubject') }),
    accountId: d.accountId,
  };
  actions.openCompose();
}

function onForward(): void {
  const d = detail.value;
  if (!d) return;
  const lines = [
    '',
    '---------- Forwarded message ----------',
    `From: ${d.fromName} <${d.fromEmail}>`,
    `Date: ${new Date(d.date).toLocaleString(locale)}`,
    `Subject: ${d.subject || ''}`,
    '',
    d.text ?? '',
  ];
  state.composeInitial.value = {
    subject: t('mail.forwardSubject', { subject: d.subject || t('mail.noSubject') }),
    body: lines.join('\n'),
    accountId: d.accountId,
  };
  actions.openCompose();
}

function formatRecipient(email: string, name?: string): string {
  if (!name || name === email) return email;
  return `${name} <${email}>`;
}

/* ── 附件下载 ── */
async function onSaveAttachment(att: AttachmentMeta): Promise<void> {
  const d = detail.value;
  if (!d) return;
  try {
    const dest = await save({ defaultPath: att.filename });
    if (!dest) return;
    await api.mailAttachmentSave(d.accountId, d.folder, d.uid, att.index, dest);
    showInfo(t('status.success'), t('mail.downloadAttachment'), 'Success');
  } catch (err) {
    showInfo(t('status.error'), String(err), 'Error');
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatFullDate(ts: number): string {
  return new Date(ts).toLocaleString(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    weekday: 'short',
    hour: '2-digit',
    minute: '2-digit',
  });
}
</script>

<style scoped>
.reading-pane {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

/* ── 操作条 ── */
.rp-toolbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
}

.rp-toolbar-sep {
  width: 1px;
  height: 20px;
  margin: 0 6px;
  background: var(--stroke-divider, rgba(0, 0, 0, 0.06));
}

.icon-glyph.is-on {
  color: var(--accent-base);
}

/* ── 滚动区 ── */
.rp-scroll {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 16px 20px 24px;
}

/* ── 发件人卡片 ── */
.rp-sender-card {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.rp-avatar {
  flex: 0 0 auto;
  width: 40px;
  height: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  color: #fff;
  font-size: 17px;
  font-weight: 600;
  user-select: none;
}

.rp-sender-info {
  min-width: 0;
}

.rp-from-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rp-from-email {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── 元信息 ── */
.rp-meta {
  margin-bottom: 12px;
  padding: 8px 12px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--card-bg-secondary, rgba(246, 246, 246, 0.5));
}

.rp-meta-line {
  display: flex;
  gap: 6px;
  font-size: 12px;
  line-height: 18px;
}

.rp-meta-label {
  flex: 0 0 auto;
  color: var(--text-tertiary);
}

.rp-meta-value {
  min-width: 0;
  color: var(--text-secondary);
  word-break: break-word;
}

/* ── 远程图片拦截横幅 ── */
.rp-remote-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding: 6px 10px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--card-bg-secondary, rgba(246, 246, 246, 0.5));
}

.rp-remote-icon {
  color: var(--text-tertiary);
}

.rp-remote-text {
  flex: 1 1 auto;
  font-size: 12px;
  color: var(--text-secondary);
}

/* ── 正文 ── */
.rp-body {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary);
  overflow-wrap: break-word;
}

.rp-body :deep(a) {
  color: var(--accent-base);
}

.rp-body :deep(img) {
  max-width: 100%;
  height: auto;
}

.rp-body-text {
  white-space: pre-wrap;
  color: var(--text-secondary);
}

/* 只隐藏被拦截(无 src)的远程图片,保留内嵌 data URL 图片 */
.rp-body.remote-blocked :deep(img:not([src])) {
  visibility: hidden;
}

/* ── 附件 ── */
.rp-attachments {
  margin-top: 20px;
  padding-top: 12px;
  border-top: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
}

.rp-attachments-title {
  margin: 0 0 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.rp-attachment-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.rp-attachment {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
  color: var(--text-primary);
  cursor: pointer;
  font: inherit;
  text-align: left;
  transition: background var(--fast-duration, 0.15s) var(--fast-out-slow-in, ease-out);
}

.rp-attachment:hover {
  background: var(--subtle-secondary, rgba(0, 0, 0, 0.04));
}

.rp-attachment:active {
  background: var(--subtle-tertiary, rgba(0, 0, 0, 0.06));
}

.rp-att-icon {
  color: var(--accent-base);
}

.rp-att-name {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rp-att-size {
  flex: 0 0 auto;
  font-size: 12px;
  color: var(--text-tertiary);
}

.icon-glyph {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 14px;
  line-height: 1;
}
</style>
