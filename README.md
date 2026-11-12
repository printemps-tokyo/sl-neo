# sl-neo

> A modern `sl`: mistype `ls` and a shinkansen, Doctor Yellow, a Yamanote Line train, a steam locomotive, a freight train, a streetcar, a night express or a maglev runs across your terminal, in 24-bit color. Linux, macOS and Windows.

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
Every train is a plain-text art file: you can draw one and run it with
`--file`, no Rust needed.

## Install

From source, with a Rust toolchain (https://rustup.rs):

```bash
cargo install --git https://github.com/printemps-tokyo/sl-neo
```

Tagged [releases](https://github.com/printemps-tokyo/sl-neo/releases) carry
prebuilt binaries for Linux (x86_64), macOS (x86_64, Apple silicon) and
Windows (x86_64, a zip with `sl-neo.exe`), with SHA-256 checksums.

Then make it your typo:

```bash
alias sl=sl-neo     # in ~/.zshrc or ~/.bashrc
```

```powershell
Set-Alias sl sl-neo   # in your PowerShell $PROFILE
```

On Windows, sl-neo runs in Windows Terminal and in the classic console of
Windows 10 and later: it switches on the console's escape-sequence processing
and falls back to printing the train once if the console refuses. Windows
Terminal gets 24-bit color (it is recognized by the `WT_SESSION` variable it
sets). The keys work as elsewhere; there are no Unix signals, so closing the
window simply ends it.

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
| `--file <PATH>` | Run a train from a file in the [`vehicles/`](./vehicles) format |
| `--check <PATH>` | Check a train file before contributing it: a summary, the contribution rules, exit 1 on an error |
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
your shell). Pass `-e` if you would rather be able to stop it. `SIGTERM` and
`SIGHUP` always end the run, after the terminal is restored, with exit status
128 + the signal; `SIGINT` from another process is ignored unless `-e`.

When the output is not a terminal (or `TERM` is `dumb`), the train is printed
once, without color unless `--color always`. `--still` prints it once on a
terminal too, in color:

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
| `freight` | Container freight train behind a red electric locomotive with two pantographs |
| `tram` | City streetcar: cream and green, a single pantograph, a cab at each end |
| `sleeper` | Blue night express: a locomotive with a headboard and blue sleeper cars |

### Colors

24-bit color is used when `COLORTERM` is `truecolor` or `24bit`, or in
Windows Terminal; otherwise the nearest colors of the 256-color palette. The art is drawn so that it still
reads as a train with `--color never`.

## Adding a train

A train is one text file: a few header lines, a color palette, and the art
for the front, a car and (optionally) the back, each with a color mask
underneath. The back can be the front mirrored. The format is documented at
the top of [`src/vehicle.rs`](./src/vehicle.rs), and the files in
[`vehicles/`](./vehicles) are the best examples.

- Try your own train without building anything: `sl-neo --file my-train.txt`.
  A mistake is reported with its line number.
- `sl-neo --check my-train.txt` prints its parts and sizes and applies the
  rules a contributed train must meet: a plain name not already taken, an
  English and a Japanese title, ASCII art (one column per character
  everywhere), at most 12 rows so it fits an ordinary terminal, and a first
  comment line saying the art is original.
- To ship it with sl-neo, add the file to `vehicles/` and to `BUILTIN` in
  `src/vehicle.rs`; `cargo test` then checks it. It joins the random picks
  unless its header says `random: no`. See [CONTRIBUTING.md](./CONTRIBUTING.md).

### Trains wanted

New trains are welcome, from anywhere in the world. Send a pull request, or,
if you would rather not touch the code, open a
[train proposal](https://github.com/printemps-tokyo/sl-neo/issues/new?template=train.yml)
with the file and the output of `sl-neo --check`; a maintainer adds it. Some
ideas nobody has drawn yet: a monorail, a rack railway climbing a mountain, a
snowplow, a double-decker, a cable car, a toy train with a smiling face, your
local line. The art must be your own (it is published under this project's
MIT license); "in the style of" a real train is fine, company logos are not.

## Credits and disclaimer

- Inspired by [`sl`](https://github.com/mtoyoda/sl) by Toyoda Masashi. sl-neo
  is a separate program: none of sl's code or art is used, and the trains here
  are original art.
- The trains are drawn "in the style of" real Japanese rolling stock for fun.
  This project is not affiliated with, or endorsed by, any railway company.

## License

[MIT](./LICENSE) (c) printemps.tokyo
