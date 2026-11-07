//! Block-level Markdown (GFM flavour): parsing into a tree, rendering to HTML with
//! `data-line` on every top-level block, and document analysis (outline, words, tasks).

use crate::code::highlight;
use crate::html::{esc, esc_into, sanitize, strip_tags};
use crate::icons::icon;
use crate::inline::{normalize_label, render_inline, Ctx, Refs};

/// One source line, remembering where it came from after container prefixes are stripped.
#[derive(Clone, Debug)]
struct Line {
    no: usize,
    s: String,
}

#[derive(Debug)]
enum Block {
    Heading { level: u8, text: String, line: usize },
    Para(String),
    Code { lang: String, text: String },
    Quote(Vec<Block>),
    Callout { kind: &'static str, blocks: Vec<Block> },
    List { ordered: bool, start: u64, tight: bool, items: Vec<Item> },
    Table { aligns: Vec<Align>, head: Vec<String>, rows: Vec<Vec<String>> },
    Hr,
    Html(String),
    Front(Vec<String>),
}

#[derive(Debug)]
struct Item {
    task: Option<(bool, usize)>,
    blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub depth: u8,
    pub text: String,
    pub line: usize,
    pub id: String,
}

/// Everything the workbench needs from one pass over a document.
#[derive(Clone, Debug, Default)]
pub struct Rendered {
    pub html: String,
    /// Start line (0-based) of each top-level block, in order.
    pub blocks: Vec<usize>,
    pub headings: Vec<Heading>,
    /// Raw text of each code block, indexed by the copy button's `data-copy`.
    pub codes: Vec<String>,
    /// Source line of each task checkbox, indexed by `data-task`.
    pub tasks: Vec<usize>,
    pub words: usize,
}

#[derive(Clone, Default)]
pub struct Options<'a> {
    /// Folder of the document, for resolving relative images. `None` for unsaved docs.
    pub base_dir: Option<&'a str>,
    pub remote_images: bool,
}

/* ───────────── line helpers ───────────── */

/// Columns of leading whitespace (tabs to the next multiple of 4) and the byte length.
fn indent(s: &str) -> (usize, usize) {
    let mut col = 0;
    for (i, c) in s.char_indices() {
        match c {
            ' ' => col += 1,
            '\t' => col += 4 - col % 4,
            _ => return (col, i),
        }
    }
    (col, s.len())
}

/// Remove `n` columns of leading whitespace, splitting a tab if needed.
fn strip_cols(s: &str, n: usize) -> String {
    let mut col = 0;
    for (i, c) in s.char_indices() {
        if col >= n {
            return s[i..].to_string();
        }
        match c {
            ' ' => col += 1,
            '\t' => {
                let w = 4 - col % 4;
                if col + w > n {
                    return " ".repeat(col + w - n) + &s[i + 1..];
                }
                col += w;
            }
            _ => return s[i..].to_string(),
        }
    }
    String::new()
}

fn blank(s: &str) -> bool {
    s.trim().is_empty()
}

fn fence_open(s: &str) -> Option<(char, usize, usize, String)> {
    let (col, b) = indent(s);
    if col > 3 {
        return None;
    }
    let rest = &s[b..];
    let ch = rest.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let n = rest.chars().take_while(|&c| c == ch).count();
    if n < 3 {
        return None;
    }
    let info = rest[n..].trim();
    if ch == '`' && info.contains('`') {
        return None;
    }
    Some((ch, n, col, info.to_string()))
}

fn fence_close(s: &str, ch: char, n: usize) -> bool {
    let (col, b) = indent(s);
    if col > 3 {
        return false;
    }
    let rest = s[b..].trim_end();
    rest.len() >= n && rest.chars().all(|c| c == ch)
}

fn atx(s: &str) -> Option<(u8, String)> {
    let (col, b) = indent(s);
    if col > 3 {
        return None;
    }
    let rest = &s[b..];
    let n = rest.chars().take_while(|&c| c == '#').count();
    if n == 0 || n > 6 {
        return None;
    }
    let after = &rest[n..];
    if !(after.is_empty() || after.starts_with(' ') || after.starts_with('\t')) {
        return None;
    }
    let mut t = after.trim();
    // Optional closing sequence: spaces then #s.
    let stripped = t.trim_end_matches('#');
    if stripped.len() != t.len() && (stripped.is_empty() || stripped.ends_with(' ') || stripped.ends_with('\t')) {
        t = stripped.trim_end();
    }
    Some((n as u8, t.to_string()))
}

fn is_hr(s: &str) -> bool {
    let (col, b) = indent(s);
    if col > 3 {
        return false;
    }
    let rest = &s[b..];
    let Some(ch) = rest.chars().next() else { return false };
    if !matches!(ch, '-' | '*' | '_') {
        return false;
    }
    let mut n = 0;
    for c in rest.chars() {
        if c == ch {
            n += 1;
        } else if c != ' ' && c != '\t' {
            return false;
        }
    }
    n >= 3
}

fn setext(s: &str) -> Option<u8> {
    let (col, b) = indent(s);
    if col > 3 {
        return None;
    }
    let t = s[b..].trim_end();
    if !t.is_empty() && t.chars().all(|c| c == '=') {
        Some(1)
    } else if !t.is_empty() && t.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

fn quote_start(s: &str) -> bool {
    let (col, b) = indent(s);
    col <= 3 && s[b..].starts_with('>')
}

/// Strip one level of `>` (and one optional space after it).
fn strip_quote(s: &str) -> String {
    let (_, b) = indent(s);
    let rest = &s[b + 1..];
    if let Some(r) = rest.strip_prefix(' ') {
        r.to_string()
    } else if rest.starts_with('\t') {
        strip_cols(rest, 1)
    } else {
        rest.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Marker {
    ordered: bool,
    /// Bullet char, or the delimiter (`.`/`)`) for ordered lists.
    ch: char,
    start: u64,
    /// Column where the item's content starts.
    content: usize,
    empty: bool,
}

fn list_marker(s: &str) -> Option<Marker> {
    let (col, b) = indent(s);
    if col > 3 {
        return None;
    }
    let rest = &s[b..];
    let (ordered, ch, start, mlen) = match rest.chars().next()? {
        c @ ('-' | '*' | '+') => (false, c, 0, 1),
        c if c.is_ascii_digit() => {
            let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits > 9 {
                return None;
            }
            let d = rest[digits..].chars().next()?;
            if d != '.' && d != ')' {
                return None;
            }
            (true, d, rest[..digits].parse().ok()?, digits + 1)
        }
        _ => return None,
    };
    let after = &rest[mlen..];
    if after.is_empty() || blank(after) {
        return Some(Marker { ordered, ch, start, content: col + mlen + 1, empty: true });
    }
    if !(after.starts_with(' ') || after.starts_with('\t')) {
        return None;
    }
    let (sp, _) = indent(after);
    let sp = if sp > 4 { 1 } else { sp };
    Some(Marker { ordered, ch, start, content: col + mlen + sp, empty: false })
}

fn same_list(a: &Marker, b: &Marker) -> bool {
    a.ordered == b.ordered && a.ch == b.ch
}

fn html_block_start(s: &str) -> Option<bool> {
    // Some(true): can interrupt a paragraph; Some(false): only at block start.
    let (col, b) = indent(s);
    if col > 3 {
        return None;
    }
    let rest = &s[b..];
    if !rest.starts_with('<') {
        return None;
    }
    if rest.starts_with("<!--") {
        return Some(true);
    }
    let r = rest.strip_prefix("</").unwrap_or(&rest[1..]);
    let name: String = r.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    if name.is_empty() || !name.chars().next().unwrap().is_ascii_alphabetic() {
        return None;
    }
    let after = &r[name.len()..];
    if !(after.is_empty() || after.starts_with([' ', '\t', '>']) || after.starts_with("/>")) {
        return None;
    }
    const BLOCK: &[&str] = &[
        "address", "article", "aside", "blockquote", "body", "center", "details", "dialog", "dd", "div", "dl", "dt", "fieldset", "figcaption",
        "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "html", "iframe", "legend", "li", "main", "nav", "ol",
        "p", "pre", "section", "summary", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "ul", "script", "style", "textarea", "picture",
    ];
    let lname = name.to_ascii_lowercase();
    if BLOCK.contains(&lname.as_str()) {
        return Some(true);
    }
    // Any other complete tag alone on its line.
    let t = rest.trim_end();
    (t.ends_with('>') && !t[1..t.len() - 1].contains(['<', '>'])).then_some(false)
}

/// GFM table delimiter row: `| :--- | ---: |`.
fn delim_row(s: &str) -> Option<Vec<Align>> {
    let t = s.trim();
    if !t.contains('-') {
        return None;
    }
    if !t.contains('|') && !t.contains(':') && t.chars().all(|c| c == '-') {
        return None; // a bare `---` is a setext underline / hr, not a table
    }
    let cells = split_row(t);
    let mut out = vec![];
    for c in cells {
        let c = c.trim();
        let core = c.trim_start_matches(':').trim_end_matches(':');
        if core.is_empty() || !core.chars().all(|x| x == '-') {
            return None;
        }
        out.push(match (c.starts_with(':'), c.ends_with(':')) {
            (true, true) => Align::Center,
            (false, true) => Align::Right,
            (true, false) => Align::Left,
            _ => Align::None,
        });
    }
    (!out.is_empty()).then_some(out)
}

/// Split a table row on unescaped pipes outside code spans.
fn split_row(s: &str) -> Vec<String> {
    let mut t = s.trim();
    if let Some(x) = t.strip_prefix('|') {
        t = x;
    }
    if t.ends_with('|') && !t.ends_with("\\|") {
        t = &t[..t.len() - 1];
    }
    let mut cells = vec![];
    let mut cur = String::new();
    let mut tick = 0usize;
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cur.push('|');
                chars.next();
            }
            '`' => {
                let mut n = 1;
                while chars.peek() == Some(&'`') {
                    chars.next();
                    n += 1;
                }
                if tick == 0 {
                    tick = n;
                } else if tick == n {
                    tick = 0;
                }
                cur.push_str(&"`".repeat(n));
            }
            '|' if tick == 0 => cells.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(c),
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

fn table_start(lines: &[Line], i: usize) -> Option<Vec<Align>> {
    let head = &lines.get(i)?.s;
    let next = &lines.get(i + 1)?.s;
    if !head.contains('|') || indent(head).0 > 3 {
        return None;
    }
    let aligns = delim_row(next)?;
    (split_row(head).len() == aligns.len()).then_some(aligns)
}

fn ref_def(s: &str) -> Option<(String, String, String)> {
    let (col, b) = indent(s);
    if col > 3 {
        return None;
    }
    let rest = s[b..].strip_prefix('[')?;
    let close = rest.find("]:")?;
    let label = &rest[..close];
    if label.trim().is_empty() || label.contains('[') {
        return None;
    }
    let after = rest[close + 2..].trim();
    if after.is_empty() {
        return None;
    }
    let (url, tail) = if let Some(r) = after.strip_prefix('<') {
        let e = r.find('>')?;
        (r[..e].to_string(), r[e + 1..].trim())
    } else {
        let e = after.find([' ', '\t']).unwrap_or(after.len());
        (after[..e].to_string(), after[e..].trim())
    };
    let title = if tail.is_empty() {
        String::new()
    } else {
        let q = tail.chars().next()?;
        let endq = match q {
            '"' => '"',
            '\'' => '\'',
            '(' => ')',
            _ => return None,
        };
        if tail.len() < 2 || !tail.ends_with(endq) {
            return None;
        }
        tail[1..tail.len() - 1].to_string()
    };
    Some((normalize_label(label), url, title))
}

/* ───────────── parser ───────────── */

struct Parser {
    refs: Refs,
    tasks: Vec<usize>,
}

/// Can `s` start a block that interrupts a paragraph?
fn interrupts(lines: &[Line], i: usize) -> bool {
    let s = &lines[i].s;
    if atx(s).is_some() || fence_open(s).is_some() || is_hr(s) || quote_start(s) || html_block_start(s) == Some(true) {
        return true;
    }
    if let Some(m) = list_marker(s) {
        if !m.empty && (!m.ordered || m.start == 1) {
            return true;
        }
    }
    false
}

impl Parser {
    fn parse(&mut self, lines: &[Line], top: bool) -> (Vec<Block>, Vec<usize>, bool) {
        let mut blocks = vec![];
        let mut starts = vec![];
        let mut i = 0;
        let mut saw_blank_between = false;
        let mut pending_blank = false;
        while i < lines.len() {
            let s = &lines[i].s;
            if blank(s) {
                pending_blank = true;
                i += 1;
                continue;
            }
            if pending_blank && !blocks.is_empty() {
                saw_blank_between = true;
            }
            pending_blank = false;
            let start_line = lines[i].no;
            let (col, _) = indent(s);

            // Front matter (top of document only).
            if top && i == 0 && lines[0].no == 0 && s.trim_end() == "---" {
                if let Some(end) = lines.iter().take(300).skip(1).position(|l| matches!(l.s.trim_end(), "---" | "...")) {
                    let end = end + 1;
                    blocks.push(Block::Front(lines[1..end].iter().map(|l| l.s.clone()).collect()));
                    starts.push(start_line);
                    i = end + 1;
                    continue;
                }
            }
            // Indented code.
            if col >= 4 {
                let mut body = vec![];
                let mut j = i;
                while j < lines.len() && (blank(&lines[j].s) || indent(&lines[j].s).0 >= 4) {
                    body.push(strip_cols(&lines[j].s, 4));
                    j += 1;
                }
                while body.last().is_some_and(|l| blank(l)) {
                    body.pop();
                    j -= 1;
                }
                blocks.push(Block::Code { lang: String::new(), text: body.join("\n") });
                starts.push(start_line);
                i = j;
                continue;
            }
            if let Some((ch, n, ind, info)) = fence_open(s) {
                let mut body = vec![];
                let mut j = i + 1;
                while j < lines.len() && !fence_close(&lines[j].s, ch, n) {
                    body.push(strip_cols(&lines[j].s, ind));
                    j += 1;
                }
                let lang = info.split_whitespace().next().unwrap_or("").to_string();
                blocks.push(Block::Code { lang, text: body.join("\n") });
                starts.push(start_line);
                i = j + 1;
                continue;
            }
            if let Some((level, text)) = atx(s) {
                blocks.push(Block::Heading { level, text, line: start_line });
                starts.push(start_line);
                i += 1;
                continue;
            }
            if is_hr(s) {
                blocks.push(Block::Hr);
                starts.push(start_line);
                i += 1;
                continue;
            }
            if quote_start(s) {
                let mut inner = vec![];
                let mut j = i;
                let mut last_para = false;
                while j < lines.len() {
                    let l = &lines[j].s;
                    if quote_start(l) {
                        let stripped = strip_quote(l);
                        last_para = !blank(&stripped) && fence_open(&stripped).is_none() && atx(&stripped).is_none();
                        inner.push(Line { no: lines[j].no, s: stripped });
                    } else if last_para && !blank(l) && !interrupts(lines, j) {
                        inner.push(Line { no: lines[j].no, s: l.clone() });
                    } else {
                        break;
                    }
                    j += 1;
                }
                let (kids, _, _) = self.parse(&inner, false);
                blocks.push(callout(kids));
                starts.push(start_line);
                i = j;
                continue;
            }
            if let Some(first) = list_marker(s) {
                let (list, j) = self.parse_list(lines, i, first);
                blocks.push(list);
                starts.push(start_line);
                i = j;
                continue;
            }
            if let Some(aligns) = table_start(lines, i) {
                let head = split_row(&lines[i].s);
                let mut rows = vec![];
                let mut j = i + 2;
                while j < lines.len() && !blank(&lines[j].s) && !interrupts(lines, j) {
                    let mut cells = split_row(&lines[j].s);
                    cells.resize(aligns.len(), String::new());
                    rows.push(cells);
                    j += 1;
                }
                blocks.push(Block::Table { aligns, head, rows });
                starts.push(start_line);
                i = j;
                continue;
            }
            if html_block_start(s).is_some() {
                let mut j = i;
                let mut body = vec![];
                let comment = s.trim_start().starts_with("<!--");
                let lower = s.trim_start().to_ascii_lowercase();
                let raw_end = ["<pre", "<script", "<style", "<textarea"].iter().find(|t| lower.starts_with(**t)).map(|t| format!("</{}>", &t[1..]));
                while j < lines.len() {
                    let l = &lines[j].s;
                    if !comment && raw_end.is_none() && blank(l) {
                        break;
                    }
                    body.push(l.clone());
                    j += 1;
                    if comment && l.contains("-->") {
                        break;
                    }
                    if let Some(e) = &raw_end {
                        if l.to_ascii_lowercase().contains(e.as_str()) {
                            break;
                        }
                    }
                }
                blocks.push(Block::Html(body.join("\n")));
                starts.push(start_line);
                i = j;
                continue;
            }
            // Paragraph (with reference definitions peeled off the front).
            if let Some((label, url, title)) = ref_def(s) {
                self.refs.entry(label).or_insert((url, title));
                i += 1;
                continue;
            }
            let mut para = vec![s.trim_start().to_string()];
            let mut j = i + 1;
            let mut heading = None;
            while j < lines.len() {
                let l = &lines[j].s;
                if blank(l) {
                    break;
                }
                if let Some(level) = setext(l) {
                    heading = Some(level);
                    j += 1;
                    break;
                }
                if interrupts(lines, j) || table_start(lines, j).is_some() {
                    break;
                }
                para.push(l.trim_start().to_string());
                j += 1;
            }
            let text = para.join("\n");
            let text = text.trim_end().to_string();
            match heading {
                Some(level) => blocks.push(Block::Heading { level, text, line: start_line }),
                None => blocks.push(Block::Para(text)),
            }
            starts.push(start_line);
            i = j;
        }
        (blocks, starts, saw_blank_between)
    }

    fn parse_list(&mut self, lines: &[Line], mut i: usize, first: Marker) -> (Block, usize) {
        let mut items = vec![];
        let mut loose = false;
        let mut blank_before_item = false;
        while i < lines.len() {
            let Some(m) = list_marker(&lines[i].s) else { break };
            if !same_list(&first, &m) {
                break;
            }
            if blank_before_item && !items.is_empty() {
                loose = true;
            }
            // First line of the item, with the marker replaced by spaces.
            let s = &lines[i].s;
            let first_line = if m.empty { String::new() } else { cut_marker(s, &m) };
            let mut inner = vec![Line { no: lines[i].no, s: first_line }];
            let mut j = i + 1;
            let mut last_blank = m.empty;
            let mut trailing_blank = 0;
            while j < lines.len() {
                let l = &lines[j].s;
                if blank(l) {
                    if m.empty && inner.len() == 1 {
                        break; // an empty item followed by a blank line ends
                    }
                    inner.push(Line { no: lines[j].no, s: String::new() });
                    last_blank = true;
                    trailing_blank += 1;
                    j += 1;
                    continue;
                }
                let (col, _) = indent(l);
                if col >= m.content {
                    inner.push(Line { no: lines[j].no, s: strip_cols(l, m.content) });
                } else if !last_blank && !interrupts(lines, j) && list_marker(l).is_none() && !is_hr(l) {
                    inner.push(Line { no: lines[j].no, s: l.trim_start().to_string() }); // lazy continuation
                } else {
                    break;
                }
                last_blank = false;
                trailing_blank = 0;
                j += 1;
            }
            // Trailing blanks belong between items, not inside this one.
            for _ in 0..trailing_blank {
                inner.pop();
            }
            blank_before_item = trailing_blank > 0;
            // Task marker.
            let mut task = None;
            if let Some(f) = inner.first_mut() {
                let t = &f.s;
                let b = t.as_bytes();
                if b.len() >= 3 && b[0] == b'[' && b[2] == b']' && matches!(b[1], b' ' | b'x' | b'X') && (b.len() == 3 || b[3] == b' ' || b[3] == b'\t') {
                    let checked = b[1] != b' ';
                    self.tasks.push(f.no);
                    task = Some((checked, self.tasks.len() - 1));
                    f.s = t[3..].trim_start().to_string();
                }
            }
            let (blocks, _, inner_loose) = self.parse(&inner, false);
            if inner_loose {
                loose = true;
            }
            items.push(Item { task, blocks });
            i = j;
            if blank_before_item && !lines.get(i).and_then(|l| list_marker(&l.s)).is_some_and(|n| same_list(&first, &n)) {
                break;
            }
        }
        (Block::List { ordered: first.ordered, start: first.start, tight: !loose, items }, i)
    }
}

/// The item's first line with its list marker removed.
fn cut_marker(s: &str, m: &Marker) -> String {
    let (_, b) = indent(s);
    let rest = &s[b..];
    let mlen = if m.ordered { rest.chars().take_while(|c| c.is_ascii_digit()).count() + 1 } else { 1 };
    let after = &rest[mlen..];
    let (sp, _) = indent(after);
    if sp > 4 {
        strip_cols(after, 1)
    } else {
        after.trim_start_matches([' ', '\t']).to_string()
    }
}

const CALLOUTS: &[(&str, &str, &str)] = &[
    ("NOTE", "info", "Note"),
    ("TIP", "bulb", "Tip"),
    ("IMPORTANT", "alert", "Important"),
    ("WARNING", "warning", "Warning"),
    ("CAUTION", "shield", "Caution"),
];

fn callout(mut kids: Vec<Block>) -> Block {
    if let Some(Block::Para(t)) = kids.first() {
        let trimmed = t.trim_start();
        if let Some(rest) = trimmed.strip_prefix("[!") {
            if let Some(close) = rest.find(']') {
                let kind = rest[..close].to_ascii_uppercase();
                if let Some(c) = CALLOUTS.iter().find(|c| c.0 == kind) {
                    let body = rest[close + 1..].trim_start_matches([' ', '\t']);
                    let body = body.strip_prefix('\n').unwrap_or(body).to_string();
                    if body.trim().is_empty() {
                        kids.remove(0);
                    } else {
                        kids[0] = Block::Para(body);
                    }
                    return Block::Callout { kind: c.0, blocks: kids };
                }
            }
        }
    }
    Block::Quote(kids)
}

/* ───────────── rendering ───────────── */

/// GitHub-style heading ids.
#[derive(Default)]
pub struct Slugger {
    seen: std::collections::HashMap<String, usize>,
}

impl Slugger {
    pub fn slug(&mut self, text: &str) -> String {
        let base = slug(text);
        let n = self.seen.entry(base.clone()).or_insert(0);
        let out = if *n == 0 { base.clone() } else { format!("{base}-{n}") };
        *n += 1;
        out
    }
}

pub fn slug(text: &str) -> String {
    let lower = text.trim().to_lowercase();
    let mut s = String::with_capacity(lower.len());
    for c in lower.chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            s.push(c);
        } else if c.is_whitespace() {
            s.push('-');
        }
    }
    if s.is_empty() {
        "section".into()
    } else {
        s
    }
}

struct Renderer<'a> {
    ctx: Ctx<'a>,
    slugger: Slugger,
    codes: Vec<String>,
    headings: Vec<Heading>,
}

impl Renderer<'_> {
    fn block(&mut self, b: &Block, out: &mut String, line: Option<usize>, tight: bool) {
        let dl = line.map(|l| format!(" data-line=\"{l}\"")).unwrap_or_default();
        match b {
            Block::Heading { level, text, line: src_line } => {
                let inner = render_inline(text, &self.ctx);
                let plain = strip_tags(&inner).trim().to_string();
                let id = self.slugger.slug(&plain);
                if line.is_some() {
                    self.headings.push(Heading { depth: *level, text: plain, line: *src_line, id: id.clone() });
                }
                out.push_str(&format!(
                    "<h{level} id=\"{id}\"{dl}><a class=\"anchor\" href=\"#{id}\" aria-label=\"Link to section\">{}</a>{inner}</h{level}>\n",
                    icon("link", ""),
                    id = esc(&id)
                ));
            }
            Block::Para(t) => {
                if tight {
                    out.push_str(&render_inline(t, &self.ctx));
                } else {
                    out.push_str(&format!("<p{dl}>{}</p>\n", render_inline(t, &self.ctx)));
                }
            }
            Block::Code { lang, text } => {
                let n = self.codes.len();
                self.codes.push(text.clone());
                let l = lang.to_ascii_lowercase();
                let body = highlight(&l, text);
                let class = if l.is_empty() { "hljs".to_string() } else { format!("hljs language-{}", esc(&l)) };
                out.push_str(&format!(
                    "<div class=\"codeblock\"{dl}><div class=\"code-head\"><span class=\"lang\">{}</span><button class=\"copy\" title=\"Copy\" data-copy=\"{n}\">{}<span>Copy</span></button></div><pre><code class=\"{class}\">{body}</code></pre></div>\n",
                    if l.is_empty() { "text".to_string() } else { esc(&l) },
                    icon("copy", "")
                ));
            }
            Block::Quote(kids) => {
                out.push_str(&format!("<blockquote{dl}>\n"));
                for k in kids {
                    self.block(k, out, None, false);
                }
                out.push_str("</blockquote>\n");
            }
            Block::Callout { kind, blocks } => {
                let c = CALLOUTS.iter().find(|c| c.0 == *kind).unwrap();
                out.push_str(&format!(
                    "<blockquote class=\"callout callout-{}\"{dl}><div class=\"callout-title\">{}{}</div>\n",
                    kind.to_ascii_lowercase(),
                    icon(c.1, ""),
                    c.2
                ));
                for k in blocks {
                    self.block(k, out, None, false);
                }
                out.push_str("</blockquote>\n");
            }
            Block::List { ordered, start, tight, items } => {
                let tasks = items.iter().any(|i| i.task.is_some());
                let cls = if tasks { " class=\"tasks\"" } else { "" };
                if *ordered {
                    let st = if *start != 1 { format!(" start=\"{start}\"") } else { String::new() };
                    out.push_str(&format!("<ol{st}{cls}{dl}>\n"));
                } else {
                    out.push_str(&format!("<ul{cls}{dl}>\n"));
                }
                for it in items {
                    let box_html = it.task.map(|(checked, n)| {
                        format!("<input type=\"checkbox\" data-task=\"{n}\"{}> ", if checked { " checked" } else { "" })
                    });
                    out.push_str(if it.task.is_some() { "<li class=\"task\">" } else { "<li>" });
                    let mut first = true;
                    for k in &it.blocks {
                        if first {
                            if let Some(bx) = &box_html {
                                if !*tight && matches!(k, Block::Para(_)) {
                                    let Block::Para(t) = k else { unreachable!() };
                                    out.push_str(&format!("<p>{bx}{}</p>\n", render_inline(t, &self.ctx)));
                                    first = false;
                                    continue;
                                }
                                out.push_str(bx);
                            }
                        } else if *tight && matches!(k, Block::Para(_)) {
                            out.push('\n');
                        }
                        first = false;
                        self.block(k, out, None, *tight);
                    }
                    if first {
                        if let Some(bx) = &box_html {
                            out.push_str(bx);
                        }
                    }
                    out.push_str("</li>\n");
                }
                out.push_str(if *ordered { "</ol>\n" } else { "</ul>\n" });
            }
            Block::Table { aligns, head, rows } => {
                let al = |i: usize| match aligns.get(i) {
                    Some(Align::Center) => " class=\"al-c\"",
                    Some(Align::Right) => " class=\"al-r\"",
                    _ => "",
                };
                out.push_str(&format!("<div class=\"table-wrap\"{dl}><table>\n<thead><tr>"));
                for (i, c) in head.iter().enumerate() {
                    out.push_str(&format!("<th{}>{}</th>", al(i), render_inline(c, &self.ctx)));
                }
                out.push_str("</tr></thead>\n");
                if !rows.is_empty() {
                    out.push_str("<tbody>\n");
                    for r in rows {
                        out.push_str("<tr>");
                        for (i, c) in r.iter().enumerate() {
                            out.push_str(&format!("<td{}>{}</td>", al(i), render_inline(c, &self.ctx)));
                        }
                        out.push_str("</tr>\n");
                    }
                    out.push_str("</tbody>");
                }
                out.push_str("</table></div>\n");
            }
            Block::Hr => out.push_str(&format!("<hr{dl}>\n")),
            Block::Html(h) => {
                let clean = sanitize(h);
                if line.is_some() {
                    out.push_str(&format!("<div class=\"html-block\"{dl}>{clean}</div>\n"));
                } else {
                    out.push_str(&clean);
                    out.push('\n');
                }
            }
            Block::Front(lines) => {
                if lines.iter().all(|l| blank(l)) {
                    return;
                }
                out.push_str(&format!("<dl class=\"frontmatter\"{dl}>"));
                let mut open_dd = false;
                for l in lines {
                    if let Some((k, v)) = front_kv(l) {
                        if open_dd {
                            out.push_str("</dd>");
                        }
                        out.push_str("<dt>");
                        esc_into(out, k);
                        out.push_str("</dt><dd>");
                        let v = v.trim();
                        let v = v.strip_prefix(['"', '\'']).unwrap_or(v);
                        let v = v.strip_suffix(['"', '\'']).unwrap_or(v);
                        esc_into(out, v);
                        open_dd = true;
                    } else if open_dd && !blank(l) {
                        let t = l.trim();
                        let t = t.strip_prefix('-').map(|x| format!("• {}", x.trim_start())).unwrap_or_else(|| t.to_string());
                        out.push_str("<br>");
                        esc_into(out, &t);
                    }
                }
                if open_dd {
                    out.push_str("</dd>");
                }
                out.push_str("</dl>\n");
            }
        }
    }
}

fn front_kv(l: &str) -> Option<(&str, &str)> {
    let colon = l.find(':')?;
    let k = l[..colon].trim_end();
    if k.is_empty() || !k.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-') || l.starts_with([' ', '\t']) {
        return None;
    }
    Some((k, &l[colon + 1..]))
}

fn to_lines(text: &str) -> Vec<Line> {
    text.split('\n').enumerate().map(|(no, s)| Line { no, s: s.strip_suffix('\r').unwrap_or(s).to_string() }).collect()
}

fn parse_doc(text: &str) -> (Vec<Block>, Vec<usize>, Parser) {
    let lines = to_lines(text);
    let mut p = Parser { refs: Refs::new(), tasks: vec![] };
    let (blocks, starts, _) = p.parse(&lines, true);
    (blocks, starts, p)
}

/// Render a whole document.
pub fn render(text: &str, opts: &Options) -> Rendered {
    let (blocks, starts, p) = parse_doc(text);
    let mut r = Renderer {
        ctx: Ctx { refs: p.refs, base_dir: opts.base_dir, remote_images: opts.remote_images },
        slugger: Slugger::default(),
        codes: vec![],
        headings: vec![],
    };
    let mut html = String::with_capacity(text.len() * 2);
    let mut kept = vec![];
    for (b, &line) in blocks.iter().zip(&starts) {
        if let Block::Front(l) = b {
            if l.iter().all(|x| blank(x)) {
                continue;
            }
        }
        r.block(b, &mut html, Some(line), false);
        kept.push(line);
    }
    Rendered { html, blocks: kept, headings: r.headings, codes: r.codes, tasks: p.tasks, words: word_count(text) }
}

/// Outline only (cheaper than a full render: skips HTML for everything but headings).
pub fn headings(text: &str) -> Vec<Heading> {
    render(text, &Options::default()).headings
}

pub fn word_count(text: &str) -> usize {
    let mut n = 0;
    let mut in_word = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if !in_word {
                n += 1;
                in_word = true;
            }
        } else if !(in_word && matches!(c, '\'' | '’' | '-')) {
            in_word = false;
        }
    }
    n
}

/// Flip the `n`th task checkbox (in document order) in the source. Returns the new text.
pub fn toggle_task(text: &str, n: usize) -> Option<String> {
    let (_, _, p) = parse_doc(text);
    let line = *p.tasks.get(n)?;
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    let l = lines.get(line)?.clone();
    // The first `[ ]` / `[x]` after the list marker on that line.
    let bytes = l.as_bytes();
    let mut k = 0;
    while k + 2 < bytes.len() {
        if bytes[k] == b'[' && bytes[k + 2] == b']' && matches!(bytes[k + 1], b' ' | b'x' | b'X') {
            let flipped = if bytes[k + 1] == b' ' { "x" } else { " " };
            lines[line] = format!("{}{}{}", &l[..k + 1], flipped, &l[k + 2..]);
            return Some(lines.join("\n"));
        }
        k += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn h(s: &str) -> String {
        render(s, &Options::default()).html
    }

    #[test]
    fn headings_and_paragraphs() {
        let r = render("# Hello *world*\n\nSome text\nmore\n\nSetext\n---\n", &Options::default());
        assert!(r.html.contains("<h1 id=\"hello-world\" data-line=\"0\">"));
        assert!(r.html.contains("<p data-line=\"2\">Some text\nmore</p>"));
        assert!(r.html.contains("<h2 id=\"setext\" data-line=\"5\">"));
        assert_eq!(r.headings.len(), 2);
        assert_eq!(r.blocks, vec![0, 2, 5]);
    }

    #[test]
    fn lists_and_tasks() {
        let s = "- [ ] one\n- [x] two\n  - nested\n\n1. a\n2. b\n";
        let r = render(s, &Options::default());
        assert!(r.html.contains("<ul class=\"tasks\" data-line=\"0\">"), "{}", r.html);
        assert!(r.html.contains("<li class=\"task\"><input type=\"checkbox\" data-task=\"0\"> one"));
        assert!(r.html.contains("data-task=\"1\" checked"));
        assert!(r.html.contains("<ul>\n<li>nested</li>"), "{}", r.html);
        assert!(r.html.contains("<ol data-line=\"4\">"));
        assert_eq!(toggle_task(s, 0).unwrap(), s.replacen("- [ ] one", "- [x] one", 1));
        assert_eq!(toggle_task(s, 1).unwrap(), s.replacen("- [x] two", "- [ ] two", 1));
        let fenced = "```\n- [ ] not a task\n```\n- [ ] real";
        assert_eq!(toggle_task(fenced, 0).unwrap(), "```\n- [ ] not a task\n```\n- [x] real");
    }

    #[test]
    fn loose_list() {
        let html = h("- a\n\n- b\n");
        assert!(html.contains("<li><p>a</p>"), "{html}");
    }

    #[test]
    fn code_and_quotes() {
        let html = h("```python\ndef f():\n    return 42\n```\n\n> [!NOTE]\n> Callout body\n\n> plain\nlazy");
        assert!(html.contains("hljs-keyword"));
        assert!(html.contains("<div class=\"codeblock\" data-line=\"0\">"));
        assert!(html.contains("callout callout-note"));
        assert!(html.contains("Callout body"));
        assert!(html.contains("<blockquote data-line=\"8\">\n<p>plain\nlazy</p>"), "{html}");
    }

    #[test]
    fn tables() {
        let html = h("| A | B |\n|:-:|--:|\n| `a|b` | 2 |\n");
        assert!(html.contains("<th class=\"al-c\">A</th><th class=\"al-r\">B</th>"), "{html}");
        assert!(html.contains("<code>a|b</code>"), "{html}");
    }

    #[test]
    fn frontmatter_and_html() {
        let r = render("---\ntitle: \"Test\"\ntags:\n- a\n---\n# T\n\n<script>x</script>\n\n<div onclick=1>ok</div>", &Options::default());
        assert!(r.html.contains("<dl class=\"frontmatter\" data-line=\"0\"><dt>title</dt><dd>Test</dd><dt>tags</dt><dd><br>• a</dd></dl>"), "{}", r.html);
        assert!(r.html.contains("<h1 id=\"t\" data-line=\"5\">"));
        assert!(!r.html.contains("script"));
        assert!(r.html.contains("<div>ok</div>"));
    }

    #[test]
    fn refs_and_slugs() {
        let html = h("[x][r] and [r]\n\n[r]: https://e.com \"T\"\n\n# A\n# A");
        assert!(html.contains("<a href=\"https://e.com\" title=\"T\">x</a>"), "{html}");
        assert!(html.contains("id=\"a-1\""));
        assert_eq!(word_count("It’s a well-known fact, 42 times."), 6);
    }

    #[test]
    fn hr_vs_list() {
        let html = h("a\n\n* * *\n\n- x\n");
        assert!(html.contains("<hr data-line=\"2\">"), "{html}");
        assert!(html.contains("<ul data-line=\"4\">"));
    }
}
