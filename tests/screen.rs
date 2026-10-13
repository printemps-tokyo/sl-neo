use sl_neo::screen::{
    ansi256, compose, grid_to_ansi, row_to_string, sprite_to_text, text_width, ColorMode, Grid,
    Sprite, STEPS_PER_FRAME,
};
use sl_neo::vehicle::{parse, Cell, Rgb};

const SRC: &str = "\
name: t
wheel: xy
cars: 1
color G: #000000 on #00ff00
[front]
FF
F@
===
ff
f@
[car]
C
C
C@
[back] mirror
";

fn text(s: &Sprite) -> Vec<String> {
    s.rows
        .iter()
        .map(|r| r.iter().map(|c| c.ch).collect())
        .collect()
}

#[test]
fn composes_parts_bottom_aligned_with_couplers() {
    let v = parse(SRC).unwrap();
    let s = compose(&v, 2, 0);
    // front(2) + gap + car(2) + gap + car(2) + gap + back(2)
    assert_eq!(s.width(), 2 + 1 + 2 + 1 + 2 + 1 + 2);
    assert_eq!(s.height(), 3);
    // The coupler is on the row above the bottom (wheel) row.
    assert_eq!(text(&s), vec!["   C  C    ", "FF=C =C =FF", "Fx Cx Cx xF"]);
}

#[test]
fn wheels_turn_every_step_and_frames_every_few() {
    let v = parse(SRC).unwrap();
    // Front and back only: two rows, wheels on the bottom one.
    assert_eq!(text(&compose(&v, 0, 1))[1], "Fy yF");
    assert_eq!(text(&compose(&v, 0, STEPS_PER_FRAME))[0], "ff=ff");
    assert_eq!(text(&compose(&v, 0, 2 * STEPS_PER_FRAME))[0], "FF=FF");
}

#[test]
fn zero_cars_and_no_back() {
    let v = parse("name: t\n[front]\nab\n[car]\nc\n").unwrap();
    assert_eq!(text(&compose(&v, 0, 0)), vec!["ab"]);
}

#[test]
fn blit_clips_and_keeps_what_is_under_transparent_cells() {
    let mut g = Grid::new(4, 2);
    g.text(0, 0, "....", None, None);
    let s = Sprite {
        rows: vec![vec![
            Cell {
                ch: 'a',
                fg: None,
                bg: None,
            },
            Cell::BLANK,
            Cell {
                ch: ' ',
                fg: None,
                bg: Some(Rgb(1, 2, 3)),
            },
            Cell {
                ch: 'd',
                fg: None,
                bg: None,
            },
        ]],
    };
    g.blit(&s, -1, 0);
    let row: String = (0..4).map(|x| g.get(x, 0).ch).collect();
    assert_eq!(row, ". d.");
    g.blit(&s, 3, 1);
    g.blit(&s, 0, 5);
    g.blit(&s, 0, -1);
    assert_eq!(g.get(3, 1).ch, 'a');
}

#[test]
fn wide_text_is_two_cells_and_never_split() {
    let mut g = Grid::new(5, 1);
    g.text(0, 0, "a遅b", None, None);
    assert_eq!(
        (
            g.get(0, 0).ch,
            g.get(1, 0).ch,
            g.get(2, 0).ch,
            g.get(3, 0).ch
        ),
        ('a', '遅', '\0', 'b')
    );
    let mut g = Grid::new(4, 1);
    g.text(3, 0, "遅", None, None);
    assert_eq!(g.get(3, 0).ch, ' ');
    g.text(-1, 0, "遅x", None, None);
    assert_eq!((g.get(0, 0).ch, g.get(1, 0).ch), (' ', 'x'));
    assert_eq!(text_width("【遅延】ls"), 10);
    assert_eq!(
        row_to_string(
            &Grid {
                width: 2,
                height: 1,
                cells: vec![
                    Cell {
                        ch: '遅',
                        fg: None,
                        bg: None
                    },
                    Cell {
                        ch: '\0',
                        fg: None,
                        bg: None
                    }
                ]
            }
            .cells,
            ColorMode::None,
            false
        ),
        "遅"
    );
}

#[test]
fn color_output() {
    let red = Cell {
        ch: 'r',
        fg: Some(Rgb(255, 0, 0)),
        bg: None,
    };
    let on_green = Cell {
        ch: 'g',
        fg: None,
        bg: Some(Rgb(0, 255, 0)),
    };
    let row = [red, red, on_green, Cell::BLANK];
    assert_eq!(
        row_to_string(&row, ColorMode::TrueColor, true),
        "\x1b[0;38;2;255;0;0mrr\x1b[0;48;2;0;255;0mg\x1b[0m"
    );
    assert_eq!(
        row_to_string(&row, ColorMode::Ansi256, true),
        "\x1b[0;38;5;196mrr\x1b[0;48;5;46mg\x1b[0m"
    );
    assert_eq!(row_to_string(&row, ColorMode::None, false), "rrg ");
    assert_eq!(
        row_to_string(&[Cell::BLANK], ColorMode::TrueColor, false),
        " "
    );
    let mut g = Grid::new(2, 2);
    g.text(0, 1, "hi", None, None);
    assert_eq!(grid_to_ansi(&g, ColorMode::None), "\x1b[1;1H  \x1b[2;1Hhi");
    let s = Sprite {
        rows: vec![row.to_vec(), vec![Cell::BLANK]],
    };
    assert_eq!(sprite_to_text(&s, ColorMode::None), "rrg\n\n");
}

#[test]
fn nearest_256_colors() {
    assert_eq!(ansi256(Rgb(0, 0, 0)), 16);
    assert_eq!(ansi256(Rgb(255, 255, 255)), 231);
    assert_eq!(ansi256(Rgb(255, 0, 0)), 196);
    assert_eq!(ansi256(Rgb(0, 0, 255)), 21);
    assert_eq!(ansi256(Rgb(128, 128, 128)), 243);
    // Levels 0, 95, 135, 175, 215, 255: 0x9a -> 135 (2), 0xcd -> 215 (4), 0x32 -> 95 (1).
    assert_eq!(ansi256(Rgb(0x9a, 0xcd, 0x32)), 16 + 36 * 2 + 6 * 4 + 1);
}
