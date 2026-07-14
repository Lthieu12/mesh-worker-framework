# MMF Development Cheat Sheet

A quick reference for the commands most commonly used while developing
MMF.

------------------------------------------------------------------------

# Cargo

## Build

``` bash
cargo build
cargo build --release
cargo build --workspace
cargo build --examples
```

## Run

``` bash
cargo run
cargo run --example example_name
```

## Tests

Run all tests:

``` bash
cargo test
```

Run one test:

``` bash
cargo test test_send_unique_message
```

Show `println!()` output:

``` bash
cargo test -- --nocapture
```

Run documentation tests:

``` bash
cargo test --doc
```

Run all targets:

``` bash
cargo test --workspace --all-targets --all-features
```

## Formatting

Check formatting:

``` bash
cargo fmt --check
```

Apply formatting:

``` bash
cargo fmt
```

## Clippy

Run Clippy:

``` bash
cargo clippy
```

Treat warnings as errors:

``` bash
cargo clippy -- -D warnings
```

MMF verification command:

``` bash
cargo clippy --workspace --lib --examples --all-features -- -D warnings
```

## Documentation

Generate documentation:

``` bash
cargo doc
```

Generate workspace documentation:

``` bash
cargo doc --workspace --no-deps
```

Open documentation:

``` bash
cargo doc --open
```

## Cleaning

``` bash
cargo clean
```

------------------------------------------------------------------------

# Git

## Status

``` bash
git status
```

## Diff

``` bash
git diff
git diff --staged
```

## Commit

``` bash
git add .
git commit -m "Commit message"
git push
```

## History

``` bash
git log --oneline --graph --decorate
```

Last commit:

``` bash
git show
```

------------------------------------------------------------------------

# Useful Cargo Environment Variables

Target directory:

``` powershell
$env:CARGO_TARGET_DIR = "target"
```

Reset:

``` powershell
Remove-Item Env:CARGO_TARGET_DIR
```

------------------------------------------------------------------------

# MMF

Run the complete verification script:

``` powershell
.\scripts\verify.ps1
```

------------------------------------------------------------------------

# Useful Rust Commands

Expand compiler messages:

``` bash
cargo check
```

Show dependency tree:

``` bash
cargo tree
```

Find outdated dependencies:

``` bash
cargo outdated
```

(if installed)

------------------------------------------------------------------------

# Tokio / Runtime Debugging

Display test output:

``` bash
cargo test -- --nocapture
```

Run a single test:

``` bash
cargo test test_name -- --nocapture
```

------------------------------------------------------------------------

# Notes

This document is intentionally short.

Only add commands that are actually used during MMF development.
