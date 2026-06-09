//! End-to-end tests that run the built binary and inspect its output.
//!
//! Cargo sets `CARGO_BIN_EXE_lindisfarner` to the path of the compiled binary
//! for integration tests, so no extra dev-dependency is needed to locate it.
//! Tests run with the crate root as the working directory, hence the bare
//! `sample.txt`.
//!
//! The binary only exists with the `cli` feature (on by default), so the whole
//! suite is gated on it — `cargo test --no-default-features` skips it cleanly.
#![cfg(feature = "cli")]

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lindisfarner"))
}

/// The visible width of a rendered line, ignoring ANSI escape sequences. The
/// fixtures here are ASCII plus single-column box-drawing glyphs, so a `char`
/// count equals the column count once escapes are stripped.
fn visible_width(line: &str) -> usize {
    let mut width = 0;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip a CSI sequence: ESC '[' ... final byte in @–~.
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

#[test]
fn renders_a_file_as_a_rectangular_block() {
    let stdout = stdout_of(&["sample.txt", "-c", "never"]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() > 3, "expected a framed block");

    // The ornate frame is on by default, and `-c never` means no escapes.
    assert!(stdout.contains('╭') && stdout.contains('❦'));
    assert!(
        !stdout.contains('\u{1b}'),
        "plain output must have no colour"
    );

    // Every row of the page is the same width — the whole point of the layout.
    let want = visible_width(lines[0]);
    for line in &lines {
        assert_eq!(visible_width(line), want, "ragged line: {line:?}");
    }
}

#[test]
fn auto_colour_stays_plain_when_piped() {
    // stdout is a pipe here (not a tty), so `auto` must not emit escapes.
    assert!(!stdout_of(&["sample.txt"]).contains('\u{1b}'));
}

#[test]
fn forced_colour_emits_escapes() {
    assert!(
        stdout_of(&["sample.txt", "-c", "always"]).contains('\u{1b}'),
        "expected ANSI escapes"
    );
}

#[test]
fn reads_from_stdin() {
    let mut child = bin()
        .args(["-c", "never", "--border", "none"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Hello from standard input")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // The drop-cap initial is rendered as art, so the literal word is broken up,
    // but the tail of the first word still appears.
    assert!(stdout.to_lowercase().contains("ello"));
}

#[test]
fn missing_file_reports_a_friendly_error() {
    let out = bin().arg("does-not-exist-12345.txt").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("lindisfarner:"));
    assert!(stderr.contains("does-not-exist-12345.txt"));
    // Friendly message, not a Debug dump of the error struct.
    assert!(!stderr.contains("Os {"));
}

/// Number of paragraph breaks in the fixture: one fewer than the count of
/// blank-line-separated, non-empty blocks. Derived so the tests don't hardcode
/// a count that drifts when sample.txt changes.
fn sample_breaks() -> usize {
    // Normalise line endings exactly as the binary does, so the count is right
    // even when git checks the fixture out with CRLF (e.g. on Windows).
    let text = std::fs::read_to_string("sample.txt")
        .unwrap()
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let paragraphs = text.split("\n\n").filter(|p| !p.trim().is_empty()).count();
    paragraphs.saturating_sub(1)
}

#[test]
fn pilcrows_mark_each_paragraph_break() {
    let stdout = stdout_of(&[
        "sample.txt",
        "-c",
        "never",
        "--border",
        "none",
        "--pilcrows",
    ]);
    assert_eq!(stdout.matches('¶').count(), sample_breaks());
}

#[test]
fn pilcrows_run_paragraphs_together() {
    let breaks = sample_breaks();

    // By default each paragraph stands alone, separated by a blank line.
    let plain = stdout_of(&[
        "sample.txt",
        "-c",
        "never",
        "--border",
        "none",
        "-d",
        "none",
    ]);
    let plain_blanks = plain.lines().filter(|l| l.trim().is_empty()).count();
    assert_eq!(
        plain_blanks, breaks,
        "default uses one blank line per break"
    );

    // With --pilcrows the paragraphs flow into one continuous block: no blank
    // separators at all, and fewer lines overall than the spaced-out default.
    let piped = stdout_of(&[
        "sample.txt",
        "-c",
        "never",
        "--border",
        "none",
        "-d",
        "none",
        "--pilcrows",
    ]);
    let piped_blanks = piped.lines().filter(|l| l.trim().is_empty()).count();
    assert_eq!(piped_blanks, 0, "pilcrows should leave no blank separators");
    assert!(
        piped.lines().count() < plain.lines().count(),
        "continuous flow should use fewer lines than the spaced default"
    );
}

#[test]
fn drolleries_fill_the_margin_regardless_of_paragraphs() {
    // Even with --pilcrows (a single continuous paragraph), figures are
    // scattered down the whole margin rather than tied to paragraph starts.
    let stdout = stdout_of(&[
        "sample.txt",
        "-c",
        "never",
        "--border",
        "none",
        "--pilcrows",
        "--drolleries",
    ]);

    // Count rows whose margin cell (left of the ┊ rule) carries a figure.
    let figured = stdout
        .lines()
        .filter(|l| {
            l.split_once('┊')
                .is_some_and(|(margin, _)| !margin.trim().is_empty())
        })
        .count();

    // The tallest single figure is four rows, so more than four figured rows
    // can only mean two or more figures were placed.
    assert!(
        figured > 4,
        "expected several drolleries down the margin, got {figured} figured rows"
    );
}

fn stdout_of(args: &[&str]) -> String {
    let out = bin().args(args).output().unwrap();
    assert!(out.status.success(), "command failed: {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn justify_distributes_spaces_in_interior_lines() {
    // Justification pads inter-word gaps, producing runs of multiple spaces that
    // ordinary prose (single-spaced) never contains.
    let just = stdout_of(&[
        "sample.txt",
        "-j",
        "-d",
        "none",
        "--border",
        "none",
        "-c",
        "never",
        "-w",
        "50",
    ]);
    let plain = stdout_of(&[
        "sample.txt",
        "-d",
        "none",
        "--border",
        "none",
        "-c",
        "never",
        "-w",
        "50",
    ]);
    assert!(
        just.lines().any(|l| l.trim_end().contains("  ")),
        "justified output should contain distributed gaps"
    );
    assert!(
        !plain.lines().any(|l| l.trim_end().contains("  ")),
        "unjustified prose should be single-spaced"
    );
}

#[test]
fn two_columns_halve_the_height() {
    let one = stdout_of(&[
        "sample.txt",
        "-d",
        "none",
        "--border",
        "none",
        "-c",
        "never",
        "-w",
        "84",
    ]);
    let two = stdout_of(&[
        "sample.txt",
        "-d",
        "none",
        "--border",
        "none",
        "-c",
        "never",
        "-w",
        "84",
        "--columns",
        "2",
    ]);
    assert!(
        two.lines().count() < one.lines().count(),
        "two columns ({}) should be shorter than one ({})",
        two.lines().count(),
        one.lines().count()
    );
}

#[test]
fn hyphenate_breaks_a_long_word_with_a_dash() {
    let mut child = bin()
        .args([
            "--hyphenate",
            "-d",
            "none",
            "--border",
            "none",
            "-c",
            "never",
            "-w",
            "12",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"antidisestablishmentarianism")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.lines().any(|l| l.trim_end().ends_with('-')),
        "expected a hyphenated break"
    );
}

#[test]
fn completions_and_man_page_generate() {
    assert!(stdout_of(&["--completions", "bash"]).contains("_lindisfarner"));
    assert!(stdout_of(&["--completions", "zsh"]).contains("#compdef"));
    assert!(stdout_of(&["--man"]).contains(".TH"));
}

fn stdout_of_stdin(args: &[&str], input: &[u8]) -> String {
    let mut child = bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "command failed: {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn code_mode_glosses_comments_and_strips_markers() {
    let out = stdout_of_stdin(
        &[
            "--code",
            "--language",
            "rust",
            "-c",
            "never",
            "--border",
            "none",
        ],
        b"fn main() { let x = 5; } // a note\n",
    );
    assert!(out.contains('┊'), "expected a gloss margin");
    assert!(
        out.contains("a note"),
        "comment text should appear as a gloss"
    );
    assert!(
        !out.contains("//"),
        "the comment marker should be stripped from the code"
    );
}

#[test]
fn code_mode_rubricates_keywords() {
    let out = stdout_of_stdin(
        &[
            "--code",
            "--language",
            "rust",
            "-c",
            "always",
            "--border",
            "none",
        ],
        b"fn main() {}\n",
    );
    assert!(
        out.contains("\u{1b}[1;31mfn\u{1b}[0m"),
        "the keyword should be rubricated in red"
    );
}

#[test]
fn code_mode_auto_detects_by_extension() {
    let path = std::env::temp_dir().join("lindisfarner_codemode_test.py");
    std::fs::write(&path, b"def f():  # gloss me\n    return 1\n").unwrap();
    let out = stdout_of(&[path.to_str().unwrap(), "-c", "never", "--border", "none"]);
    std::fs::remove_file(&path).ok();
    assert!(
        out.contains('┊') && out.contains("gloss me"),
        "a .py file should auto-enable code mode"
    );
}

/// Whether Semgrep is available; the search/magnifica-witness tests skip
/// without it (e.g. on CI), and they're covered by the pure unit tests anyway.
fn semgrep_missing() -> bool {
    Command::new("semgrep").arg("--version").output().is_err()
}

#[test]
fn search_reports_a_missing_path() {
    // Semgrep returns empty results for a bad path; we must error, not silently
    // report "nothing found". The check runs before Semgrep, so no tool needed.
    let out = bin()
        .args([
            "/tmp/lindisfarner-no-such-path-xyz",
            "--find",
            "$X.foo()",
            "--language",
            "rust",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("no such file or directory"),
        "expected a missing-path error, got: {stderr}"
    );
}

#[test]
fn find_illuminates_matches() {
    if semgrep_missing() {
        eprintln!("skipping find_illuminates_matches: semgrep not installed");
        return;
    }
    // Search the crate's own source for a structural pattern.
    let out = stdout_of(&[
        "src",
        "--find",
        "$X.unwrap()",
        "--language",
        "rust",
        "-c",
        "never",
        "-w",
        "120",
    ]);
    assert!(out.contains('┊'), "expected a gloss margin");
    assert!(out.contains("unwrap"), "expected matched code");
    assert!(
        out.contains("src/") && out.contains(".rs:"),
        "expected file:line glosses"
    );
}

#[test]
fn scan_glosses_findings_with_rule_messages() {
    if semgrep_missing() {
        eprintln!("skipping scan_glosses_findings_with_rule_messages: semgrep not installed");
        return;
    }
    let dir = std::env::temp_dir();
    let rule = dir.join("lindisfarner_scan_rule.yml");
    let code = dir.join("lindisfarner_scan_code.rs");
    std::fs::write(
        &rule,
        "rules:\n  - id: no-unwrap\n    languages: [rust]\n    severity: WARNING\n    message: avoid unwrap\n    pattern: $X.unwrap()\n",
    )
    .unwrap();
    std::fs::write(&code, "fn main() { let x = foo().unwrap(); }\n").unwrap();

    let out = stdout_of(&[
        code.to_str().unwrap(),
        "--scan",
        rule.to_str().unwrap(),
        "-c",
        "never",
        "-w",
        "80",
    ]);
    std::fs::remove_file(&rule).ok();
    std::fs::remove_file(&code).ok();

    assert!(
        out.contains("avoid unwrap"),
        "the rule message should be glossed"
    );
    assert!(
        out.contains('☞'),
        "a warning should carry the manicule mark"
    );
}

#[test]
fn scan_accepts_a_rules_directory() {
    if semgrep_missing() {
        eprintln!("skipping scan_accepts_a_rules_directory: semgrep not installed");
        return;
    }
    // The committed rules/ directory, run as a whole against the crate's source.
    let out = stdout_of(&["src", "--scan", "rules", "-c", "never", "-w", "120"]);
    assert!(
        out.contains("unwrap"),
        "the no-unwrap rule should fire on the codebase"
    );
    assert!(
        out.contains('☞') || out.contains('❧'),
        "findings should carry severity marks"
    );
}

#[test]
fn magnifica_relinquish_strikes_ai_blocks_on_disk() {
    if semgrep_missing() {
        eprintln!("skipping magnifica_relinquish: semgrep not installed");
        return;
    }
    let dir = std::env::temp_dir().join("lindisfarner_relinquish_test");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("bot.py");
    std::fs::write(
        &file,
        b"x = client.messages.create(model=\"c\")\ndef helper():\n    return 42\n",
    )
    .unwrap();

    let out = stdout_of(&[
        dir.to_str().unwrap(),
        "--magnifica",
        "relinquish",
        "-c",
        "never",
        "-w",
        "78",
    ]);
    let rewritten = std::fs::read_to_string(&file).unwrap();
    std::fs::remove_dir_all(&dir).ok();

    // The report names the action and reads the encyclical.
    assert!(
        out.contains("relinquished"),
        "the report should name the action"
    );
    assert!(
        out.contains("Magnifica Humanitas"),
        "the encyclical should appear in the report"
    );

    // On disk: the AI block is struck out and the encyclical stands in its place
    // (as comments); the non-AI code survives.
    assert!(
        !rewritten.contains("messages.create"),
        "the AI block should be struck from the file"
    );
    assert!(
        rewritten.contains("# ") && rewritten.contains("Magnifica Humanitas"),
        "the encyclical should be written into the file as a comment"
    );
    assert!(
        rewritten.contains("relinquished"),
        "the strike should leave its mark in the file"
    );
    assert!(
        rewritten.contains("def helper"),
        "non-AI code should survive on disk"
    );
}

#[test]
fn magnifica_witness_annotates_ai_usage_on_disk() {
    if semgrep_missing() {
        eprintln!("skipping magnifica_witness: semgrep not installed");
        return;
    }
    let dir = std::env::temp_dir().join("lindisfarner_magnifica_test");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("bot.py");
    std::fs::write(
        &file,
        b"import anthropic\nimport torch\nx = client.messages.create(model=\"c\")\n",
    )
    .unwrap();
    std::fs::write(dir.join("model.safetensors"), b"\0\0\0\0").unwrap(); // a "weight" file

    let out = stdout_of(&[
        dir.to_str().unwrap(),
        "--magnifica",
        "witness",
        "-c",
        "never",
        "-w",
        "110",
    ]);
    let annotated = std::fs::read_to_string(&file).unwrap();
    std::fs::remove_dir_all(&dir).ok();

    // The report names the AI usage, the weights, and reads the encyclical.
    assert!(out.contains("annotated"), "should name the action");
    assert!(out.contains("invocation"), "should name the AI usage");
    assert!(
        out.contains("framework"),
        "should name the ML framework (torch)"
    );
    assert!(
        out.contains("model weights"),
        "should list the weight file Semgrep can't see"
    );
    assert!(
        out.contains("Magnifica Humanitas"),
        "should quote the encyclical"
    );

    // On disk: the encyclical is inserted as a comment, but the code still runs.
    assert!(
        annotated.contains("# ") && annotated.contains("Magnifica Humanitas"),
        "the encyclical should be written into the file as a comment"
    );
    assert!(
        annotated.contains("messages.create") && annotated.contains("import torch"),
        "the code itself should be left intact"
    );
}

#[test]
fn corrupt_breaks_the_text_deterministically() {
    let args = [
        "sample.txt",
        "--corrupt",
        "--seed",
        "4",
        "-c",
        "never",
        "-d",
        "none",
        "--border",
        "none",
    ];
    let a = stdout_of(&args);
    let b = stdout_of(&args);
    let clean = stdout_of(&[
        "sample.txt",
        "-c",
        "never",
        "-d",
        "none",
        "--border",
        "none",
    ]);
    assert_eq!(a, b, "same seed should corrupt identically");
    assert_ne!(a, clean, "corruption should change the output");
}

#[test]
fn prose_flag_overrides_code_detection() {
    // Forcing prose on a .py file should wrap/decorate it, not gloss it.
    let path = std::env::temp_dir().join("lindisfarner_prose_test.py");
    std::fs::write(&path, b"def f():  # not a gloss\n    return 1\n").unwrap();
    let out = stdout_of(&[
        path.to_str().unwrap(),
        "--prose",
        "-c",
        "never",
        "--border",
        "none",
    ]);
    std::fs::remove_file(&path).ok();
    assert!(
        !out.contains('┊'),
        "prose mode should not produce a gloss margin"
    );
}
