<template>
  <section class="settings-pane">
    <!-- 顶部返回 -->
    <header class="sp-header">
      <WinButton
        Style="{StaticResource SubtleButtonStyle}"
        @Click="emit('close')">
        <span class="icon-glyph" aria-hidden="true">&#xE72B;</span>
        {{ t('action.back') }}
      </WinButton>
      <h2 class="sp-title">{{ t('settings.title') }}</h2>
    </header>

    <div class="sp-scroll">
      <!-- 外观 -->
      <section class="sp-section">
        <h3 class="sp-section-title">{{ t('settings.appearance') }}</h3>
        <WinExpander
          class="sp-expander"
          :Header="t('settings.theme')">
          <template #HeaderIcon>
            <span class="sp-header-icon" aria-hidden="true">&#xE790;</span>
          </template>
          <WinRadioButtons :SelectedIndex="themeIndex" @SelectionChanged="onThemeSelectionChanged">
            <WinRadioButton :Content="t('settings.light')" />
            <WinRadioButton :Content="t('settings.dark')" />
            <WinRadioButton :Content="t('settings.system')" />
          </WinRadioButtons>
        </WinExpander>
        <WinExpander
          class="sp-expander"
          :Header="t('settings.material')">
          <template #HeaderIcon>
            <span class="sp-header-icon" aria-hidden="true">&#xE7B4;</span>
          </template>
          <WinRadioButtons :SelectedIndex="materialIndex" @SelectionChanged="onMaterialSelectionChanged">
            <WinRadioButton :Content="t('settings.mica')" />
            <WinRadioButton :Content="t('settings.acrylic')" />
          </WinRadioButtons>
        </WinExpander>
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
          <WinButton
            Style="{StaticResource SubtleButtonStyle}"
            :title="t('settings.accountDelete')"
            @Click="confirmDelete(acc)">
            <span class="icon-glyph sp-delete-icon" aria-hidden="true">&#xE74D;</span>
          </WinButton>
        </div>
      </section>

      <!-- 关于 -->
      <section class="sp-section">
        <h3 class="sp-section-title">{{ t('settings.about') }}</h3>
        <div class="sp-about-card">
          <p class="sp-about-version">{{ t('settings.aboutVersion', { version }) }}</p>
          <p class="sp-about-desc">{{ t('settings.aboutDesc') }}</p>
          <WinHyperlinkButton
            @Click="onOpenOAuthDocs">
            <span class="icon-glyph" aria-hidden="true">&#xE8C7;</span>
            {{ t('settings.oauthDocs') }}
          </WinHyperlinkButton>
        </div>
      </section>
    </div>

    <!-- 删除确认 -->
    <WinContentDialog
      v-model:IsOpen="deleteDialogOpen"
      :Title="t('settings.accountDelete')"
      :Content="deleteDialogMessage"
      :PrimaryButtonText="t('action.delete')"
      :CloseButtonText="t('action.cancel')"
      DefaultButton="Close"
      @PrimaryButtonClick="doDelete"
      @CloseButtonClick="deleteDialogOpen = false"
      @Closed="deleteDialogOpen = false" />

    <WinInfoBar
      v-model:IsOpen="infoOpen"
      :Title="infoTitle"
      :Message="infoMessage"
      :Severity="infoSeverity"
      @Close="infoOpen = false" />
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
const themeOptions = ['light', 'dark', 'system'];
const materialOptions = ['mica', 'acrylic'];

const theme = ref(localStorage.getItem('winui-theme-setting') ?? 'system');
const material = ref(localStorage.getItem('winui-material-setting') ?? 'mica');

const themeIndex = computed(() => themeOptions.indexOf(theme.value));
const materialIndex = computed(() => materialOptions.indexOf(material.value));

function onThemeSelectionChanged(e: { SelectedIndex: number }): void {
  theme.value = themeOptions[e.SelectedIndex] ?? theme.value;
}

function onMaterialSelectionChanged(e: { SelectedIndex: number }): void {
  material.value = materialOptions[e.SelectedIndex] ?? material.value;
}

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
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.sp-scroll {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 16px 24px 32px;
  max-width: 720px;
}

.sp-section {
  margin-bottom: 24px;
}

.sp-section-title {
  margin: 0 0 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.sp-expander {
  margin-bottom: 8px;
}

.sp-header-icon {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 16px;
  line-height: 1;
  color: var(--text-secondary);
}

/* ── 账号行 ── */
.sp-account-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  margin-bottom: 6px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
}

.sp-avatar {
  flex: 0 0 auto;
  width: 34px;
  height: 34px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  color: #fff;
  font-size: 15px;
  font-weight: 600;
  user-select: none;
}

.sp-account-info {
  flex: 1 1 auto;
  min-width: 0;
}

.sp-account-email {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sp-account-provider {
  font-size: 12px;
  color: var(--text-secondary);
}

.sp-delete-icon {
  color: var(--text-tertiary);
}

.sp-empty {
  padding: 16px;
  color: var(--text-tertiary);
  font-size: 13px;
  border: 1px dashed var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  text-align: center;
}

/* ── 关于 ── */
.sp-about-card {
  padding: 12px 14px;
  border: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
}

.sp-about-version {
  margin: 0 0 4px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.sp-about-desc {
  margin: 0 0 8px;
  font-size: 13px;
  color: var(--text-secondary);
}

.icon-glyph {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 14px;
  line-height: 1;
}
</style>
