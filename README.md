# lindisfarner

[![CI](https://github.com/norwytch/lindisfarner/actions/workflows/ci.yml/badge.svg)](https://github.com/norwytch/lindisfarner/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Rust CLI tool that illuminates text files with ASCII art. 

```
╭───────────────────────────────❦──────────────────────────────╮
│     ...     ..      ..     r Leopold Bloom ate with relish   │
│   x*8888x.:*8888: -"888:   the inner organs of beasts and    │
│  X   48888X `8888H  8888   fowls. He liked thick giblet      │
│ X8x.  8888X  8888X  !888>  soup, nutty gizzards, a stuffed   │
│ X8888 X8888  88888   "*8%- roast heart, liver slices fried   │
│ '*888!X8888> X8888  xH8>   with crustcrumbs, fried hencod's  │
│   `?8 `8888  X888X X888>   roes. Most of all he liked        │
│   -^  '888"  X888  8888>   grilled mutton kidneys which gave │
│    dx '88~x. !88~  8888>   to his palate a fine tang of      │
│  .8888Xf.888x:!    X888X.: faintly scented urine.            │
│ :""888":~"888"     `888*"                                    │
│     "~'    "~        ""                                      │
│                                                              │
│ Kidneys were in his mind as he moved about the kitchen       │
│ softly, righting her breakfast things on the humpy tray.     │
│ Gelid light and air were in the kitchen but out of doors     │
│ gentle summer morning everywhere. Made him feel a bit        │
│ peckish.                                                     │
│                                                              │
│ The coals were reddening.                                    │
│                                                              │
│ Another slice of bread and butter: three, four: right. She   │
│ didn't like her plate full. Right. He turned from the tray,  │
│ lifted the kettle off the hob and set it sideways on the     │
│ fire. It sat there, dull and squat, its spout stuck out. Cup │
│ of tea soon. Good. Mouth dry. The cat walked stiffly round a │
│ leg of the table with tail on high.                          │
╰───────────────────────────────❦──────────────────────────────╯
```

## The craft, mapped to features

| Manuscript element | Lindisfarner adaptation |
|---|---|
| Illuminated initial / versal | A large FIGlet drop-cap; the opening lines flow down its side |
| Rubrication (red ink for key words) | `--rubricate word,word` highlights matching words |
| Gold leaf / chrysography | The initial, painted in the theme's accent colour |
| Decorative border & marginalia | `--border` styles, with an ❦ flourish on the ornate frame |
| Drolleries (marginal doodles) | `--drolleries` sets a small ASCII figure beside each paragraph |
| The page itself | A wrapped, framed text block at a chosen `--width` |

## Build & run

```sh
cargo build --release
./target/release/lindisfarner sample.txt
```

Reads a file or standard input; writes to stdout or `--output`.

## Options

```
lindisfarner [OPTIONS] [FILE]

  -w, --width <N>        Body text width            [default: 60]
  -b, --border <STYLE>   none | simple | double | ornate   [default: ornate]
  -t, --theme <THEME>    gold | crimson | mono       [default: gold]
  -c, --color <WHEN>     auto | always | never       [default: auto]
  -d, --drop-cap <WHICH> first | all | none          [default: first]
  -f, --font <FONT>      blackletter | standard      [default: blackletter]
  -r, --rubricate <W,..> words to highlight in the rubric colour
      --drolleries       adorn the left margin with ASCII marginal figures
      --seed <N>         vary which drolleries appear   [default: 0]
  -p, --pilcrows         mark paragraph breaks with ¶ instead of a blank line
  -o, --output <FILE>    write to a file instead of stdout
```

Colour is automatic: it turns on for a terminal and off when piped or written
to a file, so saved pages stay clean plain text.

## Notes & limitations

- **Line endings** are normalised, so `\r\n` (Windows) and `\r` (classic Mac)
  files split into paragraphs the same way as Unix text.
- **Minimum width.** `--width` is clamped to a floor of 24 columns so the page
  stays readable.
- **The drop-cap letter can't be rubricated.** The opening letter is lifted out
  to become the large initial, so the first word of a drop-capped paragraph
  won't match a `--rubricate` term.

## Examples

```sh
# Every paragraph gets its own initial, in a double frame
lindisfarner sample.txt --drop-cap all --border double

# Crimson theme, rubricate a few words, force colour through a pipe
lindisfarner sample.txt -t crimson -r gold,vellum,scriptorium -c always | less -R

# Save a plain (no-colour) illuminated page
lindisfarner sample.txt -o page.txt

# Drolleries in the margin, reshuffled with a seed
lindisfarner sample.txt --drolleries --seed 3

# Run paragraphs together, marking each break with a red pilcrow
lindisfarner sample.txt --pilcrows --drop-cap none
```

## How it fits together

- `src/main.rs` — CLI parsing (clap) and orchestration.
- `src/illuminate.rs` — word-wrapping and the drop-cap composition (the core).
- `src/border.rs` — the frame and its flourishes.
- `src/style.rs` — the colour palette / themes.

## Typeface

The illuminated initials default to a Fraktur capital letter, which imitates the traditional blackletter book hand used in English monasteries. The font is embedded in the binary (`fonts/fraktur.flf`), so nothing extra is needed at runtime. Use
`--font standard` for plain FIGlet block capitals instead. We encourage users to add their own fonts to experiment with different traditions of illumination. 

Credits: the blackletter glyphs come from the FIGlet font *Fraktur.flf* by
Philip Menke (1995), part of the freely distributable FIGlet font collection.
The default block font is the standard FIGlet font (Glenn Chappell & Ian Chai).

## Drolleries

With `--drolleries`, a small ASCII figure is set in the left margin beside each
paragraph, separated from the text by a ruled line. These imitate the original drolleries found in the margins of illuminated manuscripts, which most often depicted human-animal hybrid figures that reflected the wild imagination of the medieval monastic. The figures come from a fixed built-in repertoire (`src/drollery.rs`): a hare, cat, owl, fish, mouse, snail, bird, and a vine flourish. Selection is deterministic, so a given file always renders the same; pass `--seed N` to reshuffle which figure lands beside which paragraph. We encourage users to add their own drolleries by simply adding to `drollery.rs`. 

## Ideas to extend

- **Right / outer margin**: a `--margin left|right` switch (the merge already
  supports either side — just swap the column order).
- **Glosses & manicules**: a second margin channel for user notes or a `☞` set
  beside lines containing rubricated words.
- **Line fillers**: pad short final lines with `❧` or `✦` ornaments.