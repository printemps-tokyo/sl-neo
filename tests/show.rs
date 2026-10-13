use sl_neo::show::{delay_message, Lang, Rng, Scene, AFTER_STEPS, FLY_SLOPE};
use sl_neo::vehicle::{builtin, parse};

fn scene(v: &sl_neo::vehicle::Vehicle) -> Scene<'_> {
    Scene {
        vehicle: v,
        cars: v.cars,
        speed: v.speed,
        fly: false,
        board: None,
        after: None,
    }
}

#[test]
fn the_train_enters_from_the_right_and_leaves_on_the_left() {
    let v = parse("name: t\n[front]\nabc\n[car]\nd\n").unwrap();
    let s = Scene {
        cars: 0,
        speed: 2,
        ..scene(&v)
    };
    assert_eq!(s.x(0, 10), 10);
    assert_eq!(s.x(3, 10), 4);
    assert!(!s.passed(6, 10)); // x = -2, still one column on screen
    assert!(s.passed(7, 10)); // x = -4
    let g = s.render(4, 10, 3, None);
    let row: String = (0..10).map(|x| g.get(x, 1).ch).collect();
    assert_eq!(row, "  abc     ");
}

#[test]
fn flying_trains_climb_and_the_board_keeps_its_row() {
    let v = builtin("steam").unwrap();
    let h = v.front.height();
    let mut s = scene(&v);
    let base = s.y(0, 40, h);
    s.fly = true;
    assert_eq!(s.y(FLY_SLOPE * 3, 40, h), base - 3);
    s.fly = false;
    s.board = Some("x".into());
    assert!(s.y(0, 40, h) >= 2);
    assert!(
        s.y(0, 5, h) >= 2,
        "a small screen clips the train, not the board"
    );
}

#[test]
fn delay_board_scrolls_on_the_top_row() {
    let v = builtin("e5").unwrap();
    let msg = delay_message(&v, 7, Lang::Ja);
    assert!(msg.contains("E5系風の新幹線が約7分遅れて"));
    assert!(delay_message(&v, 7, Lang::En)
        .contains("The E5-style shinkansen is running about 7 min late"));
    let s = Scene {
        board: Some("NEWS".into()),
        ..scene(&v)
    };
    let g = s.render(3, 20, 20, None);
    let top: String = (0..20).map(|x| g.get(x, 0).ch).collect();
    assert_eq!(top, "                 NEW");
    assert!(g.get(0, 0).bg.is_some(), "the whole row is the board");
}

#[test]
fn closing_message_after_the_maglev() {
    let v = builtin("maglev").unwrap();
    assert!(v.after_ja.as_deref().unwrap().contains("リニア"));
    let s = Scene {
        after: v.after.clone(),
        ..scene(&v)
    };
    assert_eq!(s.after_steps(), AFTER_STEPS);
    let g = s.render(100, 60, 10, Some(0));
    let mid: String = (0..60).map(|x| g.get(x, 5).ch).collect();
    assert!(mid.contains("That was the maglev"), "{mid}");
    assert_eq!(scene(&v).after_steps(), 0);
}

#[test]
fn tiny_or_empty_screens_do_not_panic() {
    let v = builtin("e5").unwrap();
    let s = Scene {
        board: Some("遅延".into()),
        after: Some("x".into()),
        fly: true,
        ..scene(&v)
    };
    for (w, h) in [(0, 0), (1, 1), (3, 2), (200, 3)] {
        for step in [0, 1, 50, 500] {
            s.render(step, w, h, None);
            s.render(step, w, h, Some(1));
        }
    }
}

#[test]
fn language_from_the_environment() {
    let env = |pairs: &'static [(&'static str, &'static str)]| {
        move |k: &str| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    };
    assert_eq!(Lang::from_env(env(&[("LANG", "ja_JP.UTF-8")])), Lang::Ja);
    assert_eq!(
        Lang::from_env(env(&[("LC_ALL", "en_US.UTF-8"), ("LANG", "ja_JP.UTF-8")])),
        Lang::En
    );
    assert_eq!(
        Lang::from_env(env(&[("LC_ALL", ""), ("LANG", "ja_JP.UTF-8")])),
        Lang::Ja
    );
    assert_eq!(Lang::from_env(env(&[])), Lang::En);
}

#[test]
fn rng_is_repeatable() {
    let (mut a, mut b) = (Rng::new(42), Rng::new(42));
    for _ in 0..10 {
        assert_eq!(a.below(7), b.below(7));
    }
    assert!((0..1000).all(|_| a.below(5) < 5));
}
