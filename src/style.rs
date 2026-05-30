//! Pigments for the scriptorium.
//!
//! Colour is the digital stand-in for gold leaf and red lead. Every painted
//! string is wrapped in an ANSI escape only when colour is actually enabled,
//! so the same code path produces clean plain text when piped to a file.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    /// Gold initials, yellow border, red rubrics — the classic look.
    Gold,
    /// Crimson initials and border, gold rubrics.
    Crimson,
    /// No colour at all, regardless of the `--color` setting.
    Mono,
}

/// The illuminated "roles" and the escape code each uses. `rubric_alt` is the
/// second pilcrow colour: scribes alternated red and blue paragraph marks down
/// a page, and we do the same.
struct Palette {
    initial: &'static str,
    border: &'static str,
    rubric: &'static str,
    rubric_alt: &'static str,
}

const RESET: &str = "\x1b[0m";

impl Theme {
    fn palette(self) -> Palette {
        match self {
            Theme::Gold => Palette {
                initial: "\x1b[1;33m",    // bold yellow — gold leaf
                border: "\x1b[33m",       // yellow
                rubric: "\x1b[1;31m",     // bold red — the rubric
                rubric_alt: "\x1b[1;34m", // bold blue — the alternating pilcrow
            },
            Theme::Crimson => Palette {
                initial: "\x1b[1;31m",
                border: "\x1b[31m",
                rubric: "\x1b[1;33m",
                rubric_alt: "\x1b[1;34m",
            },
            Theme::Mono => Palette {
                initial: "",
                border: "",
                rubric: "",
                rubric_alt: "",
            },
        }
    }
}

/// Decides whether and how to paint a string. Holds a small counter so that
/// successive pilcrows alternate colour.
pub(crate) struct Style {
    enabled: bool,
    theme: Theme,
    pilcrow_n: Cell<usize>,
}

impl Style {
    pub(crate) fn new(enabled: bool, theme: Theme) -> Self {
        Style {
            enabled,
            theme,
            pilcrow_n: Cell::new(0),
        }
    }

    fn paint(&self, code: &str, s: &str) -> String {
        if self.enabled && !code.is_empty() {
            format!("{code}{s}{RESET}")
        } else {
            s.to_string()
        }
    }

    /// Paint an illuminated initial (the big drop-cap letter).
    pub(crate) fn initial(&self, s: &str) -> String {
        self.paint(self.theme.palette().initial, s)
    }

    /// Paint a border glyph.
    pub(crate) fn border(&self, s: &str) -> String {
        self.paint(self.theme.palette().border, s)
    }

    /// Paint a rubricated word.
    pub(crate) fn rubric(&self, s: &str) -> String {
        self.paint(self.theme.palette().rubric, s)
    }

    /// Paint a pilcrow, alternating red and blue down the page. Paragraph marks
    /// were historically drawn in red lead — the practice "rubrication" is named
    /// for — then a blue, then red again, by two passes of the rubricator.
    pub(crate) fn pilcrow(&self, s: &str) -> String {
        let n = self.pilcrow_n.get();
        self.pilcrow_n.set(n + 1);
        let palette = self.theme.palette();
        let code = if n % 2 == 0 {
            palette.rubric
        } else {
            palette.rubric_alt
        };
        self.paint(code, s)
    }
}
