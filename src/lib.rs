//! lindisfarner — a digital scriptorium.
//!
//! Turn plain text into an "illuminated" page: a large ASCII-art initial opens
//! the text, chosen words are rubricated, marginal drolleries and pilcrows adorn
//! it, and the whole is set inside a decorative border.
//!
//! The entry point is [`render`], which takes the source text and a [`Config`]
//! and returns the finished page as a `String`:
//!
//! ```
//! use lindisfarner::{render, Config};
//!
//! let page = render("Hail and well met, traveller.", &Config::default());
//! assert!(page.contains('❦')); // the ornate frame's flourish
//! ```

mod border;
mod drollery;
mod illuminate;
mod style;

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
        }
    }
}

/// Render `source` into a finished, illuminated page.
pub fn render(source: &str, cfg: &Config) -> String {
    let width = cfg.width.max(MIN_WIDTH);
    let columns = cfg.columns.max(1);
    let style = Style::new(cfg.colored, cfg.theme);
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
        style: &style,
        rubrics: &cfg.rubrics,
        justify: cfg.justify,
        hyphenate: cfg.hyphenate,
        fillers: cfg.fillers,
    };

    let content = lay_body(source, cfg, &font, &opts);

    // Set the body into columns if asked, then note its overall width.
    let (body, body_w) = if columns >= 2 {
        let laid = illuminate::lay_in_columns(&content, columns, col_w, COLUMN_GUTTER);
        let total = columns * col_w + (columns - 1) * COLUMN_GUTTER;
        (laid, total)
    } else {
        (content, width)
    };

    let rows = if cfg.drolleries && !body.is_empty() {
        let (margin, margin_w) = scatter_drolleries(body.len(), cfg.seed, &style);
        let sep = format!(" {} ", style.border(&MARGIN_RULE.to_string()));
        let merged =
            illuminate::merge_columns(&margin, &body, margin_w, body_w, &sep, MARGIN_SEP_W);
        let total = margin_w + MARGIN_SEP_W + body_w;
        border::render(&merged, total, cfg.border, &style)
    } else {
        border::render(&body, body_w, cfg.border, &style)
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
