//! Developer aid: dump a few embedded blackletter glyphs to stdout so the
//! Fraktur font can be eyeballed without running the whole pipeline.
//! Run with `cargo run --example preview`.

use figlet_rs::FIGfont;
fn main() {
    let font = FIGfont::from_content(include_str!("../fonts/fraktur.flf")).unwrap();
    for s in ["A", "W", "I", "T", "M", "S"] {
        println!("=== {s} ===");
        match font.convert(s) {
            Some(f) => println!("{}", f),
            None => println!("(no glyph)"),
        }
    }
}
