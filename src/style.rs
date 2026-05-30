//! Pigments for the scriptorium.
//!
//! Colour is the digital stand-in for gold leaf and red lead. Every painted
//! string is wrapped in an ANSI escape only when colour is actually enabled,
//! so the same code path produces clean plain text when piped to a file.

#[derive(Clone, Copy, Debug)]
pub(crate) enum Theme {
    /// Gold initials, yellow border, red rubrics — the classic look.
    Gold,
    /// Crimson initials and border, gold rubrics.
    Crimson,
    /// No colour at all, regardless of the `--color` setting.
    Mono,
}

/// The three illuminated "roles" and the escape code each uses.
struct Palette {
    initial: &'static str,
    border: &'static str,
    rubric: &'static str,
}

const RESET: &str = "\x1b[0m";

impl Theme {
    fn palette(self) -> Palette {
        match self {
            Theme::Gold => Palette {
                initial: "\x1b[1;33m", // bold yellow — gold leaf
                border: "\x1b[33m",    // yellow
                rubric: "\x1b[1;31m",  // bold red — the rubric
            },
            Theme::Crimson => Palette {
                initial: "\x1b[1;31m",
                border: "\x1b[31m",
                rubric: "\x1b[1;33m",
            },
            Theme::Mono => Palette {
                initial: "",
                border: "",
                rubric: "",
            },
        }
    }
}

/// Decides whether and how to paint a string.
pub(crate) struct Style {
    enabled: bool,
    theme: Theme,
}

impl Style {
    pub(crate) fn new(enabled: bool, theme: Theme) -> Self {
        Style { enabled, theme }
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

    /// Paint a pilcrow. Paragraph marks were historically drawn in red lead —
    /// the very practice "rubrication" is named for — so it shares that pigment.
    pub(crate) fn pilcrow(&self, s: &str) -> String {
        self.paint(self.theme.palette().rubric, s)
    }
}
