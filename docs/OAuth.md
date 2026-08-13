# Nicemail — OAuth 凭据申请指南

Gmail / Outlook 预设走 **OAuth2 授权码 + PKCE**,需要一个在你自己账号下创建的客户端 ID。
本文件指导你申请。163/126/QQ 走"授权码",不需要本流程(见文末)。

完成后把客户端 ID 填入配置文件,再重新启动应用即可。

---

## 1. 凭据放哪

OAuth 配置存放在应用数据目录的 `oauth_config.json`(Windows 上通常是):

```
%APPDATA%\com.nicemail.app\oauth_config.json
```

首次启动应用会自动创建该文件,内容形如:

```json
{
  "google":  { "clientId": "" },
  "outlook": { "clientId": "" }
}
```

把申请的 Client ID 填进对应项即可。**不需要 Client Secret**(桌面应用走 public client 流程)。

---

## 2. Google(Gmail)

1. 打开 <https://console.cloud.google.com/>,新建项目(或选已有)。
2. 左侧 **API 与服务 → 库(OAuth consent screen / Library)**,启用 **Gmail API**:
   - 到 "Enabled APIs & services" → 启用 "Gmail API"。
3. **OAuth 同意屏幕(OAuth consent screen)**:
   - User type 选 **External**(个人用),填应用名称、支持邮箱。
   - Scopes:建议加上 `.../auth/gmail.modify`、`.../auth/gmail.send`、`.../auth/gmail.imap`(也可以在代码首次授权时申请,同意屏会列出)。
   - Test users:把你要登录的 Gmail 账号加进去。
4. **凭据(Credentials)→ 创建凭据 → OAuth 客户端 ID**:
   - Application type 选 **Desktop app**。
   - 名称随意(如 "Nicemail")。
   - 创建后复制 **Client ID**。
5. (可选)在客户端详情里把 **Authorized redirect URIs** 加一条:
   - `http://127.0.0.1:52999/callback`
   - Google 桌面客户端通常接受 `http://localhost`(任意端口),加这条最稳。
6. 把 Client ID 填入 `oauth_config.json` 的 `google.clientId`。

> 注意:Gmail 对第三方应用有严格安全校验。首次授权可能出现"Google 尚未验证此应用"——点 **高级 → 继续前往(不安全)** 即可(只是信任提示,代码里没有危险操作)。

---

## 3. Microsoft(Outlook / 个人微软账号)

1. 打开 <https://portal.azure.com/>,搜索并进入 **App registrations**(应用注册)。
2. 新建注册:
   - 名称:Nicemail
   - **受支持的帐户类型(Supported account types)**:选
     - "个人 Microsoft 帐户" → 建议选 **Accounts in any organizational directory and personal Microsoft accounts**(任何组织目录中的帐户和个人 Microsoft 帐户)。
   - 重定向 URI:**平台选 Mobile and desktop applications**(移动和桌面应用程序),值填:
     - `http://127.0.0.1:52999/callback`
   - 注册完成,复制 **Application (client) ID**。
3. (可选)注册平台时如只有 "Web" 选项,也可用 Web 平台 + 重定向 `http://127.0.0.1:52999/callback`,不影响桌面流程。
4. **API 权限(Api permissions)** → 添加权限:
   - Microsoft Graph 或 Outlook 权限,添加委托权限(Delegated):
     - `IMAP.AccessAsUser.All`
     - `SMTP.Send`
   - (也可以不加,首次授权时按代码里请求的 scope 让用户同意)
5. 把 Client ID 填入 `oauth_config.json` 的 `outlook.clientId`。

---

## 4. 测试

1. 启动 Nicemail,添加账号 → 选 Gmail / Outlook。
2. 点 "使用 Google/Microsoft 授权登录",会打开系统浏览器。
3. 授权后浏览器跳到 `http://127.0.0.1:52999/callback`,页面显示 "授权成功,可以关闭此页面"。
4. 回到 Nicemail,账号自动添加完成。

---

## 5. 163 / 126 / QQ —— 授权码(不需要 OAuth)

| 服务商 | 开启位置 | 说明 |
|---|---|---|
| 163 / 126 | 网页端 → 设置 → POP3/SMTP/IMAP → 开启 IMAP/SMTP | 开启后会给出一个**授权码**,用它作为向导里的"密码" |
| QQ 邮箱 | 网页端 → 设置 → 账户 → 开启 POP3/SMTP 服务 | 会要求发短信验证,随后生成**授权码**,作为密码填入 |

授权码不是你的登录密码,请勿直接填 QQ/163 的登录密码。

---

## 6. 自定义 SMTP(需求 4)

选"自定义 IMAP/SMTP"即可手动填:
- IMAP 主机 / 端口(993 常用,SSL 开;或 143 + STARTTLS)
- SMTP 主机 / 端口(465 常用 SSL,或 587 + STARTTLS)
- 账号 + 密码

适用于企业邮箱、自建邮件服务器等。
