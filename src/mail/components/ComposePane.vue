<template>
  <div class="compose-pane">
    <!-- 顶部工具条 -->
    <header class="cp-toolbar">
      <h2 class="cp-title">{{ t('compose.title') }}</h2>
      <div class="cp-toolbar-actions">
        <WinProgressRing
          v-if="busy"
          class="cp-progress"
          :IsActive="true"
          :IsIndeterminate="true" />
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :IsEnabled="!busy"
          @Click="onSaveDraft">
          {{ t('action.saveDraft') }}
        </WinButton>
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :IsEnabled="!busy"
          @Click="onClose">
          {{ t('action.cancel') }}
        </WinButton>
        <WinButton
          Style="{StaticResource AccentButtonStyle}"
          :IsEnabled="!busy"
          @Click="onSend">
          <span class="icon-glyph" aria-hidden="true">&#xE724;</span>
          {{ busy ? t('compose.sending') : t('action.send') }}
        </WinButton>
      </div>
    </header>

    <div class="cp-scroll">
      <!-- 发件账号 -->
      <div class="cp-field">
        <WinComboBox
          v-model:SelectedItem="fromAccount"
          :ItemsSource="accounts"
          DisplayMemberPath="email"
          :Header="t('compose.fromAccount')" />
      </div>

      <!-- 收件人 -->
      <div class="cp-field">
        <WinTextBox
          v-model:Text="to"
          :Header="t('compose.to')"
          :PlaceholderText="t('compose.to')" />
      </div>
      <div v-if="showCc" class="cp-field">
        <WinTextBox
          v-model:Text="cc"
          :Header="t('compose.cc')"
          :PlaceholderText="t('compose.cc')" />
      </div>
      <div v-if="showBcc" class="cp-field">
        <WinTextBox
          v-model:Text="bcc"
          :Header="t('compose.bcc')"
          :PlaceholderText="t('compose.bcc')" />
      </div>
      <div v-if="!showCc || !showBcc" class="cp-add-row">
        <button v-if="!showCc" type="button" class="cp-add-link" @click="showCc = true">{{ t('compose.addCc') }}</button>
        <span v-if="!showCc && !showBcc" class="cp-add-sep" aria-hidden="true">·</span>
        <button v-if="!showBcc" type="button" class="cp-add-link" @click="showBcc = true">{{ t('compose.addBcc') }}</button>
      </div>

      <!-- 主题 -->
      <div class="cp-field">
        <WinTextBox
          v-model:Text="subject"
          :Header="t('compose.subject')"
          :PlaceholderText="t('compose.subject')" />
      </div>

      <!-- 正文 -->
      <div class="cp-field cp-body-field">
        <WinTextBox
          v-model:Text="body"
          :Header="t('compose.body')"
          :PlaceholderText="t('compose.body')"
          :AcceptsReturn="true"
          :TextWrapping="'Wrap'" />
      </div>

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
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :IsEnabled="!busy"
          @Click="onAddAttachment">
          <span class="icon-glyph" aria-hidden="true">&#xE710;</span>
          {{ t('action.addAttachment') }}
        </WinButton>
      </div>

      <WinInfoBar
        v-model:IsOpen="infoOpen"
        :Title="infoTitle"
        :Message="infoMessage"
        :Severity="infoSeverity"
        @Close="infoOpen = false" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue';
import { useI18n } from '../../components/i18n/index';
import * as state from '../state';
import * as actions from '../actions';
import * as api from '../api';
import type { SendAttachment } from '../api';
import { open } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const accounts = computed(() => state.accounts.value);
const fromAccount = ref<api.AccountConfig | null>(null);

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
  if (infoOpen.value) {
    infoOpen.value = false;
    void nextTick(() => { infoOpen.value = true; });
  } else {
    infoOpen.value = true;
  }
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
  fromAccount.value = pre ?? state.currentAccount() ?? accounts.value[0] ?? null;
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

.cp-progress {
  width: 20px;
  height: 20px;
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
  margin-bottom: 12px;
}

.cp-body-field :deep(.win-textbox-textarea) {
  min-height: 180px;
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

.cp-add-link:hover {
  text-decoration: underline;
}

.cp-add-sep {
  color: var(--text-tertiary);
}

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

.cp-att-icon {
  color: var(--accent-base);
}

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

.icon-glyph {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 14px;
  line-height: 1;
}
</style>
