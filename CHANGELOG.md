# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--pilcrows` (`-p`): mark paragraph breaks with a red ¶ instead of a blank
  line.
- Unicode-aware layout: line width is measured in terminal columns
  (`unicode-width`), so wide characters keep the right margin aligned.
- Integration tests covering rendering, colour modes, stdin, and error output.

### Changed

- Line endings (`\r\n`, `\r`) are normalised before paragraph splitting.
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
