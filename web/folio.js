// Folio's only JavaScript: a generic event/patch shim. It has no application logic.
// It forwards DOM events (data-a, data-i, data-keys, data-ctx, drags, scrolls) to the
// Rust `ui` command and applies the patch operations Rust sends back.
(() => {
  const invoke = window.__TAURI__.core.invoke;
  const $ = (s, r = document) => r.querySelector(s);
  const byId = id => document.getElementById(id);
  const queue = [];
  let busy = null, typeAhead = [];

  /* ── sending events (in order; repeated input on one field coalesces) ── */
  function send(ev) {
    const last = queue[queue.length - 1];
    if (ev.t === 'i' && last && last.t === 'i' && last.i === ev.i) queue[queue.length - 1] = ev; else queue.push(ev);
    pump();
  }
  async function pump() {
    if (busy || !queue.length) return;
    busy = queue.shift();
    try { apply(await invoke('ui', {ev: busy})); } catch (e) { console.error('folio', e); }
    const k = busy; busy = null;
    if (k.t === 'k' && typeAhead.length) flushTypeAhead();
    pump();
  }
  function flushTypeAhead() {
    const keys = typeAhead; typeAhead = [];
    for (const [el, key] of keys) { if (document.activeElement !== el) continue; key === 'Backspace' ? document.execCommand('delete') : document.execCommand('insertText', false, key); }
  }
  const sel = el => el && 'selectionStart' in el ? {v: el.value, s: el.selectionStart, e: el.selectionEnd} : el && 'value' in el ? {v: el.value} : {};
  const rect = el => { const r = el.getBoundingClientRect(); return [r.left, r.top, r.right, r.bottom]; };
  const docOf = el => el?.closest('.doc')?.dataset.n;

  /* ── applying operations ── */
  function apply(ops) {
    for (const o of ops || []) try { OPS[o.op](o); } catch (e) { console.error('folio op', o.op, e); }
  }
  function setHtml(el, h) {
    const a = document.activeElement;
    const keep = a && a !== document.body && el.contains(a) && a.id ? {id: a.id, ...sel(a)} : null;
    el.innerHTML = h;
    if (keep) { const n = byId(keep.id); if (n) { n.focus({preventScroll: true}); if (keep.s != null && 'value' in n) { n.value = keep.v; n.setSelectionRange(keep.s, keep.e); } } }
  }
  const fit = ta => { ta.style.height = '0px'; ta.style.height = ta.scrollHeight + 'px'; };
  function keepVisible(el, margin = 24) {
    const sc = el.closest('.scroller'); if (!sc) return;
    const top = el.offsetTop, bottom = top + el.offsetHeight;
    if (top < sc.scrollTop + 8) sc.scrollTop = top - 8;
    else if (bottom > sc.scrollTop + sc.clientHeight - margin) sc.scrollTop = bottom - sc.clientHeight + margin;
  }
  const doc = n => byId('doc-' + n);
  const blockAt = (art, line) => { let b = null; for (const x of art.children) { if (x.dataset.line == null) continue; if (+x.dataset.line <= line) b = x; else break; } return b; };
  function flash(el) { el.classList.remove('flash'); void el.offsetWidth; el.classList.add('flash'); }
  const OPS = {
    html: o => { const el = byId(o.id); if (el) setHtml(el, o.h); },
    attr: o => document.querySelectorAll(o.sel).forEach(el => o.v == null ? el.removeAttribute(o.k) : el.setAttribute(o.k, o.v)),
    cls: o => document.querySelectorAll(o.sel).forEach(el => el.classList.toggle(o.c, !!o.on)),
    css: o => document.documentElement.style.setProperty(o.k, o.v),
    val: o => { const el = byId(o.id); if (!el) return; const s = el.selectionStart, e = el.selectionEnd; el.value = o.v; if (o.s != null) el.setSelectionRange(o.s, o.e); else el.setSelectionRange(Math.min(s, o.v.length), Math.min(e, o.v.length)); if (el.classList.contains('live-ta')) fit(el); },
    focus: o => { const el = o.id ? byId(o.id) : $(o.sel); if (!el) return; el.focus({preventScroll: !!o.still}); if (o.s != null) el.setSelectionRange(o.s, o.e); else if (o.all) el.select(); },
    edit: o => { const el = byId(o.id); if (!el) return; el.focus({preventScroll: true}); el.setSelectionRange(o.s, o.e); o.text ? document.execCommand('insertText', false, o.text) : document.execCommand('delete'); if (o.ss != null) el.setSelectionRange(o.ss, o.se); },
    docnew: o => { const t = document.createElement('template'); t.innerHTML = o.h; byId('docs').append(t.content); },
    rm: o => document.querySelectorAll(o.sel).forEach(el => el.remove()),
    nth: o => { const p = byId(o.id); if (!p) return; p.querySelector(':scope > .' + o.c)?.classList.remove(o.c); const el = p.children[o.n]; if (!el) return; el.classList.add(o.c); if (o.vis && document.activeElement === byId('ta-' + o.doc)) keepVisible(el); },
    scroll: o => $(o.sel)?.scrollIntoView({block: 'nearest', inline: 'nearest'}),
    live: o => {
      const art = byId('art-' + o.doc); if (!art) return;
      const t = document.createElement('template'); t.innerHTML = o.h; const ed = t.content.firstElementChild;
      const target = o.append ? null : [...art.children].find(x => x.dataset.line == o.line);
      if (target) target.replaceWith(ed); else art.append(ed);
      const ta = ed.querySelector('textarea'); fit(ta); requestAnimationFrame(() => fit(ta));
      ta.focus({preventScroll: true}); ta.setSelectionRange(o.caret, o.caret);
      if (o.append) ed.scrollIntoView({block: 'nearest', behavior: 'smooth'});
    },
    reveal: o => {
      const d = doc(o.doc); if (!d) return;
      const mode = d.dataset.mode, src = $('.source .scroller', d), pv = $('.preview .scroller', d), art = byId('art-' + o.doc), hl = byId('hl-' + o.doc), ta = byId('ta-' + o.doc);
      requestAnimationFrame(() => {
        if (mode !== 'read' && hl) {
          const ln = hl.children[Math.floor(o.line)], frac = o.line % 1;
          const top = ln ? ln.offsetTop + frac * ln.offsetHeight : 0;
          if (o.s != null) { ta.focus({preventScroll: true}); ta.setSelectionRange(o.s, o.e); }
          else if (o.focus) ta.focus({preventScroll: true});
          src.scrollTop = o.top ? top - 8 : top - src.clientHeight / 3;
        }
        if (mode !== 'source' && art) {
          const el = o.anchor ? art.querySelector(`[id="${CSS.escape(o.anchor)}"]`) : blockAt(art, o.line);
          if (el) { pv.scrollTo({top: el.offsetTop + art.offsetTop - (o.top ? 24 : 28), behavior: o.top ? 'instant' : 'smooth'}); if (o.flash) flash(el); }
          else if (o.line <= 0) pv.scrollTop = 0;
        }
      });
    },
    findcur: o => {
      const box = byId((o.pane === 'source' ? 'fl-' : 'art-') + o.doc); if (!box) return;
      box.querySelectorAll('mark.cur').forEach(m => m.classList.remove('cur'));
      const m = box.querySelector(o.pane === 'source' ? `mark[data-i="${o.i}"]` : `mark[data-f="${o.i}"]`); if (!m) return;
      m.classList.add('cur');
      if (!o.move) return;
      const sc = m.closest('.scroller'), r = m.getBoundingClientRect(), sr = sc.getBoundingClientRect(), top = r.top - sr.top;
      if (top < 40 || top > sc.clientHeight - 60) sc.scrollTop += top - sc.clientHeight / 3;
      if (o.s != null) byId('ta-' + o.doc)?.setSelectionRange(o.s, o.e);
    },
    menu: o => {
      const m = byId('menu');
      if (o.hide) { m.hidden = true; return; }
      m.innerHTML = o.h; m.hidden = false;
      const r = m.getBoundingClientRect();
      m.style.left = Math.max(6, Math.min(o.x, innerWidth - r.width - 6)) + 'px';
      m.style.top = Math.max(6, Math.min(o.y, innerHeight - r.height - 6)) + 'px';
    },
    toast: o => {
      const t = document.createElement('div'); t.className = 'toast ' + (o.kind || 'info'); t.innerHTML = o.h;
      byId('toasts').append(t); requestAnimationFrame(() => t.classList.add('in'));
      setTimeout(() => { t.classList.remove('in'); setTimeout(() => t.remove(), 250); }, o.ms || 2600);
    },
    probe: o => {
      const r = {};
      for (const [k, [s, what]] of Object.entries(o.q)) {
        const all = document.querySelectorAll(s), el = all[0];
        r[k] = what === 'count' ? all.length : what === 'width' ? (el ? Math.round(el.getBoundingClientRect().width) : 0) : what === 'height' ? (el ? el.offsetHeight : 0) : what === 'html' ? (el ? el.outerHTML : '') : what === 'text' ? (el ? el.textContent : '') : !!el;
      }
      send({t: 'probe', r, tag: o.tag});
    },
    // Test hooks for scripted runs (FOLIO_SCRIPT): real DOM events through the normal paths.
    click: o => {
      const el = $(o.sel); if (!el) return console.warn('no', o.sel);
      const r = el.getBoundingClientRect(), init = {bubbles: true, cancelable: true, button: o.button || 0, clientX: r.left + Math.min(8, r.width / 2), clientY: r.top + r.height / 2, altKey: !!o.alt};
      for (const t of o.only ? [o.only] : ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) el.dispatchEvent(t.startsWith('pointer') ? new PointerEvent(t, init) : new MouseEvent(t, init));
    },
    type: o => { const el = $(o.sel); el.focus(); if (o.v != null) el.value = o.v; if (o.s != null) el.setSelectionRange(o.s, o.e ?? o.s); el.dispatchEvent(new Event('input', {bubbles: true})); },
    key: o => { const el = (o.sel && $(o.sel)) || document.activeElement; el.focus?.(); el.dispatchEvent(new KeyboardEvent('keydown', {key: o.key, metaKey: !!o.meta, shiftKey: !!o.shift, ctrlKey: !!o.ctrl, altKey: !!o.alt, bubbles: true, cancelable: true})); },
    sel: o => window.folio.sel(o.cmd),
    ready: () => document.body.classList.add('ready'),
  };

  /* ── input from the page ── */
  const mods = e => (e.ctrlKey ? 'Ctrl+' : '') + (e.altKey ? 'Alt+' : '') + (e.shiftKey ? 'Shift+' : '') + (e.metaKey ? 'Meta+' : '');
  const keyName = e => e.key === ' ' ? 'Space' : e.key.length === 1 ? e.key.toLowerCase() : e.key;

  document.addEventListener('keydown', e => {
    if (e.isComposing) return;
    const t = e.target;
    if (busy && busy.t === 'k' && busy.el === t.id && !e.metaKey && !e.ctrlKey && (e.key.length === 1 || e.key === 'Backspace' || e.key === 'Enter')) { e.preventDefault(); typeAhead.push([t, e.key === 'Enter' ? '\n' : e.key]); return; }
    const combo = mods(e) + keyName(e);
    for (let el = t; el; el = el.parentElement) {
      const keys = el.dataset?.keys;
      if (keys && keys.split(',').includes(combo)) {
        e.preventDefault(); e.stopPropagation();
        send({t: 'k', k: combo, id: el.dataset.k, el: t.id, doc: docOf(t), ...sel('value' in t ? t : el)});
        return;
      }
    }
  }, true);

  document.addEventListener('input', e => {
    const t = e.target, i = t.dataset?.i;
    if (!i) return;
    if (t.classList.contains('live-ta')) fit(t);
    send({t: 'i', i, doc: docOf(t), ...sel(t)});
  });
  document.addEventListener('change', e => { const t = e.target; if (t.tagName === 'SELECT' && t.dataset.i) send({t: 'i', i: t.dataset.i, v: t.value}); });

  for (const type of ['keyup', 'mouseup', 'select']) document.addEventListener(type, e => {
    const t = e.target;
    if (t.dataset?.caret) send({t: 'caret', doc: t.dataset.caret, s: t.selectionStart, e: t.selectionEnd});
  });
  document.addEventListener('focusin', e => { const n = docOf(e.target); if (n && e.target.tagName === 'TEXTAREA' && e.target.dataset.caret) send({t: 'focus', doc: n, pane: 'source'}); });
  document.addEventListener('focusout', e => {
    const t = e.target, b = t.dataset?.blur;
    if (!b) return;
    const to = e.relatedTarget;
    if (b === 'live' && (!to || t.closest('.live-editor')?.contains(to))) return;
    send({t: 'blur', b, ...sel(t)});
  });

  document.addEventListener('pointerdown', e => {
    const t = e.target;
    if (!byId('menu').hidden && !t.closest('#menu')) byId('menu').hidden = true;
    if (e.button !== 0) return;
    const self = t.dataset?.self;
    if (self) { e.preventDefault(); send({t: 'a', a: self}); return; }
    const n = docOf(t);
    if (n && t.closest('.pane.preview')) send({t: 'focus', doc: n, pane: 'preview'});
    // While a block is being edited, clicking elsewhere commits it (and may open another block).
    if (document.body.dataset.live && !t.closest('.live-editor, #palette, #menu, #findwidget')) {
      const block = t.closest('.doc[data-mode=read] .markdown-body > [data-line]');
      if (t.closest('a[href], input[data-task], .code-head')) send({t: 'outside', soft: true});
      else if (block) { e.preventDefault(); send({t: 'block', doc: n, line: +block.dataset.line, hint: hint(e, block)}); }
      else if (t.closest('.endmark')) { e.preventDefault(); send({t: 'append', doc: n}); }
      else send({t: 'outside'});
    }
    // Clicking the empty part of the source pane puts the caret at the end.
    const sc = t.closest('.source .scroller');
    if (sc && (t === sc || t.classList.contains('stack'))) {
      e.preventDefault(); const ta = $('textarea', sc); ta.focus(); ta.setSelectionRange(ta.value.length, ta.value.length);
      send({t: 'caret', doc: n, s: ta.value.length, e: ta.value.length});
    }
    startDrag(e);
  }, true);

  let suppressClick = 0;
  document.addEventListener('click', e => {
    if (performance.now() < suppressClick) { e.stopPropagation(); e.preventDefault(); return; }
    const t = e.target;
    const copy = t.closest('[data-copy]');
    if (copy) {
      send({t: 'copy', doc: docOf(copy), n: +copy.dataset.copy});
      const was = copy.innerHTML; copy.classList.add('done'); copy.innerHTML = byId('copied-tpl').innerHTML;
      setTimeout(() => { copy.classList.remove('done'); copy.innerHTML = was; }, 1400);
      return;
    }
    const a = t.closest('[data-a]');
    if (a) { send({t: 'a', a: a.dataset.a, r: rect(a), x: e.clientX, y: e.clientY}); return; }
    const art = t.closest('.markdown-body');
    if (!art || t.closest('.live-editor')) return;
    const n = docOf(art);
    const box = t.closest('input[data-task]');
    if (box) { e.preventDefault(); send({t: 'task', doc: n, n: +box.dataset.task}); return; }
    const link = t.closest('a[href]');
    const read = art.closest('.doc').dataset.mode === 'read';
    if (link && !(read && e.altKey)) { e.preventDefault(); send({t: 'link', doc: n, href: link.getAttribute('href')}); return; }
    if (!read || t.closest('.code-head') || !getSelection().isCollapsed || document.body.dataset.live) return;
    const block = t.closest('.markdown-body > [data-line]');
    if (block) send({t: 'block', doc: n, line: +block.dataset.line, hint: hint(e, block)});
    else if (t === art && !art.children.length) send({t: 'append', doc: n});
  });
  document.addEventListener('dblclick', e => { const a = e.target.closest('[data-dbl]'); if (a) send({t: 'a', a: a.dataset.dbl}); });
  document.addEventListener('auxclick', e => { const a = e.target.closest('[data-aux]'); if (a && e.button === 1) { e.preventDefault(); send({t: 'a', a: a.dataset.aux}); } });
  document.addEventListener('contextmenu', e => {
    const a = e.target.closest('[data-ctx]');
    if (!a) return;
    e.preventDefault();
    send({t: 'ctx', c: a.dataset.ctx, x: e.clientX, y: e.clientY});
  });
  document.addEventListener('wheel', e => {
    const h = e.target.closest('[data-hwheel]');
    if (h && Math.abs(e.deltaY) > Math.abs(e.deltaX)) { h.scrollLeft += e.deltaY; e.preventDefault(); }
  }, {passive: false});
  document.addEventListener('error', e => { if (e.target.tagName === 'IMG') e.target.classList.add('broken'); }, true);
  window.addEventListener('blur', () => { byId('menu').hidden = true; });
  window.addEventListener('resize', () => { byId('menu').hidden = true; });

  /** Rendered text from the start of a block up to the click point (for caret placement). */
  function hint(e, block) {
    const r = document.caretRangeFromPoint?.(e.clientX, e.clientY);
    if (!r || !block.contains(r.startContainer)) return null;
    const pre = document.createRange(); pre.selectNodeContents(block);
    try { pre.setEnd(r.startContainer, r.startOffset); } catch { return null; }
    return pre.toString();
  }

  /* ── pointer drags: moving items (data-drag → data-drop) and resizing (data-resize) ── */
  function startDrag(e) {
    const t = e.target;
    const rz = t.closest('[data-resize]');
    if (rz) return resize(e, rz);
    const src = t.closest('[data-drag]');
    if (!src || t.tagName === 'INPUT' || t.closest('.tab-close')) return;
    const kind = src.dataset.drag.split(':')[0], x0 = e.clientX, y0 = e.clientY;
    let dragging = false, over = null, after = false, ghost = null;
    const clear = () => document.querySelectorAll('.drop-target,.drop-before,.drop-after').forEach(x => x.classList.remove('drop-target', 'drop-before', 'drop-after'));
    const move = ev => {
      if (!dragging && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 6) return;
      if (!dragging) {
        dragging = true; document.body.classList.add(kind === 'tab' ? 'dragging-tab' : 'dragging-row'); src.classList.add('dragging');
        if (kind !== 'tab') { ghost = document.createElement('div'); ghost.className = 'drag-ghost'; ghost.innerHTML = (src.querySelector('.i:not(.twisty)')?.outerHTML || '') + (src.querySelector('.label')?.innerHTML || ''); document.body.append(ghost); }
      }
      if (ghost) ghost.style.transform = `translate(${ev.clientX + 12}px, ${ev.clientY + 8}px)`;
      clear(); over = null;
      const y = kind === 'tab' ? src.getBoundingClientRect().top + 12 : ev.clientY;
      const el = document.elementFromPoint(ev.clientX, y)?.closest('[data-drop]');
      if (el && el.dataset.drop.startsWith(kind + ':') && el !== src) {
        over = el;
        if (kind === 'tab') { const r = el.getBoundingClientRect(); after = ev.clientX >= r.left + r.width / 2; el.classList.add(after ? 'drop-after' : 'drop-before'); }
        else el.classList.add('drop-target');
      }
      const box = src.parentElement;
      if (kind === 'tab') { const br = box.getBoundingClientRect(); if (ev.clientX > br.right - 30) box.scrollLeft += 12; else if (ev.clientX < br.left + 30) box.scrollLeft -= 12; }
    };
    const up = () => {
      removeEventListener('pointermove', move);
      document.body.classList.remove('dragging-tab', 'dragging-row'); src.classList.remove('dragging'); ghost?.remove(); clear();
      if (dragging) suppressClick = performance.now() + 120;
      if (dragging && over) send({t: 'drop', drag: src.dataset.drag, drop: over.dataset.drop, after});
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', up, {once: true});
  }
  function resize(e, el) {
    const kind = el.dataset.resize, ref = el.dataset.ref ? el.closest(el.dataset.ref) || $(el.dataset.ref) : el.parentElement;
    const [cssVar, min, max, frac] = [el.dataset.var, +el.dataset.min, +el.dataset.max, el.dataset.frac != null];
    el.setPointerCapture(e.pointerId);
    document.body.classList.add('resizing-x'); el.closest('.doc')?.classList.add('resizing');
    let v = null;
    const move = ev => {
      const b = ref.getBoundingClientRect();
      v = frac ? (ev.clientX - b.left) / b.width : Math.round(ev.clientX - b.left);
      v = Math.min(max, Math.max(min, v));
      document.documentElement.style.setProperty(cssVar, frac ? (v * 100) + '%' : v + 'px');
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', () => {
      el.removeEventListener('pointermove', move);
      document.body.classList.remove('resizing-x'); el.closest('.doc')?.classList.remove('resizing');
      if (v != null) send({t: 'resize', k: kind, v});
    }, {once: true});
    el.ondblclick = () => send({t: 'resize', k: kind, reset: true});
  }

  /* ── scroll: split-view sync and position reports ── */
  let syncFrom = null, syncAt = 0;
  const lineAt = (hl, y) => {
    const kids = hl.children; if (!kids.length) return 0;
    let lo = 0, hi = kids.length - 1;
    while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (kids[mid].offsetTop <= y) lo = mid; else hi = mid - 1; }
    return lo + Math.min(1, Math.max(0, (y - kids[lo].offsetTop) / Math.max(1, kids[lo].offsetHeight)));
  };
  const blocks = art => [...art.children].filter(x => x.dataset.line != null);
  const topOf = (art, el) => el.offsetTop + art.offsetTop;
  function previewLine(art, pv) {
    const bs = blocks(art), y = pv.scrollTop + 24;
    if (!bs.length) return 0;
    let i = 0; while (i + 1 < bs.length && topOf(art, bs[i + 1]) <= y) i++;
    const a = bs[i], b = bs[i + 1], ya = topOf(art, a), yb = b ? topOf(art, b) : pv.scrollHeight;
    const f = Math.max(0, Math.min(1, (y - ya) / Math.max(1, yb - ya)));
    return +a.dataset.line + f * ((b ? +b.dataset.line : +a.dataset.line + 1) - +a.dataset.line);
  }
  const reported = {};
  function report(n, line) {
    clearTimeout(reported[n]);
    reported[n] = setTimeout(() => send({t: 'scroll', doc: n, line}), 90);
  }
  document.addEventListener('scroll', e => {
    const sc = e.target;
    if (!(sc instanceof Element)) return;
    if (sc.matches('.stack textarea')) { if (sc.scrollTop || sc.scrollLeft) { sc.scrollTop = 0; sc.scrollLeft = 0; } return; }
    if (!sc.classList.contains('scroller')) return;
    const d = sc.closest('.doc'); if (!d || d.classList.contains('special')) return;
    const n = d.dataset.n, mode = d.dataset.mode, art = byId('art-' + n), hl = byId('hl-' + n);
    const src = $('.source .scroller', d), pv = $('.preview .scroller', d);
    const fromSource = sc === src;
    if (fromSource ? mode !== 'read' : mode !== 'source') report(n, fromSource ? lineAt(hl, src.scrollTop + 30) : previewLine(art, pv));
    if (mode !== 'split') return;
    if (d.dataset.sync !== '1' || d.classList.contains('large')) return;
    const now = performance.now(), from = fromSource ? 'source' : 'preview';
    if (syncFrom && syncFrom !== from && now - syncAt < 80) return;
    syncFrom = from; syncAt = now;
    const bs = blocks(art); if (!bs.length) return;
    if (fromSource) {
      if (src.scrollTop <= 0) { pv.scrollTop = 0; return; }
      if (src.scrollTop >= src.scrollHeight - src.clientHeight - 1) { pv.scrollTop = pv.scrollHeight; return; }
      const L = lineAt(hl, src.scrollTop);
      let i = 0; while (i + 1 < bs.length && +bs[i + 1].dataset.line <= L) i++;
      const a = bs[i], b = bs[i + 1], ya = topOf(art, a), yb = b ? topOf(art, b) : pv.scrollHeight;
      const f = b ? (L - +a.dataset.line) / Math.max(1, +b.dataset.line - +a.dataset.line) : 0;
      pv.scrollTop = ya + Math.max(0, Math.min(1, f)) * (yb - ya) - 24;
    } else {
      if (pv.scrollTop <= 0) { src.scrollTop = 0; return; }
      if (pv.scrollTop >= pv.scrollHeight - pv.clientHeight - 1) { src.scrollTop = src.scrollHeight; return; }
      const L = previewLine(art, pv), li = Math.floor(L), el = hl.children[li];
      src.scrollTop = (el ? el.offsetTop : 0) + (L - li) * (el?.offsetHeight || 20) - 24;
    }
  }, true);

  /* ── Rust → page ── */
  window.folio = {
    apply,
    // The native menu asks for the focused editor's selection before bold/italic/link.
    sel(cmd) {
      const a = document.activeElement;
      if (a && a.tagName === 'TEXTAREA') send({t: 'sel', cmd, id: a.id, doc: docOf(a), ...sel(a)});
      else send({t: 'sel', cmd, v: String(getSelection() || '')});
    },
    send,
  };
  send({t: 'ready'});
})();
