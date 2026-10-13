//! The binary when its output is not a terminal: it prints the train once.

use std::process::Command;

fn sl(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sl-neo"))
        .args(args)
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn lists_the_trains() {
    let (ok, out, _) = sl(&["--list", "--names"]);
    assert!(ok);
    assert_eq!(out, "e5\ndoctor-yellow\nyamanote\nsteam\nmaglev\n");
    let (_, out, _) = sl(&["--list"]);
    assert!(out.contains("E5-style shinkansen  (E5系風の新幹線)"));
}

#[test]
fn prints_a_still_train_without_color_when_piped() {
    let (ok, out, _) = sl(&["--train", "steam"]);
    assert!(ok);
    assert!(
        !out.contains('\x1b'),
        "auto color is off when not a terminal"
    );
    assert!(out.contains("[==]"));
    let (_, colored, _) = sl(&["--train", "steam", "--color", "always"]);
    assert!(colored.contains("\x1b[0;"));
}

#[test]
fn the_same_seed_picks_the_same_train() {
    let a = sl(&["--seed", "7"]).1;
    assert_eq!(a, sl(&["--seed", "7"]).1);
    let picks: std::collections::HashSet<String> =
        (0..40).map(|s| sl(&["--seed", &s.to_string()]).1).collect();
    assert!(picks.len() >= 3, "random picks vary");
}

#[test]
fn delay_board_in_both_languages() {
    let (_, ja, _) = sl(&["--train", "e5", "-d", "--lang", "ja", "--seed", "1"]);
    assert!(ja.starts_with("【遅延情報】lsの打ち間違いの影響で、E5系風の新幹線が約"));
    let (_, en, _) = sl(&["--train", "e5", "-d", "--lang", "en", "--seed", "1"]);
    assert!(en.starts_with("[Service update] The E5-style shinkansen is running about "));
}

#[test]
fn rejects_unknown_trains_and_bad_numbers() {
    let (ok, _, err) = sl(&["--train", "nozomi"]);
    assert!(!ok);
    assert!(err.contains(
        "no train named \"nozomi\"; choose one of e5, doctor-yellow, yamanote, steam, maglev"
    ));
    assert!(!sl(&["--cars", "21"]).0);
    assert!(!sl(&["--speed", "0"]).0);
    assert!(!sl(&["--linear", "--train", "e5"]).0);
}

#[test]
fn linear_is_the_maglev() {
    assert_eq!(sl(&["--linear"]).1, sl(&["--train", "maglev"]).1);
}

#[test]
fn runs_a_train_from_a_file() {
    let dir = std::env::temp_dir().join(format!("sl-neo-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let good = dir.join("mini.txt");
    std::fs::write(&good, "name: mini\n[front]\n<o>\n[car]\n[ ]\n").unwrap();
    let (ok, out, _) = sl(&["--file", good.to_str().unwrap(), "--cars", "1"]);
    assert!(ok);
    assert_eq!(out, "<o> [ ]\n");
    let bad = dir.join("bad.txt");
    std::fs::write(&bad, "name: bad\n[front]\nab\n--- mask\nZ\n[car]\nc\n").unwrap();
    let (ok, _, err) = sl(&["--file", bad.to_str().unwrap()]);
    assert!(!ok);
    assert!(
        err.contains("bad.txt") && err.contains("line 5: mask letter 'Z' has no color"),
        "{err}"
    );
    assert!(!sl(&["--file", good.to_str().unwrap(), "--train", "e5"]).0);
    std::fs::remove_dir_all(&dir).unwrap();
}
