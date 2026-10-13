# Contributing

Thanks for your interest in contributing to sl-neo.

## Development

1. Fork / clone the repository
2. Install a stable Rust toolchain (https://rustup.rs)
3. Create a branch: `git switch -c feat/your-change`
4. Make your change and verify locally:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   cargo build --release
   python3 tests/pty_check.py target/release/sl-neo   # needs a Unix pty
   ```
5. Commit and open a pull request

## Adding a train

A train is one text file in `vehicles/`. The format is described at the top
of `src/vehicle.rs`, and the existing files are the best examples.

1. Draw it and try it: `cargo run -- --file vehicles/your-train.txt`
   (mistakes are reported with a line number).
2. Add it to `BUILTIN` in `src/vehicle.rs` and to the trains table in
   `README.md`. It joins the random picks unless the header says `random: no`.
3. `cargo test` checks every built-in train.

- Draw your own art. Do not paste art from other programs or sites, including
  the original `sl`, unless its license allows it and you say so in the pull
  request.
- Keep the line art readable without color (`--color never`); the color mask
  only adds to it.
- Use ASCII in the art, so every character is one column wide everywhere.
- A train "in the style of" a real one is fine; do not use railway company
  logos or names as if this project were affiliated with them.
- `cargo test` checks every built-in train: the mask fits the art, frames of a
  part are the same size, and colors are defined.
