//! Code illumination — treat a source file like a glossed manuscript.
//!
//! Where prose is wrapped and decorated, code keeps its lines and indentation
//! verbatim. Two manuscript practices map naturally onto it: *rubrication* (the
//! language's keywords set in red ink) and *glossing* (comments lifted out of
//! the code and into the margin, as a scribe set commentary beside scripture).

use unicode_width::UnicodeWidthChar;

use crate::illuminate::{display_width, Line};
use crate::style::Style;

/// The marginal rule and its visible width (a glyph flanked by two spaces).
const SEP_W: usize = 3;
/// The widest a gloss margin grows before comments are truncated.
const GLOSS_MAX: usize = 32;

/// A language's lexical surface: the keywords we rubricate and how its comments
/// are written, so they can be lifted into the margin.
pub(crate) struct Language {
    pub(crate) name: &'static str,
    extensions: &'static [&'static str],
    keywords: &'static [&'static str],
    line_comments: &'static [&'static str],
    block_comment: Option<(&'static str, &'static str)>,
}

#[rustfmt::skip]
static LANGUAGES: &[Language] = &[
    Language {
        name: "rust",
        extensions: &["rs"],
        keywords: &[
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
            "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "type",
            "unsafe", "use", "where", "while",
        ],
        line_comments: &["//"],
        block_comment: Some(("/*", "*/")),
    },
    Language {
        name: "python",
        extensions: &["py", "pyi"],
        keywords: &[
            "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
            "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is",
            "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
            "yield", "True", "False", "None",
        ],
        line_comments: &["#"],
        block_comment: None,
    },
    Language {
        name: "javascript",
        extensions: &["js", "jsx", "ts", "tsx", "mjs", "cjs"],
        keywords: &[
            "async", "await", "break", "case", "catch", "class", "const", "continue", "default",
            "delete", "do", "else", "export", "extends", "finally", "for", "function", "if", "import",
            "in", "instanceof", "interface", "let", "new", "of", "return", "static", "super", "switch",
            "this", "throw", "try", "type", "typeof", "var", "void", "while", "yield",
        ],
        line_comments: &["//"],
        block_comment: Some(("/*", "*/")),
    },
    Language {
        name: "c",
        extensions: &["c", "h", "cpp", "hpp", "cc", "cxx", "hxx"],
        keywords: &[
            "auto", "break", "case", "char", "class", "const", "continue", "default", "do", "double",
            "else", "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long",
            "namespace", "return", "short", "signed", "sizeof", "static", "struct", "switch",
            "template", "typedef", "union", "unsigned", "void", "volatile", "while",
        ],
        line_comments: &["//"],
        block_comment: Some(("/*", "*/")),
    },
    Language {
        name: "go",
        extensions: &["go"],
        keywords: &[
            "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough",
            "for", "func", "go", "goto", "if", "import", "interface", "map", "package", "range",
            "return", "select", "struct", "switch", "type", "var",
        ],
        line_comments: &["//"],
        block_comment: Some(("/*", "*/")),
    },
    Language {
        name: "shell",
        extensions: &["sh", "bash", "zsh"],
        keywords: &[
            "case", "do", "done", "elif", "else", "esac", "fi", "for", "function", "if", "in",
            "return", "select", "then", "until", "while",
        ],
        line_comments: &["#"],
        block_comment: None,
    },
    // A catch-all for `--code` on an unknown extension: the comment styles and
    // keywords common across C-family and scripting languages.
    Language {
        name: "generic",
        extensions: &[],
        keywords: &[
            "class", "const", "def", "else", "for", "function", "if", "import", "include", "let",
            "private", "public", "return", "static", "struct", "var", "void", "while",
        ],
        line_comments: &["//", "#"],
        block_comment: Some(("/*", "*/")),
    },
];

/// Look up a language by source-file extension (e.g. `"rs"`).
pub(crate) fn by_extension(ext: &str) -> Option<&'static Language> {
    let ext = ext.to_ascii_lowercase();
    LANGUAGES
        .iter()
        .find(|l| l.extensions.contains(&ext.as_str()))
}

/// Look up a language by name (e.g. `"rust"`).
pub(crate) fn by_name(name: &str) -> Option<&'static Language> {
    let name = name.to_ascii_lowercase();
    LANGUAGES.iter().find(|l| l.name == name)
}

/// The catch-all language, for `--code` without a recognised extension.
pub(crate) fn generic() -> &'static Language {
    by_name("generic").expect("generic language is always present")
}

/// Rubricate the keywords in a line of code, preserving every other character
/// (operators, punctuation, indentation) exactly.
fn rubricate(code: &str, lang: &Language, style: &Style) -> String {
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if lang.keywords.contains(&word.as_str()) {
                out.push_str(&style.rubric(&word));
            } else {
                out.push_str(&word);
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Splits source lines into their code and comment (gloss) parts, carrying
/// block-comment state across lines.
#[derive(Default)]
struct Glosser {
    in_block: bool,
}

impl Glosser {
    /// Return `(code, gloss)` for one line. `gloss` is `Some` when the line bore
    /// a comment (possibly empty); the comment text is stripped of its markers.
    fn split(&mut self, line: &str, lang: &Language) -> (String, Option<String>) {
        if self.in_block {
            let (_, end) = lang.block_comment.expect("in_block implies a block style");
            return match line.find(end) {
                Some(p) => {
                    self.in_block = false;
                    let gloss = line[..p].trim().to_string();
                    (line[p + end.len()..].to_string(), Some(gloss))
                }
                None => (String::new(), Some(line.trim().to_string())),
            };
        }

        // The earliest comment opener on the line wins.
        let mut best: Option<(usize, usize, bool)> = None; // (pos, marker_len, is_block)
        for marker in lang.line_comments {
            if let Some(p) = line.find(marker) {
                if best.is_none_or(|(bp, _, _)| p < bp) {
                    best = Some((p, marker.len(), false));
                }
            }
        }
        if let Some((open, _)) = lang.block_comment {
            if let Some(p) = line.find(open) {
                if best.is_none_or(|(bp, _, _)| p < bp) {
                    best = Some((p, open.len(), true));
                }
            }
        }

        match best {
            None => (line.to_string(), None),
            Some((p, mlen, false)) => {
                let gloss = line[p + mlen..].trim().to_string();
                (line[..p].to_string(), Some(gloss))
            }
            Some((p, mlen, true)) => {
                let before = &line[..p];
                let rest = &line[p + mlen..];
                let (_, end) = lang.block_comment.expect("is_block implies a block style");
                match rest.find(end) {
                    Some(ep) => {
                        let gloss = rest[..ep].trim().to_string();
                        (format!("{before}{}", &rest[ep + end.len()..]), Some(gloss))
                    }
                    None => {
                        self.in_block = true;
                        (before.to_string(), Some(rest.trim().to_string()))
                    }
                }
            }
        }
    }
}

/// Truncate a plain string to `width` display columns, marking any cut with `…`.
fn truncate(s: &str, width: usize) -> String {
    if display_width(s) <= width {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(0);
        if w + cw > width.saturating_sub(1) {
            break;
        }
        out.push(c);
        w += cw;
    }
    out.push('…');
    out
}

/// Illuminate `source` as code: keywords rubricated, comments set as glosses in
/// the right margin. Returns the body lines and their shared width.
pub(crate) fn illuminate(
    source: &str,
    lang: &Language,
    style: &Style,
    width: usize,
) -> (Vec<Line>, usize) {
    let source = source.replace("\r\n", "\n").replace('\r', "\n");

    let mut glosser = Glosser::default();
    let rows: Vec<(String, String)> = source
        .lines()
        .map(|line| {
            let (code, gloss) = glosser.split(line, lang);
            (code.trim_end().to_string(), gloss.unwrap_or_default())
        })
        .collect();

    let any_gloss = rows.iter().any(|(_, g)| !g.trim().is_empty());
    let code_w = rows
        .iter()
        .map(|(c, _)| display_width(c))
        .max()
        .unwrap_or(0)
        .clamp(1, width);
    let gloss_w = if any_gloss {
        rows.iter()
            .map(|(_, g)| display_width(g))
            .max()
            .unwrap_or(0)
            .clamp(1, GLOSS_MAX)
    } else {
        0
    };

    let sep = format!(" {} ", style.border("┊"));
    let body = rows
        .into_iter()
        .map(|(code, gloss)| {
            let code_plain = truncate(&code, code_w);
            let cw = display_width(&code_plain);
            let code_shown = rubricate(&code_plain, lang, style);
            if any_gloss {
                let gloss_plain = truncate(&gloss, gloss_w);
                let gw = display_width(&gloss_plain);
                let shown = format!(
                    "{code_shown}{}{sep}{}{}",
                    " ".repeat(code_w - cw),
                    style.gloss(&gloss_plain),
                    " ".repeat(gloss_w - gw),
                );
                Line {
                    shown,
                    len: code_w + SEP_W + gloss_w,
                }
            } else {
                Line {
                    shown: format!("{code_shown}{}", " ".repeat(code_w - cw)),
                    len: code_w,
                }
            }
        })
        .collect();

    let body_w = if any_gloss {
        code_w + SEP_W + gloss_w
    } else {
        code_w
    };
    (body, body_w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Theme;

    #[test]
    fn detection_by_extension_and_name() {
        assert_eq!(by_extension("rs").unwrap().name, "rust");
        assert_eq!(by_extension("PY").unwrap().name, "python");
        assert!(by_extension("zzz").is_none());
        assert_eq!(by_name("go").unwrap().name, "go");
    }

    #[test]
    fn line_comment_becomes_a_gloss() {
        let mut g = Glosser::default();
        let (code, gloss) = g.split("let x = 5; // a number", by_name("rust").unwrap());
        assert_eq!(code, "let x = 5; ");
        assert_eq!(gloss.as_deref(), Some("a number"));
    }

    #[test]
    fn block_comment_spans_lines() {
        let rust = by_name("rust").unwrap();
        let mut g = Glosser::default();
        let (c0, g0) = g.split("code /* open", rust);
        assert_eq!(c0, "code ");
        assert_eq!(g0.as_deref(), Some("open"));
        assert!(g.in_block);
        let (c1, g1) = g.split("still comment", rust);
        assert_eq!(c1, "");
        assert_eq!(g1.as_deref(), Some("still comment"));
        let (c2, g2) = g.split("close */ more", rust);
        assert_eq!(c2, " more");
        assert_eq!(g2.as_deref(), Some("close"));
        assert!(!g.in_block);
    }

    #[test]
    fn keywords_are_rubricated_in_red() {
        let style = Style::new(true, Theme::Gold);
        let out = rubricate("fn main() {", by_name("rust").unwrap(), &style);
        assert!(out.contains("\u{1b}[1;31mfn\u{1b}[0m"));
        assert!(out.contains("main")); // identifiers are left untouched
    }

    #[test]
    fn body_lines_share_one_width() {
        let style = Style::new(false, Theme::Mono);
        let src = "fn main() {\n    let x = 5; // a number\n}\n";
        let (body, w) = illuminate(src, by_name("rust").unwrap(), &style, 80);
        assert_eq!(body.len(), 3);
        for line in &body {
            assert_eq!(line.len, w);
        }
    }
}
