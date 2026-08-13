// 邮件 HTML 净化:基于 DOMParser 的白名单清洗。
// 目标:去掉脚本/事件/危险协议,保留基本的富文本结构。
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
  'cellpadding', 'cellspacing', 'border',
]);

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

function sanitizeNode(node: Node, doc: Document, output: HTMLElement): void {
  const children = Array.from(node.childNodes);
  for (const child of children) {
    if (child.nodeType === Node.TEXT_NODE) {
      output.appendChild(doc.createTextNode(child.textContent ?? ''));
      continue;
    }
    if (child.nodeType !== Node.ELEMENT_NODE) continue;
    const el = child as HTMLElement;
    const tag = el.tagName.toLowerCase();
    if (!ALLOWED_TAGS.has(tag)) {
      // 未知标签:保留文本内容,丢弃标签
      sanitizeNode(el, doc, output);
      continue;
    }
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
      newEl.setAttribute(attr.name, attr.value);
    }
    sanitizeNode(el, doc, newEl);
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
  const doc = new DOMParser().parseFromString(raw, 'text/html');
  const output = doc.createElement('div');
  sanitizeNode(doc.body, doc, output);
  return output.innerHTML;
}
