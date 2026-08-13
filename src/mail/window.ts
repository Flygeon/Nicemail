// 自绘标题栏的窗口控制(Tauri)。decorations=false 时需要。
import { getCurrentWindow } from '@tauri-apps/api/window';

const win = getCurrentWindow();

/** 仅在标题栏空白区域按下左键时开始拖拽窗口(避免按钮/输入框触发) */
export function onTitlebarPointerDown(event: PointerEvent): void {
  const target = event.target as HTMLElement | null;
  if (!target) return;
  if (target.closest('button, input, select, textarea, a[href], [role="button"], [contenteditable="true"], .no-drag')) return;
  if (event.button !== 0) return;
  void win.startDragging();
}

export function minimizeWindow(): void {
  void win.minimize();
}

export function toggleMaximizeWindow(): void {
  void win.toggleMaximize();
}

export function closeWindow(): void {
  void win.close();
}

export async function isWindowMaximized(): Promise<boolean> {
  return win.isMaximized();
}

export function onWindowMaximizedChanged(handler: (maximized: boolean) => void): () => void {
  const unlisten = win.onResized(() => {
    void isWindowMaximized().then(handler);
  });
  return () => {
    void unlisten.then((fn) => fn());
  };
}
