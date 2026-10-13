//! One run across the screen: where the train is at each step, the delay
//! board, and the message after a train has passed. Pure.

use crate::screen::{compose, text_width, train_width, Grid, Sprite};
use crate::vehicle::{Rgb, Vehicle};

/// Steps the closing message (`after:`) stays on screen.
pub const AFTER_STEPS: usize = 50;
/// A flying train climbs one row per this many columns travelled.
pub const FLY_SLOPE: usize = 6;

const BOARD_FG: Rgb = Rgb(0xff, 0xa2, 0x00);
const BOARD_BG: Rgb = Rgb(0x10, 0x10, 0x10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ja,
    En,
}

impl Lang {
    /// Japanese when the first set of LC_ALL, LC_MESSAGES and LANG starts with "ja".
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Lang {
        let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|k| get(k))
            .find(|v| !v.is_empty())
            .unwrap_or_default();
        if locale.starts_with("ja") {
            Lang::Ja
        } else {
            Lang::En
        }
    }
}

/// The delay board text.
pub fn delay_message(v: &Vehicle, minutes: u32, lang: Lang) -> String {
    match lang {
        Lang::Ja => format!(
            "【遅延情報】lsの打ち間違いの影響で、{}が約{}分遅れて運転しています。お急ぎのところ大変ご迷惑をおかけしております。",
            v.title_ja, minutes
        ),
        Lang::En => format!(
            "[Service update] The {} is running about {} min late because of a mistyped ls. We apologize for the inconvenience.",
            v.title, minutes
        ),
    }
}

/// What to show in one run.
pub struct Scene<'a> {
    pub vehicle: &'a Vehicle,
    pub cars: usize,
    /// Columns per step.
    pub speed: usize,
    pub fly: bool,
    /// The delay board text, when shown.
    pub board: Option<String>,
    /// Shown for a while after the train has passed.
    pub after: Option<String>,
}

impl Scene<'_> {
    pub fn sprite(&self, step: usize) -> Sprite {
        compose(self.vehicle, self.cars, step)
    }

    /// Left edge of the train at `step` on a screen `width` wide.
    pub fn x(&self, step: usize, width: usize) -> i64 {
        width as i64 - (step * self.speed) as i64
    }

    /// Whether the train is completely off the left edge.
    pub fn passed(&self, step: usize, width: usize) -> bool {
        self.x(step, width) + train_width(self.vehicle, self.cars) as i64 <= 0
    }

    /// Top edge of the train.
    pub fn y(&self, step: usize, height: usize, sprite_height: usize) -> i64 {
        let top = if self.board.is_some() { 2 } else { 0 };
        let room = height.saturating_sub(top);
        let base = top as i64 + (room.saturating_sub(sprite_height) / 2) as i64;
        if self.fly {
            base - ((step * self.speed) / FLY_SLOPE) as i64
        } else {
            base
        }
    }

    /// The screen at `step`. `after` counts steps since the train passed.
    pub fn render(&self, step: usize, width: usize, height: usize, after: Option<usize>) -> Grid {
        let mut g = Grid::new(width, height);
        if height == 0 || width == 0 {
            return g;
        }
        match after {
            None => {
                let s = self.sprite(step);
                g.blit(&s, self.x(step, width), self.y(step, height, s.height()));
            }
            Some(_) => {
                if let Some(msg) = &self.after {
                    let x = (width as i64 - text_width(msg) as i64) / 2;
                    g.text(x.max(0), height / 2, msg, None, None);
                }
            }
        }
        // The board goes on last, so a flying train passes behind it.
        if let Some(board) = &self.board {
            g.fill_row(0, Some(BOARD_BG));
            let span = width + text_width(board);
            let x = width as i64 - (step % span) as i64;
            g.text(x, 0, board, Some(BOARD_FG), Some(BOARD_BG));
        }
        g
    }

    /// Steps the closing message is shown for (0 without one).
    pub fn after_steps(&self) -> usize {
        if self.after.is_some() {
            AFTER_STEPS
        } else {
            0
        }
    }
}

/// splitmix64: a small, good-enough generator for picking a train.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// A number in 0..n (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}
