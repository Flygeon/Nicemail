// 邮件 HTML 净化:基于 DOMParser 的白名单清洗。
// 目标:去掉脚本/事件/危险协议,保留基本的富文本结构,并**安全地保留常见样式**
// (颜色/字体/对齐等),否则 HTML 邮件会退化成纯文本。
// 注意:在 Tauri 的 WebView2 中远程资源仍可能被加载,阅读窗格会额外用 CSS 拦截远程图片。

const ALLOWED_TAGS = new Set([
  'p', 'br', 'div', 'span', 'b', 'strong', 'i', 'em', 'u', 's', 'strike', 'del',
  'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'pre', 'code', 'ul', 'ol',
  'li', 'table', 'thead', 'tbody', 'tfoot', 'tr', 'th', 'td', 'caption', 'a',
  'img', 'hr', 'font', 'center', 'sub', 'sup', 'small', 'big', 'mark',
]);

const ALLOWED_ATTRS = new Set([
  'href', 'src', 'alt', 'title', 'width', 'height', 'align', 'valign',
  'colspan', 'rowspan', 'bgcolor', 'color', 'face', 'target', 'rel', 'cite',
  'cellpadding', 'cellspacing', 'border', 'style',
]);

/** 允许的 CSS 属性白名单(常见邮件排版) */
const ALLOWED_CSS_PROPS = new Set([
  'color', 'background-color', 'font-family', 'font-size', 'font-weight',
  'font-style', 'text-align', 'text-decoration', 'text-transform', 'text-indent',
  'line-height', 'letter-spacing', 'margin', 'margin-top', 'margin-bottom',
  'margin-left', 'margin-right', 'padding', 'padding-top', 'padding-bottom',
  'padding-left', 'padding-right', 'border', 'border-radius',
  'width', 'height', 'max-width', 'min-width', 'max-height', 'min-height',
  'vertical-align', 'white-space', 'word-break', 'overflow-wrap', 'table-layout',
  'border-collapse', 'border-spacing', 'background', 'background-image',
]);

/** 危险 CSS 模式:url/表达式/JS 等一律剔除 */
const DANGEROUS_CSS = /url\s*\(|expression\s*\(|javascript:|-moz-binding|behavior\s*:|@import|position\s*:\s*(fixed|absolute)/i;

/** 清洗单个 style 值:只保留白名单属性,剔除危险内容 */
function sanitizeStyle(value: string): string {
  const parts = value.split(';');
  const kept: string[] = [];
  for (const part of parts) {
    const idx = part.indexOf(':');
    if (idx <= 0) continue;
    const prop = part.slice(0, idx).trim().toLowerCase();
    const val = part.slice(idx + 1).trim();
    if (!ALLOWED_CSS_PROPS.has(prop)) continue;
    if (DANGEROUS_CSS.test(val)) continue;
    kept.push(`${prop}: ${val}`);
  }
  return kept.join('; ');
}

/** 允许的 URL 协议白名单 */
function isSafeUrl(value: string, attr: 'href' | 'src'): boolean {
  const trimmed = (value || '').trim();
  if (trimmed === '') return false;
  if (/^([a-zA-Z][a-zA-Z0-9+.-]*):/.test(trimmed)) {
    const scheme = trimmed.split(':')[0].toLowerCase();
    if (attr === 'href') return ['http', 'https', 'mailto', 'cid'].includes(scheme);
    return ['http', 'https', 'cid'].includes(scheme);
  }
  // 相对路径与锚点
  return /^[#/.]/.test(trimmed) || /^data:image\/(png|jpe?g|gif|webp)/i.test(trimmed);
}

// 节点预算:病理级 HTML(超深嵌套/海量节点)会卡死主线程,超出即停止展开
const NODE_BUDGET = 20000;

function sanitizeNode(node: Node, doc: Document, output: HTMLElement, budget: { count: number }): void {
  if (budget.count >= NODE_BUDGET) return;
  const children = Array.from(node.childNodes);
  for (const child of children) {
    if (budget.count >= NODE_BUDGET) return;
    if (child.nodeType === Node.TEXT_NODE) {
      budget.count++;
      output.appendChild(doc.createTextNode(child.textContent ?? ''));
      continue;
    }
    if (child.nodeType !== Node.ELEMENT_NODE) continue;
    const el = child as HTMLElement;
    const tag = el.tagName.toLowerCase();
    if (!ALLOWED_TAGS.has(tag)) {
      // 未知标签:保留文本内容,丢弃标签
      budget.count++;
      sanitizeNode(el, doc, output, budget);
      continue;
    }
    budget.count++;
    const newEl = doc.createElement(tag);
    for (const attr of Array.from(el.attributes)) {
      const name = attr.name.toLowerCase();
      if (!ALLOWED_ATTRS.has(name)) continue;
      if (name.startsWith('on')) continue;
      if (name === 'href' || name === 'src') {
        if (isSafeUrl(attr.value, name as 'href' | 'src')) {
          newEl.setAttribute(name, attr.value);
        }
        continue;
      }
      if (name === 'target') {
        newEl.setAttribute('rel', 'noopener noreferrer');
      }
      if (name === 'style') {
        const cleaned = sanitizeStyle(attr.value);
        if (cleaned) newEl.setAttribute('style', cleaned);
        continue;
      }
      newEl.setAttribute(attr.name, attr.value);
    }
    sanitizeNode(el, doc, newEl, budget);
    output.appendChild(newEl);
  }
}

/**
 * 净化邮件 HTML。返回安全 HTML 字符串。
 * 远程图片不会被删除,但阅读窗格会用 CSS(`.remote-blocked img`)默认遮罩,
 * 用户点「显示远程图片」后移除该 CSS 类。
 */
export function sanitizeHtml(raw: string): string {
  if (!raw) return '';
  // 超大 HTML(几 MB)在主线程 DOMParser 解析会卡死,截断到合理大小
  const input = raw.length > 400_000 ? raw.slice(0, 400_000) : raw;
  const doc = new DOMParser().parseFromString(input, 'text/html');
  const output = doc.createElement('div');
  sanitizeNode(doc.body, doc, output, { count: 0 });
  return output.innerHTML;
}
