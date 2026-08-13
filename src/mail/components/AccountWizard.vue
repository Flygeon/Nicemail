<template>
  <div class="wizard-overlay" @pointerdown.self="onOverlayClick">
    <div class="wizard">
      <header class="wz-header">
        <h2 class="wz-title">{{ t('wizard.title') }}</h2>
        <button
          type="button"
          class="wz-close"
          :aria-label="t('action.cancel')"
          @click="emit('close')">
          <span class="icon-glyph" aria-hidden="true">&#xE711;</span>
        </button>
      </header>

      <div class="wz-body">
        <!-- 第一步:选择服务商 -->
        <template v-if="step === 1">
          <p class="wz-step-title">{{ t('wizard.provider') }}</p>
          <div class="wz-providers">
            <button
              v-for="p in allProviders"
              :key="p.key"
              type="button"
              class="wz-provider"
              @click="selectProvider(p)">
              <span class="wz-provider-dot" :style="{ backgroundColor: p.brandColor }" aria-hidden="true"></span>
              <span class="wz-provider-label">{{ t(p.labelKey) }}</span>
            </button>
          </div>
        </template>

        <!-- 第二步:填写信息 -->
        <template v-else>
          <div class="wz-back-row">
            <button type="button" class="wz-back" @click="step = 1">
              <span class="icon-glyph" aria-hidden="true">&#xE72B;</span>
              {{ t('action.back') }}
            </button>
          </div>

          <label class="wz-field">
            <span class="wz-label">{{ t('wizard.name') }}</span>
            <input
              class="wz-input"
              v-model="name"
              type="text"
              :placeholder="t('wizard.namePlaceholder')" />
          </label>

          <label class="wz-field">
            <span class="wz-label">{{ t('wizard.email') }}</span>
            <input
              class="wz-input"
              v-model="email"
              type="text"
              :placeholder="t('wizard.emailPlaceholder')" />
          </label>

          <!-- 密码/授权码 -->
          <template v-if="preset.auth === 'password'">
            <label class="wz-field">
              <span class="wz-label">{{ preset.key === '163' || preset.key === '126' || preset.key === 'qq' ? t('wizard.password163') : t('wizard.password') }}</span>
              <input
                class="wz-input"
                v-model="password"
                type="password"
                :placeholder="t('wizard.passwordPlaceholder')" />
            </label>
            <p class="wz-hint">{{ t(preset.hintKey) }}</p>
          </template>

          <!-- OAuth2 -->
          <template v-else>
            <p class="wz-hint">{{ t(preset.hintKey) }}</p>
            <button
              type="button"
              class="wz-btn wz-btn-accent"
              :disabled="oauthWaiting"
              @click="startOAuth">
              <span class="icon-glyph" aria-hidden="true">&#xE8D7;</span>
              {{ t('wizard.oauthLogin', { provider: t(preset.labelKey) }) }}
            </button>
            <div v-if="!oauthConfigured" class="wz-msg wz-msg-warn">
              <strong>{{ t('wizard.oauthNotConfigured') }}</strong>
              <span>{{ t('wizard.oauthNotConfiguredHint') }}</span>
            </div>
          </template>

          <!-- 自定义服务器 -->
          <template v-if="preset.key === 'custom'">
            <div class="wz-grid">
              <label class="wz-field">
                <span class="wz-label">{{ t('wizard.customImap') }}</span>
                <input class="wz-input" v-model="imapHost" type="text" :placeholder="t('wizard.customImapPlaceholder')" />
              </label>
              <label class="wz-field">
                <span class="wz-label">{{ `${t('wizard.customImap')} ${t('wizard.customPort')}` }}</span>
                <input class="wz-input" v-model="imapPortStr" type="number" :placeholder="t('wizard.customPort')" />
              </label>
            </div>
            <label class="wz-toggle">
              <input v-model="imapSsl" type="checkbox" />
              <span class="wz-toggle-track" aria-hidden="true"></span>
              <span class="wz-toggle-label">{{ t('wizard.ssl') }}</span>
            </label>
            <div class="wz-grid">
              <label class="wz-field">
                <span class="wz-label">{{ t('wizard.customSmtp') }}</span>
                <input class="wz-input" v-model="smtpHost" type="text" :placeholder="t('wizard.customSmtpPlaceholder')" />
              </label>
              <label class="wz-field">
                <span class="wz-label">{{ `${t('wizard.customSmtp')} ${t('wizard.customPort')}` }}</span>
                <input class="wz-input" v-model="smtpPortStr" type="number" :placeholder="t('wizard.customPort')" />
              </label>
            </div>
            <label class="wz-toggle">
              <input v-model="smtpSsl" type="checkbox" />
              <span class="wz-toggle-track" aria-hidden="true"></span>
              <span class="wz-toggle-label">{{ t('wizard.ssl') }}</span>
            </label>
          </template>

          <!-- 测试结果 -->
          <div v-if="testResult" class="wz-test-result" :class="testResult.ok ? 'is-ok' : 'is-fail'">
            {{ testResult.ok ? t('wizard.testSuccess') : t('wizard.testFailed', { message: testResult.message }) }}
          </div>

          <!-- 底部动作 -->
          <div v-if="preset.auth === 'password'" class="wz-actions">
            <button
              type="button"
              class="wz-btn"
              :disabled="testing || adding"
              @click="onTest">
              <span class="icon-glyph" aria-hidden="true">&#xE72C;</span>
              {{ testing ? t('wizard.testing') : t('wizard.testConnection') }}
            </button>
            <button
              type="button"
              class="wz-btn wz-btn-accent"
              :disabled="testing || adding"
              @click="onAdd">
              <span class="icon-glyph" aria-hidden="true">&#xE710;</span>
              {{ t('action.add') }}
            </button>
          </div>

          <!-- 错误提示 -->
          <div v-if="errorInfoOpen" class="wz-msg wz-msg-error">
            <strong>{{ t('status.error') }}</strong>
            <span>{{ errorMessage }}</span>
          </div>
        </template>
      </div>

      <!-- OAuth 等待遮罩 -->
      <div v-if="oauthWaiting" class="wz-oauth-wait">
        <span class="wz-spinner" aria-hidden="true"></span>
        <p class="wz-oauth-title">{{ t('wizard.oauthTitle') }}</p>
        <p class="wz-oauth-hint">{{ t('wizard.oauthWait') }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from '../../components/i18n/index';
import * as actions from '../actions';
import * as api from '../api';
import { PROVIDERS, CUSTOM_PRESET, presetByProvider } from '../providers';
import type { ProviderPreset } from '../providers';
import { openUrl, openPath } from '@tauri-apps/plugin-opener';
import type { UnlistenFn } from '@tauri-apps/api/event';

const { t } = useI18n();

const emit = defineEmits<{ (e: 'close'): void }>();

const allProviders: ProviderPreset[] = [...PROVIDERS, CUSTOM_PRESET];

const step = ref(1);
const preset = ref<ProviderPreset>(CUSTOM_PRESET);

/* ── 表单字段 ── */
const name = ref('');
const email = ref('');
const password = ref('');
const imapHost = ref('');
const imapPortStr = ref(String(CUSTOM_PRESET.imapPort));
const imapSsl = ref(CUSTOM_PRESET.imapSsl);
const smtpHost = ref('');
const smtpPortStr = ref(String(CUSTOM_PRESET.smtpPort));
const smtpSsl = ref(CUSTOM_PRESET.smtpSsl);

/* ── 测试 / 添加 / OAuth 状态 ── */
const testing = ref(false);
const adding = ref(false);
const testResult = ref<api.TestResult | null>(null);
const oauthWaiting = ref(false);
const oauthConfig = ref<api.OAuthConfig | null>(null);
const errorInfoOpen = ref(false);
const errorMessage = ref('');

const oauthConfigured = computed(() => {
  const cfg = oauthConfig.value;
  if (!cfg) return false;
  if (preset.value.key === 'gmail') return cfg.google.configured;
  if (preset.value.key === 'outlook') return cfg.outlook.configured;
  return false;
});

const canAdd = computed(() => !testing.value && !adding.value);

function selectProvider(p: ProviderPreset): void {
  preset.value = p;
  if (p.key === 'custom') {
    imapHost.value = '';
    imapPortStr.value = String(CUSTOM_PRESET.imapPort);
    imapSsl.value = CUSTOM_PRESET.imapSsl;
    smtpHost.value = '';
    smtpPortStr.value = String(CUSTOM_PRESET.smtpPort);
    smtpSsl.value = CUSTOM_PRESET.smtpSsl;
  }
  testResult.value = null;
  step.value = 2;
}

function validate(): boolean {
  if (!name.value.trim() || !email.value.trim()) {
    showError(t('wizard.required'));
    return false;
  }
  if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email.value.trim())) {
    showError(t('wizard.invalidEmail'));
    return false;
  }
  if (preset.value.key === 'custom') {
    if (!imapHost.value.trim() || !smtpHost.value.trim()) {
      showError(t('wizard.required'));
      return false;
    }
  }
  return true;
}

function showError(message: string): void {
  errorMessage.value = message;
  errorInfoOpen.value = true;
}

function buildDraft(): api.AccountDraft {
  const p = preset.value;
  return {
    provider: p.key,
    name: name.value.trim(),
    email: email.value.trim(),
    auth: p.auth,
    imapHost: p.key === 'custom' ? imapHost.value.trim() : p.imapHost,
    imapPort: p.key === 'custom' ? Number(imapPortStr.value) || p.imapPort : p.imapPort,
    imapSsl: p.key === 'custom' ? imapSsl.value : p.imapSsl,
    smtpHost: p.key === 'custom' ? smtpHost.value.trim() : p.smtpHost,
    smtpPort: p.key === 'custom' ? Number(smtpPortStr.value) || p.smtpPort : p.smtpPort,
    smtpSsl: p.key === 'custom' ? smtpSsl.value : p.smtpSsl,
    password: password.value,
    useOauth: p.auth === 'oauth2',
  };
}

/** 给 Promise 加超时(防止 accountTest 因网络问题无限挂起,按钮卡死在"正在验证") */
function withTimeout<T>(promise: Promise<T>, ms: number, message: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(message)), ms);
    promise.then(
      (v) => { clearTimeout(timer); resolve(v); },
      (e) => { clearTimeout(timer); reject(e); },
    );
  });
}

async function onTest(): Promise<void> {
  if (!validate()) return;
  testing.value = true;
  testResult.value = null;
  try {
    testResult.value = await withTimeout(api.accountTest(buildDraft()), 40000, t('wizard.timeout'));
  } catch (err) {
    testResult.value = { ok: false, message: String(err) };
  } finally {
    testing.value = false;
  }
}

async function onAdd(): Promise<void> {
  if (!validate()) return;
  if (!testResult.value?.ok) {
    await onTest();
    if (!testResult.value?.ok) return;
  }
  adding.value = true;
  try {
    await actions.addAccount(buildDraft());
    emit('close');
  } catch (err) {
    showError(String(err));
  } finally {
    adding.value = false;
  }
}

/* ── OAuth ── */
async function startOAuth(): Promise<void> {
  if (preset.value.key !== 'gmail' && preset.value.key !== 'outlook') return;
  const provider = preset.value.key;
  // 未配置 clientId:引导用户打开申请文档,而不是让按钮"没反应"
  if (!oauthConfigured.value) {
    try {
      await openPath('docs/OAuth.md');
    } catch { /* 文档打不开时忽略 */ }
    showError(t('wizard.oauthNotConfiguredHint'));
    return;
  }
  try {
    const { authUrl } = await api.oauthStart(provider);
    await openUrl(authUrl);
    oauthWaiting.value = true;
  } catch (err) {
    showError(String(err));
  }
}

async function finishOAuth(provider: 'gmail' | 'outlook', oauthEmail: string): Promise<void> {
  const pres = presetByProvider(provider);
  const draft: api.AccountDraft = {
    provider,
    name: name.value.trim() || oauthEmail.split('@')[0] || oauthEmail,
    email: oauthEmail,
    auth: 'oauth2',
    imapHost: pres.imapHost,
    imapPort: pres.imapPort,
    imapSsl: pres.imapSsl,
    smtpHost: pres.smtpHost,
    smtpPort: pres.smtpPort,
    smtpSsl: pres.smtpSsl,
    password: '',
    useOauth: true,
  };
  try {
    await actions.addAccount(draft);
    oauthWaiting.value = false;
    emit('close');
  } catch (err) {
    oauthWaiting.value = false;
    showError(String(err));
  }
}

/* ── 字段变更后重置测试结果 ── */
watch(
  [name, email, password, imapHost, imapPortStr, imapSsl, smtpHost, smtpPortStr, smtpSsl],
  () => { testResult.value = null; },
);

/* ── 生命周期 ── */
const unlisteners: UnlistenFn[] = [];

onMounted(async () => {
  try {
    oauthConfig.value = await api.oauthConfig();
  } catch {
    oauthConfig.value = null;
  }
  unlisteners.push(await api.onOAuthReady((e) => {
    if (oauthWaiting.value && preset.value.key === e.provider) {
      void finishOAuth(e.provider, e.email);
    }
  }));
  unlisteners.push(await api.onOAuthError((e) => {
    if (oauthWaiting.value) {
      oauthWaiting.value = false;
      showError(e.message);
    }
  }));
});

onBeforeUnmount(() => {
  unlisteners.forEach((fn) => fn());
});

function onOverlayClick(): void {
  if (!oauthWaiting.value) emit('close');
}
</script>

<style scoped>
.wizard-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.4);
}

.wizard {
  position: relative;
  width: 540px;
  max-width: calc(100vw - 48px);
  max-height: calc(100vh - 48px);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  border-radius: 8px;
  background: var(--flyout-bg, #fff);
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.2);
}

.wz-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
}

.wz-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.wz-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
}

.wz-close:hover { background: var(--subtle-secondary, rgba(0, 0, 0, 0.04)); }

.wz-body {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px 20px;
}

/* ── 服务商网格 ── */
.wz-step-title {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.wz-providers {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.wz-provider {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
  color: var(--text-primary);
  cursor: pointer;
  font: inherit;
  text-align: left;
  transition: background var(--fast-duration, 0.15s) var(--fast-out-slow-in, ease-out);
}

.wz-provider:hover { background: var(--subtle-secondary, rgba(0, 0, 0, 0.04)); }
.wz-provider:active { background: var(--subtle-tertiary, rgba(0, 0, 0, 0.06)); }

.wz-provider-dot {
  flex: 0 0 auto;
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.wz-provider-label { font-size: 13px; }

/* ── 表单 ── */
.wz-back-row { margin-bottom: 12px; }

.wz-back {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--accent-base);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
}

.wz-back:hover { background: var(--subtle-secondary, rgba(0, 0, 0, 0.04)); }

.wz-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
}

.wz-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.wz-input {
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

.wz-input:focus {
  border-color: var(--accent-base);
  box-shadow: 0 0 0 1px var(--accent-base);
}

.wz-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 12px;
}

.wz-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 12px;
  cursor: pointer;
  user-select: none;
}

.wz-toggle input { display: none; }

.wz-toggle-track {
  width: 38px;
  height: 20px;
  flex: 0 0 auto;
  border-radius: 10px;
  background: var(--ctrl-fill-tertiary, rgba(128, 128, 128, 0.4));
  position: relative;
  transition: background var(--fast-duration, 0.15s) ease;
}

.wz-toggle-track::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  transition: transform var(--fast-duration, 0.15s) ease;
}

.wz-toggle input:checked + .wz-toggle-track {
  background: var(--accent-base);
}

.wz-toggle input:checked + .wz-toggle-track::after {
  transform: translateX(18px);
}

.wz-toggle-label { font-size: 13px; color: var(--text-primary); }

.wz-hint {
  margin: -4px 0 12px;
  font-size: 12px;
  line-height: 18px;
  color: var(--text-secondary);
}

/* ── 按钮 ── */
.wz-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  box-sizing: border-box;
  padding: 5px 16px;
  border: 1px solid var(--ctrl-border-rest, rgba(0, 0, 0, 0.06));
  border-radius: 4px;
  background: var(--ctrl-fill-default, rgba(255, 255, 255, 0.7));
  color: var(--text-primary);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
  transition: background var(--fast-duration, 0.15s) ease;
}

.wz-btn:hover:not(:disabled) { background: var(--subtle-secondary, rgba(0, 0, 0, 0.04)); }
.wz-btn:active:not(:disabled) { background: var(--subtle-tertiary, rgba(0, 0, 0, 0.06)); }
.wz-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.wz-btn-accent {
  border-color: transparent;
  background: var(--accent-base);
  color: var(--accent-text, #fff);
}

.wz-btn-accent:hover:not(:disabled) { background: var(--accent-hover, rgba(0, 103, 192, 0.9)); }
.wz-btn-accent:active:not(:disabled) { background: var(--accent-pressed, rgba(0, 103, 192, 0.8)); }

/* ── 消息 ── */
.wz-msg {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: 6px;
  font-size: 13px;
  line-height: 18px;
}

.wz-msg strong { font-weight: 600; }

.wz-msg-error {
  color: var(--SystemFillColorCriticalBrush, #c42b1c);
  background: var(--SystemFillColorCriticalBackgroundBrush, #fde7e9);
}

.wz-msg-warn {
  color: var(--SystemFillColorCautionBrush, #9d5d00);
  background: var(--SystemFillColorCautionBackgroundBrush, #fff4ce);
}

/* ── 测试结果 ── */
.wz-test-result {
  margin-bottom: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
}

.wz-test-result.is-ok {
  color: var(--SystemFillColorSuccessBrush, #0f7b0f);
  background: var(--SystemFillColorSuccessBackgroundBrush, #dff6dd);
}

.wz-test-result.is-fail {
  color: var(--SystemFillColorCriticalBrush, #c42b1c);
  background: var(--SystemFillColorCriticalBackgroundBrush, #fde7e9);
}

/* ── 底部动作 ── */
.wz-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

/* ── OAuth 等待遮罩 ── */
.wz-oauth-wait {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  background: var(--dialog-overlay, rgba(0, 0, 0, 0.3));
  border-radius: inherit;
}

.wz-spinner {
  width: 28px;
  height: 28px;
  border: 3px solid var(--subtle-secondary, rgba(0, 0, 0, 0.1));
  border-top-color: var(--accent-base);
  border-radius: 50%;
  animation: wz-spin 0.8s linear infinite;
}

@keyframes wz-spin {
  to { transform: rotate(360deg); }
}

.wz-oauth-title {
  margin: 8px 0 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.wz-oauth-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
}

.icon-glyph {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 14px;
  line-height: 1;
}
</style>
