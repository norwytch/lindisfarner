# lindisfarner

[![CI](https://github.com/norwytch/lindisfarner/actions/workflows/ci.yml/badge.svg)](https://github.com/norwytch/lindisfarner/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/lindisfarner.svg)](https://crates.io/crates/lindisfarner)
[![docs.rs](https://docs.rs/lindisfarner/badge.svg)](https://docs.rs/lindisfarner)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Rust CLI tool that illuminates text and source code with ASCII art. Also has the option of vandalizing or entirely breaking code that calls or implements neural models.

<p align="center">
  <img src="assets/banner.svg" alt="sample.txt rendered by lindisfarner with every manuscript element: a gold illuminated drop-cap, a rubricated incipit and words, a justified two-column codex, ASCII drolleries down the margin, alternating red/blue ¶ pilcrows, ❧ line fillers, and the ornate ❦ border" width="900">
</p>

## What it does

lindisfarner can:

- **Illuminate prose** — wrap plain text into a framed manuscript page: a FIGlet
  drop-cap, rubricated words, marginal drolleries, pilcrows, a two-column codex.
- **Illuminate & search code** — render source with keywords rubricated and
  comments lifted into the margin as glosses; or point a [Semgrep] pattern or
  rules at a codebase and illuminate the findings.
- **Magnificate your code base** — search a code base to identify use of neural
  models, and either annotate the code with quotes from Magnifica Humanitas or
  break the code outright. This mode writes directly to disk and does not check
  whether you have committed anything to git. **Do not use it on code you rely
  on, IT WILL BREAK IT!**

## Install

```sh
cargo install lindisfarner
```

Or build from source:

```sh
cargo build --release
./target/release/lindisfarner sample.txt
```

The code-search modes (`--find`, `--scan`, `--magnifica`) also need
[Semgrep] on your `PATH`:

```sh
python3 -m venv .venv && source .venv/bin/activate
pip install semgrep
```

## Quickstart

lindisfarner reads a file (or standard input) and writes the illuminated page to
the terminal — or to a file with `--output`.

```sh
# Illuminate a text file (the page fills your terminal width)
lindisfarner sample.txt

# From a pipe; force colour through it, then page with less -R
cat sample.txt | lindisfarner -c always | less -R

# Save a plain, colourless page to a file
lindisfarner sample.txt -o page.txt

# See every option
lindisfarner --help
```

Colour is automatic: on for a terminal, off when piped or written to a file, so
saved pages stay clean plain text. With no `--width`, the page fills the terminal
(falling back to 60 columns when piped). If you built from source, the binary is
at `./target/release/lindisfarner`; `cargo install` puts it on your `PATH`.

## Illuminating prose

| Manuscript element | lindisfarner adaptation |
|---|---|
| Illuminated initial / versal | A large FIGlet drop-cap; the opening lines flow down its side |
| Rubrication (red ink for key words) | `--rubricate word,word` highlights matching words |
| Gold leaf / chrysography | The initial, painted in the theme's accent colour |
| Decorative border & marginalia | `--border` styles, with an ❦ flourish on the ornate frame |
| Drolleries (marginal doodles) | `--drolleries` scatters small ASCII figures down the margin |
| Pilcrows | `--pilcrows` runs paragraphs together, marked by an inline ¶ |
| Two-column codex | `--columns 2` sets the body in side-by-side columns |
| The page itself | A wrapped, framed text block at a chosen `--width` |

```sh
# Every paragraph gets its own initial, in a double frame
lindisfarner sample.txt --drop-cap all --border double

# Crimson theme, rubricate a few words, force colour through a pipe
lindisfarner sample.txt -t crimson -r gold,vellum,scriptorium -c always | less -R

# A justified two-column codex page with a rubricated incipit
lindisfarner sample.txt --columns 2 --justify --incipit -w 90

# Run paragraphs together with red/blue alternating pilcrows
lindisfarner sample.txt --pilcrows -c always | less -R
```

### Drolleries

With `--drolleries`, small ASCII figures are scattered down the left margin,
separated from the text by a ruled line. They are placed at fixed intervals
independent of the paragraph structure, so the margin fills with figures whether
the text is one flowing block or many paragraphs. These imitate the original
drolleries found in the margins of illuminated manuscripts, which most often
depicted human-animal hybrid figures that reflected the wild imagination of
medieval monastics. The figures come from a fixed built-in repertoire
(`src/drollery.rs`): a hare, cat, owl, fish, mouse, snail, bird, and a vine
flourish. Selection is deterministic, so a given file always renders the same;
pass `--seed N` to reshuffle the figures. Add your own by editing `drollery.rs`.

### Typeface

The illuminated initials default to a **Fraktur** capital. The font is embedded in the binary
(`fonts/fraktur.flf`), so nothing extra is needed at runtime. Use
`--font standard` for plain FIGlet block capitals instead.

Credits: the blackletter glyphs come from the FIGlet font *Fraktur.flf* by
Philip Menke (1995), part of the freely distributable FIGlet font collection.
The default block font is the standard FIGlet font (Glenn Chappell & Ian Chai).

### Notes & limitations

- **Line endings** are normalised, so `\r\n` (Windows) and `\r` (classic Mac)
  files split into paragraphs the same way as Unix text.
- **Minimum width.** `--width` is clamped to a floor of 24 columns so the page
  stays readable.
- **Pilcrows vs. columns vs. drolleries** compose: pilcrows flow the text into
  one block, `--columns` sets that block in a codex, and drolleries scatter down
  the outer margin independent of the paragraph count.

## Illuminating code

Point lindisfarner at a source file and it switches to **code mode**: lines are
kept verbatim (indentation and all), the language's **keywords are rubricated**
in red, and **comments are lifted out into the margin as glosses** — the way a
scribe set commentary beside scripture.

```sh
lindisfarner src/main.rs            # auto-detected from the .rs extension
lindisfarner --code --language go < snippet.txt
lindisfarner notes.py --prose       # force prose on a code file instead
```

```
╭─────────────────────────────❦─────────────────────────────╮
│                                 ┊ the program entry point │
│ fn main() {                     ┊                         │
│     let greeting = "hi";        ┊ a friendly word         │
│     for i in 0..3 {             ┊ loop a few times        │
│         println!("{greeting}"); ┊                         │
│     }                           ┊                         │
│ }                               ┊                         │
╰─────────────────────────────❦─────────────────────────────╯
```

Languages are matched by extension (Rust, Python, JavaScript/TypeScript, C/C++,
Go, shell), with a generic fallback for anything else. Detection is keyword- and
comment-based rather than a full parser, so keywords inside string literals may
also be reddened — light vandalism, as intended. `--drolleries`, `--border`, and
`--theme` all still apply.

### Scribal errors

Real manuscripts are full of copying mistakes. `--corrupt` introduces the same
errata (letters transposed, dropped, doubled, or misread), purposefully
breaking the text (and, in code mode, the code):

```sh
lindisfarner src/main.rs --corrupt --seed 3
```

Only letters are touched, so whitespace, punctuation, and line count survive: the
page keeps its shape while the words quietly fall apart. The errors are
deterministic — change `--seed` for a different careless monk.

## Searching code

The `--find` and `--scan` modes use [Semgrep] (see [Install](#install)) to locate
code and illuminate the matches as a commentary page: each match set in code
style (keywords rubricated), with a gloss in the margin.

Point `--find` at a file or directory with a [Semgrep pattern][patterns]:

```sh
lindisfarner src/ --find '$X.unwrap()' --language rust
```

```
╭─────────────────────────────────────────────────❦────────────────────────────────────────────────╮
│ let (code, gloss) = g.split("let x = 5; // a number", by_name("rust").unwrap(… ┊ src/code.rs:345 │
│ let rust = by_name("rust").unwrap();                                           ┊ src/code.rs:352 │
│ let out = rubricate("fn main() {", by_name("rust").unwrap(), &style);          ┊ src/code.rs:370 │
│ let (body, w) = illuminate(src, by_name("rust").unwrap(), &style, 80);         ┊ src/code.rs:379 │
╰─────────────────────────────────────────────────❦────────────────────────────────────────────────╯
```

The language is taken from `--language` or detected from the path. The same
glossed-page rendering is available to library users as `render_glossed`.

### Rules with `--scan`

For more than a single pattern, point `--scan` at a [Semgrep rule config][rules]
— a file or a directory of rules. The rules decide *what to find* and *what to
say*; each finding is glossed with its rule message, marked by severity (`†`
ERROR, `☞` WARNING, `❧` INFO):

```yaml
# rules.yml
rules:
  - id: no-unwrap
    languages: [rust]
    severity: WARNING
    message: a careless unwrap() — handle the error
    pattern: $X.unwrap()
```

```sh
lindisfarner src/ --scan rules.yml
```

```
╭───────────────────────────❦───────────────────────────╮
│ x.unwrap() ┊ ☞ a careless unwrap() — handle the error │
│ y.unwrap() ┊ ☞ a careless unwrap() — handle the error │
╰───────────────────────────❦───────────────────────────╯
```

`--scan` accepts a single rule file **or a directory** of them, in which case
every rule runs together. This repo ships a starter library in [`rules/`](rules/)
— scan the whole codebase against all of them with `lindisfarner src/ --scan
rules`. Add your own by dropping more `.yml` files in there; each is an ordinary
[Semgrep rule][rules]. Because it's Semgrep underneath, you can also point
`--scan` at its registry (e.g. `--scan p/python`).

[Semgrep]: https://semgrep.dev
[patterns]: https://semgrep.dev/docs/writing-rules/pattern-syntax
[rules]: https://semgrep.dev/docs/writing-rules/rule-syntax

## The magnifica modes (an art project)

Point lindisfarner at a codebase and it finds where **AI is used** and answers
with the words of the encyclical *Magnifica Humanitas* of Pope Leo XIV, "On
Safeguarding the Human Person in the Time of Artificial Intelligence" — supplied
as a PDF in [`assets/`](assets/), with the passages curated into
[`assets/quotes.txt`](assets/quotes.txt) (override with `--quotes`).

"AI is used" means three things:

- **Hosted AI APIs** ([`rules/ai.yml`](rules/ai.yml)) — the major SDKs and agent
  frameworks across Python, JS/TS, and Go, plus raw API calls by hostname.
- **The ML lifecycle** ([`rules/ml.yml`](rules/ml.yml)) — training your *own*
  neural models: framework imports, data prep, loading, training, fine-tuning,
  evaluation, and deployment.
- **Model weights** — the binary artifacts (`.safetensors`, `.pt`, `.onnx`, …)
  found by walking the filesystem, since Semgrep reads only source code.

Two modes — **both write the encyclical into the files on disk**, then print an
illuminated report of what was changed. Model weight files are reported but never
modified. This mode does NOT check to see if you have committed anything to your git working tree. Engage with caution!

- **`witness`** — insert the encyclical as a comment beside every AI invocation,
  leaving the code that runs intact.

  ```sh
  lindisfarner path/to/codebase --magnifica witness
  ```

  ```py
  # In one sense, technological innovation can represent human
  # participation in the divine act of creation. — Magnifica Humanitas §111
  c.messages.create(model="claude")
  ```

  The report then names each annotated location and reads the encyclical beneath.

- **`relinquish`** — strike each AI block out of the source, leaving the
  encyclical's words (as comments) in its place — breaking what it touches.

  ```py
  # In one sense, technological innovation can represent human
  # participation in the divine act of creation. — Magnifica Humanitas §111
  # (an AI invocation, relinquished)
  ```

## Options

```
lindisfarner [OPTIONS] [FILE]

Page & layout:
  -w, --width <N>        Body text width             [default: terminal width]
  -b, --border <STYLE>   none | simple | double | ornate   [default: ornate]
  -d, --drop-cap <WHICH> first | all | none          [default: first]
      --columns <N>      set the text in N columns, codex-style   [default: 1]
  -j, --justify          set the body flush to both margins
      --hyphenate        break over-long words with a trailing hyphen
      --incipit          rubricate the opening line
      --fillers          fill short closing lines with ❧ ornaments
  -p, --pilcrows         run paragraphs together, separated by an inline ¶
      --drolleries       adorn the left margin with ASCII marginal figures
      --seed <N>         vary which drolleries / scribal errors appear  [0]

Colour & type:
  -c, --color <WHEN>     auto | always | never       [default: auto]
  -t, --theme <THEME>    gold | crimson | mono       [default: gold]
  -f, --font <FONT>      blackletter | standard      [default: blackletter]
  -r, --rubricate <W,..> words to highlight in the rubric colour

Code:
      --code             illuminate the input as source code
      --prose            force prose mode even for a recognised code file
      --language <LANG>  override the code-mode language (rust, python, c, …)
      --corrupt          introduce scribal errors that break the text (and code)

Search & magnifica (need Semgrep):
      --find <PATTERN>   find code by a Semgrep pattern and gloss the matches
      --scan <RULES>     scan with a Semgrep rule config (file or directory)
      --magnifica <M>    write the encyclical into AI-using code on disk
                           (witness | relinquish)
      --quotes <FILE>    passages file for the magnifica modes

Output & misc:
  -o, --output <FILE>    write to a file instead of stdout
      --completions <SHELL>  print a shell completion script and exit
      --man              print a roff man page and exit
```

## Use as a library

lindisfarner is also a library crate. The CLI's dependencies (clap, Semgrep
glue, terminal sizing) sit behind the default `cli` feature, so a library user
can opt out of them:

```sh
cargo add lindisfarner --no-default-features
```

Build a `Config` and call `render`:

```rust
use lindisfarner::{render, Config, Theme};

let cfg = Config {
    theme: Theme::Crimson,
    colored: true,
    justify: true,
    ..Config::default()
};
let page = render("Within the quiet scriptorium…", &cfg);
print!("{page}");
```

The public surface is small:

- **`render(source, &Config) -> String`** — illuminate prose, or source code
  with `Config { code: true, .. }`.
- **`render_glossed(rows, &Config) -> String`** — render explicit
  `(code, gloss)` rows as a commentary page, each gloss set in the margin.
- **`detect_language(filename) -> Option<String>`** — the canonical language
  name for a path's extension, for auto-enabling code mode.
- **`Config`** and its enums **`Border`**, **`Theme`**, **`DropCap`**,
  **`Font`**, plus the **`MIN_WIDTH`** constant.

The library carries no Semgrep dependency — the `--find`, `--scan`, and
`--magnifica` features live in the CLI binary only. Full API docs are on
[docs.rs](https://docs.rs/lindisfarner).

## How it fits together

- `src/lib.rs` — the public API: `Config` and `render`.
- `src/illuminate.rs` — word-wrapping, justification, and the drop-cap
  composition (the core).
- `src/border.rs` — the frame and its flourishes.
- `src/style.rs` — the colour palette / themes.
- `src/drollery.rs` — the marginal menagerie.
- `src/code.rs` — code mode: keyword rubrication and comment glosses.
- `src/scribe.rs` — scribal corruption (`--corrupt`).
- `src/cli.rs` — the command-line surface (arguments → `Config`).
- `src/search.rs` — the Semgrep bridge (`--find`, `--scan`).
- `src/magnifica.rs` — the magnifica art-project modes (`--magnifica`).
- `src/main.rs` — the CLI: routing and input/output.

## Shell completions & man page

```sh
lindisfarner --completions bash > /usr/local/etc/bash_completion.d/lindisfarner
lindisfarner --man > /usr/local/share/man/man1/lindisfarner.1
```

Completions are available for bash, zsh, fish, PowerShell, and elvish.

## Acknowledgements

This tool is a late wedding present for my dear friend Neil Douglas Reilly.
