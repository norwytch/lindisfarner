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
#[derive(Clone)]
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
    /// Pad inter-word gaps so each full line is flush on both margins.
    pub(crate) justify: bool,
    /// Break an over-long word with a trailing hyphen instead of a hard cut.
    pub(crate) hyphenate: bool,
    /// Fill the slack on a paragraph's final line with ❧ ornaments.
    pub(crate) fillers: bool,
}

/// How many words at the head of a paragraph to force into the rubric pigment,
/// independent of the `--rubricate` set — used for the incipit and to recover
/// the opening word whose first letter was lifted into a drop cap.
#[derive(Clone, Copy, PartialEq)]
enum Lead {
    None,
    Word, // just the first word
    Line, // the whole first line (an incipit)
}

/// The alphanumeric core of a word, lowercased — the key used for rubric lookup.
fn rubric_key(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Whether a word should be rubricated by the `--rubricate` set.
fn is_rubric(word: &str, rubrics: &HashSet<String>) -> bool {
    let key = rubric_key(word);
    !key.is_empty() && rubrics.contains(&key)
}

/// Colour a word: the pilcrow always takes the rubric pigment, and any other
/// word is reddened if its alphanumeric core appears in the rubric set.
fn colorize_word(word: &str, style: &Style, rubrics: &HashSet<String>) -> String {
    if word == "¶" {
        return style.pilcrow(word);
    }
    if is_rubric(word, rubrics) {
        style.rubric(word)
    } else {
        word.to_string()
    }
}

/// Greedily pack words into at most `max_lines` lines of `width` visible
/// columns. Over-long words are hard-broken (hyphenated when `opts.hyphenate`).
/// A full line is justified to both margins when `opts.justify` is set; the
/// paragraph's final line — on which the words run out — is left ragged. `lead`
/// forces the opening word or line into the rubric pigment. Consumed words are
/// removed from the deque; any remainder is left for the caller to lay out next.
fn fill_lines(
    words: &mut VecDeque<String>,
    width: usize,
    max_lines: usize,
    opts: &Options,
    lead: Lead,
) -> Vec<Line> {
    let width = width.max(1);
    let style = opts.style;
    let mut lines = Vec::new();

    while !words.is_empty() && lines.len() < max_lines {
        // The visible pieces of this line and the sum of their widths (without
        // the inter-word spaces — justification decides those).
        let mut parts: Vec<String> = Vec::new();
        let mut content_w = 0usize;
        let mut packed_w = 0usize; // running width with single spaces

        while let Some(front) = words.front() {
            let wlen = display_width(front);

            // A single word wider than the line: hard-break it.
            if wlen > width {
                if packed_w > 0 {
                    break; // finish the current line first
                }
                let word = words.pop_front().unwrap();
                let budget = if opts.hyphenate {
                    width.saturating_sub(1).max(1)
                } else {
                    width
                };
                let (mut head, mut head_w, tail) = split_at_width(&word, budget);
                if !tail.is_empty() {
                    if opts.hyphenate {
                        head.push('-');
                        head_w += 1;
                    }
                    words.push_front(tail);
                }
                parts.push(head);
                content_w += head_w;
                break; // a hard-broken fragment always ends its line
            }

            let needed = if packed_w == 0 { wlen } else { wlen + 1 };
            if packed_w + needed > width {
                break;
            }

            let word = words.pop_front().unwrap();
            // Force the lead word/line into red, except a pilcrow keeps its own
            // (possibly alternating) pigment.
            let force = word != "¶"
                && lines.is_empty()
                && match lead {
                    Lead::None => false,
                    Lead::Word => parts.is_empty(),
                    Lead::Line => true,
                };
            let shown = if force {
                style.rubric(&word)
            } else {
                colorize_word(&word, style, opts.rubrics)
            };
            parts.push(shown);
            content_w += wlen;
            packed_w += needed;
        }

        // Justify only genuinely full lines: if words remain, more text follows,
        // so this line is interior and may be stretched to the margin.
        let justify = opts.justify && !words.is_empty();
        let (shown, len) = assemble_line(&parts, content_w, width, justify);
        lines.push(Line::new(shown, len));
    }
    lines
}

/// Join the visible `parts` with spaces. With `justify`, distribute the slack
/// evenly across the gaps so the line fills `width`; otherwise use single
/// spaces. `content_w` is the summed visible width of the parts.
fn assemble_line(
    parts: &[String],
    content_w: usize,
    width: usize,
    justify: bool,
) -> (String, usize) {
    let n = parts.len();
    if n == 0 {
        return (String::new(), 0);
    }
    let packed = content_w + (n - 1);
    if !justify || n == 1 || packed >= width {
        return (parts.join(" "), packed);
    }
    let slack = width - content_w;
    let gaps = n - 1;
    let base = slack / gaps;
    let extra = slack % gaps; // the leftmost `extra` gaps get one more space
    let mut shown = String::new();
    for (i, part) in parts.iter().enumerate() {
        shown.push_str(part);
        if i < gaps {
            shown.push_str(&" ".repeat(base + usize::from(i < extra)));
        }
    }
    (shown, width)
}

/// Fill the slack on a finished line with spaced ❧ ornaments (a line filler), as
/// scribes did to keep a short closing line from looking unfinished.
fn fill_slack(line: &mut Line, width: usize, style: &Style) {
    if line.len + 2 > width {
        return;
    }
    let mut deco = String::from(" ");
    let mut w = line.len + 1;
    while w < width {
        deco.push('❧');
        w += 1;
        if w < width {
            deco.push(' ');
            w += 1;
        }
    }
    line.shown.push_str(&style.border(&deco));
    line.len = width;
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

/// Lay one tall column of `content` into `columns` side-by-side columns of
/// `col_w` each, balanced by height, with `gutter` blank columns between them —
/// the two-column setting of a real codex. All combined lines share one width.
pub(crate) fn lay_in_columns(
    content: &[Line],
    columns: usize,
    col_w: usize,
    gutter: usize,
) -> Vec<Line> {
    let columns = columns.max(1);
    let height = content.len().div_ceil(columns);
    let total = columns * col_w + columns.saturating_sub(1) * gutter;
    let gap = " ".repeat(gutter);
    let mut out = Vec::with_capacity(height);
    for row in 0..height {
        let mut shown = String::new();
        for c in 0..columns {
            if c > 0 {
                shown.push_str(&gap);
            }
            match content.get(c * height + row) {
                Some(line) => {
                    shown.push_str(&line.shown);
                    shown.push_str(&" ".repeat(col_w.saturating_sub(line.len)));
                }
                None => shown.push_str(&" ".repeat(col_w)),
            }
        }
        out.push(Line { shown, len: total });
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
    incipit: bool,
    font: &FIGfont,
    opts: &Options,
) -> Vec<Line> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let line_lead = if incipit { Lead::Line } else { Lead::None };

    let mut out = if !drop_cap {
        let mut words = tokenize(text);
        fill_lines(&mut words, opts.width, usize::MAX, opts, line_lead)
    } else {
        illuminate_with_drop_cap(text, incipit, font, opts)
    };

    // A line filler keeps the closing line from trailing off into blank space.
    if opts.fillers {
        if let Some(last) = out.last_mut() {
            fill_slack(last, opts.width, opts.style);
        }
    }
    out
}

/// The drop-cap composition: a large initial with the opening lines set down its
/// right-hand side. The caller has already trimmed `text` and ruled out empty.
fn illuminate_with_drop_cap(
    text: &str,
    incipit: bool,
    font: &FIGfont,
    opts: &Options,
) -> Vec<Line> {
    let mut chars = text.chars();
    let initial = chars.next().unwrap();
    let rest: String = chars.collect();

    let glyph = render_glyph(initial, font);
    let height = glyph.len();
    let glyph_w = glyph.iter().map(|l| display_width(l)).max().unwrap_or(0);
    let narrow = opts.width.saturating_sub(glyph_w + opts.gap);

    // Too little room for a drop cap: fall back to an ordinary paragraph.
    if narrow < 8 {
        let lead = if incipit { Lead::Line } else { Lead::None };
        let mut words = tokenize(text);
        return fill_lines(&mut words, opts.width, usize::MAX, opts, lead);
    }

    // If the opening word was a rubric target, recover it: its first letter has
    // become the initial, so rubricate the stem that flows beside the cap.
    let first_word = text.split_whitespace().next().unwrap_or("");
    let lead = if incipit {
        Lead::Line
    } else if is_rubric(first_word, opts.rubrics) {
        Lead::Word
    } else {
        Lead::None
    };

    let mut words = tokenize(&rest);
    let beside = fill_lines(&mut words, narrow, height, opts, lead);
    let below = fill_lines(&mut words, opts.width, usize::MAX, opts, Lead::None);

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

    /// Plain options with every adornment off, borrowing `style`/`rubrics`.
    fn opts<'a>(width: usize, style: &'a Style, rubrics: &'a HashSet<String>) -> Options<'a> {
        Options {
            width,
            gap: 1,
            style,
            rubrics,
            justify: false,
            hyphenate: false,
            fillers: false,
        }
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
        let rub = no_rubrics();
        let mut words = tokenize("the quick brown fox");
        let lines = fill_lines(
            &mut words,
            9,
            usize::MAX,
            &opts(9, &style, &rub),
            Lead::None,
        );
        assert_eq!(shown(&lines), vec!["the quick", "brown fox"]);
        assert!(lines.iter().all(|l| l.len <= 9));
        assert!(words.is_empty());
    }

    #[test]
    fn fill_lines_honours_max_lines_and_leaves_a_remainder() {
        let style = Style::new(false, Theme::Gold);
        let rub = no_rubrics();
        let mut words = tokenize("alpha beta gamma delta");
        let lines = fill_lines(&mut words, 5, 1, &opts(5, &style, &rub), Lead::None);
        assert_eq!(shown(&lines), vec!["alpha"]);
        // The unconsumed words remain at the front of the deque.
        assert_eq!(words.front().map(String::as_str), Some("beta"));
    }

    #[test]
    fn fill_lines_hard_breaks_an_over_long_word() {
        let style = Style::new(false, Theme::Gold);
        let rub = no_rubrics();
        let mut words = tokenize("supercalifragilistic");
        let lines = fill_lines(
            &mut words,
            8,
            usize::MAX,
            &opts(8, &style, &rub),
            Lead::None,
        );
        assert!(lines.iter().all(|l| l.len <= 8));
        // The pieces reassemble into the original word.
        let joined: String = lines.iter().map(|l| l.shown.as_str()).collect();
        assert_eq!(joined, "supercalifragilistic");
    }

    #[test]
    fn fill_lines_hyphenates_a_hard_break_when_asked() {
        let style = Style::new(false, Theme::Gold);
        let rub = no_rubrics();
        let mut o = opts(8, &style, &rub);
        o.hyphenate = true;
        let mut words = tokenize("supercalifragilistic");
        let lines = fill_lines(&mut words, 8, usize::MAX, &o, Lead::None);
        assert!(lines.iter().all(|l| l.len <= 8));
        // Every broken line but the last ends in a hyphen.
        for l in &lines[..lines.len() - 1] {
            assert!(l.shown.ends_with('-'), "expected a hyphen on {:?}", l.shown);
        }
        // Removing the hyphens reassembles the original word.
        let joined: String = lines
            .iter()
            .map(|l| l.shown.trim_end_matches('-'))
            .collect();
        assert_eq!(joined, "supercalifragilistic");
    }

    #[test]
    fn fill_lines_justifies_interior_lines() {
        let style = Style::new(false, Theme::Gold);
        let rub = no_rubrics();
        let mut o = opts(20, &style, &rub);
        o.justify = true;
        let mut words = tokenize("the quick brown fox jumps over");
        let lines = fill_lines(&mut words, 20, usize::MAX, &o, Lead::None);
        // Interior lines are stretched to exactly the column width...
        assert_eq!(lines[0].len, 20);
        assert!(lines[0].shown.contains("  "), "expected padded gaps");
        // ...while the final (ragged) line keeps single spaces.
        let last = lines.last().unwrap();
        assert!(last.len <= 20);
        assert!(!last.shown.contains("  "));
    }

    #[test]
    fn fill_lines_measures_wide_characters_for_layout() {
        let style = Style::new(false, Theme::Gold);
        let rub = no_rubrics();
        // Three two-column glyphs; at width 5 only "私 は" (2+1+2) fits a line.
        let mut words = tokenize("私 は 猫");
        let lines = fill_lines(
            &mut words,
            5,
            usize::MAX,
            &opts(5, &style, &rub),
            Lead::None,
        );
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
        let lines = fill_lines(
            &mut words,
            40,
            usize::MAX,
            &opts(40, &style, &rubrics),
            Lead::None,
        );
        // The rubric escape is woven into the visible string...
        assert!(lines[0].shown.contains('\u{1b}'));
        // ...but the tracked width still counts only printable columns.
        assert_eq!(lines[0].len, "the gold leaf".len());
    }

    #[test]
    fn drop_cap_rows_are_exactly_the_page_width() {
        let font = FIGfont::standard().unwrap();
        let style = Style::new(false, Theme::Mono);
        let rub = no_rubrics();
        let lines = illuminate_paragraph(
            "Hello world this is a reasonably long paragraph for the drop cap",
            true,
            false,
            &font,
            &opts(40, &style, &rub),
        );
        assert!(!lines.is_empty());
        // The tall initial occupies the first rows, each padded to full width.
        assert_eq!(lines[0].len, 40);
    }

    #[test]
    fn tiny_width_falls_back_to_a_plain_paragraph() {
        let font = FIGfont::standard().unwrap();
        let style = Style::new(false, Theme::Mono);
        let rub = no_rubrics();
        let text = "Hello world foo bar baz";
        // Width 10 is too narrow for a drop cap beside the glyph.
        let got = shown(&illuminate_paragraph(
            text,
            true,
            false,
            &font,
            &opts(10, &style, &rub),
        ));

        let mut words = tokenize(text);
        let expected = shown(&fill_lines(
            &mut words,
            10,
            usize::MAX,
            &opts(10, &style, &rub),
            Lead::None,
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

    use proptest::prelude::*;

    proptest! {
        /// No wrapped line ever exceeds the column width, and the words come back
        /// out in order. Tokens are kept shorter than the minimum width so no
        /// hard-break splits a word (that path is covered by its own test).
        #[test]
        fn wrapping_respects_width_and_preserves_words(
            text in "[a-z]{1,6}( [a-z]{1,6}){0,40}",
            width in 8usize..40,
        ) {
            let style = Style::new(false, Theme::Mono);
            let rub = no_rubrics();
            let mut words = tokenize(&text);
            let original: Vec<String> = words.iter().cloned().collect();

            let lines = fill_lines(&mut words, width, usize::MAX, &opts(width, &style, &rub), Lead::None);

            for line in &lines {
                prop_assert!(line.len <= width, "line {:?} wider than {width}", line.shown);
            }
            let roundtrip: Vec<String> = lines
                .iter()
                .flat_map(|l| l.shown.split_whitespace().map(str::to_string))
                .collect();
            prop_assert_eq!(roundtrip, original);
        }

        /// With justification on, every line but the last fills the width
        /// exactly. Tokens are kept short enough (≤4 with width ≥12) that no
        /// interior line is reduced to a single, unstretchable word.
        #[test]
        fn justified_interior_lines_are_flush(
            text in "[a-z]{1,4}( [a-z]{1,4}){8,40}",
            width in 12usize..40,
        ) {
            let style = Style::new(false, Theme::Mono);
            let rub = no_rubrics();
            let mut o = opts(width, &style, &rub);
            o.justify = true;
            let mut words = tokenize(&text);
            let lines = fill_lines(&mut words, width, usize::MAX, &o, Lead::None);
            prop_assume!(lines.len() >= 2);
            for line in &lines[..lines.len() - 1] {
                prop_assert_eq!(line.len, width);
            }
        }
    }
}
