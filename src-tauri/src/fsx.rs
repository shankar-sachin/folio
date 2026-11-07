//! File-system work: folder trees, file lists, workspace search, trash, and path helpers.

use regex::RegexBuilder;
use std::{
    cmp::Ordering,
    fs,
    path::Path,
    process::Command,
    time::UNIX_EPOCH,
};

pub const MD_EXT: &[&str] = &["md", "markdown", "mdown", "mkd", "mdx"];
const OPENABLE: &[&str] = &["md", "markdown", "mdown", "mkd", "mdx", "txt"];
const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", ".svn", ".hg", "build", "dist", ".build", "DerivedData", "Pods", ".next", ".venv", "venv", "__pycache__", ".cache", "target",
];
pub const MAX_FILE: u64 = 16_000_000;

/* ───────────── paths ───────────── */

pub fn basename(p: &str) -> String {
    p.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(p).to_string()
}
pub fn dirname(p: &str) -> String {
    let t = p.trim_end_matches('/');
    match t.rfind('/') {
        Some(0) | None => "/".into(),
        Some(i) => t[..i].to_string(),
    }
}
pub fn ext(p: &str) -> String {
    let b = basename(p);
    match b.rfind('.') {
        Some(i) if i > 0 => b[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}
pub fn is_markdown(p: &str) -> bool {
    MD_EXT.contains(&ext(p).as_str())
}
pub fn strip_ext(name: &str) -> String {
    let e = ext(name);
    if OPENABLE.contains(&e.as_str()) {
        name[..name.len() - e.len() - 1].to_string()
    } else {
        name.to_string()
    }
}
pub fn within(p: &str, root: &str) -> bool {
    p == root || p.starts_with(&format!("{}/", root.trim_end_matches('/')))
}
pub fn normalize(p: &str) -> String {
    folio_core::inline::normalize_path(p)
}
pub fn tilde(p: &str, home: &str) -> String {
    if !home.is_empty() && within(p, home) {
        format!("~{}", &p[home.len()..])
    } else {
        p.to_string()
    }
}
pub fn mtime(p: &str) -> Option<f64> {
    fs::metadata(p).ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs_f64())
}

/* ───────────── UTF-16 offsets (the page speaks UTF-16) ───────────── */

pub fn u16_to_byte(s: &str, n: usize) -> usize {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        if units >= n {
            return i;
        }
        units += c.len_utf16();
    }
    s.len()
}
pub fn byte_to_u16(s: &str, b: usize) -> usize {
    let b = b.min(s.len());
    let mut b = b;
    while !s.is_char_boundary(b) {
        b -= 1;
    }
    s[..b].encode_utf16().count()
}

/* ───────────── trees ───────────── */

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub missing: bool,
    pub children: Vec<Node>,
}

/// Natural ordering so "note 2" sorts before "note 10".
pub fn natord(a: &str, b: &str) -> Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ai.peek().copied().filter(char::is_ascii_digit) {
                    na.push(c);
                    ai.next();
                }
                let mut nb = String::new();
                while let Some(c) = bi.peek().copied().filter(char::is_ascii_digit) {
                    nb.push(c);
                    bi.next();
                }
                let (na, nb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = na.len().cmp(&nb.len()).then(na.cmp(nb));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                ai.next();
                bi.next();
            }
        }
    }
}

pub fn tree(root: &str, all: bool) -> Node {
    fn walk(dir: &Path, all: bool, depth: usize, budget: &mut i64) -> Vec<Node> {
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
            let path = entry.path().to_string_lossy().into_owned();
            if ft.is_dir() {
                if SKIP_DIRS.contains(&name.as_str()) || name.ends_with(".app") {
                    continue;
                }
                let children = walk(Path::new(&path), all, depth + 1, budget);
                if all || !children.is_empty() {
                    dirs.push(Node { name, path, dir: true, missing: false, children });
                }
            } else if ft.is_file() && (all || OPENABLE.contains(&ext(&name).as_str())) {
                files.push(Node { name, path, dir: false, missing: false, children: vec![] });
            }
        }
        let by = |a: &Node, b: &Node| natord(&a.name.to_lowercase(), &b.name.to_lowercase());
        dirs.sort_by(by);
        files.sort_by(by);
        dirs.into_iter().chain(files).collect()
    }
    let p = Path::new(root);
    if !p.is_dir() {
        return Node { name: basename(root), path: root.into(), dir: true, missing: true, children: vec![] };
    }
    let mut budget = 20_000i64;
    Node { name: basename(root), path: root.into(), dir: true, missing: false, children: walk(p, all, 0, &mut budget) }
}

pub fn find_node<'a>(n: &'a Node, path: &str) -> Option<&'a Node> {
    if n.path == path {
        return Some(n);
    }
    if !n.dir || !within(path, &n.path) {
        return None;
    }
    n.children.iter().find_map(|c| find_node(c, path))
}

pub fn markdown_files(roots: &[String]) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        if out.len() > 20_000 {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() {
                if !SKIP_DIRS.contains(&name.as_str()) && !name.ends_with(".app") {
                    walk(&p, out);
                }
            } else if ft.is_file() && OPENABLE.contains(&ext(&name).as_str()) {
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
    let mut out = vec![];
    for r in roots {
        walk(Path::new(r), &mut out);
    }
    out
}

/* ───────────── search ───────────── */

#[derive(Clone, Debug)]
pub struct Match {
    pub line: usize,
    pub text: String,
    /// Byte range within `text`.
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SearchResult {
    pub files: Vec<(String, Vec<Match>)>,
    pub total: usize,
    pub error: Option<String>,
}

pub fn build_regex(query: &str, regex: bool, case: bool, word: bool) -> Result<regex::Regex, String> {
    let mut pat = if regex { query.to_string() } else { regex::escape(query) };
    if word {
        pat = format!(r"\b(?:{pat})\b");
    }
    RegexBuilder::new(&pat).case_insensitive(!case).build().map_err(|_| "Invalid regular expression".to_string())
}

pub fn search(roots: &[String], query: &str, regex: bool, case: bool, word: bool) -> SearchResult {
    let re = match build_regex(query, regex, case, word) {
        Ok(r) => r,
        Err(e) => return SearchResult { error: Some(e), ..Default::default() },
    };
    let mut out = SearchResult::default();
    for file in markdown_files(roots) {
        if out.total >= 5_000 {
            break;
        }
        if fs::metadata(&file).map(|m| m.len() > MAX_FILE).unwrap_or(true) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&file) else { continue };
        let mut matches = vec![];
        for (i, line) in text.lines().enumerate() {
            for m in re.find_iter(line).take(20) {
                if m.start() == m.end() {
                    continue;
                }
                let clip = line.char_indices().nth(400).map(|(b, _)| b).unwrap_or(line.len());
                if m.start() >= clip {
                    continue;
                }
                matches.push(Match { line: i + 1, text: line[..clip].to_string(), start: m.start(), end: m.end().min(clip) });
            }
            if matches.len() >= 400 {
                break;
            }
        }
        if !matches.is_empty() {
            out.total += matches.len();
            out.files.push((file, matches));
        }
    }
    out
}

/* ───────────── file operations ───────────── */

pub fn read(path: &str) -> Result<(String, f64), String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        return Err("That’s a folder".into());
    }
    if meta.len() > MAX_FILE {
        return Err("File is larger than 16 MB".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), mtime(path).unwrap_or(0.0)))
}

pub fn write(path: &str, text: &str) -> Result<f64, String> {
    let tmp = format!("{}/.{}.folio-tmp", dirname(path), basename(path));
    fs::write(&tmp, text).and_then(|_| fs::rename(&tmp, path)).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        e.to_string()
    })?;
    Ok(mtime(path).unwrap_or(0.0))
}

pub fn create(path: &str, dir: bool, text: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        return Err(format!("“{}” already exists", basename(path)));
    }
    if dir { fs::create_dir_all(path) } else { fs::write(path, text) }.map_err(|e| e.to_string())
}

pub fn rename(from: &str, to: &str) -> Result<(), String> {
    if Path::new(to).exists() {
        return Err(format!("“{}” already exists", basename(to)));
    }
    fs::rename(from, to).map_err(|e| e.to_string())
}

pub fn trash(path: &str) -> Result<(), String> {
    let bin = format!("{}/.Trash", crate::model::home());
    let mut target = format!("{bin}/{}", basename(path));
    if Path::new(&target).exists() {
        let name = basename(path);
        let e = ext(&name);
        let stem = if e.is_empty() { name.clone() } else { name[..name.len() - e.len() - 1].to_string() };
        let stamp = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        target = format!("{bin}/{stem} {stamp}{}", if e.is_empty() { String::new() } else { format!(".{e}") });
    }
    if fs::rename(path, &target).is_ok() {
        return Ok(());
    }
    // Different volume: let Finder move it to that volume's Trash.
    let script = format!("tell application \"Finder\" to delete POSIX file \"{}\"", path.replace('\\', "\\\\").replace('"', "\\\""));
    let ok = Command::new("osascript").args(["-e", &script]).status().map(|s| s.success()).unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err("Couldn’t move the item to the Trash".into())
    }
}

pub fn reveal(path: &str) {
    let _ = Command::new("open").args(["-R", path]).spawn();
}

pub fn open_external(url: &str) {
    if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") {
        let _ = Command::new("open").arg(url).spawn();
    }
}

pub fn copy(text: &str) {
    use std::io::Write;
    if let Ok(mut child) = Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths() {
        assert_eq!(basename("/a/b.md"), "b.md");
        assert_eq!(dirname("/a/b.md"), "/a");
        assert_eq!(dirname("/a"), "/");
        assert_eq!(strip_ext("x.md"), "x");
        assert!(within("/a/b", "/a"));
        assert!(!within("/ab", "/a"));
        assert_eq!(u16_to_byte("a😀b", 3), 5);
        assert_eq!(byte_to_u16("a😀b", 5), 3);
        assert_eq!(natord("note 2", "note 10"), Ordering::Less);
    }
}
