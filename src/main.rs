//! lindisfarner — the command-line front end.
//!
//! Parses arguments (defined in [`cli`]), gathers the input, routes to the right
//! mode, and hands off to the `lindisfarner` library for the illumination.

mod cli;
mod magnifica;
mod search;

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use clap::{CommandFactory, Parser};

use cli::Args;
use lindisfarner::{render, render_glossed, Config};

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

    // The art project, then code search, then the default: illuminate the input.
    if let Some(mode) = args.magnifica {
        return run_magnifica(&args, mode);
    }
    if args.find.is_some() || args.scan.is_some() {
        return run_search(&args);
    }

    let source = match &args.file {
        Some(path) => fs::read_to_string(path).map_err(|e| at_path(path, e))?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };
    let rendered = render(&source, &args.prose_config());
    write_output(args.output.as_deref(), &rendered)
}

/// Search `args.file` with Semgrep — a `--find` pattern or a `--scan` config —
/// and illuminate the findings as a glossed commentary page.
fn run_search(args: &Args) -> io::Result<()> {
    let path = args
        .file
        .as_deref()
        .ok_or_else(|| io::Error::other("a FILE or directory to search is required"))?;

    // A bare pattern needs a language; rule configs carry their own.
    let lang = args
        .language
        .clone()
        .or_else(|| path.to_str().and_then(lindisfarner::detect_language));

    let rows = if let Some(rules) = &args.scan {
        search::scan(path, rules)?
    } else {
        let pattern = args.find.as_deref().expect("find or scan is set");
        let lang = lang.clone().ok_or_else(|| {
            io::Error::other("could not detect the language; pass --language (e.g. rust)")
        })?;
        search::find(path, pattern, &lang)?
    };
    let rows = if rows.is_empty() {
        vec![("nothing found".to_string(), String::new())]
    } else {
        rows
    };

    let cfg = Config {
        language: lang, // used to rubricate the matched code
        ..args.base_config()
    };
    let rendered = render_glossed(&rows, &cfg);
    write_output(args.output.as_deref(), &rendered)
}

/// Run a magnifica mode over `args.file`.
fn run_magnifica(args: &Args, mode: magnifica::Mode) -> io::Result<()> {
    let path = args
        .file
        .as_deref()
        .ok_or_else(|| io::Error::other("point me at a FILE or directory of code"))?;

    let plan = magnifica::Plan {
        mode,
        path,
        quotes: args.quotes.as_deref(),
        seed: args.seed,
        cfg: args.base_config(),
    };
    let rendered = magnifica::run(&plan)?;
    write_output(args.output.as_deref(), &rendered)
}

/// Write the finished page to a file or standard output.
fn write_output(output: Option<&Path>, rendered: &str) -> io::Result<()> {
    match output {
        Some(path) => fs::write(path, format!("{rendered}\n")).map_err(|e| at_path(path, e)),
        None => {
            let mut out = io::stdout().lock();
            writeln!(out, "{rendered}")
        }
    }
}

/// Tag an I/O error with the path it concerns.
fn at_path(path: &Path, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}
