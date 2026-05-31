//! lindisfarner — the command-line front end.
//!
//! Parses arguments, gathers the source text, and hands off to the
//! `lindisfarner` library for the actual illumination. Everything here is
//! input/output and option plumbing; the rendering lives in the library.

use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::Shell;

use lindisfarner::{render, Border, Config, DropCap, Font, Theme, MIN_WIDTH};

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

    /// Column width for the wrapped body text (defaults to the terminal width)
    #[arg(short, long)]
    width: Option<usize>,

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

    /// Run paragraphs together, separating them with a ¶ instead of a blank line
    #[arg(short, long)]
    pilcrows: bool,

    /// Justify body text flush to both margins
    #[arg(short = 'j', long)]
    justify: bool,

    /// Break over-long words with a trailing hyphen
    #[arg(long)]
    hyphenate: bool,

    /// Fill short closing lines with ❧ ornaments
    #[arg(long)]
    fillers: bool,

    /// Rubricate the opening line, like a manuscript incipit
    #[arg(long)]
    incipit: bool,

    /// Set the body in this many columns, codex-style
    #[arg(long, default_value_t = 1)]
    columns: usize,

    /// Illuminate the input as source code: rubricate keywords, gloss comments
    #[arg(long)]
    code: bool,

    /// Force prose illumination even for a recognised code file
    #[arg(long, conflicts_with = "code")]
    prose: bool,

    /// Override the code-mode language (e.g. rust, python, c, go, shell)
    #[arg(long, value_name = "LANG")]
    language: Option<String>,

    /// Print a shell completion script for SHELL and exit
    #[arg(long, value_enum, value_name = "SHELL")]
    completions: Option<Shell>,

    /// Print a roff man page and exit
    #[arg(long)]
    man: bool,
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

#[derive(Copy, Clone, Debug, ValueEnum)]
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

fn main() {
    if let Err(e) = run() {
        // A closed pipe (e.g. `lindisfarner … | head`) is normal; exit quietly.
        if e.kind() == io::ErrorKind::BrokenPipe {
            return;
        }
        eprintln!("lindisfarner: {e}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let args = Args::parse();

    // Generator modes short-circuit before any input is read.
    if let Some(shell) = args.completions {
        let mut cmd = Args::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut io::stdout());
        return Ok(());
    }
    if args.man {
        clap_mangen::Man::new(Args::command()).render(&mut io::stdout())?;
        return Ok(());
    }

    // Gather the source text from a file or standard input.
    let source = match &args.file {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    // Colour turns on for a terminal and off when piped or written to a file.
    let colored = match args.color {
        ColorArg::Always => true,
        ColorArg::Never => false,
        ColorArg::Auto => args.output.is_none() && io::stdout().is_terminal(),
    };

    let rubrics: HashSet<String> = args
        .rubricate
        .iter()
        .map(|w| w.trim().to_lowercase())
        .filter(|w| !w.is_empty())
        .collect();

    // Code mode is on with --code or --language, or auto-detected from a
    // recognised source-file extension; --prose forces it off.
    let detected = args
        .file
        .as_deref()
        .and_then(|p| p.to_str())
        .and_then(lindisfarner::detect_language);
    let code = !args.prose && (args.code || args.language.is_some() || detected.is_some());
    let language = args.language.clone().or(detected);

    let cfg = Config {
        width: args.width.unwrap_or_else(default_width),
        border: args.border.into(),
        theme: args.theme.into(),
        colored,
        drop_cap: args.drop_cap.into(),
        font: args.font.into(),
        rubrics,
        drolleries: args.drolleries,
        seed: args.seed,
        pilcrows: args.pilcrows,
        justify: args.justify,
        hyphenate: args.hyphenate,
        fillers: args.fillers,
        incipit: args.incipit,
        columns: args.columns,
        code,
        language,
    };

    let rendered = render(&source, &cfg);

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
