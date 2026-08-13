<template>
  <section class="message-list-pane">
    <!-- 顶部工具栏 -->
    <header class="mlp-toolbar">
      <div class="mlp-toolbar-left">
        <h2 class="mlp-folder-title">{{ folderName }}</h2>
        <span v-if="unreadCount > 0" class="mlp-unread-badge">{{ t('folder.unread', { count: unreadCount }) }}</span>
      </div>
      <div class="mlp-toolbar-actions">
        <WinProgressRing
          v-if="syncing"
          class="mlp-sync-ring"
          :IsActive="true"
          :IsIndeterminate="true" />
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          :IsEnabled="!syncing"
          @Click="onRefresh">
          <span class="icon-glyph" aria-hidden="true">&#xE72C;</span>
          {{ t('action.refresh') }}
        </WinButton>
        <WinButton
          Style="{StaticResource SubtleButtonStyle}"
          @Click="onCompose">
          <span class="icon-glyph" aria-hidden="true">&#xE715;</span>
          {{ t('action.compose') }}
        </WinButton>
      </div>
    </header>

    <!-- 搜索态横幅 -->
    <div v-if="searchOpen" class="mlp-search-bar">
      <span class="icon-glyph mlp-search-icon" aria-hidden="true">&#xE721;</span>
      <span class="mlp-search-text">{{ t('search.in', { folder: folderName }) }}</span>
      <WinButton
        Style="{StaticResource SubtleButtonStyle}"
        @Click="clearSearch">
        {{ t('action.cancel') }}
      </WinButton>
    </div>

    <!-- 邮件列表 -->
    <div ref="listEl" class="mlp-list" @scroll="onScroll">
      <div
        v-for="msg in messages"
        :key="msg.id"
        class="mlp-row"
        :class="{ 'is-unread': msg.unread, 'is-selected': msg.id === selectedMessageId }"
        @click="onSelect(msg)">
        <div class="mlp-row-indicators">
          <span
            class="mlp-star"
            role="button"
            :aria-label="t('nav.starred')"
            @click.stop="onToggleStar(msg)">
            <span v-if="msg.starred" class="icon-glyph star is-on" aria-hidden="true">&#xE734;</span>
            <span v-else class="icon-glyph star" aria-hidden="true">&#xE735;</span>
          </span>
          <span v-if="msg.unread" class="mlp-unread-dot" aria-hidden="true"></span>
        </div>
        <div class="mlp-row-main">
          <div class="mlp-row-line">
            <span class="mlp-from" :class="{ 'is-unread': msg.unread }">{{ msg.fromName || msg.fromEmail }}</span>
            <span class="mlp-time">{{ formatTime(msg.date) }}</span>
          </div>
          <div class="mlp-row-line2">
            <span class="mlp-subject" :class="{ 'is-unread': msg.unread }">{{ msg.subject || t('mail.noSubject') }}</span>
            <span v-if="msg.hasAttachments" class="icon-glyph mlp-attach" :title="t('mail.attachment')" aria-hidden="true">&#xE8B9;</span>
          </div>
          <div v-if="msg.preview" class="mlp-preview">{{ msg.preview }}</div>
        </div>
      </div>

      <div v-if="messages.length === 0 && !loadingMessages" class="mlp-empty">
        {{ searchOpen ? t('empty.noSearchResults') : t('empty.noMessages') }}
      </div>

      <div v-if="loadingMessages" class="mlp-loading">
        <WinProgressRing :IsActive="true" :IsIndeterminate="true" />
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from '../../components/i18n/index';
import * as state from '../state';
import * as actions from '../actions';
import type { MessageSummary } from '../api';

const { t, locale } = useI18n();

const listEl = ref<HTMLElement | null>(null);

const folderName = computed(() => state.currentFolder()?.name ?? '');
const unreadCount = computed(() => state.currentFolder()?.unreadCount ?? 0);
const syncing = computed(() => state.syncing.value);
const loadingMessages = computed(() => state.loadingMessages.value);
const messages = computed(() => state.messages.value);
const selectedMessageId = computed(() => state.selectedMessageId.value);
const searchOpen = computed(() => state.searchOpen.value);

function onRefresh(): void {
  void actions.refreshCurrentFolder();
}

function onCompose(): void {
  actions.openCompose();
}

function onSelect(msg: MessageSummary): void {
  void actions.selectMessage(msg);
}

function onToggleStar(msg: MessageSummary): void {
  void actions.toggleStar(msg);
}

function clearSearch(): void {
  state.searchOpen.value = false;
  void actions.refreshCurrentFolder();
}

/** 滚动触底自动加载更多 */
function onScroll(e: Event): void {
  const el = e.target as HTMLElement;
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 80) {
    void actions.loadMoreMessages();
  }
}

/** 按天格式化时间:今天→时间、昨天→"昨天"、一周内→星期、更早→日期 */
function formatTime(ts: number): string {
  const d = new Date(ts);
  const now = new Date();
  const dayStart = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diffDays = Math.round((dayStart(now) - dayStart(d)) / 86_400_000);
  if (diffDays <= 0) {
    return d.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' });
  }
  if (diffDays === 1) return t('mail.yesterday');
  if (diffDays < 7) return d.toLocaleDateString(locale, { weekday: 'short' });
  return d.toLocaleDateString(locale, { month: 'short', day: 'numeric' });
}
</script>

<style scoped>
.message-list-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  border-right: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  background: var(--card-bg, rgba(255, 255, 255, 0.5));
}

/* ── 工具栏 ── */
.mlp-toolbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
}

.mlp-toolbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.mlp-folder-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text-primary);
}

.mlp-unread-badge {
  flex: 0 0 auto;
  padding: 1px 8px;
  border-radius: 10px;
  font-size: 11px;
  line-height: 16px;
  color: var(--accent-text);
  background: var(--accent-base);
}

.mlp-toolbar-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.mlp-sync-ring {
  width: 20px;
  height: 20px;
  margin-right: 2px;
}

/* ── 搜索横幅 ── */
.mlp-search-bar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.06));
  background: var(--card-bg-secondary, rgba(246, 246, 246, 0.5));
  color: var(--text-secondary);
}

.mlp-search-icon {
  color: var(--text-tertiary);
}

.mlp-search-text {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── 列表 ── */
.mlp-list {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.mlp-row {
  position: relative;
  display: flex;
  gap: 8px;
  padding: 8px 10px 8px 6px;
  cursor: pointer;
  border-bottom: 1px solid var(--stroke-divider, rgba(0, 0, 0, 0.04));
  transition: background var(--fast-duration, 0.15s) var(--fast-out-slow-in, ease-out);
}

.mlp-row.is-unread {
  background: var(--ctrl-fill-input-active, rgba(255, 255, 255, 0.7));
}

.mlp-row:hover {
  background: var(--subtle-secondary, rgba(0, 0, 0, 0.04));
}

.mlp-row.is-selected {
  background: var(--subtle-tertiary, rgba(0, 0, 0, 0.06));
}

.mlp-row.is-selected::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 2px;
  background: var(--accent-base);
}

.mlp-row-indicators {
  flex: 0 0 auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  width: 22px;
}

.mlp-star {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 4px;
  cursor: pointer;
}

.mlp-star:hover {
  background: var(--subtle-secondary, rgba(0, 0, 0, 0.04));
}

.mlp-star .star {
  font-size: 14px;
  color: var(--text-tertiary);
  transition: color var(--fast-duration, 0.15s);
}

.mlp-star .star.is-on {
  color: var(--accent-base);
}

.mlp-unread-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent-base);
}

.mlp-row-main {
  flex: 1 1 auto;
  min-width: 0;
}

.mlp-row-line,
.mlp-row-line2 {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
}

.mlp-from {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 13px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mlp-from.is-unread {
  font-weight: 600;
  color: var(--text-primary);
}

.mlp-time {
  flex: 0 0 auto;
  font-size: 11px;
  color: var(--text-tertiary);
  white-space: nowrap;
}

.mlp-subject {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 13px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mlp-subject.is-unread {
  font-weight: 600;
  color: var(--text-primary);
}

.mlp-attach {
  flex: 0 0 auto;
  font-size: 12px;
  color: var(--text-tertiary);
}

.mlp-preview {
  margin-top: 2px;
  font-size: 12px;
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mlp-empty {
  padding: 40px 16px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
}

.mlp-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.icon-glyph {
  font-family: var(--SymbolThemeFontFamily, 'WinUIOnWebIcons');
  font-size: 14px;
  line-height: 1;
}
</style>
