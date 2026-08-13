import { defineConfig } from 'vite';
import plugin from '@vitejs/plugin-vue';

// Nicemail: Tauri 桌面应用。dev 模式下由 Tauri 指向本 dev server，
// build 产物用于 src-tauri 的 frontendDist，故 base 使用根路径。
export default defineConfig({
    base: '/',
    plugins: [plugin()],
    clearScreen: false,
    server: {
        port: 63179,
        strictPort: true,
    },
    envPrefix: ['VITE_', 'TAURI_ENV_'],
    build: {
        target: 'es2021',
        minify: 'esbuild',
        sourcemap: false,
        // SEGOEICONS.TTF(464KB)内嵌为 base64 进 CSS:
        // Tauri 打包后的自定义协议对 .ttf 的 MIME 处理会导致 WebView2 拒绝加载图标字体,
        // 内嵌后完全绕开文件请求,任何平台都能渲染图标。
        assetsInlineLimit: 524288,
    }
})
