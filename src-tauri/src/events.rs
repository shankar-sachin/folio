//! Event dispatch, commands, the palette, context menus, editor keys and `flush`.

use crate::app::*;
use crate::commands::{self, fuzzy, mark_idx, COMMANDS};
use crate::fsx::{self, basename, byte_to_u16, dirname, ext, is_markdown, u16_to_byte, within};
use crate::model::{Kind, Mode, Pane};
use crate::view::{fmt_num, menu_html};
use folio_core::{esc, icon};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
fn n(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}
fn u(v: &Value, k: &str) -> usize {
    v.get(k).and_then(Value::as_u64).unwrap_or(0) as usize
}
fn docn(v: &Value) -> String {
    match v.get("doc") {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(x)) => x.to_string(),
        _ => String::new(),
    }
}

type MenuItem = Option<(String, String, bool, Job)>;
fn mi(label: &str, kbd: &str, f: impl FnOnce(&mut App) + Send + 'static) -> MenuItem {
    Some((label.into(), kbd.into(), false, Box::new(f)))
}
fn mi_danger(label: &str, kbd: &str, f: impl FnOnce(&mut App) + Send + 'static) -> MenuItem {
    Some((label.into(), kbd.into(), true, Box::new(f)))
}

static LIST_CONT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*(?:>\s?)*)([-*+]|(\d+)([.)]))(\s+)(\[[ xX]\]\s+)?").unwrap());
static LIST_START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*([-*+]|\d+[.)])\s").unwrap());

const TEST_DOC: &str = "---\ntitle: Test\n---\n# Safety\n\n<script>window.pwned=true</script>\n\n[bad](javascript:alert(1))\n\n> [!NOTE]\n> Callout body\n\n| Idea | Audience | Views |\n|---|---|---|\n| Investigation Atlas | Undergrads, judges | Card grid by hardware family, filters (fuel, solid/gas/droplet, platform), detail drawer with DOI |\n\n- [ ] task one\n- [x] task two\n\n```python\ndef f():\n    return 42\n```\n\n## Second";

impl App {
    /* ───────────── flush: state → patches ───────────── */

    pub fn flush(&mut self) -> Vec<Value> {
        if !self.booted {
            return self.ui.take();
        }
        let f = std::mem::take(&mut self.dirty);
        if f & F_LAYOUT != 0 {
            self.flush_layout();
        }
        if f & F_ACT != 0 {
            let (top, bottom) = self.view_activity();
            self.ui.html("ab-top", top);
            self.ui.html("ab-bottom", bottom);
        }
        if f & (F_SIDE | F_TABS) != 0 {
            self.flush_sidebar();
        }
        if f & F_TABS != 0 {
            let tabs = self.view_tabs();
            self.ui.html("tabs", tabs);
            self.ui.cls("#tabbar", "empty", self.tabs.is_empty());
            let acts = self.view_editor_actions();
            self.ui.html("editor-actions", acts);
            self.ui.later(json!({"op": "scroll", "sel": "#tabs .tab.active"}));
        }
        if f & F_EDITOR != 0 {
            self.flush_editor();
        }
        if f & F_CRUMBS != 0 {
            match self.view_breadcrumbs() {
                Some(h) => {
                    self.ui.html("breadcrumbs", h);
                    self.ui.attr("#breadcrumbs", "hidden", None);
                }
                None => self.ui.attr("#breadcrumbs", "hidden", Some("")),
            }
        }
        if f & F_STATUS != 0 {
            let (l, r) = self.view_status();
            self.ui.html("sb-left", l);
            self.ui.html("sb-right", r);
        }
        if f & F_TITLE != 0 {
            self.flush_title();
        }
        if f & F_FIND != 0 {
            self.flush_find();
        }
        if f & F_PAL != 0 {
            self.ui.attr("#palette", "hidden", if self.pal.open { None } else { Some("") });
            let h = self.view_palette();
            self.ui.html("pal-list", h);
            self.ui.later(json!({"op": "scroll", "sel": ".pal-item.sel"}));
        }
        self.light_settings = false;
        self.ui.take()
    }

    fn flush_layout(&mut self) {
        let st = self.s.settings.clone();
        let th = theme(&st.theme);
        self.ui.attr("html", "data-theme", Some(th.id));
        self.ui.attr("html", "data-reader-font", Some(&st.reader_font));
        self.ui.css("--reader-size", format!("{}px", fmt_num(st.reader_size)));
        self.ui.css("--reader-width", format!("{}px", fmt_num(st.reader_width)));
        self.ui.css("--reader-lh", fmt_num(st.line_height));
        self.ui.css("--editor-size", format!("{}px", fmt_num(st.editor_size)));
        self.ui.css("--split", format!("{}%", fmt_num((st.split * 1000.0).round() / 10.0)));
        self.ui.css("--sidebar-w", format!("{}px", fmt_num(st.sidebar_width.round())));
        self.ui.cls("html", "nowrap", !st.wrap);
        self.ui.cls("html", "no-ln", !st.line_numbers);
        self.ui.cls("body", "no-sidebar", !st.sidebar_visible);
        self.ui.cls("body", "zen", st.zen);
        let (dark, bg) = (th.dark, th.bg);
        self.after(move |h| crate::window_theme(h, dark, bg));
    }

    fn flush_sidebar(&mut self) {
        let view = self.s.settings.view.clone();
        self.ui.attr("#sidebar", "data-on", Some(&view));
        match view.as_str() {
            "search" => {
                let (t, sum, res) = self.view_search();
                self.ui.html("search-toggles", t);
                self.ui.html("search-summary", sum);
                self.ui.html("search-results", res);
            }
            "outline" => {
                let h = self.view_outline();
                self.ui.html("outline-body", h);
            }
            "workspaces" => {
                let h = self.view_workspaces();
                self.ui.html("workspaces-body", h);
            }
            _ => {
                let h = self.view_explorer();
                self.ui.html("explorer-body", h);
            }
        }
        // Static view headers.
        let ib = |ic: &str, title: &str, act: &str| format!("<button class=\"icon-btn\" title=\"{title}\" data-a=\"{act}\">{}</button>", icon(ic, ""));
        self.ui.html("explorer-actions", ib("filePlus", "New File (⌘N)", "cmd:newFileInFolder") + &ib("folderPlus", "Add Folder to Workspace (⌥⌘O)", "cmd:addFolder") + &ib("more", "More", "explorermore"));
        self.ui.html("search-actions", ib("refresh", "Refresh", "srefresh") + &ib("collapse", "Collapse All", "scollapse"));
        self.ui.html("outline-actions", ib("hash", "Go to Heading (⇧⌘O)", "cmd:gotoHeading"));
        self.ui.html("workspaces-actions", ib("plus", "New Workspace", "cmd:newWorkspace"));
    }

    fn flush_editor(&mut self) {
        let active = self.active.clone();
        let keys: Vec<String> = self.tabs.clone();
        for k in &keys {
            if let Some(d) = self.docs.get(k) {
                self.ui.cls(&format!("#doc-{}", d.n), "active", active.as_deref() == Some(k.as_str()));
            }
        }
        self.ui.attr("#welcome", "hidden", if active.is_some() { Some("") } else { None });
        match active {
            None => {
                let h = self.view_welcome();
                self.ui.html("welcome", h);
            }
            Some(k) => self.sync_doc(&k),
        }
    }

    pub fn sync_doc(&mut self, key: &str) {
        let Some(d) = self.docs.get(key) else { return };
        let n = d.n;
        match d.kind {
            Kind::Settings => {
                if !self.light_settings {
                    let h = self.view_settings(n);
                    self.ui.html(&format!("sp-{n}"), h);
                }
                return;
            }
            Kind::Shortcuts => {
                let h = self.view_shortcuts();
                self.ui.html(&format!("sp-{n}"), h);
                return;
            }
            Kind::Md => {}
        }
        let sel = format!("#doc-{n}");
        self.ui.attr(&sel, "data-mode", Some(d.mode.as_str()));
        self.ui.cls(&sel, "large", d.large());
        self.ui.attr(&sel, "data-sync", Some(if self.s.settings.sync_scroll { "1" } else { "0" }));
        match self.view_banner(d) {
            Some(h) => {
                self.ui.html(&format!("ban-{n}"), h);
                self.ui.attr(&format!("#ban-{n}"), "hidden", None);
            }
            None => self.ui.attr(&format!("#ban-{n}"), "hidden", Some("")),
        }
        let d = self.docs.get_mut(key).unwrap();
        if d.mode != Mode::Read {
            if d.ta_stale {
                d.ta_stale = false;
                self.ui.now(json!({"op": "val", "id": format!("ta-{n}"), "v": d.text}));
            }
            if !d.large() {
                let h = folio_core::highlight_source(&d.text);
                self.ui.html(&format!("hl-{n}"), h);
                self.ui.later(json!({"op": "nth", "id": format!("hl-{n}"), "n": d.cursor.0, "c": "cur", "doc": n}));
            }
            let fl = if self.find.open && self.find.target == Some(Pane::Source) && !d.large() { folio_core::find_layer(&d.text, &self.find.ranges) } else { String::new() };
            self.ui.html(&format!("fl-{n}"), fl);
        }
        let d = &self.docs[key];
        if d.mode != Mode::Source && d.live.is_none() && d.preview_stale {
            self.sync_article(key);
        }
    }

    /// Render the preview article now (ops are queued in order with whatever follows).
    pub fn sync_article(&mut self, key: &str) {
        let r = self.rendered(key);
        let marked = if self.find.open && self.find.target == Some(Pane::Preview) && self.active.as_deref() == Some(key) && !self.find.query.is_empty() {
            fsx::build_regex(&self.find.query, self.find.regex, self.find.case, self.find.word)
                .ok()
                .map(|re| folio_core::mark_html(&r.html, &mut |t: &str| re.find_iter(t).filter(|m| m.start() < m.end()).map(|m| (m.start(), m.end())).collect(), 5000).0)
        } else {
            None
        };
        let d = self.docs.get_mut(key).unwrap();
        d.preview_stale = false;
        let n = d.n;
        self.ui.html(&format!("art-{n}"), marked.unwrap_or_else(|| r.html.clone()));
    }

    fn flush_title(&mut self) {
        let st = &self.s.settings;
        let side = icon(if st.sidebar_visible { "sidebarFill" } else { "sidebar" }, "");
        self.ui.html("tb-sidebar", side);
        let d = self.doc();
        let read = d.is_some_and(|d| d.mode == Mode::Read);
        let md = d.is_some_and(|d| d.kind == Kind::Md);
        let title = d.map(|d| format!("{}{} — {}", d.name, if d.is_dirty() { " •" } else { "" }, self.ws().name)).unwrap_or_else(|| format!("Folio — {}", self.ws().name));
        self.ui.html("tb-mode", icon(if read { "code" } else { "book" }, ""));
        self.ui.attr("#tb-mode", "title", Some(if read { "Show Source (⇧⌘V)" } else { "Back to Live (⇧⌘V)" }));
        self.ui.attr("#tb-mode", "disabled", if md { None } else { Some("") });
        self.ui.html("tb-theme", icon(if theme(&self.s.settings.theme).dark { "moon" } else { "sun" }, ""));
        self.ui.html("cc-icon", icon("search", ""));
        let name = esc(&self.ws().name);
        self.ui.html("cc-label", name);
        for (id, ic) in [("find-prev", "up"), ("find-next", "down"), ("find-close", "x")] {
            self.ui.html(id, icon(ic, ""));
        }
        if title != self.title {
            self.title = title.clone();
            self.after(move |h| crate::window_title(h, &title));
        }
    }

    fn flush_find(&mut self) {
        self.ui.attr("#findwidget", "hidden", if self.find.open { None } else { Some("") });
        let t = self.view_find_toggles();
        self.ui.html("find-toggles", t);
        let n = self.count_matches();
        let count = if !self.find.open {
            String::new()
        } else if self.find.error {
            "Invalid regex".into()
        } else if n == 0 || self.find.query.is_empty() {
            "No results".into()
        } else {
            format!("{} of {}", self.find.index.map(|i| i + 1).unwrap_or(0), if n >= 5000 { "5000+".into() } else { n.to_string() })
        };
        self.ui.html("find-count", count);
        self.ui.cls("#find-count", "bad", self.find.error);
    }

    /* ───────────── events ───────────── */

    pub fn handle(&mut self, ev: &Value) {
        let key_of = |app: &App| app.key_of_n(&docn(ev));
        match s(ev, "t") {
            "ready" => {
                if self.booted {
                    self.rehydrate();
                } else {
                    let paths = std::mem::take(&mut self.pending_paths);
                    self.boot(paths);
                    if self.testing {
                        self.defer(1200, |app| app.self_test_start());
                    }
                    if let Some(steps) = std::env::var("FOLIO_SCRIPT").ok().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok()) {
                        self.script = steps;
                        self.defer(800, |app| app.script_step());
                    }
                }
            }
            "a" => self.action(s(ev, "a"), ev),
            "ctx" => self.context(s(ev, "c"), n(ev, "x"), n(ev, "y")),
            "i" => self.input(s(ev, "i"), ev),
            "k" => self.key(s(ev, "id"), s(ev, "k"), ev),
            "blur" => match s(ev, "b") {
                "inline" => {
                    let v = s(ev, "v").trim().to_string();
                    self.finish_inline(Some(v).filter(|v| !v.is_empty()));
                }
                "live" => {
                    if let Some(k) = self.active.clone() {
                        self.commit_live(&k, true);
                    }
                }
                _ => {}
            },
            "block" => {
                if let Some(k) = key_of(self) {
                    let hint = ev.get("hint").and_then(Value::as_str).map(String::from);
                    self.start_live_edit(&k, u(ev, "line"), hint);
                }
            }
            "append" => {
                if let Some(k) = key_of(self) {
                    self.start_live_append(&k);
                }
            }
            "link" => {
                if let Some(k) = key_of(self) {
                    self.link(&k, s(ev, "href"));
                }
            }
            "task" => {
                if let Some(k) = key_of(self) {
                    self.toggle_task(&k, u(ev, "n"));
                }
            }
            "outside" => {
                if let Some(k) = self.active.clone() {
                    if ev.get("soft").and_then(Value::as_bool).unwrap_or(false) {
                        self.defer(250, move |app| app.commit_live(&k, true));
                    } else {
                        self.commit_live(&k, true);
                    }
                }
            }
            "caret" => {
                if let Some(k) = key_of(self) {
                    self.set_cursor(&k, u(ev, "s"), u(ev, "e"));
                }
            }
            "focus" => {
                if let Some(k) = key_of(self) {
                    let pane = if s(ev, "pane") == "preview" { Pane::Preview } else { Pane::Source };
                    let d = self.docs.get_mut(&k).unwrap();
                    if d.last_focus != pane {
                        d.last_focus = pane;
                        if self.find.open && d.mode == Mode::Split {
                            self.find_refresh(true);
                        }
                    }
                }
            }
            "scroll" => {
                if let Some(k) = key_of(self) {
                    self.track_heading(&k, n(ev, "line"));
                }
            }
            "drop" => self.drop(s(ev, "drag"), s(ev, "drop"), ev.get("after").and_then(Value::as_bool).unwrap_or(false)),
            "resize" => {
                let reset = ev.get("reset").and_then(Value::as_bool).unwrap_or(false);
                let st = &mut self.s.settings;
                match s(ev, "k") {
                    "sidebar" => st.sidebar_width = if reset { 272.0 } else { n(ev, "v").clamp(180.0, 560.0) },
                    "split" => st.split = if reset { 0.5 } else { n(ev, "v").clamp(0.2, 0.8) },
                    _ => {}
                }
                self.mark(F_LAYOUT);
                self.persist();
            }
            "sel" => self.selection_command(ev),
            "copy" => {
                if let Some(k) = key_of(self) {
                    let r = self.rendered(&k);
                    if let Some(code) = r.codes.get(u(ev, "n")) {
                        fsx::copy(code);
                    }
                }
            }
            "probe" if s(ev, "tag") == "dump" => {
                let html = ev["r"]["html"].as_str().unwrap_or("").to_string();
                if let Some(path) = self.test.take().and_then(|t| t["dump"].as_str().map(String::from)) {
                    let _ = std::fs::write(path, html);
                }
                self.defer(50, |app| app.script_step());
            }
            "probe" => self.self_test_finish(ev.get("r").cloned().unwrap_or_default()),
            _ => {}
        }
    }

    fn action(&mut self, a: &str, ev: &Value) {
        let (verb, arg) = a.split_once(':').unwrap_or((a, ""));
        let arg = arg.to_string();
        if self.menu_open && verb != "mi" {
            self.hide_menu();
        }
        let rect = ev.get("r").and_then(Value::as_array).map(|r| r.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect::<Vec<_>>()).unwrap_or_else(|| vec![0.0; 4]);
        match verb {
            "cmd" => self.command(&arg),
            "view" => self.show_view(&arg, true),
            "tab" => self.activate(&arg),
            "close" => self.close_tab(&arg, false, None),
            "pin" => {
                if let Some(d) = self.docs.get_mut(&arg) {
                    d.preview = false;
                    self.mark(F_TABS);
                }
            }
            "row" => {
                self.selected = Some(arg.clone());
                match self.find_node(&arg).map(|n| n.dir) {
                    Some(true) => {
                        if !self.expanded.remove(&arg) {
                            self.expanded.insert(arg);
                        }
                        self.mark(F_SIDE);
                        self.persist();
                    }
                    _ => {
                        self.open_file(&arg, OpenOpts { preview: true, ..Default::default() });
                    }
                }
                self.mark(F_SIDE);
            }
            "sec" => {
                let st = &mut self.s.settings;
                if arg == "openEditors" {
                    st.open_editors = !st.open_editors;
                } else {
                    st.ws_open = !st.ws_open;
                }
                self.mark(F_SIDE);
                self.persist();
            }
            "mode" => {
                if let Some(k) = self.md_key() {
                    self.set_mode(&k, Mode::parse(&arg));
                }
            }
            "moremenu" => self.more_menu(rect[2] - 220.0, rect[3] + 4.0),
            "explorermore" => self.explorer_menu(rect[0] - 180.0, rect[3] + 4.0),
            "crumb" => {
                self.expanded.insert(arg.clone());
                self.selected = Some(arg);
                self.show_view("explorer", false);
            }
            "heading" => {
                if let (Some(k), Ok(i)) = (self.md_key(), arg.parse()) {
                    self.reveal_heading(&k, i);
                }
            }
            "ws" => self.switch_workspace(&arg, None),
            "wsmenu" => {
                let is_cur = self.ws().id == arg;
                let (a1, a2, a3) = (arg.clone(), arg.clone(), arg.clone());
                let mut items = vec![];
                if !is_cur {
                    items.push(mi("Open Workspace", "", move |app| app.switch_workspace(&a1, None)));
                }
                items.push(mi("Rename…", "", move |app| app.rename_workspace(&a2)));
                items.push(mi_danger("Remove Workspace…", "", move |app| app.delete_workspace(&a3)));
                self.show_menu(rect[0], rect[3] + 4.0, items);
            }
            "rmroot" => self.remove_folder(&arg),
            "open" => {
                self.open_file(&arg, OpenOpts::default());
            }
            "revert" => self.revert(&arg),
            "keep" => {
                if let Some(d) = self.docs.get_mut(&arg) {
                    d.conflict = false;
                    d.missing = false;
                    self.mark(F_EDITOR | F_TABS | F_STATUS);
                }
            }
            "sopt" => {
                let o = &mut self.search;
                match arg.as_str() {
                    "case" => o.case = !o.case,
                    "word" => o.word = !o.word,
                    _ => o.regex = !o.regex,
                }
                self.run_search();
            }
            "sclear" => {
                self.search.scope = None;
                self.run_search();
            }
            "srefresh" => self.run_search(),
            "scollapse" => {
                if let Some(r) = &self.search.result {
                    self.search.collapsed = r.files.iter().map(|f| f.0.clone()).collect();
                }
                self.mark(F_SIDE);
            }
            "srtoggle" => {
                if !self.search.collapsed.remove(&arg) {
                    self.search.collapsed.insert(arg);
                }
                self.mark(F_SIDE);
            }
            "sr" => {
                let Some((i, path)) = arg.split_once(':') else { return };
                let i: usize = i.parse().unwrap_or(0);
                let Some(m) = self.search.result.as_ref().and_then(|r| r.files.iter().find(|f| f.0 == path)).and_then(|f| f.1.get(i)).cloned() else { return };
                self.search.selected = Some((path.to_string(), i));
                let open = self.docs.contains_key(path);
                self.open_file(path, OpenOpts { preview: !open, line: Some(m.line - 1), range: Some((m.start, m.end)), ..Default::default() });
                self.mark(F_SIDE);
            }
            "fopt" => {
                let f = &mut self.find;
                match arg.as_str() {
                    "case" => f.case = !f.case,
                    "word" => f.word = !f.word,
                    _ => f.regex = !f.regex,
                }
                self.find.index = None;
                self.find_refresh(false);
                self.find_show(true);
            }
            "pal" => self.choose_palette(arg.parse().unwrap_or(0)),
            "palclose" => self.close_palette(None),
            "mi" => {
                let job = arg.parse::<usize>().ok().and_then(|i| self.menu.get_mut(i)).and_then(Option::take);
                self.hide_menu();
                if let Some(j) = job {
                    j(self);
                }
            }
            "set" => {
                let cur = serde_json::to_value(&self.s.settings).ok().and_then(|v| v.get(&arg).and_then(Value::as_bool)).unwrap_or(false);
                self.set_setting(&arg, json!(!cur));
            }
            "append" => {
                if let Some(k) = self.key_of_n(&arg) {
                    self.start_live_append(&k);
                }
            }
            _ => {}
        }
    }

    fn input(&mut self, i: &str, ev: &Value) {
        let v = s(ev, "v").to_string();
        match i {
            "src" => {
                let Some(k) = self.key_of_n(&docn(ev)) else { return };
                let d = self.docs.get_mut(&k).unwrap();
                d.text = v;
                d.ta_stale = false;
                self.set_cursor(&k, u(ev, "s"), u(ev, "e"));
                self.text_changed(&k);
                self.after_edit_soon(&k);
            }
            "live" => {
                if let Some(k) = self.active.clone() {
                    self.live_input(&k, &v);
                }
            }
            "search" => {
                self.search.query = v;
                self.debounce("search", 260, |app| app.run_search());
            }
            "ofilter" => {
                self.outline_filter = v;
                self.mark(F_SIDE);
            }
            "find" => {
                self.find.query = v;
                self.find.index = None;
                self.find_refresh(false);
                self.find_show(true);
            }
            "pal" => {
                self.pal.value = v;
                self.update_palette();
            }
            "setq" => {
                self.settings_query = v;
                self.mark(F_EDITOR);
            }
            _ => {
                if let Some(key) = i.strip_prefix("set:") {
                    self.set_setting(key, json!(v));
                } else if let Some(key) = i.strip_prefix("setr:") {
                    let x: f64 = v.parse().unwrap_or(0.0);
                    let unit = if key == "lineHeight" { "" } else { "px" };
                    self.ui.html(&format!("out-{key}"), format!("{}{unit}", fmt_num(x)));
                    self.set_setting(key, json!(x));
                    self.light_settings = true;
                }
            }
        }
    }

    fn key(&mut self, id: &str, k: &str, ev: &Value) {
        match id {
            "global" => match k {
                "Escape" => {
                    if self.menu_open {
                        self.hide_menu();
                    } else if self.pal.open {
                    } else if self.find.open {
                        self.close_find();
                    } else if self.s.settings.zen {
                        self.command("zen");
                    }
                }
                "Ctrl+Tab" => self.cycle_tab(1),
                "Ctrl+Shift+Tab" => self.cycle_tab(-1),
                _ => {}
            },
            "live" if matches!(k, "Escape" | "Meta+Enter" | "Ctrl+Enter") => {
                if let Some(a) = self.active.clone() {
                    self.commit_live(&a, true);
                }
            }
            "src" | "live" => self.editor_key(s(ev, "el"), k, s(ev, "v"), u(ev, "s"), u(ev, "e")),
            "pal" => match k {
                "Escape" => self.close_palette(None),
                "Enter" => {
                    if let Some(i) = self.pal.sel {
                        self.choose_palette(i);
                    } else if matches!(self.pal.kind, PalKind::Input(..)) {
                        self.choose_palette(0);
                    }
                }
                "ArrowDown" | "Ctrl+n" => self.move_palette(1),
                "ArrowUp" | "Ctrl+p" => self.move_palette(-1),
                _ => {}
            },
            "find" => match k {
                "Enter" => self.find_step(1),
                "Shift+Enter" => self.find_step(-1),
                "Escape" => self.close_find(),
                _ => {}
            },
            "search" => match k {
                "Enter" => self.run_search(),
                "Escape" => {
                    self.search.query.clear();
                    self.ui.now(json!({"op": "val", "id": "search-input", "v": ""}));
                    self.run_search();
                }
                _ => {}
            },
            "inline" => match k {
                "Enter" => {
                    let v = s(ev, "v").trim().to_string();
                    self.finish_inline(Some(v).filter(|v| !v.is_empty()));
                }
                _ => self.finish_inline(None),
            },
            "tree" => self.tree_key(k),
            _ => {}
        }
    }

    fn tree_key(&mut self, k: &str) {
        let rows = self.visible_rows();
        let sel = self.selected.clone();
        let mut i = sel.as_ref().and_then(|s| rows.iter().position(|r| r == s));
        let node = sel.as_ref().and_then(|p| self.find_node(p)).map(|n| (n.path.clone(), n.dir));
        match k {
            "ArrowDown" => i = Some(i.map(|i| (i + 1).min(rows.len().saturating_sub(1))).unwrap_or(0)),
            "ArrowUp" => i = Some(i.map(|i| i.saturating_sub(1)).unwrap_or(0)),
            "ArrowRight" => {
                if let Some((p, true)) = node {
                    self.expanded.insert(p);
                    self.mark(F_SIDE);
                }
                return;
            }
            "ArrowLeft" => {
                if let Some((p, dir)) = node {
                    if dir && self.expanded.contains(&p) {
                        self.expanded.remove(&p);
                    } else if self.find_node(&dirname(&p)).is_some() {
                        self.selected = Some(dirname(&p));
                    }
                    self.mark(F_SIDE);
                }
                return;
            }
            "Enter" => {
                if let Some((p, dir)) = node {
                    if dir {
                        if !self.expanded.remove(&p) {
                            self.expanded.insert(p);
                        }
                        self.mark(F_SIDE);
                    } else {
                        self.rename_entry(&p);
                    }
                }
                return;
            }
            "Space" => {
                if let Some((p, false)) = node {
                    self.open_file(&p, OpenOpts { preview: true, ..Default::default() });
                }
                return;
            }
            "Meta+Backspace" => {
                if let Some((p, _)) = node {
                    if !self.ws().roots.contains(&p) {
                        self.trash_entry(&p);
                    }
                }
                return;
            }
            _ => return,
        }
        if let Some(r) = i.and_then(|i| rows.get(i)) {
            self.selected = Some(r.clone());
            self.mark(F_SIDE);
            self.ui.later(json!({"op": "scroll", "sel": "#explorer-body .row.selected"}));
        }
    }

    /* ───────────── editing keys (Enter continues lists, Tab indents) ───────────── */

    fn editor_key(&mut self, el: &str, k: &str, v: &str, s16: usize, e16: usize) {
        let (s, e) = (u16_to_byte(v, s16), u16_to_byte(v, e16));
        let (s, e) = (s.min(e), s.max(e));
        let edit = |app: &mut App, a: usize, b: usize, text: String, sel: Option<(usize, usize)>| {
            let mut op = json!({"op": "edit", "id": el, "s": byte_to_u16(v, a), "e": byte_to_u16(v, b), "text": text});
            if let Some((x, y)) = sel {
                // Selection offsets are in the edited text.
                let nv = format!("{}{}{}", &v[..a], text, &v[b..]);
                op["ss"] = json!(byte_to_u16(&nv, x));
                op["se"] = json!(byte_to_u16(&nv, y));
            }
            app.ui.now(op);
        };
        let ls = v[..s].rfind('\n').map(|i| i + 1).unwrap_or(0);
        match k {
            "Enter" => {
                if s != e {
                    return edit(self, s, e, "\n".into(), None);
                }
                let line = &v[ls..s];
                if let Some(m) = LIST_CONT.captures(line) {
                    let rest_end = v[s..].find('\n').map(|i| s + i).unwrap_or(v.len());
                    if line.trim() == m[0].trim() && v[s..rest_end].trim().is_empty() {
                        return edit(self, ls, s, String::new(), None);
                    }
                    let marker = match m.get(3) {
                        Some(num) => format!("{}{}", num.as_str().parse::<u64>().unwrap_or(0) + 1, &m[4]),
                        None => m[2].to_string(),
                    };
                    let task = if m.get(6).is_some() { "[ ] " } else { "" };
                    return edit(self, s, e, format!("\n{}{marker}{}{task}", &m[1], &m[5]), None);
                }
                let ind: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
                edit(self, s, e, format!("\n{ind}"), None)
            }
            "Tab" | "Shift+Tab" | "Meta+]" | "Meta+[" => {
                let shift = k == "Shift+Tab" || k == "Meta+[";
                let multi = v[s..e].contains('\n');
                let is_list = LIST_START.is_match(&v[ls..]);
                if !multi && !shift && !is_list {
                    return edit(self, s, e, "  ".into(), None);
                }
                let from = if e > s && v.as_bytes()[e - 1] == b'\n' { e - 1 } else { e }.max(s);
                let le = v[from..].find('\n').map(|i| from + i).unwrap_or(v.len());
                let block = &v[ls..le];
                let out: Vec<String> = block
                    .split('\n')
                    .map(|l| {
                        if shift {
                            if let Some(r) = l.strip_prefix("  ") {
                                r.to_string()
                            } else if let Some(r) = l.strip_prefix(' ').or_else(|| l.strip_prefix('\t')) {
                                r.to_string()
                            } else {
                                l.to_string()
                            }
                        } else {
                            format!("  {l}")
                        }
                    })
                    .collect();
                let out = out.join("\n");
                let sel = if multi {
                    (ls, ls + out.len())
                } else {
                    let c = (s as isize + out.len() as isize - block.len() as isize).max(ls as isize) as usize;
                    (c, c)
                };
                edit(self, ls, le, out, Some(sel))
            }
            _ => {}
        }
    }

    /// Bold / italic / link / find need the page's current selection; it arrives here.
    fn selection_command(&mut self, ev: &Value) {
        let (cmd, el, v) = (s(ev, "cmd"), s(ev, "id"), s(ev, "v"));
        let (s16, e16) = (u(ev, "s"), u(ev, "e"));
        if cmd == "find" {
            let seed = if el.is_empty() { v.to_string() } else { v[u16_to_byte(v, s16)..u16_to_byte(v, e16).max(u16_to_byte(v, s16))].to_string() };
            self.open_find(Some(seed));
            return;
        }
        if el.is_empty() || (el != "live-ta" && !el.starts_with("ta-")) {
            return;
        }
        let (s, e) = (u16_to_byte(v, s16), u16_to_byte(v, e16).max(u16_to_byte(v, s16)));
        let sel = &v[s..e];
        let u16 = |x: &str| x.encode_utf16().count();
        let mut push = |a: usize, b: usize, text: String, ss: usize, se: usize| {
            self.ui.now(json!({"op": "edit", "id": el, "s": byte_to_u16(v, a), "e": byte_to_u16(v, b), "text": text, "ss": ss, "se": se}));
        };
        match cmd {
            "bold" | "italic" => {
                let mark = if cmd == "bold" { "**" } else { "_" };
                let ml = mark.len();
                let base = byte_to_u16(v, s);
                if sel.starts_with(mark) && sel.ends_with(mark) && sel.len() >= ml * 2 {
                    let inner = &sel[ml..sel.len() - ml];
                    push(s, e, inner.into(), base, base + u16(inner));
                } else if s >= ml && v[..s].ends_with(mark) && v[e..].starts_with(mark) {
                    push(s - ml, e + ml, sel.into(), base - ml, base - ml + u16(sel));
                } else {
                    push(s, e, format!("{mark}{sel}{mark}"), base + ml, base + ml + u16(sel));
                }
            }
            "link" => {
                let text = if sel.is_empty() { "link text" } else { sel };
                let at = byte_to_u16(v, s) + u16(text) + 3;
                push(s, e, format!("[{text}](https://)"), at, at + 8);
            }
            _ => {}
        }
    }

    /* ───────────── links in the reader ───────────── */

    fn link(&mut self, key: &str, href: &str) {
        let n = self.docs[key].n;
        if let Some(a) = href.strip_prefix('#') {
            let id = folio_core::html::percent_decode(a);
            self.ui.later(json!({"op": "reveal", "doc": n, "line": 0, "anchor": id, "flash": true}));
            return;
        }
        let lower = href.to_ascii_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:") {
            fsx::open_external(href);
            return;
        }
        let Some(dir) = self.docs[key].dir() else { return };
        if lower.split_once(':').is_some_and(|(sch, _)| !sch.contains('/') && sch.chars().all(|c| c.is_ascii_alphanumeric())) {
            return;
        }
        let (file, anchor) = href.split_once('#').map(|(f, a)| (f, Some(a.to_string()))).unwrap_or((href, None));
        if file.is_empty() {
            return;
        }
        let file = folio_core::html::percent_decode(file);
        let path = if file.starts_with('/') { fsx::normalize(&file) } else { fsx::normalize(&format!("{dir}/{file}")) };
        let e = ext(&path);
        if is_markdown(&path) || e == "txt" || e.is_empty() {
            let target = if e.is_empty() { format!("{path}.md") } else { path };
            self.open_file(&target, OpenOpts { anchor, ..Default::default() });
        } else {
            fsx::reveal(&path);
        }
    }

    /* ───────────── drag & drop ───────────── */

    fn drop(&mut self, drag: &str, drop: &str, after: bool) {
        match (drag.split_once(':'), drop.split_once(':')) {
            (Some(("tree", from)), Some(("tree", dir))) => {
                if dir == dirname(from) || within(dir, from) {
                    return;
                }
                let (from, to) = (from.to_string(), format!("{}/{}", dir.trim_end_matches('/'), basename(from)));
                self.move_path(&from, &to);
            }
            (Some(("tab", key)), Some(("tab", target))) => {
                if key == target {
                    return;
                }
                let key = key.to_string();
                self.tabs.retain(|k| *k != key);
                let i = self.tabs.iter().position(|k| k == target).map(|i| i + after as usize).unwrap_or(self.tabs.len());
                self.tabs.insert(i, key);
                self.mark(F_TABS | F_SIDE);
                self.persist();
            }
            _ => {}
        }
    }

    /* ───────────── menus ───────────── */

    pub fn show_menu(&mut self, x: f64, y: f64, items: Vec<MenuItem>) {
        let mut html_items = vec![];
        self.menu.clear();
        for it in items {
            match it {
                None => {
                    html_items.push(None);
                    self.menu.push(None);
                }
                Some((l, k, d, job)) => {
                    html_items.push(Some((l, k, d)));
                    self.menu.push(Some(job));
                }
            }
        }
        self.menu_open = true;
        self.ui.later(json!({"op": "menu", "h": menu_html(&html_items), "x": x, "y": y}));
    }

    pub fn hide_menu(&mut self) {
        self.menu_open = false;
        self.ui.now(json!({"op": "menu", "hide": true}));
    }

    fn context(&mut self, c: &str, x: f64, y: f64) {
        let (verb, arg) = c.split_once(':').unwrap_or((c, ""));
        let arg = arg.to_string();
        match verb {
            "explorer" => {
                let items = vec![mi("New File…", "", |app| app.new_entry(None, false)), mi("Add Folder to Workspace…", "", |app| app.add_folder_dialog())];
                self.show_menu(x, y, items);
            }
            "row" => {
                let path = arg;
                let is_dir = self.find_node(&path).is_some_and(|n| n.dir);
                let is_root = self.ws().roots.contains(&path);
                self.selected = Some(path.clone());
                self.mark(F_SIDE);
                let dir = if is_dir { path.clone() } else { dirname(&path) };
                let rel = self.rel_path(&path);
                let mut items = vec![];
                let p = path.clone();
                if !is_dir {
                    let (p1, p2) = (p.clone(), p.clone());
                    items.push(mi("Open", "", move |app| {
                        app.open_file(&p1, OpenOpts::default());
                    }));
                    items.push(mi("Open in Split View", "", move |app| {
                        app.open_file(&p2, OpenOpts { mode: Some(Mode::Split), ..Default::default() });
                    }));
                    items.push(None);
                }
                let (d1, d2) = (dir.clone(), dir.clone());
                items.push(mi("New File…", "", move |app| app.new_entry(Some(d1), false)));
                items.push(mi("New Folder…", "", move |app| app.new_entry(Some(d2), true)));
                items.push(None);
                let (p3, p4) = (p.clone(), p.clone());
                items.push(mi("Reveal in Finder", "⌥⌘R", move |_| fsx::reveal(&p3)));
                items.push(mi("Copy Path", "", move |app| {
                    fsx::copy(&p4);
                    app.toast("Path copied", "ok");
                }));
                items.push(mi("Copy Relative Path", "", move |app| {
                    fsx::copy(&rel);
                    app.toast("Relative path copied", "ok");
                }));
                if is_dir {
                    let p5 = p.clone();
                    items.push(None);
                    items.push(mi("Find in Folder…", "", move |app| {
                        app.search.scope = Some(p5);
                        app.show_view("search", false);
                        app.run_search();
                    }));
                }
                items.push(None);
                if is_root {
                    let p6 = p.clone();
                    items.push(mi("Remove Folder from Workspace", "", move |app| app.remove_folder(&p6)));
                } else {
                    let (p7, p8) = (p.clone(), p.clone());
                    items.push(mi("Rename…", "↵", move |app| app.rename_entry(&p7)));
                    items.push(mi_danger("Move to Trash", "⌘⌫", move |app| app.trash_entry(&p8)));
                }
                self.show_menu(x, y, items);
            }
            "tab" => {
                let k = arg;
                let Some(d) = self.docs.get(&k) else { return };
                let (path, preview) = (d.path.clone(), d.preview);
                let tabs = self.tabs.clone();
                let idx = tabs.iter().position(|t| *t == k).unwrap_or(0);
                let others: Vec<String> = tabs.iter().filter(|t| **t != k).cloned().collect();
                let right: Vec<String> = tabs[idx + 1..].to_vec();
                let saved: Vec<String> = tabs.iter().filter(|t| !self.docs[*t].is_dirty()).cloned().collect();
                let k1 = k.clone();
                let mut items = vec![
                    mi("Close", "⌘W", move |app| app.close_tab(&k1, false, None)),
                    mi("Close Others", "", move |app| app.close_many(others)),
                    mi("Close to the Right", "", move |app| app.close_many(right)),
                    mi("Close Saved", "", move |app| app.close_many(saved)),
                    mi("Close All", "⌥⌘W", move |app| app.close_many(tabs)),
                    None,
                ];
                if let Some(p) = path {
                    let (p1, p2, p3) = (p.clone(), p.clone(), p);
                    items.push(mi("Copy Path", "", move |app| {
                        fsx::copy(&p1);
                        app.toast("Path copied", "ok");
                    }));
                    items.push(mi("Reveal in Finder", "⌥⌘R", move |_| fsx::reveal(&p2)));
                    items.push(mi("Reveal in Explorer", "", move |app| {
                        app.selected = Some(p3);
                        app.show_view("explorer", false);
                    }));
                }
                if preview {
                    let k2 = k.clone();
                    items.push(mi("Keep Open", "", move |app| {
                        if let Some(d) = app.docs.get_mut(&k2) {
                            d.preview = false;
                        }
                        app.mark(F_TABS);
                    }));
                }
                self.show_menu(x, y, items);
            }
            _ => {}
        }
    }

    fn more_menu(&mut self, x: f64, y: f64) {
        let Some(d) = self.doc().filter(|d| d.kind == Kind::Md) else { return };
        let has_path = d.path.is_some();
        let st = &self.s.settings;
        let check = |on: bool, l: &str| format!("{}{l}", if on { "✓ " } else { "" });
        let mut items = vec![
            mi("Find", "⌘F", |app| app.command("find")),
            mi("Go to Heading…", "⇧⌘O", |app| app.command("gotoHeading")),
            None,
            mi(&check(st.sync_scroll, "Sync Scrolling"), "", |app| app.command("toggleSync")),
            mi(&check(st.wrap, "Word Wrap"), "⌥Z", |app| app.command("wrap")),
            mi(&check(st.line_numbers, "Line Numbers"), "", |app| app.command("lineNumbers")),
            None,
            mi("Export as HTML…", "", |app| app.command("exportHtml")),
            mi("Copy as HTML", "", |app| app.command("copyHtml")),
        ];
        if has_path {
            items.push(None);
            items.push(mi("Reveal in Finder", "⌥⌘R", |app| app.command("reveal")));
            items.push(mi("Revert File", "", |app| app.command("revert")));
        }
        self.show_menu(x, y, items);
    }

    fn explorer_menu(&mut self, x: f64, y: f64) {
        let st = &self.s.settings;
        let (all, oe) = (st.show_all_files, st.open_editors);
        let items = vec![
            mi("New Folder…", "", |app| app.command("newFolder")),
            mi("Refresh Explorer", "", |app| app.command("refreshExplorer")),
            mi("Collapse Folders", "", |app| app.command("collapseFolders")),
            None,
            mi(&format!("{}Show All Files", if all { "✓ " } else { "" }), "", |app| app.command("showAllFiles")),
            mi(&format!("{}Show Open Editors", if oe { "✓ " } else { "" }), "", |app| {
                app.s.settings.open_editors = !app.s.settings.open_editors;
                app.mark(F_SIDE);
                app.persist();
            }),
        ];
        self.show_menu(x, y, items);
    }

    /* ───────────── palette ───────────── */

    pub fn open_palette(&mut self, prefix: &str) {
        self.cancel_palette_kind();
        self.pal.kind = PalKind::Normal;
        let ph = if prefix == ">" { "Type a command" } else { "Search files by name (type > for commands, @ for headings, : for a line)" };
        self.show_palette(prefix, ph, false);
    }

    fn show_palette(&mut self, value: &str, placeholder: &str, select: bool) {
        self.pal.open = true;
        self.pal.value = value.to_string();
        self.pal.placeholder = placeholder.to_string();
        self.ui.now(json!({"op": "val", "id": "pal-input", "v": value, "s": byte_to_u16(value, value.len()), "e": byte_to_u16(value, value.len())}));
        self.ui.now(json!({"op": "attr", "sel": "#pal-input", "k": "placeholder", "v": placeholder}));
        self.ui.attr("#palette", "hidden", None);
        let mut f = json!({"op": "focus", "id": "pal-input"});
        if select {
            f["all"] = json!(true);
        }
        self.ui.later(f);
        self.update_palette();
    }

    fn cancel_palette_kind(&mut self) {
        match std::mem::replace(&mut self.pal.kind, PalKind::Normal) {
            PalKind::Pick(_, Some(t)) => t(self, None),
            PalKind::Input(_, Some(t)) => t(self, None),
            _ => {}
        }
    }

    pub fn quick_pick(&mut self, items: Vec<PalItem>, placeholder: &str, then: impl FnOnce(&mut App, Option<PalAct>) + Send + 'static) {
        self.cancel_palette_kind();
        self.pal.kind = PalKind::Pick(items, Some(Box::new(then)));
        self.show_palette("", placeholder, false);
    }

    pub fn quick_input(&mut self, placeholder: &str, value: &str, prompt: &str, then: InputFn) {
        self.cancel_palette_kind();
        self.pal.kind = PalKind::Input(prompt.to_string(), Some(then));
        self.show_palette(value, placeholder, true);
    }

    pub fn close_palette(&mut self, result: Option<PalAct>) {
        if !self.pal.open {
            return;
        }
        self.pal.open = false;
        self.mark(F_PAL);
        let value = self.pal.value.clone();
        let none = result.is_none();
        match std::mem::replace(&mut self.pal.kind, PalKind::Normal) {
            PalKind::Pick(_, Some(t)) => t(self, result),
            PalKind::Input(_, Some(t)) => t(self, if none { None } else { Some(value) }),
            _ => {}
        }
        if none {
            if let Some(k) = self.md_key().filter(|k| self.docs[k].mode != Mode::Read) {
                self.focus_source(&k, None);
            }
        }
    }

    fn choose_palette(&mut self, i: usize) {
        if matches!(self.pal.kind, PalKind::Input(..)) {
            return self.close_palette(Some(PalAct::None));
        }
        let Some(x) = self.pal.items.get(i).cloned().filter(|x| !x.passive) else { return };
        if matches!(self.pal.kind, PalKind::Pick(..)) {
            return self.close_palette(Some(x.act));
        }
        self.close_palette(Some(PalAct::None));
        match x.act {
            PalAct::Cmd(id) => self.command(id),
            PalAct::Open(p) => {
                self.open_file(&p, OpenOpts::default());
            }
            PalAct::Heading(h) => {
                if let Some(k) = self.md_key() {
                    self.reveal_heading(&k, h);
                }
            }
            PalAct::Line(l) => {
                if let Some(k) = self.md_key() {
                    if self.docs[&k].mode == Mode::Read {
                        self.set_mode(&k, Mode::Split);
                    }
                    self.reveal_line(&k, l, None);
                }
            }
            _ => {}
        }
    }

    fn move_palette(&mut self, dir: isize) {
        let n = self.pal.items.len();
        if n == 0 {
            return;
        }
        let mut i = self.pal.sel.map(|x| x as isize).unwrap_or(-1);
        for _ in 0..n {
            i = (i + dir).rem_euclid(n as isize);
            if !self.pal.items[i as usize].passive {
                break;
            }
        }
        self.pal.sel = Some(i as usize);
        self.mark(F_PAL);
    }

    fn rank(list: Vec<PalItem>, q: &str, keep_order: bool) -> Vec<PalItem> {
        if q.is_empty() {
            return list;
        }
        let mut out: Vec<(f64, PalItem)> = list
            .into_iter()
            .filter_map(|mut x| {
                let (score, idx) = fuzzy(q, &x.label)?;
                x.label_html = Some(mark_idx(&x.label, &idx));
                Some((score, x))
            })
            .collect();
        if !keep_order {
            out.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        }
        out.into_iter().map(|x| x.1).collect()
    }

    pub fn update_palette(&mut self) {
        let q = self.pal.value.clone();
        let items = match &self.pal.kind {
            PalKind::Input(prompt, _) => vec![PalItem::passive(if prompt.trim().is_empty() { "Press Enter to confirm or Escape to cancel" } else { prompt.as_str() })],
            PalKind::Pick(list, _) => Self::rank(list.clone(), q.trim(), false),
            PalKind::Normal => {
                if let Some(rest) = q.strip_prefix('>') {
                    let list = COMMANDS
                        .iter()
                        .filter(|c| !c.hidden && self.command_enabled(c.id))
                        .map(|c| {
                            let mut it = PalItem::new(c.title, c.icon, PalAct::Cmd(c.id)).detail(c.group);
                            it.kbd = c.key.into();
                            it
                        })
                        .collect();
                    Self::rank(list, rest.trim(), false)
                } else if let Some(rest) = q.strip_prefix('@') {
                    match self.md_key() {
                        Some(k) => {
                            let r = self.rendered(&k);
                            if r.headings.is_empty() {
                                vec![PalItem::passive("No headings in this document")]
                            } else {
                                let list = r
                                    .headings
                                    .iter()
                                    .enumerate()
                                    .map(|(i, h)| {
                                        let mut it = PalItem::new(if h.text.is_empty() { "(untitled)" } else { &h.text }, "hash", PalAct::Heading(i)).detail(format!("H{} · line {}", h.depth, h.line + 1));
                                        it.indent = h.depth as usize - 1;
                                        it
                                    })
                                    .collect();
                                Self::rank(list, rest.trim(), true)
                            }
                        }
                        None => vec![PalItem::passive("Open a document first")],
                    }
                } else if let Some(rest) = q.strip_prefix(':') {
                    match self.md_key() {
                        Some(k) => {
                            let total = self.docs[&k].text.split('\n').count();
                            match rest.trim().parse::<usize>().ok().filter(|&x| x > 0) {
                                Some(l) => vec![PalItem::new(format!("Go to line {l}"), "hash", PalAct::Line(l.min(total) - 1))],
                                None => vec![PalItem::passive(format!("Type a line number between 1 and {total}"))],
                            }
                        }
                        None => vec![PalItem::passive("Open a document first")],
                    }
                } else {
                    self.file_items(q.trim())
                }
            }
        };
        self.pal.sel = items.iter().position(|x| !x.passive);
        self.pal.items = items;
        self.mark(F_PAL);
    }

    fn file_items(&mut self, q: &str) -> Vec<PalItem> {
        let files = self.get_files();
        let mut seen = std::collections::HashSet::new();
        let mut pool: Vec<(String, &'static str)> = vec![];
        for k in &self.tabs {
            if let Some(p) = self.docs.get(k).and_then(|d| d.path.clone()) {
                if seen.insert(p.clone()) {
                    pool.push((p, "open"));
                }
            }
        }
        for p in &self.s.recent {
            if seen.insert(p.clone()) {
                pool.push((p.clone(), "recent"));
            }
        }
        for p in files {
            if seen.insert(p.clone()) {
                pool.push((p, ""));
            }
        }
        let item = |app: &App, p: &str, tag: &'static str, html: Option<String>| {
            let mut it = PalItem::new(basename(p), "md", PalAct::Open(p.to_string())).detail(app.rel_path(&dirname(p)));
            it.tag = tag;
            it.label_html = html;
            it
        };
        let mut items: Vec<PalItem> = if q.is_empty() {
            pool.iter().take(60).map(|(p, t)| item(self, p, t, None)).collect()
        } else {
            let mut scored: Vec<(f64, &String, &str, Option<Vec<usize>>)> = vec![];
            for (p, tag) in &pool {
                let name = basename(p);
                if let Some((sc, idx)) = fuzzy(q, &name) {
                    scored.push((sc + 30.0 + if *tag == "open" { 5.0 } else { 0.0 }, p, tag, Some(idx)));
                } else if let Some((sc, _)) = fuzzy(q, &self.rel_path(p)) {
                    scored.push((sc, p, tag, None));
                }
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.into_iter().take(60).map(|(_, p, tag, idx)| item(self, p, if tag == "open" { "open" } else { "" }, idx.map(|i| mark_idx(&basename(p), &i)))).collect()
        };
        if items.is_empty() {
            items.push(PalItem::passive(if self.ws().roots.is_empty() { "Add a folder to this workspace to search its files" } else { "No matching files" }));
        }
        items
    }

    /* ───────────── commands ───────────── */

    pub fn command_enabled(&self, id: &str) -> bool {
        let md = self.doc().is_some_and(|d| d.kind == Kind::Md);
        let path = self.doc().is_some_and(|d| d.kind == Kind::Md && d.path.is_some());
        match id {
            "newFileInFolder" | "newFolder" => !self.ws().roots.is_empty(),
            "revert" | "reveal" | "copyPath" => path,
            "exportHtml" | "copyHtml" => md,
            _ => true,
        }
    }

    pub fn command(&mut self, id: &str) {
        if self.menu_open {
            self.hide_menu();
        }
        let md = self.md_key();
        match id {
            "newFile" => {
                self.new_untitled("");
            }
            "newFileInFolder" => {
                if self.ws().roots.is_empty() {
                    self.new_untitled("");
                } else {
                    let d = self.selected_dir();
                    self.new_entry(d, false);
                }
            }
            "newFolder" => {
                let d = self.selected_dir();
                self.new_entry(d, true);
            }
            "open" => self.open_file_dialog(),
            "addFolder" => self.add_folder_dialog(),
            "save" => {
                if let Some(k) = md {
                    self.save_doc(&k, false, None);
                }
            }
            "saveAs" => {
                if let Some(k) = md {
                    self.save_doc(&k, true, None);
                }
            }
            "saveAll" => self.save_all(None),
            "revert" => {
                if let Some(k) = md {
                    self.revert(&k);
                }
            }
            "reveal" => {
                if let Some(p) = self.doc().and_then(|d| d.path.clone()) {
                    fsx::reveal(&p);
                }
            }
            "copyPath" => {
                if let Some(p) = self.doc().and_then(|d| d.path.clone()) {
                    fsx::copy(&p);
                    self.toast("Path copied", "ok");
                }
            }
            "exportHtml" => self.export_html(),
            "copyHtml" => {
                if let Some(k) = md {
                    let h = self.clean_html(&k);
                    fsx::copy(&h);
                    self.toast("HTML copied", "ok");
                }
            }
            "closeTab" => {
                if let Some(k) = self.active.clone() {
                    self.close_tab(&k, false, None);
                }
            }
            "closeAll" => self.close_many(self.tabs.clone()),
            "closeOthers" => {
                let keep = self.active.clone();
                self.close_many(self.tabs.iter().filter(|k| Some(*k) != keep.as_ref()).cloned().collect());
            }
            "closeSaved" => self.close_many(self.tabs.iter().filter(|k| !self.docs[*k].is_dirty()).cloned().collect()),
            "reopenTab" => {
                if let Some((p, m)) = self.closed.pop() {
                    self.open_file(&p, OpenOpts { mode: Some(m), ..Default::default() });
                }
            }
            "nextTab" => self.cycle_tab(1),
            "prevTab" => self.cycle_tab(-1),
            "quickOpen" => self.open_palette(""),
            "palette" => self.open_palette(">"),
            "gotoHeading" => self.open_palette("@"),
            "gotoLine" => self.open_palette(":"),
            "find" | "bold" | "italic" | "link" => {
                if id == "find" && md.is_none() {
                    return;
                }
                self.ui.later(json!({"op": "sel", "cmd": id}));
            }
            "findNext" => self.find_step(1),
            "findPrev" => self.find_step(-1),
            "closeFind" => self.close_find(),
            "searchView" => self.show_view("search", false),
            "explorerView" => self.show_view("explorer", false),
            "outlineView" => self.show_view("outline", false),
            "workspacesView" => self.show_view("workspaces", false),
            "toggleSidebar" => {
                let st = &mut self.s.settings;
                st.sidebar_visible = !st.sidebar_visible;
                st.zen = false;
                self.mark(F_ALL);
                self.persist();
            }
            "read" | "split" | "source" => {
                if let Some(k) = md {
                    self.set_mode(&k, Mode::parse(id));
                }
            }
            "togglePreview" => {
                if let Some(k) = md {
                    let m = if self.docs[&k].mode == Mode::Read { Mode::Split } else { Mode::Read };
                    self.set_mode(&k, m);
                }
            }
            "cycleMode" => {
                if let Some(k) = md {
                    self.cycle_mode(&k);
                }
            }
            "zoomIn" => self.zoom(1),
            "zoomOut" => self.zoom(-1),
            "zoomReset" => self.zoom(0),
            "wrap" => self.set_setting("wrap", json!(!self.s.settings.wrap)),
            "lineNumbers" => self.set_setting("lineNumbers", json!(!self.s.settings.line_numbers)),
            "toggleSync" => self.set_setting("syncScroll", json!(!self.s.settings.sync_scroll)),
            "zen" => {
                self.s.settings.zen = !self.s.settings.zen;
                self.mark(F_LAYOUT);
                self.persist();
            }
            "theme" => {
                let t = &self.s.settings.theme;
                let next = if theme(t).dark { if t == "midnight" { "paper" } else { "light" } } else if t == "paper" { "midnight" } else { "dark" };
                self.set_setting("theme", json!(next));
            }
            "pickTheme" => {
                let before = self.s.settings.theme.clone();
                let items = THEMES.iter().map(|t| PalItem::new(t.label, if t.dark { "moon" } else { "sun" }, PalAct::Value(t.id.into())).detail(if t.id == before { "current" } else { "" })).collect();
                self.quick_pick(items, "Select color theme", |app, pick| {
                    if let Some(PalAct::Value(v)) = pick {
                        app.set_setting("theme", json!(v));
                    }
                });
            }
            "settings" => self.open_special("folio:settings", false),
            "shortcuts" => self.open_special("folio:shortcuts", false),
            "welcome" => {
                if let Some(k) = self.active.clone() {
                    self.commit_live(&k, true);
                }
                self.active = None;
                self.mark(F_ALL);
            }
            "newWorkspace" => self.create_workspace(),
            "switchWorkspace" => {
                let cur = self.ws().id.clone();
                let mut list: Vec<_> = self.s.workspaces.iter().collect();
                list.sort_by(|a, b| b.opened.partial_cmp(&a.opened).unwrap_or(std::cmp::Ordering::Equal));
                let mut items: Vec<PalItem> = list
                    .iter()
                    .map(|w| {
                        let detail = if w.id == cur { "current".into() } else { format!("{} folder{} · {}", w.roots.len(), if w.roots.len() == 1 { "" } else { "s" }, crate::view::time_ago(w.opened)) };
                        PalItem::new(w.name.clone(), "layers", PalAct::Value(w.id.clone())).detail(detail)
                    })
                    .collect();
                items.push(PalItem::new("New Workspace…", "plus", PalAct::Value("__new".into())));
                self.quick_pick(items, "Switch to workspace", |app, pick| match pick {
                    Some(PalAct::Value(v)) if v == "__new" => app.create_workspace(),
                    Some(PalAct::Value(v)) => app.switch_workspace(&v, None),
                    _ => {}
                });
            }
            "renameWorkspace" => {
                let id = self.ws().id.clone();
                self.rename_workspace(&id);
            }
            "refreshExplorer" => {
                self.file_cache = None;
                self.refresh_trees();
            }
            "collapseFolders" => {
                self.expanded.clear();
                self.mark(F_SIDE);
                self.persist();
            }
            "showAllFiles" => self.set_setting("showAllFiles", json!(!self.s.settings.show_all_files)),
            _ => {
                if let Some(n) = id.strip_prefix("tab").and_then(|x| x.parse::<usize>().ok()) {
                    let k = if n == 9 { self.tabs.last().cloned() } else { self.tabs.get(n - 1).cloned() };
                    if let Some(k) = k {
                        self.activate(&k);
                    }
                }
                let _ = commands::get(id);
            }
        }
    }

    pub fn cycle_tab(&mut self, dir: isize) {
        if self.tabs.is_empty() {
            return;
        }
        let i = self.active.as_ref().and_then(|a| self.tabs.iter().position(|k| k == a)).map(|i| i as isize).unwrap_or(0);
        let k = self.tabs[((i + dir).rem_euclid(self.tabs.len() as isize)) as usize].clone();
        self.activate(&k);
    }

    /* ───────────── self-test (`Folio --self-test`) ───────────── */

    /// One step of a FOLIO_SCRIPT run: {"op":…} page op, {"cmd":id}, {"ev":…}, {"wait":ms}, {"dump":path}.
    pub fn script_step(&mut self) {
        if self.script.is_empty() {
            if std::env::var("FOLIO_SCRIPT_QUIT").is_ok() {
                self.after(|h| crate::quit(h));
            }
            return;
        }
        let step = self.script.remove(0);
        if let Some(op) = step.get("op") {
            self.ui.later(op.clone());
        }
        if let Some(c) = step.get("cmd").and_then(Value::as_str) {
            self.command(c);
        }
        if let Some(ev) = step.get("ev") {
            self.handle(ev);
        }
        if let Some(path) = step.get("dump").and_then(Value::as_str) {
            self.test = Some(json!({"dump": path}));
            self.ui.later(json!({"op": "probe", "tag": "dump", "q": {"html": ["html", "html"]}}));
            return;
        }
        let wait = step.get("wait").and_then(Value::as_u64).unwrap_or(150);
        self.defer(wait, |app| app.script_step());
    }

    pub fn self_test_start(&mut self) {
        let key = self.new_untitled(TEST_DOC);
        self.set_mode(&key, Mode::Split);
        self.test = Some(json!({"key": key}));
        self.defer(500, |app| {
            let Some(key) = app.test.as_ref().and_then(|t| t["key"].as_str()).map(String::from) else { return };
            let n = app.docs[&key].n;
            let q = |sel: String, what: &str| json!([sel, what]);
            app.ui.later(json!({"op": "probe", "q": {
                "heading": q(format!("#art-{n} h1#safety"), "exists"),
                "frontmatter": q(format!("#art-{n} dl.frontmatter"), "exists"),
                "table": q(format!("#art-{n} .table-wrap table"), "exists"),
                "tableCellWidth": q(format!("#art-{n} .table-wrap td"), "width"),
                "highlight": q(format!("#art-{n} .hljs-keyword"), "exists"),
                "callout": q(format!("#art-{n} .callout-note .callout-title"), "exists"),
                "script": q(format!("#art-{n} script"), "exists"),
                "unsafeLink": q(format!("#art-{n} [href^=\"javascript:\"]"), "exists"),
                "sourceHighlight": q(format!("#hl-{n} .t-h1"), "exists"),
                "hlHeight": q(format!("#hl-{n}"), "height"),
                "taHeight": q(format!("#ta-{n}"), "height"),
                "dirtyTab": q(".tab.active.dirty".into(), "exists"),
                "treeRows": q("#explorer-body .tree .row".into(), "count"),
            }}));
        });
    }

    fn self_test_finish(&mut self, r: Value) {
        let Some(key) = self.test.take().and_then(|t| t["key"].as_str().map(String::from)) else { return };
        let b = |k: &str| r.get(k).and_then(Value::as_bool).unwrap_or(false);
        let num = |k: &str| r.get(k).and_then(Value::as_f64).unwrap_or(0.0);
        let mut c = serde_json::Map::new();
        for k in ["heading", "frontmatter", "table", "highlight", "callout", "sourceHighlight", "dirtyTab"] {
            c.insert(k.into(), json!(b(k)));
        }
        c.insert("tableCellWidth".into(), json!(num("tableCellWidth")));
        c.insert("noScript".into(), json!(!b("script")));
        c.insert("noUnsafeLink".into(), json!(!b("unsafeLink")));
        c.insert("overlayAligned".into(), json!(num("hlHeight") > 0.0 && (num("hlHeight") - num("taHeight")).abs() <= 2.0));
        c.insert("treeRows".into(), json!(num("treeRows")));
        let rendered = self.rendered(&key);
        c.insert("outline".into(), json!(rendered.headings.len() == 2));
        self.toggle_task(&key, 0);
        c.insert("taskToggle".into(), json!(self.docs[&key].text.contains("- [x] task one")));
        self.set_mode(&key, Mode::Read);
        self.sync_article(&key);
        let h2 = self.rendered(&key).headings.get(1).map(|h| h.line);
        if let Some(line) = h2 {
            self.start_live_edit(&key, line, None);
        }
        c.insert("liveEditorOpen".into(), json!(self.docs[&key].live.is_some()));
        self.live_input(&key, "## Second, edited live");
        self.commit_live(&key, true);
        let html = self.rendered(&key).html.clone();
        c.insert("liveEditApplied".into(), json!(self.docs[&key].text.ends_with("## Second, edited live") && html.contains("id=\"second-edited-live\"")));
        let d = self.docs.get_mut(&key).unwrap();
        d.baseline = d.text.clone();
        self.close_tab(&key, true, None);
        c.insert("workspaces".into(), json!(self.s.workspaces.len()));
        c.insert("roots".into(), json!(self.ws().roots.len()));
        c.insert("tabs".into(), json!(self.tabs.len()));
        let checks = Value::Object(c);
        if let Some(k) = self.tabs.first().cloned() {
            self.activate(&k);
        }
        self.after(move |h| crate::write_test_result(h, checks));
    }
}
