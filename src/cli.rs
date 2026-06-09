//! Command-line surface: the argument definitions, their mapping to library
//! types, and the small helpers that turn a parsed `Args` into a `Config`.

use std::io::{self, IsTerminal};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use clap_complete::Shell;

use lindisfarner::{Border, Config, DropCap, Font, Theme, MIN_WIDTH};

use crate::magnifica;

#[derive(Parser, Debug)]
#[command(
    name = "lindisfarner",
    version,
    about = "A digital scriptorium: illuminate plain text with ASCII-art initials, rubrics, and ornate borders."
)]
pub(crate) struct Args {
    /// Input file to illuminate (reads standard input if omitted)
    pub(crate) file: Option<PathBuf>,

    /// Write the result to a file instead of standard output
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,

    /// Column width for the wrapped body text (defaults to the terminal width)
    #[arg(short, long)]
    pub(crate) width: Option<usize>,

    /// Border style around the page
    #[arg(short, long, value_enum, default_value_t = BorderArg::Ornate)]
    pub(crate) border: BorderArg,

    /// Colour theme for the pigments
    #[arg(short = 't', long, value_enum, default_value_t = ThemeArg::Gold)]
    pub(crate) theme: ThemeArg,

    /// When to emit colour
    #[arg(short, long, value_enum, default_value_t = ColorArg::Auto)]
    pub(crate) color: ColorArg,

    /// Which paragraphs receive an illuminated initial
    #[arg(short = 'd', long, value_enum, default_value_t = DropCapArg::First)]
    pub(crate) drop_cap: DropCapArg,

    /// Typeface for the illuminated initials
    #[arg(short = 'f', long, value_enum, default_value_t = FontArg::Blackletter)]
    pub(crate) font: FontArg,

    /// Words to rubricate (highlight); comma-separated, may be repeated
    #[arg(short, long, value_delimiter = ',')]
    pub(crate) rubricate: Vec<String>,

    /// Adorn the left margin with whimsical ASCII drolleries
    #[arg(long)]
    pub(crate) drolleries: bool,

    /// Seed for drollery selection (varies which figures appear)
    #[arg(long, default_value_t = 0)]
    pub(crate) seed: u64,

    /// Run paragraphs together, separating them with a ¶ instead of a blank line
    #[arg(short, long)]
    pub(crate) pilcrows: bool,

    /// Justify body text flush to both margins
    #[arg(short = 'j', long)]
    pub(crate) justify: bool,

    /// Break over-long words with a trailing hyphen
    #[arg(long)]
    pub(crate) hyphenate: bool,

    /// Fill short closing lines with ❧ ornaments
    #[arg(long)]
    pub(crate) fillers: bool,

    /// Rubricate the opening line, like a manuscript incipit
    #[arg(long)]
    pub(crate) incipit: bool,

    /// Set the body in this many columns, codex-style
    #[arg(long, default_value_t = 1)]
    pub(crate) columns: usize,

    /// Illuminate the input as source code: rubricate keywords, gloss comments
    #[arg(long)]
    pub(crate) code: bool,

    /// Force prose illumination even for a recognised code file
    #[arg(long, conflicts_with = "code")]
    pub(crate) prose: bool,

    /// Override the code-mode language (e.g. rust, python, c, go, shell)
    #[arg(long, value_name = "LANG")]
    pub(crate) language: Option<String>,

    /// Introduce scribal transcription errors that purposefully break the text
    /// (and code); vary them with --seed
    #[arg(long)]
    pub(crate) corrupt: bool,

    /// Search the FILE (or directory) for a Semgrep pattern and illuminate the
    /// matching code as a glossed commentary page, e.g. '$X.unwrap()'
    #[arg(long, value_name = "PATTERN")]
    pub(crate) find: Option<String>,

    /// Scan the FILE (or directory) with a Semgrep rule config (file or
    /// directory); each finding is glossed with its message, marked by severity
    #[arg(long, value_name = "RULES", conflicts_with = "find")]
    pub(crate) scan: Option<PathBuf>,

    /// An art project: find AI-tool usage in the FILE (or directory) and answer
    /// with the encyclical Magnifica Humanitas, in one of two modes
    #[arg(long, value_enum, value_name = "MODE")]
    pub(crate) magnifica: Option<magnifica::Mode>,

    /// Use a different passages file for the magnifica modes
    #[arg(long, value_name = "FILE")]
    pub(crate) quotes: Option<PathBuf>,

    /// Print a shell completion script for SHELL and exit
    #[arg(long, value_enum, value_name = "SHELL")]
    pub(crate) completions: Option<Shell>,

    /// Print a roff man page and exit
    #[arg(long)]
    pub(crate) man: bool,
}

impl Args {
    /// Whether to emit colour: forced on/off, or auto (on for a terminal, off
    /// when piped or written to a file).
    pub(crate) fn colored(&self) -> bool {
        match self.color {
            ColorArg::Always => true,
            ColorArg::Never => false,
            ColorArg::Auto => self.output.is_none() && io::stdout().is_terminal(),
        }
    }

    /// The page-framing options common to every mode: width, border, theme, and
    /// colour. Other fields keep their defaults.
    pub(crate) fn base_config(&self) -> Config {
        Config {
            width: self.width.unwrap_or_else(default_width),
            border: self.border.into(),
            theme: self.theme.into(),
            colored: self.colored(),
            drolleries: self.drolleries,
            seed: self.seed,
            ..Config::default()
        }
    }

    /// The full prose-illumination config, including rubrics and the resolved
    /// code-mode decision.
    pub(crate) fn prose_config(&self) -> Config {
        let rubrics = self
            .rubricate
            .iter()
            .map(|w| w.trim().to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();

        // Code mode is on with --code/--language, or auto-detected from a
        // recognised extension; --prose forces it off.
        let detected = self
            .file
            .as_deref()
            .and_then(|p| p.to_str())
            .and_then(lindisfarner::detect_language);
        let code = !self.prose && (self.code || self.language.is_some() || detected.is_some());
        let language = self.language.clone().or(detected);

        Config {
            drop_cap: self.drop_cap.into(),
            font: self.font.into(),
            rubrics,
            pilcrows: self.pilcrows,
            justify: self.justify,
            hyphenate: self.hyphenate,
            fillers: self.fillers,
            incipit: self.incipit,
            columns: self.columns,
            code,
            language,
            corrupt: self.corrupt,
            ..self.base_config()
        }
    }
}

/// Default body width: the terminal width (less room for the border) when
/// writing to a terminal, otherwise a sane fixed default for pipes and files.
fn default_width() -> usize {
    use terminal_size::{terminal_size, Width};
    if io::stdout().is_terminal() {
        if let Some((Width(cols), _)) = terminal_size() {
            return (cols as usize).saturating_sub(4).max(MIN_WIDTH);
        }
    }
    60
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum BorderArg {
    None,
    Simple,
    Double,
    Ornate,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum ThemeArg {
    Gold,
    Crimson,
    Mono,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum DropCapArg {
    First,
    All,
    None,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum FontArg {
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

impl From<DropCapArg> for DropCap {
    fn from(d: DropCapArg) -> Self {
        match d {
            DropCapArg::First => DropCap::First,
            DropCapArg::All => DropCap::All,
            DropCapArg::None => DropCap::None,
        }
    }
}

impl From<FontArg> for Font {
    fn from(f: FontArg) -> Self {
        match f {
            FontArg::Blackletter => Font::Blackletter,
            FontArg::Standard => Font::Standard,
        }
    }
}
