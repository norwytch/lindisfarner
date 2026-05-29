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
