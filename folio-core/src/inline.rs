//! Inline Markdown: code spans, emphasis (CommonMark delimiter algorithm), links,
//! images, autolinks, strikethrough, hard breaks, entities and sanitized inline HTML.

use crate::html::{clean_tag, entity_len, esc, esc_into, is_swallowed, parse_tag, safe_url, strip_tags};
use std::collections::HashMap;

pub type Refs = HashMap<String, (String, String)>;

/// How images resolve: relative paths become `mdasset://` URLs under `base_dir`.
#[derive(Clone, Default)]
pub struct Ctx<'a> {
    pub refs: Refs,
    pub base_dir: Option<&'a str>,
    pub remote_images: bool,
}

enum Tok {
    Html(String),
    Delim { ch: u8, n: usize, orig: usize, open: bool, close: bool, opens: Vec<&'static str>, closes: Vec<&'static str> },
}

pub fn normalize_label(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn run(b: &[u8], i: usize, c: u8) -> usize {
    b[i..].iter().take_while(|&&x| x == c).count()
}

/// Find a closing backtick run of exactly `n` starting at `from`.
fn backtick_close(b: &[u8], from: usize, n: usize) -> Option<usize> {
    let mut j = from;
    while j < b.len() {
        if b[j] == b'`' {
            let m = run(b, j, b'`');
            if m == n {
                return Some(j);
            }
            j += m;
        } else {
            j += 1;
        }
    }
    None
}

fn is_punct(c: char) -> bool {
    c.is_ascii_punctuation() || (!c.is_alphanumeric() && !c.is_whitespace() && !c.is_ascii())
}

pub fn render_inline(src: &str, ctx: &Ctx) -> String {
    let b = src.as_bytes();
    let mut toks: Vec<Tok> = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    macro_rules! flush {
        () => {
            if !text.is_empty() {
                toks.push(Tok::Html(std::mem::take(&mut text)));
            }
        };
    }
    while i < b.len() {
        let c = b[i];
        match c {
            b'\\' => {
                if b.get(i + 1) == Some(&b'\n') {
                    flush!();
                    toks.push(Tok::Html("<br>\n".into()));
                    i += 2;
                } else if b.get(i + 1).is_some_and(|x| x.is_ascii_punctuation()) {
                    esc_into(&mut text, &src[i + 1..i + 2]);
                    i += 2;
                } else {
                    text.push('\\');
                    i += 1;
                }
            }
            b'`' => {
                let n = run(b, i, b'`');
                if let Some(end) = backtick_close(b, i + n, n) {
                    let mut code = src[i + n..end].replace('\n', " ");
                    if code.len() >= 2 && code.starts_with(' ') && code.ends_with(' ') && code.trim() != "" {
                        code = code[1..code.len() - 1].to_string();
                    }
                    flush!();
                    toks.push(Tok::Html(format!("<code>{}</code>", esc(&code))));
                    i = end + n;
                } else {
                    text.push_str(&"`".repeat(n));
                    i += n;
                }
            }
            b'!' if b.get(i + 1) == Some(&b'[') => {
                if let Some((html, len)) = try_link(src, i + 1, ctx, true) {
                    flush!();
                    toks.push(Tok::Html(html));
                    i += 1 + len;
                } else {
                    text.push('!');
                    i += 1;
                }
            }
            b'[' => {
                if let Some((html, len)) = try_link(src, i, ctx, false) {
                    flush!();
                    toks.push(Tok::Html(html));
                    i += len;
                } else {
                    text.push('[');
                    i += 1;
                }
            }
            b'<' => {
                let rest = &src[i..];
                if let Some((html, len)) = autolink(rest) {
                    flush!();
                    toks.push(Tok::Html(html));
                    i += len;
                } else if rest.starts_with("<!--") {
                    i += rest.find("-->").map(|e| e + 3).unwrap_or(rest.len());
                } else if let Some(t) = parse_tag(rest) {
                    flush!();
                    if let Some(clean) = clean_tag(&t) {
                        toks.push(Tok::Html(clean));
                    } else if !t.close && is_swallowed(&t.name) {
                        // drop the tag; its text content stays escaped
                    }
                    i += t.len;
                } else {
                    text.push_str("&lt;");
                    i += 1;
                }
            }
            b'&' => {
                if let Some(n) = entity_len(&src[i..]) {
                    text.push_str(&src[i..i + n]);
                    i += n;
                } else {
                    text.push_str("&amp;");
                    i += 1;
                }
            }
            b'*' | b'_' | b'~' => {
                let n = run(b, i, c);
                let prev = src[..i].chars().next_back();
                let next = src[i + n..].chars().next();
                let ws = |x: Option<char>| x.map_or(true, |c| c.is_whitespace());
                let pu = |x: Option<char>| x.is_some_and(is_punct);
                let left = !ws(next) && (!pu(next) || ws(prev) || pu(prev));
                let right = !ws(prev) && (!pu(prev) || ws(next) || pu(next));
                let (open, close) = match c {
                    b'_' => (left && (!right || pu(prev)), right && (!left || pu(next))),
                    b'~' if n != 2 => (false, false),
                    _ => (left, right),
                };
                flush!();
                toks.push(Tok::Delim { ch: c, n, orig: n, open, close, opens: vec![], closes: vec![] });
                i += n;
            }
            b'\n' => {
                let trimmed = text.trim_end_matches(' ').len();
                let hard = text.len() - trimmed >= 2;
                text.truncate(trimmed);
                if hard {
                    flush!();
                    toks.push(Tok::Html("<br>\n".into()));
                } else {
                    text.push('\n');
                }
                i += 1;
            }
            b'h' | b'w' if src[..i].chars().next_back().map_or(true, |p| !p.is_alphanumeric()) => {
                if let Some((html, len)) = bare_url(&src[i..]) {
                    flush!();
                    toks.push(Tok::Html(html));
                    i += len;
                } else {
                    text.push(c as char);
                    i += 1;
                }
            }
            _ => {
                let ch = src[i..].chars().next().unwrap();
                match ch {
                    '>' => text.push_str("&gt;"),
                    '"' => text.push_str("&quot;"),
                    _ => text.push(ch),
                }
                i += ch.len_utf8();
            }
        }
    }
    flush!();
    emphasis(&mut toks);
    let mut out = String::with_capacity(src.len() + 16);
    for t in toks {
        match t {
            Tok::Html(h) => out.push_str(&h),
            Tok::Delim { ch, n, opens, closes, .. } => {
                for c in closes {
                    out.push_str(c);
                }
                for _ in 0..n {
                    out.push(ch as char);
                }
                for o in opens {
                    out.push_str(o);
                }
            }
        }
    }
    out
}

fn emphasis(toks: &mut [Tok]) {
    let mut c = 0;
    while c < toks.len() {
        let (cch, cn, corig, copen) = match &toks[c] {
            Tok::Delim { ch, n, orig, open, close: true, .. } if *n > 0 => (*ch, *n, *orig, *open),
            _ => {
                c += 1;
                continue;
            }
        };
        let mut found = None;
        let mut o = c;
        while o > 0 {
            o -= 1;
            if let Tok::Delim { ch, n, orig, open: true, close, .. } = &toks[o] {
                if *ch != cch || *n == 0 {
                    continue;
                }
                if (*close || copen) && (orig + corig) % 3 == 0 && !(orig % 3 == 0 && corig % 3 == 0) {
                    continue;
                }
                if cch == b'~' && (*n < 2 || cn < 2) {
                    continue;
                }
                found = Some(o);
                break;
            }
        }
        let Some(o) = found else {
            c += 1;
            continue;
        };
        let on = if let Tok::Delim { n, .. } = &toks[o] { *n } else { 0 };
        let used = if cch == b'~' || (on >= 2 && cn >= 2) { 2 } else { 1 };
        let (open_tag, close_tag) = match (cch, used) {
            (b'~', _) => ("<del>", "</del>"),
            (_, 2) => ("<strong>", "</strong>"),
            _ => ("<em>", "</em>"),
        };
        if let Tok::Delim { n, opens, .. } = &mut toks[o] {
            *n -= used;
            opens.insert(0, open_tag);
        }
        if let Tok::Delim { n, closes, .. } = &mut toks[c] {
            *n -= used;
            closes.push(close_tag);
        }
        for t in toks.iter_mut().take(c).skip(o + 1) {
            if let Tok::Delim { open, close, .. } = t {
                *open = false;
                *close = false;
            }
        }
        if matches!(&toks[c], Tok::Delim { n: 0, .. }) {
            c += 1;
        }
    }
}

fn try_link(src: &str, at: usize, ctx: &Ctx, image: bool) -> Option<(String, usize)> {
    let b = src.as_bytes();
    let start = at + 1;
    let mut depth = 0usize;
    let mut j = start;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'`' => {
                let n = run(b, j, b'`');
                j = backtick_close(b, j + n, n).map(|e| e + n).unwrap_or(j + n);
            }
            b'[' => {
                depth += 1;
                j += 1;
            }
            b']' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                j += 1;
            }
            _ => j += 1,
        }
    }
    if j >= b.len() {
        return None;
    }
    let label = &src[start..j];
    let after = j + 1;
    if b.get(after) == Some(&b'(') {
        if let Some((dest, title, end)) = inline_dest(src, after + 1) {
            return Some((make_link(label, &dest, &title, image, ctx), end - at));
        }
    }
    let (key, end) = if b.get(after) == Some(&b'[') {
        let close = src[after + 1..].find(']')? + after + 1;
        let k = &src[after + 1..close];
        (if k.trim().is_empty() { label } else { k }, close + 1)
    } else {
        (label, after)
    };
    let (url, title) = ctx.refs.get(&normalize_label(key))?;
    Some((make_link(label, url, title, image, ctx), end - at))
}

fn inline_dest(src: &str, mut k: usize) -> Option<(String, String, usize)> {
    let b = src.as_bytes();
    let skip_ws = |k: &mut usize| {
        while *k < b.len() && (b[*k] == b' ' || b[*k] == b'\t' || b[*k] == b'\n') {
            *k += 1;
        }
    };
    skip_ws(&mut k);
    let dest;
    if b.get(k) == Some(&b'<') {
        let e = src[k + 1..].find(['>', '\n'])? + k + 1;
        if b[e] != b'>' {
            return None;
        }
        dest = src[k + 1..e].to_string();
        k = e + 1;
    } else {
        let s = k;
        let mut depth = 0i32;
        while k < b.len() && !b[k].is_ascii_whitespace() {
            match b[k] {
                b'\\' => k += 1,
                b'(' => depth += 1,
                b')' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            k += 1;
        }
        dest = src[s..k.min(b.len())].to_string();
    }
    skip_ws(&mut k);
    let mut title = String::new();
    if let Some(&q) = b.get(k) {
        if q == b'"' || q == b'\'' || q == b'(' {
            let endq = if q == b'(' { ')' } else { q as char };
            let e = src[k + 1..].find(endq)? + k + 1;
            title = src[k + 1..e].to_string();
            k = e + 1;
            skip_ws(&mut k);
        }
    }
    if b.get(k) != Some(&b')') {
        return None;
    }
    Some((unbackslash(&dest), unbackslash(&title), k + 1))
}

fn unbackslash(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' && it.peek().is_some_and(|n| n.is_ascii_punctuation()) {
            out.push(it.next().unwrap());
        } else {
            out.push(c);
        }
    }
    out
}

/// Resolve image sources: relative paths go through the local `mdasset://` scheme.
pub fn image_src(url: &str, ctx: &Ctx) -> Result<String, &'static str> {
    let u = url.trim();
    let lower = u.to_ascii_lowercase();
    if lower.starts_with("data:image/") {
        return safe_url(u, true).ok_or("unsafe");
    }
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return if ctx.remote_images { Ok(u.to_string()) } else { Err("remote") };
    }
    if lower.contains(':') && lower.find(':') < lower.find('/') {
        return Err("unsafe");
    }
    let Some(base) = ctx.base_dir else { return Err("unsaved") };
    let path = u.split(['?', '#']).next().unwrap_or("");
    let path = crate::html::percent_decode(path);
    let abs = if path.starts_with('/') { normalize_path(&path) } else { normalize_path(&format!("{base}/{path}")) };
    Ok(format!("mdasset://local{}", crate::html::encode_path(&abs)))
}

pub fn normalize_path(p: &str) -> String {
    let mut out: Vec<&str> = vec![];
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    format!("/{}", out.join("/"))
}

fn make_link(label: &str, dest: &str, title: &str, image: bool, ctx: &Ctx) -> String {
    let title_attr = if title.is_empty() { String::new() } else { format!(" title=\"{}\"", esc(title)) };
    if image {
        let alt = strip_tags(&render_inline(label, ctx));
        return match image_src(dest, ctx) {
            Ok(src) => format!("<img src=\"{}\" alt=\"{}\"{} loading=\"lazy\">", esc(&src), esc(&alt), title_attr),
            Err("remote") => format!("<span class=\"img-blocked\" title=\"{}\">Remote image blocked{}</span>", esc(dest), if alt.is_empty() { String::new() } else { format!(" — {}", esc(&alt)) }),
            Err(_) => format!("<span class=\"img-blocked\">{}</span>", esc(if alt.is_empty() { dest } else { &alt })),
        };
    }
    let inner = render_inline(label, ctx);
    match safe_url(dest, false) {
        Some(u) => format!("<a href=\"{}\"{}>{}</a>", esc(&u), title_attr, inner),
        None => inner,
    }
}

fn autolink(rest: &str) -> Option<(String, usize)> {
    let end = rest.find('>')?;
    let inner = &rest[1..end];
    if inner.is_empty() || inner.contains([' ', '<', '\n']) {
        return None;
    }
    if let Some(colon) = inner.find(':') {
        let scheme = &inner[..colon];
        if (2..=32).contains(&scheme.len()) && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c)) && scheme.chars().next()?.is_ascii_alphabetic() {
            let href = safe_url(inner, false)?;
            return Some((format!("<a href=\"{}\">{}</a>", esc(&href), esc(inner)), end + 1));
        }
    }
    if inner.contains('@') && inner.contains('.') && !inner.starts_with('@') {
        return Some((format!("<a href=\"mailto:{0}\">{0}</a>", esc(inner)), end + 1));
    }
    None
}

fn bare_url(rest: &str) -> Option<(String, usize)> {
    let www = rest.starts_with("www.");
    if !(rest.starts_with("https://") || rest.starts_with("http://") || www) {
        return None;
    }
    let mut end = rest.find(|c: char| c.is_whitespace() || c == '<').unwrap_or(rest.len());
    loop {
        let s = &rest[..end];
        let last = s.chars().next_back()?;
        if "?!.,:*_~'\"".contains(last) {
            end -= last.len_utf8();
        } else if last == ')' && s.matches('(').count() < s.matches(')').count() {
            end -= 1;
        } else {
            break;
        }
    }
    let text = &rest[..end];
    let min = if www { 5 } else { 9 };
    if text.len() < min {
        return None;
    }
    let href = if www { format!("http://{text}") } else { text.to_string() };
    Some((format!("<a href=\"{}\">{}</a>", esc(&href), esc(text)), end))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(s: &str) -> String {
        render_inline(s, &Ctx::default())
    }
    #[test]
    fn emphasis_and_code() {
        assert_eq!(r("**bold** and *em*"), "<strong>bold</strong> and <em>em</em>");
        assert_eq!(r("***both***"), "<em><strong>both</strong></em>");
        assert_eq!(r("snake_case_name"), "snake_case_name");
        assert_eq!(r("`a < b`"), "<code>a &lt; b</code>");
        assert_eq!(r("~~gone~~"), "<del>gone</del>");
        assert_eq!(r("2 * 3 * 4"), "2 * 3 * 4");
    }
    #[test]
    fn links() {
        assert_eq!(r("[x](https://a.com \"T\")"), "<a href=\"https://a.com\" title=\"T\">x</a>");
        assert_eq!(r("[x](javascript:alert(1))"), "x");
        assert_eq!(r("see https://a.com/x."), "see <a href=\"https://a.com/x\">https://a.com/x</a>.");
        assert_eq!(r("<b onclick=1>hi</b><script>"), "<b>hi</b>");
        assert_eq!(r("[**b**](x.md)"), "<a href=\"x.md\"><strong>b</strong></a>");
        assert!(r("![a](https://x/y.png)").contains("img-blocked"));
    }
}
