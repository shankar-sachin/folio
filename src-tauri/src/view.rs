//! HTML for every workbench region. Pure functions of the `App` state.

use crate::app::{theme, App, PalItem, THEMES};
use crate::commands::COMMANDS;
use crate::fsx::{basename, dirname, is_markdown, Node};
use crate::model::{now_ms, Doc, Kind, Mode};
use folio_core::{esc, icon};
use std::fmt::Write;

fn i(name: &str) -> String {
    icon(name, "")
}
fn a(s: &str) -> String {
    esc(s)
}

pub fn file_icon(name: &str, dir: bool, open: bool) -> String {
    if dir {
        icon(if open { "folderOpen" } else { "folder" }, "fi-folder")
    } else if is_markdown(name) {
        icon("md", "fi-md")
    } else {
        icon("file", "fi-file")
    }
}

fn doc_icon(d: &Doc) -> String {
    match d.kind {
        Kind::Md => file_icon(&d.name, false, false),
        Kind::Settings => icon("settings", "fi-file"),
        Kind::Shortcuts => icon("keyboard", "fi-file"),
    }
}

fn plural(n: usize, w: &str) -> String {
    let s = n.to_string();
    let mut grouped = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{grouped} {w}{}", if n == 1 { "" } else { "s" })
}

pub fn time_ago(t: f64) -> String {
    if t <= 0.0 {
        return String::new();
    }
    let s = (now_ms() - t) / 1000.0;
    if s < 60.0 {
        "just now".into()
    } else if s < 3600.0 {
        format!("{}m ago", (s / 60.0) as u64)
    } else if s < 86400.0 {
        format!("{}h ago", (s / 3600.0) as u64)
    } else if s < 86400.0 * 30.0 {
        format!("{}d ago", (s / 86400.0) as u64)
    } else {
        format!("{} months ago", (s / 86400.0 / 30.0) as u64)
    }
}

fn guides(depth: usize) -> String {
    (0..depth).map(|g| format!("<span class=\"guide\" style=\"--g:{g}\"></span>")).collect()
}

fn keys(k: &str) -> String {
    k.split(' ').map(|x| format!("<kbd>{}</kbd>", esc(x))).collect()
}

/* ───────────── documents ───────────── */

pub fn doc_shell(d: &Doc) -> String {
    let n = d.n;
    if d.kind != Kind::Md {
        return format!("<div class=\"doc special\" id=\"doc-{n}\" data-n=\"{n}\" data-mode=\"special\"><div class=\"special-scroll\" id=\"sp-{n}\"></div></div>");
    }
    format!(
        r#"<div class="doc" id="doc-{n}" data-n="{n}" data-mode="{mode}">
<div class="banner" id="ban-{n}" hidden></div>
<div class="panes">
<section class="pane source"><div class="scroller"><div class="stack">
<div class="findlayer" id="fl-{n}" aria-hidden="true"></div>
<div class="hl" id="hl-{n}" aria-hidden="true"></div>
<textarea id="ta-{n}" data-i="src" data-caret="{n}" data-keys="Enter,Tab,Shift+Tab,Meta+[,Meta+]" data-k="src" spellcheck="false" autocapitalize="off" autocomplete="off" autocorrect="off" aria-label="Markdown source"></textarea>
</div></div></section>
<div class="divider" role="separator" aria-label="Resize editor and preview" data-resize="split" data-ref=".panes" data-var="--split" data-min="0.2" data-max="0.8" data-frac></div>
<section class="pane preview"><div class="scroller"><article class="markdown-body" id="art-{n}"></article><div class="endmark" data-a="append:{n}"><span></span></div></div></section>
</div></div>"#,
        mode = d.mode.as_str()
    )
}

pub fn live_editor(src: &str, append: bool) -> String {
    format!(
        "<div class=\"live-editor{}\"><textarea class=\"live-ta\" id=\"live-ta\" spellcheck=\"true\" rows=\"1\" placeholder=\"{}\" data-i=\"live\" data-blur=\"live\" data-k=\"live\" data-keys=\"Escape,Meta+Enter,Ctrl+Enter,Enter,Tab,Shift+Tab,Meta+[,Meta+]\">\n{}</textarea><div class=\"live-hint\"><span>{}Markdown</span><span><kbd>⌘B</kbd> bold <kbd>⌘I</kbd> italic <kbd>⌘K</kbd> link <kbd>Esc</kbd> done</span></div></div>",
        if append { " append" } else { "" },
        if append { "Write Markdown…" } else { "" },
        esc(src),
        i("md")
    )
}

impl App {
    pub fn view_banner(&self, d: &Doc) -> Option<String> {
        if !d.conflict && !d.missing {
            return None;
        }
        let mut h = format!(
            "<span>{}{}</span>",
            i("warning"),
            if d.missing { format!("<b>{}</b> was moved or deleted on disk.", esc(&d.name)) } else { format!("<b>{}</b> changed on disk while you had unsaved edits.", esc(&d.name)) }
        );
        if !d.missing {
            let _ = write!(h, "<button class=\"btn small\" data-a=\"revert:{}\">Load Disk Version</button>", a(&d.key));
        }
        let _ = write!(h, "<button class=\"btn small ghost\" data-a=\"keep:{}\">{}</button>", a(&d.key), if d.missing { "Dismiss" } else { "Keep Mine" });
        Some(h)
    }

    /* ───────────── activity bar & title bar ───────────── */

    pub fn view_activity(&self) -> (String, String) {
        let st = self.set();
        let mut top = String::new();
        let dirty = self.dirty_keys().len();
        for (v, ic, label, key) in [("explorer", "files", "Explorer", "⇧⌘E"), ("search", "search", "Search", "⇧⌘F"), ("outline", "outline", "Outline", "⇧⌘U"), ("workspaces", "layers", "Workspaces", "⇧⌘K")] {
            let on = st.view == v && st.sidebar_visible;
            let badge = if v == "explorer" && dirty > 0 { format!("<span class=\"badge\">{dirty}</span>") } else { String::new() };
            let _ = write!(top, "<button class=\"ab-btn{}\" title=\"{label} ({key})\" data-a=\"view:{v}\">{}{badge}</button>", if on { " on" } else { "" }, i(ic));
        }
        let bottom = format!(
            "<button class=\"ab-btn\" title=\"Keyboard Shortcuts\" data-a=\"cmd:shortcuts\">{}</button><button class=\"ab-btn{}\" title=\"Settings (⌘,)\" data-a=\"cmd:settings\">{}</button>",
            i("keyboard"),
            if self.active.as_deref() == Some("folio:settings") { " on" } else { "" },
            i("settings")
        );
        (top, bottom)
    }

    /* ───────────── explorer ───────────── */

    fn section(&self, id: &str, title: &str, open: bool, actions: &[(&str, &str, &str)], body: &str) -> String {
        let acts: String = actions.iter().map(|(ic, label, act)| format!("<button class=\"icon-btn\" title=\"{label}\" data-a=\"{}\">{}</button>", a(act), i(ic))).collect();
        format!(
            "<div class=\"sec{o}\"><div class=\"sec-head{o}\" tabindex=\"0\" data-a=\"sec:{id}\">{}<span>{}</span><div class=\"sec-actions\">{acts}</div></div>{}</div>",
            icon("chevron", "twisty"),
            esc(title),
            if open { body } else { "" },
            o = if open { " open" } else { "" }
        )
    }

    pub fn view_explorer(&self) -> String {
        let w = self.ws();
        let mut list = String::from("<div class=\"open-editors\">");
        for k in &self.tabs {
            let Some(d) = self.docs.get(k) else { continue };
            let cls = format!("row oe{}{}", if self.active.as_deref() == Some(k) { " active" } else { "" }, if d.is_dirty() { " dirty" } else { "" });
            let desc = d.path.as_deref().map(|p| self.rel_path(&dirname(p))).unwrap_or_default();
            let _ = write!(
                list,
                "<div class=\"{cls}\" title=\"{}\" data-a=\"tab:{key}\" data-aux=\"close:{key}\"><button class=\"icon-btn close\" title=\"Close\" data-a=\"close:{key}\">{}</button>{}<span class=\"label\">{}</span><span class=\"desc\">{}</span></div>",
                a(&d.path.as_deref().map(|p| self.tilde(p)).unwrap_or_else(|| d.name.clone())),
                i("x"),
                doc_icon(d),
                esc(&d.name),
                esc(&desc),
                key = a(k)
            );
        }
        if self.tabs.is_empty() {
            list.push_str("<div class=\"empty-note\">No open editors</div>");
        }
        list.push_str("</div>");
        let mut out = self.section("openEditors", "Open Editors", self.set().open_editors, &[("x", "Close All", "cmd:closeAll")], &list);

        let mut tree = String::from("<div class=\"tree\" role=\"tree\">");
        if w.roots.is_empty() {
            let _ = write!(
                tree,
                "<div class=\"empty-state\"><p>“{}” has no folders yet. Add a folder of Markdown files to browse, search and edit them here.</p><button class=\"btn primary\" data-a=\"cmd:addFolder\">{}Add Folder</button><button class=\"btn\" data-a=\"cmd:newFile\">{}New File</button></div>",
                esc(&w.name),
                i("folderPlus"),
                i("filePlus")
            );
        }
        for root in &w.roots {
            match self.trees.get(root) {
                Some(t) => self.view_node(&mut tree, t, 0, true),
                None => {
                    let n = Node { name: basename(root), path: root.clone(), dir: true, missing: false, children: vec![] };
                    self.view_node(&mut tree, &n, 0, true);
                    let _ = write!(tree, "<div class=\"empty-note\" style=\"--d:1\"><span class=\"spinner\"></span>Loading…</div>");
                }
            }
        }
        tree.push_str("</div>");
        out.push_str(&self.section(
            "wsOpen",
            &w.name,
            self.set().ws_open,
            &[("filePlus", "New File…", "cmd:newFileInFolder"), ("folderPlus", "New Folder…", "cmd:newFolder"), ("refresh", "Refresh Explorer", "cmd:refreshExplorer"), ("collapse", "Collapse Folders", "cmd:collapseFolders")],
            &tree,
        ));
        out
    }

    fn inline_row(&self, depth: usize, dir: bool, value: &str) -> String {
        format!(
            "<div class=\"row inline-input\" style=\"--d:{depth}\">{}<span class=\"twisty-space\"></span>{}<input id=\"inline-input\" value=\"{}\" spellcheck=\"false\" placeholder=\"{}\" data-keys=\"Enter,Escape\" data-k=\"inline\" data-blur=\"inline\"></div>",
            guides(depth),
            if dir { icon("folder", "fi-folder") } else { icon("md", "fi-md") },
            a(value),
            if dir { "Folder name" } else { "File name" }
        )
    }

    fn view_node(&self, out: &mut String, n: &Node, depth: usize, root: bool) {
        let open = self.expanded.contains(&n.path);
        let d = self.docs.get(&n.path);
        let inline = self.inline.as_ref();
        if inline.is_some_and(|x| x.renaming && x.anchor == n.path) {
            out.push_str(&self.inline_row(depth, n.dir, &inline.unwrap().value));
        } else {
            let mut cls = vec!["row", if n.dir { "dir" } else { "file" }];
            if root {
                cls.push("root");
            }
            if self.active.as_deref() == Some(&n.path) {
                cls.push("active");
            }
            if self.selected.as_deref() == Some(&n.path) {
                cls.push("selected");
            }
            let dirty = d.is_some_and(Doc::is_dirty);
            if dirty {
                cls.push("dirty");
            }
            if d.is_some() {
                cls.push("opened");
            }
            if n.missing {
                cls.push("missing");
            }
            let drop = if n.dir { n.path.clone() } else { dirname(&n.path) };
            let p = a(&n.path);
            let _ = write!(
                out,
                "<div class=\"{}\" role=\"treeitem\" style=\"--d:{depth}\" title=\"{}\" data-a=\"row:{p}\" data-dbl=\"pin:{p}\" data-ctx=\"row:{p}\"{} data-drop=\"tree:{}\">{}{}{}<span class=\"label\">{}</span>{}{}</div>",
                cls.join(" "),
                a(&self.tilde(&n.path)),
                if root { String::new() } else { format!(" data-drag=\"tree:{p}\"") },
                a(&drop),
                guides(depth),
                if n.dir { icon("chevron", if open { "twisty open" } else { "twisty" }) } else { "<span class=\"twisty-space\"></span>".into() },
                file_icon(&n.name, n.dir, open),
                esc(&n.name),
                if n.missing { "<span class=\"desc\">missing</span>" } else { "" },
                if dirty { "<span class=\"dot\"></span>" } else { "" }
            );
        }
        if n.dir && open {
            if inline.is_some_and(|x| !x.renaming && x.anchor == n.path) {
                out.push_str(&self.inline_row(depth + 1, inline.unwrap().dir, ""));
            }
            for c in &n.children {
                self.view_node(out, c, depth + 1, false);
            }
            if root && n.children.is_empty() && self.trees.contains_key(&n.path) {
                let msg = if n.missing { "Folder not found" } else if self.set().show_all_files { "Empty folder" } else { "No Markdown files here" };
                let _ = write!(out, "<div class=\"empty-note\" style=\"--d:1\">{msg}</div>");
            }
        }
    }

    /* ───────────── search ───────────── */

    pub fn view_search(&self) -> (String, String, String) {
        let s = &self.search;
        let toggles = format!(
            "<button class=\"{}\" title=\"Match Case\" data-a=\"sopt:case\">Aa</button><button class=\"{}\" title=\"Match Whole Word\" data-a=\"sopt:word\"><u>ab</u></button><button class=\"{}\" title=\"Use Regular Expression\" data-a=\"sopt:regex\">.*</button>",
            if s.case { "on" } else { "" },
            if s.word { "on" } else { "" },
            if s.regex { "on" } else { "" }
        );
        let scope = s.scope.as_deref().map(basename);
        let summary = if s.query.is_empty() {
            String::new()
        } else if self.ws().roots.is_empty() && s.scope.is_none() {
            "Add a folder to this workspace to search it.".into()
        } else if s.running {
            "<span class=\"spinner\"></span> Searching…".into()
        } else if let Some(r) = &s.result {
            if let Some(e) = &r.error {
                esc(e)
            } else if r.total > 0 {
                format!(
                    "{} in {}{}",
                    plural(r.total, "result"),
                    plural(r.files.len(), "file"),
                    scope.map(|x| format!(" · in <b>{}</b> <button class=\"link\" data-a=\"sclear\">clear</button>", esc(&x))).unwrap_or_default()
                )
            } else {
                format!("No results{}", scope.map(|x| format!(" in {} <button class=\"link\" data-a=\"sclear\">search everywhere</button>", esc(&x))).unwrap_or_default())
            }
        } else {
            String::new()
        };
        let mut out = String::new();
        if let Some(r) = s.result.as_ref().filter(|_| !s.query.is_empty()) {
            for (path, matches) in &r.files {
                let open = !s.collapsed.contains(path);
                let p = a(path);
                let _ = write!(
                    out,
                    "<div class=\"sr-file{o}\"><div class=\"row sr-head\" title=\"{}\" data-a=\"srtoggle:{p}\">{}{}<span class=\"label\">{}</span><span class=\"desc\">{}</span><span class=\"badge\">{}</span></div><div class=\"sr-list\">",
                    a(&self.tilde(path)),
                    icon("chevron", if open { "twisty open" } else { "twisty" }),
                    file_icon(path, false, false),
                    esc(&basename(path)),
                    esc(&self.rel_path(&dirname(path))),
                    matches.len(),
                    o = if open { " open" } else { "" }
                );
                if open {
                    for (k, m) in matches.iter().take(200).enumerate() {
                        let t = &m.text;
                        let pre_start = t[..m.start].char_indices().rev().nth(39).map(|(b, _)| b).unwrap_or(0);
                        let pre = t[pre_start..m.start].trim_start();
                        let post_end = t[m.end..].char_indices().nth(120).map(|(b, _)| m.end + b).unwrap_or(t.len());
                        let sel = s.selected.as_ref().is_some_and(|(sp, si)| sp == path && *si == k);
                        let _ = write!(
                            out,
                            "<div class=\"row sr-match{}\" title=\"Line {}\" data-a=\"sr:{k}:{p}\"><span class=\"ln-no\">{}</span><span class=\"label\">{}{}<mark>{}</mark>{}</span></div>",
                            if sel { " selected" } else { "" },
                            m.line,
                            m.line,
                            if pre_start > 0 { "…" } else { "" },
                            esc(pre),
                            esc(&t[m.start..m.end]),
                            esc(&t[m.end..post_end])
                        );
                    }
                }
                out.push_str("</div></div>");
            }
        }
        (toggles, summary, out)
    }

    /* ───────────── outline ───────────── */

    pub fn view_outline(&mut self) -> String {
        let Some(key) = self.md_key() else { return "<div class=\"empty-note pad\">Open a Markdown file to see its outline.</div>".into() };
        let r = self.rendered(&key);
        if r.headings.is_empty() {
            return "<div class=\"empty-note pad\">This document has no headings yet.</div>".into();
        }
        let cur = self.docs[&key].cur_heading;
        let q = self.outline_filter.trim().to_lowercase();
        let min = r.headings.iter().map(|h| h.depth).min().unwrap_or(1) as usize;
        let mut out = String::new();
        for (k, h) in r.headings.iter().enumerate() {
            if !q.is_empty() && !h.text.to_lowercase().contains(&q) {
                continue;
            }
            let d = h.depth as usize - min;
            let _ = write!(
                out,
                "<div class=\"row ol{}\" style=\"--d:{d}\" title=\"{}\" data-a=\"heading:{k}\">{}<span class=\"hd-level\">H{}</span><span class=\"label\">{}</span><span class=\"desc\">{}</span></div>",
                if cur == Some(k) { " current" } else { "" },
                a(&h.text),
                guides(d),
                h.depth,
                esc(if h.text.is_empty() { "(untitled)" } else { &h.text }),
                h.line + 1
            );
        }
        out
    }

    /* ───────────── workspaces ───────────── */

    pub fn view_workspaces(&self) -> String {
        let cur = self.ws().id.clone();
        let mut list: Vec<_> = self.s.workspaces.iter().collect();
        list.sort_by(|x, y| (y.id == cur).cmp(&(x.id == cur)).then(y.opened.partial_cmp(&x.opened).unwrap_or(std::cmp::Ordering::Equal)));
        let mut out = String::from("<div class=\"ws-list\">");
        for w in list {
            let is_cur = w.id == cur;
            let initials: String = w.name.split_whitespace().filter_map(|x| x.chars().next()).take(2).collect::<String>().to_uppercase();
            let hue = w.id.chars().map(|c| c as u32).sum::<u32>() % 360;
            let tabs = if is_cur { self.tabs.len() } else { w.tabs.len() };
            let _ = write!(
                out,
                "<div class=\"ws-card{}\"{}><div class=\"ws-avatar\" style=\"--hue:{hue}\">{}</div><div class=\"ws-meta\"><div class=\"ws-name\">{}{}</div><div class=\"ws-sub\">{} · {}{}</div></div><button class=\"icon-btn\" title=\"More\" data-a=\"wsmenu:{}\">{}</button>",
                if is_cur { " current" } else { "" },
                if is_cur { String::new() } else { format!(" data-a=\"ws:{}\"", a(&w.id)) },
                esc(&initials),
                esc(&w.name),
                if is_cur { "<span class=\"pill\">Current</span>" } else { "" },
                plural(w.roots.len(), "folder"),
                plural(tabs, "tab"),
                if is_cur { String::new() } else { format!(" · {}", time_ago(w.opened)) },
                a(&w.id),
                i("more")
            );
            if is_cur {
                out.push_str("<div class=\"ws-folders\">");
                for r in &w.roots {
                    let _ = write!(
                        out,
                        "<div class=\"ws-folder\" title=\"{}\">{}<span class=\"label\">{}</span><span class=\"desc\">{}</span><button class=\"icon-btn\" title=\"Remove from workspace\" data-a=\"rmroot:{}\">{}</button></div>",
                        a(&self.tilde(r)),
                        icon("folder", "fi-folder"),
                        esc(&basename(r)),
                        esc(&self.tilde(&dirname(r))),
                        a(r),
                        i("x")
                    );
                }
                let _ = write!(out, "<button class=\"btn ghost small\" data-a=\"cmd:addFolder\">{}Add Folder…</button></div>", i("folderPlus"));
            }
            out.push_str("</div>");
        }
        let _ = write!(
            out,
            "</div><div class=\"ws-foot\"><button class=\"btn primary block\" data-a=\"cmd:newWorkspace\">{}New Workspace</button><p class=\"hint\">A workspace is a set of folders plus the tabs you had open. Switch between them to jump between projects.</p></div>",
            i("plus")
        );
        out
    }

    /* ───────────── tabs, actions, breadcrumbs, status ───────────── */

    pub fn view_tabs(&self) -> String {
        let mut names = std::collections::HashMap::new();
        for k in &self.tabs {
            if let Some(d) = self.docs.get(k) {
                *names.entry(d.name.clone()).or_insert(0) += 1;
            }
        }
        let mut out = String::new();
        for k in &self.tabs {
            let Some(d) = self.docs.get(k) else { continue };
            let mut cls = String::from("tab");
            if self.active.as_deref() == Some(k) {
                cls.push_str(" active");
            }
            if d.is_dirty() {
                cls.push_str(" dirty");
            }
            if d.preview {
                cls.push_str(" preview");
            }
            if d.conflict {
                cls.push_str(" conflict");
            }
            let desc = match &d.path {
                Some(p) if names.get(&d.name).copied().unwrap_or(0) > 1 => format!("<span class=\"desc\">{}</span>", esc(&basename(&dirname(p)))),
                _ => String::new(),
            };
            let key = a(k);
            let _ = write!(
                out,
                "<div class=\"{cls}\" role=\"tab\" title=\"{}\" data-a=\"tab:{key}\" data-dbl=\"pin:{key}\" data-aux=\"close:{key}\" data-ctx=\"tab:{key}\" data-drag=\"tab:{key}\" data-drop=\"tab:{key}\">{}<span class=\"label\">{}</span>{desc}<button class=\"tab-close\" title=\"Close (⌘W)\" data-a=\"close:{key}\">{}{}</button></div>",
                a(&d.path.as_deref().map(|p| self.tilde(p)).unwrap_or_else(|| d.name.clone())),
                doc_icon(d),
                esc(&d.name),
                i("x"),
                icon("dot", "dirty-dot")
            );
        }
        out
    }

    pub fn view_editor_actions(&self) -> String {
        let Some(d) = self.doc().filter(|d| d.kind == Kind::Md) else { return String::new() };
        let mut out = String::from("<div class=\"segmented\" role=\"group\" aria-label=\"View mode\">");
        for (m, ic, label, key) in [(Mode::Read, "edit", "Live", "⌘1"), (Mode::Split, "columns", "Split", "⌘2"), (Mode::Source, "code", "Source", "⌘3")] {
            let _ = write!(out, "<button class=\"{}\" title=\"{label} ({key})\" data-a=\"mode:{}\">{}<span>{label}</span></button>", if d.mode == m { "on" } else { "" }, m.as_str(), i(ic));
        }
        let _ = write!(out, "</div><button class=\"icon-btn\" title=\"More Actions\" data-a=\"moremenu\">{}</button>", i("more"));
        out
    }

    pub fn view_breadcrumbs(&mut self) -> Option<String> {
        let d = self.doc()?;
        if d.kind != Kind::Md {
            return Some(format!("<span class=\"crumb\">{}{}</span>", i(if d.kind == Kind::Settings { "settings" } else { "keyboard" }), esc(&d.name)));
        }
        let key = d.key.clone();
        let mut out = String::new();
        let sep = format!("<span class=\"sep\">{}</span>", i("chevron"));
        match d.path.clone() {
            Some(p) => {
                let root = self.root_of(&p);
                let mut parts: Vec<(String, Option<String>, bool, bool)> = vec![];
                match &root {
                    Some(r) => {
                        parts.push((basename(r), Some(r.clone()), true, false));
                        let rel: Vec<&str> = p[r.len() + 1..].split('/').collect();
                        let mut acc = r.clone();
                        for (k, seg) in rel.iter().enumerate() {
                            acc = format!("{acc}/{seg}");
                            parts.push((seg.to_string(), Some(acc.clone()), k < rel.len() - 1, k == rel.len() - 1));
                        }
                    }
                    None => {
                        let t = self.tilde(&p);
                        let segs: Vec<&str> = t.split('/').filter(|s| !s.is_empty()).collect();
                        for (k, seg) in segs.iter().enumerate() {
                            parts.push((seg.to_string(), None, k < segs.len() - 1, k == segs.len() - 1));
                        }
                    }
                }
                for (k, (label, path, folder, file)) in parts.iter().enumerate() {
                    if k > 0 {
                        out.push_str(&sep);
                    }
                    let ic = if *file { file_icon(label, false, false) } else if *folder && k == 0 && root.is_some() { icon("folder", "fi-folder") } else { String::new() };
                    let act = match path {
                        Some(p) if *folder => format!("crumb:{}", a(p)),
                        _ => "cmd:gotoHeading".into(),
                    };
                    let _ = write!(out, "<button class=\"crumb{}\" data-a=\"{act}\">{ic}{}</button>", if *file { " file" } else { "" }, esc(label));
                }
            }
            None => {
                let _ = write!(out, "<button class=\"crumb file\" data-a=\"cmd:gotoHeading\">{}{}</button>", file_icon(&d.name, false, false), esc(&d.name));
            }
        }
        let cur = self.docs[&key].cur_heading;
        let r = self.rendered(&key);
        if let Some(c) = cur.filter(|&c| c < r.headings.len()) {
            let mut chain = vec![];
            let mut depth = 99;
            for k in (0..=c).rev() {
                if r.headings[k].depth < depth {
                    chain.insert(0, k);
                    depth = r.headings[k].depth;
                }
            }
            for k in chain {
                let _ = write!(out, "{sep}<button class=\"crumb heading\" data-a=\"heading:{k}\">{}{}</button>", i("hash"), esc(&r.headings[k].text));
            }
        }
        Some(out)
    }

    pub fn view_status(&mut self) -> (String, String) {
        let mut left = format!("<button class=\"sb-item ws\" title=\"Switch Workspace (⌃⌘R)\" data-a=\"cmd:switchWorkspace\">{}{}</button>", i("layers"), esc(&self.ws().name));
        let dc = self.dirty_keys().len();
        let _ = write!(
            left,
            "<span class=\"sb-item{}\" title=\"{}\">{}</span>",
            if dc > 0 { " warn" } else { "" },
            if dc > 0 { "Unsaved files" } else { "All changes saved" },
            if dc > 0 { format!("{}{dc} unsaved", i("dot")) } else { format!("{}Saved", i("check")) }
        );
        if self.doc().is_some_and(|d| d.conflict) {
            let _ = write!(left, "<span class=\"sb-item warn\">{}Changed on disk</span>", i("warning"));
        }
        let mut right = String::new();
        if let Some(key) = self.md_key() {
            let r = self.rendered(&key);
            let d = &self.docs[&key];
            let cursor = if d.mode == Mode::Read {
                plural(d.text.split('\n').count(), "line")
            } else {
                let (l, c, s) = d.cursor;
                format!("Ln {}, Col {c}{}", l + 1, if s > 0 { format!(" ({s} selected)") } else { String::new() })
            };
            let _ = write!(
                right,
                "<span class=\"sb-item\">{cursor}</span><span class=\"sb-item\" title=\"Words · reading time\">{} · {} min read</span><button class=\"sb-item\" title=\"Change view mode\" data-a=\"cmd:cycleMode\">{}</button><span class=\"sb-item\">{}</span><span class=\"sb-item\">UTF-8</span>",
                plural(r.words, "word"),
                ((r.words as f64) / 230.0).round().max(1.0),
                d.mode.label(),
                if is_markdown(&d.name) { "Markdown" } else { "Plain Text" }
            );
        }
        let _ = write!(right, "<button class=\"sb-item\" title=\"Theme\" data-a=\"cmd:pickTheme\">{}</button>", i(if theme(&self.set().theme).dark { "moon" } else { "sun" }));
        (left, right)
    }

    /* ───────────── welcome ───────────── */

    pub fn view_welcome(&self) -> String {
        let w = self.ws();
        let action = |ic: &str, label: &str, kbd: &str, act: &str| {
            format!("<button class=\"w-action\" data-a=\"{}\">{}<span>{label}</span>{}</button>", a(act), i(ic), if kbd.is_empty() { String::new() } else { format!("<kbd>{kbd}</kbd>") })
        };
        let mut others: Vec<_> = self.s.workspaces.iter().filter(|x| x.id != w.id).collect();
        others.sort_by(|x, y| y.opened.partial_cmp(&x.opened).unwrap_or(std::cmp::Ordering::Equal));
        let others: String = if others.is_empty() {
            "<p class=\"muted\">Workspaces you create show up here.</p>".into()
        } else {
            others.iter().take(4).map(|o| action("layers", &esc(&o.name), "", &format!("ws:{}", o.id))).collect()
        };
        let recent: Vec<&String> = self.s.recent.iter().filter(|p| !self.docs.contains_key(*p)).take(7).collect();
        let recent: String = if recent.is_empty() {
            "<p class=\"muted\">Files you open will be listed here.</p>".into()
        } else {
            recent
                .iter()
                .map(|p| format!("<button class=\"w-recent\" title=\"{}\" data-a=\"open:{}\">{}<span class=\"label\">{}</span><span class=\"desc\">{}</span></button>", a(&self.tilde(p)), a(p), file_icon(p, false, false), esc(&basename(p)), esc(&self.tilde(&dirname(p)))))
                .collect()
        };
        format!(
            "<div class=\"w-inner\"><div class=\"w-hero\"><div class=\"w-logo\"><i></i></div><div><h1>Folio</h1><p>Read Markdown beautifully. Click anything to edit it in place.</p></div></div><div class=\"w-grid\"><div class=\"w-col\"><h2>Start</h2>{}{}{}{}{}<h2>Workspaces</h2>{others}</div><div class=\"w-col\"><h2>Recent</h2>{recent}<h2>Good to know</h2><ul class=\"w-tips\"><li><kbd>⌘P</kbd> jumps to any file in the workspace. Type <kbd>@</kbd> for headings, <kbd>&gt;</kbd> for commands.</li><li>In <b>Live</b> mode, click any paragraph, heading, list or table to edit its Markdown right there. <kbd>Esc</kbd> to finish.</li><li><kbd>⌘1</kbd> <kbd>⌘2</kbd> <kbd>⌘3</kbd> switch between Live, Split and Source.</li><li>Check boxes in the reader to tick tasks off. The Markdown updates for you.</li><li>Drop files or folders from Finder anywhere on the window.</li></ul></div></div></div>",
            action("filePlus", "New File", "⌘N", "cmd:newFile"),
            action("files", "Open…", "⌘O", "cmd:open"),
            action("folderPlus", &format!("Add Folder to “{}”", esc(&w.name)), "⌥⌘O", "cmd:addFolder"),
            action("layers", "New Workspace…", "⌃⌘N", "cmd:newWorkspace"),
            action("command", "Command Palette", "⇧⌘P", "cmd:palette"),
        )
    }

    /* ───────────── settings & shortcuts ───────────── */

    pub fn view_settings(&self, n: u32) -> String {
        let st = serde_json::to_value(self.set()).unwrap_or_default();
        let q = self.settings_query.to_lowercase();
        let themes: Vec<(&str, &str)> = THEMES.iter().map(|t| (t.id, t.label)).collect();
        let fonts = [("sans", "San Francisco"), ("serif", "New York"), ("charter", "Charter"), ("iowan", "Iowan Old Style"), ("mono", "SF Mono")];
        let modes = [("read", "Live (edit in place)"), ("split", "Split"), ("source", "Source")];
        enum C<'a> {
            Sel(&'a [(&'a str, &'a str)]),
            Range(f64, f64, f64, &'a str),
            Toggle,
        }
        let groups: Vec<(&str, Vec<(&str, &str, C)>)> = vec![
            ("Appearance", vec![
                ("theme", "Color theme", C::Sel(&themes)),
                ("readerFont", "Reader typeface", C::Sel(&fonts)),
                ("readerSize", "Reader text size", C::Range(12.0, 24.0, 1.0, "px")),
                ("readerWidth", "Reading column width", C::Range(560.0, 1200.0, 20.0, "px")),
                ("lineHeight", "Reader line height", C::Range(1.4, 2.1, 0.05, "")),
            ]),
            ("Editor", vec![
                ("editorSize", "Source font size", C::Range(10.0, 20.0, 1.0, "px")),
                ("wrap", "Wrap long lines", C::Toggle),
                ("lineNumbers", "Show line numbers", C::Toggle),
                ("syncScroll", "Sync scrolling in Split view", C::Toggle),
                ("defaultMode", "Open Markdown files in", C::Sel(&modes)),
            ]),
            ("Workbench", vec![
                ("previewTabs", "Single-click opens a preview tab (italic) that the next file replaces", C::Toggle),
                ("restoreTabs", "Restore open tabs when switching workspaces or relaunching", C::Toggle),
                ("showAllFiles", "Show all files in the explorer, not just Markdown", C::Toggle),
                ("autoReload", "Reload files automatically when they change on disk", C::Toggle),
            ]),
            ("Privacy", vec![("remoteImages", "Load remote images (https) in previews", C::Toggle)]),
        ];
        let mut out = format!(
            "<div class=\"settings\"><h1>Settings</h1><div class=\"set-search\"><input id=\"setq-{n}\" placeholder=\"Search settings\" value=\"{}\" spellcheck=\"false\" data-i=\"setq\"></div>",
            a(&self.settings_query)
        );
        for (group, items) in &groups {
            let mut sec = String::new();
            for (key, label, c) in items {
                if !q.is_empty() && !label.to_lowercase().contains(&q) && !group.to_lowercase().contains(&q) {
                    continue;
                }
                let v = &st[*key];
                let control = match c {
                    C::Toggle => {
                        let on = v.as_bool().unwrap_or(false);
                        format!("<button class=\"switch{}\" role=\"switch\" aria-checked=\"{on}\" data-a=\"set:{key}\"><i></i></button>", if on { " on" } else { "" })
                    }
                    C::Sel(opts) => {
                        let cur = v.as_str().unwrap_or("");
                        let o: String = opts.iter().map(|(k, l)| format!("<option value=\"{k}\"{}>{l}</option>", if *k == cur { " selected" } else { "" })).collect();
                        format!("<select data-i=\"set:{key}\">{o}</select>")
                    }
                    C::Range(min, max, step, unit) => {
                        let x = v.as_f64().unwrap_or(0.0);
                        format!("<div class=\"range\"><input type=\"range\" min=\"{min}\" max=\"{max}\" step=\"{step}\" value=\"{x}\" data-i=\"setr:{key}\"><output id=\"out-{key}\">{}{unit}</output></div>", fmt_num(x))
                    }
                };
                let _ = write!(sec, "<div class=\"set-row\"><label>{label}<small>{key}</small></label>{control}</div>");
            }
            if !sec.is_empty() {
                let _ = write!(out, "<section><h2>{group}</h2>{sec}</section>");
            }
        }
        out.push_str("<p class=\"muted small\">Settings are stored on this Mac in ~/Library/Application Support/Folio.</p></div>");
        out
    }

    pub fn view_shortcuts(&self) -> String {
        let mut groups: Vec<(&str, String)> = vec![];
        for c in COMMANDS.iter().filter(|c| !c.key.is_empty()) {
            let row = format!("<div class=\"set-row\"><label>{}</label><span class=\"keys\">{}</span></div>", esc(c.title), keys(c.key));
            match groups.iter_mut().find(|g| g.0 == c.group) {
                Some(g) => g.1.push_str(&row),
                None => groups.push((c.group, row)),
            }
        }
        let body: String = groups.iter().map(|(g, rows)| format!("<section><h2>{g}</h2>{rows}</section>")).collect();
        format!("<div class=\"settings shortcuts\"><h1>Keyboard Shortcuts</h1>{body}</div>")
    }

    /* ───────────── palette & menus ───────────── */

    pub fn view_palette(&self) -> String {
        let mut out = String::new();
        for (k, x) in self.pal.items.iter().enumerate() {
            out.push_str(&pal_row(k, x, self.pal.sel == Some(k)));
        }
        out
    }

    pub fn view_find_toggles(&self) -> String {
        let f = &self.find;
        format!(
            "<button class=\"{}\" title=\"Match Case\" data-a=\"fopt:case\">Aa</button><button class=\"{}\" title=\"Match Whole Word\" data-a=\"fopt:word\"><u>ab</u></button><button class=\"{}\" title=\"Use Regular Expression\" data-a=\"fopt:regex\">.*</button>",
            if f.case { "on" } else { "" },
            if f.word { "on" } else { "" },
            if f.regex { "on" } else { "" }
        )
    }
}

fn pal_row(k: usize, x: &PalItem, sel: bool) -> String {
    let mut cls = String::from("pal-item");
    if sel {
        cls.push_str(" sel");
    }
    if x.passive {
        cls.push_str(" passive");
    }
    format!(
        "<div class=\"{cls}\" role=\"option\" style=\"--indent:{}\"{}>{}<span class=\"label\">{}</span>{}{}{}</div>",
        x.indent,
        if x.passive { String::new() } else { format!(" data-a=\"pal:{k}\"") },
        icon(x.icon, if x.icon == "md" { "fi-md" } else { "" }),
        x.label_html.clone().unwrap_or_else(|| esc(&x.label)),
        if x.detail.is_empty() { String::new() } else { format!("<span class=\"desc\">{}</span>", esc(&x.detail)) },
        if x.tag.is_empty() { String::new() } else { format!("<span class=\"tag\">{}</span>", x.tag) },
        if x.kbd.is_empty() { String::new() } else { format!("<span class=\"keys\">{}</span>", keys(&x.kbd)) }
    )
}

pub fn menu_html(items: &[Option<(String, String, bool)>]) -> String {
    let mut out = String::new();
    let mut last_sep = true;
    for (k, it) in items.iter().enumerate() {
        match it {
            None => {
                if !last_sep {
                    out.push_str("<div class=\"sep\"></div>");
                    last_sep = true;
                }
            }
            Some((label, kbd, danger)) => {
                let _ = write!(
                    out,
                    "<button class=\"mi{}\" data-a=\"mi:{k}\"><span>{}</span>{}</button>",
                    if *danger { " danger" } else { "" },
                    esc(label),
                    if kbd.is_empty() { String::new() } else { format!("<kbd>{}</kbd>", esc(kbd)) }
                );
                last_sep = false;
            }
        }
    }
    if out.ends_with("<div class=\"sep\"></div>") {
        out.truncate(out.len() - "<div class=\"sep\"></div>".len());
    }
    out
}

pub fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{}", x as i64)
    } else {
        let s = format!("{x:.2}");
        s.trim_end_matches('0').to_string()
    }
}
