//! The frame around the illuminated page.
//!
//! Borders are built from Unicode box-drawing characters. The "ornate" style
//! sets a small floral flourish (❦) at the centre of the top and bottom rules,
//! echoing the marginal decoration of a real manuscript.

use crate::illuminate::Line;
use crate::style::Style;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Border {
    None,
    Simple,
    Double,
    Ornate,
}

struct Glyphs {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
}

impl Border {
    fn glyphs(self) -> Glyphs {
        match self {
            Border::Simple => Glyphs {
                tl: '┌',
                tr: '┐',
                bl: '└',
                br: '┘',
                h: '─',
                v: '│',
            },
            Border::Double => Glyphs {
                tl: '╔',
                tr: '╗',
                bl: '╚',
                br: '╝',
                h: '═',
                v: '║',
            },
            // Rounded corners for the ornate style; a flourish is added below.
            Border::Ornate => Glyphs {
                tl: '╭',
                tr: '╮',
                bl: '╰',
                br: '╯',
                h: '─',
                v: '│',
            },
            Border::None => Glyphs {
                tl: ' ',
                tr: ' ',
                bl: ' ',
                br: ' ',
                h: ' ',
                v: ' ',
            },
        }
    }
}

/// Pad a (possibly colour-coded) line out to `width` *visible* columns.
fn pad(line: &Line, width: usize) -> String {
    let mut s = line.shown.clone();
    if line.len < width {
        s.push_str(&" ".repeat(width - line.len));
    }
    s
}

/// Wrap the content lines in the chosen border and return the final rows.
pub(crate) fn render(lines: &[Line], width: usize, border: Border, style: &Style) -> Vec<String> {
    if matches!(border, Border::None) {
        return lines.iter().map(|l| pad(l, width)).collect();
    }

    let g = border.glyphs();
    let inner = width + 2; // one space of padding on each side of the text

    let mut rule: Vec<char> = std::iter::repeat_n(g.h, inner).collect();
    if matches!(border, Border::Ornate) && inner >= 3 {
        rule[inner / 2] = '❦';
    }
    let rule: String = rule.into_iter().collect();

    let top = format!("{}{}{}", g.tl, rule, g.tr);
    let bottom = format!("{}{}{}", g.bl, rule, g.br);

    let mut out = Vec::with_capacity(lines.len() + 2);
    out.push(style.border(&top));
    let bar = style.border(&g.v.to_string());
    for line in lines {
        let body = pad(line, width);
        out.push(format!("{bar} {body} {bar}"));
    }
    out.push(style.border(&bottom));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Theme;

    fn plain() -> Style {
        Style::new(false, Theme::Mono)
    }

    fn line(s: &str) -> Line {
        Line {
            shown: s.to_string(),
            len: s.chars().count(),
        }
    }

    #[test]
    fn none_border_pads_without_a_frame() {
        let out = render(&[line("hi")], 5, Border::None, &plain());
        assert_eq!(out, vec!["hi   "]);
    }

    #[test]
    fn simple_border_frames_every_line() {
        let out = render(&[line("hi")], 5, Border::Simple, &plain());
        // top + one body row + bottom
        assert_eq!(out.len(), 3);
        assert!(out[0].starts_with('┌') && out[0].ends_with('┐'));
        assert!(out[1].starts_with('│') && out[1].ends_with('│'));
        assert!(out[2].starts_with('└') && out[2].ends_with('┘'));
    }

    #[test]
    fn ornate_border_carries_a_flourish() {
        let out = render(&[line("hi")], 5, Border::Ornate, &plain());
        assert!(out[0].contains('❦'));
        assert!(out[out.len() - 1].contains('❦'));
    }
}
