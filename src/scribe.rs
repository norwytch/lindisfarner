//! Scribal corruption — the copying errors of a tired monk.
//!
//! Real manuscripts are riddled with the mistakes scribes made transcribing
//! them: letters transposed, dropped, doubled, or misread. Turn this on and
//! lindisfarner introduces the same errata, deterministically (varied by the
//! seed). It meddles only with letters, so whitespace, punctuation, and line
//! count survive — the structure of a page (or a program) stays, but the words
//! (and identifiers, and keywords) are quietly broken.

/// A small deterministic hash (splitmix64), so a given seed always corrupts a
/// given text the same way.
fn mix(seed: u64, n: u64) -> u64 {
    let mut z = seed
        .wrapping_add(n.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(0x1234_5678);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Roughly one letter in this many is miscopied.
const RATE: u64 = 40;

/// Return `source` with scribal transcription errors introduced.
pub(crate) fn corrupt(source: &str, seed: u64) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Only letters are at risk; structure (spaces, newlines, braces) is left
        // untouched so the page keeps its shape.
        if c.is_alphabetic() && mix(seed, i as u64) % RATE == 0 {
            match mix(seed, (i as u64) ^ 0x5bd1_e995) % 4 {
                // Transposition: swap with the following letter.
                0 if i + 1 < chars.len() && chars[i + 1].is_alphabetic() => {
                    out.push(chars[i + 1]);
                    out.push(c);
                    i += 2;
                    continue;
                }
                // Omission: drop the letter entirely.
                1 => {}
                // Dittography: write it twice.
                2 => {
                    out.push(c);
                    out.push(c);
                }
                // Substitution (and the transposition fallthrough): misread it.
                _ => out.push(misread(c, seed, i)),
            }
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Misread a letter as another of the same case, a few places along.
fn misread(c: char, seed: u64, i: usize) -> char {
    let shift = (mix(seed, i as u64 + 7) % 5 + 1) as u8;
    if c.is_ascii_lowercase() {
        (b'a' + (c as u8 - b'a' + shift) % 26) as char
    } else if c.is_ascii_uppercase() {
        (b'A' + (c as u8 - b'A' + shift) % 26) as char
    } else {
        c // leave non-ASCII letters alone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A body of text long enough to be reliably corrupted at the given rate.
    const TEXT: &str =
        "fn main() {\n    let greeting = \"hello world\";\n    println!(\"{greeting}\");\n}\n";

    #[test]
    fn corruption_is_deterministic() {
        assert_eq!(corrupt(TEXT, 7), corrupt(TEXT, 7));
        assert_ne!(corrupt(TEXT, 1), corrupt(TEXT, 2));
    }

    #[test]
    fn corruption_actually_changes_the_text() {
        assert_ne!(corrupt(TEXT, 3), TEXT);
    }

    #[test]
    fn structure_survives_letters_break() {
        let out = corrupt(TEXT, 5);
        // Line count is preserved: only letters are touched, never newlines.
        assert_eq!(out.matches('\n').count(), TEXT.matches('\n').count());
        // Non-letter scaffolding (braces, punctuation) is preserved verbatim.
        for ch in ['{', '}', '(', ')', ';', '"'] {
            assert_eq!(
                out.matches(ch).count(),
                TEXT.matches(ch).count(),
                "punctuation {ch:?} should be untouched"
            );
        }
    }
}
