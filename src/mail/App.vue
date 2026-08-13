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
              <WinButton :Content="t('action.add')" Style="{StaticResource AccentButtonStyle}" @Click="openWizard" />
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
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, provide, ref, watch } from 'vue';
import { useI18n } from '../components/i18n/index';
import WinToolTipService from '../components/WinToolTipService.vue';
import WinAutoSuggestBox from '../components/WinAutoSuggestBox.vue';
import WinNavigationView from '../components/WinNavigationView.vue';
import WinButton from '../components/WinButton.vue';

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

  @font-face {
    font-family: 'WinUIOnWebIcons';
    src: url('../assets/Fonts/SEGOEICONS.TTF') format('truetype');
    font-display: block;
  }
</style>
