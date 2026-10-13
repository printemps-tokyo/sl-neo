//! Putting a train together and drawing it. Pure: no terminal access.

use std::fmt::Write as _;

use unicode_width::UnicodeWidthChar;

use crate::vehicle::{Cell, Part, Rgb, Vehicle};

/// A frame advances every this many steps (smoke, for example).
pub const STEPS_PER_FRAME: usize = 4;

/// The coupler drawn in the one-column gap between two parts, on the row
/// above the bottom one (the bottom row is the wheels).
const COUPLER: Cell = Cell {
    ch: '=',
    fg: Some(Rgb(0x80, 0x80, 0x80)),
    bg: None,
};

/// A whole train at one step: the front, the cars and the back side by side,
/// bottom-aligned, with a one-column gap and a coupler between parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub rows: Vec<Vec<Cell>>,
}

impl Sprite {
    pub fn width(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }
    pub fn height(&self) -> usize {
        self.rows.len()
    }
}

/// Width of the train with `cars` middle cars, without composing it.
pub fn train_width(v: &Vehicle, cars: usize) -> usize {
    let parts = 1 + cars + usize::from(v.back.is_some());
    v.front.width() + cars * v.car.width() + v.back.as_ref().map_or(0, Part::width) + (parts - 1)
}

/// The train at `step`, with `cars` middle cars.
pub fn compose(v: &Vehicle, cars: usize, step: usize) -> Sprite {
    let mut parts: Vec<&Part> = vec![&v.front];
    parts.extend(std::iter::repeat_n(&v.car, cars));
    if let Some(back) = &v.back {
        parts.push(back);
    }
    let height = parts.iter().map(|p| p.height()).max().unwrap_or(0);
    let wheel = if v.wheel.is_empty() {
        None
    } else {
        Some(v.wheel[step % v.wheel.len()])
    };
    let mut rows: Vec<Vec<Cell>> = vec![Vec::new(); height];
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            for (y, row) in rows.iter_mut().enumerate() {
                row.push(if y + 2 == height {
                    COUPLER
                } else {
                    Cell::BLANK
                });
            }
        }
        let frame = &part.frames[(step / STEPS_PER_FRAME) % part.frames.len()];
        let offset = height - frame.height();
        for (y, row) in rows.iter_mut().enumerate() {
            if y < offset {
                row.extend(std::iter::repeat_n(Cell::BLANK, frame.width()));
            } else {
                row.extend(frame.rows[y - offset].iter().map(|c| match (c.ch, wheel) {
                    ('@', Some(w)) => Cell { ch: w, ..*c },
                    _ => *c,
                }));
            }
        }
    }
    Sprite { rows }
}

/// What the terminal is drawn into. A wide character takes two cells; the
/// second holds `'\0'` and is not printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid {
            width,
            height,
            cells: vec![Cell::BLANK; width * height],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Cell {
        self.cells[y * self.width + x]
    }

    /// Set one cell. Overwriting half of a wide character blanks its other
    /// half, so a row never holds a wide character without its second cell.
    fn set(&mut self, x: i64, y: i64, cell: Cell) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let (x, row) = (x as usize, y as usize * self.width);
        let old = self.cells[row + x];
        if old.ch == '\0' && x > 0 && cell.ch != '\0' {
            self.cells[row + x - 1].ch = ' ';
        }
        if x + 1 < self.width && self.cells[row + x + 1].ch == '\0' && old.ch != '\0' {
            self.cells[row + x + 1].ch = ' ';
        }
        self.cells[row + x] = cell;
    }

    /// Draw a sprite with its top-left corner at (x, y), which may be off screen.
    /// Transparent cells leave what is under them.
    pub fn blit(&mut self, s: &Sprite, x: i64, y: i64) {
        for (dy, row) in s.rows.iter().enumerate() {
            for (dx, cell) in row.iter().enumerate() {
                if !cell.is_transparent() {
                    self.set(x + dx as i64, y + dy as i64, *cell);
                }
            }
        }
    }

    /// Fill row `y` with a background.
    pub fn fill_row(&mut self, y: usize, bg: Option<Rgb>) {
        for x in 0..self.width {
            self.set(
                x as i64,
                y as i64,
                Cell {
                    ch: ' ',
                    fg: None,
                    bg,
                },
            );
        }
    }

    /// Write text starting at column x (may be negative) on row y. A wide
    /// character that would be cut by an edge is drawn as a space.
    pub fn text(&mut self, x: i64, y: usize, text: &str, fg: Option<Rgb>, bg: Option<Rgb>) {
        let mut col = x;
        for ch in text.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0) as i64;
            if w == 0 {
                continue;
            }
            let fits = col >= 0 && col + w <= self.width as i64;
            if w == 2 && !fits {
                for c in col..col + w {
                    self.set(c, y as i64, Cell { ch: ' ', fg, bg });
                }
            } else {
                self.set(col, y as i64, Cell { ch, fg, bg });
                if w == 2 {
                    self.set(col + 1, y as i64, Cell { ch: '\0', fg, bg });
                }
            }
            col += w;
        }
    }
}

/// Display width of a string in columns.
pub fn text_width(s: &str) -> usize {
    s.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
        .sum()
}

/// How colors are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    /// 24-bit colors (`COLORTERM=truecolor` or `24bit`).
    TrueColor,
    /// The 256-color palette, nearest match.
    Ansi256,
    /// No escape sequences for color.
    None,
}

/// Channel levels of the xterm 6x6x6 color cube (indexes 16-231).
const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// The nearest color in the xterm 256-color palette: the closest cube color
/// or the closest gray of the ramp 232-255 (8 + 10 * i), whichever is nearer.
pub fn ansi256(c: Rgb) -> u8 {
    let Rgb(r, g, b) = c;
    let nearest = |v: u8| -> usize {
        (0..6)
            .min_by_key(|&i| (CUBE[i] as i32 - v as i32).abs())
            .unwrap()
    };
    let (ri, gi, bi) = (nearest(r), nearest(g), nearest(b));
    let cube = Rgb(CUBE[ri], CUBE[gi], CUBE[bi]);
    let avg = (r as i32 + g as i32 + b as i32) / 3;
    let gi_ramp = ((avg - 8 + 5) / 10).clamp(0, 23);
    let gray_level = (8 + 10 * gi_ramp) as u8;
    let dist = |p: Rgb| -> i32 {
        let d = |a: u8, b: u8| (a as i32 - b as i32).pow(2);
        d(p.0, r) + d(p.1, g) + d(p.2, b)
    };
    if dist(Rgb(gray_level, gray_level, gray_level)) < dist(cube) {
        232 + gi_ramp as u8
    } else {
        16 + 36 * ri as u8 + 6 * gi as u8 + bi as u8
    }
}

fn sgr(out: &mut String, fg: Option<Rgb>, bg: Option<Rgb>, mode: ColorMode) {
    out.push_str("\x1b[0");
    match mode {
        ColorMode::TrueColor => {
            if let Some(Rgb(r, g, b)) = fg {
                let _ = write!(out, ";38;2;{r};{g};{b}");
            }
            if let Some(Rgb(r, g, b)) = bg {
                let _ = write!(out, ";48;2;{r};{g};{b}");
            }
        }
        ColorMode::Ansi256 => {
            if let Some(c) = fg {
                let _ = write!(out, ";38;5;{}", ansi256(c));
            }
            if let Some(c) = bg {
                let _ = write!(out, ";48;5;{}", ansi256(c));
            }
        }
        ColorMode::None => {}
    }
    out.push('m');
}

/// One row of cells as text, colored for `mode`, ending with a color reset
/// when any color was written. Trailing transparent cells are dropped when
/// `trim` is set.
pub fn row_to_string(cells: &[Cell], mode: ColorMode, trim: bool) -> String {
    let end = if trim {
        cells
            .iter()
            .rposition(|c| !c.is_transparent())
            .map_or(0, |i| i + 1)
    } else {
        cells.len()
    };
    let mut out = String::new();
    let mut current: (Option<Rgb>, Option<Rgb>) = (None, None);
    for c in &cells[..end] {
        if c.ch == '\0' {
            continue;
        }
        if mode != ColorMode::None && (c.fg, c.bg) != current {
            sgr(&mut out, c.fg, c.bg, mode);
            current = (c.fg, c.bg);
        }
        out.push(c.ch);
    }
    if current != (None, None) {
        out.push_str("\x1b[0m");
    }
    out
}

/// The whole grid as one write: each row positioned with a cursor move, so a
/// frame replaces the previous one without clearing the screen.
pub fn grid_to_ansi(g: &Grid, mode: ColorMode) -> String {
    let mut out = String::with_capacity(g.width * g.height * 2);
    for y in 0..g.height {
        let _ = write!(out, "\x1b[{};1H", y + 1);
        out.push_str(&row_to_string(
            &g.cells[y * g.width..(y + 1) * g.width],
            mode,
            false,
        ));
    }
    out
}

/// A sprite as plain lines (for output that is not a terminal, or --still).
pub fn sprite_to_text(s: &Sprite, mode: ColorMode) -> String {
    let mut out = String::new();
    for row in &s.rows {
        out.push_str(&row_to_string(row, mode, true));
        out.push('\n');
    }
    out
}
