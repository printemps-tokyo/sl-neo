use sl_neo::vehicle::{builtin, parse, parse_hex, Rgb, BUILTIN};

const MINI: &str = "\
name: mini
title: mini train
speed: 3
cars: 2
wheel: ab
color G: #000000 on #00ff00
color w: #ffffff

[front]
 /@@
/__|
--- mask
 www
GGGG
===
 \\@@
/__|
--- mask
 www
GGGG

[car]
|==|
|@@|
--- mask
GGGG

[back] mirror
";

#[test]
fn every_builtin_train_parses_and_is_consistent() {
    for (name, src) in BUILTIN {
        let v = parse(src).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(&v.name, name, "file name and `name:` agree");
        assert!(v.speed >= 1);
        for part in [Some(&v.front), Some(&v.car), v.back.as_ref()]
            .into_iter()
            .flatten()
        {
            let (w, h) = (part.width(), part.height());
            assert!(w > 0 && h > 0, "{name}");
            for f in &part.frames {
                assert_eq!(
                    (f.width(), f.height()),
                    (w, h),
                    "{name}: frames share a size"
                );
                assert!(
                    f.rows.iter().all(|r| r.len() == w),
                    "{name}: rows are padded"
                );
            }
        }
        // Art is ASCII, so every character is one column wide.
        assert!(
            v.front.frames[0]
                .rows
                .iter()
                .flatten()
                .all(|c| c.ch.is_ascii()),
            "{name}"
        );
        assert!(builtin(name).is_some());
    }
    assert!(builtin("nope").is_none());
}

#[test]
fn parses_header_palette_frames_and_mirror() {
    let v = parse(MINI).unwrap();
    assert_eq!(
        (v.name.as_str(), v.title.as_str(), v.title_ja.as_str()),
        ("mini", "mini train", "mini train")
    );
    assert_eq!((v.speed, v.cars, v.wheel.clone()), (3, 2, vec!['a', 'b']));
    assert_eq!(v.front.frames.len(), 2);
    assert_eq!((v.front.width(), v.front.height()), (4, 2));
    let first = &v.front.frames[0].rows;
    assert_eq!(first[0][1].fg, Some(Rgb(255, 255, 255)));
    assert_eq!(first[0][1].bg, None);
    assert_eq!(first[1][0].bg, Some(Rgb(0, 255, 0)));
    // The car's second row has no mask row: no color.
    assert_eq!(v.car.frames[0].rows[1][0].fg, None);
    // The back mirrors the front, swapping directional characters.
    let back: String = v.back.as_ref().unwrap().frames[0].rows[1]
        .iter()
        .map(|c| c.ch)
        .collect();
    assert_eq!(back, "|__\\");
    let back_top: String = v.back.as_ref().unwrap().frames[1].rows[0]
        .iter()
        .map(|c| c.ch)
        .collect();
    assert_eq!(back_top, "@@/ ");
}

#[test]
fn art_like_a_section_header_is_art() {
    let src = "name: x\n[front]\n[car]x\n[==]\n--- mask\n[car]\n##\n";
    let v = parse(src).unwrap();
    let row: String = v.front.frames[0].rows[1].iter().map(|c| c.ch).collect();
    assert_eq!(row, "[==]  "); // padded to the widest row, "[car]x"
    assert_eq!(v.car.frames[0].rows[0][0].ch, '#');
}

#[test]
fn colors() {
    assert_eq!(parse_hex("#1f9B6b"), Some(Rgb(0x1f, 0x9b, 0x6b)));
    assert_eq!(parse_hex("1f9b6b"), None);
    assert_eq!(parse_hex("#1f9b6"), None);
    let v = parse("name: x\ncolor B: on #102030\n[front]\nab\n--- mask\nB\n[car]\nc\n").unwrap();
    assert_eq!(v.front.frames[0].rows[0][0].bg, Some(Rgb(0x10, 0x20, 0x30)));
    assert_eq!(v.front.frames[0].rows[0][0].fg, None);
    assert_eq!(v.front.frames[0].rows[0][1].bg, None);
}

fn error_of(src: &str) -> String {
    parse(src).unwrap_err().to_string()
}

#[test]
fn reports_mistakes_with_a_line_number() {
    assert_eq!(
        error_of("title: x\n[front]\na\n[car]\nb\n"),
        "missing `name`"
    );
    assert_eq!(error_of("name: x\n[car]\nb\n"), "missing [front]");
    assert_eq!(error_of("name: x\n[front]\na\n"), "missing [car]");
    assert_eq!(
        error_of("name: x\n[front]\nab\n--- mask\nZ\n[car]\nb\n"),
        "line 5: mask letter 'Z' has no color"
    );
    assert_eq!(
        error_of("name: x\n[front]\na\n--- mask\n.\nX\n[car]\nb\n"),
        "line 3: the mask has 2 rows but the art has 1"
    );
    assert_eq!(
        error_of("name: x\n[front]\na\n===\nab\n[car]\nb\n"),
        "line 5: every frame of [front] must be 1x1, this one is 2x1"
    );
    assert_eq!(
        error_of("name: x\nspeed: 0\n[front]\na\n[car]\nb\n"),
        "line 2: speed must be a number from 1 to 200"
    );
    assert_eq!(
        error_of("name: x\ncolour G: #000000\n"),
        "line 2: unknown key \"colour G\""
    );
    assert_eq!(
        error_of("name: x\n[engine]\n"),
        "line 2: unknown section \"[engine]\"; use [front], [car], [back] or [back] mirror"
    );
    assert_eq!(
        error_of("name: x\n[car] mirror\n"),
        "line 2: only [back] can be a mirror"
    );
    assert_eq!(
        error_of("name: x\ncolor G: green\n"),
        "line 2: bad color \"green\"; use #rrggbb, #rrggbb on #rrggbb or on #rrggbb"
    );
    assert_eq!(
        error_of("name: x\n[front]\nä\n[car]\nb\n"),
        "line 3: art must be printable ASCII, found 'ä'"
    );
    assert_eq!(
        error_of("name: x\n[front]\na\n[front]\nb\n"),
        "line 4: [front] appears twice"
    );
}

#[test]
fn trailing_blank_mask_rows_are_fine() {
    let v = parse("name: x\ncolor G: #000000\n[front]\nab\n--- mask\nGG\n\n\n[car]\nc\n").unwrap();
    assert_eq!(v.front.height(), 1);
}

#[test]
fn wheel_and_random_are_checked() {
    assert_eq!(
        error_of("name: x\nwheel: |遅\n[front]\na\n[car]\nb\n"),
        "line 2: wheel characters must be printable ASCII, found '遅'"
    );
    assert_eq!(
        error_of("name: x\nrandom: maybe\n[front]\na\n[car]\nb\n"),
        "line 2: random must be yes or no"
    );
    assert!(
        !parse("name: x\nrandom: no\n[front]\na\n[car]\nb\n")
            .unwrap()
            .random
    );
}

#[test]
fn only_the_maglev_opts_out_of_random_picks() {
    let random: Vec<&str> = BUILTIN
        .iter()
        .filter(|(n, _)| builtin(n).unwrap().random)
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(random, vec!["e5", "doctor-yellow", "yamanote", "steam"]);
}
