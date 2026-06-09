//! lindisfarner — a digital scriptorium.
//!
//! Turn plain text **or source code** into an "illuminated" page: a large
//! ASCII-art initial opens the text, chosen words are rubricated, marginal
//! drolleries and pilcrows adorn it, and the whole is set inside a decorative
//! border.
//!
//! # Illuminating prose
//!
//! [`render`] takes the source and a [`Config`] and returns the finished page as
//! a `String`:
//!
//! ```
//! use lindisfarner::{render, Config};
//!
//! let page = render("Hail and well met, traveller.", &Config::default());
//! assert!(page.contains('❦')); // the ornate frame's flourish
//! ```
//!
//! Set [`Config::corrupt`] to let a careless scribe introduce transcription
//! errors, varied deterministically by [`Config::seed`].
//!
//! # Illuminating code
//!
//! Set [`Config::code`] to treat the input as source rather than prose: lines are
//! kept verbatim, the language's keywords are rubricated, and comments are lifted
//! into the margin as glosses. Name the language with [`Config::language`], or
//! derive it from a path with [`detect_language`].
//!
//! ```
//! use lindisfarner::{render, Config};
//!
//! let cfg = Config { code: true, language: Some("rust".into()), ..Config::default() };
//! let page = render("fn main() {} // the entry point\n", &cfg);
//! assert!(page.contains("the entry point")); // the comment becomes a gloss
//! ```
//!
//! [`render_glossed`] lays out explicit `(code, gloss)` rows — each line of code
//! beside a note in the margin — for building commentary pages directly.
//!
//! # Searching code, and the magnifica modes
//!
//! With the `cli` feature (on by default), two further modules back the
//! command-line tool:
//!
//! - `search` — run a [Semgrep](https://semgrep.dev) pattern or rule set over a
//!   path and gloss the matches (the `--find` and `--scan` modes).
//! - `magnifica` — an art project that finds where a codebase uses AI and writes
//!   the words of the encyclical *Magnifica Humanitas* into those files,
//!   annotating or breaking them (the `--magnifica` modes).
//!
//! Both pull in extra dependencies; drop them with `default-features = false`.

#![warn(missing_docs)]

mod border;
mod code;
mod drollery;
mod illuminate;
mod scribe;
mod style;

/// Semgrep-backed code search, behind the `cli` feature. Powers `--find` and
/// `--scan`.
#[cfg(feature = "cli")]
pub mod search;

/// The magnifica art-project modes, behind the `cli` feature. Powers
/// `--magnifica`.
#[cfg(feature = "cli")]
pub mod magnifica;

use std::collections::HashSet;

use figlet_rs::FIGfont;

pub use border::Border;
pub use style::Theme;

use crate::illuminate::{display_width, illuminate_paragraph, Line, Options};
use crate::style::Style;

/// The narrowest a body column is allowed to get, so the page stays readable.
pub const MIN_WIDTH: usize = 24;

/// The marginal rule between a drollery and the body, and its visible width.
const MARGIN_RULE: char = '┊';
const MARGIN_SEP_W: usize = 3;
/// Blank rows left between successive drolleries down the margin.
const DROLLERY_GAP: usize = 3;
/// Blank columns between text columns in a multi-column layout.
const COLUMN_GUTTER: usize = 3;

/// Which paragraphs receive an illuminated initial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropCap {
    /// Only the first paragraph.
    First,
    /// Every paragraph.
    All,
    /// None.
    None,
}

/// The typeface for the illuminated initials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    /// Ornate Fraktur capitals — the manuscript look.
    Blackletter,
    /// Plain FIGlet block capitals.
    Standard,
}

/// Everything that controls how a page is rendered. Construct via
/// [`Config::default`] and adjust the fields you care about.
#[derive(Clone, Debug)]
pub struct Config {
    /// Column width for the wrapped body text (clamped to [`MIN_WIDTH`]).
    pub width: usize,
    /// The frame around the page.
    pub border: Border,
    /// The colour palette.
    pub theme: Theme,
    /// Whether to emit ANSI colour at all.
    pub colored: bool,
    /// Which paragraphs are illuminated with a drop cap.
    pub drop_cap: DropCap,
    /// The initial typeface.
    pub font: Font,
    /// Words to rubricate, already lower-cased.
    pub rubrics: HashSet<String>,
    /// Adorn the left margin with ASCII drolleries.
    pub drolleries: bool,
    /// Seed varying which drolleries appear.
    pub seed: u64,
    /// Run paragraphs together, separated by an inline ¶.
    pub pilcrows: bool,
    /// Justify body lines flush to both margins.
    pub justify: bool,
    /// Hyphenate hard-broken over-long words.
    pub hyphenate: bool,
    /// Fill short closing lines with ❧ ornaments.
    pub fillers: bool,
    /// Rubricate the opening line, like a manuscript incipit.
    pub incipit: bool,
    /// Number of text columns (1 = single column).
    pub columns: usize,
    /// Illuminate the input as source code rather than prose: keep lines
    /// verbatim, rubricate keywords, and set comments as marginal glosses.
    pub code: bool,
    /// The language to use in code mode (by name, e.g. `"rust"`). When `None`,
    /// a generic fallback is used.
    pub language: Option<String>,
    /// Introduce scribal transcription errors, purposefully breaking the text
    /// (and code). Varied by [`Config::seed`].
    pub corrupt: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            width: 60,
            border: Border::Ornate,
            theme: Theme::Gold,
            colored: false,
            drop_cap: DropCap::First,
            font: Font::Blackletter,
            rubrics: HashSet::new(),
            drolleries: false,
            seed: 0,
            pilcrows: false,
            justify: false,
            hyphenate: false,
            fillers: false,
            incipit: false,
            columns: 1,
            code: false,
            language: None,
            corrupt: false,
        }
    }
}

/// The canonical language name for a filename's extension, if recognised (e.g.
/// `"main.rs"` → `Some("rust")`). Used to auto-enable code mode.
///
/// ```
/// use lindisfarner::detect_language;
///
/// assert_eq!(detect_language("main.rs").as_deref(), Some("rust"));
/// assert_eq!(detect_language("notes.txt"), None);
/// ```
#[must_use]
pub fn detect_language(filename: &str) -> Option<String> {
    let ext = std::path::Path::new(filename).extension()?.to_str()?;
    code::by_extension(ext).map(|lang| lang.name.to_string())
}

/// Render `source` into a finished, illuminated page.
#[must_use]
pub fn render(source: &str, cfg: &Config) -> String {
    let width = cfg.width.max(MIN_WIDTH);
    let style = Style::new(cfg.colored, cfg.theme);

    // A careless scribe meddles with the text before it is ever set down.
    let corrupted;
    let source = if cfg.corrupt {
        corrupted = scribe::corrupt(source, cfg.seed);
        corrupted.as_str()
    } else {
        source
    };

    // Two illumination modes: a code file kept line-for-line with glossed
    // comments, or prose wrapped and decorated.
    let (body, body_w) = if cfg.code {
        let lang = cfg
            .language
            .as_deref()
            .and_then(code::by_name)
            .unwrap_or_else(code::generic);
        code::illuminate(source, lang, &style, width)
    } else {
        lay_prose(source, cfg, width, &style)
    };

    frame(&body, body_w, cfg, &style)
}

/// Render explicit `(code, gloss)` rows as an illuminated commentary page: each
/// code line rubricated, its gloss set in the margin, the whole framed. Used to
/// present `--find` matches (the gloss is each match's `file:line`).
///
/// ```
/// use lindisfarner::{render_glossed, Config};
///
/// let rows = vec![("let x = 1;".to_string(), "a binding".to_string())];
/// let page = render_glossed(&rows, &Config::default());
/// assert!(page.contains("a binding")); // the gloss is set in the margin
/// ```
#[must_use]
pub fn render_glossed(rows: &[(String, String)], cfg: &Config) -> String {
    let width = cfg.width.max(MIN_WIDTH);
    let style = Style::new(cfg.colored, cfg.theme);
    let lang = cfg
        .language
        .as_deref()
        .and_then(code::by_name)
        .unwrap_or_else(code::generic);
    let (body, body_w) = code::lay_rows(rows, lang, &style, width);
    frame(&body, body_w, cfg, &style)
}

/// Wrap and decorate `source` as prose, returning the body and its width.
fn lay_prose(source: &str, cfg: &Config, width: usize, style: &Style) -> (Vec<Line>, usize) {
    let columns = cfg.columns.max(1);
    let font = load_font(cfg.font);

    // In a multi-column layout each column is wrapped to its own narrower width.
    let col_w = if columns >= 2 {
        (width.saturating_sub((columns - 1) * COLUMN_GUTTER) / columns).max(1)
    } else {
        width
    };

    let opts = Options {
        width: col_w,
        gap: 1,
        style,
        rubrics: &cfg.rubrics,
        justify: cfg.justify,
        hyphenate: cfg.hyphenate,
        fillers: cfg.fillers,
    };

    let content = lay_body(source, cfg, &font, &opts);

    if columns >= 2 {
        let laid = illuminate::lay_in_columns(&content, columns, col_w, COLUMN_GUTTER);
        let total = columns * col_w + (columns - 1) * COLUMN_GUTTER;
        (laid, total)
    } else {
        (content, width)
    }
}

/// Attach an optional drollery margin and the border, returning the finished
/// page. Shared by both illumination modes.
fn frame(body: &[Line], body_w: usize, cfg: &Config, style: &Style) -> String {
    let rows = if cfg.drolleries && !body.is_empty() {
        let (margin, margin_w) = scatter_drolleries(body.len(), cfg.seed, style);
        let sep = format!(" {} ", style.border(&MARGIN_RULE.to_string()));
        let merged = illuminate::merge_columns(&margin, body, margin_w, body_w, &sep, MARGIN_SEP_W);
        let total = margin_w + MARGIN_SEP_W + body_w;
        border::render(&merged, total, cfg.border, style)
    } else {
        border::render(body, body_w, cfg.border, style)
    };
    rows.join("\n")
}

/// Load the chosen FIGlet font. Both options are baked into the binary, so this
/// only fails on a corrupt build — hence the `expect`.
fn load_font(font: Font) -> FIGfont {
    match font {
        Font::Standard => FIGfont::standard().expect("built-in FIGlet standard font"),
        Font::Blackletter => FIGfont::from_content(include_str!("../fonts/fraktur.flf"))
            .expect("embedded Fraktur font is valid"),
    }
}

/// Split the source into paragraphs and lay each out, honouring pilcrows (which
/// run the paragraphs together as one continuous block) and the drop-cap rule.
fn lay_body(source: &str, cfg: &Config, font: &FIGfont, opts: &Options) -> Vec<Line> {
    let source = source.replace("\r\n", "\n").replace('\r', "\n");
    let split: Vec<&str> = source
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    let joined;
    let paragraphs: Vec<&str> = if cfg.pilcrows {
        joined = split.join(" ¶ ");
        vec![joined.as_str()]
    } else {
        split
    };

    let mut content: Vec<Line> = Vec::new();
    for (i, para) in paragraphs.iter().enumerate() {
        let drop_cap = match cfg.drop_cap {
            DropCap::All => true,
            DropCap::None => false,
            DropCap::First => i == 0,
        };
        if i > 0 {
            content.push(Line {
                shown: String::new(),
                len: 0,
            }); // blank spacer line
        }
        let incipit = cfg.incipit && i == 0;
        content.extend(illuminate_paragraph(para, drop_cap, incipit, font, opts));
    }
    content
}

/// Build a margin column `height` rows tall, scattering drolleries down it at
/// fixed intervals. Returns the margin and its width.
fn scatter_drolleries(height: usize, seed: u64, style: &Style) -> (Vec<Line>, usize) {
    let margin_w = drollery::max_width();
    let mut margin: Vec<Line> = (0..height)
        .map(|_| Line {
            shown: String::new(),
            len: 0,
        })
        .collect();

    let mut idx = 0;
    let mut nth = 0u64;
    while idx < margin.len() {
        let figure = drollery::pick(seed, nth);
        for (r, row) in figure.iter().enumerate() {
            if idx + r >= margin.len() {
                break;
            }
            margin[idx + r] = Line {
                shown: style.border(row),
                len: display_width(row),
            };
        }
        idx += figure.len() + DROLLERY_GAP;
        nth += 1;
    }
    (margin, margin_w)
}
