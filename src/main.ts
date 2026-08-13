import { createApp } from 'vue'
import App from './mail/App.vue'
import './styles/theme.css'
import './styles/animations.css'
import { createI18n, i18nKey } from './components/i18n/index'
import mailEnUS from './mail/strings/en-US'
import mailZhCN from './mail/strings/zh-CN'

// 邮件应用资源叠加在组件库资源之上(组件库自带中英文案)
const i18n = createI18n(navigator.language, {
  'en-US': mailEnUS,
  'zh-CN': mailZhCN
})

document.documentElement.lang = i18n.locale
document.title = i18n.t('app.title')

const app = createApp(App)
app.provide(i18nKey, i18n)
app.config.globalProperties.$t = i18n.t
app.mount('#app')

document.addEventListener('contextmenu', (e) => {
  e.preventDefault();
});
