// 主流邮箱服务商预设。auth 字段区分两套认证:
//   'password' — 163/126/QQ/自定义,填密码或授权码
//   'oauth2'   — Gmail/Outlook,走 OAuth2 PKCE(需要 config 中配置客户端 ID)
import type { Provider } from './api';

export interface ProviderPreset {
  key: Provider;
  labelKey: string;          // i18n key,如 'wizard.preset163'
  imapHost: string;
  imapPort: number;
  imapSsl: boolean;
  smtpHost: string;
  smtpPort: number;
  smtpSsl: boolean;
  auth: 'password' | 'oauth2';
  hintKey: string;           // 帮助文案 i18n key
  brandColor: string;
  /** 域名后缀,用于自动识别 */
  domain?: string;
}

export const PROVIDERS: ProviderPreset[] = [
  {
    key: '163',
    labelKey: 'wizard.preset163',
    imapHost: 'imap.163.com',
    imapPort: 993,
    imapSsl: true,
    smtpHost: 'smtp.163.com',
    smtpPort: 465,
    smtpSsl: true,
    auth: 'password',
    hintKey: 'wizard.authCodeHint163',
    brandColor: '#C7000B',
    domain: '163.com',
  },
  {
    key: '126',
    labelKey: 'wizard.preset126',
    imapHost: 'imap.126.com',
    imapPort: 993,
    imapSsl: true,
    smtpHost: 'smtp.126.com',
    smtpPort: 465,
    smtpSsl: true,
    auth: 'password',
    hintKey: 'wizard.authCodeHint163',
    brandColor: '#005BAC',
    domain: '126.com',
  },
  {
    key: 'qq',
    labelKey: 'wizard.presetQQ',
    imapHost: 'imap.qq.com',
    imapPort: 993,
    imapSsl: true,
    smtpHost: 'smtp.qq.com',
    smtpPort: 465,
    smtpSsl: true,
    auth: 'password',
    hintKey: 'wizard.authCodeHintQQ',
    brandColor: '#12B7F5',
    domain: 'qq.com',
  },
  {
    key: 'gmail',
    labelKey: 'wizard.presetGmail',
    imapHost: 'imap.gmail.com',
    imapPort: 993,
    imapSsl: true,
    smtpHost: 'smtp.gmail.com',
    smtpPort: 465,
    smtpSsl: true,
    auth: 'oauth2',
    hintKey: 'wizard.authCodeHintGmail',
    brandColor: '#EA4335',
    domain: 'gmail.com',
  },
  {
    key: 'outlook',
    labelKey: 'wizard.presetOutlook',
    imapHost: 'outlook.office365.com',
    imapPort: 993,
    imapSsl: true,
    smtpHost: 'smtp.office365.com',
    smtpPort: 587,
    smtpSsl: false, // STARTTLS
    auth: 'oauth2',
    hintKey: 'wizard.authCodeHintOutlook',
    brandColor: '#0F6CBD',
    domain: 'outlook.com',
  },
];

/** 自定义 SMTP 服务(需求 4) */
export const CUSTOM_PRESET: ProviderPreset = {
  key: 'custom',
  labelKey: 'wizard.presetCustom',
  imapHost: '',
  imapPort: 993,
  imapSsl: true,
  smtpHost: '',
  smtpPort: 465,
  smtpSsl: true,
  auth: 'password',
  hintKey: 'wizard.authCodeHint163',
  brandColor: '#8A8A8A',
};

export function presetByProvider(key: Provider): ProviderPreset {
  return PROVIDERS.find((p) => p.key === key) ?? CUSTOM_PRESET;
}

/** 尝试根据邮箱地址猜测服务商 */
export function guessProvider(email: string): ProviderPreset {
  const lower = email.toLowerCase();
  for (const p of PROVIDERS) {
    if (p.domain && lower.endsWith(`@${p.domain}`)) return p;
  }
  return CUSTOM_PRESET;
}
