<template>
  <section class="settings-pane">
    <!-- 顶部返回 -->
    <header class="sp-header">
      <button
        type="button"
        class="sp-btn"
        @click="emit('close')">
        <span class="icon-glyph" aria-hidden="true">&#xE72B;</span>
        {{ t('action.back') }}
      </button>
      <h2 class="sp-title">{{ t('settings.title') }}</h2>
    </header>

    <div class="sp-scroll">
      <!-- 外观 -->
      <section class="sp-section">
        <h3 class="sp-section-title">{{ t('settings.appearance') }}</h3>

        <div class="sp-group">
          <span class="sp-group-title">{{ t('settings.theme') }}</span>
          <label class="sp-radio">
            <input v-model="theme" type="radio" value="light" />
            <span class="sp-radio-dot" aria-hidden="true"></span>
            {{ t('settings.light') }}
          </label>
          <label class="sp-radio">
            <input v-model="theme" type="radio" value="dark" />
            <span class="sp-radio-dot" aria-hidden="true"></span>
            {{ t('settings.dark') }}
          </label>
          <label class="sp-radio">
            <input v-model="theme" type="radio" value="system" />
            <span class="sp-radio-dot" aria-hidden="true"></span>
            {{ t('settings.system') }}
          </label>
        </div>

        <div class="sp-group">
          <span class="sp-group-title">{{ t('settings.material') }}</span>
          <label class="sp-radio">
            <input v-model="material" type="radio" value="mica" />
            <span class="sp-radio-dot" aria-hidden="true"></span>
            {{ t('settings.mica') }}
          </label>
          <label class="sp-radio">
            <input v-model="material" type="radio" value="acrylic" />
            <span class="sp-radio-dot" aria-hidden="true"></span>
            {{ t('settings.acrylic') }}
          </label>
        </div>
      </section>

      <!-- 账号 -->
      <section class="sp-section">
        <h3 class="sp-section-title">{{ t('settings.accounts') }}</h3>
        <div v-if="accounts.length === 0" class="sp-empty">{{ t('empty.noAccounts') }}</div>
        <div v-for="acc in accounts" :key="acc.id" class="sp-account-row">
          <div class="sp-avatar" :style="{ backgroundColor: accountColorOf(acc) }">{{ initialOf(acc) }}</div>
          <div class="sp-account-info">
            <div class="sp-account-email">{{ acc.email }}</div>
            <div class="sp-account-provider">{{ providerLabel(acc.provider) }}</div>
          </div>
          <button
            type="button"
            class="sp-btn sp-delete-btn"
            :title="t('settings.accountDelete')"
            @click="confirmDelete(acc)">
            <span class="icon-glyph sp-delete-icon" aria-hidden="true">&#xE74D;</span>
          </button>
        </div>
      </section>

      <!-- 关于 -->
      <section class="sp-section">
        <h3 class="sp-section-title">{{ t('settings.about') }}</h3>
        <div class="sp-about-card">
          <p class="sp-about-version">{{ t('settings.aboutVersion', { version }) }}</p>
          <p class="sp-about-desc">{{ t('settings.aboutDesc') }}</p>
          <button
            type="button"
            class="sp-link"
            @click="onOpenOAuthDocs">
            <span class="icon-glyph" aria-hidden="true">&#xE8C7;</span>
            {{ t('settings.oauthDocs') }}
          </button>
        </div>
      </section>
    </div>

    <!-- 删除确认 -->
    <div v-if="deleteDialogOpen" class="sp-dialog-overlay" @click.self="deleteDialogOpen = false">
      <div class="sp-dialog">
        <h3 class="sp-dialog-title">{{ t('settings.accountDelete') }}</h3>
        <p class="sp-dialog-message">{{ deleteDialogMessage }}</p>
        <div class="sp-dialog-actions">
          <button type="button" class="sp-btn" @click="deleteDialogOpen = false">
            {{ t('action.cancel') }}
          </button>
          <button type="button" class="sp-btn sp-btn-danger" @click="doDelete">
            {{ t('action.delete') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 提示 -->
    <div v-if="infoOpen" class="sp-msg" :class="`is-${infoSeverity.toLowerCase()}`">
      <strong>{{ infoTitle }}</strong>
      <span>{{ infoMessage }}</span>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useI18n } from '../../components/i18n/index';
import * as state from '../state';
import * as actions from '../actions';
import { presetByProvider } from '../providers';
import type { AccountConfig } from '../api';
import { getVersion } from '@tauri-apps/api/app';
import { openPath } from '@tauri-apps/plugin-opener';

const { t } = useI18n();

const emit = defineEmits<{ (e: 'close'): void }>();

const accounts = computed(() => state.accounts.value);

/* ── 主题 / 材质 ── */
const theme = ref(localStorage.getItem('winui-theme-setting') ?? 'system');
const material = ref(localStorage.getItem('winui-material-setting') ?? 'mica');

function applyTheme(mode: string): void {
  const html = document.documentElement;
  html.classList.remove('theme-light', 'theme-dark');
  if (mode === 'light') html.classList.add('theme-light');
  else if (mode === 'dark') html.classList.add('theme-dark');
}

watch(theme, (v) => {
  localStorage.setItem('winui-theme-setting', v);
  applyTheme(v);
}, { immediate: true });

watch(material, (v) => {
  localStorage.setItem('winui-material-setting', v);
});

/* ── 账号删除 ── */
const deleteDialogOpen = ref(false);
const deleteDialogMessage = ref('');
let pendingDelete: AccountConfig | null = null;

function confirmDelete(acc: AccountConfig): void {
  pendingDelete = acc;
  deleteDialogMessage.value = t('settings.accountDeleteConfirm', { email: acc.email });
  deleteDialogOpen.value = true;
}

async function doDelete(): Promise<void> {
  const acc = pendingDelete;
  deleteDialogOpen.value = false;
  pendingDelete = null;
  if (acc) await actions.removeAccount(acc.id);
}

function accountColorOf(acc: AccountConfig): string {
  return state.accountColor(acc);
}

function initialOf(acc: AccountConfig): string {
  return (acc.name || acc.email).trim().charAt(0).toUpperCase();
}

function providerLabel(provider: string): string {
  const pres = presetByProvider(provider as Parameters<typeof presetByProvider>[0]);
  return t(pres.labelKey);
}

/* ── 关于 ── */
const version = ref('0.1.0');
onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch {
    // 非 Tauri 环境回退默认版本
  }
});

/* ── OAuth 文档 ── */
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

async function onOpenOAuthDocs(): Promise<void> {
  try {
    await openPath('docs/OAuth.md');
  } catch {
    showInfo(t('settings.oauthDocs'), 'docs/OAuth.md', 'Informational');
  }
}
</script>

<style scoped>
.settings-pane {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.sp-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
}

.sp-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.sp-scroll {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 24px 40px;
  max-width: 640px;
}

.sp-section { margin-bottom: 28px; }

.sp-section-title {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.sp-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}

.sp-group-title {
  font-size: 12px;
  color: var(--text-secondary);
}

.sp-radio {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
  user-select: none;
}

.sp-radio input { display: none; }

.sp-radio-dot {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  border: 1px solid var(--ctrl-strong-stroke, rgba(0, 0, 0, 0.45));
  border-radius: 50%;
  position: relative;
}

.sp-radio input:checked + .sp-radio-dot {
  border-color: var(--accent-base);
  border-width: 2px;
}

.sp-radio input:checked + .sp-radio-dot::after {
  content: '';
  position: absolute;
  inset: 3px;
  border-radius: 50%;
  background: var(--accent-base);
}

/* ── 按钮 ── */
.sp-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  box-sizing: border-box;
  padding: 4px 10px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--text-primary);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  transition: background var(--fast-duration, 0.15s) ease;
}

.sp-btn:hover { background: var(--subtle-secondary, rgba(0, 0, 0, 0.04)); }
.sp-btn:active { background: var(--subtle-tertiary, rgba(0, 0, 0, 0.06)); }

.sp-delete-btn { color: var(--text-secondary); }
.sp-delete-btn:hover { color: var(--SystemFillColorCriticalBrush, #c42b1c); }

.sp-btn-danger {
  border: 1px solid transparent;
  background: var(--SystemFillColorCriticalBrush, #c42b1c);
  color: #fff;
}

.sp-btn-danger:hover { background: rgba(196, 43, 28, 0.9); }

.sp-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0;
  border: 0;
  background: transparent;
  font: inherit;
  font-size: 13px;
  color: var(--accent-base);
  cursor: pointer;
}

.sp-link:hover { text-decoration: underline; }

/* ── 账号 ── */
.sp-empty {
  padding: 16px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
}

.sp-account-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 4px;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.04));
}

.sp-avatar {
  flex: 0 0 auto;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 14px;
  font-weight: 600;
}

.sp-account-info {
  flex: 1 1 auto;
  min-width: 0;
}

.sp-account-email {
  font-size: 13px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sp-account-provider {
  font-size: 12px;
  color: var(--text-tertiary);
}

/* ── 关于 ── */
.sp-about-card {
  padding: 12px 16px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 8px;
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
}

.sp-about-version {
  margin: 0 0 4px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.sp-about-desc {
  margin: 0 0 10px;
  font-size: 13px;
  color: var(--text-secondary);
}

/* ── 对话框 ── */
.sp-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.4);
}

.sp-dialog {
  width: 360px;
  max-width: calc(100vw - 48px);
  padding: 20px;
  border-radius: 8px;
  background: var(--flyout-bg, #fff);
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.2);
}

.sp-dialog-title {
  margin: 0 0 8px;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.sp-dialog-message {
  margin: 0 0 16px;
  font-size: 13px;
  line-height: 20px;
  color: var(--text-secondary);
}

.sp-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

/* ── 提示 ── */
.sp-msg {
  position: absolute;
  left: 16px;
  right: 16px;
  bottom: 16px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  border-radius: 6px;
  font-size: 13px;
  line-height: 18px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
}

.sp-msg strong { font-weight: 600; }

.sp-msg.is-success {
  color: var(--SystemFillColorSuccessBrush, #0f7b0f);
  background: var(--SystemFillColorSuccessBackgroundBrush, #dff6dd);
}

.sp-msg.is-error {
  color: var(--SystemFillColorCriticalBrush, #c42b1c);
  background: var(--SystemFillColorCriticalBackgroundBrush, #fde7e9);
}

.sp-msg.is-warning {
  color: var(--SystemFillColorCautionBrush, #9d5d00);
  background: var(--SystemFillColorCautionBackgroundBrush, #fff4ce);
}

.sp-msg.is-informational {
  color: var(--text-secondary);
  background: var(--subtle-secondary, rgba(0, 0, 0, 0.04));
}

.icon-glyph {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 14px;
  line-height: 1;
}
</style>
