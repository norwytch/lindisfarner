//! lindisfarner — a digital scriptorium.
//!
//! Reads a plain text file (or standard input) and "illuminates" it: a large
//! ASCII-art initial opens the text, chosen words are rubricated, and the whole
//! page is set inside a decorative border.

mod border;
mod drollery;
mod illuminate;
mod style;

use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use figlet_rs::FIGfont;

use crate::border::Border;
use crate::illuminate::{display_width, illuminate_paragraph, Line, Options};
use crate::style::{Style, Theme};

/// The marginal rule between a drollery and the body text, and its visible
/// width (a single-column glyph flanked by two spaces).
const MARGIN_RULE: char = '┊';
const MARGIN_SEP_W: usize = 3;

#[derive(Parser, Debug)]
#[command(
    name = "lindisfarner",
    version,
    about = "A digital scriptorium: illuminate plain text with ASCII-art initials, rubrics, and ornate borders."
)]
struct Args {
    /// Input file to illuminate (reads standard input if omitted)
    file: Option<PathBuf>,

    /// Write the result to a file instead of standard output
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Column width for the wrapped body text
    #[arg(short, long, default_value_t = 60)]
    width: usize,

    /// Border style around the page
    #[arg(short, long, value_enum, default_value_t = BorderArg::Ornate)]
    border: BorderArg,

    /// Colour theme for the pigments
    #[arg(short = 't', long, value_enum, default_value_t = ThemeArg::Gold)]
    theme: ThemeArg,

    /// When to emit colour
    #[arg(short, long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// Which paragraphs receive an illuminated initial
    #[arg(short = 'd', long, value_enum, default_value_t = DropCapArg::First)]
    drop_cap: DropCapArg,

    /// Typeface for the illuminated initials
    #[arg(short = 'f', long, value_enum, default_value_t = FontArg::Blackletter)]
    font: FontArg,

    /// Words to rubricate (highlight); comma-separated, may be repeated
    #[arg(short, long, value_delimiter = ',')]
    rubricate: Vec<String>,

    /// Adorn the left margin with whimsical ASCII drolleries
    #[arg(long)]
    drolleries: bool,

    /// Seed for drollery selection (varies which figures appear)
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Mark paragraph breaks with a ¶ pilcrow instead of a blank line
    #[arg(short, long)]
    pilcrows: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum BorderArg {
    None,
    Simple,
    Double,
    Ornate,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ThemeArg {
    Gold,
    Crimson,
    Mono,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Copy, Clone, Debug, PartialEq, ValueEnum)]
enum DropCapArg {
    First,
    All,
    None,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum FontArg {
    /// Ornate Fraktur capitals — the manuscript look (default)
    Blackletter,
    /// Plain FIGlet block capitals
    Standard,
}

impl From<BorderArg> for Border {
    fn from(b: BorderArg) -> Self {
        match b {
            BorderArg::None => Border::None,
            BorderArg::Simple => Border::Simple,
            BorderArg::Double => Border::Double,
            BorderArg::Ornate => Border::Ornate,
        }
    }
}

impl From<ThemeArg> for Theme {
    fn from(t: ThemeArg) -> Self {
        match t {
            ThemeArg::Gold => Theme::Gold,
            ThemeArg::Crimson => Theme::Crimson,
            ThemeArg::Mono => Theme::Mono,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("lindisfarner: {e}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let args = Args::parse();

    // Gather the source text from a file or standard input. Normalise Windows
    // and classic-Mac line endings so paragraph detection works the same way
    // regardless of where the file came from.
    let source = match &args.file {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };
    let source = source.replace("\r\n", "\n").replace('\r', "\n");

    // Keep the page readable: don't let the width fall below a sane minimum.
    let width = args.width.max(24);

    // Decide whether to colourise. When writing to a file or a non-terminal,
    // "auto" stays plain so the saved page contains no escape codes.
    let colored = match args.color {
        ColorArg::Always => true,
        ColorArg::Never => false,
        ColorArg::Auto => args.output.is_none() && io::stdout().is_terminal(),
    };

    let style = Style::new(colored, args.theme.into());

    let rubrics: HashSet<String> = args
        .rubricate
        .iter()
        .map(|w| w.trim().to_lowercase())
        .filter(|w| !w.is_empty())
        .collect();

    let font = match args.font {
        FontArg::Standard => FIGfont::standard().map_err(io::Error::other)?,
        FontArg::Blackletter => {
            // The Fraktur font is embedded in the binary, so no font file is
            // needed at runtime.
            FIGfont::from_content(include_str!("../fonts/fraktur.flf")).map_err(io::Error::other)?
        }
    };

    let opts = Options {
        width,
        gap: 1,
        style: &style,
        rubrics: &rubrics,
    };

    // Split the source into paragraphs on blank lines.
    let paragraphs: Vec<&str> = source
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    let mut content: Vec<Line> = Vec::new();
    let mut starts: Vec<usize> = Vec::new();
    for (i, para) in paragraphs.iter().enumerate() {
        let drop_cap = match args.drop_cap {
            DropCapArg::All => true,
            DropCapArg::None => false,
            DropCapArg::First => i == 0,
        };
        if i > 0 {
            content.push(Line {
                shown: String::new(),
                len: 0,
            }); // blank spacer line
        }
        starts.push(content.len());
        content.extend(illuminate_paragraph(para, drop_cap, &font, &opts));
    }

    // Either frame the body alone, or attach a margin of drolleries first.
    let rows = if args.drolleries && !content.is_empty() {
        let margin_w = drollery::max_width();
        let mut margin: Vec<Line> = (0..content.len())
            .map(|_| Line {
                shown: String::new(),
                len: 0,
            })
            .collect();

        for (pi, &start) in starts.iter().enumerate() {
            let figure = drollery::pick(args.seed, pi as u64);
            for (r, row) in figure.iter().enumerate() {
                let idx = start + r;
                if idx >= margin.len() {
                    break;
                }
                if margin[idx].len != 0 {
                    continue; // don't clobber a neighbouring figure
                }
                margin[idx] = Line {
                    shown: style.border(row),
                    len: display_width(row),
                };
            }
        }

        let sep = format!(" {} ", style.border(&MARGIN_RULE.to_string()));
        let merged =
            illuminate::merge_columns(&margin, &content, margin_w, width, &sep, MARGIN_SEP_W);
        let total = margin_w + MARGIN_SEP_W + width;
        border::render(&merged, total, args.border.into(), &style)
    } else {
        border::render(&content, width, args.border.into(), &style)
    };

    let rendered = rows.join("\n");

    match &args.output {
        Some(path) => fs::write(path, format!("{rendered}\n"))
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?,
        None => {
            let mut out = io::stdout().lock();
            writeln!(out, "{rendered}")?;
        }
    }

    Ok(())
}
