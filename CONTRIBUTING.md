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
   cargo run -- --train <name>
   ```
5. Commit and open a pull request

## Adding a train

A train is one text file in `vehicles/`; no Rust is needed beyond adding the
file to `BUILTIN` in `src/vehicle.rs`. The format is described at the top of
`src/vehicle.rs`, and the existing files are the best examples.

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
