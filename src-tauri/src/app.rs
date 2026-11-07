//! Folio's application state and behaviour. The page is a projection of this struct:
//! every event mutates it, and `flush` turns what changed into patch operations.

use crate::fsx::{self, basename, byte_to_u16, dirname, is_markdown, u16_to_byte, within, Node, SearchResult};
use crate::model::{self, now_ms, uid, Doc, Kind, Live, Mode, Pane, Persisted, Settings, Workspace};
use crate::ui::{hash, Ui};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tauri::AppHandle;

pub type Job = Box<dyn FnOnce(&mut App) + Send>;
pub type Then = Box<dyn FnOnce(&mut App, bool) + Send>;
pub type InputFn = Box<dyn FnOnce(&mut App, Option<String>) + Send>;
pub type After = Box<dyn FnOnce(&AppHandle) + Send>;

pub const F_ACT: u32 = 1;
pub const F_SIDE: u32 = 2;
pub const F_TABS: u32 = 4;
pub const F_EDITOR: u32 = 8;
pub const F_CRUMBS: u32 = 16;
pub const F_STATUS: u32 = 32;
pub const F_TITLE: u32 = 64;
pub const F_LAYOUT: u32 = 128;
pub const F_FIND: u32 = 256;
pub const F_PAL: u32 = 512;
pub const F_ALL: u32 = u32::MAX;

pub struct Theme {
    pub id: &'static str,
    pub label: &'static str,
    pub dark: bool,
    pub bg: (u8, u8, u8),
}
pub const THEMES: &[Theme] = &[
    Theme { id: "dark", label: "Folio Dark", dark: true, bg: (18, 19, 24) },
    Theme { id: "midnight", label: "Midnight (OLED)", dark: true, bg: (0, 0, 0) },
    Theme { id: "light", label: "Folio Light", dark: false, bg: (244, 244, 246) },
    Theme { id: "paper", label: "Paper", dark: false, bg: (241, 236, 226) },
];
pub fn theme(id: &str) -> &'static Theme {
    THEMES.iter().find(|t| t.id == id).unwrap_or(&THEMES[0])
}

#[derive(Default)]
pub struct SearchView {
    pub query: String,
    pub case: bool,
    pub word: bool,
    pub regex: bool,
    pub scope: Option<String>,
    pub result: Option<SearchResult>,
    pub running: bool,
    pub seq: u64,
    pub collapsed: HashSet<String>,
    pub selected: Option<(String, usize)>,
}

#[derive(Default)]
pub struct Find {
    pub open: bool,
    pub query: String,
    pub case: bool,
    pub word: bool,
    pub regex: bool,
    pub error: bool,
    pub ranges: Vec<(usize, usize)>,
    pub marks: Vec<usize>,
    pub index: Option<usize>,
    pub target: Option<Pane>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PalAct {
    None,
    Cmd(&'static str),
    Open(String),
    Heading(usize),
    Line(usize),
    Value(String),
}

#[derive(Clone, Debug)]
pub struct PalItem {
    pub label: String,
    pub label_html: Option<String>,
    pub detail: String,
    pub icon: &'static str,
    pub kbd: String,
    pub tag: &'static str,
    pub indent: usize,
    pub passive: bool,
    pub act: PalAct,
}

impl PalItem {
    pub fn new(label: impl Into<String>, icon: &'static str, act: PalAct) -> Self {
        PalItem { label: label.into(), label_html: None, detail: String::new(), icon, kbd: String::new(), tag: "", indent: 0, passive: false, act }
    }
    pub fn passive(label: impl Into<String>) -> Self {
        PalItem { passive: true, ..PalItem::new(label, "info", PalAct::None) }
    }
    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = d.into();
        self
    }
}

pub enum PalKind {
    Normal,
    Pick(Vec<PalItem>, Option<Box<dyn FnOnce(&mut App, Option<PalAct>) + Send>>),
    Input(String, Option<InputFn>),
}

pub struct Pal {
    pub open: bool,
    pub value: String,
    pub placeholder: String,
    pub items: Vec<PalItem>,
    pub sel: Option<usize>,
    pub kind: PalKind,
}

pub struct Inline {
    pub anchor: String,
    pub value: String,
    pub dir: bool,
    pub renaming: bool,
    pub then: Option<InputFn>,
}

pub struct App {
    pub handle: Option<AppHandle>,
    pub s: Persisted,
    pub docs: HashMap<String, Doc>,
    pub tabs: Vec<String>,
    pub active: Option<String>,
    pub closed: Vec<(String, Mode)>,
    pub trees: HashMap<String, Node>,
    pub tree_seq: u64,
    pub expanded: HashSet<String>,
    pub selected: Option<String>,
    pub file_cache: Option<Vec<String>>,
    pub home: String,
    pub next_n: u32,
    pub untitled_count: u32,
    pub search: SearchView,
    pub outline_filter: String,
    pub settings_query: String,
    pub find: Find,
    pub pal: Pal,
    pub menu: Vec<Option<Job>>,
    pub menu_open: bool,
    pub inline: Option<Inline>,
    pub ui: Ui,
    pub dirty: u32,
    pub after: Vec<After>,
    pub booted: bool,
    pub testing: bool,
    pub pending_paths: Vec<String>,
    pub allowed: Arc<Mutex<HashSet<String>>>,
    pub gen: HashMap<&'static str, u64>,
    pub title: String,
    pub fullscreen: bool,
    pub light_settings: bool,
    pub test: Option<Value>,
    pub script: Vec<Value>,
}

impl App {
    pub fn new(allowed: Arc<Mutex<HashSet<String>>>) -> Self {
        App {
            handle: None,
            s: Persisted::default(),
            docs: HashMap::new(),
            tabs: vec![],
            active: None,
            closed: vec![],
            trees: HashMap::new(),
            tree_seq: 0,
            expanded: HashSet::new(),
            selected: None,
            file_cache: None,
            home: model::home(),
            next_n: 1,
            untitled_count: 0,
            search: SearchView::default(),
            outline_filter: String::new(),
            settings_query: String::new(),
            find: Find::default(),
            pal: Pal { open: false, value: String::new(), placeholder: String::new(), items: vec![], sel: None, kind: PalKind::Normal },
            menu: vec![],
            menu_open: false,
            inline: None,
            ui: Ui::default(),
            dirty: 0,
            after: vec![],
            booted: false,
            testing: false,
            pending_paths: vec![],
            allowed,
            gen: HashMap::new(),
            title: String::new(),
            fullscreen: false,
            light_settings: false,
            test: None,
            script: vec![],
        }
    }

    /* ───────────── plumbing ───────────── */

    pub fn mark(&mut self, f: u32) {
        self.dirty |= f;
    }
    pub fn set(&self) -> &Settings {
        &self.s.settings
    }
    pub fn after(&mut self, f: impl FnOnce(&AppHandle) + Send + 'static) {
        self.after.push(Box::new(f));
    }
    /// Run `job` after `ms` milliseconds (on a worker thread, then back under the lock).
    pub fn defer(&mut self, ms: u64, job: impl FnOnce(&mut App) + Send + 'static) {
        crate::timer(ms, Box::new(job));
    }
    /// Debounce: only the latest call for `key` runs.
    pub fn debounce(&mut self, key: &'static str, ms: u64, job: impl FnOnce(&mut App) + Send + 'static) {
        let g = self.gen.entry(key).or_insert(0);
        *g += 1;
        let mine = *g;
        self.defer(ms, move |app| {
            if app.gen.get(key) == Some(&mine) {
                job(app);
            }
        });
    }
    /// Run blocking work off the UI thread, then apply its result.
    pub fn background<T: Send + 'static>(&mut self, work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(&mut App, T) + Send + 'static) {
        self.after(move |h| {
            let h = h.clone();
            std::thread::spawn(move || {
                let r = work();
                crate::with_app(&h, move |app| done(app, r));
            });
        });
    }
    pub fn toast(&mut self, msg: &str, kind: &str) {
        let ic = match kind {
            "error" => "warning",
            "ok" => "check",
            _ => "info",
        };
        self.ui.later(json!({"op": "toast", "kind": kind, "h": format!("{}<span>{}</span>", folio_core::icon(ic, ""), msg)}));
    }
    pub fn allow(&self, dir: &str) {
        self.allowed.lock().unwrap().insert(dir.to_string());
    }

    pub fn ws(&self) -> &Workspace {
        let cur = self.s.current.as_deref();
        self.s.workspaces.iter().find(|w| Some(w.id.as_str()) == cur).unwrap_or(&self.s.workspaces[0])
    }
    pub fn ws_mut(&mut self) -> &mut Workspace {
        let cur = self.s.current.clone();
        let i = self.s.workspaces.iter().position(|w| Some(&w.id) == cur.as_ref()).unwrap_or(0);
        &mut self.s.workspaces[i]
    }
    pub fn doc(&self) -> Option<&Doc> {
        self.active.as_ref().and_then(|k| self.docs.get(k))
    }
    pub fn md_key(&self) -> Option<String> {
        self.doc().filter(|d| d.kind == Kind::Md).map(|d| d.key.clone())
    }
    pub fn dirty_keys(&self) -> Vec<String> {
        self.tabs.iter().filter(|k| self.docs.get(*k).is_some_and(Doc::is_dirty)).cloned().collect()
    }
    pub fn key_of_n(&self, n: &str) -> Option<String> {
        let n: u32 = n.parse().ok()?;
        self.docs.values().find(|d| d.n == n).map(|d| d.key.clone())
    }
    pub fn tilde(&self, p: &str) -> String {
        fsx::tilde(p, &self.home)
    }
    pub fn root_of(&self, p: &str) -> Option<String> {
        self.ws().roots.iter().filter(|r| within(p, r)).max_by_key(|r| r.len()).cloned()
    }
    pub fn rel_path(&self, p: &str) -> String {
        match self.root_of(p) {
            Some(r) if p == r => basename(&r),
            Some(r) => format!("{}{}", if self.ws().roots.len() > 1 { format!("{}/", basename(&r)) } else { String::new() }, &p[r.len() + 1..]),
            None => self.tilde(p),
        }
    }

    /// The rendered form of a document, cached by text.
    pub fn rendered(&mut self, key: &str) -> Arc<folio_core::Rendered> {
        let remote = self.s.settings.remote_images;
        let d = self.docs.get_mut(key).unwrap();
        let h = hash(&d.text);
        if let Some((ch, cr, r)) = &d.cache {
            if *ch == h && *cr == remote {
                return r.clone();
            }
        }
        let dir = d.dir();
        let r = Arc::new(folio_core::render(&d.text, &folio_core::Options { base_dir: dir.as_deref(), remote_images: remote }));
        d.cache = Some((h, remote, r.clone()));
        r
    }

    pub fn persist(&mut self) {
        self.debounce("persist", 500, |app| app.persist_now());
    }
    pub fn persist_now(&mut self) {
        if self.s.workspaces.is_empty() {
            return;
        }
        let tabs: Vec<model::TabRef> = self
            .tabs
            .iter()
            .filter_map(|k| self.docs.get(k))
            .filter(|d| d.path.is_some() || d.kind != Kind::Md)
            .map(|d| model::TabRef { key: d.key.clone(), path: d.path.clone(), mode: d.mode.as_str().into() })
            .collect();
        let active = self.active.clone();
        let expanded: Vec<String> = self.expanded.iter().cloned().collect();
        let w = self.ws_mut();
        w.tabs = tabs;
        w.active = active;
        w.expanded = expanded;
        w.opened = now_ms();
        if !self.testing || std::env::var("FOLIO_STATE_FILE").is_ok() {
            model::save(&self.s);
        }
    }

    /* ───────────── boot ───────────── */

    pub fn boot(&mut self, paths: Vec<String>) {
        if let Some(saved) = model::load() {
            self.s = saved;
        } else {
            let w = Workspace::new("My Workspace", vec![]);
            self.s.current = Some(w.id.clone());
            self.s.workspaces = vec![w];
        }
        if self.s.current.as_ref().is_none_or(|c| !self.s.workspaces.iter().any(|w| &w.id == c)) {
            self.s.current = Some(self.s.workspaces[0].id.clone());
        }
        if theme(&self.s.settings.theme).id != self.s.settings.theme {
            self.s.settings.theme = "dark".into();
        }
        self.s.settings.zen = false;
        self.booted = true;
        self.load_workspace();
        self.open_paths(paths);
        self.ui.html("copied-tpl", format!("{}<span>Copied</span>", folio_core::icon("check", "")));
        self.ui.later(json!({"op": "ready"}));
        self.mark(F_ALL);
    }

    /// The page reloaded: rebuild its DOM from our state.
    pub fn rehydrate(&mut self) {
        self.ui.reset();
        let keys: Vec<String> = self.docs.keys().cloned().collect();
        for k in keys {
            let d = self.docs.get_mut(&k).unwrap();
            d.live = None;
            d.preview_stale = true;
            self.create_dom(&k);
        }
        self.ui.html("copied-tpl", format!("{}<span>Copied</span>", folio_core::icon("check", "")));
        self.ui.later(json!({"op": "ready"}));
        self.mark(F_ALL);
    }

    pub fn load_workspace(&mut self) {
        let w = self.ws().clone();
        self.expanded = if w.expanded.is_empty() { w.roots.iter().cloned().collect() } else { w.expanded.iter().cloned().collect() };
        self.trees.clear();
        self.file_cache = None;
        for r in &w.roots {
            self.allow(r);
        }
        self.watch_roots();
        self.refresh_trees();
        if self.s.settings.restore_tabs {
            for t in &w.tabs {
                if t.key.starts_with("folio:") {
                    self.open_special(&t.key, true);
                } else if let Some(p) = &t.path {
                    self.open_file(p, OpenOpts { background: true, mode: Some(Mode::parse(&t.mode)), ..Default::default() });
                }
            }
        }
        let active = w.active.filter(|a| self.tabs.contains(a)).or_else(|| self.tabs.first().cloned());
        match active {
            Some(k) => self.activate(&k),
            None => {
                self.active = None;
                self.mark(F_ALL);
            }
        }
    }

    pub fn watch_roots(&mut self) {
        let roots = self.ws().roots.clone();
        self.after(move |h| crate::watch(h, roots));
    }

    pub fn refresh_trees(&mut self) {
        let roots = self.ws().roots.clone();
        let all = self.s.settings.show_all_files;
        self.tree_seq += 1;
        let seq = self.tree_seq;
        self.background(
            move || roots.iter().map(|r| (r.clone(), fsx::tree(r, all))).collect::<Vec<_>>(),
            move |app, trees| {
                if app.tree_seq != seq {
                    return;
                }
                app.trees = trees.into_iter().collect();
                app.mark(F_SIDE);
            },
        );
    }

    pub fn find_node(&self, path: &str) -> Option<&Node> {
        self.trees.values().find_map(|t| fsx::find_node(t, path))
    }

    pub fn get_files(&mut self) -> Vec<String> {
        if self.file_cache.is_none() {
            let roots = self.ws().roots.clone();
            self.file_cache = Some(if roots.is_empty() { vec![] } else { fsx::markdown_files(&roots) });
        }
        self.file_cache.clone().unwrap()
    }

    /* ───────────── documents ───────────── */

    fn new_doc(&mut self, key: String, kind: Kind, path: Option<String>, name: String, text: String, baseline: String, mode: Mode) -> &mut Doc {
        let n = self.next_n;
        self.next_n += 1;
        let d = Doc {
            key: key.clone(),
            kind,
            path,
            name,
            text,
            baseline,
            mtime: 0.0,
            mode,
            preview: false,
            conflict: false,
            missing: false,
            n,
            live: None,
            cursor: (0, 1, 0),
            top_line: 0.0,
            cur_heading: None,
            last_focus: Pane::Source,
            ta_stale: false,
            preview_stale: true,
            cache: None,
        };
        self.docs.insert(key.clone(), d);
        self.create_dom(&key);
        self.docs.get_mut(&key).unwrap()
    }

    fn create_dom(&mut self, key: &str) {
        let d = &self.docs[key];
        let h = crate::view::doc_shell(d);
        self.ui.now(json!({"op": "docnew", "h": h}));
        if d.kind == Kind::Md {
            self.ui.now(json!({"op": "val", "id": format!("ta-{}", d.n), "v": d.text, "s": 0, "e": 0}));
        }
    }

    fn destroy_doc(&mut self, key: &str) {
        if let Some(d) = self.docs.remove(key) {
            self.ui.now(json!({"op": "rm", "sel": format!("#doc-{}", d.n)}));
            for p in ["hl-", "art-", "fl-", "ban-", "sp-"] {
                self.ui.forget(&format!("{p}{}", d.n));
            }
        }
    }

    fn insert_tab(&mut self, key: &str) {
        let i = self.active.as_ref().and_then(|a| self.tabs.iter().position(|k| k == a));
        match i {
            Some(i) => self.tabs.insert(i + 1, key.to_string()),
            None => self.tabs.push(key.to_string()),
        }
    }

    pub fn add_recent(&mut self, p: &str) {
        self.s.recent.retain(|x| x != p);
        self.s.recent.insert(0, p.to_string());
        self.s.recent.truncate(30);
    }

    pub fn open_file(&mut self, path: &str, o: OpenOpts) -> Option<String> {
        if !self.docs.contains_key(path) {
            let (text, mtime) = match fsx::read(path) {
                Ok(r) => r,
                Err(e) => {
                    self.toast(&format!("Couldn’t open {} — {}", folio_core::esc(&basename(path)), folio_core::esc(&e)), "error");
                    return None;
                }
            };
            self.allow(&dirname(path));
            let mode = o.mode.unwrap_or(if is_markdown(path) { Mode::parse(&self.s.settings.default_mode) } else { Mode::Source });
            let preview = o.preview && self.s.settings.preview_tabs;
            let d = self.new_doc(path.into(), Kind::Md, Some(path.into()), basename(path), text.clone(), text, mode);
            d.mtime = mtime;
            d.preview = preview;
            let replace = if preview {
                self.tabs.iter().find(|k| *k != path && self.docs.get(*k).is_some_and(|x| x.preview && !x.is_dirty())).cloned()
            } else {
                None
            };
            match replace {
                Some(pv) => {
                    let i = self.tabs.iter().position(|k| *k == pv).unwrap();
                    self.tabs[i] = path.into();
                    self.destroy_doc(&pv);
                }
                None => self.insert_tab(path),
            }
            self.add_recent(path);
        } else if !o.preview {
            self.docs.get_mut(path).unwrap().preview = false;
        }
        if !o.background {
            self.activate(path);
        }
        if let Some(line) = o.line {
            self.reveal_line(path, line, o.range);
        } else if let Some(a) = o.anchor {
            self.reveal_anchor(path, &a);
        }
        self.mark(F_TABS | F_SIDE);
        self.persist();
        Some(path.to_string())
    }

    pub fn new_untitled(&mut self, text: &str) -> String {
        self.untitled_count += 1;
        let key = format!("untitled:{}", uid());
        let name = format!("Untitled-{}.md", self.untitled_count);
        let mode = Mode::parse(&self.s.settings.default_mode);
        self.new_doc(key.clone(), Kind::Md, None, name, text.into(), String::new(), mode);
        self.insert_tab(&key);
        self.activate(&key);
        if mode == Mode::Read {
            self.start_live_append(&key);
        } else {
            self.focus_source(&key, None);
        }
        key
    }

    pub fn open_special(&mut self, key: &str, background: bool) {
        if !self.docs.contains_key(key) {
            let kind = if key == "folio:settings" { Kind::Settings } else { Kind::Shortcuts };
            let name = if kind == Kind::Settings { "Settings" } else { "Keyboard Shortcuts" };
            self.new_doc(key.into(), kind, None, name.into(), String::new(), String::new(), Mode::Read);
            self.insert_tab(key);
        }
        if !background {
            self.activate(key);
        }
        self.mark(F_TABS | F_SIDE);
        self.persist();
    }

    pub fn activate(&mut self, key: &str) {
        if !self.docs.contains_key(key) {
            return;
        }
        if let Some(prev) = self.active.clone() {
            if prev != key {
                self.commit_live(&prev, true);
            }
        }
        self.active = Some(key.to_string());
        if let Some(p) = self.docs[key].path.clone() {
            for r in self.ws().roots.clone() {
                if within(&p, &r) {
                    let mut d = dirname(&p);
                    while within(&d, &r) {
                        self.expanded.insert(d.clone());
                        if d == r {
                            break;
                        }
                        d = dirname(&d);
                    }
                }
            }
            self.selected = Some(p);
        }
        self.docs.get_mut(key).unwrap().preview_stale = true;
        if self.find.open {
            self.find_refresh(true);
        }
        self.mark(F_ALL);
        self.persist();
    }

    /// Close a tab, asking about unsaved changes. `then(true)` once it is closed.
    pub fn close_tab(&mut self, key: &str, force: bool, then: Option<Then>) {
        let Some(d) = self.docs.get(key) else {
            if let Some(t) = then {
                t(self, true);
            }
            return;
        };
        let (dirty, name, path, mode) = (d.is_dirty(), d.name.clone(), d.path.clone(), d.mode);
        if !force && dirty {
            let k = key.to_string();
            self.confirm(
                &format!("Do you want to save the changes you made to {name}?"),
                "Your changes will be lost if you don’t save them.",
                &["Save", "Cancel", "Don’t Save"],
                move |app, r| match r {
                    0 => {
                        let k2 = k.clone();
                        app.save_doc(&k, false, Some(Box::new(move |app, ok| {
                            if ok {
                                app.close_tab(&k2, true, then);
                            } else if let Some(t) = then {
                                t(app, false);
                            }
                        })));
                    }
                    2 => app.close_tab(&k, true, then),
                    _ => {
                        if let Some(t) = then {
                            t(app, false);
                        }
                    }
                },
            );
            return;
        }
        self.commit_live(key, false);
        let i = self.tabs.iter().position(|k| k == key).unwrap_or(0);
        if let Some(p) = path {
            self.closed.push((p, mode));
        }
        self.tabs.retain(|k| k != key);
        self.destroy_doc(key);
        if self.active.as_deref() == Some(key) {
            self.active = if self.tabs.is_empty() { None } else { Some(self.tabs[i.min(self.tabs.len() - 1)].clone()) };
        }
        match self.active.clone() {
            Some(a) => self.activate(&a),
            None => self.mark(F_ALL),
        }
        self.persist();
        if let Some(t) = then {
            t(self, true);
        }
    }

    pub fn close_many(&mut self, mut keys: Vec<String>) {
        if keys.is_empty() {
            return;
        }
        let first = keys.remove(0);
        self.close_tab(&first, false, Some(Box::new(move |app, ok| {
            if ok {
                app.close_many(keys);
            }
        })));
    }

    /* ───────────── saving ───────────── */

    pub fn save_doc(&mut self, key: &str, save_as: bool, then: Option<Then>) {
        let Some(d) = self.docs.get(key).filter(|d| d.kind == Kind::Md) else {
            if let Some(t) = then {
                t(self, false);
            }
            return;
        };
        match d.path.clone().filter(|_| !save_as) {
            Some(p) => {
                let ok = self.write_doc(key, &p);
                if let Some(t) = then {
                    t(self, ok);
                }
            }
            None => {
                let name = d.path.as_deref().map(basename).unwrap_or_else(|| d.name.clone());
                let dir = d.dir().or_else(|| self.ws().roots.first().cloned()).unwrap_or_default();
                let k = key.to_string();
                self.save_dialog(&name, &dir, move |app, path| match path {
                    Some(p) => {
                        let ok = app.write_doc(&k, &p);
                        if let Some(t) = then {
                            t(app, ok);
                        }
                    }
                    None => {
                        if let Some(t) = then {
                            t(app, false);
                        }
                    }
                });
            }
        }
    }

    fn write_doc(&mut self, key: &str, path: &str) -> bool {
        self.commit_live(key, true);
        let text = self.docs[key].text.clone();
        match fsx::write(path, &text) {
            Ok(mtime) => {
                let mut key = key.to_string();
                if path != key {
                    if self.docs.contains_key(path) {
                        let p = path.to_string();
                        self.docs.get_mut(&p).unwrap().baseline = self.docs[&p].text.clone();
                        self.close_tab(&p, true, None);
                    }
                    let mut d = self.docs.remove(&key).unwrap();
                    d.key = path.into();
                    d.path = Some(path.into());
                    d.name = basename(path);
                    d.cache = None;
                    d.preview_stale = true;
                    self.docs.insert(path.into(), d);
                    for t in self.tabs.iter_mut() {
                        if *t == key {
                            *t = path.into();
                        }
                    }
                    if self.active.as_deref() == Some(&key) {
                        self.active = Some(path.into());
                    }
                    self.allow(&dirname(path));
                    self.add_recent(path);
                    self.file_cache = None;
                    self.refresh_trees();
                    key = path.into();
                }
                let d = self.docs.get_mut(&key).unwrap();
                d.baseline = d.text.clone();
                d.mtime = mtime;
                d.preview = false;
                d.conflict = false;
                self.mark(F_ALL);
                self.persist();
                true
            }
            Err(e) => {
                let name = self.docs[key].name.clone();
                self.toast(&format!("Couldn’t save {} — {}", folio_core::esc(&name), folio_core::esc(&e)), "error");
                false
            }
        }
    }

    pub fn save_all(&mut self, then: Option<Then>) {
        let mut list = self.dirty_keys();
        if list.is_empty() {
            if let Some(t) = then {
                t(self, true);
            }
            return;
        }
        let first = list.remove(0);
        self.save_doc(&first, false, Some(Box::new(move |app, ok| {
            if !ok {
                if let Some(t) = then {
                    t(app, false);
                }
                return;
            }
            let _ = list;
            app.save_all(then);
        })));
    }

    /// Ask what to do with unsaved docs before `verb`. `then(true)` means go ahead.
    pub fn resolve_dirty(&mut self, verb: &str, then: Then) {
        let list = self.dirty_keys();
        if list.is_empty() {
            return then(self, true);
        }
        let names: Vec<String> = list.iter().map(|k| self.docs[k].name.clone()).collect();
        let msg = if list.len() == 1 { format!("Save changes to {} before {verb}?", names[0]) } else { format!("Save changes to {} files before {verb}?", list.len()) };
        self.confirm(&msg, &names.join(", "), &["Save All", "Cancel", "Don’t Save"], move |app, r| match r {
            0 => app.save_all(Some(then)),
            2 => then(app, true),
            _ => then(app, false),
        });
    }

    pub fn revert(&mut self, key: &str) {
        let Some(p) = self.docs.get(key).and_then(|d| d.path.clone()) else { return };
        match fsx::read(&p) {
            Ok((text, mtime)) => {
                self.commit_live(key, false);
                let d = self.docs.get_mut(key).unwrap();
                d.mtime = mtime;
                d.conflict = false;
                self.set_text(key, text, true);
            }
            Err(e) => self.toast(&folio_core::esc(&e), "error"),
        }
    }

    /// Replace a document's text from Rust's side (the page textarea follows).
    pub fn set_text(&mut self, key: &str, text: String, as_baseline: bool) {
        let d = self.docs.get_mut(key).unwrap();
        d.text = text;
        if as_baseline {
            d.baseline = d.text.clone();
        }
        d.ta_stale = true;
        d.preview_stale = true;
        self.text_changed(key);
        self.mark(F_EDITOR | F_STATUS | F_CRUMBS | F_SIDE);
    }

    pub fn text_changed(&mut self, key: &str) {
        let d = self.docs.get_mut(key).unwrap();
        if d.preview && d.is_dirty() {
            d.preview = false;
        }
        self.mark(F_TABS | F_TITLE | F_ACT | F_EDITOR);
        if self.find.open && self.active.as_deref() == Some(key) {
            self.debounce("find", 150, |app| {
                app.find_refresh(true);
                app.mark(F_FIND);
            });
        }
    }

    /* ───────────── live editing ───────────── */

    pub fn start_live_edit(&mut self, key: &str, line: usize, hint: Option<String>) {
        let Some(d) = self.docs.get(key) else { return };
        if d.mode != Mode::Read {
            return;
        }
        let mut line = line;
        if let Some(l) = d.live.clone() {
            let shift = if l.append { 0 } else { l.end as isize - l.orig_end as isize };
            let after = line > l.start;
            self.commit_live(key, true);
            if after {
                line = (line as isize + shift).max(0) as usize;
            }
        }
        let r = self.rendered(key);
        let Some(i) = r.blocks.iter().position(|&b| b == line) else { return };
        let text = &self.docs[key].text;
        let lines: Vec<&str> = text.split('\n').collect();
        let start = line;
        let mut end = r.blocks.get(i + 1).copied().unwrap_or(lines.len()).min(lines.len());
        while end > start + 1 && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        let src = lines[start..end].join("\n");
        let caret = caret_from_hint(&src, hint.as_deref());
        self.open_live(key, start, end, src, caret, false);
    }

    pub fn start_live_append(&mut self, key: &str) {
        let Some(d) = self.docs.get(key) else { return };
        if d.kind != Kind::Md || d.mode != Mode::Read {
            return;
        }
        if d.live.as_ref().is_some_and(|l| l.append) {
            self.ui.later(json!({"op": "focus", "id": "live-ta", "still": true}));
            return;
        }
        if d.live.is_some() {
            self.commit_live(key, true);
        } else {
            self.sync_article(key);
        }
        self.open_live(key, 0, 0, String::new(), 0, true);
    }

    fn open_live(&mut self, key: &str, start: usize, end: usize, src: String, caret: usize, append: bool) {
        let d = self.docs.get_mut(key).unwrap();
        d.live = Some(Live { start, end, orig_end: end, append, base: d.text.trim_end().to_string(), original: d.text.clone() });
        let n = d.n;
        self.ui.later(json!({"op": "live", "doc": n, "line": start, "append": append, "h": crate::view::live_editor(&src, append), "caret": byte_to_u16(&src, caret)}));
        self.ui.forget(&format!("art-{n}"));
        self.ui.attr("body", "data-live", Some("1"));
    }

    pub fn live_input(&mut self, key: &str, v: &str) {
        let Some(d) = self.docs.get_mut(key) else { return };
        let Some(l) = d.live.as_mut() else { return };
        if l.append {
            d.text = if v.trim().is_empty() { l.original.clone() } else { format!("{}{}{}\n", l.base, if l.base.is_empty() { "" } else { "\n\n" }, v) };
        } else {
            let mut lines: Vec<String> = d.text.split('\n').map(String::from).collect();
            let repl: Vec<String> = v.split('\n').map(String::from).collect();
            let end = l.end.min(lines.len());
            let start = l.start.min(end);
            let n = repl.len();
            lines.splice(start..end, repl);
            l.end = start + n;
            d.text = lines.join("\n");
        }
        d.ta_stale = true;
        self.text_changed(key);
        self.after_edit_soon(key);
    }

    /// Close the live editor, folding its text back into the rendered document.
    pub fn commit_live(&mut self, key: &str, render: bool) {
        let Some(d) = self.docs.get_mut(key) else { return };
        if d.live.take().is_none() {
            return;
        }
        d.preview_stale = true;
        self.ui.attr("body", "data-live", None);
        if render && d.mode != Mode::Source {
            self.sync_article(key);
            self.mark(F_SIDE | F_CRUMBS | F_STATUS);
        }
    }

    pub fn after_edit_soon(&mut self, key: &str) {
        let k = key.to_string();
        self.debounce("edit", 140, move |app| {
            if let Some(d) = app.docs.get_mut(&k) {
                if d.live.is_none() {
                    d.preview_stale = true;
                }
                app.mark(F_EDITOR | F_SIDE | F_STATUS | F_CRUMBS);
            }
        });
    }

    pub fn toggle_task(&mut self, key: &str, n: usize) {
        let Some(t) = self.docs.get(key).and_then(|d| folio_core::toggle_task(&d.text, n)) else { return };
        self.set_text(key, t, false);
    }

    /* ───────────── modes, scrolling, revealing ───────────── */

    pub fn set_mode(&mut self, key: &str, mode: Mode) {
        let Some(d) = self.docs.get(key) else { return };
        if d.kind != Kind::Md || d.mode == mode {
            return;
        }
        self.commit_live(key, false);
        let d = self.docs.get_mut(key).unwrap();
        let prev = d.mode;
        d.mode = mode;
        d.preview_stale = true;
        let (n, top) = (d.n, d.top_line);
        self.mark(F_EDITOR | F_TABS | F_STATUS | F_TITLE);
        // Keep the reading position across the switch.
        self.ui.later(json!({"op": "reveal", "doc": n, "line": top, "top": true, "focus": prev == Mode::Read && mode != Mode::Read}));
        if self.find.open {
            self.find_refresh(true);
        }
        self.persist();
    }

    pub fn cycle_mode(&mut self, key: &str) {
        let m = match self.docs[key].mode {
            Mode::Read => Mode::Split,
            Mode::Split => Mode::Source,
            Mode::Source => Mode::Read,
        };
        self.set_mode(key, m);
    }

    pub fn focus_source(&mut self, key: &str, at: Option<usize>) {
        let d = &self.docs[key];
        let mut op = json!({"op": "focus", "id": format!("ta-{}", d.n), "still": true});
        if let Some(b) = at {
            let u = byte_to_u16(&d.text, b);
            op["s"] = json!(u);
            op["e"] = json!(u);
        }
        self.ui.later(op);
    }

    pub fn reveal_line(&mut self, key: &str, line: usize, range: Option<(usize, usize)>) {
        let d = &self.docs[key];
        let mut op = json!({"op": "reveal", "doc": d.n, "line": line, "flash": true});
        if d.mode != Mode::Read {
            let off: usize = d.text.split('\n').take(line).map(|l| l.len() + 1).sum::<usize>().min(d.text.len());
            let (s, e) = range.map(|(a, b)| (off + a, off + b)).unwrap_or((off, off));
            op["s"] = json!(byte_to_u16(&d.text, s));
            op["e"] = json!(byte_to_u16(&d.text, e.min(d.text.len())));
        }
        self.ui.later(op);
    }

    pub fn reveal_heading(&mut self, key: &str, i: usize) {
        let r = self.rendered(key);
        let Some(h) = r.headings.get(i) else { return };
        let d = &self.docs[key];
        self.ui.later(json!({"op": "reveal", "doc": d.n, "line": h.line, "anchor": if d.mode == Mode::Source { Value::Null } else { json!(h.id) }, "flash": true}));
    }

    pub fn reveal_anchor(&mut self, key: &str, anchor: &str) {
        let r = self.rendered(key);
        let slug = folio_core::slug(anchor);
        if let Some(i) = r.headings.iter().position(|h| h.id == anchor || h.id == slug) {
            self.reveal_heading(key, i);
        }
    }

    pub fn track_heading(&mut self, key: &str, line: f64) {
        let r = self.rendered(key);
        let d = self.docs.get_mut(key).unwrap();
        d.top_line = line;
        let cur = r.headings.iter().rposition(|h| (h.line as f64) <= line + 0.5);
        if cur != d.cur_heading {
            d.cur_heading = cur;
            self.mark(F_CRUMBS | F_SIDE);
            self.ui.later(json!({"op": "scroll", "sel": "#outline-body .row.current"}));
        }
    }

    pub fn set_cursor(&mut self, key: &str, s16: usize, e16: usize) {
        let Some(d) = self.docs.get_mut(key) else { return };
        let s = u16_to_byte(&d.text, s16);
        let e = u16_to_byte(&d.text, e16);
        let line = d.text[..s].matches('\n').count();
        let ls = d.text[..s].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = d.text[ls..s].chars().count() + 1;
        let sel = d.text[s.min(e)..e.max(s)].chars().count();
        let changed_line = d.cursor.0 != line;
        d.cursor = (line, col, sel);
        let n = d.n;
        self.ui.later(json!({"op": "nth", "id": format!("hl-{n}"), "n": line, "c": "cur", "doc": n, "vis": true}));
        let _ = changed_line;
        self.mark(F_STATUS);
    }

    /* ───────────── workspaces ───────────── */

    pub fn switch_workspace(&mut self, id: &str, then: Option<Then>) {
        if self.s.current.as_deref() == Some(id) {
            return;
        }
        let id = id.to_string();
        self.resolve_dirty(
            "switching workspaces",
            Box::new(move |app, ok| {
                if !ok {
                    if let Some(t) = then {
                        t(app, false);
                    }
                    return;
                }
                app.persist_now();
                for k in app.tabs.clone() {
                    app.destroy_doc(&k);
                }
                app.tabs.clear();
                app.active = None;
                app.closed.clear();
                app.s.current = Some(id);
                app.load_workspace();
                app.persist();
                let name = folio_core::esc(&app.ws().name);
                app.toast(&format!("Switched to {name}"), "ok");
                if let Some(t) = then {
                    t(app, true);
                }
            }),
        );
    }

    pub fn create_workspace(&mut self) {
        self.quick_input("Name your new workspace", "", "Press Enter to create the workspace, then add folders to it.", Box::new(|app, name| {
            let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else { return };
            let w = Workspace::new(&name, vec![]);
            let id = w.id.clone();
            app.s.workspaces.push(w);
            app.switch_workspace(&id, None);
            app.show_view("explorer", false);
        }));
    }

    pub fn rename_workspace(&mut self, id: &str) {
        let Some(w) = self.s.workspaces.iter().find(|w| w.id == id) else { return };
        let (id, name) = (w.id.clone(), w.name.clone());
        self.quick_input("Workspace name", &name, "Rename workspace", Box::new(move |app, name| {
            let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else { return };
            if let Some(w) = app.s.workspaces.iter_mut().find(|w| w.id == id) {
                w.name = name;
            }
            app.persist();
            app.mark(F_ALL);
        }));
    }

    pub fn delete_workspace(&mut self, id: &str) {
        if self.s.workspaces.len() == 1 {
            self.toast("You need at least one workspace.", "error");
            return;
        }
        let Some(w) = self.s.workspaces.iter().find(|w| w.id == id) else { return };
        let id = w.id.clone();
        self.confirm(
            &format!("Remove the workspace “{}”?", w.name),
            "Only the workspace is removed. Your folders and files stay exactly where they are.",
            &["Remove Workspace", "Cancel"],
            move |app, r| {
                if r != 0 {
                    return;
                }
                let id2 = id.clone();
                let remove: Then = Box::new(move |app, ok| {
                    if ok {
                        app.s.workspaces.retain(|w| w.id != id);
                        app.persist();
                        app.mark(F_ALL);
                    }
                });
                if app.s.current.as_deref() == Some(id2.as_str()) {
                    if let Some(other) = app.s.workspaces.iter().find(|w| w.id != id2).map(|w| w.id.clone()) {
                        app.switch_workspace(&other, Some(remove));
                        return;
                    }
                }
                remove(app, true);
            },
        );
    }

    pub fn add_folders(&mut self, paths: Vec<String>) {
        let mut added = vec![];
        for p in paths {
            if !self.ws().roots.contains(&p) {
                self.ws_mut().roots.push(p.clone());
                self.expanded.insert(p.clone());
                self.allow(&p);
                added.push(p);
            }
        }
        if added.is_empty() {
            return;
        }
        self.watch_roots();
        self.file_cache = None;
        self.refresh_trees();
        self.show_view("explorer", false);
        self.persist();
        let name = folio_core::esc(&self.ws().name);
        let msg = if added.len() == 1 { format!("Added {} to {name}", folio_core::esc(&basename(&added[0]))) } else { format!("Added {} folders to {name}", added.len()) };
        self.toast(&msg, "ok");
    }

    pub fn remove_folder(&mut self, root: &str) {
        self.ws_mut().roots.retain(|r| r != root);
        self.trees.remove(root);
        self.file_cache = None;
        self.watch_roots();
        self.mark(F_ALL);
        self.persist();
    }

    pub fn add_folder_dialog(&mut self) {
        let title = format!("Add folders to “{}”", self.ws().name);
        self.open_dialog(true, &title, |app, paths| app.add_folders(paths));
    }

    pub fn open_file_dialog(&mut self) {
        self.open_dialog(false, "Open Markdown files", |app, paths| app.open_paths(paths));
    }

    pub fn open_paths(&mut self, paths: Vec<String>) {
        let folders: Vec<String> = paths.iter().filter(|p| std::path::Path::new(p).is_dir()).cloned().collect();
        let files: Vec<String> = paths.iter().filter(|p| std::path::Path::new(p).is_file()).cloned().collect();
        if !folders.is_empty() {
            self.add_folders(folders);
        }
        let last = files.last().cloned();
        for f in &files {
            self.open_file(f, OpenOpts { background: Some(f) != last.as_ref(), ..Default::default() });
        }
    }

    /* ───────────── explorer operations ───────────── */

    pub fn selected_dir(&self) -> Option<String> {
        let Some(sel) = &self.selected else { return self.ws().roots.first().cloned() };
        match self.find_node(sel) {
            Some(n) if n.dir => Some(n.path.clone()),
            _ => Some(dirname(sel)),
        }
    }

    pub fn new_entry(&mut self, dir: Option<String>, is_dir: bool) {
        let Some(parent) = dir.or_else(|| self.ws().roots.first().cloned()) else {
            self.new_untitled("");
            return;
        };
        self.expanded.insert(parent.clone());
        let anchor = parent.clone();
        self.inline_input(&anchor, "", is_dir, false, Box::new(move |app, name| {
            let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else { return };
            let mut name = name;
            if !is_dir && !name.contains('.') {
                name.push_str(".md");
            }
            let path = format!("{}/{}", parent.trim_end_matches('/'), name);
            let text = if is_dir { String::new() } else { format!("# {}\n\n", fsx::strip_ext(&basename(&name))) };
            match fsx::create(&path, is_dir, &text) {
                Ok(()) => {
                    app.file_cache = None;
                    app.refresh_trees();
                    if is_dir {
                        app.expanded.insert(path.clone());
                        app.selected = Some(path);
                        app.mark(F_SIDE);
                    } else if let Some(k) = app.open_file(&path, OpenOpts::default()) {
                        if app.docs[&k].mode == Mode::Read {
                            app.start_live_append(&k);
                        } else {
                            let len = app.docs[&k].text.len();
                            app.focus_source(&k, Some(len));
                        }
                    }
                }
                Err(e) => app.toast(&folio_core::esc(&e), "error"),
            }
        }));
    }

    pub fn rename_entry(&mut self, path: &str) {
        let is_dir = self.find_node(path).is_some_and(|n| n.dir);
        let old = path.to_string();
        self.inline_input(path, &basename(path), is_dir, true, Box::new(move |app, name| {
            let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty() && *n != basename(&old)) else { return };
            let to = format!("{}/{}", dirname(&old), name);
            app.move_path(&old, &to);
        }));
    }

    pub fn move_path(&mut self, from: &str, to: &str) {
        if from == to || within(to, from) {
            return;
        }
        if let Err(e) = fsx::rename(from, to) {
            self.toast(&folio_core::esc(&e), "error");
            return;
        }
        let keys: Vec<String> = self.docs.values().filter(|d| d.path.as_deref().is_some_and(|p| within(p, from))).map(|d| d.key.clone()).collect();
        for old in keys {
            let mut d = self.docs.remove(&old).unwrap();
            let np = format!("{to}{}", &old[from.len()..]);
            d.key = np.clone();
            d.path = Some(np.clone());
            d.name = basename(&np);
            d.cache = None;
            d.preview_stale = true;
            self.docs.insert(np.clone(), d);
            for t in self.tabs.iter_mut() {
                if *t == old {
                    *t = np.clone();
                }
            }
            if self.active.as_deref() == Some(&old) {
                self.active = Some(np);
            }
        }
        if self.expanded.remove(from) {
            self.expanded.insert(to.to_string());
        }
        self.selected = Some(to.to_string());
        self.allow(&dirname(to));
        self.file_cache = None;
        self.refresh_trees();
        self.mark(F_ALL);
        self.persist();
    }

    pub fn trash_entry(&mut self, path: &str) {
        let is_dir = self.find_node(path).is_some_and(|n| n.dir);
        let p = path.to_string();
        self.confirm(
            &format!("Move “{}” to the Trash?", basename(path)),
            if is_dir { "The folder and everything in it will be moved to the Trash." } else { "You can restore it from the Trash." },
            &["Move to Trash", "Cancel"],
            move |app, r| {
                if r != 0 {
                    return;
                }
                match fsx::trash(&p) {
                    Ok(()) => {
                        let keys: Vec<String> = app.docs.values().filter(|d| d.path.as_deref().is_some_and(|x| within(x, &p))).map(|d| d.key.clone()).collect();
                        for k in keys {
                            app.close_tab(&k, true, None);
                        }
                        app.file_cache = None;
                        app.refresh_trees();
                        app.toast(&format!("Moved {} to the Trash", folio_core::esc(&basename(&p))), "ok");
                    }
                    Err(e) => app.toast(&folio_core::esc(&e), "error"),
                }
            },
        );
    }

    pub fn inline_input(&mut self, anchor: &str, value: &str, dir: bool, renaming: bool, then: InputFn) {
        if let Some(old) = self.inline.take() {
            if let Some(t) = old.then {
                t(self, None);
            }
        }
        self.inline = Some(Inline { anchor: anchor.into(), value: value.into(), dir, renaming, then: Some(then) });
        let sel_end = if renaming { value.rfind('.').filter(|&d| d > 0).unwrap_or(value.len()) } else { value.len() };
        self.show_view("explorer", false);
        self.ui.later(json!({"op": "focus", "id": "inline-input", "s": 0, "e": byte_to_u16(value, sel_end)}));
    }

    pub fn finish_inline(&mut self, v: Option<String>) {
        let Some(mut i) = self.inline.take() else { return };
        self.mark(F_SIDE);
        if let Some(t) = i.then.take() {
            t(self, v);
        }
    }

    /// Visible explorer rows in order (for keyboard navigation).
    pub fn visible_rows(&self) -> Vec<String> {
        fn walk(n: &Node, exp: &HashSet<String>, out: &mut Vec<String>) {
            out.push(n.path.clone());
            if n.dir && exp.contains(&n.path) {
                for c in &n.children {
                    walk(c, exp, out);
                }
            }
        }
        let mut out = vec![];
        for r in &self.ws().roots {
            if let Some(t) = self.trees.get(r) {
                walk(t, &self.expanded, &mut out);
            }
        }
        out
    }

    /* ───────────── views ───────────── */

    pub fn show_view(&mut self, v: &str, toggle: bool) {
        let st = &mut self.s.settings;
        if toggle && st.view == v && st.sidebar_visible {
            st.sidebar_visible = false;
        } else {
            st.view = v.into();
            st.sidebar_visible = true;
        }
        st.zen = false;
        self.mark(F_LAYOUT | F_ACT | F_SIDE | F_TITLE);
        if v == "search" && self.s.settings.sidebar_visible {
            self.ui.later(json!({"op": "focus", "id": "search-input", "all": true}));
        }
        self.persist();
    }

    pub fn run_search(&mut self) {
        let q = self.search.query.clone();
        let roots = match &self.search.scope {
            Some(s) => vec![s.clone()],
            None => self.ws().roots.clone(),
        };
        self.search.seq += 1;
        if q.is_empty() || roots.is_empty() {
            self.search.result = None;
            self.search.running = false;
            self.mark(F_SIDE);
            return;
        }
        self.search.running = true;
        self.mark(F_SIDE);
        let seq = self.search.seq;
        let (rx, cs, w) = (self.search.regex, self.search.case, self.search.word);
        self.background(
            move || fsx::search(&roots, &q, rx, cs, w),
            move |app, r| {
                if app.search.seq != seq {
                    return;
                }
                app.search.running = false;
                app.search.result = Some(r);
                app.search.collapsed.clear();
                app.mark(F_SIDE);
            },
        );
    }

    /* ───────────── find in document ───────────── */

    pub fn open_find(&mut self, seed: Option<String>) {
        let Some(key) = self.md_key() else { return };
        self.find.open = true;
        if let Some(s) = seed.filter(|s| !s.is_empty() && !s.contains('\n') && s.chars().count() < 200) {
            self.find.query = s.clone();
            self.ui.now(json!({"op": "val", "id": "find-input", "v": s}));
        }
        self.ui.later(json!({"op": "focus", "id": "find-input", "all": true}));
        self.find.index = None;
        let _ = key;
        self.find_refresh(false);
        self.find_show(true);
        self.mark(F_FIND);
    }

    pub fn close_find(&mut self) {
        let target = self.find.target;
        self.find.open = false;
        self.find.ranges.clear();
        self.find.marks.clear();
        if let Some(k) = self.md_key() {
            let d = self.docs.get_mut(&k).unwrap();
            d.preview_stale = true;
            if target == Some(Pane::Source) {
                self.focus_source(&k, None);
            }
        }
        self.mark(F_FIND | F_EDITOR);
    }

    pub fn find_refresh(&mut self, keep: bool) {
        self.find.ranges.clear();
        self.find.marks.clear();
        self.find.error = false;
        let Some(key) = self.md_key().filter(|_| self.find.open) else {
            self.find.target = None;
            return;
        };
        let d = &self.docs[&key];
        let target = if d.mode == Mode::Read || (d.mode == Mode::Split && d.last_focus == Pane::Preview) { Pane::Preview } else { Pane::Source };
        self.find.target = Some(target);
        self.docs.get_mut(&key).unwrap().preview_stale = true;
        self.mark(F_EDITOR | F_FIND);
        if self.find.query.is_empty() {
            self.find.index = None;
            return;
        }
        let re = match fsx::build_regex(&self.find.query, self.find.regex, self.find.case, self.find.word) {
            Ok(r) => r,
            Err(_) => {
                self.find.error = true;
                return;
            }
        };
        let d = &self.docs[&key];
        if target == Pane::Source {
            self.find.ranges = re.find_iter(&d.text).filter(|m| m.start() < m.end()).take(5000).map(|m| (m.start(), m.end())).collect();
        } else {
            let r = self.rendered(&key);
            let (_, marks) = folio_core::mark_html(&r.html, &mut |t: &str| re.find_iter(t).filter(|m| m.start() < m.end()).map(|m| (m.start(), m.end())).collect(), 5000);
            self.find.marks = marks;
        }
        let n = self.count_matches();
        if !keep || self.find.index.is_some_and(|i| i >= n) {
            self.find.index = None;
        }
        if self.find.index.is_none() && n > 0 {
            let d = &self.docs[&key];
            self.find.index = Some(if target == Pane::Source {
                let caret = d.text.split('\n').take(d.cursor.0).map(|l| l.len() + 1).sum::<usize>();
                self.find.ranges.iter().position(|r| r.0 >= caret).unwrap_or(0)
            } else {
                let top = d.top_line;
                self.find.marks.iter().position(|&l| l as f64 >= top.floor()).unwrap_or(0)
            });
        }
    }

    pub fn count_matches(&self) -> usize {
        if self.find.target == Some(Pane::Source) {
            self.find.ranges.len()
        } else {
            self.find.marks.len()
        }
    }

    pub fn find_show(&mut self, mv: bool) {
        self.mark(F_FIND);
        let (Some(i), Some(key)) = (self.find.index, self.md_key()) else { return };
        let d = &self.docs[&key];
        let mut op = json!({"op": "findcur", "doc": d.n, "i": i, "pane": if self.find.target == Some(Pane::Source) { "source" } else { "preview" }, "move": mv});
        if self.find.target == Some(Pane::Source) && mv {
            if let Some(&(s, e)) = self.find.ranges.get(i) {
                op["s"] = json!(byte_to_u16(&d.text, s));
                op["e"] = json!(byte_to_u16(&d.text, e));
            }
        }
        self.ui.later(op);
    }

    pub fn find_step(&mut self, dir: isize) {
        if !self.find.open {
            self.open_find(None);
            return;
        }
        let n = self.count_matches();
        if n == 0 {
            return;
        }
        let i = self.find.index.map(|i| i as isize).unwrap_or(-1);
        self.find.index = Some(((i + dir).rem_euclid(n as isize)) as usize);
        self.find_show(true);
    }

    /* ───────────── layout, settings ───────────── */

    pub fn set_setting(&mut self, key: &str, v: Value) {
        let mut cur = serde_json::to_value(&self.s.settings).unwrap_or(json!({}));
        cur[key] = v;
        if let Ok(s) = serde_json::from_value::<Settings>(cur) {
            self.s.settings = s;
        }
        if key == "showAllFiles" {
            self.refresh_trees();
        }
        if key == "remoteImages" || key == "readerFont" {
            for d in self.docs.values_mut() {
                d.preview_stale = true;
            }
        }
        self.mark(F_ALL);
        self.persist();
    }

    pub fn zoom(&mut self, dir: i32) {
        let d = Settings::default();
        let s = &mut self.s.settings;
        if dir == 0 {
            s.reader_size = d.reader_size;
            s.editor_size = d.editor_size;
        } else {
            s.reader_size = (s.reader_size + dir as f64).clamp(11.0, 28.0);
            s.editor_size = (s.editor_size + dir as f64).clamp(9.0, 22.0);
        }
        let pct = (s.reader_size / d.reader_size * 100.0).round();
        self.mark(F_LAYOUT);
        self.persist();
        self.toast(&format!("Text size {pct}%"), "info");
    }

    /* ───────────── external changes ───────────── */

    pub fn on_fs_change(&mut self) {
        self.debounce("fs", 350, |app| {
            app.file_cache = None;
            app.refresh_trees();
            app.check_external();
        });
    }

    pub fn check_external(&mut self) {
        let keys: Vec<String> = self.docs.values().filter(|d| d.path.is_some()).map(|d| d.key.clone()).collect();
        for k in keys {
            let p = self.docs[&k].path.clone().unwrap();
            let Some(m) = fsx::mtime(&p) else {
                let d = self.docs.get_mut(&k).unwrap();
                if !d.missing {
                    d.missing = true;
                    self.dirty |= F_EDITOR;
                }
                continue;
            };
            let d = self.docs.get_mut(&k).unwrap();
            if d.missing {
                d.missing = false;
                self.dirty |= F_EDITOR;
            }
            if d.mtime > 0.0 && (m - d.mtime).abs() > 0.0005 {
                if !d.is_dirty() && self.s.settings.auto_reload && d.live.is_none() {
                    if let Ok((text, mtime)) = fsx::read(&p) {
                        let d = self.docs.get_mut(&k).unwrap();
                        d.mtime = mtime;
                        if text != d.text {
                            let name = d.name.clone();
                            self.set_text(&k, text, true);
                            if self.active.as_deref() == Some(k.as_str()) {
                                self.toast(&format!("Reloaded {} — it changed on disk", folio_core::esc(&name)), "info");
                            }
                        }
                    }
                } else if !d.conflict {
                    d.conflict = true;
                    d.mtime = m;
                    self.dirty |= F_EDITOR | F_TABS | F_STATUS;
                }
            }
        }
    }

    /* ───────────── quitting ───────────── */

    pub fn request_close(&mut self) {
        self.resolve_dirty(
            "quitting",
            Box::new(|app, ok| {
                if !ok {
                    return;
                }
                let keys: Vec<String> = app.docs.keys().cloned().collect();
                for k in keys {
                    app.commit_live(&k, false);
                }
                app.persist_now();
                app.after(|h| crate::quit(h));
            }),
        );
    }

    /* ───────────── dialogs (non-blocking; callbacks come back under the lock) ───────────── */

    pub fn confirm(&mut self, message: &str, detail: &str, buttons: &[&str], then: impl FnOnce(&mut App, usize) + Send + 'static) {
        let (message, detail) = (message.to_string(), detail.to_string());
        let labels: Vec<String> = buttons.iter().map(|s| s.to_string()).collect();
        self.after(move |h| crate::confirm(h, message, detail, labels, then));
    }

    pub fn save_dialog(&mut self, name: &str, dir: &str, then: impl FnOnce(&mut App, Option<String>) + Send + 'static) {
        let (name, dir) = (name.to_string(), dir.to_string());
        self.after(move |h| crate::save_dialog(h, name, dir, then));
    }

    pub fn open_dialog(&mut self, folders: bool, title: &str, then: impl FnOnce(&mut App, Vec<String>) + Send + 'static) {
        let title = title.to_string();
        self.after(move |h| crate::open_dialog(h, folders, title, then));
    }

    /* ───────────── export ───────────── */

    pub fn clean_html(&mut self, key: &str) -> String {
        let r = self.rendered(key);
        strip_ui(&r.html)
    }

    pub fn export_html(&mut self) {
        let Some(key) = self.md_key() else { return };
        let d = &self.docs[&key];
        let name = format!("{}.html", fsx::strip_ext(&d.name));
        let dir = d.dir().unwrap_or_default();
        let title = fsx::strip_ext(&d.name);
        let body = self.clean_html(&key);
        let dark = theme(&self.s.settings.theme).dark;
        let font = self.s.settings.reader_font.clone();
        self.save_dialog(&name, &dir, move |app, path| {
            let Some(path) = path else { return };
            let path = if path.ends_with(".html") { path } else { format!("{path}.html") };
            let html = format!(
                "<!doctype html><html data-theme=\"{}\" data-reader-font=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>{}\nbody{{overflow:auto;height:auto;background:var(--bg-0);opacity:1}}.export{{padding:48px 24px}}</style></head><body><div class=\"export\"><article class=\"markdown-body\">{}</article></div></body></html>",
                if dark { "dark" } else { "light" },
                font,
                folio_core::esc(&title),
                crate::STYLE,
                body
            );
            match fsx::write(&path, &html) {
                Ok(_) => app.toast(&format!("Exported {}", folio_core::esc(&basename(&path))), "ok"),
                Err(e) => app.toast(&folio_core::esc(&e), "error"),
            }
        });
    }
}


/// Remove workbench-only markup (anchors, code headers, data-line) from rendered HTML.
pub fn strip_ui(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let skip = ["<a class=\"anchor\"", "<div class=\"code-head\""].iter().find(|p| rest.starts_with(**p)).map(|p| if p.starts_with("<a") { "</a>" } else { "</div>" });
        if let Some(close) = skip {
            if let Some(e) = rest.find(close) {
                rest = &rest[e + close.len()..];
                continue;
            }
        }
        let e = rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
        let tag = &rest[..e];
        match tag.find(" data-line=\"") {
            Some(p) => {
                let q = tag[p + 12..].find('"').map(|q| p + 12 + q + 1).unwrap_or(tag.len());
                out.push_str(&tag[..p]);
                out.push_str(&tag[q..]);
            }
            None => out.push_str(tag),
        }
        rest = &rest[e..];
    }
    out.push_str(rest);
    out
}

#[derive(Default)]
pub struct OpenOpts {
    pub preview: bool,
    pub line: Option<usize>,
    pub range: Option<(usize, usize)>,
    pub anchor: Option<String>,
    pub mode: Option<Mode>,
    pub background: bool,
}

/// Map rendered text up to the click point onto an offset in the block's Markdown.
pub fn caret_from_hint(src: &str, hint: Option<&str>) -> usize {
    let Some(hint) = hint else { return src.len() };
    let flat: String = hint.split_whitespace().collect::<Vec<_>>().join(" ") + if hint.ends_with(char::is_whitespace) { " " } else { "" };
    if flat.trim().is_empty() {
        let re = regex::Regex::new(r"^\s*(#{1,6}\s+|(?:[-*+]|\d+[.)])\s+(?:\[[ xX]\]\s+)?|>\s*)").unwrap();
        return re.find(src).map(|m| m.end()).unwrap_or(0);
    }
    let chars: Vec<char> = flat.chars().collect();
    for n in [18usize, 10, 5, 2] {
        let take = n.min(chars.len());
        if take < n.min(chars.len()) {
            continue;
        }
        let tail: String = chars[chars.len() - take..].iter().collect();
        let from = flat.len().saturating_sub(tail.len()).min(src.len());
        let from = (0..=from).rev().find(|&i| src.is_char_boundary(i)).unwrap_or(0);
        let i = src[from..].find(&tail).map(|i| i + from).or_else(|| src.find(&tail));
        if let Some(i) = i {
            return i + tail.len();
        }
    }
    src.len()
}
