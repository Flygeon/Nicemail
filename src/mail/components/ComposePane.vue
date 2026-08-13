<template>
  <div class="compose-pane">
    <!-- 顶部工具条 -->
    <header class="cp-toolbar">
      <h2 class="cp-title">{{ t('compose.title') }}</h2>
      <div class="cp-toolbar-actions">
        <span v-if="busy" class="cp-spinner" aria-hidden="true"></span>
        <button
          type="button"
          class="cp-btn"
          :disabled="busy"
          @click="onSaveDraft">
          {{ t('action.saveDraft') }}
        </button>
        <button
          type="button"
          class="cp-btn"
          :disabled="busy"
          @click="onClose">
          {{ t('action.cancel') }}
        </button>
        <button
          type="button"
          class="cp-btn cp-btn-accent"
          :disabled="busy"
          @click="onSend">
          <span class="icon-glyph" aria-hidden="true">&#xE724;</span>
          {{ busy ? t('compose.sending') : t('action.send') }}
        </button>
      </div>
    </header>

    <div class="cp-scroll">
      <!-- 发件账号 -->
      <label class="cp-field">
        <span class="cp-label">{{ t('compose.fromAccount') }}</span>
        <select v-model="fromAccountId" class="cp-input" :disabled="busy">
          <option v-for="acc in accounts" :key="acc.id" :value="acc.id">{{ acc.email }}</option>
        </select>
      </label>

      <!-- 收件人 -->
      <label class="cp-field">
        <span class="cp-label">{{ t('compose.to') }}</span>
        <input
          v-model="to"
          class="cp-input"
          type="text"
          :placeholder="t('compose.to')" />
      </label>
      <label v-if="showCc" class="cp-field">
        <span class="cp-label">{{ t('compose.cc') }}</span>
        <input v-model="cc" class="cp-input" type="text" :placeholder="t('compose.cc')" />
      </label>
      <label v-if="showBcc" class="cp-field">
        <span class="cp-label">{{ t('compose.bcc') }}</span>
        <input v-model="bcc" class="cp-input" type="text" :placeholder="t('compose.bcc')" />
      </label>
      <div v-if="!showCc || !showBcc" class="cp-add-row">
        <button v-if="!showCc" type="button" class="cp-add-link" @click="showCc = true">{{ t('compose.addCc') }}</button>
        <span v-if="!showCc && !showBcc" class="cp-add-sep" aria-hidden="true">·</span>
        <button v-if="!showBcc" type="button" class="cp-add-link" @click="showBcc = true">{{ t('compose.addBcc') }}</button>
      </div>

      <!-- 主题 -->
      <label class="cp-field">
        <span class="cp-label">{{ t('compose.subject') }}</span>
        <input
          v-model="subject"
          class="cp-input"
          type="text"
          :placeholder="t('compose.subject')" />
      </label>

      <!-- 正文 -->
      <label class="cp-field">
        <span class="cp-label">{{ t('compose.body') }}</span>
        <textarea
          v-model="body"
          class="cp-input cp-body"
          :placeholder="t('compose.body')"></textarea>
      </label>

      <!-- 附件 -->
      <div class="cp-attachments">
        <div
          v-for="(att, i) in attachments"
          :key="i"
          class="cp-attachment">
          <span class="icon-glyph cp-att-icon" aria-hidden="true">&#xE8B9;</span>
          <span class="cp-att-name">{{ att.filename }}</span>
          <button
            type="button"
            class="cp-att-remove"
            :aria-label="t('action.remove')"
            @click="removeAttachment(i)">
            <span class="icon-glyph" aria-hidden="true">&#xE711;</span>
          </button>
        </div>
        <button
          type="button"
          class="cp-btn"
          :disabled="busy"
          @click="onAddAttachment">
          <span class="icon-glyph" aria-hidden="true">&#xE710;</span>
          {{ t('action.addAttachment') }}
        </button>
      </div>

      <!-- 提示 -->
      <div v-if="infoOpen" class="cp-msg" :class="`is-${infoSeverity.toLowerCase()}`">
        <strong>{{ infoTitle }}</strong>
        <span>{{ infoMessage }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from '../../components/i18n/index';
import * as state from '../state';
import * as actions from '../actions';
import * as api from '../api';
import type { SendAttachment } from '../api';
import { open } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const accounts = computed(() => state.accounts.value);
const fromAccountId = ref('');
const fromAccount = computed(() => accounts.value.find((a) => a.id === fromAccountId.value) ?? null);

const to = ref('');
const cc = ref('');
const bcc = ref('');
const subject = ref('');
const body = ref('');
const showCc = ref(false);
const showBcc = ref(false);
const attachments = ref<SendAttachment[]>([]);
const busy = ref(false);

const infoOpen = ref(false);
const infoTitle = ref('');
const infoMessage = ref('');
const infoSeverity = ref<'Informational' | 'Success' | 'Warning' | 'Error'>('Informational');

function showInfo(title: string, message: string, severity: 'Informational' | 'Success' | 'Warning' | 'Error'): void {
  infoTitle.value = title;
  infoMessage.value = message;
  infoSeverity.value = severity;
  infoOpen.value = true;
}

onMounted(() => {
  const initial = state.composeInitial.value;
  if (initial) {
    if (initial.to) to.value = initial.to;
    if (initial.cc) cc.value = initial.cc;
    if (initial.bcc) bcc.value = initial.bcc;
    if (initial.subject) subject.value = initial.subject;
    if (initial.body) body.value = initial.body;
    if (initial.cc) showCc.value = true;
    if (initial.bcc) showBcc.value = true;
  }
  const pre = initial?.accountId
    ? accounts.value.find((a) => a.id === initial.accountId)
    : undefined;
  fromAccountId.value = (pre ?? state.currentAccount() ?? accounts.value[0] ?? null)?.id ?? '';
  state.composeInitial.value = null;
});

function parseRecipients(value: string): string[] {
  return value
    .split(/[,;，；]+\s*/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function buildRequest(): api.SendRequest | null {
  const acc = fromAccount.value;
  if (!acc) return null;
  return {
    accountId: acc.id,
    to: parseRecipients(to.value),
    cc: parseRecipients(cc.value),
    bcc: parseRecipients(bcc.value),
    subject: subject.value,
    bodyHtml: '',
    bodyText: body.value,
    attachments: attachments.value,
  };
}

async function onSend(): Promise<void> {
  const req = buildRequest();
  if (!req) return;
  if (req.to.length === 0) {
    showInfo(t('status.warning'), t('compose.noRecipients'), 'Warning');
    return;
  }
  busy.value = true;
  try {
    await api.mailSend(req);
    actions.closeCompose();
  } catch (err) {
    showInfo(t('status.error'), t('compose.sendFailed', { message: String(err) }), 'Error');
  } finally {
    busy.value = false;
  }
}

async function onSaveDraft(): Promise<void> {
  const acc = fromAccount.value;
  const req = buildRequest();
  if (!acc || !req) return;
  busy.value = true;
  try {
    await api.mailSaveDraft(acc.id, req);
    showInfo(t('status.success'), t('compose.draftSaved'), 'Success');
  } catch (err) {
    showInfo(t('status.error'), t('compose.draftFailed', { message: String(err) }), 'Error');
  } finally {
    busy.value = false;
  }
}

function onClose(): void {
  actions.closeCompose();
}

async function onAddAttachment(): Promise<void> {
  const result = await open({ multiple: true });
  if (!result) return;
  const paths = Array.isArray(result) ? result : [result];
  const newAtts: SendAttachment[] = paths.map((p) => ({ path: p, filename: basename(p), mime: null }));
  attachments.value = [...attachments.value, ...newAtts];
}

function removeAttachment(index: number): void {
  attachments.value = attachments.value.filter((_, i) => i !== index);
}

/** 兼容 win 与 unix 路径分隔符的 basename */
function basename(p: string): string {
  const parts = p.split(/[\\/]/);
  return parts[parts.length - 1] || p;
}
</script>

<style scoped>
.compose-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--layer-default, #f3f3f3);
}

/* ── 工具条 ── */
.cp-toolbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
}

.cp-title {
  flex: 1 1 auto;
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cp-toolbar-actions {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 6px;
}

.cp-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  box-sizing: border-box;
  padding: 4px 12px;
  border: 1px solid var(--ctrl-border-rest, rgba(0, 0, 0, 0.06));
  border-radius: 4px;
  background: var(--ctrl-fill-default, rgba(255, 255, 255, 0.7));
  color: var(--text-primary);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  transition: background var(--fast-duration, 0.15s) ease;
}

.cp-btn:hover:not(:disabled) { background: var(--subtle-secondary, rgba(0, 0, 0, 0.04)); }
.cp-btn:active:not(:disabled) { background: var(--subtle-tertiary, rgba(0, 0, 0, 0.06)); }
.cp-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.cp-btn-accent {
  border-color: transparent;
  background: var(--accent-base);
  color: var(--accent-text, #fff);
}

.cp-btn-accent:hover:not(:disabled) { background: var(--accent-hover, rgba(0, 103, 192, 0.9)); }
.cp-btn-accent:active:not(:disabled) { background: var(--accent-pressed, rgba(0, 103, 192, 0.8)); }

.cp-spinner {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  border: 2px solid var(--subtle-secondary, rgba(0, 0, 0, 0.1));
  border-top-color: var(--accent-base);
  border-radius: 50%;
  animation: cp-spin 0.8s linear infinite;
}

@keyframes cp-spin {
  to { transform: rotate(360deg); }
}

/* ── 表单区 ── */
.cp-scroll {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 12px 16px 24px;
}

.cp-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
}

.cp-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.cp-input {
  box-sizing: border-box;
  width: 100%;
  padding: 5px 10px;
  border: 1px solid var(--ctrl-border-rest, rgba(0, 0, 0, 0.06));
  border-radius: 4px;
  background: var(--ctrl-fill-input-active, #fff);
  color: var(--text-primary);
  font: inherit;
  font-size: 13px;
  line-height: 20px;
  outline: none;
  transition: border-color var(--fast-duration, 0.15s) ease;
}

.cp-input:focus {
  border-color: var(--accent-base);
  box-shadow: 0 0 0 1px var(--accent-base);
}

.cp-body {
  min-height: 180px;
  resize: vertical;
  line-height: 22px;
}

.cp-add-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: -4px 0 12px;
  padding-left: 4px;
}

.cp-add-link {
  border: 0;
  background: transparent;
  padding: 0;
  font: inherit;
  font-size: 12px;
  color: var(--accent-base);
  cursor: pointer;
}

.cp-add-link:hover { text-decoration: underline; }

.cp-add-sep { color: var(--text-tertiary); }

/* ── 附件 ── */
.cp-attachments {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  margin-top: 4px;
}

.cp-attachment {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 5px 10px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
}

.cp-att-icon { color: var(--accent-base); }

.cp-att-name {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 13px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cp-att-remove {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
}

.cp-att-remove:hover {
  background: var(--subtle-secondary, rgba(0, 0, 0, 0.04));
  color: var(--text-primary);
}

/* ── 提示 ── */
.cp-msg {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: 6px;
  font-size: 13px;
  line-height: 18px;
}

.cp-msg strong { font-weight: 600; }

.cp-msg.is-success {
  color: var(--SystemFillColorSuccessBrush, #0f7b0f);
  background: var(--SystemFillColorSuccessBackgroundBrush, #dff6dd);
}

.cp-msg.is-error {
  color: var(--SystemFillColorCriticalBrush, #c42b1c);
  background: var(--SystemFillColorCriticalBackgroundBrush, #fde7e9);
}

.cp-msg.is-warning {
  color: var(--SystemFillColorCautionBrush, #9d5d00);
  background: var(--SystemFillColorCautionBackgroundBrush, #fff4ce);
}

.cp-msg.is-informational {
  color: var(--text-secondary);
  background: var(--subtle-secondary, rgba(0, 0, 0, 0.04));
}

.icon-glyph {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 14px;
  line-height: 1;
}
</style>
