//! Drolleries — the whimsical figures scribes drew in the margins.
//!
//! A fixed repertoire of small ASCII creatures and flourishes. Selection is
//! deterministic (seeded), so the same input always produces the same page;
//! changing `--seed` reshuffles which figures appear down the margin.

/// The repertoire. Each figure is a slice of rows; rows may differ in width and
/// are left-aligned within the margin column when rendered.
const DROLLERIES: &[&[&str]] = &[
    // hare
    &[" (\\_/)", "(='.'=)", "(\")_(\")"],
    // cat
    &[" /\\_/\\", "( o.o )", " > ^ <"],
    // owl
    &["{O,o}", "|)``)", "-\"-\"-"],
    // fish
    &["><(((°>"],
    // mouse
    &["<:3 )~~"],
    // snail
    &["  _", " (@)__", "_(___)>"],
    // bird
    &["(o>", "//\\", "V_/_"],
    // vine flourish
    &[" ❧", " |", "~|~", " |"],
];

/// Number of figures in the repertoire.
fn count() -> usize {
    DROLLERIES.len()
}

/// Width of the widest row across every figure — the margin column width.
pub(crate) fn max_width() -> usize {
    DROLLERIES
        .iter()
        .flat_map(|d| d.iter())
        .map(|l| crate::illuminate::display_width(l))
        .max()
        .unwrap_or(0)
}

/// A small deterministic hash (splitmix64) used to pick a figure from a seed
/// and a paragraph index, so output is reproducible but varied.
fn splitmix(seed: u64, n: u64) -> u64 {
    let mut z = seed.wrapping_add(n.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Pick a figure for the `n`-th slot down the margin, as owned rows.
pub(crate) fn pick(seed: u64, n: u64) -> Vec<String> {
    let i = (splitmix(seed, n) % count() as u64) as usize;
    DROLLERIES[i].iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repertoire_is_non_empty() {
        assert!(count() > 0);
        assert!(max_width() > 0);
    }

    #[test]
    fn pick_is_deterministic() {
        assert_eq!(pick(0, 0), pick(0, 0));
        assert_eq!(pick(42, 7), pick(42, 7));
    }

    #[test]
    fn pick_always_returns_a_figure() {
        for n in 0..100u64 {
            assert!(!pick(3, n).is_empty());
        }
    }
}
