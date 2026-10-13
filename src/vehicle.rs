//! Trains as data.
//!
//! A train is a text file: a header of `key: value` lines, then sections.
//!
//! ```text
//! # comment
//! name: e5
//! title: E5-style shinkansen
//! title_ja: E5系風の新幹線
//! speed: 2              columns per frame
//! cars: 3               middle cars by default
//! wheel: |/-\           characters an @ in the art cycles through
//! random: no            never picked at random (optional; default yes)
//! after: Too fast?      shown for a moment after the train has passed (optional)
//! after_ja: 速すぎた?    the same in Japanese (optional)
//! color G: #0d4a33 on #1f9b6b   foreground, and optionally "on" a background
//! color g: #2bb07a              foreground only
//!
//! [front]               the leading end (the train runs right to left)
//! <art lines>
//! --- mask
//! <mask lines>          one letter per art character: a color, or space/'.' for none
//! ===                   another frame of the same part (optional)
//! <art lines>
//! --- mask
//! <mask lines>
//!
//! [car]                 repeated `cars` times
//! ...
//! [back] mirror         the front, mirrored (or a [back] with its own art; optional)
//! ```
//!
//! Art lines are taken verbatim, so a section's art starts on the line after
//! its header. A section header is one of the exact lines `[front]`, `[car]`,
//! `[back]` and `[back] mirror`; `--- mask` and `===` are reserved too. Rows are padded with spaces to the widest art or mask line, so
//! trailing spaces can be trimmed by an editor. Missing mask rows mean no
//! color; extra mask rows must be blank.

use std::collections::HashMap;
use std::fmt;

/// A 24-bit color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// One character cell. `ch == ' '` with no background is transparent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

impl Cell {
    pub const BLANK: Cell = Cell {
        ch: ' ',
        fg: None,
        bg: None,
    };

    pub fn is_transparent(&self) -> bool {
        self.ch == ' ' && self.bg.is_none()
    }
}

/// One frame of a part: rows of cells, all the same width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub rows: Vec<Vec<Cell>>,
}

impl Frame {
    pub fn width(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }
    pub fn height(&self) -> usize {
        self.rows.len()
    }
}

/// A part of the train (front, car or back): one or more frames of the same size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub frames: Vec<Frame>,
}

impl Part {
    pub fn width(&self) -> usize {
        self.frames[0].width()
    }
    pub fn height(&self) -> usize {
        self.frames[0].height()
    }

    /// The part flipped left to right, with directional characters swapped.
    pub fn mirrored(&self) -> Part {
        Part {
            frames: self
                .frames
                .iter()
                .map(|f| Frame {
                    rows: f
                        .rows
                        .iter()
                        .map(|r| {
                            r.iter()
                                .rev()
                                .map(|c| Cell {
                                    ch: mirror_char(c.ch),
                                    ..*c
                                })
                                .collect()
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

fn mirror_char(c: char) -> char {
    match c {
        '/' => '\\',
        '\\' => '/',
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        other => other,
    }
}

/// A train.
#[derive(Clone, Debug)]
pub struct Vehicle {
    pub name: String,
    pub title: String,
    pub title_ja: String,
    /// Columns per frame.
    pub speed: usize,
    /// Middle cars by default.
    pub cars: usize,
    /// Characters an `@` in the art cycles through; empty means `@` stays.
    pub wheel: Vec<char>,
    /// Whether a run without --train may pick it (`random: no` to opt out).
    pub random: bool,
    /// Shown after the train has passed (`after:` / `after_ja:`), if set.
    pub after: Option<String>,
    pub after_ja: Option<String>,
    pub front: Part,
    pub car: Part,
    pub back: Option<Part>,
}

/// A parse error, with the 1-based line it was found on (0 when it is about the whole file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

fn err<T>(line: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError {
        line,
        message: message.into(),
    })
}

/// `#rrggbb`.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// `#fg`, `#fg on #bg` or `on #bg`.
fn parse_style(s: &str) -> Option<(Option<Rgb>, Option<Rgb>)> {
    let s = s.trim();
    let (fg, bg) = match s.split_once(" on ") {
        Some((a, b)) => (a.trim(), Some(b.trim())),
        None => match s.strip_prefix("on ") {
            Some(b) => ("", Some(b.trim())),
            None => (s, None),
        },
    };
    let fg = if fg.is_empty() {
        None
    } else {
        Some(parse_hex(fg)?)
    };
    let bg = match bg {
        Some(b) => Some(parse_hex(b)?),
        None => None,
    };
    if fg.is_none() && bg.is_none() {
        return None;
    }
    Some((fg, bg))
}

type Palette = HashMap<char, (Option<Rgb>, Option<Rgb>)>;

/// Raw lines of one frame, with the line number of the first art line.
struct RawFrame {
    line: usize,
    art: Vec<String>,
    mask: Vec<String>,
}

fn build_frame(raw: &RawFrame, palette: &Palette) -> Result<Frame, ParseError> {
    let mut mask = raw.mask.clone();
    while mask.len() > raw.art.len() {
        if !mask.last().is_some_and(|m| m.trim().is_empty()) {
            return err(
                raw.line,
                format!(
                    "the mask has {} rows but the art has {}",
                    mask.len(),
                    raw.art.len()
                ),
            );
        }
        mask.pop();
    }
    if raw.art.is_empty() {
        return err(raw.line, "a frame has no art");
    }
    let width = raw
        .art
        .iter()
        .chain(mask.iter())
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    if width == 0 {
        return err(raw.line, "a frame is empty");
    }
    let mut rows = Vec::with_capacity(raw.art.len());
    for (i, art) in raw.art.iter().enumerate() {
        let art: Vec<char> = art.chars().collect();
        let m: Vec<char> = mask.get(i).map(|m| m.chars().collect()).unwrap_or_default();
        let mut row = Vec::with_capacity(width);
        for x in 0..width {
            let ch = art.get(x).copied().unwrap_or(' ');
            if !(ch == ' ' || ch.is_ascii_graphic()) {
                return err(
                    raw.line + i,
                    format!("art must be printable ASCII, found {ch:?}"),
                );
            }
            let key = m.get(x).copied().unwrap_or(' ');
            let (fg, bg) = match key {
                ' ' | '.' => (None, None),
                k => match palette.get(&k) {
                    Some(style) => *style,
                    None => {
                        return err(
                            raw.line + raw.art.len() + 1 + i,
                            format!("mask letter {k:?} has no color"),
                        )
                    }
                },
            };
            row.push(Cell { ch, fg, bg });
        }
        rows.push(row);
    }
    Ok(Frame { rows })
}

fn build_part(name: &str, frames: &[RawFrame], palette: &Palette) -> Result<Part, ParseError> {
    let built = frames
        .iter()
        .map(|f| build_frame(f, palette))
        .collect::<Result<Vec<_>, _>>()?;
    let (w, h) = (built[0].width(), built[0].height());
    for (f, raw) in built.iter().zip(frames) {
        if f.width() != w || f.height() != h {
            return err(
                raw.line,
                format!(
                    "every frame of [{name}] must be {w}x{h}, this one is {}x{}",
                    f.width(),
                    f.height()
                ),
            );
        }
    }
    Ok(Part { frames: built })
}

/// Parse a train file.
pub fn parse(src: &str) -> Result<Vehicle, ParseError> {
    let mut header: HashMap<String, (usize, String)> = HashMap::new();
    let mut palette: Palette = HashMap::new();
    // section name -> (mirror?, frames)
    let mut sections: Vec<(String, usize, bool, Vec<RawFrame>)> = Vec::new();
    #[derive(PartialEq)]
    enum Mode {
        Header,
        Art,
        Mask,
    }
    let mut mode = Mode::Header;

    for (idx, line) in src.lines().enumerate() {
        let n = idx + 1;
        let trimmed = line.trim();
        // Only these exact lines (from column 0) start a section, so art such
        // as "[==]" is never mistaken for a header.
        let header_line = match line.trim_end() {
            "[front]" => Some(("front", false)),
            "[car]" => Some(("car", false)),
            "[back]" => Some(("back", false)),
            "[back] mirror" => Some(("back", true)),
            "[front] mirror" | "[car] mirror" => return err(n, "only [back] can be a mirror"),
            _ => None,
        };
        if header_line.is_none() && mode == Mode::Header && trimmed.starts_with('[') {
            return err(
                n,
                format!("unknown section {trimmed:?}; use [front], [car], [back] or [back] mirror"),
            );
        }
        if let Some((name, mirror)) = header_line {
            let name = name.to_string();
            if sections.iter().any(|s| s.0 == name) {
                return err(n, format!("[{name}] appears twice"));
            }
            let frames = if mirror {
                Vec::new()
            } else {
                vec![RawFrame {
                    line: n + 1,
                    art: Vec::new(),
                    mask: Vec::new(),
                }]
            };
            sections.push((name, n, mirror, frames));
            mode = if mirror { Mode::Header } else { Mode::Art };
            continue;
        }
        match mode {
            Mode::Header => {
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if !sections.is_empty() {
                    return err(n, "unexpected text after [back] mirror");
                }
                if let Some(rest) = trimmed.strip_prefix("color ") {
                    let (key, style) = match rest.split_once(':') {
                        Some(kv) => kv,
                        None => return err(n, "expected `color X: #rrggbb`"),
                    };
                    let mut chars = key.trim().chars();
                    let letter = match (chars.next(), chars.next()) {
                        (Some(c), None) if c.is_ascii_alphanumeric() => c,
                        _ => return err(n, "a color name is one letter or digit"),
                    };
                    let parsed = match parse_style(style) {
                        Some(p) => p,
                        None => {
                            return err(
                                n,
                                format!(
                                    "bad color {:?}; use #rrggbb, #rrggbb on #rrggbb or on #rrggbb",
                                    style.trim()
                                ),
                            )
                        }
                    };
                    if palette.insert(letter, parsed).is_some() {
                        return err(n, format!("color {letter} is defined twice"));
                    }
                    continue;
                }
                let (k, v) = match trimmed.split_once(':') {
                    Some(kv) => kv,
                    None => return err(n, "expected `key: value`"),
                };
                let k = k.trim().to_string();
                if !matches!(
                    k.as_str(),
                    "name"
                        | "title"
                        | "title_ja"
                        | "speed"
                        | "cars"
                        | "wheel"
                        | "random"
                        | "after"
                        | "after_ja"
                ) {
                    return err(n, format!("unknown key {k:?}"));
                }
                if header
                    .insert(k.clone(), (n, v.trim().to_string()))
                    .is_some()
                {
                    return err(n, format!("{k} is set twice"));
                }
            }
            Mode::Art | Mode::Mask => {
                let frames = &mut sections.last_mut().unwrap().3;
                if trimmed == "--- mask" {
                    if mode == Mode::Mask {
                        return err(n, "a second `--- mask` in one frame");
                    }
                    mode = Mode::Mask;
                } else if trimmed == "===" {
                    frames.push(RawFrame {
                        line: n + 1,
                        art: Vec::new(),
                        mask: Vec::new(),
                    });
                    mode = Mode::Art;
                } else if mode == Mode::Art {
                    frames.last_mut().unwrap().art.push(line.to_string());
                } else {
                    frames.last_mut().unwrap().mask.push(line.to_string());
                }
            }
        }
    }

    let get = |k: &str| header.get(k).map(|(_, v)| v.as_str());
    let name = match get("name") {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => return err(0, "missing `name`"),
    };
    let title = get("title").unwrap_or(&name).to_string();
    let title_ja = get("title_ja").unwrap_or(&title).to_string();
    let number = |k: &str, default: usize, min: usize, max: usize| -> Result<usize, ParseError> {
        match header.get(k) {
            None => Ok(default),
            Some((line, v)) => match v.parse::<usize>() {
                Ok(x) if (min..=max).contains(&x) => Ok(x),
                _ => err(*line, format!("{k} must be a number from {min} to {max}")),
            },
        }
    };
    let speed = number("speed", 1, 1, 200)?;
    let cars = number("cars", 2, 0, 50)?;
    let wheel: Vec<char> = get("wheel").unwrap_or("").chars().collect();
    if let Some(bad) = wheel.iter().find(|c| !c.is_ascii_graphic()) {
        return err(
            header["wheel"].0,
            format!("wheel characters must be printable ASCII, found {bad:?}"),
        );
    }
    let random = match get("random") {
        None | Some("yes") => true,
        Some("no") => false,
        Some(_) => return err(header["random"].0, "random must be yes or no"),
    };
    let after = get("after").filter(|s| !s.is_empty()).map(str::to_string);
    let after_ja = get("after_ja")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| after.clone());

    let mut front = None;
    let mut car = None;
    let mut back = None;
    let mut back_mirror = false;
    for (sname, line, mirror, frames) in &sections {
        if *mirror {
            back_mirror = true;
            continue;
        }
        let part = build_part(sname, frames, &palette).map_err(|e| ParseError {
            line: if e.line == 0 { *line } else { e.line },
            message: e.message,
        })?;
        match sname.as_str() {
            "front" => front = Some(part),
            "car" => car = Some(part),
            _ => back = Some(part),
        }
    }
    let front = match front {
        Some(f) => f,
        None => return err(0, "missing [front]"),
    };
    let car = match car {
        Some(c) => c,
        None => return err(0, "missing [car]"),
    };
    if back_mirror {
        back = Some(front.mirrored());
    }
    Ok(Vehicle {
        name,
        title,
        title_ja,
        speed,
        cars,
        wheel,
        random,
        after,
        after_ja,
        front,
        car,
        back,
    })
}

/// The trains that ship with sl-neo, in `--list` order.
pub const BUILTIN: &[(&str, &str)] = &[
    ("e5", include_str!("../vehicles/e5.txt")),
    (
        "doctor-yellow",
        include_str!("../vehicles/doctor-yellow.txt"),
    ),
    ("yamanote", include_str!("../vehicles/yamanote.txt")),
    ("steam", include_str!("../vehicles/steam.txt")),
    ("maglev", include_str!("../vehicles/maglev.txt")),
];

/// The built-in train with this name.
pub fn builtin(name: &str) -> Option<Vehicle> {
    BUILTIN
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(n, src)| parse(src).unwrap_or_else(|e| panic!("built-in train {n}: {e}")))
}
