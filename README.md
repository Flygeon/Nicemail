# Nicemail 📧

WinUI 风格、跨平台的第三方邮件客户端。基于 **Tauri 2 + Vue 3 + Rust**,复用 WinUIonWeb 组件库实现现代化界面。

## 功能

- 🔐 主流邮箱预设登录:**163 / 126 / QQ**(授权码)、**Gmail / Outlook**(OAuth2 PKCE)
- ⚙️ **自定义 IMAP/SMTP 服务**登录(企业邮箱 / 自建服务器)
- 📥 收件:增量同步、文件夹树、未读角标、搜索(FTS5)
- 📤 发件:HTML/纯文本、附件、草稿(存回服务器)
- 📌 星标 / 已读未读 双向同步到服务器
- 🌓 浅色/深色主题 + Mica/Acrylic 材质
- 💬 中英双语界面

## 技术栈

| 层 | 技术 |
|---|---|
| 外壳 | Tauri 2(Windows/macOS/Linux) |
| 前端 | Vue 3 + TypeScript + Vite,WinUIonWeb 组件库 |
| 邮件引擎 | Rust:`imap`+`native-tls`(IMAP)、`lettre`(SMTP)、`mail-parser`(MIME) |
| 存储 | SQLite(`rusqlite`,本地缓存 + FTS5 搜索) |
| 凭据 | Windows 凭据管理器(`keyring`),授权码/refresh token 不落明文库 |

## 开发

要求:Node 20+ / 22+,Rust 1.77+。

```bash
npm install

# 仅前端
npm run dev                 # http://localhost:63179

# 完整桌面应用(会先起 dev server)
npm run tauri dev

# 打包
npm run tauri build
```

> 首次 `npm run tauri dev` 会编译 Rust 依赖,较慢,请耐心。

## 配置 OAuth(使用 Gmail/Outlook 前必读)

见 [docs/OAuth.md](docs/OAuth.md)。申请 Google Cloud / Azure 的客户端 ID 后填入:

```
%APPDATA%\com.nicemail.app\oauth_config.json
```

163/126/QQ 只需在网页端开启 IMAP/SMTP 并获取**授权码**,填入向导即可。

## 目录结构

```
src/                  # 前端(Vue)
  components/         # WinUIonWeb 组件库(原样保留)
  mail/               # Nicemail 邮件应用
    api.ts            # Rust ⇄ TS 桥接契约
    actions.ts        # 前端数据操作层
    state.ts          # 响应式状态
    components/       # 邮件面板(列表/阅读/写信/设置/账号向导)
  gallery/            # WinUI 控件示例(保留作参考)
src-tauri/            # Rust 后端 + Tauri 配置
docs/OAuth.md         # OAuth 申请指南
```

## 声明

界面基于 [WinUIonWeb](https://github.com/Furry-Xiyi/WinUIonWeb)(微软 WinUI 的 Web 实现,与微软无关)。邮件收发逻辑完全自研。
