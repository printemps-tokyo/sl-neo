# sl-neo

> A modern `sl`: mistype `ls` and a shinkansen, Doctor Yellow, a Yamanote Line train, a steam locomotive or a maglev runs across your terminal, in 24-bit color.

[![CI](https://github.com/printemps-tokyo/sl-neo/actions/workflows/ci.yml/badge.svg)](https://github.com/printemps-tokyo/sl-neo/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)

```text
                    ____________________ ________________________________________ ____________________
               ____/  [==]  [==]  [==]   |  [==]  [==]  [==]  [==]  [==]  [==]      [==]  [==]  [==]  \____
          ____/                          |                                                                 \____
     ____/                               |                                                                      \____
  __/                                    |                                                                           \__
 (______________________________________=|_______________________________________=______________________________________)
     (|)  (|)                (|)  (|)        (|)  (|)                 (|)  (|)       (|)  (|)                (|)  (|)
```

`sl` has been punishing `ls` typos since 1993. sl-neo keeps the joke and the
rule that you have to wait for the train, and brings Japanese trains, solid
24-bit color, a delay board and a maglev that is gone before you see it.
Every train is a plain-text art file, so adding one needs no Rust.

## Install

From source, with a Rust toolchain (https://rustup.rs):

```bash
cargo install --git https://github.com/printemps-tokyo/sl-neo
```

Tagged [releases](https://github.com/printemps-tokyo/sl-neo/releases) carry
prebuilt binaries for Linux (x86_64) and macOS (x86_64, Apple silicon), with
SHA-256 checksums.

Then make it your typo:

```bash
alias sl=sl-neo     # in ~/.zshrc or ~/.bashrc
```

## Usage

```bash
sl-neo                    # a random train
sl-neo --train e5         # pick one (see --list)
sl-neo -d                 # with a delay board
sl-neo -F                 # it flies
sl-neo --linear           # the maglev (blink and you miss it)
sl-neo -e                 # allow Ctrl+C, q or Esc to stop it
sl-neo --list
```

| Option | Description |
| --- | --- |
| `-t, --train <NAME>` | Train to run; a random one (not the maglev) when omitted |
| `--list` | List the trains (`--names` for names only) |
| `--cars <N>` | Middle cars, 0-20; each train has its own default |
| `--speed <N>` | Columns per step, 1-200; each train has its own default |
| `-F, --fly` | Let the train fly |
| `-d, --delay` | Show a scrolling delay board above the train |
| `--linear` | Run the maglev (same as `--train maglev`) |
| `-e, --escape` | Allow Ctrl+C, `q` or `Esc` to stop the train |
| `--lang <auto\|ja\|en>` | Language of the delay board and messages; `auto` reads `LC_ALL`, `LC_MESSAGES`, `LANG` |
| `--color <auto\|always\|never>` | Colors; `auto` honors [`NO_COLOR`](https://no-color.org/) and is off when not a terminal |
| `--seed <N>` | Repeat the same random train and delay |
| `--still` | Print the train once instead of running it |

Like the original, Ctrl+C does nothing until the train has passed (the
terminal is in raw mode, so keys are read and ignored rather than piling up in
your shell). Pass `-e` if you would rather be able to stop it.

When the output is not a terminal, or with `--still`, the train is printed
once, without color unless `--color always`:

```text
   (  )    (    )
 (    )  (
    ||                  _________
 ___||_____.___ _____  |  [==]   |          ________________________
|  _______ ( ) |     | |         |_______  |  [==]  [==]  [==]  [==]|
| |_______||_| |_____| |_________|:::::::| |                        |
<_|___|___|___|________|_________|_______|=|________________________|
   (|)  (|)  (|)   (|)      (|)     (|)       (|)  (|)    (|)  (|)
```

### Trains

| Name | Train |
| --- | --- |
| `e5` | E5-style shinkansen: green and white with a pink stripe, long nose |
| `doctor-yellow` | Doctor Yellow-style track inspection train: yellow with a blue stripe |
| `yamanote` | Yamanote Line-style commuter train: silver with yellow-green doors and band |
| `steam` | Steam locomotive with a tender and coaches, and smoke |
| `maglev` | A maglev. It floats, so there are no wheels. It is also very fast |

### Colors

24-bit color is used when `COLORTERM` is `truecolor` or `24bit`; otherwise the
nearest colors of the 256-color palette. The art is drawn so that it still
reads as a train with `--color never`.

## Adding a train

A train is one file in [`vehicles/`](./vehicles): a few header lines, a color
palette, and the art for the front, a car and (optionally) the back, each with
a color mask underneath. The back can be the front mirrored. The format is
documented at the top of [`src/vehicle.rs`](./src/vehicle.rs); the existing
files are the best examples, and `cargo test` checks every built-in train. See
[CONTRIBUTING.md](./CONTRIBUTING.md).

## Credits and disclaimer

- Inspired by [`sl`](https://github.com/mtoyoda/sl) by Toyoda Masashi. sl-neo
  is a separate program: none of sl's code or art is used, and the trains here
  are original art.
- The trains are drawn "in the style of" real Japanese rolling stock for fun.
  This project is not affiliated with, or endorsed by, any railway company.

## License

[MIT](./LICENSE) (c) printemps.tokyo
