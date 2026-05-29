//! The core illumination: word-wrapping, drop-cap initials, and composition.
//!
//! The crown of the craft is the *drop cap*: the first letter of a paragraph is
//! rendered large (via a FIGlet font) and the opening lines of the paragraph
//! flow down its right-hand side, exactly as a scribe would set text beside a
//! decorated versal.

use std::collections::{HashSet, VecDeque};

use figlet_rs::FIGfont;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::style::Style;

/// The number of terminal columns a string occupies. This is *not* the same as
/// its byte length or its `char` count: CJK and many emoji are two columns wide,
/// and combining marks are zero, so we ask `unicode-width` rather than guess.
pub(crate) fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// A rendered line together with its *visible* width. The visible width can
/// differ from the byte length once ANSI colour codes have been woven in, so we
/// track it explicitly to keep the right margin aligned.
pub(crate) struct Line {
    pub(crate) shown: String,
    pub(crate) len: usize,
}

impl Line {
    fn new(shown: String, len: usize) -> Self {
        Line { shown, len }
    }
}

/// Settings shared across paragraph rendering.
pub(crate) struct Options<'a> {
    pub(crate) width: usize,
    pub(crate) gap: usize,
    pub(crate) style: &'a Style,
    pub(crate) rubrics: &'a HashSet<String>,
}

/// Colour a word red if its alphanumeric core appears in the rubric set.
fn colorize_word(word: &str, style: &Style, rubrics: &HashSet<String>) -> String {
    if rubrics.is_empty() {
        return word.to_string();
    }
    let key: String = word
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if !key.is_empty() && rubrics.contains(&key) {
        style.rubric(word)
    } else {
        word.to_string()
    }
}

/// Greedily pack words into at most `max_lines` lines of `width` visible
/// columns. Over-long words are hard-broken. Consumed words are removed from the
/// front of the deque; any remainder is left for the caller to lay out next.
fn fill_lines(
    words: &mut VecDeque<String>,
    width: usize,
    max_lines: usize,
    style: &Style,
    rubrics: &HashSet<String>,
) -> Vec<Line> {
    let width = width.max(1);
    let mut lines = Vec::new();

    while !words.is_empty() && lines.len() < max_lines {
        let mut shown = String::new();
        let mut len = 0usize;

        while let Some(front) = words.front() {
            let wlen = display_width(front);

            // A single word wider than the line: hard-break it.
            if wlen > width {
                if len > 0 {
                    break; // finish the current line first
                }
                let word = words.pop_front().unwrap();
                let (head, head_w, tail) = split_at_width(&word, width);
                if !tail.is_empty() {
                    words.push_front(tail);
                }
                shown.push_str(&head);
                len = head_w;
                break;
            }

            let needed = if len == 0 { wlen } else { wlen + 1 };
            if len + needed > width {
                break;
            }

            let word = words.pop_front().unwrap();
            if len > 0 {
                shown.push(' ');
            }
            shown.push_str(&colorize_word(&word, style, rubrics));
            len += needed;
        }

        lines.push(Line::new(shown, len));
    }
    lines
}

fn tokenize(text: &str) -> VecDeque<String> {
    text.split_whitespace().map(|s| s.to_string()).collect()
}

/// Split `word` into a head that fits within `width` display columns and the
/// remaining tail. At least one character is always taken so a word that cannot
/// fit (e.g. a two-column glyph in a one-column slot) still makes progress
/// instead of looping forever. Returns `(head, head_width, tail)`.
fn split_at_width(word: &str, width: usize) -> (String, usize, String) {
    let mut head = String::new();
    let mut head_w = 0;
    let mut chars = word.chars();
    for c in chars.by_ref() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(0);
        if head_w + cw > width && !head.is_empty() {
            // This char overflows; put it back by rebuilding the tail below.
            let tail: String = std::iter::once(c).chain(chars).collect();
            return (head, head_w, tail);
        }
        head.push(c);
        head_w += cw;
    }
    (head, head_w, String::new())
}

/// Lay a `margin` column to the left of the `body` column, with `sep` ruling
/// between them, producing combined lines that are all the same visible width.
/// Either column may be shorter; missing rows are treated as blank.
pub(crate) fn merge_columns(
    margin: &[Line],
    body: &[Line],
    margin_w: usize,
    body_w: usize,
    sep: &str,
    sep_w: usize,
) -> Vec<Line> {
    let rows = margin.len().max(body.len());
    let blank = Line {
        shown: String::new(),
        len: 0,
    };
    let mut out = Vec::with_capacity(rows);
    for i in 0..rows {
        let m = margin.get(i).unwrap_or(&blank);
        let b = body.get(i).unwrap_or(&blank);
        let m_pad = " ".repeat(margin_w.saturating_sub(m.len));
        let b_pad = " ".repeat(body_w.saturating_sub(b.len));
        let shown = format!("{}{}{}{}{}", m.shown, m_pad, sep, b.shown, b_pad);
        out.push(Line {
            shown,
            len: margin_w + sep_w + body_w,
        });
    }
    out
}

/// Render a single character as a block of ASCII-art lines, normalised so every
/// row has the same width and no blank rows top or bottom.
fn render_glyph(ch: char, font: &FIGfont) -> Vec<String> {
    let upper: String = ch.to_uppercase().collect();
    let figure = match font.convert(&upper) {
        Some(f) => f.to_string(),
        None => return vec![ch.to_string()],
    };

    let mut lines: Vec<String> = figure.lines().map(|l| l.trim_end().to_string()).collect();
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return vec![ch.to_string()];
    }

    let w = lines.iter().map(|l| display_width(l)).max().unwrap_or(0);
    for l in &mut lines {
        let deficit = w - display_width(l);
        if deficit > 0 {
            l.push_str(&" ".repeat(deficit));
        }
    }
    lines
}

/// Illuminate one paragraph. With `drop_cap`, the first letter is rendered large
/// and the opening lines flow down its right-hand side.
pub(crate) fn illuminate_paragraph(
    text: &str,
    drop_cap: bool,
    font: &FIGfont,
    opts: &Options,
) -> Vec<Line> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    if !drop_cap {
        let mut words = tokenize(text);
        return fill_lines(&mut words, opts.width, usize::MAX, opts.style, opts.rubrics);
    }

    let mut chars = text.chars();
    let initial = chars.next().unwrap();
    let rest: String = chars.collect();

    let glyph = render_glyph(initial, font);
    let height = glyph.len();
    let glyph_w = glyph.iter().map(|l| display_width(l)).max().unwrap_or(0);
    let narrow = opts.width.saturating_sub(glyph_w + opts.gap);

    // Too little room for a drop cap: fall back to an ordinary paragraph.
    if narrow < 8 {
        let mut words = tokenize(text);
        return fill_lines(&mut words, opts.width, usize::MAX, opts.style, opts.rubrics);
    }

    let mut words = tokenize(&rest);
    let beside = fill_lines(&mut words, narrow, height, opts.style, opts.rubrics);
    let below = fill_lines(&mut words, opts.width, usize::MAX, opts.style, opts.rubrics);

    let gap = " ".repeat(opts.gap);
    let mut out = Vec::with_capacity(height + below.len());
    for (i, glyph_row) in glyph.iter().enumerate() {
        let glyph_line = opts.style.initial(glyph_row);
        let (body, body_len) = match beside.get(i) {
            Some(l) => (l.shown.clone(), l.len),
            None => (String::new(), 0),
        };
        let body_pad = " ".repeat(narrow - body_len);
        let shown = format!("{glyph_line}{gap}{body}{body_pad}");
        out.push(Line::new(shown, glyph_w + opts.gap + narrow));
    }
    out.extend(below);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Theme;

    fn no_rubrics() -> HashSet<String> {
        HashSet::new()
    }

    fn shown(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|l| l.shown.clone()).collect()
    }

    #[test]
    fn display_width_counts_columns_not_chars() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("私"), 2); // a wide (CJK) character is two columns
        assert_eq!(display_width("私は"), 4);
        assert_eq!(display_width("e\u{0301}"), 1); // 'e' + combining acute = one column
    }

    #[test]
    fn split_at_width_breaks_on_a_char_boundary() {
        assert_eq!(
            split_at_width("hello", 3),
            ("hel".to_string(), 3, "lo".to_string())
        );
    }

    #[test]
    fn split_at_width_respects_wide_characters() {
        // "私A" is 2 + 1 columns; a width of 2 takes only the wide char.
        assert_eq!(
            split_at_width("私A", 2),
            ("私".to_string(), 2, "A".to_string())
        );
    }

    #[test]
    fn split_at_width_always_makes_progress() {
        // A two-column glyph cannot fit a one-column slot, but we must still
        // consume it (and overflow by one) rather than loop forever.
        let (head, head_w, tail) = split_at_width("私A", 1);
        assert_eq!(head, "私");
        assert_eq!(head_w, 2);
        assert_eq!(tail, "A");
    }

    #[test]
    fn fill_lines_wraps_greedily() {
        let style = Style::new(false, Theme::Gold);
        let mut words = tokenize("the quick brown fox");
        let lines = fill_lines(&mut words, 9, usize::MAX, &style, &no_rubrics());
        assert_eq!(shown(&lines), vec!["the quick", "brown fox"]);
        assert!(lines.iter().all(|l| l.len <= 9));
        assert!(words.is_empty());
    }

    #[test]
    fn fill_lines_honours_max_lines_and_leaves_a_remainder() {
        let style = Style::new(false, Theme::Gold);
        let mut words = tokenize("alpha beta gamma delta");
        let lines = fill_lines(&mut words, 5, 1, &style, &no_rubrics());
        assert_eq!(shown(&lines), vec!["alpha"]);
        // The unconsumed words remain at the front of the deque.
        assert_eq!(words.front().map(String::as_str), Some("beta"));
    }

    #[test]
    fn fill_lines_hard_breaks_an_over_long_word() {
        let style = Style::new(false, Theme::Gold);
        let mut words = tokenize("supercalifragilistic");
        let lines = fill_lines(&mut words, 8, usize::MAX, &style, &no_rubrics());
        assert!(lines.iter().all(|l| l.len <= 8));
        // The pieces reassemble into the original word.
        let joined: String = lines.iter().map(|l| l.shown.as_str()).collect();
        assert_eq!(joined, "supercalifragilistic");
    }

    #[test]
    fn fill_lines_measures_wide_characters_for_layout() {
        let style = Style::new(false, Theme::Gold);
        // Three two-column glyphs; at width 5 only "私 は" (2+1+2) fits a line.
        let mut words = tokenize("私 は 猫");
        let lines = fill_lines(&mut words, 5, usize::MAX, &style, &no_rubrics());
        assert_eq!(shown(&lines), vec!["私 は", "猫"]);
        assert_eq!(lines[0].len, 5);
        assert_eq!(lines[1].len, 2);
    }

    #[test]
    fn fill_lines_rubricates_without_inflating_the_width() {
        let style = Style::new(true, Theme::Gold);
        let mut rubrics = HashSet::new();
        rubrics.insert("gold".to_string());
        let mut words = tokenize("the gold leaf");
        let lines = fill_lines(&mut words, 40, usize::MAX, &style, &rubrics);
        // The rubric escape is woven into the visible string...
        assert!(lines[0].shown.contains('\u{1b}'));
        // ...but the tracked width still counts only printable columns.
        assert_eq!(lines[0].len, "the gold leaf".len());
    }

    #[test]
    fn drop_cap_rows_are_exactly_the_page_width() {
        let font = FIGfont::standard().unwrap();
        let style = Style::new(false, Theme::Mono);
        let opts = Options {
            width: 40,
            gap: 1,
            style: &style,
            rubrics: &no_rubrics(),
        };
        let lines = illuminate_paragraph(
            "Hello world this is a reasonably long paragraph for the drop cap",
            true,
            &font,
            &opts,
        );
        assert!(!lines.is_empty());
        // The tall initial occupies the first rows, each padded to full width.
        assert_eq!(lines[0].len, 40);
    }

    #[test]
    fn tiny_width_falls_back_to_a_plain_paragraph() {
        let font = FIGfont::standard().unwrap();
        let style = Style::new(false, Theme::Mono);
        let text = "Hello world foo bar baz";
        let opts = Options {
            width: 10, // too narrow for a drop cap beside the glyph
            gap: 1,
            style: &style,
            rubrics: &no_rubrics(),
        };
        let got = shown(&illuminate_paragraph(text, true, &font, &opts));

        let mut words = tokenize(text);
        let expected = shown(&fill_lines(
            &mut words,
            10,
            usize::MAX,
            &style,
            &no_rubrics(),
        ));
        assert_eq!(got, expected);
    }

    #[test]
    fn merge_columns_yields_uniform_width() {
        let margin = vec![Line {
            shown: "ab".to_string(),
            len: 2,
        }];
        let body = vec![
            Line {
                shown: "hello".to_string(),
                len: 5,
            },
            Line {
                shown: "x".to_string(),
                len: 1,
            },
        ];
        let out = merge_columns(&margin, &body, 4, 6, " | ", 3);

        // One row per the taller of the two columns.
        assert_eq!(out.len(), 2);
        // Every merged line reports the same visible width.
        for line in &out {
            assert_eq!(line.len, 4 + 3 + 6);
        }
    }

    #[test]
    fn merge_columns_handles_empty_columns() {
        let out = merge_columns(&[], &[], 4, 6, " | ", 3);
        assert!(out.is_empty());
    }
}
