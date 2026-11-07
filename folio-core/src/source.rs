//! The Markdown source editor's syntax layer (one `.ln` div per line, `t-*` classes),
//! plus find-match marking for both the source layer and rendered HTML.

use crate::html::{esc, esc_into};

/* ───────────── inline tokens in source lines ───────────── */

fn punc(out: &mut String, s: &str) {
    out.push_str("<span class=\"t-punc\">");
    esc_into(out, s);
    out.push_str("</span>");
}

fn is_ws(c: char) -> bool {
    c.is_whitespace()
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Lazily find the end of `delim…content…delim` where content's last char satisfies `last_ok`.
fn find_closing(s: &str, from: usize, delim: &str, last_ok: impl Fn(char) -> bool, after_ok: impl Fn(Option<char>) -> bool) -> Option<usize> {
    let mut j = from;
    // Content must be at least one char.
    let first = s[from..].chars().next()?;
    j += first.len_utf8();
    loop {
        let k = s[j..].find(delim)? + j;
        let last = s[..k].chars().next_back()?;
        if k > from && last_ok(last) && after_ok(s[k + delim.len()..].chars().next()) {
            return Some(k);
        }
        j = k + s[k..].chars().next()?.len_utf8();
        if j >= s.len() {
            return None;
        }
    }
}

/// Try each token kind at byte `p` (in order). Returns (end, html).
fn token_at(s: &str, p: usize) -> Option<(usize, String)> {
    let rest = &s[p..];
    let c = rest.chars().next()?;
    let mut out = String::new();
    match c {
        '`' => {
            let n = rest.chars().take_while(|&x| x == '`').count();
            for k in (1..=n).rev() {
                let fence = "`".repeat(k);
                let start = p + k;
                if start >= s.len() {
                    continue;
                }
                let first = s[start..].chars().next()?.len_utf8();
                if let Some(e) = s[start + first..].find(&fence) {
                    let e = start + first + e;
                    out.push_str("<span class=\"t-code\">");
                    punc(&mut out, &fence);
                    esc_into(&mut out, &s[start..e]);
                    punc(&mut out, &fence);
                    out.push_str("</span>");
                    return Some((e + k, out));
                }
            }
        }
        '!' | '[' => {
            let open = if c == '!' { "![" } else { "[" };
            if rest.starts_with(open) {
                let a = p + open.len();
                let close = s[a..].find(']')? + a;
                if s[a..close].contains('\n') || !s[close..].starts_with("](") {
                    return None;
                }
                let u = close + 2;
                let ue = s[u..].find(|x: char| x == ')' || x.is_whitespace()).map(|e| e + u).unwrap_or(s.len());
                // Optional title: whitespace + "…", then ).
                let mut t = ue;
                let ws_end = t + s[t..].find(|x: char| !x.is_whitespace()).unwrap_or(s.len() - t);
                if ws_end > t && s[ws_end..].starts_with('"') {
                    if let Some(q) = s[ws_end + 1..].find('"') {
                        t = ws_end + 1 + q + 1;
                    }
                }
                if !s[t..].starts_with(')') {
                    return None;
                }
                out.push_str("<span class=\"t-link\">");
                punc(&mut out, open);
                out.push_str("<span class=\"t-ltext\">");
                esc_into(&mut out, &s[a..close]);
                out.push_str("</span>");
                punc(&mut out, "](");
                out.push_str("<span class=\"t-url\">");
                esc_into(&mut out, &s[u..ue]);
                out.push_str("</span>");
                punc(&mut out, &s[ue..t + 1]);
                out.push_str("</span>");
                return Some((t + 1, out));
            }
        }
        '*' | '_' => {
            let dbl: String = [c, c].iter().collect();
            if rest.starts_with(&dbl) && rest[2..].starts_with(|x: char| !is_ws(x)) {
                if let Some(k) = find_closing(s, p + 2, &dbl, |l| !is_ws(l), |_| true) {
                    out.push_str("<span class=\"t-strong\">");
                    punc(&mut out, &dbl);
                    out.push_str(&inline(&s[p + 2..k]));
                    punc(&mut out, &dbl);
                    out.push_str("</span>");
                    return Some((k + 2, out));
                }
            }
            let one = c.to_string();
            if rest[1..].starts_with(|x: char| !is_ws(x) && x != '*' && x != '_') {
                if let Some(k) = find_closing(s, p + 1, &one, |l| !is_ws(l) && l != '\\', |a| !a.is_some_and(|x| word(x) || x == '*')) {
                    out.push_str("<span class=\"t-em\">");
                    punc(&mut out, &one);
                    out.push_str(&inline(&s[p + 1..k]));
                    punc(&mut out, &one);
                    out.push_str("</span>");
                    return Some((k + 1, out));
                }
            }
        }
        '~' => {
            if rest.starts_with("~~") && rest.len() > 2 {
                let first = rest[2..].chars().next()?.len_utf8();
                if let Some(k) = rest[2 + first..].find("~~") {
                    let k = p + 2 + first + k;
                    out.push_str("<span class=\"t-strike\">");
                    punc(&mut out, "~~");
                    out.push_str(&inline(&s[p + 2..k]));
                    punc(&mut out, "~~");
                    out.push_str("</span>");
                    return Some((k + 2, out));
                }
            }
        }
        '<' => {
            let r = rest.strip_prefix("</").unwrap_or(&rest[1..]);
            if r.starts_with(|x: char| x.is_ascii_alphabetic()) {
                if let Some(e) = rest.find('>') {
                    if !rest[..e].contains('\n') {
                        out.push_str("<span class=\"t-html\">");
                        esc_into(&mut out, &rest[..e + 1]);
                        out.push_str("</span>");
                        return Some((p + e + 1, out));
                    }
                }
            }
        }
        'h' => {
            if rest.starts_with("https://") || rest.starts_with("http://") {
                let skip = if rest.starts_with("https://") { 8 } else { 7 };
                let e = rest[skip..].find(|x: char| x.is_whitespace() || ")<>]".contains(x)).map(|e| e + skip).unwrap_or(rest.len());
                if e > skip {
                    out.push_str("<span class=\"t-url\">");
                    esc_into(&mut out, &rest[..e]);
                    out.push_str("</span>");
                    return Some((p + e, out));
                }
            }
        }
        '\\' => {
            if let Some(n) = rest[1..].chars().next() {
                if "\\`*_{}[]()#+-.!|>~".contains(n) {
                    punc(&mut out, &rest[..1 + n.len_utf8()]);
                    return Some((p + 1 + n.len_utf8(), out));
                }
            }
        }
        _ => {}
    }
    None
}

/// Highlight inline Markdown in one source line.
pub fn inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut last = 0;
    let mut p = 0;
    while p < s.len() {
        if let Some((end, html)) = token_at(s, p) {
            esc_into(&mut out, &s[last..p]);
            out.push_str(&html);
            p = end;
            last = end;
        } else {
            p += s[p..].chars().next().unwrap().len_utf8();
        }
    }
    esc_into(&mut out, &s[last..]);
    out
}

/* ───────────── block structure per line ───────────── */

fn lead(s: &str, max: usize) -> usize {
    let n = s.bytes().take_while(|&b| b == b' ').count();
    if n <= max {
        n
    } else {
        usize::MAX
    }
}

/// One `<div class="ln">` per source line, with Markdown syntax classes.
pub fn highlight_source(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = String::with_capacity(text.len() * 2 + lines.len() * 24);
    let mut fence: Option<String> = None;
    let mut front = lines.first() == Some(&"---");
    for (i, &l) in lines.iter().enumerate() {
        let mut cls = "";
        let mut html = String::new();
        if front {
            cls = "l-meta";
            let t = l.trim_end();
            if i > 0 && (t == "---" || t == "...") {
                front = false;
                punc(&mut html, l);
            } else if i == 0 {
                punc(&mut html, l);
            } else {
                // `  - key:` → key highlighted
                let ws = l.len() - l.trim_start_matches([' ', '\t']).len();
                let mut k = ws;
                if l[k..].starts_with('-') {
                    k += 1;
                    k += l[k..].len() - l[k..].trim_start_matches([' ', '\t']).len();
                }
                let key_len = l[k..].find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.' || c == '-')).unwrap_or(l.len() - k);
                if key_len > 0 && l[k + key_len..].starts_with(':') {
                    esc_into(&mut html, &l[..k]);
                    html.push_str("<span class=\"t-key\">");
                    esc_into(&mut html, &l[k..k + key_len]);
                    html.push_str("</span><span class=\"t-punc\">:</span>");
                    esc_into(&mut html, &l[k + key_len + 1..]);
                } else {
                    esc_into(&mut html, l);
                }
            }
        } else if let Some(f) = &fence {
            cls = "l-code";
            let t = l.trim();
            if t.len() >= f.len() && t.chars().all(|c| Some(c) == f.chars().next()) && l.starts_with([' ', '`', '~', '\t']) {
                html.push_str("<span class=\"t-fence\">");
                esc_into(&mut html, l);
                html.push_str("</span>");
                fence = None;
            } else {
                html.push_str("<span class=\"t-cb\">");
                esc_into(&mut html, l);
                html.push_str("</span>");
            }
        } else if let Some(ind) = Some(lead(l, 3)).filter(|&n| n != usize::MAX && (l[n..].starts_with("```") || l[n..].starts_with("~~~"))) {
            let ch = l[ind..].chars().next().unwrap();
            let n = l[ind..].chars().take_while(|&c| c == ch).count();
            fence = Some(ch.to_string().repeat(n));
            cls = "l-code";
            html.push_str(&l[..ind]);
            html.push_str("<span class=\"t-fence\">");
            esc_into(&mut html, &l[ind..ind + n]);
            html.push_str("</span><span class=\"t-lang\">");
            esc_into(&mut html, &l[ind + n..]);
            html.push_str("</span>");
        } else if let Some((ind, n)) = Some(lead(l, 3)).filter(|&n| n != usize::MAX).and_then(|ind| {
            let n = l[ind..].bytes().take_while(|&b| b == b'#').count();
            ((1..=6).contains(&n) && (l.len() == ind + n || l[ind + n..].starts_with(char::is_whitespace))).then_some((ind, n))
        }) {
            html.push_str(&l[..ind]);
            html.push_str(&format!("<span class=\"t-h t-h{n}\"><span class=\"t-hmark\">{}</span>{}</span>", &l[ind..ind + n], inline(&l[ind + n..])));
        } else if is_rule(l) {
            html.push_str("<span class=\"t-hr\">");
            esc_into(&mut html, l);
            html.push_str("</span>");
        } else if let Some(q) = quote_prefix(l) {
            html.push_str("<span class=\"t-quote-mark\">");
            esc_into(&mut html, &l[..q]);
            html.push_str("</span><span class=\"t-quote\">");
            html.push_str(&inline(&l[q..]));
            html.push_str("</span>");
        } else if let Some((ws, m, sp, task)) = list_prefix(l) {
            html.push_str(&l[..ws]);
            html.push_str("<span class=\"t-list\">");
            esc_into(&mut html, &l[ws..ws + m]);
            html.push_str("</span>");
            html.push_str(&l[ws + m..ws + m + sp]);
            let after = ws + m + sp;
            if task {
                html.push_str("<span class=\"t-task\">");
                esc_into(&mut html, &l[after..after + 3]);
                html.push_str("</span>");
                html.push_str(&inline(&l[after + 3..]));
            } else {
                html.push_str(&inline(&l[after..]));
            }
        } else if l.trim_start().starts_with('|') || is_table_delim(l) {
            for (j, part) in l.split('|').enumerate() {
                if j > 0 {
                    html.push_str("<span class=\"t-pipe\">|</span>");
                }
                let t = part.trim();
                if !t.is_empty() && t.trim_matches(':').chars().all(|c| c == '-') && t.contains('-') {
                    punc(&mut html, part);
                } else {
                    html.push_str(&inline(part));
                }
            }
        } else if let Some((a, b, c)) = ref_line(l) {
            html.push_str("<span class=\"t-ltext\">");
            esc_into(&mut html, &l[..a]);
            html.push_str("</span>");
            html.push_str(&l[a..b]);
            html.push_str("<span class=\"t-url\">");
            esc_into(&mut html, &l[b..c]);
            html.push_str("</span>");
            esc_into(&mut html, &l[c..]);
        } else {
            html = inline(l);
        }
        out.push_str("<div class=\"ln");
        if !cls.is_empty() {
            out.push(' ');
            out.push_str(cls);
        }
        out.push_str("\">");
        out.push_str(if html.is_empty() { "<br>" } else { &html });
        out.push_str("</div>");
    }
    out
}

fn is_rule(l: &str) -> bool {
    let ind = lead(l, 3);
    if ind == usize::MAX {
        return false;
    }
    let t = l[ind..].trim_end();
    let Some(ch) = t.chars().next() else { return false };
    matches!(ch, '-' | '*' | '_') && t.chars().filter(|&c| c == ch).count() >= 3 && t.chars().all(|c| c == ch || c == ' ' || c == '\t')
}

fn quote_prefix(l: &str) -> Option<usize> {
    let mut i = l.len() - l.trim_start().len();
    if !l[i..].starts_with('>') {
        return None;
    }
    while l[i..].starts_with('>') {
        i += 1;
        if l[i..].starts_with(' ') {
            i += 1;
        }
        let ws = l[i..].len() - l[i..].trim_start().len();
        if l[i + ws..].starts_with('>') {
            i += ws;
        }
    }
    Some(i)
}

/// (leading ws, marker len, spaces len, has task box)
fn list_prefix(l: &str) -> Option<(usize, usize, usize, bool)> {
    let ws = l.len() - l.trim_start().len();
    let r = &l[ws..];
    let m = if r.starts_with(['-', '*', '+']) {
        1
    } else {
        let d = r.bytes().take_while(u8::is_ascii_digit).count();
        if d == 0 || !r[d..].starts_with(['.', ')']) {
            return None;
        }
        d + 1
    };
    let sp = r[m..].len() - r[m..].trim_start().len();
    if sp == 0 {
        return None;
    }
    let after = &r[m + sp..];
    let b = after.as_bytes();
    let task = b.len() >= 3 && b[0] == b'[' && b[2] == b']' && matches!(b[1], b' ' | b'x' | b'X') && (b.len() == 3 || b[3].is_ascii_whitespace());
    Some((ws, m, sp, task))
}

fn is_table_delim(l: &str) -> bool {
    let t = l.trim_start();
    let t = t.strip_prefix(':').unwrap_or(t);
    let n = t.bytes().take_while(|&b| b == b'-').count();
    n >= 3 && t[n..].trim_start_matches(':').trim_start().starts_with('|')
}

fn ref_line(l: &str) -> Option<(usize, usize, usize)> {
    let ws = l.len() - l.trim_start().len();
    if !l[ws..].starts_with('[') {
        return None;
    }
    let close = l[ws..].find("]:")? + ws;
    if close == ws + 1 {
        return None;
    }
    let a = close + 2;
    let b = a + (l[a..].len() - l[a..].trim_start().len());
    let c = b + l[b..].find(char::is_whitespace).unwrap_or(l.len() - b);
    (c > b).then_some((a, b, c))
}

/* ───────────── find marking ───────────── */

/// The source find layer: same line structure as the highlight, with `<mark data-i>`
/// around each match (byte ranges into `text`, sorted, non-overlapping).
pub fn find_layer(text: &str, ranges: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(text.len() + ranges.len() * 24);
    let mut line = String::new();
    let mut open: Option<usize> = None;
    let mut r = 0;
    let mut last = 0;
    let flush_line = |out: &mut String, line: &mut String| {
        out.push_str("<div class=\"ln\">");
        out.push_str(if line.is_empty() { "<br>" } else { line });
        out.push_str("</div>");
        line.clear();
    };
    // Walk boundaries: match starts/ends and newlines.
    let bytes = text.as_bytes();
    let mut i = 0;
    while i <= bytes.len() {
        let at_start = open.is_none() && r < ranges.len() && ranges[r].0 == i;
        let at_end = open.is_some() && ranges[open.unwrap()].1 == i;
        let nl = i < bytes.len() && bytes[i] == b'\n';
        if at_end || at_start || nl || i == bytes.len() {
            esc_into(&mut line, &text[last..i]);
            last = i;
        }
        if at_end {
            line.push_str("</mark>");
            open = None;
            r += 1;
        }
        if open.is_none() && r < ranges.len() && ranges[r].0 == i && ranges[r].1 > i {
            line.push_str(&format!("<mark data-i=\"{r}\">"));
            open = Some(r);
        }
        if i == bytes.len() {
            if open.is_some() {
                line.push_str("</mark>");
            }
            flush_line(&mut out, &mut line);
            break;
        }
        if nl {
            if open.is_some() {
                line.push_str("</mark>");
            }
            flush_line(&mut out, &mut line);
            last = i + 1;
            if let Some(o) = open {
                line.push_str(&format!("<mark data-i=\"{o}\">"));
            }
        }
        i += 1;
    }
    out
}

/// Wrap matches in the text content of rendered HTML with `<mark class="find-hit">`,
/// skipping code-block headers and heading anchors. `find` returns byte ranges into
/// decoded text. Returns the new HTML and, per mark, the `data-line` of its top block.
pub fn mark_html(html: &str, find: &mut dyn FnMut(&str) -> Vec<(usize, usize)>, limit: usize) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(html.len() + 256);
    let mut marks = vec![];
    let mut skip_depth: Option<(String, usize)> = None;
    let mut depth = 0usize;
    let mut top_line = 0usize;
    let mut i = 0;
    let b = html.as_bytes();
    while i < b.len() {
        if b[i] == b'<' {
            let e = html[i..].find('>').map(|e| i + e + 1).unwrap_or(html.len());
            let tag = &html[i..e];
            out.push_str(tag);
            i = e;
            let close = tag.starts_with("</");
            let name: String = tag[if close { 2 } else { 1 }..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let void = matches!(name.as_str(), "br" | "hr" | "img" | "input" | "wbr" | "path" | "circle" | "rect") || tag.ends_with("/>");
            if close {
                depth = depth.saturating_sub(1);
                if let Some((n, d)) = &skip_depth {
                    if *n == name && *d == depth {
                        skip_depth = None;
                    }
                }
            } else if !void {
                if depth == 0 {
                    if let Some(p) = tag.find("data-line=\"") {
                        top_line = tag[p + 11..].split('"').next().and_then(|x| x.parse().ok()).unwrap_or(top_line);
                    }
                }
                if skip_depth.is_none() && (tag.contains("class=\"code-head") || tag.contains("class=\"anchor") || name == "svg") {
                    skip_depth = Some((name.clone(), depth));
                }
                depth += 1;
            }
            continue;
        }
        let e = html[i..].find('<').map(|e| i + e).unwrap_or(html.len());
        let raw = &html[i..e];
        i = e;
        if skip_depth.is_some() || marks.len() >= limit {
            out.push_str(raw);
            continue;
        }
        let text = crate::html::unescape(raw);
        let ranges = find(&text);
        if ranges.is_empty() {
            out.push_str(raw);
            continue;
        }
        let mut last = 0;
        for (s, t) in ranges {
            if marks.len() >= limit {
                break;
            }
            out.push_str(&esc(&text[last..s]));
            out.push_str(&format!("<mark class=\"find-hit\" data-f=\"{}\">", marks.len()));
            out.push_str(&esc(&text[s..t]));
            out.push_str("</mark>");
            marks.push(top_line);
            last = t;
        }
        out.push_str(&esc(&text[last..]));
    }
    (out, marks)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_lines() {
        let h = highlight_source("# Title\n- [ ] task **b**\n```js\nx\n```\n> quote");
        assert!(h.starts_with("<div class=\"ln\"><span class=\"t-h t-h1\"><span class=\"t-hmark\">#</span> Title</span></div>"), "{h}");
        assert!(h.contains("<span class=\"t-task\">[ ]</span>"));
        assert!(h.contains("<span class=\"t-strong\">"));
        assert!(h.contains("<div class=\"ln l-code\"><span class=\"t-cb\">x</span></div>"));
        assert!(h.contains("t-quote-mark"));
        assert_eq!(h.matches("class=\"ln").count(), 6);
        assert!(inline("`a` [t](u \"x\") *e* snake_case").contains("t-em"));
        assert!(!inline("snake_case_name").contains("t-em"));
    }
    #[test]
    fn find_marks() {
        let l = find_layer("ab\ncab", &[(0, 1), (4, 5)]);
        assert_eq!(l, "<div class=\"ln\"><mark data-i=\"0\">a</mark>b</div><div class=\"ln\">c<mark data-i=\"1\">a</mark>b</div>");
        let (h, m) = mark_html("<p data-line=\"3\">a &amp; a</p><div class=\"code-head\">a</div>", &mut |t: &str| t.match_indices('a').map(|(i, _)| (i, i + 1)).collect(), 100);
        assert_eq!(h, "<p data-line=\"3\"><mark class=\"find-hit\" data-f=\"0\">a</mark> &amp; <mark class=\"find-hit\" data-f=\"1\">a</mark></p><div class=\"code-head\">a</div>");
        assert_eq!(m, vec![3, 3]);
    }
}
