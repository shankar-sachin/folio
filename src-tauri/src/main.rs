// Folio — native host (Tauri). The workbench UI lives in ../web; this file is the
// file-system, dialog, menu and window layer it talks to through one `api` command.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use regex::RegexBuilder;
use serde_json::{json, Map, Value};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, UNIX_EPOCH},
};
use tauri::{
    http,
    menu::{AboutMetadataBuilder, Menu, MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder},
    AppHandle, DragDropEvent, Manager, RunEvent, WebviewWindow, WindowEvent, Wry,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult};

const MD_EXT: &[&str] = &["md", "markdown", "mdown", "mkd", "mdx", "txt"];
const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", ".svn", ".hg", "build", "dist", ".build", "DerivedData", "Pods", ".next", ".venv", "venv",
    "__pycache__", ".cache", "target",
];
const MAX_FILE: u64 = 16_000_000;

#[derive(Default)]
struct AppState {
    allowed: Mutex<HashSet<PathBuf>>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    fs_pending: Arc<AtomicBool>,
    dirty: AtomicUsize,
    force_close: AtomicBool,
    ready: AtomicBool,
    pending: Mutex<Vec<String>>,
}

/* ───────────── helpers ───────────── */

fn js(app: &AppHandle, code: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.eval(&format!("window.folio && {code}"));
    }
}
fn js_paths(app: &AppHandle, paths: &[String]) {
    js(app, &format!("window.folio.openPaths({})", json!(paths)));
}
fn mtime(p: &Path) -> Option<f64> {
    fs::metadata(p).ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs_f64())
}
fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}
fn state_file() -> PathBuf {
    let dir = home().join("Library/Application Support/Folio");
    let _ = fs::create_dir_all(&dir);
    dir.join(std::env::var("FOLIO_STATE_FILE").unwrap_or_else(|_| "state.json".into()))
}
fn ext_of(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}
fn name_of(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}
fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
fn strs(v: &Value, k: &str) -> Vec<String> {
    v.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(v) = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/* ───────────── file system ───────────── */

fn tree(root: &Path, all: bool) -> Value {
    fn walk(dir: &Path, all: bool, depth: usize, budget: &mut i64) -> Vec<Value> {
        if depth > 24 || *budget <= 0 {
            return vec![];
        }
        let Ok(rd) = fs::read_dir(dir) else { return vec![] };
        let mut dirs = vec![];
        let mut files = vec![];
        for entry in rd.flatten() {
            *budget -= 1;
            if *budget <= 0 {
                break;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(ft) = entry.file_type() else { continue };
            let path = entry.path();
            if ft.is_dir() {
                if SKIP_DIRS.contains(&name.as_str()) || name.ends_with(".app") {
                    continue;
                }
                let children = walk(&path, all, depth + 1, budget);
                if all || !children.is_empty() {
                    dirs.push((name.to_lowercase(), json!({"name": name, "path": path, "dir": true, "children": children})));
                }
            } else if ft.is_file() && (all || MD_EXT.contains(&ext_of(&path).as_str())) {
                files.push((name.to_lowercase(), json!({"name": name, "path": path, "dir": false})));
            }
        }
        let natural = |a: &(String, Value), b: &(String, Value)| natord(&a.0, &b.0);
        dirs.sort_by(natural);
        files.sort_by(natural);
        dirs.into_iter().chain(files).map(|x| x.1).collect()
    }
    if !root.is_dir() {
        return json!({"name": name_of(root), "path": root, "dir": true, "missing": true, "children": []});
    }
    let mut budget = 20_000i64;
    let children = walk(root, all, 0, &mut budget);
    json!({"name": name_of(root), "path": root, "dir": true, "children": children, "truncated": budget <= 0})
}

/// Natural ordering so "note 2" sorts before "note 10".
fn natord(a: &str, b: &str) -> std::cmp::Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ai.peek().copied().filter(char::is_ascii_digit) { na.push(c); ai.next(); }
                let mut nb = String::new();
                while let Some(c) = bi.peek().copied().filter(char::is_ascii_digit) { nb.push(c); bi.next(); }
                let (na, nb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = na.len().cmp(&nb.len()).then(na.cmp(nb));
                if o != std::cmp::Ordering::Equal { return o; }
            }
            (Some(x), Some(y)) => {
                if x != y { return x.cmp(&y); }
                ai.next();
                bi.next();
            }
        }
    }
}

fn markdown_files(roots: &[String]) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        if out.len() > 20_000 { return; }
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') { continue; }
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() {
                if !SKIP_DIRS.contains(&name.as_str()) && !name.ends_with(".app") { walk(&p, out); }
            } else if ft.is_file() && MD_EXT.contains(&ext_of(&p).as_str()) {
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
    let mut out = vec![];
    for r in roots { walk(Path::new(r), &mut out); }
    out
}

fn search(roots: &[String], query: &str, regex: bool, case: bool, word: bool) -> Value {
    let mut pat = if regex { query.to_string() } else { regex::escape(query) };
    if word { pat = format!(r"\b(?:{pat})\b"); }
    let Ok(re) = RegexBuilder::new(&pat).case_insensitive(!case).build() else {
        return json!({"error": "Invalid regular expression"});
    };
    let mut results = vec![];
    let mut total = 0usize;
    for file in markdown_files(roots) {
        if total >= 5_000 { break; }
        let p = Path::new(&file);
        if fs::metadata(p).map(|m| m.len() > MAX_FILE).unwrap_or(true) { continue; }
        let Ok(text) = fs::read_to_string(p) else { continue };
        let mut matches = vec![];
        for (i, line) in text.lines().enumerate() {
            for m in re.find_iter(line).take(20) {
                if m.start() == m.end() { continue; }
                let clipped: String = line.chars().take(400).collect();
                matches.push(json!({"line": i + 1, "text": clipped, "start": utf16_len(&line[..m.start()]), "length": utf16_len(m.as_str())}));
            }
            if matches.len() >= 400 { break; }
        }
        if !matches.is_empty() {
            total += matches.len();
            results.push(json!({"path": file, "matches": matches}));
        }
    }
    json!({"results": results, "total": total})
}

fn trash(path: &Path) -> Result<(), String> {
    let bin = home().join(".Trash");
    let mut target = bin.join(name_of(path));
    if target.exists() {
        let stem = path.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        let ext = path.extension().map(|x| format!(".{}", x.to_string_lossy())).unwrap_or_default();
        let stamp = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        target = bin.join(format!("{stem} {stamp}{ext}"));
    }
    if fs::rename(path, &target).is_ok() { return Ok(()); }
    // Different volume: let Finder move it to that volume's Trash.
    let script = format!("tell application \"Finder\" to delete POSIX file \"{}\"", path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\""));
    let ok = Command::new("osascript").args(["-e", &script]).status().map(|s| s.success()).unwrap_or(false);
    if ok { Ok(()) } else { Err("Couldn’t move the item to the Trash".into()) }
}

fn set_roots(app: &AppHandle, roots: Vec<String>) {
    let state = app.state::<AppState>();
    {
        let mut allowed = state.allowed.lock().unwrap();
        for r in &roots { allowed.insert(PathBuf::from(r)); }
    }
    let handle = app.clone();
    let pending = state.fs_pending.clone();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        if matches!(ev.kind, notify::EventKind::Access(_)) { return; }
        if pending.swap(true, Ordering::SeqCst) { return; }
        let (h, p) = (handle.clone(), pending.clone());
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            p.store(false, Ordering::SeqCst);
            js(&h, "window.folio.fsChanged()");
        });
    });
    let mut slot = state.watcher.lock().unwrap();
    *slot = None;
    if let Ok(mut w) = watcher {
        use notify::Watcher;
        for r in &roots { let _ = w.watch(Path::new(r), notify::RecursiveMode::Recursive); }
        *slot = Some(w);
    }
}

fn allowed(app: &AppHandle, p: &Path) -> bool {
    let p = fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    app.state::<AppState>().allowed.lock().unwrap().iter().any(|root| {
        let r = fs::canonicalize(root).unwrap_or_else(|_| root.clone());
        p.starts_with(r)
    })
}

/* ───────────── dialogs ───────────── */

fn confirm(app: &AppHandle, window: &WebviewWindow, p: &Value) -> Value {
    let labels = strs(p, "buttons");
    let buttons = match labels.len() {
        0 | 1 => MessageDialogButtons::OkCustom(labels.first().cloned().unwrap_or_else(|| "OK".into())),
        2 => MessageDialogButtons::OkCancelCustom(labels[0].clone(), labels[1].clone()),
        _ => MessageDialogButtons::YesNoCancelCustom(labels[0].clone(), labels[2].clone(), labels[1].clone()),
    };
    let r = app.dialog().message(s(p, "detail")).title(s(p, "message"))
        .kind(if b(p, "warning") { MessageDialogKind::Warning } else { MessageDialogKind::Info })
        .buttons(buttons).parent(window).blocking_show_with_result();
    let idx = match r {
        MessageDialogResult::Custom(t) => labels.iter().position(|l| *l == t).unwrap_or(1),
        MessageDialogResult::Yes | MessageDialogResult::Ok => 0,
        MessageDialogResult::No => 2,
        MessageDialogResult::Cancel => 1,
    };
    json!(idx)
}

/* ───────────── the one command the UI calls ───────────── */

#[tauri::command]
async fn api(app: AppHandle, window: WebviewWindow, action: String, payload: Value) -> Result<Value, String> {
    let p = &payload;
    let state = app.state::<AppState>();
    let blocking = |f: Box<dyn FnOnce() -> Value + Send>| async move { tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string()) };
    match action.as_str() {
        "ready" => {
            state.ready.store(true, Ordering::SeqCst);
            let paths: Vec<String> = state.pending.lock().unwrap().drain(..).collect();
            Ok(json!({"paths": paths, "home": home(), "testing": std::env::args().any(|a| a == "--self-test")}))
        }
        "loadState" => Ok(fs::read_to_string(state_file()).map(Value::String).unwrap_or(Value::Null)),
        "saveState" => {
            let tmp = state_file().with_extension("json.tmp");
            fs::write(&tmp, s(p, "json")).and_then(|_| fs::rename(&tmp, state_file())).map_err(|e| e.to_string())?;
            Ok(json!(true))
        }
        "roots" => { set_roots(&app, strs(p, "roots")); Ok(json!(true)) }
        "allowDir" => { state.allowed.lock().unwrap().insert(PathBuf::from(s(p, "path"))); Ok(json!(true)) }
        "tree" => {
            let (root, all) = (PathBuf::from(s(p, "root")), b(p, "all"));
            blocking(Box::new(move || tree(&root, all))).await
        }
        "listFiles" => {
            let roots = strs(p, "roots");
            blocking(Box::new(move || json!(markdown_files(&roots)))).await
        }
        "search" => {
            let (roots, q, rx, cs, w) = (strs(p, "roots"), s(p, "query").to_string(), b(p, "regex"), b(p, "case"), b(p, "word"));
            blocking(Box::new(move || search(&roots, &q, rx, cs, w))).await
        }
        "read" => {
            let path = PathBuf::from(s(p, "path"));
            let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
            if meta.is_dir() { return Err("That’s a folder".into()); }
            if meta.len() > MAX_FILE { return Err("File is larger than 16 MB".into()); }
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            if let Some(dir) = path.parent() { state.allowed.lock().unwrap().insert(dir.to_path_buf()); }
            Ok(json!({"text": String::from_utf8_lossy(&bytes), "mtime": mtime(&path).unwrap_or(0.0)}))
        }
        "write" => {
            let path = PathBuf::from(s(p, "path"));
            let tmp = path.with_file_name(format!(".{}.folio-tmp", name_of(&path)));
            fs::write(&tmp, s(p, "text")).and_then(|_| fs::rename(&tmp, &path)).map_err(|e| { let _ = fs::remove_file(&tmp); e.to_string() })?;
            Ok(json!({"mtime": mtime(&path).unwrap_or(0.0)}))
        }
        "stat" => {
            let mut out = Map::new();
            for x in strs(p, "paths") {
                let v = fs::metadata(&x).ok().map(|m| json!({"m": mtime(Path::new(&x)).unwrap_or(0.0), "dir": m.is_dir()})).unwrap_or(Value::Null);
                out.insert(x, v);
            }
            Ok(Value::Object(out))
        }
        "create" => {
            let path = PathBuf::from(s(p, "path"));
            if path.exists() { return Err(format!("“{}” already exists", name_of(&path))); }
            if b(p, "dir") { fs::create_dir_all(&path) } else { fs::write(&path, s(p, "text")) }.map_err(|e| e.to_string())?;
            Ok(json!(true))
        }
        "rename" => {
            let to = PathBuf::from(s(p, "to"));
            if to.exists() { return Err(format!("“{}” already exists", name_of(&to))); }
            fs::rename(s(p, "from"), &to).map_err(|e| e.to_string())?;
            Ok(json!(true))
        }
        "trash" => { trash(Path::new(s(p, "path")))?; Ok(json!(true)) }
        "reveal" => { let _ = Command::new("open").args(["-R", s(p, "path")]).spawn(); Ok(json!(true)) }
        "openWith" => { let _ = Command::new("open").arg(s(p, "path")).spawn(); Ok(json!(true)) }
        "external" => {
            let url = s(p, "url");
            if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") { let _ = Command::new("open").arg(url).spawn(); }
            Ok(json!(true))
        }
        "copy" => {
            let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
            child.stdin.take().unwrap().write_all(s(p, "text").as_bytes()).map_err(|e| e.to_string())?;
            let _ = child.wait();
            Ok(json!(true))
        }
        "openDialog" => {
            let mut d = app.dialog().file().set_parent(&window).set_can_create_directories(true);
            if !s(p, "message").is_empty() { d = d.set_title(s(p, "message")); }
            let picked = if b(p, "folders") { d.blocking_pick_folders() } else { d.add_filter("Markdown", &["md", "markdown", "mdown", "mkd", "mdx", "txt"]).blocking_pick_files() };
            Ok(json!(picked.unwrap_or_default().into_iter().filter_map(|f| f.into_path().ok()).collect::<Vec<_>>()))
        }
        "saveDialog" => {
            let mut d = app.dialog().file().set_parent(&window).set_can_create_directories(true).set_file_name(s(p, "name"));
            if !s(p, "dir").is_empty() { d = d.set_directory(s(p, "dir")); }
            Ok(d.blocking_save_file().and_then(|f| f.into_path().ok()).map(|x| json!(x)).unwrap_or(Value::Null))
        }
        "confirm" => Ok(confirm(&app, &window, p)),
        "window" => {
            if let Some(t) = p.get("title").and_then(Value::as_str) { let _ = window.set_title(t); }
            if let Some(d) = p.get("dirty").and_then(Value::as_u64) { state.dirty.store(d as usize, Ordering::SeqCst); }
            if let Some(dark) = p.get("dark").and_then(Value::as_bool) { let _ = window.set_theme(Some(if dark { tauri::Theme::Dark } else { tauri::Theme::Light })); }
            if let Some(bg) = p.get("bg").and_then(Value::as_array) {
                let c: Vec<u8> = bg.iter().map(|x| (x.as_f64().unwrap_or(0.0) * 255.0).round() as u8).collect();
                if c.len() == 3 { let _ = window.set_background_color(Some(tauri::window::Color(c[0], c[1], c[2], 255))); }
            }
            Ok(json!(true))
        }
        "quit" => { state.force_close.store(true, Ordering::SeqCst); app.exit(0); Ok(json!(true)) }
        "testResult" => {
            let out = std::env::var("FOLIO_TEST_OUTPUT").unwrap_or_else(|_| "/tmp/folio-test.json".into());
            let mut v: Value = serde_json::from_str(s(p, "json")).unwrap_or(json!({}));
            if let (Ok(pos), Ok(size), Ok(scale)) = (window.outer_position(), window.outer_size(), window.scale_factor()) {
                v["frame"] = json!([pos.x as f64 / scale, pos.y as f64 / scale, size.width as f64 / scale, size.height as f64 / scale]);
            }
            fs::write(out, v.to_string()).map_err(|e| e.to_string())?;
            Ok(json!(true))
        }
        other => Err(format!("unknown action {other}")),
    }
}

/* ───────────── menu ───────────── */

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, label: &str, accel: &str| -> tauri::Result<MenuItem<Wry>> {
        let mut b = MenuItemBuilder::with_id(format!("cmd:{id}"), label);
        if !accel.is_empty() { b = b.accelerator(accel); }
        b.build(app)
    };
    let about = AboutMetadataBuilder::new().name(Some("Folio")).version(Some("2.0.0")).comments(Some("A live-edit Markdown reader.")).build();
    let app_menu = SubmenuBuilder::new(app, "Folio")
        .about(Some(about)).separator()
        .item(&item("settings", "Settings…", "CmdOrCtrl+Comma")?).separator()
        .services().separator()
        .hide().hide_others().show_all().separator()
        .item(&MenuItemBuilder::with_id("quit", "Quit Folio").accelerator("CmdOrCtrl+Q").build(app)?)
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&item("newFile", "New File", "CmdOrCtrl+N")?)
        .item(&item("newFileInFolder", "New File in Workspace…", "Alt+CmdOrCtrl+N")?).separator()
        .item(&item("open", "Open File…", "CmdOrCtrl+O")?)
        .item(&item("addFolder", "Add Folder to Workspace…", "Alt+CmdOrCtrl+O")?)
        .item(&item("newWorkspace", "New Workspace…", "Control+CmdOrCtrl+N")?)
        .item(&item("switchWorkspace", "Switch Workspace…", "Control+CmdOrCtrl+R")?).separator()
        .item(&item("save", "Save", "CmdOrCtrl+S")?)
        .item(&item("saveAs", "Save As…", "Shift+CmdOrCtrl+S")?)
        .item(&item("saveAll", "Save All", "Alt+CmdOrCtrl+S")?).separator()
        .item(&item("exportHtml", "Export as HTML…", "")?)
        .item(&item("revert", "Revert File", "")?)
        .item(&item("reveal", "Reveal in Finder", "Alt+CmdOrCtrl+R")?).separator()
        .item(&item("closeTab", "Close Tab", "CmdOrCtrl+W")?)
        .item(&item("reopenTab", "Reopen Closed Tab", "Shift+CmdOrCtrl+T")?)
        .item(&item("closeAll", "Close All Tabs", "Alt+CmdOrCtrl+W")?)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo().redo().separator().cut().copy().paste().select_all().separator()
        .item(&item("find", "Find", "CmdOrCtrl+F")?)
        .item(&item("findNext", "Find Next", "CmdOrCtrl+G")?)
        .item(&item("findPrev", "Find Previous", "Shift+CmdOrCtrl+G")?)
        .item(&item("searchView", "Find in Workspace", "Shift+CmdOrCtrl+F")?).separator()
        .item(&item("bold", "Bold", "CmdOrCtrl+B")?)
        .item(&item("italic", "Italic", "CmdOrCtrl+I")?)
        .item(&item("link", "Insert Link", "CmdOrCtrl+K")?)
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&item("palette", "Command Palette…", "Shift+CmdOrCtrl+P")?)
        .item(&item("quickOpen", "Go to File…", "CmdOrCtrl+P")?)
        .item(&item("gotoHeading", "Go to Heading…", "Shift+CmdOrCtrl+O")?).separator()
        .item(&item("explorerView", "Explorer", "Shift+CmdOrCtrl+E")?)
        .item(&item("outlineView", "Outline", "Shift+CmdOrCtrl+U")?)
        .item(&item("workspacesView", "Workspaces", "Shift+CmdOrCtrl+K")?)
        .item(&item("toggleSidebar", "Toggle Sidebar", "CmdOrCtrl+Backslash")?).separator()
        .item(&item("read", "Live", "CmdOrCtrl+1")?)
        .item(&item("split", "Split", "CmdOrCtrl+2")?)
        .item(&item("source", "Source", "CmdOrCtrl+3")?)
        .item(&item("togglePreview", "Toggle Live / Source", "Shift+CmdOrCtrl+V")?).separator()
        .item(&item("zoomIn", "Bigger Text", "CmdOrCtrl+Equal")?)
        .item(&item("zoomOut", "Smaller Text", "CmdOrCtrl+Minus")?)
        .item(&item("zoomReset", "Actual Size", "CmdOrCtrl+0")?)
        .item(&item("wrap", "Toggle Word Wrap", "Alt+Z")?)
        .item(&item("zen", "Zen Mode", "Control+CmdOrCtrl+J")?)
        .item(&item("theme", "Toggle Light / Dark", "Alt+CmdOrCtrl+T")?).separator()
        .fullscreen()
        .build()?;
    let go = SubmenuBuilder::new(app, "Go")
        .item(&item("nextTab", "Next Tab", "Shift+CmdOrCtrl+BracketRight")?)
        .item(&item("prevTab", "Previous Tab", "Shift+CmdOrCtrl+BracketLeft")?)
        .item(&item("gotoLine", "Go to Line…", "Control+G")?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window").minimize().maximize().separator().close_window().build()?;
    let help = SubmenuBuilder::new(app, "Help")
        .item(&item("shortcuts", "Keyboard Shortcuts", "CmdOrCtrl+Slash")?)
        .item(&item("welcome", "Welcome", "")?)
        .build()?;
    MenuBuilder::new(app).items(&[&app_menu, &file, &edit, &view, &go, &window, &help]).build()
}

/// Close/quit always goes through the UI so it can save state and ask about unsaved edits.
fn request_close(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.ready.load(Ordering::SeqCst) || state.force_close.load(Ordering::SeqCst) {
        state.force_close.store(true, Ordering::SeqCst);
        app.exit(0);
        return;
    }
    js(app, "window.folio.requestClose()");
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .register_uri_scheme_protocol("mdasset", |ctx, req| {
            let app = ctx.app_handle();
            let path = PathBuf::from(percent_decode(req.uri().path()));
            let types = [("png", "image/png"), ("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("gif", "image/gif"), ("webp", "image/webp"), ("svg", "image/svg+xml"), ("avif", "image/avif")];
            let mime = types.iter().find(|(e, _)| *e == ext_of(&path)).map(|x| x.1);
            match (mime, allowed(app, &path)) {
                (Some(mime), true) => match fs::read(&path) {
                    Ok(data) if data.len() < 40_000_000 => http::Response::builder().header("Content-Type", mime).header("Cache-Control", "no-cache").body(data).unwrap(),
                    _ => http::Response::builder().status(404).body(Vec::new()).unwrap(),
                },
                _ => http::Response::builder().status(403).body(Vec::new()).unwrap(),
            }
        })
        .menu(build_menu)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == "quit" { request_close(app); }
            else if let Some(cmd) = id.strip_prefix("cmd:") { js(app, &format!("window.folio.command({})", json!(cmd))); }
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    if !app.state::<AppState>().force_close.load(Ordering::SeqCst) { api.prevent_close(); request_close(app); }
                }
                WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) => {
                    let list: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                    js_paths(app, &list);
                }
                WindowEvent::Focused(true) => js(app, "window.folio.focusChanged()"),
                _ => {}
            }
        })
        .setup(|app| {
            let args: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with('-')).collect();
            let cwd = std::env::current_dir().unwrap_or_default();
            let paths = args.into_iter().map(|a| { let p = PathBuf::from(&a); if p.is_absolute() { a } else { cwd.join(p).to_string_lossy().into_owned() } });
            app.state::<AppState>().pending.lock().unwrap().extend(paths);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api])
        .build(tauri::generate_context!())
        .expect("error while building Folio");

    app.run(|app, event| match event {
        RunEvent::ExitRequested { api, .. } => {
            let state = app.state::<AppState>();
            if !state.force_close.load(Ordering::SeqCst) && state.ready.load(Ordering::SeqCst) { api.prevent_exit(); request_close(app); }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => {
            let paths: Vec<String> = urls.iter().filter_map(|u| u.to_file_path().ok()).map(|p| p.to_string_lossy().into_owned()).collect();
            let state = app.state::<AppState>();
            if state.ready.load(Ordering::SeqCst) { js_paths(app, &paths); } else { state.pending.lock().unwrap().extend(paths); }
        }
        _ => {}
    });
}
