//! HTML escaping, URL safety and a whitelist sanitizer for raw HTML found in Markdown.

/// Escape text for HTML element content and attribute values.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    esc_into(&mut out, s);
    out
}

pub fn esc_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

/// Length of an HTML entity (`&amp;`, `&#123;`, `&#x1F;`) at the start of `s`, if any.
pub fn entity_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if b.first() != Some(&b'&') {
        return None;
    }
    let mut i = 1;
    if b.get(1) == Some(&b'#') {
        i = 2;
        let hex = matches!(b.get(2), Some(b'x') | Some(b'X'));
        if hex {
            i = 3;
        }
        let start = i;
        while i < b.len() && i - start < 7 && (if hex { b[i].is_ascii_hexdigit() } else { b[i].is_ascii_digit() }) {
            i += 1;
        }
        if i == start {
            return None;
        }
    } else {
        let start = i;
        while i < b.len() && i - start < 32 && b[i].is_ascii_alphanumeric() {
            i += 1;
        }
        if i == start || !b[start].is_ascii_alphabetic() {
            return None;
        }
    }
    (b.get(i) == Some(&b';')).then_some(i + 1)
}

/// Escape text but keep valid entities intact (used for raw HTML text).
pub fn esc_keep_entities(out: &mut String, s: &str) {
    let mut i = 0;
    while i < s.len() {
        let c = s[i..].chars().next().unwrap();
        if c == '&' {
            if let Some(n) = entity_len(&s[i..]) {
                out.push_str(&s[i..i + n]);
                i += n;
                continue;
            }
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
        i += c.len_utf8();
    }
}

/// Decode the handful of entities our own renderer emits (for plain-text extraction).
pub fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&nbsp;", " ").replace("&amp;", "&")
}

/// Strip tags from rendered HTML, leaving decoded text.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    unescape(&out)
}

/// Returns the URL if it is safe to put in href/src.
pub fn safe_url(raw: &str, image: bool) -> Option<String> {
    let u = raw.trim();
    let lower: String = u.chars().filter(|c| !c.is_control() && !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    let scheme_end = lower.find(':');
    let first_sep = lower.find(['/', '?', '#']);
    let has_scheme = matches!((scheme_end, first_sep), (Some(c), Some(s)) if c < s) || (scheme_end.is_some() && first_sep.is_none());
    if has_scheme {
        let ok = if image {
            lower.starts_with("http://") || lower.starts_with("https://")
                || ["data:image/png;", "data:image/jpeg;", "data:image/jpg;", "data:image/gif;", "data:image/webp;"].iter().any(|p| lower.starts_with(p))
        } else {
            lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
        };
        if !ok {
            return None;
        }
    }
    Some(u.to_string())
}

const ALLOWED: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "blockquote", "br", "center", "cite", "code", "dd", "del", "details", "dfn", "div", "dl", "dt", "em",
    "figcaption", "figure", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "i", "img", "ins", "kbd", "li", "mark", "ol", "p", "pre", "q", "rp",
    "rt", "ruby", "s", "samp", "section", "small", "span", "strike", "strong", "sub", "summary", "sup", "table", "tbody", "td", "tfoot",
    "th", "thead", "time", "tr", "u", "ul", "var", "wbr",
];
/// Tags whose content must disappear with them.
const SWALLOW: &[&str] = &["script", "style", "textarea", "title", "template", "noscript", "xmp", "iframe", "noembed", "noframes", "object", "svg", "math", "select"];
const VOID: &[&str] = &["br", "hr", "img", "wbr"];

fn attr_allowed(tag: &str, attr: &str) -> bool {
    matches!(attr, "title" | "lang" | "dir")
        || matches!((tag, attr),
            ("a", "href") | ("a", "name") | ("img", "src") | ("img", "alt") | ("img", "width") | ("img", "height")
            | ("td", "colspan") | ("td", "rowspan") | ("td", "align") | ("th", "colspan") | ("th", "rowspan") | ("th", "align")
            | ("ol", "start") | ("ol", "reversed") | ("li", "value") | ("details", "open") | ("time", "datetime")
            | ("p", "align") | ("div", "align") | ("h1", "align") | ("h2", "align") | ("h3", "align") | ("img", "align"))
}

pub struct Tag {
    pub close: bool,
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub len: usize,
}

/// Parse one tag at the start of `s` (which begins with `<`).
pub fn parse_tag(s: &str) -> Option<Tag> {
    let b = s.as_bytes();
    let mut i = 1;
    let close = b.get(1) == Some(&b'/');
    if close {
        i += 1;
    }
    let ns = i;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-') {
        i += 1;
    }
    if i == ns || !b[ns].is_ascii_alphabetic() {
        return None;
    }
    let name = s[ns..i].to_ascii_lowercase();
    let mut attrs = vec![];
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return None;
        }
        match b[i] {
            b'>' => return Some(Tag { close, name, attrs, len: i + 1 }),
            b'/' if b.get(i + 1) == Some(&b'>') => return Some(Tag { close, name, attrs, len: i + 2 }),
            _ => {}
        }
        let an = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'>' | b'/' | b'"' | b'\'') {
            i += 1;
        }
        if i == an {
            return None;
        }
        let aname = s[an..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if b.get(i) == Some(&b'=') {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            match b.get(i) {
                Some(&q) if q == b'"' || q == b'\'' => {
                    let end = s[i + 1..].find(q as char)? + i + 1;
                    value = s[i + 1..end].to_string();
                    i = end + 1;
                }
                Some(_) => {
                    let vs = i;
                    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                        i += 1;
                    }
                    value = s[vs..i].to_string();
                }
                None => return None,
            }
        }
        attrs.push((aname, value));
    }
}

/// Re-emit an allowed tag with only safe attributes. `None` means drop it.
pub fn clean_tag(t: &Tag) -> Option<String> {
    if !ALLOWED.contains(&t.name.as_str()) {
        return None;
    }
    if t.close {
        return (!VOID.contains(&t.name.as_str())).then(|| format!("</{}>", t.name));
    }
    let mut out = format!("<{}", t.name);
    for (k, v) in &t.attrs {
        if !attr_allowed(&t.name, k) {
            continue;
        }
        let v = unescape(v);
        let v = match k.as_str() {
            "href" => match safe_url(&v, false) { Some(u) => u, None => continue },
            "src" => match safe_url(&v, true) { Some(u) => u, None => continue },
            _ => v,
        };
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        esc_into(&mut out, &v);
        out.push('"');
    }
    out.push('>');
    Some(out)
}

pub fn is_swallowed(name: &str) -> bool {
    SWALLOW.contains(&name)
}

/// Sanitize a block of raw HTML: allowed tags are normalized, dangerous ones removed
/// with their contents, everything else is escaped as text.
pub fn sanitize(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        if rest.starts_with("<!--") {
            i += rest.find("-->").map(|e| e + 3).unwrap_or(rest.len());
            continue;
        }
        if rest.starts_with('<') {
            if let Some(t) = parse_tag(rest) {
                i += t.len;
                if !t.close && is_swallowed(&t.name) {
                    let lower = src[i..].to_ascii_lowercase();
                    let end = format!("</{}", t.name);
                    i += lower.find(&end).map(|e| e + src[i + e..].find('>').map(|g| g + 1).unwrap_or(end.len())).unwrap_or(src.len() - i);
                    continue;
                }
                if let Some(clean) = clean_tag(&t) {
                    out.push_str(&clean);
                }
                continue;
            }
        }
        let first = rest.chars().next().map_or(1, char::len_utf8);
        let next = rest[first..].find('<').map(|n| n + first).unwrap_or(rest.len());
        esc_keep_entities(&mut out, &rest[..next]);
        i += next;
    }
    out
}

/// Percent-encode a filesystem path for use in a URL path.
pub fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 8);
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sanitizer() {
        assert_eq!(sanitize("<script>alert(1)</script><b onclick=x>hi</b>"), "<b>hi</b>");
        assert_eq!(sanitize("<a href=\"javascript:alert(1)\">x</a>"), "<a>x</a>");
        assert_eq!(sanitize("<img src=x onerror=alert(1)>"), "<img src=\"x\">");
        assert_eq!(sanitize("<kbd>⌘</kbd> &amp; <unknown>"), "<kbd>⌘</kbd> &amp; ");
        assert_eq!(sanitize("1 < 2"), "1 &lt; 2");
        assert!(safe_url("JaVa\tScript:alert(1)", false).is_none());
        assert_eq!(percent_decode("/a%20b"), "/a b");
    }
}
