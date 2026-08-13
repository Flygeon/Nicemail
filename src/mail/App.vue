<template>
  <WinToolTipService />
  <div class="mail-root">
    <!-- 自绘标题栏 -->
    <header
      class="mail-titlebar"
      @pointerdown="onTitlebarPointerDown">
      <div class="mail-titlebar-brand">
        <span class="mail-titlebar-logo" aria-hidden="true">&#xE8D7;</span>
        <span class="mail-titlebar-title">{{ t('app.title') }}</span>
      </div>
      <div class="mail-titlebar-search no-drag">
        <WinAutoSuggestBox
          v-model:Text="searchText"
          :ItemsSource="searchSuggestions"
          TextMemberPath="title"
          :PlaceholderText="t('search.placeholder')"
          QueryIcon="Find"
          @QuerySubmitted="onSearchSubmitted" />
      </div>
      <div class="mail-titlebar-window-controls no-drag">
        <button class="win-btn" type="button" :aria-label="t('action.settings')" @click="goSettings">
          <span aria-hidden="true">&#xE713;</span>
        </button>
        <button class="win-btn" type="button" aria-label="minimize" @click="minimizeWindow">
          <span aria-hidden="true">&#xE921;</span>
        </button>
        <button class="win-btn" type="button" :aria-label="isMax ? 'restore' : 'maximize'" @click="toggleMaximizeWindow">
          <span aria-hidden="true">{{ isMax ? '&#xE923;' : '&#xE922;' }}</span>
        </button>
        <button class="win-btn win-btn-close" type="button" aria-label="close" @click="closeWindow">
          <span aria-hidden="true">&#xE8BB;</span>
        </button>
      </div>
    </header>

    <div class="mail-app-body">
      <WinNavigationView
        v-model:SelectedItem="selectedNav"
        :MenuItems="navItems"
        :FooterMenuItems="footerItems"
        PaneDisplayMode="Left"
        :IsSettingsVisible="false"
        IsBackButtonVisible="Collapsed"
        :IsPaneToggleButtonVisible="true"
        :IsPaneOpen="paneOpen"
        @ItemInvoked="onNavInvoked">
        <div class="mail-content">
          <!-- 设置 -->
          <SettingsPane v-if="view === 'settings'" @close="goMail" />

          <!-- 无账号:引导 -->
          <div v-else-if="accounts.length === 0" class="mail-empty">
            <div class="mail-empty-inner">
              <span class="mail-empty-icon" aria-hidden="true">&#xE8D7;</span>
              <h2>{{ t('empty.noAccounts') }}</h2>
              <p>{{ t('empty.noAccountsHint') }}</p>
              <button type="button" class="mail-empty-btn" @click="openWizard">
                {{ t('action.add') }}
              </button>
            </div>
          </div>

          <!-- 邮件 3 栏 -->
          <div v-else class="mail-panes">
            <MessageListPane />
            <ReadingPane />
            <!-- 写信覆盖层 -->
            <ComposePane v-if="composeOpen" />
          </div>
        </div>
      </WinNavigationView>
    </div>

    <!-- 账号向导(模态) -->
    <AccountWizard v-if="wizardOpen" @close="wizardOpen = false" />

    <!-- 全局轻提示(WinUI 风格) -->
    <Transition name="toast">
      <div v-if="state.toastVisible" class="mail-toast" role="status">
        <span class="icon-glyph mail-toast-icon" aria-hidden="true">&#xE73E;</span>
        <span class="mail-toast-text">{{ state.toastMessage }}</span>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, provide, ref, watch } from 'vue';
import { useI18n } from '../components/i18n/index';
import WinToolTipService from '../components/WinToolTipService.vue';
import WinAutoSuggestBox from '../components/WinAutoSuggestBox.vue';
import WinNavigationView from '../components/WinNavigationView.vue';

import * as state from './state';
import * as actions from './actions';
import { accountList } from './api';
import { onMailChanged } from './api';
import { onTitlebarPointerDown, minimizeWindow, toggleMaximizeWindow, closeWindow, isWindowMaximized, onWindowMaximizedChanged } from './window';

import MessageListPane from './components/MessageListPane.vue';
import ReadingPane from './components/ReadingPane.vue';
import ComposePane from './components/ComposePane.vue';
import SettingsPane from './components/SettingsPane.vue';
import AccountWizard from './components/AccountWizard.vue';

const { t } = useI18n();

const paneOpen = ref(true);
const isMax = ref(false);
const searchText = ref('');
const wizardOpen = ref(false);

const view = computed(() => state.view.value);
const accounts = computed(() => state.accounts.value);
const folders = computed(() => state.folders.value);
const selection = computed(() => state.selection);
const composeOpen = computed(() => state.composeOpen.value);

// 供部分组件注入的主题/材质
const themeSetting = ref(localStorage.getItem('winui-theme-setting') ?? 'system');
const materialSetting = ref(localStorage.getItem('winui-material-setting') ?? 'mica');
provide('themeSetting', themeSetting);
provide('materialSetting', materialSetting);
// WinThemeWrapper 提供的上下文,WinComboBox/WinMenuFlyout 等组件 inject 它
provide('winuiTheme', computed(() =>
  themeSetting.value === 'dark' || (themeSetting.value === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
    ? 'dark'
    : 'light'
));

function applyTheme(mode: string): void {
  const html = document.documentElement;
  html.classList.remove('theme-light', 'theme-dark');
  if (mode === 'light') html.classList.add('theme-light');
  else if (mode === 'dark') html.classList.add('theme-dark');
}
watch(themeSetting, (v) => applyTheme(v), { immediate: true });
watch(themeSetting, (v) => localStorage.setItem('winui-theme-setting', v));
watch(materialSetting, (v) => localStorage.setItem('winui-material-setting', v));

/* ── 导航项 ── */
interface NavItem {
  Tag: string;
  Icon?: string;
  Content?: string;
  infoBadge?: { Value: number };
  SelectsOnInvoked?: boolean;
  MenuItems?: NavItem[];
}

const navItems = computed<NavItem[]>(() => {
  const items: NavItem[] = [
    { Tag: '__compose__', Icon: '', Content: t('action.compose'), SelectsOnInvoked: false },
  ];
  for (const acc of accounts.value) {
    const accFolders = folders.value[acc.id] ?? [];
    const children: NavItem[] = accFolders.map((f) => ({
      Tag: `folder:${acc.id}:${f.fullName}`,
      Content: f.name,
      infoBadge: f.unreadCount > 0 ? { Value: f.unreadCount } : undefined,
    }));
    items.push({
      Tag: `acc:${acc.id}`,
      Icon: '',
      Content: acc.email,
      SelectsOnInvoked: false,
      MenuItems: children,
    });
  }
  return items;
});

const footerItems = computed<NavItem[]>(() => [
  { Tag: '__add__', Icon: '', Content: t('nav.addAccount'), SelectsOnInvoked: false },
  { Tag: '__settings__', Icon: '', Content: t('nav.settings') },
]);

const selectedNav = computed({
  get: () => {
    if (view.value === 'settings') return { Tag: '__settings__', Content: t('nav.settings') };
    const acc = selection.value.accountId;
    const folder = selection.value.folder;
    if (acc && folder) return { Tag: `folder:${acc}:${folder}` };
    if (acc) return { Tag: `acc:${acc}` };
    return null;
  },
  set: () => { /* 选中由 ItemInvoked 驱动 */ },
});

function parseFolderTag(tag: string): { accountId: string; folder: string } | null {
  if (!tag.startsWith('folder:')) return null;
  const rest = tag.slice('folder:'.length);
  const idx = rest.indexOf(':');
  if (idx < 0) return null;
  return { accountId: rest.slice(0, idx), folder: rest.slice(idx + 1) };
}

function onNavInvoked(e: { IsSettingsInvoked: boolean; InvokedItemContainer?: NavItem }): void {
  const item = e.InvokedItemContainer;
  if (!item) return;
  const tag = item.Tag;
  if (tag === '__compose__') {
    if (accounts.value.length === 0) wizardOpen.value = true;
    else actions.openCompose();
    return;
  }
  if (tag === '__add__') { wizardOpen.value = true; return; }
  if (tag === '__settings__') { state.view.value = 'settings'; return; }
  const parsed = parseFolderTag(tag);
  if (parsed) {
    state.view.value = 'mail';
    if (parsed.accountId !== selection.value.accountId || parsed.folder !== selection.value.folder) {
      void actions.selectFolder(parsed.accountId, parsed.folder);
    }
  }
}

function goSettings(): void { state.view.value = 'settings'; }
function goMail(): void { state.view.value = 'mail'; }
function openWizard(): void { wizardOpen.value = true; }

/* ── 搜索 ── */
const searchSuggestions = computed(() => {
  if (!searchText.value.trim()) return [];
  return [{ title: t('search.allAccounts'), Tag: '__all__' }];
});

function onSearchSubmitted(e: { QueryText?: string }): void {
  const q = (e.QueryText ?? searchText.value).trim();
  if (!q) return;
  state.searchOpen.value = true;
  if (accounts.value.length > 0) {
    void actions.runSearch(q, selection.value.folder ?? undefined);
  }
}

/* ── 生命周期 ── */
const unlisteners: Array<() => void> = [];

onMounted(async () => {
  isMax.value = await isWindowMaximized();
  unlisteners.push(onWindowMaximizedChanged((m) => { isMax.value = m; }));

  try {
    const accs = await accountList();
    state.accounts.value = accs;
    if (accs.length > 0) await actions.selectAccount(accs[0].id);
  } catch (err) {
    console.error('init failed', err);
  }

  unlisteners.push(await onMailChanged(() => {
    if (state.selection.accountId) void actions.refreshUnreadCounts(state.selection.accountId);
  }));
});

onBeforeUnmount(() => {
  unlisteners.forEach((fn) => fn());
});
</script>

<style>
  @import '../styles/theme.css';
  @import '../styles/animations.css';
  @import './mail.css';

  :root {
    /* 所有 WinUI 组件用 var(--SymbolThemeFontFamily, 'Segoe Fluent Icons') 取图标字体。
       这里显式指向打包进来的同名字体,避免依赖 Win11 才有的系统字体(Win10 上会变豆腐块)。 */
    --SymbolThemeFontFamily: 'Segoe Fluent Icons';
  }

  @font-face {
    /* SEGOEICONS.TTF 内部字体名就是 "Segoe Fluent Icons"。
       注册成同名后,在 Win10/macOS/Linux 等无该系统字体的平台也能渲染全部图标。 */
    font-family: 'Segoe Fluent Icons';
    src: url('../assets/Fonts/SEGOEICONS.TTF') format('truetype');
    font-display: block;
  }

  /* 关键:WinUI 组件里大量图标元素(<span class="icon"> 等)不通过 --SymbolThemeFontFamily 取字体,
     而是依赖这条全局规则(原 WinUIonWeb gallery App.vue 自带,删 gallery 时被一并删除)。
     必须恢复,否则这些元素退化为继承字体(Segoe UI 无 PUA 字形),全部显示为豆腐块。 */
  body .icon,
  body .icon-btn,
  body .ptr-icon-wrapper,
  body .symbol-icon,
  body .win-symbol-icon,
  body .win-asb-icon,
  body .picker-icon,
  body .checkbox-glyph,
  body .win-combo-chevron,
  body .win-cbf-icon,
  body .win-cbf-overflow-icon,
  body .win-expander-header-icon,
  body .win-expander-arrow,
  body .infobadge-icon,
  body .close-icon,
  body .win-menu-flyout-icon,
  body .win-menu-flyout-check,
  body .win-menu-flyout-check-placeholder,
  body .win-menu-flyout-chevron,
  body .win-number-spin-button span,
  body .win-number-compact-indicator span,
  body .win-number-popup-button span,
  body .win-password-reveal span,
  body .win-rating-glyph,
  body .scrollbar-button,
  body .win-settings-card-icon,
  body .win-settings-card-action-icon,
  body .win-teaching-tip-icon,
  body .win-teaching-tip-close,
  body .win-textbox-delete-glyph,
  body .font-icon,
  body .icon-glyph,
  body .icon-preview-glyph,
  body .group-icon,
  body .tree-icon {
    font-family: 'Segoe Fluent Icons';
  }
</style>
