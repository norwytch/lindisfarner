//! End-to-end tests that run the built binary and inspect its output.
//!
//! Cargo sets `CARGO_BIN_EXE_lindisfarner` to the path of the compiled binary
//! for integration tests, so no extra dev-dependency is needed to locate it.
//! Tests run with the crate root as the working directory, hence the bare
//! `sample.txt`.

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
    let out = bin()
        .args(["sample.txt", "-c", "never"])
        .output()
        .expect("failed to run binary");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
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
    let out = bin().arg("sample.txt").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains('\u{1b}'));
}

#[test]
fn forced_colour_emits_escapes() {
    let out = bin().args(["sample.txt", "-c", "always"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains('\u{1b}'), "expected ANSI escapes");
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
    let out = bin()
        .args([
            "sample.txt",
            "-c",
            "never",
            "--border",
            "none",
            "--pilcrows",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.matches('¶').count(), sample_breaks());
}

#[test]
fn pilcrows_run_paragraphs_together() {
    let breaks = sample_breaks();

    // By default each paragraph stands alone, separated by a blank line.
    let plain = bin()
        .args([
            "sample.txt",
            "-c",
            "never",
            "--border",
            "none",
            "-d",
            "none",
        ])
        .output()
        .unwrap();
    let plain = String::from_utf8(plain.stdout).unwrap();
    let plain_blanks = plain.lines().filter(|l| l.trim().is_empty()).count();
    assert_eq!(
        plain_blanks, breaks,
        "default uses one blank line per break"
    );

    // With --pilcrows the paragraphs flow into one continuous block: no blank
    // separators at all, and fewer lines overall than the spaced-out default.
    let piped = bin()
        .args([
            "sample.txt",
            "-c",
            "never",
            "--border",
            "none",
            "-d",
            "none",
            "--pilcrows",
        ])
        .output()
        .unwrap();
    let piped = String::from_utf8(piped.stdout).unwrap();
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
    let out = bin()
        .args([
            "sample.txt",
            "-c",
            "never",
            "--border",
            "none",
            "--pilcrows",
            "--drolleries",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

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
