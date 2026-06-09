//! The magnifica modes — an art project on the use of AI tools, named for the
//! encyclical *Magnifica Humanitas*.
//!
//! Point lindisfarner at a codebase; it finds where AI is used — hosted-vendor
//! API calls and the ML lifecycle (via the embedded Semgrep rules), plus model
//! weight files on disk — and answers with the words of the encyclical, in one
//! of two modes. **Both write the changes to the files on disk**, then print an
//! illuminated report:
//!
//! - **witness** — annotate the source: insert the encyclical as a comment
//!   beside every AI invocation. The code is not broken.
//! - **relinquish** — strike each AI block out of the source and leave the
//!   encyclical's words in its place, breaking what it touches.
//!
//! The writes go straight to disk with no safety net — keeping a backup (or a
//! clean commit) is the caller's responsibility. Weight files are reported but
//! never modified.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use clap::ValueEnum;
use lindisfarner::{render, render_glossed, Config};

use crate::search::{self, Finding};

/// The detection rules and the encyclical passages, baked into the binary.
const AI_RULES: &str = include_str!("../rules/ai.yml");
const ML_RULES: &str = include_str!("../rules/ml.yml");
const QUOTES: &str = include_str!("../assets/quotes.txt");

/// File extensions that denote trained model weights — found by walking the
/// filesystem, since Semgrep reads only source code, not binary artifacts.
const WEIGHT_EXTENSIONS: &[&str] = &[
    "pt",
    "pth",
    "safetensors",
    "ckpt",
    "h5",
    "hdf5",
    "onnx",
    "gguf",
    "ggml",
    "tflite",
    "joblib",
    "npz",
    "caffemodel",
    "mlmodel",
    "pdparams",
    "pkl",
];

/// Directories not worth walking for weight files.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    "build",
];

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum Mode {
    /// Annotate the source with the encyclical beside every AI invocation.
    Witness,
    /// Strike each AI block out of the source, leaving the encyclical in its place.
    Relinquish,
}

/// Everything a magnifica run needs.
pub(crate) struct Plan<'a> {
    pub mode: Mode,
    pub path: &'a Path,
    pub quotes: Option<&'a Path>,
    pub seed: u64,
    pub cfg: Config,
}

/// Carry out the plan: write the encyclical into the files, then return the
/// illuminated report to print.
pub(crate) fn run(plan: &Plan) -> io::Result<String> {
    let quotes = load_quotes(plan.quotes)?;
    if quotes.is_empty() {
        return Err(io::Error::other("no encyclical passages to draw from"));
    }

    let findings = search::findings(plan.path, &[AI_RULES, ML_RULES])?;
    let weights = weight_files(plan.path);

    // The matched lines — signature, what they are, and where — captured before
    // we rewrite the files (which would change what `signature` reads back).
    let located: Vec<(String, String, String)> = findings
        .iter()
        .map(|f| {
            let label = f
                .message
                .clone()
                .unwrap_or_else(|| "an AI invocation".into());
            (f.signature(), label, f.location())
        })
        .collect();

    if findings.is_empty() {
        if weights.is_empty() {
            return Ok(render(
                "No AI tools were found here. For now, this remains the work of human hands.",
                &plan.cfg,
            ));
        }
        // Only weights — nothing in the source to write.
        return Ok(report(plan, &quotes, &[], &weights, "found"));
    }

    let verb = match plan.mode {
        Mode::Witness => {
            annotate(plan, &quotes, &findings)?;
            "annotated"
        }
        Mode::Relinquish => {
            strike(plan, &quotes, &findings)?;
            "relinquished"
        }
    };
    Ok(report(plan, &quotes, &located, &weights, verb))
}

/// The illuminated report printed after writing: each location with the action
/// taken, the (untouched) weight files, and a reading from the encyclical.
fn report(
    plan: &Plan,
    quotes: &[String],
    located: &[(String, String, String)],
    weights: &[(String, u64)],
    verb: &str,
) -> String {
    let mut rows: Vec<(String, String)> = located
        .iter()
        .map(|(sig, label, loc)| (sig.clone(), format!("{label} · {verb} · {loc}")))
        .collect();
    for (file, size) in weights {
        rows.push((
            file.clone(),
            format!("model weights · {} (left in place)", human_size(*size)),
        ));
    }
    let usage = render_glossed(&rows, &plan.cfg);
    let sermon = render(
        &sermon_text(quotes, plan.seed, rows.len().max(1)),
        &plan.cfg,
    );
    let note = if located.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n  {} location(s) {verb}. Review with `git diff`; undo with `git checkout`.",
            located.len()
        )
    };
    format!("{usage}\n\n{sermon}{note}")
}

// ---- writing the files -----------------------------------------------------

/// witness: insert the encyclical as a comment above every AI invocation,
/// leaving the code itself in place.
fn annotate(plan: &Plan, quotes: &[String], findings: &[Finding]) -> io::Result<()> {
    for (file, group) in group_by_file(findings) {
        let marker = comment_marker(file);
        let mut lines = read_lines(file)?;
        // Insert from the bottom up so earlier line numbers stay valid.
        let mut ordered: Vec<&&Finding> = group.iter().collect();
        ordered.sort_by_key(|f| std::cmp::Reverse(f.line));
        for f in ordered {
            let at = (f.line as usize).saturating_sub(1).min(lines.len());
            let indent = leading_whitespace(lines.get(at).map(String::as_str).unwrap_or(""));
            let block = comment_block(quotes, plan, f.line, &indent, marker);
            lines.splice(at..at, block);
        }
        write_lines(file, &lines)?;
    }
    Ok(())
}

/// relinquish: replace each AI block with the encyclical's words (as comments),
/// breaking the code where it called out.
fn strike(plan: &Plan, quotes: &[String], findings: &[Finding]) -> io::Result<()> {
    for (file, group) in group_by_file(findings) {
        let marker = comment_marker(file);
        let mut lines = read_lines(file)?;
        let mut ordered: Vec<&&Finding> = group.iter().collect();
        ordered.sort_by_key(|f| std::cmp::Reverse(f.line));
        for f in ordered {
            let start = (f.line as usize).saturating_sub(1).min(lines.len());
            let end = (f.end_line as usize).min(lines.len());
            let indent = leading_whitespace(lines.get(start).map(String::as_str).unwrap_or(""));
            let mut block = comment_block(quotes, plan, f.line, &indent, marker);
            block.push(format!("{indent}{marker} (an AI invocation, relinquished)"));
            lines.splice(start..end, block);
        }
        write_lines(file, &lines)?;
    }
    Ok(())
}

/// A passage wrapped into comment lines at the given indentation.
fn comment_block(
    quotes: &[String],
    plan: &Plan,
    n: u64,
    indent: &str,
    marker: &str,
) -> Vec<String> {
    let quote = pick(quotes, plan.seed, n);
    let avail = plan
        .cfg
        .width
        .saturating_sub(indent.len() + marker.len() + 2)
        .max(20);
    word_wrap(quote, avail)
        .into_iter()
        .map(|l| format!("{indent}{marker} {l}"))
        .collect()
}

/// The line-comment marker for a file, by extension.
fn comment_marker(file: &str) -> &'static str {
    match Path::new(file).extension().and_then(|e| e.to_str()) {
        Some("py" | "rb" | "sh" | "bash" | "yaml" | "yml" | "toml" | "pl" | "r") => "#",
        Some("lua" | "sql" | "hs") => "--",
        _ => "//",
    }
}

fn write_lines(file: &str, lines: &[String]) -> io::Result<()> {
    fs::write(file, lines.join("\n")).map_err(|e| io::Error::new(e.kind(), format!("{file}: {e}")))
}

// ---- helpers ---------------------------------------------------------------

fn load_quotes(path: Option<&Path>) -> io::Result<Vec<String>> {
    let text = match path {
        Some(p) => fs::read_to_string(p)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", p.display())))?,
        None => QUOTES.to_string(),
    };
    Ok(parse_quotes(&text))
}

/// Passages are separated by blank lines; `#` lines are comments.
fn parse_quotes(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .split("\n\n")
        .map(|block| {
            block
                .lines()
                .filter(|l| !l.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|q| !q.is_empty())
        .collect()
}

/// A short reading: up to `want` distinct passages, chosen by the seed.
fn sermon_text(quotes: &[String], seed: u64, want: usize) -> String {
    let want = want.clamp(1, quotes.len().min(6));
    let start = (pick_index(seed, 0) % quotes.len() as u64) as usize;
    (0..want)
        .map(|i| quotes[(start + i) % quotes.len()].clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Deterministically choose a passage for a given line/index.
fn pick(quotes: &[String], seed: u64, n: u64) -> &str {
    &quotes[(pick_index(seed, n) % quotes.len() as u64) as usize]
}

/// A small deterministic hash (splitmix64).
fn pick_index(seed: u64, n: u64) -> u64 {
    let mut z = seed
        .wrapping_add(n.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(0x6D2B_79F5);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Group findings by their file, preserving order within each file.
fn group_by_file(findings: &[Finding]) -> BTreeMap<&str, Vec<&Finding>> {
    let mut map: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for f in findings {
        map.entry(f.file.as_str()).or_default().push(f);
    }
    map
}

fn read_lines(file: &str) -> io::Result<Vec<String>> {
    let text =
        fs::read_to_string(file).map_err(|e| io::Error::new(e.kind(), format!("{file}: {e}")))?;
    Ok(text
        .replace("\r\n", "\n")
        .split('\n')
        .map(String::from)
        .collect())
}

/// The leading whitespace of a line, so a replacement sits at its indentation.
fn leading_whitespace(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Greedily wrap `text` to `width` columns (the passages are plain text).
fn word_wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.len() + 1 + word.len() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

// ---- model weights (the filesystem half Semgrep can't do) ------------------

/// Walk `path` for trained-model weight files, returning `(display path, size)`
/// sorted by path. Common heavy directories are skipped.
fn weight_files(path: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    collect_weights(path, &mut out);
    out.sort();
    out
}

fn collect_weights(path: &Path, out: &mut Vec<(String, u64)>) {
    if path.is_file() {
        if is_weight(path) {
            if let Ok(meta) = path.metadata() {
                out.push((path.display().to_string(), meta.len()));
            }
        }
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let skip = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if !skip {
                collect_weights(&p, out);
            }
        } else if is_weight(&p) {
            if let Ok(meta) = p.metadata() {
                out.push((p.display().to_string(), meta.len()));
            }
        }
    }
}

/// Whether a path has a model-weight extension.
fn is_weight(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|e| WEIGHT_EXTENSIONS.contains(&e.as_str()))
}

/// Format a byte count as a human-readable size.
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_quotes_parse_cleanly() {
        let quotes = parse_quotes(QUOTES);
        assert!(quotes.len() >= 10, "expected the curated passages");
        assert!(
            quotes.iter().all(|q| !q.starts_with('#')),
            "no comment lines"
        );
        assert!(
            quotes.iter().all(|q| !q.contains("  ")),
            "whitespace normalised"
        );
        assert!(quotes.iter().any(|q| q.contains("Magnifica Humanitas")));
    }

    #[test]
    fn pick_is_deterministic() {
        let quotes = parse_quotes(QUOTES);
        assert_eq!(pick(&quotes, 5, 3), pick(&quotes, 5, 3));
    }

    #[test]
    fn word_wrap_respects_width() {
        let lines = word_wrap("the quick brown fox jumps over the lazy dog", 12);
        assert!(lines.iter().all(|l| l.len() <= 12), "lines fit the width");
        assert_eq!(
            lines.join(" "),
            "the quick brown fox jumps over the lazy dog"
        );
    }

    #[test]
    fn weight_extensions_recognised() {
        assert!(is_weight(Path::new("model.safetensors")));
        assert!(is_weight(Path::new("ckpt/epoch.PT"))); // case-insensitive
        assert!(is_weight(Path::new("a/b/weights.onnx")));
        assert!(!is_weight(Path::new("train.py")));
        assert!(!is_weight(Path::new("README.md")));
    }

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
        assert!(human_size(3 * 1024 * 1024 * 1024).ends_with("GB"));
    }
}
