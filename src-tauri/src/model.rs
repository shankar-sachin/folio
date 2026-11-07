//! Persistent state (settings, workspaces, recents) and the in-memory document model.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub reader_font: String,
    pub reader_size: f64,
    pub reader_width: f64,
    pub editor_size: f64,
    pub line_height: f64,
    pub wrap: bool,
    pub line_numbers: bool,
    pub show_all_files: bool,
    pub sync_scroll: bool,
    pub default_mode: String,
    pub remote_images: bool,
    pub sidebar_width: f64,
    pub sidebar_visible: bool,
    pub view: String,
    pub split: f64,
    pub preview_tabs: bool,
    pub auto_reload: bool,
    pub restore_tabs: bool,
    pub open_editors: bool,
    pub ws_open: bool,
    pub zen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "dark".into(),
            reader_font: "sans".into(),
            reader_size: 16.0,
            reader_width: 800.0,
            editor_size: 13.0,
            line_height: 1.7,
            wrap: true,
            line_numbers: true,
            show_all_files: false,
            sync_scroll: true,
            default_mode: "read".into(),
            remote_images: false,
            sidebar_width: 272.0,
            sidebar_visible: true,
            view: "explorer".into(),
            split: 0.5,
            preview_tabs: true,
            auto_reload: true,
            restore_tabs: true,
            open_editors: true,
            ws_open: true,
            zen: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct TabRef {
    pub key: String,
    pub path: Option<String>,
    pub mode: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub roots: Vec<String>,
    pub tabs: Vec<TabRef>,
    pub active: Option<String>,
    pub expanded: Vec<String>,
    pub created: f64,
    pub opened: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Persisted {
    pub settings: Settings,
    pub workspaces: Vec<Workspace>,
    pub current: Option<String>,
    pub recent: Vec<String>,
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

pub fn uid() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    let n = h.finish();
    let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
    (0..8).map(|i| alphabet[((n >> (i * 6)) % 36) as usize] as char).collect()
}

impl Workspace {
    pub fn new(name: &str, roots: Vec<String>) -> Self {
        Workspace { id: uid(), name: name.into(), expanded: roots.clone(), roots, tabs: vec![], active: None, created: now_ms(), opened: now_ms() }
    }
}

pub fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".into())
}

pub fn state_file() -> PathBuf {
    let dir = PathBuf::from(home()).join("Library/Application Support/Folio");
    let _ = fs::create_dir_all(&dir);
    dir.join(std::env::var("FOLIO_STATE_FILE").unwrap_or_else(|_| "state.json".into()))
}

pub fn load() -> Option<Persisted> {
    let text = fs::read_to_string(state_file()).ok()?;
    let p: Persisted = serde_json::from_str(&text).ok()?;
    (!p.workspaces.is_empty()).then_some(p)
}

pub fn save(p: &Persisted) {
    let Ok(json) = serde_json::to_string(p) else { return };
    let file = state_file();
    let tmp = file.with_extension("json.tmp");
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, &file);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Read,
    Split,
    Source,
}

impl Mode {
    pub fn parse(s: &str) -> Mode {
        match s {
            "split" => Mode::Split,
            "source" => Mode::Source,
            _ => Mode::Read,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Read => "read",
            Mode::Split => "split",
            Mode::Source => "source",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Mode::Read => "Live",
            Mode::Split => "Split",
            Mode::Source => "Source",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Md,
    Settings,
    Shortcuts,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pane {
    Source,
    Preview,
}

/// A block being edited in place in Live mode.
#[derive(Clone, Debug)]
pub struct Live {
    pub start: usize,
    pub end: usize,
    pub orig_end: usize,
    pub append: bool,
    pub base: String,
    pub original: String,
}

pub struct Doc {
    pub key: String,
    pub kind: Kind,
    pub path: Option<String>,
    pub name: String,
    pub text: String,
    pub baseline: String,
    pub mtime: f64,
    pub mode: Mode,
    pub preview: bool,
    pub conflict: bool,
    pub missing: bool,
    /// DOM id number (`doc-N`).
    pub n: u32,
    pub live: Option<Live>,
    pub cursor: (usize, usize, usize),
    pub top_line: f64,
    pub cur_heading: Option<usize>,
    pub last_focus: Pane,
    /// The textarea in the page is behind `text` (edited while hidden).
    pub ta_stale: bool,
    /// The preview should be re-rendered on the next flush.
    pub preview_stale: bool,
    pub cache: Option<(u64, bool, Arc<folio_core::Rendered>)>,
}

pub const LARGE: usize = 400_000;

impl Doc {
    pub fn is_dirty(&self) -> bool {
        self.kind == Kind::Md && self.text != self.baseline
    }
    pub fn large(&self) -> bool {
        self.text.len() > LARGE
    }
    pub fn dir(&self) -> Option<String> {
        self.path.as_deref().map(crate::fsx::dirname)
    }
}
