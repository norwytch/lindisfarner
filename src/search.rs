//! The Semgrep bridge.
//!
//! lindisfarner does not link Semgrep; it runs the CLI as a subprocess and
//! parses its `--json` output. Two entry points return ready-to-render
//! `(code, gloss)` rows for [`crate::render_glossed`]:
//!
//! - [`find`] runs a single pattern and glosses each match with its `file:line`.
//! - [`scan`] runs a rule config (file or directory) and glosses each finding
//!   with the rule's message, marked by severity.
//!
//! Semgrep redacts the matched source in its JSON for unauthenticated runs, so
//! the matched code is read back from the file by line range — which also keeps
//! the JSON parsing pure and testable.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// One Semgrep result: a location, plus the rule message and severity when the
/// result came from a rule config.
pub(crate) struct Finding {
    pub(crate) file: String,
    pub(crate) line: u64,
    pub(crate) end_line: u64,
    pub(crate) message: Option<String>,
    pub(crate) severity: Option<String>,
}

impl Finding {
    /// The first line of the matched span, read from the file.
    pub(crate) fn signature(&self) -> String {
        self.span_text()
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    pub(crate) fn location(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }

    /// The matched lines, read back from the file (Semgrep gates them in JSON).
    fn span_text(&self) -> String {
        let lines: Vec<String> = fs::read_to_string(&self.file)
            .map(|s| s.lines().map(String::from).collect())
            .unwrap_or_default();
        let start = (self.line as usize).saturating_sub(1);
        let end = (self.end_line as usize).min(lines.len());
        lines
            .get(start..end)
            .map(|s| s.join("\n"))
            .unwrap_or_default()
    }
}

/// Find matches of a single `pattern`; each row is the matched code with its
/// `file:line` as the gloss.
pub(crate) fn find(path: &Path, pattern: &str, lang: &str) -> io::Result<Vec<(String, String)>> {
    let json = run(
        path,
        &["scan", "--pattern", pattern, "--lang", lang_id(lang)],
    )?;
    Ok(parse(&json)
        .into_iter()
        .map(|f| (f.signature(), f.location()))
        .collect())
}

/// Scan with a rule config (a file or a directory of rules); each row is the
/// matched code with the rule's message — prefixed by a severity mark — gloss.
pub(crate) fn scan(path: &Path, rules: &Path) -> io::Result<Vec<(String, String)>> {
    let rules = rules
        .to_str()
        .ok_or_else(|| io::Error::other("the rules path is not valid UTF-8"))?;
    let json = run(path, &["scan", "--config", rules])?;
    Ok(parse(&json).into_iter().map(row_for).collect())
}

/// Run one or more rule sets (given as inline text) and return structured
/// findings — the lower-level entry point used by the magnifica modes, which
/// need each match's file and line. Semgrep has no inline-rules flag, so each
/// rule set is written to a file in a temporary config directory and the whole
/// directory is scanned at once.
pub(crate) fn findings(path: &Path, rule_texts: &[&str]) -> io::Result<Vec<Finding>> {
    let dir = std::env::temp_dir().join(format!("lindisfarner-rules-{}", std::process::id()));
    fs::create_dir_all(&dir)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", dir.display())))?;
    for (i, text) in rule_texts.iter().enumerate() {
        fs::write(dir.join(format!("r{i}.yml")), text)?;
    }
    let json = run(
        path,
        &["scan", "--config", dir.to_str().unwrap_or_default()],
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(parse(&json?))
}

/// A rendered row for a rule finding: matched code, and the message marked by
/// severity.
fn row_for(f: Finding) -> (String, String) {
    let mark = severity_mark(f.severity.as_deref().unwrap_or("INFO"));
    let message = f.message.clone().unwrap_or_default();
    (f.signature(), format!("{mark} {message}"))
}

/// Run `semgrep` with the given arguments plus the search `path`, returning its
/// JSON stdout. A missing binary becomes a friendly install hint.
fn run(path: &Path, args: &[&str]) -> io::Result<String> {
    // Semgrep reports an empty result set (not an error) for a path that does
    // not exist, which would otherwise read as "nothing found". Catch it first.
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{}: no such file or directory", path.display()),
        ));
    }

    let output = Command::new("semgrep")
        .args(args)
        .args(["--json", "--quiet", "--metrics=off"])
        .arg(path)
        .output()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "semgrep not found — install it (e.g. `pip install semgrep`); see https://semgrep.dev",
                )
            } else {
                e
            }
        })?;

    // Valid Semgrep output is a JSON object with a `results` array, even when
    // empty. If we don't get that, surface the error rather than report nothing.
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let is_results = serde_json::from_str::<Value>(&stdout)
        .ok()
        .is_some_and(|v| v.get("results").is_some());
    if is_results {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::other(format!(
            "semgrep failed: {}",
            stderr.trim().lines().last().unwrap_or("unknown error")
        )))
    }
}

/// Parse Semgrep's `--json` output into findings (pure: no file access).
fn parse(json: &str) -> Vec<Finding> {
    let value: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    value
        .get("results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|r| {
            let line = r.get("start")?.get("line")?.as_u64()?;
            let end_line = r
                .get("end")
                .and_then(|e| e.get("line"))
                .and_then(Value::as_u64)
                .unwrap_or(line);
            let extra = r.get("extra");
            Some(Finding {
                file: r.get("path")?.as_str()?.to_string(),
                line,
                end_line,
                message: extra
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .map(String::from),
                severity: extra
                    .and_then(|e| e.get("severity"))
                    .and_then(Value::as_str)
                    .map(String::from),
            })
        })
        .collect()
}

/// Map a lindisfarner language name to Semgrep's language id.
fn lang_id(name: &str) -> &str {
    match name {
        "shell" => "bash",
        other => other, // rust, python, javascript, c, go all match Semgrep
    }
}

/// A scribe's mark for a Semgrep severity (ERROR / WARNING / INFO): the obelus
/// (†) for errors — historically set beside spurious text — a manicule (☞) for
/// a warning, else a fleuron.
fn severity_mark(severity: &str) -> char {
    match severity.to_ascii_uppercase().as_str() {
        "ERROR" => '†',
        "WARNING" => '☞',
        _ => '❧',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_fields() {
        let json = r#"{"results": [
            {"check_id": "no-unwrap", "path": "src/scribe.rs",
             "start": {"line": 12}, "end": {"line": 12},
             "extra": {"message": "avoid unwrap", "severity": "WARNING"}},
            {"check_id": "-", "path": "probe.rs",
             "start": {"line": 1}, "end": {"line": 3}, "extra": {}}
        ], "errors": []}"#;
        let got = parse(json);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].location(), "src/scribe.rs:12");
        assert_eq!(got[0].message.as_deref(), Some("avoid unwrap"));
        assert_eq!(got[0].severity.as_deref(), Some("WARNING"));
        assert_eq!(got[1].line, 1);
        assert_eq!(got[1].end_line, 3);
    }

    #[test]
    fn parse_tolerates_empty_or_garbage() {
        assert!(parse(r#"{"results": []}"#).is_empty());
        assert!(parse("not json").is_empty());
    }

    #[test]
    fn severity_marks() {
        assert_eq!(severity_mark("ERROR"), '†');
        assert_eq!(severity_mark("WARNING"), '☞');
        assert_eq!(severity_mark("INFO"), '❧');
    }
}
