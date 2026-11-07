import {marked} from 'marked';
import DOMPurify from 'dompurify';
import hljs from 'highlight.js/lib/core';
import javascript from 'highlight.js/lib/languages/javascript';
import typescript from 'highlight.js/lib/languages/typescript';
import python from 'highlight.js/lib/languages/python';
import bash from 'highlight.js/lib/languages/bash';
import shell from 'highlight.js/lib/languages/shell';
import json from 'highlight.js/lib/languages/json';
import xml from 'highlight.js/lib/languages/xml';
import css from 'highlight.js/lib/languages/css';
import scss from 'highlight.js/lib/languages/scss';
import markdown from 'highlight.js/lib/languages/markdown';
import yaml from 'highlight.js/lib/languages/yaml';
import swift from 'highlight.js/lib/languages/swift';
import rust from 'highlight.js/lib/languages/rust';
import go from 'highlight.js/lib/languages/go';
import java from 'highlight.js/lib/languages/java';
import kotlin from 'highlight.js/lib/languages/kotlin';
import c from 'highlight.js/lib/languages/c';
import cpp from 'highlight.js/lib/languages/cpp';
import csharp from 'highlight.js/lib/languages/csharp';
import ruby from 'highlight.js/lib/languages/ruby';
import php from 'highlight.js/lib/languages/php';
import sql from 'highlight.js/lib/languages/sql';
import diff from 'highlight.js/lib/languages/diff';
import dockerfile from 'highlight.js/lib/languages/dockerfile';
import ini from 'highlight.js/lib/languages/ini';
import makefile from 'highlight.js/lib/languages/makefile';
import plaintext from 'highlight.js/lib/languages/plaintext';
import objectivec from 'highlight.js/lib/languages/objectivec';
import lua from 'highlight.js/lib/languages/lua';
import r from 'highlight.js/lib/languages/r';
import latex from 'highlight.js/lib/languages/latex';
import powershell from 'highlight.js/lib/languages/powershell';

Object.entries({javascript, typescript, python, bash, shell, json, xml, css, scss, markdown, yaml, swift, rust, go, java, kotlin, c, cpp, csharp, ruby, php, sql, diff, dockerfile, ini, makefile, plaintext, objectivec, lua, r, latex, powershell})
  .forEach(([name, lang]) => hljs.registerLanguage(name, lang));
hljs.registerAliases(['js', 'jsx', 'mjs'], {languageName: 'javascript'});
hljs.registerAliases(['ts', 'tsx'], {languageName: 'typescript'});
hljs.registerAliases(['sh', 'zsh', 'console'], {languageName: 'bash'});
hljs.registerAliases(['html', 'svg', 'plist'], {languageName: 'xml'});
hljs.registerAliases(['toml'], {languageName: 'ini'});
hljs.registerAliases(['py'], {languageName: 'python'});
hljs.registerAliases(['yml'], {languageName: 'yaml'});
hljs.registerAliases(['text', 'txt'], {languageName: 'plaintext'});

/* ───────────────────────── helpers ───────────────────────── */

const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
function h(tag, props = {}, ...kids) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props || {})) {
    if (v == null || v === false) continue;
    if (k === 'class') el.className = v;
    else if (k === 'html') el.innerHTML = v;
    else if (k === 'text') el.textContent = v;
    else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
    else if (k === 'style' && typeof v === 'object') Object.entries(v).forEach(([p, x]) => el.style.setProperty(p, x));
    else if (k === 'dataset') Object.assign(el.dataset, v);
    else el.setAttribute(k, v === true ? '' : v);
  }
  for (const kid of kids.flat()) if (kid != null && kid !== false) el.append(kid.nodeType ? kid : document.createTextNode(String(kid)));
  return el;
}
const esc = s => String(s).replace(/[&<>"']/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'}[c]));
const basename = p => p.split('/').filter(Boolean).pop() || p;
const dirname = p => p.replace(/\/[^/]*\/?$/, '') || '/';
function normalize(p) {
  const out = [];
  for (const seg of p.split('/')) { if (!seg || seg === '.') continue; if (seg === '..') out.pop(); else out.push(seg); }
  return '/' + out.join('/');
}
const ext = p => (p.match(/\.([^./]+)$/) || [])[1]?.toLowerCase() || '';
const MD_EXT = ['md', 'markdown', 'mdown', 'mkd', 'mdx'];
const isMarkdown = p => MD_EXT.includes(ext(p));
const stripExt = n => n.replace(/\.(md|markdown|mdown|mkd|mdx|txt)$/i, '');
const uid = () => Math.random().toString(36).slice(2, 10);
const debounce = (fn, ms) => { let t; return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); }; };
const within = (p, root) => p === root || p.startsWith(root.endsWith('/') ? root : root + '/');
const plural = (n, w) => `${n.toLocaleString()} ${w}${n === 1 ? '' : 's'}`;
const isMac = true;
let HOME = '';
const tilde = p => HOME && p.startsWith(HOME) ? '~' + p.slice(HOME.length) : p;
function timeAgo(t) {
  if (!t) return '';
  const s = (Date.now() - t) / 1000;
  if (s < 60) return 'just now';
  if (s < 3600) return Math.floor(s / 60) + 'm ago';
  if (s < 86400) return Math.floor(s / 3600) + 'h ago';
  if (s < 86400 * 30) return Math.floor(s / 86400) + 'd ago';
  return new Date(t).toLocaleDateString();
}

/* ───────────────────────── icons ───────────────────────── */

const FILE = '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>';
const FOLDER = '<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 16.5z"/>';
const I = {
  files: '<path d="M9 7V5.5A2.5 2.5 0 0 1 11.5 3H16l4 4v9.5a2.5 2.5 0 0 1-2.5 2.5H16"/><path d="M4 10.5A2.5 2.5 0 0 1 6.5 8H11l4 4v6.5a2.5 2.5 0 0 1-2.5 2.5h-6A2.5 2.5 0 0 1 4 18.5z"/>',
  search: '<circle cx="11" cy="11" r="6.5"/><path d="m20 20-4.2-4.2"/>',
  outline: '<path d="M4 6h16M8 12h12M12 18h8"/><circle cx="4.5" cy="12" r=".6"/><circle cx="8.5" cy="18" r=".6"/>',
  layers: '<path d="m12 3 9 4.8-9 4.8-9-4.8z"/><path d="m3 12.2 9 4.8 9-4.8"/><path d="m3 16.6 9 4.8 9-4.8" opacity=".5"/>',
  settings: '<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M4.6 4.6 6 6M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4"/>',
  moon: '<path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z"/>',
  chevron: '<path d="m9.5 6 6 6-6 6"/>',
  chevronDown: '<path d="m6 9.5 6 6 6-6"/>',
  folder: FOLDER,
  folderOpen: '<path d="M3 17V7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h6.5A2.5 2.5 0 0 1 20 9.5V10"/><path d="M3 17.5 5.6 11a1.6 1.6 0 0 1 1.5-1H21l-2.8 7.5a2.3 2.3 0 0 1-2.1 1.5H5a2 2 0 0 1-2-1.5z"/>',
  file: FILE,
  md: '<rect x="2.5" y="5.5" width="19" height="13" rx="2.5"/><path d="M6 15V9l2.5 3L11 9v6M16 9v6M13.8 13l2.2 2.2 2.2-2.2"/>',
  x: '<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  filePlus: FILE + '<path d="M12 11.5v6M9 14.5h6"/>',
  folderPlus: FOLDER + '<path d="M12 9.8v6M9 12.8h6"/>',
  refresh: '<path d="M20 12a8 8 0 1 1-2.6-5.9"/><path d="M20 4v5h-5"/>',
  collapse: '<rect x="4" y="4" width="16" height="16" rx="3"/><path d="M8.5 12h7"/>',
  book: '<path d="M3 5.5h6A3 3 0 0 1 12 8.5V20a2.3 2.3 0 0 0-2.3-2.3H3z"/><path d="M21 5.5h-6a3 3 0 0 0-3 3V20a2.3 2.3 0 0 1 2.3-2.3H21z"/>',
  columns: '<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M12 4v16"/>',
  code: '<path d="m8.5 8-4 4 4 4M15.5 8l4 4-4 4"/>',
  more: '<circle cx="5.5" cy="12" r="1.1"/><circle cx="12" cy="12" r="1.1"/><circle cx="18.5" cy="12" r="1.1"/>',
  sidebar: '<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M9.5 4v16"/>',
  sidebarFill: '<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M9.5 4v16"/><path d="M3.8 5.2h5v13.6h-5z" fill="currentColor" stroke="none" opacity=".35"/>',
  up: '<path d="M12 19V5M6 11l6-6 6 6"/>',
  down: '<path d="M12 5v14M6 13l6 6 6-6"/>',
  copy: '<rect x="8.5" y="8.5" width="12" height="12" rx="2.5"/><path d="M15.5 8.5V6a2.5 2.5 0 0 0-2.5-2.5H6A2.5 2.5 0 0 0 3.5 6v7A2.5 2.5 0 0 0 6 15.5h2.5"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  link: '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 12.5A1.6 1.6 0 0 0 8.6 21h6.8a1.6 1.6 0 0 0 1.6-1.5L18 7M9 7V4h6v3"/>',
  edit: '<path d="M4 20h4L19.5 8.5a2.1 2.1 0 0 0-3-3L5 17z"/>',
  hash: '<path d="M5 9h14M5 15h14M10 4 8 20M16 4l-2 16"/>',
  command: '<path d="M9 6.5A2.5 2.5 0 1 0 6.5 9H17.5A2.5 2.5 0 1 0 15 6.5v11a2.5 2.5 0 1 0 2.5-2.5h-11A2.5 2.5 0 1 0 9 17.5z"/>',
  keyboard: '<rect x="2.5" y="6" width="19" height="12" rx="2.5"/><path d="M6.5 10h.01M10 10h.01M14 10h.01M17.5 10h.01M7.5 14h9"/>',
  zen: '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>',
  dot: '<circle cx="12" cy="12" r="4" fill="currentColor" stroke="none"/>',
  warning: '<path d="M12 4 2.8 19.5h18.4z"/><path d="M12 10v4.2M12 17h.01"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5.5M12 7.8h.01"/>',
  bulb: '<path d="M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2V16h5v-.1c0-.8.4-1.5 1-2A6 6 0 0 0 12 3z"/>',
  alert: '<circle cx="12" cy="12" r="9"/><path d="M12 7.5v5.5M12 16.4h.01"/>',
  shield: '<path d="M12 3 4.5 6v5.5c0 4.6 3.2 8.4 7.5 9.5 4.3-1.1 7.5-4.9 7.5-9.5V6z"/>',
  history: '<path d="M3.5 12a8.5 8.5 0 1 0 2.5-6"/><path d="M3.5 4v4h4M12 7.5V12l3 2"/>',
  export: '<path d="M12 3v12M7 8l5-5 5 5"/><path d="M5 14v4.5A2.5 2.5 0 0 0 7.5 21h9a2.5 2.5 0 0 0 2.5-2.5V14"/>',
  finder: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M12 4c-1.5 3-1.8 7-1 10.5M8 9v1.5M16 9v1.5M8 15c2.4 1.6 5.6 1.6 8 0"/>',
  wrap: '<path d="M4 6h16M4 12h13a3 3 0 0 1 0 6h-4M4 18h5"/><path d="m14.5 16-2 2 2 2"/>',
};
const icon = (n, cls = '') => `<svg class="i ${cls}" viewBox="0 0 24 24" aria-hidden="true">${I[n] || ''}</svg>`;

/* ───────────────────────── native bridge ───────────────────────── */

const native = window.__TAURI__?.core;
const api = (action, payload = {}) => native ? native.invoke('api', {action, payload}) : mock(action, payload);

/* ───────────────────────── settings, themes, state ───────────────────────── */

const THEMES = {
  dark: {label: 'Folio Dark', dark: true, bg: [0.071, 0.075, 0.09]},
  midnight: {label: 'Midnight (OLED)', dark: true, bg: [0, 0, 0]},
  light: {label: 'Folio Light', dark: false, bg: [0.957, 0.957, 0.965]},
  paper: {label: 'Paper', dark: false, bg: [0.945, 0.925, 0.886]},
};
const DEFAULTS = {
  theme: 'dark', readerFont: 'sans', readerSize: 16, readerWidth: 800, editorSize: 13, lineHeight: 1.7,
  wrap: true, lineNumbers: true, showAllFiles: false, syncScroll: true, defaultMode: 'read', remoteImages: false,
  sidebarWidth: 272, sidebarVisible: true, view: 'explorer', split: 0.5, previewTabs: true, autoReload: true,
  restoreTabs: true, openEditors: true, zen: false,
};
let S = {settings: {...DEFAULTS}, workspaces: [], current: null, recent: []};
const docs = new Map();
let tabs = [];
let active = null;
let closedStack = [];
const trees = new Map();
let expanded = new Set();
let selectedPath = null;
let fileCache = null;
let fullscreen = false;
const ws = () => S.workspaces.find(w => w.id === S.current);
const doc = () => docs.get(active);
const isDirty = d => d && d.kind === 'md' && d.text !== d.baseline;
const dirtyDocs = () => [...docs.values()].filter(isDirty);

const persistNow = () => {
  const w = ws();
  if (w) {
    w.tabs = tabs.map(k => docs.get(k)).filter(d => d && (d.path || d.kind !== 'md')).map(d => ({key: d.key, path: d.path, mode: d.mode}));
    w.active = active;
    w.expanded = [...expanded];
    w.opened = Date.now();
  }
  return api('saveState', {json: JSON.stringify(S)});
};
const persist = debounce(persistNow, 500);

/* ───────────────────────── toasts ───────────────────────── */

function toast(message, kind = 'info', actions = []) {
  const el = h('div', {class: 'toast ' + kind, html: icon(kind === 'error' ? 'warning' : kind === 'ok' ? 'check' : 'info')}, h('span', {}, message));
  for (const [label, fn] of actions) el.append(h('button', {onclick: () => { fn(); el.remove(); }}, label));
  $('#toasts').append(el);
  requestAnimationFrame(() => el.classList.add('in'));
  setTimeout(() => { el.classList.remove('in'); setTimeout(() => el.remove(), 250); }, actions.length ? 7000 : 2600);
}

/* ───────────────────────── markdown rendering ───────────────────────── */

marked.setOptions({gfm: true});
const PURIFY = {FORBID_TAGS: ['style', 'form', 'iframe', 'object', 'embed', 'button', 'textarea', 'select', 'link', 'meta', 'base'], FORBID_ATTR: ['style', 'srcset'], ADD_ATTR: ['checked', 'disabled', 'type']};

function slugger() {
  const seen = new Map();
  return text => {
    let s = text.toLowerCase().trim().replace(/<[^>]+>/g, '').replace(/[^\p{L}\p{N}\s_-]/gu, '').replace(/\s/g, '-') || 'section';
    const n = seen.get(s) || 0; seen.set(s, n + 1);
    return n ? `${s}-${n}` : s;
  };
}
const plainHeading = t => t.replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[*_`~]/g, '').replace(/<[^>]+>/g, '').trim();

function splitFrontMatter(text) {
  if (!/^---\r?\n/.test(text)) return {front: null, body: text, offset: 0};
  const lines = text.split('\n');
  for (let i = 1; i < Math.min(lines.length, 300); i++) {
    if (/^(---|\.\.\.)\s*$/.test(lines[i])) return {front: lines.slice(1, i), body: lines.slice(i + 1).join('\n'), offset: i + 1};
  }
  return {front: null, body: text, offset: 0};
}

/** Lex once per text change; reused by outline, preview, and stats. */
function analyze(d) {
  if (d.analysis && d.analysis.text === d.text) return d.analysis;
  const {front, body, offset} = splitFrontMatter(d.text);
  let tokens;
  try { tokens = marked.lexer(body); } catch { tokens = []; }
  const slug = slugger();
  const headings = [];
  let line = offset;
  for (const t of tokens) {
    t._line = line;
    if (t.type === 'heading') { const text = plainHeading(t.text); headings.push({depth: t.depth, text, line, id: slug(text)}); }
    line += (t.raw.match(/\n/g) || []).length;
  }
  const words = (d.text.match(/[\p{L}\p{N}][\p{L}\p{N}'’-]*/gu) || []).length;
  d.analysis = {text: d.text, front, tokens, headings, words};
  return d.analysis;
}

function renderPreview(d) {
  if (d.rendered === d.text && d.renderedRemote === S.settings.remoteImages) return;
  const a = analyze(d);
  const article = d.el.article;
  const frag = document.createDocumentFragment();
  if (a.front && a.front.some(l => l.trim())) {
    const dl = h('dl', {class: 'frontmatter', dataset: {line: '0'}});
    let last = null;
    for (const l of a.front) {
      const m = l.match(/^([\w.-]+)\s*:\s*(.*)$/);
      if (m) { dl.append(h('dt', {}, m[1])); last = h('dd', {}, m[2].replace(/^["']|["']$/g, '')); dl.append(last); }
      else if (last && l.trim()) last.append(h('br'), l.trim().replace(/^-\s*/, '• '));
    }
    frag.append(dl);
  }
  const links = a.tokens.links || {};
  for (const t of a.tokens) {
    if (t.type === 'space') continue;
    const list = [t]; list.links = links;
    let html;
    try { html = marked.parser(list); } catch { html = `<pre>${esc(t.raw)}</pre>`; }
    const tpl = document.createElement('template');
    tpl.innerHTML = DOMPurify.sanitize(html, PURIFY);
    for (const el of tpl.content.children) el.dataset.line = t._line;
    frag.append(tpl.content);
  }
  article.replaceChildren(frag);
  enhance(d, article, a);
  d.rendered = d.text;
  d.renderedRemote = S.settings.remoteImages;
  d.blocks = $$(':scope > [data-line]', article).map(el => ({line: +el.dataset.line, el}));
  if (find.open) findRefresh();
}

const CALLOUTS = {NOTE: ['info', 'Note'], TIP: ['bulb', 'Tip'], IMPORTANT: ['alert', 'Important'], WARNING: ['warning', 'Warning'], CAUTION: ['shield', 'Caution']};

function enhance(d, article, a) {
  // Headings: stable ids that match the outline.
  const top = $$(':scope > h1, :scope > h2, :scope > h3, :scope > h4, :scope > h5, :scope > h6', article);
  top.forEach((el, i) => { if (a.headings[i]) el.id = a.headings[i].id; });
  const slug = slugger(); a.headings.forEach(x => slug(x.text));
  for (const el of $$('h1,h2,h3,h4,h5,h6', article)) {
    if (!el.id) el.id = slug(el.textContent);
    el.prepend(h('a', {class: 'anchor', href: '#' + el.id, 'aria-label': 'Link to section', html: icon('link')}));
  }
  // Callouts (GitHub alert syntax).
  for (const bq of $$('blockquote', article)) {
    const p = bq.firstElementChild;
    const m = p?.tagName === 'P' && p.firstChild?.nodeType === 3 && p.firstChild.textContent.match(/^\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*/i);
    if (!m) continue;
    const kind = m[1].toUpperCase();
    p.firstChild.textContent = p.firstChild.textContent.slice(m[0].length);
    if (p.firstChild.nextSibling?.tagName === 'BR') p.firstChild.nextSibling.remove();
    if (!p.textContent.trim() && !p.children.length) p.remove();
    bq.classList.add('callout', 'callout-' + kind.toLowerCase());
    bq.prepend(h('div', {class: 'callout-title', html: icon(CALLOUTS[kind][0]) + esc(CALLOUTS[kind][1])}));
  }
  // Task lists become live checkboxes that edit the source.
  $$('input', article).forEach((box, i) => {
    if (box.type !== 'checkbox') { box.remove(); return; }
    box.disabled = false; box.dataset.task = i;
    box.closest('li')?.classList.add('task');
    box.closest('ul,ol')?.classList.add('tasks');
  });
  // Code blocks.
  for (const code of $$('pre > code', article)) {
    const pre = code.parentElement;
    const lang = (code.className.match(/language-([\w+#-]+)/) || [])[1]?.toLowerCase() || '';
    const text = code.textContent;
    if (lang && hljs.getLanguage(lang)) {
      try { code.innerHTML = hljs.highlight(text, {language: lang, ignoreIllegals: true}).value; } catch {}
    }
    code.classList.add('hljs');
    const wrap = h('div', {class: 'codeblock', dataset: {line: pre.dataset.line || ''}});
    const head = h('div', {class: 'code-head'}, h('span', {class: 'lang'}, lang || 'text'),
      h('button', {class: 'copy', title: 'Copy', html: icon('copy') + '<span>Copy</span>', onclick: e => {
        api('copy', {text}); const b = e.currentTarget; b.classList.add('done'); b.innerHTML = icon('check') + '<span>Copied</span>';
        setTimeout(() => { b.classList.remove('done'); b.innerHTML = icon('copy') + '<span>Copy</span>'; }, 1400);
      }}));
    pre.replaceWith(wrap); wrap.append(head, pre); delete pre.dataset.line;
  }
  // Tables scroll horizontally instead of crushing columns.
  for (const t of $$('table', article)) {
    const wrap = h('div', {class: 'table-wrap', dataset: {line: t.dataset.line || ''}});
    t.replaceWith(wrap); wrap.append(t); delete t.dataset.line;
    if (!wrap.dataset.line) delete wrap.dataset.line;
  }
  // Images.
  const base = d.path ? dirname(d.path) : null;
  for (const img of $$('img', article)) {
    const src = img.getAttribute('src') || '';
    img.loading = 'lazy';
    if (/^data:image\/(png|jpe?g|gif|webp|svg\+xml);/i.test(src)) continue;
    if (/^https?:/i.test(src)) {
      if (S.settings.remoteImages) continue;
      img.replaceWith(h('span', {class: 'img-blocked', title: src, html: icon('shield') + `<span>Remote image blocked${img.alt ? ' — ' + esc(img.alt) : ''}</span>`}));
      continue;
    }
    if (!base || /^[a-z]+:/i.test(src)) { img.removeAttribute('src'); continue; }
    let path = src.split(/[?#]/)[0];
    try { path = decodeURIComponent(path); } catch {}
    path = path.startsWith('/') ? normalize(path) : normalize(base + '/' + path);
    img.src = 'mdasset://local' + path.split('/').map(encodeURIComponent).join('/');
    img.addEventListener('error', () => img.classList.add('broken'), {once: true});
  }
  for (const a2 of $$('a[href]', article)) {
    const href = a2.getAttribute('href');
    if (/^(https?|mailto):/i.test(href)) a2.title = href;
  }
}

function onPreviewClick(e, d) {
  if (e.target.closest('.live-editor')) return;
  const box = e.target.closest('input[data-task]');
  if (box) { e.preventDefault(); toggleTask(d, +box.dataset.task); return; }
  const a = e.target.closest('a[href]');
  if (!a || (d.mode === 'read' && e.altKey)) {
    if (d.mode !== 'read' || e.target.closest('.code-head') || !window.getSelection().isCollapsed) return;
    const block = e.target.closest('.markdown-body > [data-line]');
    if (block) startLiveEdit(d, block, caretHint(e, block));
    else if (e.target === d.el.article && !d.el.article.children.length) startLiveAppend(d);
    return;
  }
  e.preventDefault();
  const href = a.getAttribute('href');
  if (href.startsWith('#')) {
    const id = decodeURIComponent(href.slice(1));
    const target = d.el.article.querySelector(`[id="${CSS.escape(id)}"]`);
    if (target) scrollPreviewTo(d, target);
    return;
  }
  if (/^(https?|mailto):/i.test(href)) { api('external', {url: href}); return; }
  if (/^[a-z]+:/i.test(href) || !d.path) return;
  let [file, anchor] = href.split('#');
  try { file = decodeURIComponent(file); } catch {}
  const path = file.startsWith('/') ? normalize(file) : normalize(dirname(d.path) + '/' + file);
  if (isMarkdown(path) || ext(path) === 'txt' || !ext(path)) openFile(isMarkdown(path) || ext(path) ? path : path + '.md', {anchor, preview: false});
  else api('reveal', {path});
}

function toggleTask(d, index) {
  const lines = d.text.split('\n');
  let fence = null, n = -1;
  for (let i = 0; i < lines.length; i++) {
    const L = lines[i];
    const f = L.match(/^\s*(`{3,}|~{3,})/);
    if (f) { if (!fence) fence = f[1][0]; else if (f[1][0] === fence) fence = null; continue; }
    if (fence) continue;
    const m = L.match(/^(\s*(?:>\s*)*(?:[-*+]|\d+[.)])\s+\[)([ xX])(\])/);
    if (m && ++n === index) {
      lines[i] = m[1] + (m[2] === ' ' ? 'x' : ' ') + m[3] + L.slice(m[0].length);
      setText(d, lines.join('\n'));
      return;
    }
  }
}

/* ───────────────────────── source highlighting ───────────────────────── */

const INLINE = /(`+)(.+?)\1|(!?\[)([^\]\n]*)(\]\()([^)\s]*)((?:\s+"[^"]*")?\))|(\*\*|__)(?=\S)(.+?\S)\8|(\*|_)(?=[^\s*_])(.+?[^\s\\])\10(?![\w*])|(~~)(.+?)~~|(<\/?[A-Za-z][^>\n]*>)|(https?:\/\/[^\s)<>\]]+)|(\\[\\`*_{}\[\]()#+\-.!|>~])/g;
function inline(s) {
  let out = '', last = 0, m;
  INLINE.lastIndex = 0;
  while ((m = INLINE.exec(s))) {
    out += esc(s.slice(last, m.index));
    if (m[1]) out += `<span class="t-code"><span class="t-punc">${esc(m[1])}</span>${esc(m[2])}<span class="t-punc">${esc(m[1])}</span></span>`;
    else if (m[3]) out += `<span class="t-link"><span class="t-punc">${esc(m[3])}</span><span class="t-ltext">${esc(m[4])}</span><span class="t-punc">${esc(m[5])}</span><span class="t-url">${esc(m[6])}</span><span class="t-punc">${esc(m[7])}</span></span>`;
    else if (m[8]) out += `<span class="t-strong"><span class="t-punc">${m[8]}</span>${inline(m[9])}<span class="t-punc">${m[8]}</span></span>`;
    else if (m[10]) out += `<span class="t-em"><span class="t-punc">${m[10]}</span>${inline(m[11])}<span class="t-punc">${m[10]}</span></span>`;
    else if (m[12]) out += `<span class="t-strike"><span class="t-punc">~~</span>${inline(m[13])}<span class="t-punc">~~</span></span>`;
    else if (m[14]) out += `<span class="t-html">${esc(m[14])}</span>`;
    else if (m[15]) out += `<span class="t-url">${esc(m[15])}</span>`;
    else if (m[16]) out += `<span class="t-punc">${esc(m[16])}</span>`;
    last = INLINE.lastIndex;
  }
  return out + esc(s.slice(last));
}

function highlightSource(text) {
  const lines = text.split('\n');
  let out = '', fence = null, front = lines[0] === '---';
  for (let i = 0; i < lines.length; i++) {
    const L = lines[i];
    let html, cls = '', m;
    if (front) {
      cls = 'l-meta';
      if (i > 0 && /^(---|\.\.\.)\s*$/.test(L)) { front = false; html = `<span class="t-punc">${esc(L)}</span>`; }
      else if (i === 0) html = `<span class="t-punc">${esc(L)}</span>`;
      else html = esc(L).replace(/^(\s*-?\s*)([\w.-]+)(:)/, '$1<span class="t-key">$2</span><span class="t-punc">$3</span>');
    } else if (fence) {
      cls = 'l-code';
      if (new RegExp('^\\s*' + fence + '+\\s*$').test(L) && L.trim().length >= fence.length) { fence = null; html = `<span class="t-fence">${esc(L)}</span>`; }
      else html = `<span class="t-cb">${esc(L)}</span>`;
    } else if ((m = L.match(/^(\s{0,3})(`{3,}|~{3,})(.*)$/))) {
      fence = m[2]; cls = 'l-code';
      html = `${m[1]}<span class="t-fence">${esc(m[2])}</span><span class="t-lang">${esc(m[3])}</span>`;
    } else if ((m = L.match(/^(\s{0,3})(#{1,6})(\s.*|$)/))) {
      html = `${m[1]}<span class="t-h t-h${m[2].length}"><span class="t-hmark">${m[2]}</span>${inline(m[3])}</span>`;
    } else if (/^\s{0,3}([-*_])(\s*\1){2,}\s*$/.test(L)) {
      html = `<span class="t-hr">${esc(L)}</span>`;
    } else if ((m = L.match(/^(\s*(?:>\s?)+)(.*)$/))) {
      html = `<span class="t-quote-mark">${esc(m[1])}</span><span class="t-quote">${inline(m[2])}</span>`;
    } else if ((m = L.match(/^(\s*)([-*+]|\d+[.)])(\s+)(\[[ xX]\](?=\s|$))?(.*)$/))) {
      html = `${m[1]}<span class="t-list">${esc(m[2])}</span>${m[3]}${m[4] ? `<span class="t-task">${esc(m[4])}</span>` : ''}${inline(m[5])}`;
    } else if (/^\s*\|/.test(L) || /^\s*:?-{3,}:?\s*\|/.test(L)) {
      html = L.split(/(\|)/).map((p, j) => j % 2 ? '<span class="t-pipe">|</span>' : /^\s*:?-+:?\s*$/.test(p) ? `<span class="t-punc">${esc(p)}</span>` : inline(p)).join('');
    } else if ((m = L.match(/^(\s*\[[^\]]+\]:)(\s*)(\S+)(.*)$/))) {
      html = `<span class="t-ltext">${esc(m[1])}</span>${m[2]}<span class="t-url">${esc(m[3])}</span>${esc(m[4])}`;
    } else html = inline(L);
    out += `<div class="ln${cls ? ' ' + cls : ''}">${html || '<br>'}</div>`;
  }
  return out;
}

/* ───────────────────────── documents & editor views ───────────────────────── */

const LARGE = 400_000;

function createDocView(d) {
  const root = h('div', {class: 'doc', dataset: {mode: d.mode}});
  root.innerHTML = `
    <div class="banner" hidden></div>
    <div class="panes">
      <section class="pane source"><div class="scroller"><div class="stack">
        <div class="findlayer" aria-hidden="true"></div>
        <div class="hl" aria-hidden="true"></div>
        <textarea spellcheck="false" autocapitalize="off" autocomplete="off" autocorrect="off" aria-label="Markdown source"></textarea>
      </div></div></section>
      <div class="divider" role="separator" aria-label="Resize editor and preview"></div>
      <section class="pane preview"><div class="scroller"><article class="markdown-body"></article><div class="endmark"><span></span></div></div></section>
    </div>`;
  d.el = {
    root, banner: $('.banner', root), src: $('.source .scroller', root), stack: $('.stack', root), findlayer: $('.findlayer', root),
    hl: $('.hl', root), ta: $('textarea', root), pv: $('.preview .scroller', root), article: $('article', root), divider: $('.divider', root),
  };
  const {ta, src, pv, article, divider} = d.el;
  ta.value = d.text;
  d.large = d.text.length > LARGE;
  root.classList.toggle('large', d.large);
  ta.addEventListener('input', () => { d.text = ta.value; onTextChanged(d); });
  ta.addEventListener('keydown', e => editorKeys(e, d));
  ta.addEventListener('scroll', () => { if (ta.scrollTop || ta.scrollLeft) { ta.scrollTop = 0; ta.scrollLeft = 0; } });
  ta.addEventListener('focus', () => { d.lastFocus = 'source'; });
  const caret = () => { updateCursor(d); };
  ta.addEventListener('keyup', caret); ta.addEventListener('mouseup', caret); ta.addEventListener('select', caret);
  src.addEventListener('mousedown', e => { if (e.target === src || e.target === d.el.stack) { e.preventDefault(); ta.focus(); ta.setSelectionRange(ta.value.length, ta.value.length); caret(); } });
  src.addEventListener('scroll', () => { if (d.mode === 'split') syncScroll(d, 'source'); if (d.mode !== 'read') trackHeading(d); }, {passive: true});
  pv.addEventListener('scroll', () => { if (d.mode === 'split') syncScroll(d, 'preview'); if (d.mode === 'read') trackHeading(d); }, {passive: true});
  article.addEventListener('click', e => onPreviewClick(e, d));
  d.el.root.querySelector('.endmark').addEventListener('click', () => { if (d.mode === 'read') startLiveAppend(d); });
  divider.addEventListener('pointerdown', e => {
    divider.setPointerCapture(e.pointerId); root.classList.add('resizing');
    const move = ev => { const b = $('.panes', root).getBoundingClientRect(); S.settings.split = Math.min(.8, Math.max(.2, (ev.clientX - b.left) / b.width)); applyLayout(); };
    const up = () => { divider.removeEventListener('pointermove', move); root.classList.remove('resizing'); persist(); };
    divider.addEventListener('pointermove', move); divider.addEventListener('pointerup', up, {once: true});
  });
  divider.addEventListener('dblclick', () => { S.settings.split = .5; applyLayout(); persist(); });
  $('#docs').append(root);
  d.hlText = null;
}

function setText(d, text, asBaseline = false) {
  d.text = text;
  if (asBaseline) d.baseline = text;
  if (d.el) {
    const ta = d.el.ta, s = ta.selectionStart, e = ta.selectionEnd;
    ta.value = text;
    ta.setSelectionRange(Math.min(s, text.length), Math.min(e, text.length));
  }
  onTextChanged(d);
}

const notifyDirty = debounce(() => { api('window', {dirty: dirtyDocs().length}); }, 120);
const afterEdit = debounce(d => {
  if (d === doc()) { renderOutline(); renderStatus(); renderBreadcrumbs(); }
  if (d.mode !== 'source' && d === doc() && !d.live) renderPreview(d);
}, 140);

function onTextChanged(d) {
  const wasDirty = d.wasDirty;
  d.wasDirty = isDirty(d);
  if (d.preview && d.wasDirty) { d.preview = false; }
  if (d.mode !== 'read') updateHighlight(d);
  if (wasDirty !== d.wasDirty) { renderTabs(); renderExplorerSoon(); updateTitle(); }
  notifyDirty();
  afterEdit(d);
  if (find.open && d === doc()) findRefreshSoon();
}

function updateHighlight(d) {
  if (!d.el || d.hlText === d.text) return;
  d.hlText = d.text;
  if (d.large) { d.el.hl.textContent = ''; return; }
  d.el.hl.innerHTML = highlightSource(d.text);
  d.curLine = -1;
  updateCursor(d, true);
}

function lineOfOffset(text, off) {
  let n = 0;
  for (let i = text.indexOf('\n'); i !== -1 && i < off; i = text.indexOf('\n', i + 1)) n++;
  return n;
}

function updateCursor(d, quiet) {
  if (!d.el || d !== doc()) return;
  const ta = d.el.ta;
  const line = lineOfOffset(ta.value, ta.selectionStart);
  if (line !== d.curLine && !d.large) {
    d.el.hl.children[d.curLine]?.classList.remove('cur');
    d.el.hl.children[line]?.classList.add('cur');
    d.curLine = line;
  }
  d.cursor = {line, col: ta.selectionStart - ta.value.lastIndexOf('\n', ta.selectionStart - 1), sel: ta.selectionEnd - ta.selectionStart};
  if (!quiet) { renderStatusCursor(); ensureCaretVisible(d); }
}

function ensureCaretVisible(d) {
  const el = d.el.hl.children[d.curLine];
  if (!el || document.activeElement !== d.el.ta) return;
  const sc = d.el.src, top = el.offsetTop, bottom = top + el.offsetHeight;
  if (top < sc.scrollTop + 8) sc.scrollTop = top - 8;
  else if (bottom > sc.scrollTop + sc.clientHeight - 24) sc.scrollTop = bottom - sc.clientHeight + 24;
}

function insert(text) {
  if (text) document.execCommand('insertText', false, text);
  else document.execCommand('delete', false);
}

function editorKeys(e, d, ta = d.el.ta) {
  const {value, selectionStart: s, selectionEnd: t} = ta;
  if (e.key === 'Enter' && !e.shiftKey && !e.metaKey && !e.altKey && !e.ctrlKey && !e.isComposing && s === t) {
    const ls = value.lastIndexOf('\n', s - 1) + 1;
    const line = value.slice(ls, s);
    const m = line.match(/^(\s*(?:>\s?)*)([-*+]|(\d+)([.)]))(\s+)(\[[ xX]\]\s+)?/);
    if (m) {
      e.preventDefault();
      if (line.trim() === m[0].trim() && value.slice(s, value.indexOf('\n', s) === -1 ? value.length : value.indexOf('\n', s)).trim() === '') {
        ta.setSelectionRange(ls, s); insert('');
      } else {
        const marker = m[3] ? (+m[3] + 1) + m[4] : m[2];
        insert('\n' + m[1] + marker + m[5] + (m[6] ? '[ ] ' : ''));
      }
      return;
    }
    const ind = line.match(/^[ \t]*/)[0];
    if (ind) { e.preventDefault(); insert('\n' + ind); }
    return;
  }
  if (e.key === 'Tab' && !e.metaKey && !e.ctrlKey && !e.altKey) {
    e.preventDefault();
    const ls = value.lastIndexOf('\n', s - 1) + 1;
    const multi = value.slice(s, t).includes('\n');
    const isList = /^\s*([-*+]|\d+[.)])\s/.test(value.slice(ls));
    if (!multi && !e.shiftKey && !isList) { insert('  '); return; }
    let le = value.indexOf('\n', Math.max(t - (t > s && value[t - 1] === '\n' ? 1 : 0), s));
    if (le === -1) le = value.length;
    const block = value.slice(ls, le);
    const out = block.split('\n').map(l => e.shiftKey ? l.replace(/^( {1,2}|\t)/, '') : '  ' + l).join('\n');
    ta.setSelectionRange(ls, le); insert(out);
    ta.setSelectionRange(multi ? ls : Math.max(ls, s + (out.length - block.length)), multi ? ls + out.length : Math.max(ls, s + (out.length - block.length)));
    return;
  }
  if ((e.key === '[' || e.key === ']') && e.metaKey && !e.shiftKey) {
    e.preventDefault(); ta.dispatchEvent(new KeyboardEvent('keydown', {key: 'Tab', shiftKey: e.key === '['})); return;
  }
  if (ta === d.el.ta && /^Arrow|^Page|^Home|^End/.test(e.key)) requestAnimationFrame(() => updateCursor(d));
}

function wrapSelection(d, mark) {
  if (!d) return;
  const ta = d.live ? d.live.ta : d.mode !== 'read' ? d.el.ta : null;
  if (!ta) return;
  ta.focus();
  const {value, selectionStart: s, selectionEnd: t} = ta;
  const sel = value.slice(s, t);
  if (sel.startsWith(mark) && sel.endsWith(mark) && sel.length >= mark.length * 2) { insert(sel.slice(mark.length, -mark.length)); ta.setSelectionRange(s, t - mark.length * 2); return; }
  if (value.slice(s - mark.length, s) === mark && value.slice(t, t + mark.length) === mark) { ta.setSelectionRange(s - mark.length, t + mark.length); insert(sel); ta.setSelectionRange(s - mark.length, t - mark.length); return; }
  insert(mark + sel + mark); ta.setSelectionRange(s + mark.length, t + mark.length);
}

/* ───────────────────────── live editing (edit blocks in place) ───────────────────────── */

const LEAD = /^\s*(#{1,6}\s+|(?:[-*+]|\d+[.)])\s+(?:\[[ xX]\]\s+)?|>\s*)/;

/** Rendered text from the start of the block up to the click point. */
function caretHint(e, block) {
  const r = document.caretRangeFromPoint?.(e.clientX, e.clientY);
  if (!r || !block.contains(r.startContainer)) return null;
  const pre = document.createRange();
  pre.selectNodeContents(block);
  try { pre.setEnd(r.startContainer, r.startOffset); } catch { return null; }
  return pre.toString();
}

/** Map a rendered-text prefix to an offset in the block's Markdown. */
function caretFromHint(src, hint) {
  if (hint == null) return src.length;
  const flat = hint.replace(/\s+/g, ' ');
  if (!flat.trim()) return (src.match(LEAD) || [''])[0].length;
  for (const n of [18, 10, 5, 2]) {
    const tail = flat.slice(-n);
    if (tail.length < Math.min(n, flat.length)) continue;
    let i = src.indexOf(tail, Math.max(0, flat.length - tail.length));
    if (i < 0) i = src.indexOf(tail);
    if (i >= 0) return i + tail.length;
  }
  return src.length;
}

function startLiveEdit(d, block, hint = null) {
  if (!block || d.mode !== 'read') return;
  let line = +block.dataset.line;
  if (d.live) {
    const L = d.live;
    if (L.el === block) return;
    const shift = L.append ? 0 : L.end - L.origEnd, after = line > L.start;
    commitLive(d);
    if (after) line += shift;
    block = d.blocks.find(b => b.line === line)?.el;
    if (!block) return;
  }
  const blocks = d.blocks || [];
  const i = blocks.findIndex(b => b.el === block);
  const lines = d.text.split('\n');
  const start = line;
  let end = i >= 0 && i + 1 < blocks.length ? blocks[i + 1].line : lines.length;
  while (end > start + 1 && lines[end - 1].trim() === '') end--;
  const src = lines.slice(start, end).join('\n');
  openLiveEditor(d, {start, end, src, replace: block, caret: caretFromHint(src, hint)});
}

function startLiveAppend(d) {
  if (!d || d.kind !== 'md' || d.mode !== 'read') return;
  if (d.live?.append) { d.live.ta.focus(); return; }
  if (d.live) commitLive(d);
  renderPreview(d);
  openLiveEditor(d, {append: true, src: '', caret: 0});
  d.live.el.scrollIntoView({block: 'nearest', behavior: 'smooth'});
}

function openLiveEditor(d, {start = 0, end = 0, src, replace, append = false, caret}) {
  const ta = h('textarea', {class: 'live-ta', spellcheck: 'true', rows: 1, placeholder: append ? 'Write Markdown…' : ''});
  ta.value = src;
  const ed = h('div', {class: 'live-editor' + (append ? ' append' : '')}, ta,
    h('div', {class: 'live-hint', html: `<span>${icon('md')}Markdown</span><span><kbd>⌘B</kbd> bold <kbd>⌘I</kbd> italic <kbd>⌘K</kbd> link <kbd>Esc</kbd> done</span>`}));
  if (replace) replace.replaceWith(ed); else d.el.article.append(ed);
  const L = d.live = {start, end, origEnd: end, el: ed, ta, append, base: d.text.replace(/\s+$/, ''), original: d.text};
  const fit = () => { ta.style.height = '0px'; ta.style.height = ta.scrollHeight + 'px'; };
  ta.addEventListener('input', () => {
    fit();
    if (L.append) d.text = ta.value.trim() ? L.base + (L.base ? '\n\n' : '') + ta.value + '\n' : L.original;
    else {
      const lines = d.text.split('\n'), repl = ta.value.split('\n');
      lines.splice(L.start, L.end - L.start, ...repl);
      L.end = L.start + repl.length;
      d.text = lines.join('\n');
    }
    d.el.ta.value = d.text;
    onTextChanged(d);
  });
  ta.addEventListener('keydown', e => {
    if (e.key === 'Escape' || (e.key === 'Enter' && (e.metaKey || e.ctrlKey))) { e.preventDefault(); e.stopPropagation(); commitLive(d); return; }
    editorKeys(e, d, ta);
    requestAnimationFrame(fit);
  });
  ta.addEventListener('blur', e => { if (e.relatedTarget && !ed.contains(e.relatedTarget) && d.live === L) commitLive(d); });
  requestAnimationFrame(fit);
  fit();
  ta.focus({preventScroll: true});
  ta.setSelectionRange(caret, caret);
}

function commitLive(d, render = true) {
  const L = d.live;
  if (!L) return;
  d.live = null;
  d.rendered = null;
  if (render && d.mode !== 'source') {
    const sc = d.el.pv.scrollTop;
    renderPreview(d);
    d.el.pv.scrollTop = sc;
    if (d === doc()) { renderOutline(); renderBreadcrumbs(); renderStatus(); }
  }
}

function insertLink(d) {
  if (!d) return;
  const ta = d.live ? d.live.ta : d.mode !== 'read' ? d.el.ta : null;
  if (!ta) return;
  ta.focus();
  const s = ta.selectionStart, t = ta.selectionEnd, sel = ta.value.slice(s, t) || 'link text';
  insert(`[${sel}](https://)`);
  const at = s + sel.length + 3;
  ta.setSelectionRange(at, at + 8);
}

/** Clicking elsewhere while a block is open commits it, and opens the clicked block in one step. */
function liveEvents() {
  document.addEventListener('pointerdown', e => {
    const d = doc();
    if (!d?.live || e.button !== 0 || e.target.closest('.live-editor, #palette, #menu, #findwidget')) return;
    if (e.target.closest('a[href], input[data-task], .code-head')) { setTimeout(() => { if (d.live) commitLive(d); }, 250); return; }
    const block = d.mode === 'read' && e.target.closest('.markdown-body > [data-line]');
    if (block && d.el.article.contains(block)) { e.preventDefault(); startLiveEdit(d, block, caretHint(e, block)); return; }
    if (e.target.closest('.endmark')) { e.preventDefault(); startLiveAppend(d); return; }
    commitLive(d);
  }, true);
}

/* ───────────────────────── scrolling & navigation ───────────────────────── */

function lineTop(d, line) {
  const el = d.el.hl.children[line];
  return el ? el.offsetTop : (line * S.settings.editorSize * 1.7);
}
function lineAtTop(d, y) {
  const kids = d.el.hl.children;
  if (!kids.length) return 0;
  let lo = 0, hi = kids.length - 1;
  while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (kids[mid].offsetTop <= y) lo = mid; else hi = mid - 1; }
  const el = kids[lo];
  return lo + Math.min(1, Math.max(0, (y - el.offsetTop) / Math.max(1, el.offsetHeight)));
}
const blockTop = (d, el) => el.offsetTop + d.el.article.offsetTop;

function syncScroll(d, from) {
  if (!S.settings.syncScroll || !d.blocks?.length || d.large) return;
  const now = performance.now();
  if (d.syncFrom && d.syncFrom !== from && now - d.syncAt < 80) return;
  d.syncFrom = from; d.syncAt = now;
  const {src, pv} = d.el;
  const blocks = d.blocks;
  if (from === 'source') {
    if (src.scrollTop <= 0) { pv.scrollTop = 0; return; }
    if (src.scrollTop >= src.scrollHeight - src.clientHeight - 1) { pv.scrollTop = pv.scrollHeight; return; }
    const L = lineAtTop(d, src.scrollTop);
    let i = 0; while (i + 1 < blocks.length && blocks[i + 1].line <= L) i++;
    const a = blocks[i], b = blocks[i + 1];
    const ya = blockTop(d, a.el), yb = b ? blockTop(d, b.el) : pv.scrollHeight;
    const frac = b ? (L - a.line) / Math.max(1, b.line - a.line) : 0;
    pv.scrollTop = ya + Math.max(0, Math.min(1, frac)) * (yb - ya) - 24;
  } else {
    if (pv.scrollTop <= 0) { src.scrollTop = 0; return; }
    if (pv.scrollTop >= pv.scrollHeight - pv.clientHeight - 1) { src.scrollTop = src.scrollHeight; return; }
    const y = pv.scrollTop + 24;
    let i = 0; while (i + 1 < blocks.length && blockTop(d, blocks[i + 1].el) <= y) i++;
    const a = blocks[i], b = blocks[i + 1];
    const ya = blockTop(d, a.el), yb = b ? blockTop(d, b.el) : pv.scrollHeight;
    const frac = Math.max(0, Math.min(1, (y - ya) / Math.max(1, yb - ya)));
    const L = a.line + frac * ((b ? b.line : a.line + 1) - a.line);
    const li = Math.floor(L);
    src.scrollTop = lineTop(d, li) + (L - li) * (d.el.hl.children[li]?.offsetHeight || 20);
  }
}

function scrollPreviewTo(d, el, flash = true) {
  d.el.pv.scrollTo({top: blockTop(d, el) - 28, behavior: 'smooth'});
  if (flash) { el.classList.remove('flash'); void el.offsetWidth; el.classList.add('flash'); }
}

function revealLine(d, line, range) {
  if (!d.el) return;
  requestAnimationFrame(() => {
    if (d.mode !== 'read') {
      updateHighlight(d);
      const ta = d.el.ta;
      let off = 0;
      for (let i = 0; i < line && off !== -1; i++) off = ta.value.indexOf('\n', off) + 1;
      if (off < 0) off = 0;
      const start = off + (range?.start || 0);
      ta.focus({preventScroll: true});
      ta.setSelectionRange(start, start + (range?.length || 0));
      d.el.src.scrollTop = lineTop(d, line) - d.el.src.clientHeight / 3;
      updateCursor(d, true); renderStatusCursor();
    }
    if (d.mode !== 'source') {
      renderPreview(d);
      const blocks = d.blocks || [];
      let b = null; for (const x of blocks) { if (x.line <= line) b = x; else break; }
      if (b) scrollPreviewTo(d, b.el);
    }
  });
}

function revealHeading(d, hd) {
  if (d.mode === 'source') { revealLine(d, hd.line); return; }
  renderPreview(d);
  const el = d.el.article.querySelector(`[id="${CSS.escape(hd.id)}"]`);
  if (el) scrollPreviewTo(d, el);
  if (d.mode === 'split') revealLine(d, hd.line);
}

function revealAnchor(d, anchor) {
  const hd = analyze(d).headings.find(x => x.id === anchor || x.id === slugger()(anchor));
  if (hd) revealHeading(d, hd);
}

function trackHeading(d) {
  if (d !== doc()) return;
  const hs = analyze(d).headings;
  let cur = -1;
  if (d.mode === 'read' || (d.mode === 'split' && d.lastFocus !== 'source')) {
    const y = d.el.pv.scrollTop + 60;
    const els = hs.map(x => d.el.article.querySelector(`[id="${CSS.escape(x.id)}"]`));
    els.forEach((el, i) => { if (el && blockTop(d, el) <= y) cur = i; });
  } else {
    const L = lineAtTop(d, d.el.src.scrollTop + 30);
    hs.forEach((x, i) => { if (x.line <= L) cur = i; });
  }
  if (cur !== d.curHeading) { d.curHeading = cur; markOutline(); renderBreadcrumbs(); }
}

/* ───────────────────────── opening, tabs ───────────────────────── */

async function openFile(path, {preview = false, line = null, range = null, anchor = null, mode = null, background = false} = {}) {
  let d = docs.get(path);
  if (!d) {
    let r;
    try { r = await api('read', {path}); } catch (e) { toast(`Couldn’t open ${basename(path)} — ${e.message || e}`, 'error'); return null; }
    if (docs.has(path)) d = docs.get(path);
    else {
      d = {key: path, kind: 'md', path, name: basename(path), text: r.text, baseline: r.text, mtime: r.mtime,
        mode: mode || (isMarkdown(path) ? S.settings.defaultMode : 'source'), preview: preview && S.settings.previewTabs};
      docs.set(path, d);
      createDocView(d);
      const pv = d.preview ? tabs.find(k => k !== path && docs.get(k)?.preview && !isDirty(docs.get(k))) : null;
      if (pv) { const i = tabs.indexOf(pv); tabs.splice(i, 1, path); destroyDoc(pv); }
      else insertTab(path);
      addRecent(path);
    }
  } else if (!preview && d.preview) d.preview = false;
  if (!background) activate(d.key);
  if (line != null) revealLine(d, line, range);
  else if (anchor) revealAnchor(d, anchor);
  renderTabs(); persist();
  return d;
}

function insertTab(key) {
  const i = tabs.indexOf(active);
  tabs.splice(i < 0 ? tabs.length : i + 1, 0, key);
}

let untitledCount = 0;
function newUntitled(text = '') {
  const n = ++untitledCount;
  const key = 'untitled:' + uid();
  const d = {key, kind: 'md', path: null, name: `Untitled-${n}.md`, text, baseline: '', mode: S.settings.defaultMode, untitled: true};
  docs.set(key, d); createDocView(d); insertTab(key); activate(key);
  requestAnimationFrame(() => { if (d.mode === 'read') startLiveAppend(d); else d.el.ta.focus(); });
  return d;
}

function openSpecial(key, {background = false} = {}) {
  if (!docs.has(key)) {
    const d = {key, kind: key.slice(6), name: key === 'folio:settings' ? 'Settings' : 'Keyboard Shortcuts', mode: 'read'};
    d.el = {root: h('div', {class: 'doc special', dataset: {mode: 'special'}})};
    $('#docs').append(d.el.root);
    docs.set(key, d); insertTab(key);
  }
  if (!background) activate(key);
  renderTabs(); persist();
}

function destroyDoc(key) {
  const d = docs.get(key);
  if (d?.el?.root) d.el.root.remove();
  docs.delete(key);
}

function activate(key) {
  const prev = doc();
  if (prev && prev.key !== key && prev.el?.ta) prev.wasFocused = document.activeElement === prev.el.ta;
  active = key;
  const d = doc();
  if (d?.path) {
    for (const w of ws()?.roots || []) if (within(d.path, w)) { let p = dirname(d.path); while (within(p, w)) { expanded.add(p); if (p === w) break; p = dirname(p); } }
    selectedPath = d.path;
  }
  if (find.open) findRefreshSoon();
  renderAll();
  if (d?.el?.ta && d.mode !== 'read' && d.wasFocused) requestAnimationFrame(() => d.el.ta.focus({preventScroll: true}));
  persist();
}

async function closeTab(key, {force = false} = {}) {
  const d = docs.get(key);
  if (!d) return true;
  if (!force && isDirty(d)) {
    const r = await api('confirm', {message: `Do you want to save the changes you made to ${d.name}?`, detail: 'Your changes will be lost if you don’t save them.', buttons: ['Save', 'Cancel', 'Don’t Save'], warning: true});
    if (r === 1) return false;
    if (r === 0 && !(await saveDoc(d))) return false;
  }
  const i = tabs.indexOf(d.key);
  if (d.path) closedStack.push({path: d.path, mode: d.mode});
  tabs.splice(i, 1);
  destroyDoc(d.key);
  if (active === d.key) active = tabs[Math.min(i, tabs.length - 1)] ?? null;
  if (active) activate(active); else renderAll();
  notifyDirty(); persist();
  return true;
}

async function closeMany(keys) { for (const k of keys) if (!(await closeTab(k))) return false; return true; }

/* ───────────────────────── saving ───────────────────────── */

async function saveDoc(d, as = false) {
  if (!d || d.kind !== 'md') return false;
  let path = d.path;
  if (!path || as) {
    path = await api('saveDialog', {name: d.path ? basename(d.path) : d.name, dir: d.path ? dirname(d.path) : (ws()?.roots[0] || '')});
    if (!path) return false;
  }
  try {
    const r = await api('write', {path, text: d.text});
    if (path !== d.key) {
      if (docs.has(path) && docs.get(path) !== d) await closeTab(path, {force: true});
      const old = d.key;
      docs.delete(old); d.key = path; d.path = path; d.name = basename(path); d.untitled = false; docs.set(path, d);
      tabs = tabs.map(k => k === old ? path : k);
      if (active === old) active = path;
      d.rendered = null;
      api('allowDir', {path: dirname(path)});
      addRecent(path);
      refreshTrees();
    }
    d.baseline = d.text; d.mtime = r.mtime; d.preview = false; d.conflict = false; d.wasDirty = false;
    renderAll(); notifyDirty(); persist();
    return true;
  } catch (e) {
    toast(`Couldn’t save ${d.name} — ${e.message || e}`, 'error');
    return false;
  }
}

async function saveAll() {
  for (const d of dirtyDocs()) if (!(await saveDoc(d))) return false;
  return true;
}

async function resolveDirty(list, verb = 'continuing') {
  if (!list.length) return true;
  const r = await api('confirm', {message: list.length === 1 ? `Save changes to ${list[0].name} before ${verb}?` : `Save changes to ${list.length} files before ${verb}?`,
    detail: list.map(d => d.name).join(', '), buttons: ['Save All', 'Cancel', 'Don’t Save'], warning: true});
  if (r === 1) return false;
  if (r === 0) { for (const d of list) if (!(await saveDoc(d))) return false; }
  return true;
}

async function revert(d) {
  if (!d?.path) return;
  try { const r = await api('read', {path: d.path}); d.mtime = r.mtime; d.conflict = false; setText(d, r.text, true); renderAll(); } catch (e) { toast(String(e.message || e), 'error'); }
}

/* ───────────────────────── workspaces ───────────────────────── */

function newWorkspaceObject(name, roots = []) { return {id: uid(), name, roots, tabs: [], expanded: [...roots], created: Date.now(), opened: Date.now()}; }

async function loadWorkspace() {
  const w = ws();
  expanded = new Set(w.expanded || w.roots);
  trees.clear(); fileCache = null;
  api('roots', {roots: w.roots});
  for (const r of w.roots) api('allowDir', {path: r});
  refreshTrees();
  if (S.settings.restoreTabs) {
    for (const t of w.tabs || []) {
      if (t.key?.startsWith('folio:')) openSpecial(t.key, {background: true});
      else if (t.path) await openFile(t.path, {background: true, mode: t.mode});
    }
  }
  active = tabs.includes(w.active) ? w.active : tabs[0] ?? null;
  if (active) activate(active); else renderAll();
}

async function switchWorkspace(id) {
  if (id === S.current) return;
  if (!(await resolveDirty(dirtyDocs(), 'switching workspaces'))) return;
  await persistNow();
  for (const k of [...tabs]) destroyDoc(k);
  tabs = []; active = null; closedStack = [];
  S.current = id;
  await loadWorkspace();
  persist();
  toast(`Switched to ${ws().name}`, 'ok');
}

async function createWorkspace() {
  const name = await quickInput({placeholder: 'Name your new workspace', value: '', prompt: 'Press Enter to create the workspace, then add folders to it.'});
  if (!name?.trim()) return;
  const w = newWorkspaceObject(name.trim());
  S.workspaces.push(w);
  await switchWorkspace(w.id);
  showView('explorer');
}

async function renameWorkspace(w = ws()) {
  const name = await quickInput({placeholder: 'Workspace name', value: w.name, prompt: 'Rename workspace'});
  if (!name?.trim()) return;
  w.name = name.trim(); persist(); renderAll();
}

async function deleteWorkspace(w) {
  if (S.workspaces.length === 1) { toast('You need at least one workspace.', 'error'); return; }
  const r = await api('confirm', {message: `Remove the workspace “${w.name}”?`, detail: 'Only the workspace is removed. Your folders and files stay exactly where they are.', buttons: ['Remove Workspace', 'Cancel'], warning: true});
  if (r !== 0) return;
  if (w.id === S.current) { const other = S.workspaces.find(x => x.id !== w.id); await switchWorkspace(other.id); if (S.current === w.id) return; }
  S.workspaces = S.workspaces.filter(x => x.id !== w.id); persist(); renderAll();
}

async function addFolders(paths) {
  const w = ws();
  const added = [];
  for (const p of paths) if (!w.roots.includes(p)) { w.roots.push(p); expanded.add(p); added.push(p); api('allowDir', {path: p}); }
  if (!added.length) return;
  api('roots', {roots: w.roots}); fileCache = null;
  await refreshTrees();
  showView('explorer');
  persist();
  toast(added.length === 1 ? `Added ${basename(added[0])} to ${w.name}` : `Added ${added.length} folders to ${w.name}`, 'ok');
}

function removeFolder(root) {
  const w = ws();
  w.roots = w.roots.filter(r => r !== root); trees.delete(root); fileCache = null;
  api('roots', {roots: w.roots}); renderAll(); persist();
}

async function addFolderDialog() {
  const paths = await api('openDialog', {folders: true, prompt: 'Add', message: `Add folders to “${ws().name}”`});
  if (paths?.length) addFolders(paths);
}

async function openDialog() {
  const paths = await api('openDialog', {folders: false, message: 'Open Markdown files'});
  if (paths?.length) openPaths(paths);
}

async function openPaths(paths) {
  if (!paths?.length) return;
  const st = await api('stat', {paths});
  const folders = paths.filter(p => st[p]?.dir), files = paths.filter(p => st[p] && !st[p].dir);
  if (folders.length) await addFolders(folders);
  for (const f of files) await openFile(f, {background: f !== files[files.length - 1]});
}

/* ───────────────────────── file tree ───────────────────────── */

let treeSeq = 0;
async function refreshTrees() {
  const w = ws(); if (!w) return;
  const seq = ++treeSeq;
  const results = await Promise.all(w.roots.map(r => api('tree', {root: r, all: S.settings.showAllFiles}).catch(() => ({name: basename(r), path: r, dir: true, missing: true, children: []}))));
  if (seq !== treeSeq) return;
  trees.clear();
  w.roots.forEach((r, i) => trees.set(r, results[i]));
  renderExplorer();
}

function findNode(path) {
  for (const t of trees.values()) {
    const stack = [t];
    while (stack.length) { const n = stack.pop(); if (n.path === path) return n; if (n.children && within(path, n.path)) stack.push(...n.children); }
  }
  return null;
}

async function getFiles() {
  if (fileCache) return fileCache;
  const w = ws();
  fileCache = w.roots.length ? await api('listFiles', {roots: w.roots}) : [];
  return fileCache;
}

const rootOf = p => (ws()?.roots || []).filter(r => within(p, r)).sort((a, b) => b.length - a.length)[0];
const relPath = p => { const r = rootOf(p); return r ? (ws().roots.length > 1 ? basename(r) + '/' : '') + p.slice(r.length + 1) : tilde(p); };

async function newEntry(dirPath, isDir) {
  const parent = dirPath || ws().roots[0];
  if (!parent) { const d = newUntitled(); return d; }
  expanded.add(parent);
  renderExplorer();
  const name = await inlineInput(parent, '', isDir ? 'folder' : 'file');
  if (!name) return;
  let finalName = name.trim();
  if (!isDir && !/\.[^.]+$/.test(finalName)) finalName += '.md';
  const path = parent.replace(/\/$/, '') + '/' + finalName;
  try {
    await api('create', {path, dir: isDir, text: isDir ? '' : `# ${stripExt(basename(finalName))}\n\n`});
    await refreshTrees();
    if (!isDir) { const d = await openFile(path); if (d) requestAnimationFrame(() => { if (d.mode === 'read') startLiveAppend(d); else { d.el.ta.focus(); d.el.ta.setSelectionRange(d.text.length, d.text.length); } }); }
    else { expanded.add(path); selectedPath = path; renderExplorer(); }
  } catch (e) { toast(String(e.message || e), 'error'); }
}

async function renameEntry(path) {
  const node = findNode(path);
  const name = await inlineInput(path, basename(path), node?.dir ? 'folder' : 'file', true);
  if (!name || name === basename(path)) return;
  await movePath(path, dirname(path) + '/' + name.trim());
}

async function movePath(from, to) {
  if (from === to || within(to, from)) return;
  try {
    await api('rename', {from, to});
    for (const d of [...docs.values()]) {
      if (d.path && within(d.path, from)) {
        const np = to + d.path.slice(from.length), old = d.key;
        docs.delete(old); d.key = np; d.path = np; d.name = basename(np); docs.set(np, d);
        tabs = tabs.map(k => k === old ? np : k); if (active === old) active = np;
        d.rendered = null;
      }
    }
    if (expanded.has(from)) { expanded.delete(from); expanded.add(to); }
    selectedPath = to;
    fileCache = null; await refreshTrees(); renderAll(); persist();
  } catch (e) { toast(String(e.message || e), 'error'); }
}

async function trashEntry(path) {
  const node = findNode(path);
  const r = await api('confirm', {message: `Move “${basename(path)}” to the Trash?`, detail: node?.dir ? 'The folder and everything in it will be moved to the Trash.' : 'You can restore it from the Trash.', buttons: ['Move to Trash', 'Cancel'], warning: true});
  if (r !== 0) return;
  try {
    await api('trash', {path});
    for (const d of [...docs.values()]) if (d.path && within(d.path, path)) { d.baseline = d.text; await closeTab(d.key, {force: true}); }
    fileCache = null; await refreshTrees();
    toast(`Moved ${basename(path)} to the Trash`, 'ok');
  } catch (e) { toast(String(e.message || e), 'error'); }
}

/** Inline name editor inside the explorer. Resolves with the text or null. */
let pendingInline = null;
function inlineInput(anchorPath, value, kind, renaming = false) {
  return new Promise(resolve => {
    pendingInline = {anchorPath, value, kind, renaming, resolve};
    showView('explorer');
    renderExplorer();
  });
}

/* ───────────────────────── explorer rendering ───────────────────────── */

const fileIcon = (name, dir, open) => dir ? icon(open ? 'folderOpen' : 'folder', 'fi-folder') : isMarkdown(name) ? icon('md', 'fi-md') : icon('file', 'fi-file');
const renderExplorerSoon = debounce(() => renderExplorer(), 30);

function sectionEl(id, title, open, actions, body) {
  const head = h('div', {class: 'sec-head' + (open ? ' open' : ''), tabindex: 0, html: icon('chevron', 'twisty') + `<span>${esc(title)}</span>`});
  const acts = h('div', {class: 'sec-actions'});
  for (const [ic, label, fn] of actions) acts.append(h('button', {class: 'icon-btn', title: label, html: icon(ic), onclick: e => { e.stopPropagation(); fn(); }}));
  head.append(acts);
  head.addEventListener('click', () => { S.settings[id] = !open; persist(); renderExplorer(); });
  return h('div', {class: 'sec' + (open ? ' open' : '')}, head, open ? body : null);
}

function renderExplorer() {
  const body = $('#explorer-body');
  if (!body || S.settings.view !== 'explorer' && !pendingInline) { return; }
  const scroll = body.scrollTop;
  body.replaceChildren();
  const w = ws();
  // Open editors
  const list = h('div', {class: 'open-editors'});
  for (const k of tabs) {
    const d = docs.get(k);
    const row = h('div', {class: 'row oe' + (k === active ? ' active' : '') + (isDirty(d) ? ' dirty' : ''), title: d.path ? tilde(d.path) : d.name,
      html: `<button class="icon-btn close" title="Close">${icon('x')}</button>${d.kind === 'md' ? fileIcon(d.name) : icon(d.kind === 'settings' ? 'settings' : 'keyboard', 'fi-file')}<span class="label">${esc(d.name)}</span><span class="desc">${esc(d.path ? relPath(dirname(d.path)) : '')}</span>`});
    row.addEventListener('click', e => { if (e.target.closest('.close')) closeTab(k); else activate(k); });
    row.addEventListener('auxclick', e => { if (e.button === 1) closeTab(k); });
    list.append(row);
  }
  if (!tabs.length) list.append(h('div', {class: 'empty-note'}, 'No open editors'));
  body.append(sectionEl('openEditors', 'Open Editors', S.settings.openEditors, [['x', 'Close All', () => command('closeAll')]], list));

  // Workspace folders
  const tree = h('div', {class: 'tree', role: 'tree'});
  if (!w.roots.length) {
    tree.append(h('div', {class: 'empty-state'},
      h('p', {}, `“${w.name}” has no folders yet. Add a folder of Markdown files to browse, search and edit them here.`),
      h('button', {class: 'btn primary', onclick: addFolderDialog, html: icon('folderPlus') + 'Add Folder'}),
      h('button', {class: 'btn', onclick: () => command('newFile'), html: icon('filePlus') + 'New File'})));
  }
  for (const root of w.roots) {
    const t = trees.get(root) || {name: basename(root), path: root, dir: true, children: [], loading: true};
    renderNode(tree, t, 0, true);
  }
  if (pendingInline && !pendingInline.renaming && !w.roots.length) { pendingInline.resolve(null); pendingInline = null; }
  const wsTitle = w.name;
  body.append(sectionEl('wsOpen', wsTitle, S.settings.wsOpen !== false, [
    ['filePlus', 'New File…', () => newEntry(selectedDir(), false)],
    ['folderPlus', 'New Folder…', () => newEntry(selectedDir(), true)],
    ['refresh', 'Refresh Explorer', () => { fileCache = null; refreshTrees(); }],
    ['collapse', 'Collapse Folders', () => { expanded = new Set(); renderExplorer(); persist(); }],
  ], tree));
  if (S.settings.wsOpen === undefined) S.settings.wsOpen = true;
  body.scrollTop = scroll;
  const input = $('.inline-input input', body);
  if (input) { input.focus(); const dot = input.value.lastIndexOf('.'); input.setSelectionRange(0, pendingInline?.renaming && dot > 0 ? dot : input.value.length); }
}

function selectedDir() {
  if (!selectedPath) return ws().roots[0];
  const n = findNode(selectedPath);
  return n?.dir ? n.path : dirname(selectedPath);
}

function inlineRow(depth, kind, value) {
  const row = h('div', {class: 'row inline-input', style: {'--d': depth}, html: guides(depth) + '<span class="twisty-space"></span>' + (kind === 'folder' ? icon('folder', 'fi-folder') : icon('md', 'fi-md'))});
  const input = h('input', {value, spellcheck: 'false', placeholder: kind === 'folder' ? 'Folder name' : 'File name'});
  let done = false;
  const finish = v => { if (done) return; done = true; const p = pendingInline; pendingInline = null; p?.resolve(v); renderExplorer(); };
  input.addEventListener('keydown', e => { if (e.key === 'Enter') finish(input.value.trim() || null); if (e.key === 'Escape') finish(null); e.stopPropagation(); });
  input.addEventListener('blur', () => finish(input.value.trim() || null));
  row.append(input);
  return row;
}

const guides = depth => Array.from({length: depth}, (_, i) => `<span class="guide" style="--g:${i}"></span>`).join('');

function renderNode(container, n, depth, isRoot = false) {
  const open = expanded.has(n.path);
  const d = docs.get(n.path);
  if (pendingInline?.renaming && pendingInline.anchorPath === n.path) { container.append(inlineRow(depth, pendingInline.kind, pendingInline.value)); }
  else {
    const cls = ['row', n.dir ? 'dir' : 'file'];
    if (isRoot) cls.push('root');
    if (n.path === active) cls.push('active');
    if (n.path === selectedPath) cls.push('selected');
    if (isDirty(d)) cls.push('dirty');
    if (d) cls.push('opened');
    if (n.missing) cls.push('missing');
    const row = h('div', {class: cls.join(' '), role: 'treeitem', dataset: {path: n.path}, style: {'--d': depth}, title: tilde(n.path),
      html: guides(depth) + (n.dir ? icon('chevron', 'twisty' + (open ? ' open' : '')) : '<span class="twisty-space"></span>') + fileIcon(n.name, n.dir, open)
        + `<span class="label">${esc(n.name)}</span>` + (n.missing ? '<span class="desc">missing</span>' : '') + (isDirty(d) ? '<span class="dot"></span>' : '')});
    container.append(row);
  }
  if (n.dir && open) {
    if (pendingInline && !pendingInline.renaming && pendingInline.anchorPath === n.path) container.append(inlineRow(depth + 1, pendingInline.kind, ''));
    for (const c of n.children || []) renderNode(container, c, depth + 1);
    if (isRoot && !(n.children || []).length && !n.loading) container.append(h('div', {class: 'empty-note', style: {'--d': 1}}, n.missing ? 'Folder not found' : S.settings.showAllFiles ? 'Empty folder' : 'No Markdown files here'));
  }
}

let suppressTreeClick = false;
function explorerEvents() {
  const body = $('#explorer-body');
  body.addEventListener('click', e => {
    if (suppressTreeClick) return;
    const row = e.target.closest('.tree .row[data-path]'); if (!row) return;
    const path = row.dataset.path, n = findNode(path);
    selectedPath = path;
    if (n?.dir) { expanded.has(path) ? expanded.delete(path) : expanded.add(path); renderExplorer(); persist(); }
    else openFile(path, {preview: true});
  });
  body.addEventListener('dblclick', e => {
    const row = e.target.closest('.tree .row.file'); if (!row) return;
    const d = docs.get(row.dataset.path); if (d) { d.preview = false; renderTabs(); }
  });
  body.addEventListener('contextmenu', e => {
    const row = e.target.closest('.tree .row[data-path]');
    e.preventDefault();
    if (!row) { showMenu(e.clientX, e.clientY, [['New File…', () => newEntry(ws().roots[0], false)], ['Add Folder to Workspace…', addFolderDialog]]); return; }
    const path = row.dataset.path, n = findNode(path), isRoot = ws().roots.includes(path);
    selectedPath = path; renderExplorer();
    const dir = n?.dir ? path : dirname(path);
    const items = [];
    if (!n?.dir) items.push(['Open', () => openFile(path)], ['Open in Split View', () => openFile(path, {mode: 'split'})], null);
    items.push(['New File…', () => newEntry(dir, false)], ['New Folder…', () => newEntry(dir, true)], null);
    items.push(['Reveal in Finder', () => api('reveal', {path}), '⌥⌘R'], ['Copy Path', () => { api('copy', {text: path}); toast('Path copied', 'ok'); }], ['Copy Relative Path', () => { api('copy', {text: relPath(path)}); toast('Relative path copied', 'ok'); }]);
    if (n?.dir) items.push(null, ['Find in Folder…', () => { showView('search'); $('#search-input').focus(); searchScope = path; runSearch(); }]);
    items.push(null);
    if (isRoot) items.push(['Remove Folder from Workspace', () => removeFolder(path)]);
    else items.push(['Rename…', () => renameEntry(path), '↵'], ['Move to Trash', () => trashEntry(path), '⌘⌫', true]);
    showMenu(e.clientX, e.clientY, items);
  });
  body.addEventListener('pointerdown', e => {
    const row = e.target.closest('.tree .row[data-path]');
    if (!row || e.button !== 0 || row.classList.contains('root') || e.target.tagName === 'INPUT') return;
    const from = row.dataset.path, x0 = e.clientX, y0 = e.clientY;
    let dragging = false, dest = null, ghost = null;
    const clear = () => $$('.drop-target', body).forEach(x => x.classList.remove('drop-target'));
    const move = ev => {
      if (!dragging && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 6) return;
      if (!dragging) {
        dragging = true; document.body.classList.add('dragging-row');
        ghost = h('div', {class: 'drag-ghost', html: fileIcon(basename(from), !!findNode(from)?.dir) + esc(basename(from))});
        document.body.append(ghost);
      }
      ghost.style.transform = `translate(${ev.clientX + 12}px, ${ev.clientY + 8}px)`;
      clear(); dest = null;
      const over = document.elementFromPoint(ev.clientX, ev.clientY)?.closest('.tree .row[data-path]');
      if (!over) return;
      const n = findNode(over.dataset.path);
      const dir = n?.dir ? n.path : dirname(over.dataset.path);
      if (dir === dirname(from) || within(dir, from)) return;
      dest = dir;
      const target = $(`.tree .row[data-path="${CSS.escape(dir)}"]`, body);
      (target || over).classList.add('drop-target');
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      document.body.classList.remove('dragging-row');
      ghost?.remove(); clear();
      if (dragging) { suppressTreeClick = true; setTimeout(() => { suppressTreeClick = false; }, 0); }
      if (dragging && dest) movePath(from, dest + '/' + basename(from));
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up, {once: true});
  });
  body.addEventListener('keydown', e => {
    if (e.target.tagName === 'INPUT') return;
    const rows = $$('.tree .row[data-path]', body);
    let i = rows.findIndex(r => r.dataset.path === selectedPath);
    const n = selectedPath && findNode(selectedPath);
    if (e.key === 'ArrowDown') i = Math.min(rows.length - 1, i + 1);
    else if (e.key === 'ArrowUp') i = Math.max(0, i - 1);
    else if (e.key === 'ArrowRight' && n?.dir) { expanded.add(n.path); renderExplorer(); return; }
    else if (e.key === 'ArrowLeft' && n) { if (n.dir && expanded.has(n.path)) expanded.delete(n.path); else { const p = dirname(n.path); if (findNode(p)) selectedPath = p; } renderExplorer(); return; }
    else if (e.key === 'Enter' && n) { e.preventDefault(); if (n.dir) { expanded.has(n.path) ? expanded.delete(n.path) : expanded.add(n.path); renderExplorer(); } else renameEntry(n.path); return; }
    else if (e.key === ' ' && n && !n.dir) { e.preventDefault(); openFile(n.path, {preview: true}); return; }
    else if ((e.key === 'Backspace' && e.metaKey) && n && !ws().roots.includes(n.path)) { trashEntry(n.path); return; }
    else return;
    e.preventDefault();
    if (rows[i]) { selectedPath = rows[i].dataset.path; renderExplorer(); $('.row.selected', body)?.scrollIntoView({block: 'nearest'}); }
  });
}

/* ───────────────────────── search view ───────────────────────── */

const searchOpts = {case: false, word: false, regex: false};
let searchScope = null;
let searchSeq = 0;
let lastSearch = null;
const runSearchSoon = debounce(() => runSearch(), 260);

async function runSearch() {
  const q = $('#search-input').value;
  const out = $('#search-results'), sum = $('#search-summary');
  const roots = searchScope ? [searchScope] : ws().roots;
  if (!q) { out.replaceChildren(); sum.textContent = ''; lastSearch = null; return; }
  if (!roots.length) { sum.textContent = 'Add a folder to this workspace to search it.'; out.replaceChildren(); return; }
  const seq = ++searchSeq;
  sum.innerHTML = '<span class="spinner"></span> Searching…';
  let r;
  try { r = await api('search', {roots, query: q, ...searchOpts}); } catch (e) { r = {error: String(e.message || e)}; }
  if (seq !== searchSeq) return;
  if (r.error) { sum.textContent = r.error; out.replaceChildren(); return; }
  lastSearch = r;
  const files = r.results.length;
  sum.innerHTML = r.total ? `${plural(r.total, 'result')} in ${plural(files, 'file')}${searchScope ? ` · in <b>${esc(basename(searchScope))}</b> <button class="link" id="clear-scope">clear</button>` : ''}` : `No results${searchScope ? ` in ${esc(basename(searchScope))} <button class="link" id="clear-scope">search everywhere</button>` : ''}`;
  $('#clear-scope')?.addEventListener('click', () => { searchScope = null; runSearch(); });
  const frag = document.createDocumentFragment();
  for (const f of r.results) {
    const group = h('div', {class: 'sr-file open'});
    const head = h('div', {class: 'row sr-head', title: tilde(f.path), html: icon('chevron', 'twisty open') + fileIcon(f.path) + `<span class="label">${esc(basename(f.path))}</span><span class="desc">${esc(relPath(dirname(f.path)))}</span><span class="badge">${f.matches.length}</span>`});
    head.addEventListener('click', () => { group.classList.toggle('open'); $('.twisty', head).classList.toggle('open'); });
    group.append(head);
    const list = h('div', {class: 'sr-list'});
    for (const m of f.matches.slice(0, 200)) {
      const t = m.text, s = m.start, e = s + m.length;
      const pre = t.slice(Math.max(0, s - 40), s).replace(/^\s+/, ''), mid = t.slice(s, e), post = t.slice(e, e + 120);
      const row = h('div', {class: 'row sr-match', title: `Line ${m.line}`, html: `<span class="ln-no">${m.line}</span><span class="label">${s > 40 ? '…' : ''}${esc(pre)}<mark>${esc(mid)}</mark>${esc(post)}</span>`});
      row.addEventListener('click', () => {
        $$('.sr-match.selected').forEach(x => x.classList.remove('selected')); row.classList.add('selected');
        const d = docs.get(f.path);
        openFile(f.path, {preview: !d, line: m.line - 1, range: {start: s, length: m.length}});
      });
      list.append(row);
    }
    group.append(list);
    frag.append(group);
  }
  out.replaceChildren(frag);
}

/* ───────────────────────── outline view ───────────────────────── */

function renderOutline() {
  const body = $('#outline-body');
  if (S.settings.view !== 'outline') return;
  const d = doc();
  body.replaceChildren();
  if (!d || d.kind !== 'md') { body.append(h('div', {class: 'empty-note pad'}, 'Open a Markdown file to see its outline.')); return; }
  const hs = analyze(d).headings;
  const q = $('#outline-filter').value.trim().toLowerCase();
  if (!hs.length) { body.append(h('div', {class: 'empty-note pad'}, 'This document has no headings yet.')); return; }
  const min = Math.min(...hs.map(x => x.depth));
  hs.forEach((x, i) => {
    if (q && !x.text.toLowerCase().includes(q)) return;
    const row = h('div', {class: 'row ol' + (i === d.curHeading ? ' current' : ''), dataset: {i}, style: {'--d': x.depth - min}, title: x.text,
      html: guides(x.depth - min) + `<span class="hd-level">H${x.depth}</span><span class="label">${esc(x.text || '(untitled)')}</span><span class="desc">${x.line + 1}</span>`});
    row.addEventListener('click', () => revealHeading(d, x));
    body.append(row);
  });
}
function markOutline() {
  const d = doc();
  $$('#outline-body .row.ol').forEach(r => r.classList.toggle('current', +r.dataset.i === d?.curHeading));
  $('#outline-body .row.current')?.scrollIntoView({block: 'nearest'});
}

/* ───────────────────────── workspaces view ───────────────────────── */

function renderWorkspaces() {
  const body = $('#workspaces-body');
  if (S.settings.view !== 'workspaces') return;
  body.replaceChildren();
  const cur = ws();
  const list = h('div', {class: 'ws-list'});
  for (const w of [...S.workspaces].sort((a, b) => (b.id === cur.id) - (a.id === cur.id) || (b.opened || 0) - (a.opened || 0))) {
    const isCur = w.id === cur.id;
    const card = h('div', {class: 'ws-card' + (isCur ? ' current' : '')});
    const initials = w.name.split(/\s+/).map(x => x[0]).join('').slice(0, 2).toUpperCase();
    const hue = [...w.id].reduce((a, ch) => a + ch.charCodeAt(0), 0) % 360;
    card.append(
      h('div', {class: 'ws-avatar', style: {'--hue': hue}}, initials),
      h('div', {class: 'ws-meta'},
        h('div', {class: 'ws-name'}, w.name, isCur ? h('span', {class: 'pill'}, 'Current') : null),
        h('div', {class: 'ws-sub'}, `${plural(w.roots.length, 'folder')} · ${plural((isCur ? tabs.length : w.tabs?.length || 0), 'tab')}${isCur ? '' : ' · ' + timeAgo(w.opened)}`)),
      h('button', {class: 'icon-btn', title: 'More', html: icon('more'), onclick: e => { e.stopPropagation(); const b = e.currentTarget.getBoundingClientRect();
        showMenu(b.left, b.bottom + 4, [isCur ? null : ['Open Workspace', () => switchWorkspace(w.id)], ['Rename…', () => renameWorkspace(w)], ['Remove Workspace…', () => deleteWorkspace(w), '', true]].filter(x => x !== null)); }}));
    if (!isCur) card.addEventListener('click', () => switchWorkspace(w.id));
    if (isCur) {
      const folders = h('div', {class: 'ws-folders'});
      for (const r of w.roots) folders.append(h('div', {class: 'ws-folder', title: tilde(r), html: icon('folder', 'fi-folder') + `<span class="label">${esc(basename(r))}</span><span class="desc">${esc(tilde(dirname(r)))}</span>`},
        h('button', {class: 'icon-btn', title: 'Remove from workspace', html: icon('x'), onclick: e => { e.stopPropagation(); removeFolder(r); }})));
      folders.append(h('button', {class: 'btn ghost small', html: icon('folderPlus') + 'Add Folder…', onclick: addFolderDialog}));
      card.append(folders);
    }
    list.append(card);
  }
  body.append(list, h('div', {class: 'ws-foot'}, h('button', {class: 'btn primary block', html: icon('plus') + 'New Workspace', onclick: createWorkspace}),
    h('p', {class: 'hint'}, 'A workspace is a set of folders plus the tabs you had open. Switch between them to jump between projects.')));
}

/* ───────────────────────── tabs, breadcrumbs, status ───────────────────────── */

function renderTabs() {
  const bar = $('#tabs');
  bar.replaceChildren();
  const names = {};
  for (const k of tabs) { const d = docs.get(k); names[d.name] = (names[d.name] || 0) + 1; }
  for (const k of tabs) {
    const d = docs.get(k);
    const dirty = isDirty(d);
    const t = h('div', {class: 'tab' + (k === active ? ' active' : '') + (dirty ? ' dirty' : '') + (d.preview ? ' preview' : '') + (d.conflict ? ' conflict' : ''), role: 'tab', dataset: {key: k}, title: d.path ? tilde(d.path) : d.name,
      html: (d.kind === 'md' ? fileIcon(d.name) : icon(d.kind === 'settings' ? 'settings' : 'keyboard', 'fi-file')) + `<span class="label">${esc(d.name)}</span>`
        + (names[d.name] > 1 && d.path ? `<span class="desc">${esc(basename(dirname(d.path)))}</span>` : '')
        + `<button class="tab-close" title="Close (⌘W)">${icon('x')}${icon('dot', 'dirty-dot')}</button>`});
    bar.append(t);
  }
  $('.tab.active', bar)?.scrollIntoView({block: 'nearest', inline: 'nearest'});
  $('#tabbar').classList.toggle('empty', !tabs.length);
  renderEditorActions();
}

function tabEvents() {
  const bar = $('#tabs');
  bar.addEventListener('mousedown', e => {
    const t = e.target.closest('.tab'); if (!t || e.button !== 0 || e.target.closest('.tab-close')) return;
    if (t.dataset.key !== active) activate(t.dataset.key);
  });
  bar.addEventListener('click', e => { const t = e.target.closest('.tab'); if (t && e.target.closest('.tab-close')) closeTab(t.dataset.key); });
  bar.addEventListener('auxclick', e => { const t = e.target.closest('.tab'); if (t && e.button === 1) closeTab(t.dataset.key); });
  bar.addEventListener('dblclick', e => { const t = e.target.closest('.tab'); if (t) { const d = docs.get(t.dataset.key); d.preview = false; renderTabs(); } });
  bar.addEventListener('wheel', e => { if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) { bar.scrollLeft += e.deltaY; e.preventDefault(); } }, {passive: false});
  bar.addEventListener('contextmenu', e => {
    const t = e.target.closest('.tab'); if (!t) return; e.preventDefault();
    const k = t.dataset.key, d = docs.get(k);
    showMenu(e.clientX, e.clientY, [
      ['Close', () => closeTab(k), '⌘W'], ['Close Others', () => closeMany(tabs.filter(x => x !== k))], ['Close to the Right', () => closeMany(tabs.slice(tabs.indexOf(k) + 1))],
      ['Close Saved', () => closeMany(tabs.filter(x => !isDirty(docs.get(x))))], ['Close All', () => closeMany([...tabs]), '⌥⌘W'], null,
      d.path ? ['Copy Path', () => { api('copy', {text: d.path}); toast('Path copied', 'ok'); }] : null,
      d.path ? ['Reveal in Finder', () => api('reveal', {path: d.path}), '⌥⌘R'] : null,
      d.path ? ['Reveal in Explorer', () => { selectedPath = d.path; showView('explorer'); }] : null,
      d.preview ? ['Keep Open', () => { d.preview = false; renderTabs(); }] : null,
    ].filter(x => x !== null));
  });
  bar.addEventListener('pointerdown', e => {
    const t = e.target.closest('.tab');
    if (!t || e.button !== 0 || e.target.closest('.tab-close')) return;
    const key = t.dataset.key, x0 = e.clientX;
    let dragging = false, target = null, after = false;
    const clear = () => $$('.tab', bar).forEach(x => x.classList.remove('drop-before', 'drop-after'));
    const move = ev => {
      if (!dragging && Math.abs(ev.clientX - x0) < 6) return;
      if (!dragging) { dragging = true; document.body.classList.add('dragging-tab'); }
      const src = $(`.tab[data-key="${CSS.escape(key)}"]`, bar);
      src?.classList.add('dragging');
      const over = document.elementFromPoint(ev.clientX, bar.getBoundingClientRect().top + 12)?.closest('.tab');
      clear(); target = null;
      if (over && over.dataset.key !== key) {
        const r = over.getBoundingClientRect(); after = ev.clientX >= r.left + r.width / 2;
        over.classList.add(after ? 'drop-after' : 'drop-before'); target = over.dataset.key;
      }
      const br = bar.getBoundingClientRect();
      if (ev.clientX > br.right - 30) bar.scrollLeft += 12; else if (ev.clientX < br.left + 30) bar.scrollLeft -= 12;
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      document.body.classList.remove('dragging-tab');
      clear(); $$('.tab.dragging', bar).forEach(x => x.classList.remove('dragging'));
      if (dragging && target) {
        tabs.splice(tabs.indexOf(key), 1);
        tabs.splice(tabs.indexOf(target) + (after ? 1 : 0), 0, key);
        renderTabs(); renderExplorer(); persist();
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up, {once: true});
  });
}

function renderEditorActions() {
  const box = $('#editor-actions');
  const d = doc();
  box.replaceChildren();
  if (!d || d.kind !== 'md') return;
  const seg = h('div', {class: 'segmented', role: 'group', 'aria-label': 'View mode'});
  for (const [m, ic, label, key] of [['read', 'edit', 'Live', '⌘1'], ['split', 'columns', 'Split', '⌘2'], ['source', 'code', 'Source', '⌘3']]) {
    seg.append(h('button', {class: d.mode === m ? 'on' : '', title: `${label} (${key})`, html: icon(ic) + `<span>${label}</span>`, onclick: () => setMode(d, m)}));
  }
  box.append(seg, h('button', {class: 'icon-btn', title: 'More Actions', html: icon('more'), onclick: e => {
    const b = e.currentTarget.getBoundingClientRect();
    showMenu(b.right - 220, b.bottom + 4, [
      ['Find', () => command('find'), '⌘F'], ['Go to Heading…', () => command('gotoHeading'), '⇧⌘O'], null,
      [`${S.settings.syncScroll ? '✓ ' : ''}Sync Scrolling`, () => command('toggleSync')], [`${S.settings.wrap ? '✓ ' : ''}Word Wrap`, () => command('wrap'), '⌥Z'], [`${S.settings.lineNumbers ? '✓ ' : ''}Line Numbers`, () => command('lineNumbers')], null,
      ['Export as HTML…', () => command('exportHtml')], ['Copy as HTML', () => command('copyHtml')], null,
      d.path ? ['Reveal in Finder', () => command('reveal'), '⌥⌘R'] : null, d.path ? ['Revert File', () => command('revert')] : null,
    ].filter(x => x !== null));
  }}));
}

function renderBreadcrumbs() {
  const bc = $('#breadcrumbs');
  const d = doc();
  bc.replaceChildren();
  bc.hidden = !d;
  if (!d) return;
  if (d.kind !== 'md') { bc.append(h('span', {class: 'crumb', html: icon(d.kind === 'settings' ? 'settings' : 'keyboard') + esc(d.name)})); return; }
  const parts = [];
  if (d.path) {
    const root = rootOf(d.path);
    const rel = root ? d.path.slice(root.length + 1).split('/') : tilde(d.path).split('/').filter(Boolean);
    if (root) parts.push({label: basename(root), path: root, folder: true});
    let acc = root || '';
    rel.forEach((seg, i) => { acc = root ? acc + '/' + seg : acc; parts.push({label: seg, path: root ? acc : null, folder: i < rel.length - 1, file: i === rel.length - 1}); });
  } else parts.push({label: d.name, file: true});
  parts.forEach((p, i) => {
    if (i) bc.append(h('span', {class: 'sep', html: icon('chevron')}));
    const el = h('button', {class: 'crumb' + (p.file ? ' file' : ''), html: (p.file ? fileIcon(p.label) : p.folder && i === 0 ? icon('folder', 'fi-folder') : '') + esc(p.label)});
    if (p.path) el.addEventListener('click', () => { if (p.folder) { expanded.add(p.path); selectedPath = p.path; showView('explorer'); } else openPalette('@'); });
    else el.addEventListener('click', () => openPalette('@'));
    bc.append(el);
  });
  const hs = analyze(d).headings, cur = hs[d.curHeading];
  if (cur) {
    const chain = []; let depth = 99;
    for (let i = d.curHeading; i >= 0; i--) if (hs[i].depth < depth) { chain.unshift(hs[i]); depth = hs[i].depth; }
    for (const x of chain) { bc.append(h('span', {class: 'sep', html: icon('chevron')}), h('button', {class: 'crumb heading', html: icon('hash') + esc(x.text), onclick: () => revealHeading(d, x)})); }
  }
}

function renderStatus() {
  const L = $('#statusbar .sb-left'), R = $('#statusbar .sb-right');
  const d = doc(), w = ws();
  L.replaceChildren(); R.replaceChildren();
  L.append(h('button', {class: 'sb-item ws', title: 'Switch Workspace (⌃⌘R)', html: icon('layers') + esc(w.name), onclick: () => command('switchWorkspace')}));
  const dc = dirtyDocs().length;
  L.append(h('span', {class: 'sb-item' + (dc ? ' warn' : ''), title: dc ? 'Unsaved files' : 'All changes saved', html: dc ? icon('dot') + `${dc} unsaved` : icon('check') + 'Saved'}));
  if (d?.conflict) L.append(h('span', {class: 'sb-item warn', html: icon('warning') + 'Changed on disk'}));
  if (d?.kind === 'md') {
    const a = analyze(d);
    R.append(h('span', {class: 'sb-item', id: 'sb-cursor'}));
    R.append(h('span', {class: 'sb-item', title: 'Words · reading time'}, `${plural(a.words, 'word')} · ${Math.max(1, Math.round(a.words / 230))} min read`));
    R.append(h('button', {class: 'sb-item', title: 'Change view mode', onclick: () => cycleMode(d)}, {read: 'Live', split: 'Split', source: 'Source'}[d.mode]));
    R.append(h('span', {class: 'sb-item'}, isMarkdown(d.name) ? 'Markdown' : 'Plain Text'));
    R.append(h('span', {class: 'sb-item'}, 'UTF-8'));
    renderStatusCursor();
  }
  R.append(h('button', {class: 'sb-item', title: 'Theme', html: icon(THEMES[S.settings.theme].dark ? 'moon' : 'sun'), onclick: () => command('pickTheme')}));
}
function renderStatusCursor() {
  const el = $('#sb-cursor'), d = doc();
  if (!el || !d || d.kind !== 'md') return;
  el.textContent = d.mode === 'read' || !d.cursor ? `${plural(d.text.split('\n').length, 'line')}` : `Ln ${d.cursor.line + 1}, Col ${d.cursor.col}${d.cursor.sel ? ` (${d.cursor.sel} selected)` : ''}`;
}

function updateTitle() {
  const d = doc(), w = ws();
  $('#cc-label').textContent = w.name;
  api('window', {title: d ? `${d.name}${isDirty(d) ? ' •' : ''} — ${w.name}` : `Folio — ${w.name}`, path: d?.path || ''});
}

/* ───────────────────────── editor area ───────────────────────── */

function setMode(d, mode) {
  if (!d || d.kind !== 'md' || d.mode === mode) return;
  if (d.live) commitLive(d, false);
  const pvRatio = d.el.pv.scrollTop / Math.max(1, d.el.pv.scrollHeight - d.el.pv.clientHeight);
  const topLine = d.mode !== 'read' ? lineAtTop(d, d.el.src.scrollTop) : null;
  const prev = d.mode;
  d.mode = mode;
  d.el.root.dataset.mode = mode;
  if (mode !== 'read') updateHighlight(d);
  if (mode !== 'source') renderPreview(d);
  applyLayout();
  requestAnimationFrame(() => {
    if (prev === 'read' && mode !== 'read') {
      // Keep the reader's position when jumping into the source.
      const y = d.el.pv.scrollTop + 24;
      let b = d.blocks?.[0]; for (const x of d.blocks || []) { if (blockTop(d, x.el) <= y) b = x; else break; }
      if (b) d.el.src.scrollTop = lineTop(d, b.line) - 8;
      d.el.ta.focus({preventScroll: true});
    } else if (prev !== 'read' && mode === 'read' && topLine != null) {
      let b = null; for (const x of d.blocks || []) { if (x.line <= topLine) b = x; else break; }
      d.el.pv.scrollTop = b ? blockTop(d, b.el) - 24 : pvRatio * (d.el.pv.scrollHeight - d.el.pv.clientHeight);
    }
  });
  renderEditorActions(); renderStatus(); renderTitlebar(); persist();
}
const cycleMode = d => setMode(d, {read: 'split', split: 'source', source: 'read'}[d.mode]);

function applyLayout() {
  const s = S.settings;
  document.documentElement.style.setProperty('--split', (s.split * 100) + '%');
  document.documentElement.style.setProperty('--sidebar-w', s.sidebarWidth + 'px');
  document.body.classList.toggle('no-sidebar', !s.sidebarVisible);
  document.body.classList.toggle('zen', !!s.zen);
}

function renderEditor() {
  const d = doc();
  for (const x of docs.values()) x.el?.root.classList.toggle('active', x === d);
  $('#welcome').hidden = !!d;
  if (!d) { renderWelcome(); return; }
  if (d.kind === 'settings') renderSettings(d);
  else if (d.kind === 'shortcuts') renderShortcuts(d);
  else {
    if (d.mode !== 'read') updateHighlight(d);
    if (d.mode !== 'source') renderPreview(d);
    renderBanner(d);
  }
}

function renderBanner(d) {
  const b = d.el.banner;
  if (!d.conflict && !d.missing) { b.hidden = true; return; }
  b.hidden = false;
  b.replaceChildren(h('span', {html: icon('warning') + (d.missing ? `<b>${esc(d.name)}</b> was moved or deleted on disk.` : `<b>${esc(d.name)}</b> changed on disk while you had unsaved edits.`)}));
  if (!d.missing) b.append(h('button', {class: 'btn small', onclick: () => revert(d)}, 'Load Disk Version'));
  b.append(h('button', {class: 'btn small ghost', onclick: () => { d.conflict = false; d.missing = false; renderBanner(d); renderTabs(); }}, d.missing ? 'Dismiss' : 'Keep Mine'));
}

function renderWelcome() {
  const w = ws();
  const el = $('#welcome');
  const recent = S.recent.filter(p => !docs.has(p)).slice(0, 7);
  const others = S.workspaces.filter(x => x.id !== w.id).sort((a, b) => (b.opened || 0) - (a.opened || 0)).slice(0, 4);
  const action = (ic, label, kbd, fn) => h('button', {class: 'w-action', onclick: fn, html: icon(ic) + `<span>${label}</span>` + (kbd ? `<kbd>${kbd}</kbd>` : '')});
  el.replaceChildren(h('div', {class: 'w-inner'},
    h('div', {class: 'w-hero'},
      h('div', {class: 'w-logo'}, h('i')),
      h('div', {}, h('h1', {}, 'Folio'), h('p', {}, 'Read Markdown beautifully. Click anything to edit it in place.'))),
    h('div', {class: 'w-grid'},
      h('div', {class: 'w-col'},
        h('h2', {}, 'Start'),
        action('filePlus', 'New File', '⌘N', () => command('newFile')),
        action('files', 'Open…', '⌘O', () => command('open')),
        action('folderPlus', `Add Folder to “${w.name}”`, '⌥⌘O', () => command('addFolder')),
        action('layers', 'New Workspace…', '⌃⌘N', () => command('newWorkspace')),
        action('command', 'Command Palette', '⇧⌘P', () => command('palette')),
        h('h2', {}, 'Workspaces'),
        others.length ? others.map(o => action('layers', esc(o.name), '', () => switchWorkspace(o.id))) : h('p', {class: 'muted'}, 'Workspaces you create show up here.'),
      ),
      h('div', {class: 'w-col'},
        h('h2', {}, 'Recent'),
        recent.length ? recent.map(p => h('button', {class: 'w-recent', title: tilde(p), onclick: () => openFile(p), html: fileIcon(p) + `<span class="label">${esc(basename(p))}</span><span class="desc">${esc(tilde(dirname(p)))}</span>`}))
          : h('p', {class: 'muted'}, 'Files you open will be listed here.'),
        h('h2', {}, 'Good to know'),
        h('ul', {class: 'w-tips'},
          h('li', {html: '<kbd>⌘P</kbd> jumps to any file in the workspace. Type <kbd>@</kbd> for headings, <kbd>&gt;</kbd> for commands.'}),
          h('li', {html: 'In <b>Live</b> mode, click any paragraph, heading, list or table to edit its Markdown right there. <kbd>Esc</kbd> to finish.'}),
          h('li', {html: '<kbd>⌘1</kbd> <kbd>⌘2</kbd> <kbd>⌘3</kbd> switch between Live, Split and Source.'}),
          h('li', {html: 'Check boxes in the reader to tick tasks off. The Markdown updates for you.'}),
          h('li', {html: 'Drop files or folders from Finder anywhere on the window.'}))))));
}

/* ───────────────────────── settings & shortcuts tabs ───────────────────────── */

const SETTINGS_UI = [
  ['Appearance', [
    ['theme', 'Color theme', 'select', Object.fromEntries(Object.entries(THEMES).map(([k, v]) => [k, v.label]))],
    ['readerFont', 'Reader typeface', 'select', {sans: 'San Francisco', serif: 'New York', charter: 'Charter', iowan: 'Iowan Old Style', mono: 'SF Mono'}],
    ['readerSize', 'Reader text size', 'range', [12, 24, 1, 'px']],
    ['readerWidth', 'Reading column width', 'range', [560, 1200, 20, 'px']],
    ['lineHeight', 'Reader line height', 'range', [1.4, 2.1, 0.05, '']],
  ]],
  ['Editor', [
    ['editorSize', 'Source font size', 'range', [10, 20, 1, 'px']],
    ['wrap', 'Wrap long lines', 'toggle'],
    ['lineNumbers', 'Show line numbers', 'toggle'],
    ['syncScroll', 'Sync scrolling in Split view', 'toggle'],
    ['defaultMode', 'Open Markdown files in', 'select', {read: 'Live (edit in place)', split: 'Split', source: 'Source'}],
  ]],
  ['Workbench', [
    ['previewTabs', 'Single-click opens a preview tab (italic) that the next file replaces', 'toggle'],
    ['restoreTabs', 'Restore open tabs when switching workspaces or relaunching', 'toggle'],
    ['showAllFiles', 'Show all files in the explorer, not just Markdown', 'toggle'],
    ['autoReload', 'Reload files automatically when they change on disk', 'toggle'],
  ]],
  ['Privacy', [
    ['remoteImages', 'Load remote images (https) in previews', 'toggle'],
  ]],
];

function renderSettings(d) {
  const root = d.el.root;
  const q = root.querySelector('.set-search input')?.value || '';
  const wrap = h('div', {class: 'settings'});
  const search = h('div', {class: 'set-search'}, h('input', {placeholder: 'Search settings', value: q, spellcheck: 'false'}));
  wrap.append(h('h1', {}, 'Settings'), search);
  const ql = q.toLowerCase();
  for (const [group, items] of SETTINGS_UI) {
    const sec = h('section', {}, h('h2', {}, group));
    let any = false;
    for (const [key, label, type, opt] of items) {
      if (ql && !label.toLowerCase().includes(ql) && !group.toLowerCase().includes(ql)) continue;
      any = true;
      const v = S.settings[key];
      let control;
      if (type === 'toggle') control = h('button', {class: 'switch' + (v ? ' on' : ''), role: 'switch', 'aria-checked': String(!!v), onclick: () => setSetting(key, !S.settings[key])}, h('i'));
      else if (type === 'select') { control = h('select', {onchange: e => setSetting(key, e.target.value)}); for (const [k, l] of Object.entries(opt)) control.append(h('option', {value: k, selected: k === v}, l)); }
      else if (type === 'range') {
        const [min, max, step, unit] = opt;
        const out = h('output', {}, v + unit);
        control = h('div', {class: 'range'}, h('input', {type: 'range', min, max, step, value: v, oninput: e => { out.textContent = e.target.value + unit; setSetting(key, +e.target.value, true); }}), out);
      }
      sec.append(h('div', {class: 'set-row'}, h('label', {}, label, h('small', {}, key)), control));
    }
    if (any) wrap.append(sec);
  }
  wrap.append(h('p', {class: 'muted small'}, 'Settings are stored on this Mac in ~/Library/Application Support/Folio.'));
  root.replaceChildren(h('div', {class: 'special-scroll'}, wrap));
  const input = $('.set-search input', root);
  input.addEventListener('input', debounce(() => { renderSettings(d); const i = $('.set-search input', root); i.focus(); i.setSelectionRange(i.value.length, i.value.length); }, 120));
}

function setSetting(key, value, light = false) {
  S.settings[key] = value;
  applySettings();
  if (key === 'showAllFiles') { refreshTrees(); }
  if (key === 'remoteImages' || key === 'readerFont') for (const d of docs.values()) d.rendered = null;
  if (!light) renderAll(); else { for (const d of docs.values()) if (d.kind === 'md' && d.blocks) d.blocks = d.blocks; }
  persist();
}

function renderShortcuts(d) {
  const groups = {};
  for (const c of COMMANDS) if (c.key) (groups[c.group || 'General'] ||= []).push(c);
  const wrap = h('div', {class: 'settings shortcuts'}, h('h1', {}, 'Keyboard Shortcuts'));
  for (const [g, list] of Object.entries(groups)) {
    const sec = h('section', {}, h('h2', {}, g));
    for (const c of list) sec.append(h('div', {class: 'set-row'}, h('label', {}, c.title), h('span', {class: 'keys', html: c.key.split(' ').map(k => `<kbd>${esc(k)}</kbd>`).join('')})));
    wrap.append(sec);
  }
  d.el.root.replaceChildren(h('div', {class: 'special-scroll'}, wrap));
}

/* ───────────────────────── find widget ───────────────────────── */

const find = {open: false, opts: {case: false, word: false, regex: false}, matches: [], index: -1, target: 'preview'};
const findRefreshSoon = debounce(() => findRefresh(true), 150);

function findRegex() {
  const q = $('#find-input').value;
  if (!q) return null;
  let src = find.opts.regex ? q : q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  if (find.opts.word) src = `\\b${src}\\b`;
  try { return new RegExp(src, find.opts.case ? 'g' : 'gi'); } catch { return undefined; }
}

function clearFindMarks() {
  for (const d of docs.values()) {
    if (!d.el?.article) continue;
    d.el.findlayer.innerHTML = '';
    for (const m of $$('mark.find-hit', d.el.article)) { const p = m.parentNode; m.replaceWith(document.createTextNode(m.textContent)); p.normalize(); }
  }
}

function findRefresh(keep = false) {
  clearFindMarks();
  const d = doc(), count = $('#find-count');
  find.matches = [];
  if (!find.open || !d || d.kind !== 'md') { count.textContent = ''; return; }
  const re = findRegex();
  if (re === undefined) { count.textContent = 'Invalid regex'; count.classList.add('bad'); return; }
  count.classList.remove('bad');
  if (!re) { count.textContent = 'No results'; return; }
  find.target = d.mode === 'read' || (d.mode === 'split' && d.lastFocus === 'preview') ? 'preview' : 'source';
  if (find.target === 'source') {
    let m; re.lastIndex = 0;
    while ((m = re.exec(d.text)) && find.matches.length < 5000) { if (!m[0].length) { re.lastIndex++; continue; } find.matches.push({start: m.index, end: m.index + m[0].length}); }
    if (!d.large) {
      let html = '', last = 0;
      find.matches.forEach((x, i) => { html += esc(d.text.slice(last, x.start)) + `<mark data-i="${i}">${esc(d.text.slice(x.start, x.end))}</mark>`; last = x.end; });
      html += esc(d.text.slice(last));
      d.el.findlayer.innerHTML = html.split('\n').map(l => `<div class="ln">${l || '<br>'}</div>`).join('');
    }
  } else {
    renderPreview(d);
    const walker = document.createTreeWalker(d.el.article, NodeFilter.SHOW_TEXT, {acceptNode: n => n.parentElement.closest('.code-head,.anchor') ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT});
    const nodes = []; while (walker.nextNode()) nodes.push(walker.currentNode);
    for (const node of nodes) {
      const t = node.textContent; re.lastIndex = 0;
      const parts = []; let m, last = 0;
      while ((m = re.exec(t))) { if (!m[0].length) { re.lastIndex++; continue; } parts.push([m.index, m.index + m[0].length]); }
      if (!parts.length) continue;
      const frag = document.createDocumentFragment();
      for (const [s, e] of parts) { frag.append(t.slice(last, s)); const mk = h('mark', {class: 'find-hit'}, t.slice(s, e)); frag.append(mk); find.matches.push({el: mk}); last = e; }
      frag.append(t.slice(last)); node.replaceWith(frag);
    }
  }
  if (!keep || find.index >= find.matches.length) find.index = -1;
  if (find.index < 0 && find.matches.length) {
    // Start from the first match after the current view position.
    if (find.target === 'source') { const caret = d.el.ta.selectionStart; find.index = Math.max(0, find.matches.findIndex(x => x.start >= caret)); }
    else { const y = d.el.pv.scrollTop; find.index = Math.max(0, find.matches.findIndex(x => x.el.offsetTop + d.el.article.offsetTop >= y)); }
  }
  findShow(false);
}

function findShow(move = true) {
  const d = doc(), n = find.matches.length, count = $('#find-count');
  count.textContent = n ? `${find.index + 1} of ${n >= 5000 ? '5000+' : n}` : 'No results';
  if (!n || !d) return;
  const m = find.matches[find.index];
  if (find.target === 'source') {
    $$('mark.cur', d.el.findlayer).forEach(x => x.classList.remove('cur'));
    const mk = $(`mark[data-i="${find.index}"]`, d.el.findlayer); mk?.classList.add('cur');
    if (move) {
      const line = lineOfOffset(d.text, m.start);
      const top = lineTop(d, line), sc = d.el.src;
      if (top < sc.scrollTop || top > sc.scrollTop + sc.clientHeight - 40) sc.scrollTop = top - sc.clientHeight / 3;
      d.el.ta.setSelectionRange(m.start, m.end);
    }
  } else {
    $$('mark.find-hit.cur', d.el.article).forEach(x => x.classList.remove('cur'));
    m.el.classList.add('cur');
    if (move) { const top = m.el.getBoundingClientRect().top - d.el.pv.getBoundingClientRect().top; if (top < 40 || top > d.el.pv.clientHeight - 60) d.el.pv.scrollTop += top - d.el.pv.clientHeight / 3; }
  }
}

function findStep(dir) {
  if (!find.open) { openFind(); return; }
  if (!find.matches.length) return;
  find.index = (find.index + dir + find.matches.length) % find.matches.length;
  findShow(true);
}

function openFind() {
  const d = doc(); if (!d || d.kind !== 'md') return;
  if (document.activeElement === d.el.ta) d.lastFocus = 'source';
  const sel = d.el.ta.value.slice(d.el.ta.selectionStart, d.el.ta.selectionEnd);
  const winSel = String(window.getSelection() || '');
  find.open = true; $('#findwidget').hidden = false;
  const input = $('#find-input');
  const seed = document.activeElement === d.el.ta ? sel : winSel;
  if (seed && !seed.includes('\n') && seed.length < 200) input.value = seed;
  input.focus(); input.select();
  find.index = -1; findRefresh();
}

function closeFind() {
  const d = doc();
  find.open = false; $('#findwidget').hidden = true; clearFindMarks(); find.matches = [];
  if (d?.kind === 'md' && find.target === 'source') d.el.ta.focus();
}

function findEvents() {
  const input = $('#find-input');
  input.addEventListener('input', () => { find.index = -1; findRefresh(); findShow(true); });
  input.addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); findStep(e.shiftKey ? -1 : 1); }
    if (e.key === 'Escape') { e.preventDefault(); closeFind(); }
  });
  $('#find-prev').innerHTML = icon('up'); $('#find-next').innerHTML = icon('down'); $('#find-close').innerHTML = icon('x');
  $('#find-prev').onclick = () => findStep(-1); $('#find-next').onclick = () => findStep(1); $('#find-close').onclick = closeFind;
  $$('#findwidget .toggles button').forEach(b => b.addEventListener('click', () => { find.opts[b.dataset.opt] = !find.opts[b.dataset.opt]; b.classList.toggle('on'); findRefresh(); findShow(true); }));
}

/* ───────────────────────── palette / quick input ───────────────────────── */

const pal = {open: false, items: [], sel: 0, resolve: null, custom: null, inputOnly: false};

function fuzzy(q, s) {
  if (!q) return {score: 0, idx: []};
  const ql = q.toLowerCase(), sl = s.toLowerCase();
  let score = 0, j = 0, prev = -2; const idx = [];
  for (let i = 0; i < sl.length && j < ql.length; i++) {
    if (sl[i] === ql[j]) {
      let bonus = 1;
      if (i === prev + 1) bonus += 4;
      if (i === 0 || /[\s/_.-]/.test(s[i - 1]) || (s[i] !== sl[i] && s[i - 1] === s[i - 1].toLowerCase())) bonus += 6;
      score += bonus; idx.push(i); prev = i; j++;
    }
  }
  if (j < ql.length) return null;
  const sub = sl.indexOf(ql);
  if (sub !== -1) score += 20 + (sub === 0 ? 15 : 0);
  return {score: score - s.length * 0.05, idx};
}
const markIdx = (s, idx) => { let out = '', set = new Set(idx); for (let i = 0; i < s.length; i++) out += set.has(i) ? `<b>${esc(s[i])}</b>` : esc(s[i]); return out; };

function openPalette(prefix = '') {
  pal.custom = null; pal.inputOnly = false;
  pal.resolve?.(null); pal.resolve = null;
  showPalette(prefix, prefix === '>' ? 'Type a command' : 'Search files by name (type > for commands, @ for headings, : for a line)');
}

function showPalette(value, placeholder) {
  pal.open = true;
  $('#palette').hidden = false;
  const input = $('#pal-input');
  input.value = value; input.placeholder = placeholder;
  input.focus();
  input.setSelectionRange(value.length, value.length);
  updatePalette();
}

function closePalette(result = null) {
  if (!pal.open) return;
  pal.open = false; $('#palette').hidden = true;
  const r = pal.resolve; pal.resolve = null; pal.custom = null;
  r?.(result);
  const d = doc(); if (d?.kind === 'md' && d.mode !== 'read' && result == null) d.el.ta.focus({preventScroll: true});
}

function quickPick(items, placeholder) {
  return new Promise(resolve => { pal.resolve?.(null); pal.custom = items; pal.inputOnly = false; pal.resolve = resolve; showPalette('', placeholder); });
}
function quickInput({placeholder, value = '', prompt = ''}) {
  return new Promise(resolve => { pal.resolve?.(null); pal.custom = null; pal.inputOnly = prompt || ' '; pal.resolve = resolve; showPalette(value, placeholder); $('#pal-input').select(); });
}

let palSeq = 0;
async function updatePalette() {
  const q = $('#pal-input').value;
  const seq = ++palSeq;
  let items = [];
  if (pal.inputOnly) items = [{label: pal.inputOnly.trim() || 'Press Enter to confirm or Escape to cancel', icon: 'info', passive: true}];
  else if (pal.custom) items = rank(pal.custom, q, x => x.label);
  else if (q.startsWith('>')) {
    items = rank(COMMANDS.filter(c => !c.hidden && (!c.when || c.when())).map(c => ({label: c.title, detail: c.group, kbd: c.key, icon: c.icon || 'command', run: () => command(c.id)})), q.slice(1).trim(), x => x.label);
  } else if (q.startsWith('@')) {
    const d = doc();
    const hs = d?.kind === 'md' ? analyze(d).headings : [];
    items = rank(hs.map(x => ({label: x.text || '(untitled)', detail: `H${x.depth} · line ${x.line + 1}`, icon: 'hash', indent: x.depth - 1, run: () => revealHeading(d, x)})), q.slice(1).trim(), x => x.label, true);
    if (!hs.length) items = [{label: d ? 'No headings in this document' : 'Open a document first', icon: 'info', passive: true}];
  } else if (q.startsWith(':')) {
    const d = doc(), n = parseInt(q.slice(1), 10);
    items = d?.kind === 'md' ? [{label: n ? `Go to line ${n}` : `Type a line number between 1 and ${d.text.split('\n').length}`, icon: 'hash', passive: !n, run: () => { if (d.mode === 'read') setMode(d, 'split'); revealLine(d, Math.max(0, n - 1)); }}] : [{label: 'Open a document first', icon: 'info', passive: true}];
  } else {
    const files = await getFiles().catch(() => []);
    if (seq !== palSeq) return;
    const seen = new Set();
    const pool = [];
    for (const k of tabs) { const d = docs.get(k); if (d.path) { seen.add(d.path); pool.push({path: d.path, open: true}); } }
    for (const p of S.recent) if (!seen.has(p)) { seen.add(p); pool.push({path: p, recent: true}); }
    for (const p of files) if (!seen.has(p)) { seen.add(p); pool.push({path: p}); }
    const qt = q.trim();
    if (!qt) items = pool.slice(0, 60).map(x => ({label: basename(x.path), detail: relPath(dirname(x.path)), icon: 'md', tag: x.open ? 'open' : x.recent ? 'recent' : '', run: () => openFile(x.path)}));
    else {
      const scored = [];
      for (const x of pool) {
        const rel = relPath(x.path), name = basename(x.path);
        const a = fuzzy(qt, name), b = a ? null : fuzzy(qt, rel);
        if (!a && !b) continue;
        scored.push({x, score: a ? a.score + 30 + (x.open ? 5 : 0) : b.score, idx: a?.idx, name, rel});
      }
      scored.sort((p, q2) => q2.score - p.score);
      items = scored.slice(0, 60).map(s => ({labelHtml: s.idx ? markIdx(s.name, s.idx) : esc(s.name), label: s.name, detail: relPath(dirname(s.x.path)), icon: 'md', tag: s.x.open ? 'open' : '', run: () => openFile(s.x.path)}));
    }
    if (!items.length) items = [{label: ws().roots.length ? 'No matching files' : 'Add a folder to this workspace to search its files', icon: 'info', passive: true}];
  }
  pal.items = items; pal.sel = items.findIndex(x => !x.passive);
  drawPalette();
}

function rank(list, q, key, keepOrder = false) {
  if (!q) return list;
  const out = [];
  for (const x of list) { const f = fuzzy(q, key(x)); if (f) out.push({...x, _s: f.score, labelHtml: markIdx(key(x), f.idx)}); }
  if (!keepOrder) out.sort((a, b) => b._s - a._s);
  return out;
}

function drawPalette() {
  const list = $('#pal-list');
  list.replaceChildren();
  pal.items.forEach((x, i) => {
    const row = h('div', {class: 'pal-item' + (i === pal.sel ? ' sel' : '') + (x.passive ? ' passive' : ''), role: 'option', style: {'--indent': x.indent || 0},
      html: icon(x.icon || 'file', x.icon === 'md' ? 'fi-md' : '') + `<span class="label">${x.labelHtml || esc(x.label)}</span>` + (x.detail ? `<span class="desc">${esc(x.detail)}</span>` : '')
        + (x.tag ? `<span class="tag">${x.tag}</span>` : '') + (x.kbd ? `<span class="keys">${x.kbd.split(' ').map(k => `<kbd>${esc(k)}</kbd>`).join('')}</span>` : '')});
    row.addEventListener('mousemove', () => { if (pal.sel !== i && !x.passive) { pal.sel = i; $$('.pal-item', list).forEach((r, j) => r.classList.toggle('sel', j === i)); } });
    row.addEventListener('click', () => choosePalette(i));
    list.append(row);
  });
}

function choosePalette(i) {
  if (pal.inputOnly) { closePalette($('#pal-input').value); return; }
  const x = pal.items[i];
  if (!x || x.passive) return;
  if (pal.custom) { closePalette(x); return; }
  closePalette(true);
  x.run?.();
}

function paletteEvents() {
  const input = $('#pal-input');
  input.addEventListener('input', updatePalette);
  input.addEventListener('keydown', e => {
    const n = pal.items.length;
    if (e.key === 'Escape') { e.preventDefault(); closePalette(null); }
    else if (e.key === 'Enter') { e.preventDefault(); choosePalette(pal.sel); }
    else if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || (e.ctrlKey && (e.key === 'n' || e.key === 'p'))) {
      e.preventDefault(); if (!n) return;
      const dir = e.key === 'ArrowDown' || e.key === 'n' ? 1 : -1;
      let i = pal.sel; for (let k = 0; k < n; k++) { i = (i + dir + n) % n; if (!pal.items[i].passive) break; }
      pal.sel = i; drawPalette(); $('.pal-item.sel')?.scrollIntoView({block: 'nearest'});
    }
  });
  $('#palette').addEventListener('mousedown', e => { if (e.target.id === 'palette') { e.preventDefault(); closePalette(null); } });
}

/* ───────────────────────── context menu ───────────────────────── */

function showMenu(x, y, items) {
  const m = $('#menu');
  m.replaceChildren();
  for (const it of items) {
    if (!it) { if (m.lastChild && !m.lastChild.classList.contains('sep')) m.append(h('div', {class: 'sep'})); continue; }
    const [label, fn, kbd, danger] = it;
    m.append(h('button', {class: 'mi' + (danger ? ' danger' : ''), onclick: () => { hideMenu(); fn(); }}, h('span', {}, label), kbd ? h('kbd', {}, kbd) : null));
  }
  if (m.lastChild?.classList.contains('sep')) m.lastChild.remove();
  m.hidden = false;
  const r = m.getBoundingClientRect();
  m.style.left = Math.max(6, Math.min(x, innerWidth - r.width - 6)) + 'px';
  m.style.top = Math.max(6, Math.min(y, innerHeight - r.height - 6)) + 'px';
}
const hideMenu = () => { $('#menu').hidden = true; };

/* ───────────────────────── views, activity bar, title bar ───────────────────────── */

const VIEWS = [['explorer', 'files', 'Explorer', '⇧⌘E'], ['search', 'search', 'Search', '⇧⌘F'], ['outline', 'outline', 'Outline', '⇧⌘U'], ['workspaces', 'layers', 'Workspaces', '⇧⌘K']];

function showView(v, toggle = false) {
  if (toggle && S.settings.view === v && S.settings.sidebarVisible) { S.settings.sidebarVisible = false; }
  else { S.settings.view = v; S.settings.sidebarVisible = true; }
  if (S.settings.zen) S.settings.zen = false;
  applyLayout(); renderActivity(); renderSidebar(); renderTitlebar(); persist();
  if (v === 'search' && S.settings.sidebarVisible) { const i = $('#search-input'); i.focus(); i.select(); }
}

function renderActivity() {
  const top = $('#activitybar .ab-top'), bottom = $('#activitybar .ab-bottom');
  top.replaceChildren(); bottom.replaceChildren();
  for (const [v, ic, label, key] of VIEWS) {
    const on = S.settings.view === v && S.settings.sidebarVisible;
    const b = h('button', {class: 'ab-btn' + (on ? ' on' : ''), title: `${label} (${key})`, html: icon(ic), onclick: () => showView(v, true)});
    if (v === 'explorer' && dirtyDocs().length) b.append(h('span', {class: 'badge'}, String(dirtyDocs().length)));
    top.append(b);
  }
  bottom.append(
    h('button', {class: 'ab-btn', title: 'Keyboard Shortcuts', html: icon('keyboard'), onclick: () => command('shortcuts')}),
    h('button', {class: 'ab-btn' + (active === 'folio:settings' ? ' on' : ''), title: 'Settings (⌘,)', html: icon('settings'), onclick: () => command('settings')}));
}

function renderSidebar() {
  $$('#sidebar .view').forEach(v => v.classList.toggle('on', v.dataset.view === S.settings.view));
  if (S.settings.view === 'explorer') renderExplorer();
  if (S.settings.view === 'outline') renderOutline();
  if (S.settings.view === 'workspaces') renderWorkspaces();
}

function renderTitlebar() {
  $('#tb-sidebar').innerHTML = icon(S.settings.sidebarVisible ? 'sidebarFill' : 'sidebar');
  const d = doc();
  $('#tb-mode').innerHTML = icon(d?.mode === 'read' ? 'code' : 'book');
  $('#tb-mode').title = d?.mode === 'read' ? 'Show Source (⇧⌘V)' : 'Back to Live (⇧⌘V)';
  $('#tb-mode').disabled = !(d?.kind === 'md');
  $('#tb-theme').innerHTML = icon(THEMES[S.settings.theme].dark ? 'moon' : 'sun');
  reportNoDrag();
}

const reportNoDrag = () => {};

function renderAll() {
  renderActivity(); renderSidebar(); renderTabs(); renderEditor(); renderBreadcrumbs(); renderStatus(); renderTitlebar(); updateTitle();
}

function applySettings() {
  const s = S.settings, root = document.documentElement;
  if (!THEMES[s.theme]) s.theme = 'dark';
  root.dataset.theme = s.theme;
  root.dataset.readerFont = s.readerFont;
  root.style.setProperty('--reader-size', s.readerSize + 'px');
  root.style.setProperty('--reader-width', s.readerWidth + 'px');
  root.style.setProperty('--reader-lh', s.lineHeight);
  root.style.setProperty('--editor-size', s.editorSize + 'px');
  root.classList.toggle('nowrap', !s.wrap);
  root.classList.toggle('no-ln', !s.lineNumbers);
  applyLayout();
  const t = THEMES[s.theme];
  api('window', {dark: t.dark, bg: t.bg});
}

/* ───────────────────────── commands ───────────────────────── */

const mdDoc = () => { const d = doc(); return d?.kind === 'md' ? d : null; };
const COMMANDS = [
  {id: 'newFile', title: 'New File', key: '⌘N', group: 'File', icon: 'filePlus', run: () => newUntitled()},
  {id: 'newFileInFolder', title: 'New File in Workspace…', group: 'File', icon: 'filePlus', run: () => newEntry(selectedDir(), false), when: () => ws().roots.length},
  {id: 'open', title: 'Open…', key: '⌘O', group: 'File', icon: 'files', run: openDialog},
  {id: 'addFolder', title: 'Add Folder to Workspace…', key: '⌥⌘O', group: 'Workspace', icon: 'folderPlus', run: addFolderDialog},
  {id: 'save', title: 'Save', key: '⌘S', group: 'File', run: () => saveDoc(mdDoc())},
  {id: 'saveAs', title: 'Save As…', key: '⇧⌘S', group: 'File', run: () => saveDoc(mdDoc(), true)},
  {id: 'saveAll', title: 'Save All', key: '⌥⌘S', group: 'File', run: saveAll},
  {id: 'revert', title: 'Revert File', group: 'File', run: () => revert(mdDoc()), when: () => mdDoc()?.path},
  {id: 'reveal', title: 'Reveal in Finder', key: '⌥⌘R', group: 'File', icon: 'finder', run: () => { const d = mdDoc(); if (d?.path) api('reveal', {path: d.path}); }, when: () => mdDoc()?.path},
  {id: 'copyPath', title: 'Copy Path of Active File', group: 'File', icon: 'copy', run: () => { const d = mdDoc(); if (d?.path) { api('copy', {text: d.path}); toast('Path copied', 'ok'); } }, when: () => mdDoc()?.path},
  {id: 'exportHtml', title: 'Export as HTML…', group: 'File', icon: 'export', run: exportHtml, when: () => mdDoc()},
  {id: 'copyHtml', title: 'Copy Rendered HTML', group: 'File', icon: 'copy', run: () => { const d = mdDoc(); if (!d) return; renderPreview(d); api('copy', {text: cleanHtml(d)}); toast('HTML copied', 'ok'); }, when: () => mdDoc()},
  {id: 'closeTab', title: 'Close Tab', key: '⌘W', group: 'Tabs', run: () => active && closeTab(active)},
  {id: 'closeAll', title: 'Close All Tabs', key: '⌥⌘W', group: 'Tabs', run: () => closeMany([...tabs])},
  {id: 'closeOthers', title: 'Close Other Tabs', group: 'Tabs', run: () => closeMany(tabs.filter(k => k !== active))},
  {id: 'closeSaved', title: 'Close Saved Tabs', group: 'Tabs', run: () => closeMany(tabs.filter(k => !isDirty(docs.get(k))))},
  {id: 'reopenTab', title: 'Reopen Closed Tab', key: '⇧⌘T', group: 'Tabs', run: () => { const c = closedStack.pop(); if (c) openFile(c.path, {mode: c.mode}); }},
  {id: 'nextTab', title: 'Next Tab', key: '⇧⌘]', group: 'Tabs', run: () => cycleTab(1)},
  {id: 'prevTab', title: 'Previous Tab', key: '⇧⌘[', group: 'Tabs', run: () => cycleTab(-1)},
  {id: 'quickOpen', title: 'Go to File…', key: '⌘P', group: 'Navigate', icon: 'search', run: () => openPalette('')},
  {id: 'palette', title: 'Show All Commands', key: '⇧⌘P', group: 'Navigate', icon: 'command', run: () => openPalette('>')},
  {id: 'gotoHeading', title: 'Go to Heading…', key: '⇧⌘O', group: 'Navigate', icon: 'hash', run: () => openPalette('@')},
  {id: 'gotoLine', title: 'Go to Line…', key: '⌃G', group: 'Navigate', icon: 'hash', run: () => openPalette(':')},
  {id: 'find', title: 'Find', key: '⌘F', group: 'Edit', icon: 'search', run: openFind},
  {id: 'findNext', title: 'Find Next', key: '⌘G', group: 'Edit', run: () => findStep(1), hidden: true},
  {id: 'findPrev', title: 'Find Previous', key: '⇧⌘G', group: 'Edit', run: () => findStep(-1), hidden: true},
  {id: 'searchView', title: 'Search in Workspace', key: '⇧⌘F', group: 'View', icon: 'search', run: () => showView('search')},
  {id: 'bold', title: 'Toggle Bold', key: '⌘B', group: 'Edit', run: () => wrapSelection(mdDoc(), '**')},
  {id: 'italic', title: 'Toggle Italic', key: '⌘I', group: 'Edit', run: () => wrapSelection(mdDoc(), '_')},
  {id: 'link', title: 'Insert Link', key: '⌘K', group: 'Edit', run: () => insertLink(mdDoc())},
  {id: 'explorerView', title: 'Show Explorer', key: '⇧⌘E', group: 'View', icon: 'files', run: () => showView('explorer')},
  {id: 'outlineView', title: 'Show Outline', key: '⇧⌘U', group: 'View', icon: 'outline', run: () => showView('outline')},
  {id: 'workspacesView', title: 'Show Workspaces', key: '⇧⌘K', group: 'View', icon: 'layers', run: () => showView('workspaces')},
  {id: 'toggleSidebar', title: 'Toggle Sidebar', key: '⌘\\', group: 'View', icon: 'sidebar', run: () => { S.settings.sidebarVisible = !S.settings.sidebarVisible; S.settings.zen = false; applyLayout(); renderAll(); persist(); }},
  {id: 'read', title: 'View: Live (edit in place)', key: '⌘1', group: 'View', icon: 'book', run: () => setMode(mdDoc(), 'read')},
  {id: 'split', title: 'View: Split Editor & Preview', key: '⌘2', group: 'View', icon: 'columns', run: () => setMode(mdDoc(), 'split')},
  {id: 'source', title: 'View: Source', key: '⌘3', group: 'View', icon: 'code', run: () => setMode(mdDoc(), 'source')},
  {id: 'togglePreview', title: 'Toggle Live / Source', key: '⇧⌘V', group: 'View', run: () => { const d = mdDoc(); if (d) setMode(d, d.mode === 'read' ? 'split' : 'read'); }},
  {id: 'zoomIn', title: 'Increase Text Size', key: '⌘=', group: 'View', run: () => zoom(1)},
  {id: 'zoomOut', title: 'Decrease Text Size', key: '⌘-', group: 'View', run: () => zoom(-1)},
  {id: 'zoomReset', title: 'Reset Text Size', key: '⌘0', group: 'View', run: () => zoom(0)},
  {id: 'wrap', title: 'Toggle Word Wrap', key: '⌥Z', group: 'View', icon: 'wrap', run: () => setSetting('wrap', !S.settings.wrap)},
  {id: 'lineNumbers', title: 'Toggle Line Numbers', group: 'View', run: () => setSetting('lineNumbers', !S.settings.lineNumbers)},
  {id: 'toggleSync', title: 'Toggle Sync Scrolling', group: 'View', run: () => setSetting('syncScroll', !S.settings.syncScroll)},
  {id: 'zen', title: 'Toggle Zen Mode', key: '⌃⌘J', group: 'View', icon: 'zen', run: () => { S.settings.zen = !S.settings.zen; applyLayout(); persist(); reportNoDrag(); }},
  {id: 'theme', title: 'Toggle Light / Dark Theme', key: '⌥⌘T', group: 'Appearance', icon: 'sun', run: () => setSetting('theme', THEMES[S.settings.theme].dark ? (S.settings.theme === 'midnight' ? 'paper' : 'light') : (S.settings.theme === 'paper' ? 'midnight' : 'dark'))},
  {id: 'pickTheme', title: 'Color Theme…', group: 'Appearance', icon: 'sun', run: async () => {
    const before = S.settings.theme;
    const pick = await quickPick(Object.entries(THEMES).map(([k, v]) => ({label: v.label, detail: k === before ? 'current' : '', icon: v.dark ? 'moon' : 'sun', value: k})), 'Select color theme');
    if (pick) setSetting('theme', pick.value);
  }},
  {id: 'settings', title: 'Open Settings', key: '⌘,', group: 'Preferences', icon: 'settings', run: () => openSpecial('folio:settings')},
  {id: 'shortcuts', title: 'Keyboard Shortcuts', key: '⌘/', group: 'Preferences', icon: 'keyboard', run: () => openSpecial('folio:shortcuts')},
  {id: 'welcome', title: 'Show Welcome', group: 'Help', icon: 'info', run: () => { active = null; renderAll(); }},
  {id: 'newWorkspace', title: 'New Workspace…', key: '⌃⌘N', group: 'Workspace', icon: 'layers', run: createWorkspace},
  {id: 'switchWorkspace', title: 'Switch Workspace…', key: '⌃⌘R', group: 'Workspace', icon: 'layers', run: async () => {
    const cur = ws();
    const items = S.workspaces.slice().sort((a, b) => (b.opened || 0) - (a.opened || 0)).map(w => ({label: w.name, detail: w.id === cur.id ? 'current' : `${plural(w.roots.length, 'folder')} · ${timeAgo(w.opened)}`, icon: 'layers', value: w.id}));
    items.push({label: 'New Workspace…', icon: 'plus', value: '__new'});
    const pick = await quickPick(items, 'Switch to workspace');
    if (pick?.value === '__new') createWorkspace(); else if (pick) switchWorkspace(pick.value);
  }},
  {id: 'renameWorkspace', title: 'Rename Workspace…', group: 'Workspace', icon: 'edit', run: () => renameWorkspace()},
  {id: 'refreshExplorer', title: 'Refresh Explorer', group: 'Workspace', icon: 'refresh', run: () => { fileCache = null; refreshTrees(); }},
  {id: 'collapseFolders', title: 'Collapse Folders in Explorer', group: 'Workspace', icon: 'collapse', run: () => { expanded = new Set(); renderExplorer(); }},
  {id: 'showAllFiles', title: 'Toggle Showing All Files', group: 'Workspace', run: () => setSetting('showAllFiles', !S.settings.showAllFiles)},
];
for (let n = 1; n <= 9; n++) COMMANDS.push({id: 'tab' + n, title: `Go to Tab ${n}`, hidden: true, run: () => { const k = n === 9 ? tabs[tabs.length - 1] : tabs[n - 1]; if (k) activate(k); }});

function command(id) {
  hideMenu();
  const c = COMMANDS.find(x => x.id === id);
  if (c) c.run();
}

function cycleTab(dir) { if (!tabs.length) return; const i = tabs.indexOf(active); activate(tabs[(i + dir + tabs.length) % tabs.length]); }

function zoom(dir) {
  const s = S.settings;
  if (dir === 0) { s.readerSize = DEFAULTS.readerSize; s.editorSize = DEFAULTS.editorSize; }
  else { s.readerSize = Math.min(28, Math.max(11, s.readerSize + dir)); s.editorSize = Math.min(22, Math.max(9, s.editorSize + dir)); }
  applySettings(); persist();
  toast(`Text size ${Math.round(s.readerSize / DEFAULTS.readerSize * 100)}%`);
}

function cleanHtml(d) {
  const clone = d.el.article.cloneNode(true);
  $$('.anchor, .code-head', clone).forEach(x => x.remove());
  $$('mark.find-hit', clone).forEach(m => m.replaceWith(m.textContent));
  $$('[data-line]', clone).forEach(x => x.removeAttribute('data-line'));
  return clone.innerHTML;
}

async function exportHtml() {
  const d = mdDoc(); if (!d) return;
  renderPreview(d);
  const name = stripExt(d.name) + '.html';
  const path = await api('saveDialog', {name, dir: d.path ? dirname(d.path) : ''});
  if (!path) return;
  const styles = [...document.styleSheets].flatMap(s => { try { return [...s.cssRules].map(r => r.cssText); } catch { return []; } }).join('\n');
  const html = `<!doctype html><html data-theme="${THEMES[S.settings.theme].dark ? 'dark' : 'light'}" data-reader-font="${S.settings.readerFont}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${esc(stripExt(d.name))}</title><style>${styles}\nbody{overflow:auto;height:auto;background:var(--bg-0)}.export{padding:48px 24px}</style></head><body><div class="export"><article class="markdown-body">${cleanHtml(d)}</article></div></body></html>`;
  try { await api('write', {path: path.endsWith('.html') ? path : path + '.html', text: html}); toast(`Exported ${basename(path)}`, 'ok'); } catch (e) { toast(String(e.message || e), 'error'); }
}

/* ───────────────────────── recents ───────────────────────── */

function addRecent(p) { S.recent = [p, ...S.recent.filter(x => x !== p)].slice(0, 30); }

/* ───────────────────────── external changes ───────────────────────── */

const onFsChange = debounce(async () => { fileCache = null; await refreshTrees(); await checkExternal(); }, 350);
let checking = false;
async function checkExternal() {
  if (checking) return; checking = true;
  try {
    const open = [...docs.values()].filter(d => d.path);
    if (!open.length) return;
    const st = await api('stat', {paths: open.map(d => d.path)});
    for (const d of open) {
      const s = st[d.path];
      if (!s) { if (!d.missing) { d.missing = true; renderBanner(d); } continue; }
      if (d.missing) { d.missing = false; renderBanner(d); }
      if (d.mtime && Math.abs(s.m - d.mtime) > 0.0005) {
        if (!isDirty(d) && S.settings.autoReload) {
          const r = await api('read', {path: d.path});
          if (r.text !== d.text) { d.mtime = r.mtime; setText(d, r.text, true); if (d === doc()) toast(`Reloaded ${d.name} — it changed on disk`); }
          else d.mtime = r.mtime;
        } else if (!d.conflict) { d.conflict = true; d.mtime = s.m; renderBanner(d); renderTabs(); renderStatus(); }
      }
    }
  } catch {} finally { checking = false; }
}

/* ───────────────────────── global keys, sash, misc ───────────────────────── */

function globalEvents() {
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape') {
      if (!$('#menu').hidden) { hideMenu(); return; }
      if (pal.open) return;
      if (find.open) { closeFind(); return; }
      if (S.settings.zen) { command('zen'); return; }
    }
    if (e.ctrlKey && e.key === 'Tab') { e.preventDefault(); cycleTab(e.shiftKey ? -1 : 1); return; }
    if (e.ctrlKey && !e.metaKey && e.key === 'g') { e.preventDefault(); command('gotoLine'); return; }
    if (e.metaKey && e.key === 'g') { e.preventDefault(); findStep(e.shiftKey ? -1 : 1); return; }
    if (!native) {
      // Browser fallback for menu shortcuts (the native menu handles these in the app).
      const k = e.key.toLowerCase();
      const map = {p: e.shiftKey ? 'palette' : 'quickOpen', b: 'toggleSidebar', s: e.shiftKey ? 'saveAs' : 'save', f: e.shiftKey ? 'searchView' : 'find', '1': 'read', '2': 'split', '3': 'source', ',': 'settings'};
      if (e.metaKey && map[k]) { e.preventDefault(); command(map[k]); }
    }
  });
  document.addEventListener('mousedown', e => { if (!$('#menu').hidden && !e.target.closest('#menu')) hideMenu(); });
  window.addEventListener('blur', hideMenu);
  window.addEventListener('resize', () => { hideMenu(); reportNoDrag(); });
  // Sash
  const sash = $('#sash');
  sash.addEventListener('pointerdown', e => {
    sash.setPointerCapture(e.pointerId); document.body.classList.add('resizing-x');
    const move = ev => { S.settings.sidebarWidth = Math.round(Math.min(560, Math.max(180, ev.clientX - $('#activitybar').offsetWidth))); applyLayout(); };
    sash.addEventListener('pointermove', move);
    sash.addEventListener('pointerup', () => { sash.removeEventListener('pointermove', move); document.body.classList.remove('resizing-x'); persist(); reportNoDrag(); }, {once: true});
  });
  sash.addEventListener('dblclick', () => { S.settings.sidebarWidth = DEFAULTS.sidebarWidth; applyLayout(); persist(); });
  // Title bar
  $('#command-center').addEventListener('click', () => openPalette(''));
  $('#command-center .cc-icon').innerHTML = icon('search');
  $('#tb-sidebar').addEventListener('click', () => command('toggleSidebar'));
  $('#tb-mode').addEventListener('click', () => command('togglePreview'));
  $('#tb-theme').addEventListener('click', () => command('pickTheme'));
  // Search view
  $('#search-input').addEventListener('input', runSearchSoon);
  $('#search-input').addEventListener('keydown', e => { if (e.key === 'Enter') runSearch(); if (e.key === 'Escape') { e.target.value = ''; runSearch(); } });
  $$('.search-box .toggles button', $('[data-view=search]')).forEach(b => b.addEventListener('click', () => { searchOpts[b.dataset.opt] = !searchOpts[b.dataset.opt]; b.classList.toggle('on'); runSearch(); }));
  $('#search-actions').append(
    h('button', {class: 'icon-btn', title: 'Refresh', html: icon('refresh'), onclick: runSearch}),
    h('button', {class: 'icon-btn', title: 'Collapse All', html: icon('collapse'), onclick: () => $$('.sr-file').forEach(g => { g.classList.remove('open'); $('.twisty', g).classList.remove('open'); })}));
  $('#outline-filter').addEventListener('input', renderOutline);
  $('#outline-actions').append(h('button', {class: 'icon-btn', title: 'Go to Heading (⇧⌘O)', html: icon('hash'), onclick: () => openPalette('@')}));
  $('#explorer-actions').append(
    h('button', {class: 'icon-btn', title: 'New File (⌘N)', html: icon('filePlus'), onclick: () => ws().roots.length ? newEntry(selectedDir(), false) : command('newFile')}),
    h('button', {class: 'icon-btn', title: 'Add Folder to Workspace (⌥⌘O)', html: icon('folderPlus'), onclick: addFolderDialog}),
    h('button', {class: 'icon-btn', title: 'More', html: icon('more'), onclick: e => { const b = e.currentTarget.getBoundingClientRect(); showMenu(b.left - 180, b.bottom + 4, [
      ['New Folder…', () => newEntry(selectedDir(), true)], ['Refresh Explorer', () => command('refreshExplorer')], ['Collapse Folders', () => command('collapseFolders')], null,
      [`${S.settings.showAllFiles ? '✓ ' : ''}Show All Files`, () => command('showAllFiles')], [`${S.settings.openEditors ? '✓ ' : ''}Show Open Editors`, () => { S.settings.openEditors = !S.settings.openEditors; renderExplorer(); persist(); }],
    ]); }}));
  $('#workspaces-actions').append(h('button', {class: 'icon-btn', title: 'New Workspace', html: icon('plus'), onclick: createWorkspace}));
  // Drag & drop in the browser fallback
  document.addEventListener('dragover', e => { if (e.dataTransfer?.types?.includes('Files')) e.preventDefault(); });
  document.addEventListener('drop', e => { if (e.dataTransfer?.types?.includes('Files')) { e.preventDefault(); toast('Drop files onto the Folio window in the app to open them'); } });
  // Preview focus tracking for find target
  $('#docs').addEventListener('mousedown', e => { const d = doc(); if (d?.kind === 'md' && e.target.closest('.pane.preview')) d.lastFocus = 'preview'; });
}

/* ───────────────────────── native → web API ───────────────────────── */

window.folio = {
  command,
  openPaths,
  fsChanged: () => onFsChange(),
  focusChanged: () => checkExternal(),
  fullscreen: on => { fullscreen = on; document.body.classList.toggle('fullscreen', on); reportNoDrag(); },
  saveAll,
  async requestClose() {
    if (!(await resolveDirty(dirtyDocs(), 'quitting'))) return;
    for (const d of docs.values()) if (d.live) commitLive(d, false);
    try { await persistNow(); } catch {}
    api('quit');
  },
  async selftest() {
    const checks = {};
    const d = newUntitled('---\ntitle: Test\n---\n# Safety\n\n<script>window.pwned=true</script>\n\n[bad](javascript:alert(1))\n\n> [!NOTE]\n> Callout body\n\n| Idea | Audience | Views |\n|---|---|---|\n| Investigation Atlas | Undergrads, judges | Card grid by hardware family, filters (fuel, solid/gas/droplet, platform), detail drawer with DOI |\n\n- [ ] task one\n- [x] task two\n\n```python\ndef f():\n    return 42\n```\n\n## Second');
    setMode(d, 'split');
    renderPreview(d);
    await new Promise(r => setTimeout(r, 200));
    const a = d.el.article;
    checks.heading = !!$('h1#safety', a);
    checks.frontmatter = !!$('dl.frontmatter', a);
    checks.table = !!$('.table-wrap table', a);
    const firstCell = $('.table-wrap td', a);
    checks.tableCellWidth = firstCell ? Math.round(firstCell.getBoundingClientRect().width) : 0;
    checks.highlight = !!$('.hljs-keyword', a);
    checks.callout = !!$('.callout-note .callout-title', a);
    checks.noScript = !$('script', a) && !window.pwned;
    checks.noUnsafeLink = !$('[href^="javascript:"]', a);
    checks.outline = analyze(d).headings.length === 2;
    checks.sourceHighlight = !!$('.hl .t-h1', d.el.root);
    checks.overlayAligned = d.el.hl.offsetHeight > 0 && Math.abs(d.el.hl.offsetHeight - d.el.ta.offsetHeight) <= 2;
    checks.dirty = isDirty(d) && $('.tab.active').classList.contains('dirty');
    toggleTask(d, 0);
    checks.taskToggle = d.text.includes('- [x] task one');
    setMode(d, 'read');
    await new Promise(r => setTimeout(r, 50));
    const para = $$(':scope > [data-line]', d.el.article).find(x => x.tagName === 'H2');
    startLiveEdit(d, para);
    checks.liveEditorOpen = !!$('.live-editor textarea', d.el.article);
    d.live.ta.value = '## Second, edited live';
    d.live.ta.dispatchEvent(new Event('input'));
    commitLive(d);
    checks.liveEditApplied = d.text.endsWith('## Second, edited live') && !!$('h2#second-edited-live', d.el.article);
    closePalette(null);
    d.baseline = d.text;
    await closeTab(d.key, {force: true});
    checks.workspaces = S.workspaces.length;
    checks.roots = ws().roots.length;
    checks.treeRows = $$('#explorer-body .tree .row').length;
    checks.tabs = tabs.length;
    // Leave a representative screen for the snapshot.
    if (tabs.length) activate(tabs[0]);
    return checks;
  },
};

/* ───────────────────────── boot ───────────────────────── */

async function boot() {
  explorerEvents(); tabEvents(); findEvents(); paletteEvents(); globalEvents(); liveEvents();
  applySettings();
  const info = await api('ready');
  HOME = info?.home || '';
  let saved = null;
  try { saved = JSON.parse((await api('loadState')) || 'null'); } catch {}
  if (saved?.workspaces?.length) {
    S = {...S, ...saved, settings: {...DEFAULTS, ...saved.settings}};
    S.recent ||= [];
  } else {
    const w = newWorkspaceObject('My Workspace');
    S.workspaces = [w]; S.current = w.id;
  }
  if (!ws()) S.current = S.workspaces[0].id;
  S.settings.zen = false;
  applySettings();
  await loadWorkspace();
  if (info?.paths?.length) await openPaths(info.paths);
  renderAll();
  document.body.classList.add('ready');
  if (info?.testing) setTimeout(async () => { let r; try { r = await window.folio.selftest(); } catch (e) { r = {error: String(e && e.stack || e)}; } await new Promise(x => setTimeout(x, 400)); api('testResult', {json: JSON.stringify(r)}); }, 1200);
}

/* ───────────────────────── browser mock (for development in a normal browser) ───────────────────────── */

function mock(action, p) {
  const KEY = 'folio-demo-fs-v2';
  const fs = mock.fs ||= JSON.parse(localStorage.getItem(KEY) || 'null') || {
    '/Notebook/Welcome.md': '# Welcome to Folio\n\nThis is a **live-edit reader**. Everything you see is rendered Markdown — and every block is editable. Go on: *click this paragraph.*\n\n> [!TIP]\n> Press **Esc** (or click somewhere else) to put the block back. Your Markdown is still plain text underneath.\n\n## Try a few things\n\n- [x] Open Folio\n- [ ] Tick this box — it rewrites the Markdown for you\n- [ ] Press `⌘P` and jump to **Roadmap**\n- [ ] Switch to Split view with the buttons at the top right\n\n## Links between notes\n\nNotes link to each other like a wiki: read the [roadmap](Projects/Roadmap.md), skim the [reading list](Reading/Reading%20list.md), or jump to a [section](Projects/Roadmap.md#later).\n\n---\n\nNothing here leaves your browser. In the Mac app, nothing leaves your Mac.\n',
    '/Notebook/Projects/Roadmap.md': '---\ntitle: Roadmap\nstatus: in progress\nowner: you\n---\n# Roadmap\n\n## Now\n\n| Area | What | Why it matters |\n|---|---|---|\n| Reading | Typography presets, reading column width, four themes | Long documents should feel like a book, not a terminal |\n| Editing | Click-to-edit blocks, list continuation, ⌘B / ⌘I / ⌘K | Fix a typo without leaving the page |\n| Workspaces | Folders + open tabs, saved per workspace | Jump between projects in one keystroke |\n\n## Next\n\n- [x] Command palette\n- [x] Workspace search with regex\n- [ ] Math and diagrams\n- [ ] Export to PDF\n\n## Later\n\n```rust\nfn main() {\n    let notes = folio::open("~/Notebook")?;\n    println!("{} notes, zero lock-in", notes.len());\n}\n```\n',
    '/Notebook/Reading/Reading list.md': '# Reading list\n\n> Good ideas deserve a little room to breathe.\n\n1. *The Design of Everyday Things* — Don Norman\n2. *How Buildings Learn* — Stewart Brand\n3. *A Pattern Language* — Christopher Alexander\n\n## Notes\n\nKeep quotes in blockquotes, and summaries short. Folio’s outline (⇧⌘U) turns these headings into a table of contents.\n',
    '/Notebook/Journal/2026-11-06.md': '# Friday, 6 November\n\n- Shipped the landing page\n- Coffee with the team\n\n> [!NOTE]\n> Daily notes are just files. Back them up any way you like.\n',
  };
  const save = () => localStorage.setItem(KEY, JSON.stringify(fs));
  const now = () => Date.now() / 1000;
  const dirs = () => { const set = new Set(); for (const f of Object.keys(fs)) { let d = dirname(f); while (d !== '/') { set.add(d); d = dirname(d); } } (mock.dirs ||= new Set()).forEach(x => set.add(x)); return set; };
  switch (action) {
    case 'ready': return Promise.resolve({paths: [], home: '/Users/you'});
    case 'loadState': return Promise.resolve(localStorage.getItem('folio-demo-state-v2') || JSON.stringify({workspaces: [
      {id: 'demo', name: 'Notebook', roots: ['/Notebook'], tabs: [{key: '/Notebook/Welcome.md', path: '/Notebook/Welcome.md', mode: 'read'}, {key: '/Notebook/Projects/Roadmap.md', path: '/Notebook/Projects/Roadmap.md', mode: 'read'}], active: '/Notebook/Welcome.md', expanded: ['/Notebook', '/Notebook/Projects']},
      {id: 'work', name: 'Client work', roots: [], tabs: [], opened: Date.now() - 86400000 * 2}], current: 'demo', recent: [], settings: {sidebarWidth: 240}}));
    case 'saveState': localStorage.setItem('folio-demo-state-v2', p.json); return Promise.resolve(true);
    case 'tree': {
      const all = dirs();
      const build = path => ({name: basename(path), path, dir: true, children: [
        ...[...all].filter(d => dirname(d) === path).sort().map(build),
        ...Object.keys(fs).filter(f => dirname(f) === path).sort().map(f => ({name: basename(f), path: f, dir: false}))]});
      return Promise.resolve(build(p.root));
    }
    case 'listFiles': return Promise.resolve(Object.keys(fs).filter(f => p.roots.some(r => within(f, r))));
    case 'read': return fs[p.path] != null ? Promise.resolve({text: fs[p.path], mtime: 1}) : Promise.reject(new Error('Not found'));
    case 'write': fs[p.path] = p.text; save(); return Promise.resolve({mtime: 1});
    case 'stat': return Promise.resolve(Object.fromEntries(p.paths.map(x => [x, fs[x] != null ? {m: 1, dir: false} : dirs().has(x) ? {m: 1, dir: true} : null])));
    case 'create': if (p.dir) (mock.dirs ||= new Set()).add(p.path); else fs[p.path] = p.text; save(); return Promise.resolve(true);
    case 'rename': for (const k of Object.keys(fs)) if (within(k, p.from)) { fs[p.to + k.slice(p.from.length)] = fs[k]; delete fs[k]; } save(); return Promise.resolve(true);
    case 'trash': for (const k of Object.keys(fs)) if (within(k, p.path)) delete fs[k]; save(); return Promise.resolve(true);
    case 'search': {
      let re; try { re = new RegExp((p.word ? '\\b' : '') + (p.regex ? p.query : p.query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')) + (p.word ? '\\b' : ''), p.case ? 'g' : 'gi'); } catch { return Promise.resolve({error: 'Invalid regular expression'}); }
      const results = []; let total = 0;
      for (const [f, t] of Object.entries(fs)) { if (!p.roots.some(r => within(f, r))) continue; const matches = []; t.split('\n').forEach((line, i) => { let m; re.lastIndex = 0; while ((m = re.exec(line))) { if (!m[0].length) { re.lastIndex++; continue; } matches.push({line: i + 1, text: line, start: m.index, length: m[0].length}); } }); if (matches.length) { total += matches.length; results.push({path: f, matches}); } }
      return Promise.resolve({results, total});
    }
    case 'openDialog': return Promise.resolve(['/Demo']);
    case 'saveDialog': { const n = prompt('Save as', (p.dir || '/Demo') + '/' + p.name); return Promise.resolve(n); }
    case 'confirm': return Promise.resolve(confirm(p.message + '\n\n' + (p.detail || '')) ? 0 : 1);
    case 'copy': navigator.clipboard?.writeText(p.text).catch(() => {}); return Promise.resolve(true);
    case 'external': window.open(p.url, '_blank'); return Promise.resolve(true);
    default: return Promise.resolve(true);
  }
}

boot().catch(e => { console.error(e); document.body.classList.add('ready'); toast('Folio hit a problem starting up: ' + (e.message || e), 'error'); });
