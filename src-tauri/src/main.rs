// Folio — a live-edit Markdown reader. Rust owns all state and logic; the webview is a
// projection of `App` driven by a tiny event/patch shim (web/folio.js).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod events;
mod fsx;
mod model;
mod ui;
mod view;

use app::App;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{
    http,
    menu::{AboutMetadataBuilder, Menu, MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder},
    AppHandle, DragDropEvent, Manager, RunEvent, WindowEvent, Wry,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult};

/// Embedded for "Export as HTML".
pub const STYLE: &str = include_str!("../../web/style.css");

struct Shared {
    app: Mutex<App>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    fs_pending: Arc<AtomicBool>,
    force_close: AtomicBool,
}

/// Run `f` against the app, then push the resulting patches to the page.
pub fn with_app(h: &AppHandle, f: impl FnOnce(&mut App)) {
    let (ops, afters) = run(h, f);
    if !ops.is_empty() {
        if let Some(w) = h.get_webview_window("main") {
            let _ = w.eval(format!("window.folio && window.folio.apply({})", Value::Array(ops)));
        }
    }
    for a in afters {
        a(h);
    }
}

fn run(h: &AppHandle, f: impl FnOnce(&mut App)) -> (Vec<Value>, Vec<app::After>) {
    let shared = h.state::<Shared>();
    let mut app = shared.app.lock().unwrap_or_else(|e| e.into_inner());
    if app.handle.is_none() {
        app.handle = Some(h.clone());
    }
    f(&mut app);
    let ops = app.flush();
    (ops, std::mem::take(&mut app.after))
}

/// The page's single entry point: one event in, patch operations out.
#[tauri::command]
fn ui(h: AppHandle, ev: Value) -> Value {
    let (ops, afters) = run(&h, |app| app.handle(&ev));
    for a in afters {
        a(&h);
    }
    Value::Array(ops)
}

/* ───────────── host services used by App ───────────── */

static TIMER: std::sync::OnceLock<std::sync::mpsc::Sender<(std::time::Instant, app::Job)>> = std::sync::OnceLock::new();

/// Run `job` against the app after `ms` milliseconds (one timer thread for all of them).
pub fn timer(ms: u64, job: app::Job) {
    if let Some(tx) = TIMER.get() {
        let _ = tx.send((std::time::Instant::now() + Duration::from_millis(ms), job));
    }
}

fn start_timer(h: AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel::<(std::time::Instant, app::Job)>();
    let _ = TIMER.set(tx);
    std::thread::spawn(move || {
        let mut queue: Vec<(std::time::Instant, app::Job)> = vec![];
        loop {
            let next = queue.iter().map(|q| q.0).min();
            let msg = match next {
                Some(t) => rx.recv_timeout(t.saturating_duration_since(std::time::Instant::now())),
                None => rx.recv().map_err(|_| std::sync::mpsc::RecvTimeoutError::Disconnected),
            };
            match msg {
                Ok(item) => queue.push(item),
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                Err(_) => {}
            }
            let now = std::time::Instant::now();
            let mut due = vec![];
            let mut k = 0;
            while k < queue.len() {
                if queue[k].0 <= now {
                    due.push(queue.remove(k).1);
                } else {
                    k += 1;
                }
            }
            for job in due {
                with_app(&h, job);
            }
        }
    });
}

pub fn watch(h: &AppHandle, roots: Vec<String>) {
    let shared = h.state::<Shared>();
    let handle = h.clone();
    let pending = shared.fs_pending.clone();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        if matches!(ev.kind, notify::EventKind::Access(_)) {
            return;
        }
        if ev.paths.iter().all(|p| p.file_name().is_some_and(|n| n.to_string_lossy().ends_with(".folio-tmp"))) {
            return;
        }
        if pending.swap(true, Ordering::SeqCst) {
            return;
        }
        let (h, p) = (handle.clone(), pending.clone());
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            p.store(false, Ordering::SeqCst);
            with_app(&h, |app| app.on_fs_change());
        });
    });
    let mut slot = shared.watcher.lock().unwrap();
    *slot = None;
    if let Ok(mut w) = watcher {
        use notify::Watcher;
        for r in &roots {
            let _ = w.watch(std::path::Path::new(r), notify::RecursiveMode::Recursive);
        }
        *slot = Some(w);
    }
}

pub fn confirm(h: &AppHandle, message: String, detail: String, labels: Vec<String>, then: impl FnOnce(&mut App, usize) + Send + 'static) {
    let buttons = match labels.len() {
        0 | 1 => MessageDialogButtons::OkCustom(labels.first().cloned().unwrap_or_else(|| "OK".into())),
        2 => MessageDialogButtons::OkCancelCustom(labels[0].clone(), labels[1].clone()),
        _ => MessageDialogButtons::YesNoCancelCustom(labels[0].clone(), labels[2].clone(), labels[1].clone()),
    };
    let mut d = h.dialog().message(detail).title(message).kind(MessageDialogKind::Warning).buttons(buttons);
    if let Some(w) = h.get_webview_window("main") {
        d = d.parent(&w);
    }
    let handle = h.clone();
    d.show_with_result(move |r| {
        let idx = match r {
            MessageDialogResult::Custom(t) => labels.iter().position(|l| *l == t).unwrap_or(1),
            MessageDialogResult::Yes | MessageDialogResult::Ok => 0,
            MessageDialogResult::No => 2,
            MessageDialogResult::Cancel => 1,
        };
        with_app(&handle, move |app| then(app, idx));
    });
}

pub fn save_dialog(h: &AppHandle, name: String, dir: String, then: impl FnOnce(&mut App, Option<String>) + Send + 'static) {
    let mut d = h.dialog().file().set_can_create_directories(true).set_file_name(name);
    if let Some(w) = h.get_webview_window("main") {
        d = d.set_parent(&w);
    }
    if !dir.is_empty() {
        d = d.set_directory(dir);
    }
    let handle = h.clone();
    d.save_file(move |f| {
        let p = f.and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned());
        with_app(&handle, move |app| then(app, p));
    });
}

pub fn open_dialog(h: &AppHandle, folders: bool, title: String, then: impl FnOnce(&mut App, Vec<String>) + Send + 'static) {
    let mut d = h.dialog().file().set_can_create_directories(true).set_title(title);
    if let Some(w) = h.get_webview_window("main") {
        d = d.set_parent(&w);
    }
    let handle = h.clone();
    let done = move |picked: Option<Vec<tauri_plugin_dialog::FilePath>>| {
        let paths: Vec<String> = picked.unwrap_or_default().into_iter().filter_map(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned()).collect();
        if !paths.is_empty() {
            with_app(&handle, move |app| then(app, paths));
        }
    };
    if folders {
        d.pick_folders(done);
    } else {
        d.add_filter("Markdown", &["md", "markdown", "mdown", "mkd", "mdx", "txt"]).pick_files(done);
    }
}

pub fn window_theme(h: &AppHandle, dark: bool, bg: (u8, u8, u8)) {
    if let Some(w) = h.get_webview_window("main") {
        let _ = w.set_theme(Some(if dark { tauri::Theme::Dark } else { tauri::Theme::Light }));
        let _ = w.set_background_color(Some(tauri::window::Color(bg.0, bg.1, bg.2, 255)));
    }
}

pub fn window_title(h: &AppHandle, title: &str) {
    if let Some(w) = h.get_webview_window("main") {
        let _ = w.set_title(title);
    }
}

pub fn quit(h: &AppHandle) {
    h.state::<Shared>().force_close.store(true, Ordering::SeqCst);
    h.exit(0);
}

pub fn write_test_result(h: &AppHandle, mut v: Value) {
    let out = std::env::var("FOLIO_TEST_OUTPUT").unwrap_or_else(|_| "/tmp/folio-test.json".into());
    if let Some(w) = h.get_webview_window("main") {
        if let (Ok(pos), Ok(size), Ok(scale)) = (w.outer_position(), w.outer_size(), w.scale_factor()) {
            v["frame"] = json!([pos.x as f64 / scale, pos.y as f64 / scale, size.width as f64 / scale, size.height as f64 / scale]);
        }
    }
    let _ = std::fs::write(out, v.to_string());
}

/* ───────────── menu ───────────── */

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, label: &str, accel: &str| -> tauri::Result<MenuItem<Wry>> {
        let mut b = MenuItemBuilder::with_id(format!("cmd:{id}"), label);
        if !accel.is_empty() {
            b = b.accelerator(accel);
        }
        b.build(app)
    };
    let about = AboutMetadataBuilder::new().name(Some("Folio")).version(Some(env!("CARGO_PKG_VERSION"))).comments(Some("A live-edit Markdown reader.")).build();
    let app_menu = SubmenuBuilder::new(app, "Folio")
        .about(Some(about))
        .separator()
        .item(&item("settings", "Settings…", "CmdOrCtrl+Comma")?)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .item(&MenuItemBuilder::with_id("quit", "Quit Folio").accelerator("CmdOrCtrl+Q").build(app)?)
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&item("newFile", "New File", "CmdOrCtrl+N")?)
        .item(&item("newFileInFolder", "New File in Workspace…", "Alt+CmdOrCtrl+N")?)
        .separator()
        .item(&item("open", "Open File…", "CmdOrCtrl+O")?)
        .item(&item("addFolder", "Add Folder to Workspace…", "Alt+CmdOrCtrl+O")?)
        .item(&item("newWorkspace", "New Workspace…", "Control+CmdOrCtrl+N")?)
        .item(&item("switchWorkspace", "Switch Workspace…", "Control+CmdOrCtrl+R")?)
        .separator()
        .item(&item("save", "Save", "CmdOrCtrl+S")?)
        .item(&item("saveAs", "Save As…", "Shift+CmdOrCtrl+S")?)
        .item(&item("saveAll", "Save All", "Alt+CmdOrCtrl+S")?)
        .separator()
        .item(&item("exportHtml", "Export as HTML…", "")?)
        .item(&item("revert", "Revert File", "")?)
        .item(&item("reveal", "Reveal in Finder", "Alt+CmdOrCtrl+R")?)
        .separator()
        .item(&item("closeTab", "Close Tab", "CmdOrCtrl+W")?)
        .item(&item("reopenTab", "Reopen Closed Tab", "Shift+CmdOrCtrl+T")?)
        .item(&item("closeAll", "Close All Tabs", "Alt+CmdOrCtrl+W")?)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&item("find", "Find", "CmdOrCtrl+F")?)
        .item(&item("findNext", "Find Next", "CmdOrCtrl+G")?)
        .item(&item("findPrev", "Find Previous", "Shift+CmdOrCtrl+G")?)
        .item(&item("searchView", "Find in Workspace", "Shift+CmdOrCtrl+F")?)
        .separator()
        .item(&item("bold", "Bold", "CmdOrCtrl+B")?)
        .item(&item("italic", "Italic", "CmdOrCtrl+I")?)
        .item(&item("link", "Insert Link", "CmdOrCtrl+K")?)
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&item("palette", "Command Palette…", "Shift+CmdOrCtrl+P")?)
        .item(&item("quickOpen", "Go to File…", "CmdOrCtrl+P")?)
        .item(&item("gotoHeading", "Go to Heading…", "Shift+CmdOrCtrl+O")?)
        .separator()
        .item(&item("explorerView", "Explorer", "Shift+CmdOrCtrl+E")?)
        .item(&item("outlineView", "Outline", "Shift+CmdOrCtrl+U")?)
        .item(&item("workspacesView", "Workspaces", "Shift+CmdOrCtrl+K")?)
        .item(&item("toggleSidebar", "Toggle Sidebar", "CmdOrCtrl+Backslash")?)
        .separator()
        .item(&item("read", "Live", "CmdOrCtrl+1")?)
        .item(&item("split", "Split", "CmdOrCtrl+2")?)
        .item(&item("source", "Source", "CmdOrCtrl+3")?)
        .item(&item("togglePreview", "Toggle Live / Source", "Shift+CmdOrCtrl+V")?)
        .separator()
        .item(&item("zoomIn", "Bigger Text", "CmdOrCtrl+Equal")?)
        .item(&item("zoomOut", "Smaller Text", "CmdOrCtrl+Minus")?)
        .item(&item("zoomReset", "Actual Size", "CmdOrCtrl+0")?)
        .item(&item("wrap", "Toggle Word Wrap", "Alt+Z")?)
        .item(&item("zen", "Zen Mode", "Control+CmdOrCtrl+J")?)
        .item(&item("theme", "Toggle Light / Dark", "Alt+CmdOrCtrl+T")?)
        .separator()
        .fullscreen()
        .build()?;
    let go = SubmenuBuilder::new(app, "Go")
        .item(&item("nextTab", "Next Tab", "Shift+CmdOrCtrl+BracketRight")?)
        .item(&item("prevTab", "Previous Tab", "Shift+CmdOrCtrl+BracketLeft")?)
        .item(&item("gotoLine", "Go to Line…", "Control+G")?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window").minimize().maximize().separator().close_window().build()?;
    let help = SubmenuBuilder::new(app, "Help").item(&item("shortcuts", "Keyboard Shortcuts", "CmdOrCtrl+Slash")?).item(&item("welcome", "Welcome", "")?).build()?;
    MenuBuilder::new(app).items(&[&app_menu, &file, &edit, &view, &go, &window, &help]).build()
}

/// Close and quit always go through the app so it can ask about unsaved edits.
fn request_close(h: &AppHandle) {
    let shared = h.state::<Shared>();
    let booted = shared.app.lock().map(|a| a.booted).unwrap_or(false);
    if !booted || shared.force_close.load(Ordering::SeqCst) {
        quit(h);
        return;
    }
    with_app(h, |app| app.request_close());
}

fn main() {
    let allowed = Arc::new(Mutex::new(HashSet::new()));
    let mut core = App::new(allowed.clone());
    core.testing = std::env::args().any(|a| a == "--self-test");
    let cwd = std::env::current_dir().unwrap_or_default();
    core.pending_paths = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .map(|a| {
            let p = PathBuf::from(&a);
            if p.is_absolute() {
                a
            } else {
                cwd.join(p).to_string_lossy().into_owned()
            }
        })
        .collect();
    let shared = Shared { app: Mutex::new(core), watcher: Mutex::new(None), fs_pending: Arc::new(AtomicBool::new(false)), force_close: AtomicBool::new(false) };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(shared)
        .register_uri_scheme_protocol("mdasset", move |_ctx, req| {
            let path = folio_core::html::percent_decode(req.uri().path());
            let types = [("png", "image/png"), ("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("gif", "image/gif"), ("webp", "image/webp"), ("svg", "image/svg+xml"), ("avif", "image/avif")];
            let mime = types.iter().find(|(e, _)| *e == fsx::ext(&path)).map(|x| x.1);
            let canon = std::fs::canonicalize(&path).map(|p| p.to_string_lossy().into_owned()).unwrap_or(path.clone());
            let ok = allowed.lock().unwrap().iter().any(|root| {
                let r = std::fs::canonicalize(root).map(|p| p.to_string_lossy().into_owned()).unwrap_or(root.clone());
                fsx::within(&canon, &r)
            });
            match (mime, ok) {
                (Some(mime), true) => match std::fs::read(&path) {
                    Ok(data) if data.len() < 40_000_000 => http::Response::builder().header("Content-Type", mime).header("Cache-Control", "no-cache").body(data).unwrap(),
                    _ => http::Response::builder().status(404).body(Vec::new()).unwrap(),
                },
                _ => http::Response::builder().status(403).body(Vec::new()).unwrap(),
            }
        })
        .menu(build_menu)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == "quit" {
                request_close(app);
            } else if let Some(cmd) = id.strip_prefix("cmd:") {
                let cmd = cmd.to_string();
                with_app(app, move |a| a.command(&cmd));
            }
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    if !app.state::<Shared>().force_close.load(Ordering::SeqCst) {
                        api.prevent_close();
                        request_close(app);
                    }
                }
                WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) => {
                    let list: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                    with_app(app, move |a| a.open_paths(list));
                }
                WindowEvent::Focused(true) => with_app(app, |a| {
                    if a.booted {
                        a.check_external();
                    }
                }),
                WindowEvent::Resized(_) => {
                    let fs = window.is_fullscreen().unwrap_or(false);
                    with_app(app, move |a| {
                        if a.fullscreen != fs {
                            a.fullscreen = fs;
                            a.ui.cls("body", "fullscreen", fs);
                        }
                    });
                }
                _ => {}
            }
        })
        .setup(|app| {
            start_timer(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ui])
        .build(tauri::generate_context!())
        .expect("error while building Folio");

    app.run(|app, event| match event {
        RunEvent::ExitRequested { api, .. } => {
            let shared = app.state::<Shared>();
            let booted = shared.app.lock().map(|a| a.booted).unwrap_or(false);
            if !shared.force_close.load(Ordering::SeqCst) && booted {
                api.prevent_exit();
                request_close(app);
            }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => {
            let paths: Vec<String> = urls.iter().filter_map(|u| u.to_file_path().ok()).map(|p| p.to_string_lossy().into_owned()).collect();
            let booted = app.state::<Shared>().app.lock().map(|a| a.booted).unwrap_or(false);
            if booted {
                with_app(app, move |a| a.open_paths(paths));
            } else {
                app.state::<Shared>().app.lock().unwrap().pending_paths.extend(paths);
            }
        }
        _ => {}
    });
}
