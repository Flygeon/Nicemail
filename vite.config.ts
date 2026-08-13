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
    }
})
