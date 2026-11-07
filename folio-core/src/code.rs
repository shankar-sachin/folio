//! Syntax highlighting for fenced code blocks. A small table-driven lexer per language
//! family that emits highlight.js-compatible `hljs-*` classes, so themes stay portable.

use crate::html::esc_into;

struct Lang {
    line: &'static [&'static str],
    block: &'static [(&'static str, &'static str)],
    quotes: &'static str,
    triple: bool,
    keywords: &'static str,
    types: &'static str,
    literals: &'static str,
    builtins: &'static str,
    def_kw: &'static str,
    class_kw: &'static str,
    nocase: bool,
    cap_types: bool,
    /// `@name` decorators/annotations.
    at_meta: bool,
    /// `$name` variables (shell, PHP, Perl-ish).
    dollar_vars: bool,
    /// `#` at line start is a preprocessor line.
    preproc: bool,
    /// `name!` macros (Rust).
    macros: bool,
    /// `#[...]` attributes (Rust).
    rust_attr: bool,
}

const BASE: Lang = Lang {
    line: &[],
    block: &[],
    quotes: "\"'",
    triple: false,
    keywords: "",
    types: "",
    literals: "true false null",
    builtins: "",
    def_kw: "",
    class_kw: "class struct enum interface trait",
    nocase: false,
    cap_types: false,
    at_meta: false,
    dollar_vars: false,
    preproc: false,
    macros: false,
    rust_attr: false,
};

const JS_KW: &str = "break case catch class const continue debugger default delete do else export extends finally for from function if import in instanceof let new of return static super switch this throw try typeof var void while with yield async await get set as";
const JS: Lang = Lang {
    line: &["//"],
    block: &[("/*", "*/")],
    quotes: "\"'`",
    keywords: JS_KW,
    literals: "true false null undefined NaN Infinity",
    builtins: "console window document Math JSON Object Array String Number Boolean Promise Map Set Symbol Date RegExp Error require module exports process globalThis",
    def_kw: "function",
    at_meta: true,
    ..BASE
};
const TS: Lang = Lang {
    keywords: "break case catch class const continue debugger default delete do else export extends finally for from function if import in instanceof let new of return static super switch this throw try typeof var void while with yield async await get set as implements interface type enum namespace declare readonly private public protected abstract keyof infer is satisfies",
    types: "string number boolean any unknown never void object bigint symbol",
    class_kw: "class interface type enum",
    cap_types: true,
    ..JS
};
const PY: Lang = Lang {
    line: &["#"],
    triple: true,
    keywords: "and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield match case",
    literals: "True False None",
    builtins: "print len range open str int float list dict set tuple bool type isinstance super self cls enumerate zip map filter sorted sum min max abs any all repr input iter next object Exception ValueError TypeError KeyError",
    def_kw: "def",
    class_kw: "class",
    at_meta: true,
    ..BASE
};
const SH: Lang = Lang {
    line: &["#"],
    keywords: "if then else elif fi for in do done while until case esac function return local export readonly declare unset shift break continue select time",
    literals: "true false",
    builtins: "echo printf cd ls pwd cat grep sed awk find xargs mkdir rm cp mv chmod chown touch source exit set test read eval exec trap kill sleep curl git npm cargo brew sudo open tar ssh which env",
    def_kw: "function",
    class_kw: "",
    dollar_vars: true,
    ..BASE
};
const RUST: Lang = Lang {
    line: &["//"],
    block: &[("/*", "*/")],
    quotes: "\"",
    keywords: "as async await break const continue crate dyn else enum extern fn for if impl in let loop match mod move mut pub ref return static struct super trait type unsafe use where while Self self",
    types: "i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char str String Vec Option Result Box Rc Arc HashMap HashSet",
    literals: "true false None Some Ok Err",
    def_kw: "fn",
    class_kw: "struct enum trait type impl union",
    cap_types: true,
    macros: true,
    rust_attr: true,
    ..BASE
};
const GO: Lang = Lang {
    line: &["//"],
    block: &[("/*", "*/")],
    quotes: "\"'`",
    keywords: "break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var",
    types: "bool byte complex64 complex128 error float32 float64 int int8 int16 int32 int64 rune string uint uint8 uint16 uint32 uint64 uintptr any",
    literals: "true false nil iota",
    builtins: "append cap close copy delete len make new panic print println recover",
    def_kw: "func",
    class_kw: "type",
    cap_types: true,
    ..BASE
};
const C: Lang = Lang {
    line: &["//"],
    block: &[("/*", "*/")],
    keywords: "auto break case const continue default do else enum extern for goto if inline register restrict return sizeof static struct switch typedef union volatile while class namespace template typename public private protected virtual override final new delete this throw try catch using operator friend explicit constexpr nullptr noexcept static_cast dynamic_cast reinterpret_cast const_cast @interface @implementation @end @property self super",
    types: "void char short int long float double signed unsigned bool size_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t std string vector map auto id BOOL NSString NSInteger",
    literals: "true false NULL nullptr nil YES NO",
    builtins: "printf scanf malloc free memcpy strlen cout cin endl",
    cap_types: false,
    preproc: true,
    ..BASE
};
const CS: Lang = Lang {
    keywords: "abstract as base break case catch class const continue default delegate do else enum event explicit extern finally fixed for foreach goto if implicit in interface internal is lock namespace new operator out override params private protected public readonly ref return sealed sizeof static struct switch this throw try typeof unchecked unsafe using virtual volatile while async await var record get set init",
    types: "bool byte char decimal double float int long object sbyte short string uint ulong ushort void dynamic",
    literals: "true false null",
    cap_types: true,
    preproc: true,
    ..C
};
const JAVA: Lang = Lang {
    line: &["//"],
    block: &[("/*", "*/")],
    keywords: "abstract assert break case catch class continue default do else enum extends final finally for if implements import instanceof interface native new package private protected public return static strictfp super switch synchronized this throw throws transient try volatile while var record yield fun val when object companion data sealed open override lateinit by is in out suspend inline internal typealias",
    types: "boolean byte char double float int long short void String Int Long Double Float Boolean Unit Any List Map",
    literals: "true false null",
    def_kw: "fun",
    cap_types: true,
    at_meta: true,
    ..BASE
};
const SWIFT: Lang = Lang {
    line: &["//"],
    block: &[("/*", "*/")],
    quotes: "\"",
    triple: true,
    keywords: "associatedtype class deinit enum extension fileprivate func import init inout internal let open operator private protocol public rethrows static struct subscript typealias var break case continue default defer do else fallthrough for guard if in repeat return switch where while as catch is super self Self throw throws try async await some any actor weak lazy override mutating final convenience required",
    types: "Int Double Float String Bool Character Array Dictionary Set Optional Void UInt Data URL Date Error",
    literals: "true false nil",
    builtins: "print",
    def_kw: "func",
    class_kw: "class struct enum protocol extension actor",
    cap_types: true,
    at_meta: true,
    ..BASE
};
const RUBY: Lang = Lang {
    line: &["#"],
    keywords: "alias and begin break case class def defined? do else elsif end ensure for if in module next not or redo rescue retry return self super then undef unless until when while yield require attr_accessor attr_reader private public protected",
    literals: "true false nil",
    builtins: "puts print p raise",
    def_kw: "def",
    class_kw: "class module",
    cap_types: true,
    ..BASE
};
const PHP: Lang = Lang {
    line: &["//", "#"],
    block: &[("/*", "*/")],
    keywords: "abstract and array as break callable case catch class clone const continue declare default do echo else elseif empty enddeclare endfor endforeach endif endswitch endwhile extends final finally fn for foreach function global goto if implements include include_once instanceof insteadof interface isset list match namespace new or print private protected public readonly require require_once return static switch throw trait try unset use var while xor yield",
    literals: "true false null TRUE FALSE NULL",
    def_kw: "function",
    dollar_vars: true,
    ..BASE
};
const SQL: Lang = Lang {
    line: &["--"],
    block: &[("/*", "*/")],
    keywords: "select from where and or not insert into values update set delete create table drop alter add column index primary key foreign references join inner left right outer full on as group by order having limit offset union all distinct case when then else end is null in exists between like begin commit rollback transaction view with returning default unique check constraint if replace asc desc",
    types: "int integer bigint smallint varchar char text boolean bool date timestamp datetime float double decimal numeric real serial uuid json jsonb blob",
    literals: "true false null",
    builtins: "count sum avg min max coalesce now lower upper length substr cast",
    class_kw: "",
    nocase: true,
    ..BASE
};
const LUA: Lang = Lang {
    line: &["--"],
    block: &[("--[[", "]]")],
    keywords: "and break do else elseif end for function goto if in local not or repeat return then until while",
    literals: "true false nil",
    builtins: "print pairs ipairs require table string math type tostring tonumber",
    def_kw: "function",
    class_kw: "",
    ..BASE
};
const R: Lang = Lang {
    line: &["#"],
    keywords: "if else repeat while function for in next break return library require",
    literals: "TRUE FALSE NULL NA Inf NaN T F",
    builtins: "c print paste list data.frame vector matrix length sum mean",
    class_kw: "",
    ..BASE
};
const PS: Lang = Lang {
    line: &["#"],
    block: &[("<#", "#>")],
    keywords: "begin break catch class continue data do dynamicparam else elseif end exit filter finally for foreach from function if in param process return switch throw trap try until using var while",
    literals: "$true $false $null",
    def_kw: "function",
    class_kw: "class",
    dollar_vars: true,
    nocase: true,
    ..BASE
};
const DOCKER: Lang = Lang {
    line: &["#"],
    keywords: "FROM RUN CMD LABEL MAINTAINER EXPOSE ENV ADD COPY ENTRYPOINT VOLUME USER WORKDIR ARG ONBUILD STOPSIGNAL HEALTHCHECK SHELL AS",
    class_kw: "",
    dollar_vars: true,
    nocase: true,
    ..BASE
};
const PLAIN: Lang = Lang { quotes: "", literals: "", class_kw: "", ..BASE };

fn word_in(list: &str, w: &str, nocase: bool) -> bool {
    if nocase {
        list.split_ascii_whitespace().any(|k| k.eq_ignore_ascii_case(w))
    } else {
        list.split_ascii_whitespace().any(|k| k == w)
    }
}

fn span(out: &mut String, cls: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    out.push_str("<span class=\"");
    out.push_str(cls);
    out.push_str("\">");
    esc_into(out, text);
    out.push_str("</span>");
}

fn lang_of(name: &str) -> Option<Kind> {
    Some(match name {
        "js" | "jsx" | "mjs" | "cjs" | "javascript" => Kind::Gen(&JS),
        "ts" | "tsx" | "typescript" => Kind::Gen(&TS),
        "py" | "python" | "python3" => Kind::Gen(&PY),
        "sh" | "zsh" | "bash" | "shell" | "console" | "fish" => Kind::Gen(&SH),
        "rs" | "rust" => Kind::Gen(&RUST),
        "go" | "golang" => Kind::Gen(&GO),
        "c" | "h" | "cpp" | "c++" | "cc" | "hpp" | "objc" | "objectivec" | "objective-c" | "m" | "mm" => Kind::Gen(&C),
        "cs" | "csharp" | "c#" => Kind::Gen(&CS),
        "java" | "kotlin" | "kt" | "kts" | "scala" | "groovy" | "dart" => Kind::Gen(&JAVA),
        "swift" => Kind::Gen(&SWIFT),
        "rb" | "ruby" => Kind::Gen(&RUBY),
        "php" => Kind::Gen(&PHP),
        "sql" | "mysql" | "postgres" | "postgresql" | "sqlite" => Kind::Gen(&SQL),
        "lua" => Kind::Gen(&LUA),
        "r" => Kind::Gen(&R),
        "ps" | "ps1" | "powershell" | "pwsh" => Kind::Gen(&PS),
        "dockerfile" | "docker" => Kind::Gen(&DOCKER),
        "json" | "jsonc" | "json5" => Kind::Json,
        "html" | "xml" | "svg" | "plist" | "xhtml" | "vue" => Kind::Xml,
        "css" | "scss" | "sass" | "less" => Kind::Css,
        "yaml" | "yml" => Kind::Yaml,
        "toml" | "ini" | "cfg" | "conf" | "properties" | "env" | "dotenv" => Kind::Ini,
        "diff" | "patch" => Kind::Diff,
        "md" | "markdown" | "mdx" => Kind::Markdown,
        "make" | "makefile" | "mk" => Kind::Make,
        "tex" | "latex" => Kind::Latex,
        "text" | "txt" | "plaintext" | "plain" => Kind::Gen(&PLAIN),
        _ => return None,
    })
}

enum Kind {
    Gen(&'static Lang),
    Json,
    Xml,
    Css,
    Yaml,
    Ini,
    Diff,
    Markdown,
    Make,
    Latex,
}

/// Highlight `code` written in `lang` (already lower-cased). Unknown languages are escaped only.
pub fn highlight(lang: &str, code: &str) -> String {
    let mut out = String::with_capacity(code.len() * 2);
    match lang_of(lang) {
        None => esc_into(&mut out, code),
        Some(Kind::Gen(l)) => generic(l, code, &mut out),
        Some(Kind::Json) => json(code, &mut out),
        Some(Kind::Xml) => xml(code, &mut out),
        Some(Kind::Css) => css(code, &mut out),
        Some(Kind::Yaml) => per_line(code, &mut out, yaml_line),
        Some(Kind::Ini) => per_line(code, &mut out, ini_line),
        Some(Kind::Diff) => per_line(code, &mut out, diff_line),
        Some(Kind::Markdown) => per_line(code, &mut out, md_line),
        Some(Kind::Make) => per_line(code, &mut out, make_line),
        Some(Kind::Latex) => latex(code, &mut out),
    }
    out
}

pub fn is_known(lang: &str) -> bool {
    lang_of(lang).is_some()
}

fn ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Scan a quoted string starting at `i` (which holds the quote). Returns the end index.
fn scan_string(s: &str, i: usize, q: char, triple: bool) -> usize {
    let b = s.as_bytes();
    let qs = q.to_string().repeat(3);
    if triple && s[i..].starts_with(&qs) {
        return s[i + 3..].find(&qs).map(|e| i + 3 + e + 3).unwrap_or(s.len());
    }
    let mut j = i + q.len_utf8();
    while j < b.len() {
        let c = s[j..].chars().next().unwrap();
        if c == '\\' {
            j += 1 + s[j + 1..].chars().next().map_or(0, char::len_utf8);
            continue;
        }
        if c == q {
            return j + c.len_utf8();
        }
        if c == '\n' && q != '`' {
            return j;
        }
        j += c.len_utf8();
    }
    s.len()
}

fn scan_number(s: &str, i: usize) -> usize {
    let b = s.as_bytes();
    let mut j = i;
    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.' || ((b[j] == b'+' || b[j] == b'-') && j > i && matches!(b[j - 1], b'e' | b'E') && !s[i..].starts_with("0x"))) {
        if b[j] == b'.' && !b.get(j + 1).is_some_and(u8::is_ascii_digit) {
            break;
        }
        j += 1;
    }
    j
}

fn generic(l: &Lang, s: &str, out: &mut String) {
    let b = s.as_bytes();
    let mut i = 0;
    let mut pending: Option<&str> = None; // class for the next identifier
    let mut line_start = true;
    while i < s.len() {
        let rest = &s[i..];
        let c = rest.chars().next().unwrap();
        if c == '\n' {
            out.push('\n');
            i += 1;
            line_start = true;
            continue;
        }
        if line_start && l.preproc && rest.trim_start_matches([' ', '\t']).starts_with('#') {
            let e = rest.find('\n').unwrap_or(rest.len());
            span(out, "hljs-meta", &rest[..e]);
            i += e;
            continue;
        }
        if !c.is_whitespace() {
            line_start = false;
        }
        if let Some(&(open, close)) = l.block.iter().find(|(o, _)| rest.starts_with(o)) {
            let e = rest[open.len()..].find(close).map(|e| e + open.len() + close.len()).unwrap_or(rest.len());
            span(out, "hljs-comment", &rest[..e]);
            i += e;
            continue;
        }
        if l.line.iter().any(|p| rest.starts_with(p)) && (c != '#' || !l.dollar_vars || i == 0 || !ident_char(s[..i].chars().next_back().unwrap())) {
            let e = rest.find('\n').unwrap_or(rest.len());
            span(out, "hljs-comment", &rest[..e]);
            i += e;
            continue;
        }
        if l.rust_attr && rest.starts_with("#[") || l.rust_attr && rest.starts_with("#![") {
            let e = rest.find(']').map(|e| e + 1).unwrap_or(rest.len());
            span(out, "hljs-meta", &rest[..e]);
            i += e;
            continue;
        }
        if l.quotes.contains(c) && !(c == '\'' && l.macros && is_lifetime(rest)) {
            let e = scan_string(s, i, c, l.triple);
            span(out, "hljs-string", &s[i..e]);
            i = e;
            continue;
        }
        if l.macros && c == '\'' {
            // Rust char literal or lifetime.
            if let Some(e) = rest[1..].find('\'').filter(|&e| e <= 4 && !rest[1..1 + e].contains(' ')) {
                span(out, "hljs-string", &rest[..e + 2]);
                i += e + 2;
            } else {
                let e = 1 + rest[1..].find(|ch: char| !ident_char(ch)).unwrap_or(rest.len() - 1);
                span(out, "hljs-symbol", &rest[..e]);
                i += e;
            }
            continue;
        }
        if c.is_ascii_digit() && (i == 0 || !ident_char(s[..i].chars().next_back().unwrap())) {
            let e = scan_number(s, i);
            span(out, "hljs-number", &s[i..e]);
            i = e;
            continue;
        }
        if l.dollar_vars && c == '$' {
            let r = &rest[1..];
            let e = if r.starts_with('{') {
                r.find('}').map(|e| e + 2).unwrap_or(1)
            } else if r.starts_with(|d: char| d.is_ascii_digit() || "@#?*!$".contains(d)) {
                2
            } else {
                1 + r.find(|ch: char| !ident_char(ch)).unwrap_or(r.len())
            };
            if e > 1 {
                let w = &rest[..e];
                span(out, if word_in(l.literals, w, true) { "hljs-literal" } else { "hljs-variable" }, w);
                i += e;
                continue;
            }
        }
        if l.at_meta && c == '@' && rest[1..].starts_with(|ch: char| ch.is_alphabetic()) {
            let e = 1 + rest[1..].find(|ch: char| !(ident_char(ch) || ch == '.')).unwrap_or(rest.len() - 1);
            span(out, "hljs-meta", &rest[..e]);
            i += e;
            continue;
        }
        if c.is_alphabetic() || c == '_' || (c == '$' && !l.dollar_vars) || (c == '@' && !l.at_meta) {
            let mut e = c.len_utf8() + rest[c.len_utf8()..].find(|ch: char| !(ident_char(ch) || ch == '$' || (l.nocase && ch == '?'))).unwrap_or(rest.len() - c.len_utf8());
            if l.keywords.contains("defined?") && rest[..e].ends_with('?') && rest[..e] != *"defined?" {
                e -= 1;
            }
            let w = &rest[..e];
            let after = &rest[e..];
            let cls = if let Some(p) = pending.take() {
                Some(p)
            } else if word_in(l.keywords, w, l.nocase) {
                Some("hljs-keyword")
            } else if word_in(l.literals, w, l.nocase) {
                Some("hljs-literal")
            } else if word_in(l.types, w, l.nocase) {
                Some("hljs-type")
            } else if l.macros && after.starts_with('!') && !after.starts_with("!=") {
                span(out, "hljs-built_in", &rest[..e + 1]);
                i += e + 1;
                continue;
            } else if word_in(l.builtins, w, l.nocase) {
                Some("hljs-built_in")
            } else if after.starts_with('(') {
                Some("hljs-title function_")
            } else if l.cap_types && w.chars().next().unwrap().is_uppercase() && w.chars().any(|x| x.is_lowercase()) {
                Some("hljs-type")
            } else {
                None
            };
            if word_in(l.def_kw, w, l.nocase) {
                pending = Some("hljs-title function_");
            } else if word_in(l.class_kw, w, l.nocase) {
                pending = Some("hljs-title class_");
            }
            match cls {
                Some(k) => span(out, k, w),
                None => esc_into(out, w),
            }
            i += e;
            continue;
        }
        if !c.is_whitespace() && pending.is_some() && !matches!(c, '*' | '&') {
            pending = None;
        }
        esc_into(out, &rest[..c.len_utf8()]);
        i += c.len_utf8();
    }
    let _ = b;
}

fn is_lifetime(rest: &str) -> bool {
    // 'a  (not followed by a closing quote within a char) → lifetime
    let r = &rest[1..];
    let ident = r.chars().take_while(|c| ident_char(*c)).count();
    ident > 0 && !r[ident..].starts_with('\'') && !r.starts_with('\\')
}

fn json(s: &str, out: &mut String) {
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let c = rest.chars().next().unwrap();
        if c == '"' {
            let e = scan_string(s, i, '"', false);
            let is_key = s[e..].trim_start_matches([' ', '\t']).starts_with(':');
            span(out, if is_key { "hljs-attr" } else { "hljs-string" }, &s[i..e]);
            i = e;
        } else if c.is_ascii_digit() || (c == '-' && rest[1..].starts_with(|d: char| d.is_ascii_digit())) {
            let e = i + 1 + scan_number(s, i + 1) - (i + 1);
            span(out, "hljs-number", &s[i..e]);
            i = e;
        } else if let Some(w) = ["true", "false", "null"].iter().find(|w| rest.starts_with(**w)) {
            span(out, "hljs-literal", w);
            i += w.len();
        } else if rest.starts_with("//") {
            let e = rest.find('\n').unwrap_or(rest.len());
            span(out, "hljs-comment", &rest[..e]);
            i += e;
        } else {
            esc_into(out, &rest[..c.len_utf8()]);
            i += c.len_utf8();
        }
    }
}

fn xml(s: &str, out: &mut String) {
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with("<!--") {
            let e = rest.find("-->").map(|e| e + 3).unwrap_or(rest.len());
            span(out, "hljs-comment", &rest[..e]);
            i += e;
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            let e = rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
            span(out, "hljs-meta", &rest[..e]);
            i += e;
        } else if rest.starts_with('<') && rest[1..].starts_with(|c: char| c.is_alphabetic() || c == '/') {
            let e = rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
            let tag = &rest[..e];
            out.push_str("<span class=\"hljs-tag\">");
            let close = tag.starts_with("</");
            let start = if close { 2 } else { 1 };
            esc_into(out, &tag[..start]);
            let name_end = start + tag[start..].find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(tag.len() - start);
            span(out, "hljs-name", &tag[start..name_end]);
            let mut j = name_end;
            while j < tag.len() {
                let r = &tag[j..];
                let ch = r.chars().next().unwrap();
                if ch == '"' || ch == '\'' {
                    let e2 = r[1..].find(ch).map(|x| x + 2).unwrap_or(r.len());
                    span(out, "hljs-string", &r[..e2]);
                    j += e2;
                } else if ch.is_alphabetic() || ch == '_' || ch == ':' || ch == '@' {
                    let e2 = r.find(|x: char| x.is_whitespace() || x == '=' || x == '>' || x == '/').unwrap_or(r.len());
                    span(out, "hljs-attr", &r[..e2]);
                    j += e2;
                } else {
                    esc_into(out, &r[..ch.len_utf8()]);
                    j += ch.len_utf8();
                }
            }
            out.push_str("</span>");
            i += e;
        } else {
            let c = rest.chars().next().unwrap();
            esc_into(out, &rest[..c.len_utf8()]);
            i += c.len_utf8();
        }
    }
}

fn css(s: &str, out: &mut String) {
    let mut i = 0;
    let mut depth = 0usize;
    let mut in_value = false;
    while i < s.len() {
        let rest = &s[i..];
        let c = rest.chars().next().unwrap();
        if rest.starts_with("/*") {
            let e = rest.find("*/").map(|e| e + 2).unwrap_or(rest.len());
            span(out, "hljs-comment", &rest[..e]);
            i += e;
            continue;
        }
        if rest.starts_with("//") {
            let e = rest.find('\n').unwrap_or(rest.len());
            span(out, "hljs-comment", &rest[..e]);
            i += e;
            continue;
        }
        match c {
            '"' | '\'' => {
                let e = scan_string(s, i, c, false);
                span(out, "hljs-string", &s[i..e]);
                i = e;
                continue;
            }
            '{' => {
                depth += 1;
                in_value = false;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                in_value = false;
            }
            ';' => in_value = false,
            '@' => {
                let e = 1 + rest[1..].find(|x: char| !(x.is_alphanumeric() || x == '-')).unwrap_or(rest.len() - 1);
                span(out, "hljs-keyword", &rest[..e]);
                i += e;
                continue;
            }
            '$' => {
                let e = 1 + rest[1..].find(|x: char| !(x.is_alphanumeric() || x == '-' || x == '_')).unwrap_or(rest.len() - 1);
                span(out, "hljs-variable", &rest[..e]);
                i += e;
                continue;
            }
            _ => {}
        }
        if in_value {
            if c == '#' && rest[1..].starts_with(|x: char| x.is_ascii_hexdigit()) {
                let e = 1 + rest[1..].find(|x: char| !x.is_ascii_alphanumeric()).unwrap_or(rest.len() - 1);
                span(out, "hljs-number", &rest[..e]);
                i += e;
                continue;
            }
            if c.is_ascii_digit() || (c == '.' && rest[1..].starts_with(|x: char| x.is_ascii_digit())) || (c == '-' && rest[1..].starts_with(|x: char| x.is_ascii_digit())) {
                let e = 1 + rest[1..].find(|x: char| !(x.is_ascii_alphanumeric() || x == '.' || x == '%')).unwrap_or(rest.len() - 1);
                span(out, "hljs-number", &rest[..e]);
                i += e;
                continue;
            }
            if c == '!' && rest.starts_with("!important") {
                span(out, "hljs-keyword", "!important");
                i += 10;
                continue;
            }
        } else if c.is_alphabetic() || c == '-' || c == '.' || c == '#' || c == '_' {
            let start = if c == '.' || c == '#' { 1 } else { 0 };
            let e = start + rest[start..].find(|x: char| !(x.is_alphanumeric() || x == '-' || x == '_')).unwrap_or(rest.len() - start);
            let after = rest[e..].trim_start_matches([' ', '\t']);
            let word = &rest[..e];
            let cls = if depth > 0 && after.starts_with(':') && !after.starts_with("::") && start == 0 {
                in_value = true;
                "hljs-attribute"
            } else if c == '.' {
                "hljs-selector-class"
            } else if c == '#' {
                "hljs-selector-id"
            } else {
                "hljs-selector-tag"
            };
            if e > start {
                span(out, cls, word);
                i += e;
                continue;
            }
        }
        esc_into(out, &rest[..c.len_utf8()]);
        i += c.len_utf8();
    }
}

fn per_line(s: &str, out: &mut String, f: fn(&str, &mut String)) {
    for (k, line) in s.split('\n').enumerate() {
        if k > 0 {
            out.push('\n');
        }
        f(line, out);
    }
}

fn value(v: &str, out: &mut String) {
    let t = v.trim();
    let lead = &v[..v.len() - v.trim_start().len()];
    let trail = &v[lead.len() + t.len()..];
    out.push_str(lead);
    if t.is_empty() {
    } else if t.starts_with(['"', '\'']) {
        span(out, "hljs-string", t);
    } else if t.parse::<f64>().is_ok() {
        span(out, "hljs-number", t);
    } else if matches!(t, "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~") {
        span(out, "hljs-literal", t);
    } else {
        span(out, "hljs-string", t);
    }
    out.push_str(trail);
}

fn split_comment(line: &str, marks: &[char]) -> (usize, Option<usize>) {
    let mut q: Option<char> = None;
    for (i, c) in line.char_indices() {
        match q {
            Some(x) if c == x => q = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => q = Some(c),
            None if marks.contains(&c) && (i == 0 || line[..i].ends_with([' ', '\t'])) => return (i, Some(i)),
            None => {}
        }
    }
    (line.len(), None)
}

fn yaml_line(line: &str, out: &mut String) {
    let (end, comment) = split_comment(line, &['#']);
    let body = &line[..end];
    if body.trim() == "---" || body.trim() == "..." {
        span(out, "hljs-meta", body);
    } else {
        let indent = body.len() - body.trim_start().len();
        out.push_str(&body[..indent]);
        let mut rest = &body[indent..];
        if let Some(r) = rest.strip_prefix("- ") {
            span(out, "hljs-bullet", "-");
            out.push(' ');
            rest = r;
        } else if rest == "-" {
            span(out, "hljs-bullet", "-");
            rest = "";
        }
        match rest.find(": ").or_else(|| rest.strip_suffix(':').map(|r| r.len())) {
            Some(k) if !rest.starts_with(['"', '\'', '{', '[']) || rest[..k].ends_with(['"', '\'']) => {
                span(out, "hljs-attr", &rest[..k]);
                out.push(':');
                value(rest.get(k + 1..).unwrap_or(""), out);
            }
            _ => value(rest, out),
        }
    }
    if let Some(c) = comment {
        span(out, "hljs-comment", &line[c..]);
    }
}

fn ini_line(line: &str, out: &mut String) {
    let t = line.trim_start();
    if t.starts_with('#') || t.starts_with(';') {
        span(out, "hljs-comment", line);
        return;
    }
    if t.starts_with('[') {
        span(out, "hljs-section", line);
        return;
    }
    let (end, comment) = split_comment(line, &['#']);
    let body = &line[..end];
    match body.find(['=', ':']) {
        Some(k) => {
            let key = &body[..k];
            let lead = key.len() - key.trim_start().len();
            out.push_str(&key[..lead]);
            span(out, "hljs-attr", key[lead..].trim_end());
            out.push_str(&key[lead + key[lead..].trim_end().len()..]);
            esc_into(out, &body[k..k + 1]);
            value(&body[k + 1..], out);
        }
        None => esc_into(out, body),
    }
    if let Some(c) = comment {
        span(out, "hljs-comment", &line[c..]);
    }
}

fn diff_line(line: &str, out: &mut String) {
    let cls = if line.starts_with("+++") || line.starts_with("---") || line.starts_with("diff ") || line.starts_with("index ") {
        "hljs-meta"
    } else if line.starts_with("@@") {
        "hljs-meta"
    } else if line.starts_with('+') || line.starts_with('>') {
        "hljs-addition"
    } else if line.starts_with('-') || line.starts_with('<') {
        "hljs-deletion"
    } else {
        ""
    };
    if cls.is_empty() {
        esc_into(out, line);
    } else {
        span(out, cls, line);
    }
}

fn md_line(line: &str, out: &mut String) {
    let t = line.trim_start();
    if t.starts_with('#') {
        span(out, "hljs-section", line);
    } else if t.starts_with('>') {
        span(out, "hljs-quote", line);
    } else if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        out.push_str(&line[..line.len() - t.len()]);
        span(out, "hljs-bullet", &t[..1]);
        out.push(' ');
        esc_into(out, r);
    } else if t.starts_with("```") || t.starts_with("~~~") {
        span(out, "hljs-meta", line);
    } else {
        esc_into(out, line);
    }
}

fn make_line(line: &str, out: &mut String) {
    let (end, comment) = split_comment(line, &['#']);
    let body = &line[..end];
    if !body.starts_with(['\t', ' ']) && body.contains(':') && !body.contains(":=") && !body.contains('=') {
        let k = body.find(':').unwrap();
        span(out, "hljs-title", &body[..k]);
        esc_into(out, &body[k..]);
    } else if let Some(k) = body.find([':', '?', '+']).filter(|&k| body[k..].starts_with(":=") || body[k..].starts_with("?=") || body[k..].starts_with("+=")).or_else(|| body.find('=')).filter(|_| !body.starts_with('\t')) {
        span(out, "hljs-variable", &body[..k]);
        esc_into(out, &body[k..]);
    } else {
        let mut i = 0;
        while i < body.len() {
            let r = &body[i..];
            if r.starts_with("$(") || r.starts_with("${") {
                let e = r.find([')', '}']).map(|e| e + 1).unwrap_or(r.len());
                span(out, "hljs-variable", &r[..e]);
                i += e;
            } else {
                let c = r.chars().next().unwrap();
                esc_into(out, &r[..c.len_utf8()]);
                i += c.len_utf8();
            }
        }
    }
    if let Some(c) = comment {
        span(out, "hljs-comment", &line[c..]);
    }
}

fn latex(s: &str, out: &mut String) {
    let mut i = 0;
    while i < s.len() {
        let r = &s[i..];
        let c = r.chars().next().unwrap();
        if c == '%' {
            let e = r.find('\n').unwrap_or(r.len());
            span(out, "hljs-comment", &r[..e]);
            i += e;
        } else if c == '\\' {
            let e = 1 + r[1..].find(|x: char| !x.is_alphabetic()).unwrap_or(r.len() - 1).max(1).min(r.len() - 1);
            span(out, "hljs-keyword", &r[..e]);
            i += e;
        } else if c == '$' {
            let e = r[1..].find('$').map(|e| e + 2).unwrap_or(1);
            span(out, "hljs-string", &r[..e]);
            i += e;
        } else {
            esc_into(out, &r[..c.len_utf8()]);
            i += c.len_utf8();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn python() {
        let h = highlight("python", "def f(x):\n    # hi\n    return \"s\" + 42");
        assert!(h.contains("<span class=\"hljs-keyword\">def</span> <span class=\"hljs-title function_\">f</span>"), "{h}");
        assert!(h.contains("<span class=\"hljs-comment\"># hi</span>"));
        assert!(h.contains("<span class=\"hljs-string\">&quot;s&quot;</span>"));
        assert!(h.contains("<span class=\"hljs-number\">42</span>"));
    }
    #[test]
    fn others() {
        assert!(highlight("rust", "fn main() { println!(\"{}\", 'a'); let x: &'a str; }").contains("hljs-built_in\">println!"));
        assert!(highlight("json", "{\"a\": 1, \"b\": true}").contains("<span class=\"hljs-attr\">&quot;a&quot;</span>"));
        assert!(highlight("html", "<a href=\"x\">y</a>").contains("<span class=\"hljs-name\">a</span>"));
        assert!(highlight("bash", "echo $HOME # c").contains("hljs-variable\">$HOME"));
        assert!(highlight("css", ".a { color: #fff; }").contains("hljs-attribute\">color"));
        assert!(highlight("yaml", "key: value # c").contains("hljs-attr\">key"));
        assert!(highlight("diff", "+a\n-b").contains("hljs-addition"));
        assert_eq!(highlight("nope", "<x>"), "&lt;x&gt;");
    }
}
