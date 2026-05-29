# Contributing

Thanks for your interest in **lindisfarner**! This is a small toy project, so
the process is light.

## Getting started

```sh
git clone https://github.com/norwytch/lindisfarner
cd lindisfarner
cargo build
cargo run -- sample.txt
```

## Before you open a pull request

Please run the same checks CI does:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Ideas

The README's "Ideas to extend" section lists features that would make good
first contributions — extra drolleries, margin placement, glosses and
manicules, line fillers, pilcrows, and more.

## Reporting bugs

Open an issue with the input that triggered it, the command line you ran, and
what you expected versus what you got.
