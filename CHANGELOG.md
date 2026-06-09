# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **The magnifica modes** (`--magnifica <witness|relinquish>`) — an art project.
  lindisfarner finds where a codebase uses AI — hosted-vendor SDKs (`rules/ai.yml`),
  the model lifecycle of training your own neural net (`rules/ml.yml`), and model
  weight files on disk (a filesystem scan Semgrep can't do) — and answers with the
  words of the encyclical *Magnifica Humanitas*:
  Both modes **write the encyclical into the files on disk**, then print an
  illuminated report: `witness` inserts the encyclical as a comment beside each AI
  invocation, leaving the code intact; `relinquish` strikes each AI block out and
  leaves the encyclical's words in its place, breaking what it touches. Model
  weight files are reported but never modified. The writes go straight to disk
  with no safety net — keeping a backup or a clean commit is the caller's
  responsibility. Passages live in `assets/quotes.txt` (override with
  `--quotes`).

- **Code illumination** (`--code`): treat a source file like a glossed
  manuscript — keep its lines verbatim, rubricate the language's keywords in
  red, and lift comments out into the margin as glosses. Auto-detected from the
  file extension (Rust, Python, JavaScript/TypeScript, C/C++, Go, shell);
  `--language` overrides detection and `--prose` forces prose mode.
- `--corrupt`: introduce scribal transcription errors — the transposed, dropped,
  doubled, and misread letters of a tired monk — purposefully breaking the text
  (and code), deterministically by `--seed`. Only letters are touched, so
  whitespace, punctuation, and line count survive.
- `--find <PATTERN>`: search a file or directory for a [Semgrep] pattern and
  illuminate the matches as a glossed commentary page — each match rubricated,
  glossed with its `file:line`. Requires `semgrep` on `PATH`. Also exposed as the
  library function `render_glossed`.
- `--scan <RULES>`: scan with a [Semgrep] rule config — a file, a whole directory
  of rules, or a registry ruleset — and gloss each finding with its rule message,
  marked by severity († ERROR, ☞ WARNING, ❧ INFO). A starter rule library ships
  in `rules/`.

[Semgrep]: https://semgrep.dev
- `--pilcrows` (`-p`): run paragraphs together as one continuous block, a red ¶
  marking each break inline (as a medieval scribe did) instead of a blank line.
  Successive pilcrows alternate red and blue.
- `--columns <N>`: set the body in N columns, codex-style.
- `--justify` (`-j`): set the body flush to both margins.
- `--hyphenate`: break over-long words with a trailing hyphen.
- `--incipit`: rubricate the opening line.
- `--fillers`: fill short closing lines with ❧ ornaments.
- `--completions <SHELL>` and `--man`: emit shell completions and a man page.
- A public library API: `Config` and `render`, so the scriptorium can be used
  from other Rust programs.
- Unicode-aware layout: line width is measured in terminal columns
  (`unicode-width`), so wide characters keep the right margin aligned.
- Property tests (`proptest`) for the wrapping invariants, plus integration
  tests covering rendering, colour modes, stdin, and error output.
- CI now tests on Linux, macOS, and Windows, with a dedicated MSRV (1.85) job.

### Changed

- `--width` now defaults to the terminal width (60 columns when piped).
- The first word of a drop-capped paragraph can now be rubricated again — the
  stem beside the initial recovers the rubric.
- Drolleries are now scattered down the margin at fixed intervals, independent
  of paragraph structure, so the menagerie fills the margin even for a single
  (or pilcrow-joined) paragraph.
- Line endings (`\r\n`, `\r`) are normalised before paragraph splitting.
- A closed output pipe (e.g. `… | head`) now exits quietly.
- Errors now print a concise `lindisfarner: <message>` instead of a `Debug`
  dump, and file errors include the path.

## [0.1.0] - 2026-05-29

### Added

- Initial release.
- Illuminated drop-cap initials via FIGlet, with an embedded blackletter
  (Fraktur) font and a plain `--font standard` alternative.
- Rubrication of chosen words (`--rubricate`).
- Border styles: `none`, `simple`, `double`, and an `ornate` frame with a ❦
  flourish.
- Colour themes (`gold`, `crimson`, `mono`) with `--color auto|always|never`;
  output stays plain text when piped or written to a file.
- Marginal drolleries (`--drolleries`) with deterministic, `--seed`-able
  selection.
- Reads from a file or standard input; writes to stdout or `--output`.

[Unreleased]: https://github.com/norwytch/lindisfarner/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/norwytch/lindisfarner/releases/tag/v0.1.0
