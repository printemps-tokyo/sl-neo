use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};

use sl_neo::screen::{grid_to_ansi, sprite_to_text, ColorMode};
use sl_neo::show::{delay_message, Lang, Rng, Scene};
use sl_neo::vehicle::{self, Vehicle, BUILTIN};

/// Time per step.
const STEP: Duration = Duration::from_millis(40);

/// Trains picked at random; the maglev only comes with --linear or --train maglev.
const RANDOM_POOL: &[&str] = &["e5", "doctor-yellow", "yamanote", "steam"];

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LangArg {
    Auto,
    Ja,
    En,
}

#[derive(Parser, Debug)]
#[command(
    name = "sl-neo",
    version,
    about = "A modern sl: mistype ls and a train runs across your terminal.",
    long_about = "A modern sl: mistype ls and a train runs across your terminal, in 24-bit color.\n\
                  Like the original sl, Ctrl+C does nothing until the train has passed; pass -e to allow it.\n\
                  Make it your typo with: alias sl=sl-neo",
    after_help = "Trains: e5, doctor-yellow, yamanote, steam, maglev (see --list).\n\
                  When the output is not a terminal, or with --still, the train is printed once."
)]
struct Cli {
    /// Train to run (see --list); a random one when omitted
    #[arg(short = 't', long, value_name = "NAME")]
    train: Option<String>,

    /// List the trains and exit
    #[arg(long)]
    list: bool,

    /// With --list, print only the names, one per line
    #[arg(long, requires = "list")]
    names: bool,

    /// Number of middle cars (0-20); each train has its own default
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u16).range(0..=20))]
    cars: Option<u16>,

    /// Columns per step (1-200); each train has its own default
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u16).range(1..=200))]
    speed: Option<u16>,

    /// Let the train fly
    #[arg(short = 'F', long)]
    fly: bool,

    /// Show a delay board above the train
    #[arg(short = 'd', long)]
    delay: bool,

    /// Run the maglev (same as --train maglev)
    #[arg(long, conflicts_with = "train")]
    linear: bool,

    /// Allow Ctrl+C, q or Esc to stop the train
    #[arg(short = 'e', long)]
    escape: bool,

    /// Language of the delay board and messages
    #[arg(long, value_enum, default_value = "auto")]
    lang: LangArg,

    /// Use colors
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorArg,

    /// Seed for the random train and delay (for a repeatable run)
    #[arg(long, value_name = "N")]
    seed: Option<u64>,

    /// Print the train once instead of running it
    #[arg(long)]
    still: bool,
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

fn color_mode(arg: ColorArg, is_tty: bool) -> ColorMode {
    let rich = || match env("COLORTERM").as_deref() {
        Some("truecolor") | Some("24bit") => ColorMode::TrueColor,
        _ => ColorMode::Ansi256,
    };
    match arg {
        ColorArg::Never => ColorMode::None,
        ColorArg::Always => rich(),
        ColorArg::Auto => {
            if env("NO_COLOR").is_some_and(|v| !v.is_empty()) || !is_tty {
                ColorMode::None
            } else {
                rich()
            }
        }
    }
}

fn list(names_only: bool) -> Result<()> {
    let mut out = io::stdout().lock();
    for (name, _) in BUILTIN {
        let v = vehicle::builtin(name).expect("listed");
        if names_only {
            writeln!(out, "{name}")?;
        } else {
            writeln!(out, "{:<14} {}  ({})", name, v.title, v.title_ja)?;
        }
    }
    Ok(())
}

/// Puts the terminal back however the run ends (including a panic, which unwinds).
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<TerminalGuard> {
        terminal::enable_raw_mode().context("cannot switch the terminal to raw mode")?;
        let guard = TerminalGuard;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn is_quit(k: &KeyEvent) -> bool {
    k.kind == KeyEventKind::Press
        && (matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
            || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL)))
}

/// Wait until `deadline`, reading keys. Returns true when the run should stop.
fn wait(deadline: Instant, escape: bool) -> Result<bool> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Ok(false);
        }
        if event::poll(deadline - now)? {
            // Keys are read even when they are ignored, so they do not pile
            // up and spill into the shell afterwards.
            if let Event::Key(k) = event::read()? {
                if escape && is_quit(&k) {
                    return Ok(true);
                }
            }
        }
    }
}

fn run(scene: &Scene, mode: ColorMode, escape: bool) -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut out = io::stdout();
    let mut next = Instant::now();
    let mut passed_at: Option<usize> = None;
    let mut step = 0usize;
    loop {
        let (w, h) = terminal::size().context("cannot read the terminal size")?;
        let (w, h) = (w as usize, h as usize);
        if passed_at.is_none() && scene.passed(step, w) {
            passed_at = Some(step);
        }
        let after = passed_at.map(|p| step - p);
        if after.is_some_and(|a| a >= scene.after_steps()) {
            return Ok(());
        }
        let grid = scene.render(step, w, h, after);
        out.write_all(grid_to_ansi(&grid, mode).as_bytes())?;
        out.flush()?;
        next += STEP;
        if wait(next, escape)? {
            return Ok(());
        }
        step += 1;
    }
}

fn pick(cli: &Cli, rng: &mut Rng) -> Result<Vehicle> {
    let name = if cli.linear {
        "maglev".to_string()
    } else if let Some(t) = &cli.train {
        t.clone()
    } else {
        RANDOM_POOL[rng.below(RANDOM_POOL.len() as u64) as usize].to_string()
    };
    match vehicle::builtin(&name) {
        Some(v) => Ok(v),
        None => {
            let names: Vec<&str> = BUILTIN.iter().map(|(n, _)| *n).collect();
            bail!(
                "no train named {name:?}; choose one of {}",
                names.join(", ")
            )
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.list {
        return list(cli.names);
    }
    let seed = cli.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64)
    });
    let mut rng = Rng::new(seed);
    let v = pick(&cli, &mut rng)?;
    let lang = match cli.lang {
        LangArg::Ja => Lang::Ja,
        LangArg::En => Lang::En,
        LangArg::Auto => Lang::from_env(env),
    };
    let tty = io::stdout().is_terminal();
    let mode = color_mode(cli.color, tty);
    let cars = cli.cars.map_or(v.cars, usize::from);
    let minutes = 1 + rng.below(15) as u32;
    let scene = Scene {
        vehicle: &v,
        cars,
        speed: cli.speed.map_or(v.speed, usize::from),
        fly: cli.fly,
        board: cli.delay.then(|| delay_message(&v, minutes, lang)),
        after: match lang {
            Lang::Ja => v.after_ja.clone(),
            Lang::En => v.after.clone(),
        },
    };
    if cli.still || !tty {
        let mut out = io::stdout().lock();
        if let Some(board) = &scene.board {
            writeln!(out, "{board}")?;
        }
        out.write_all(sprite_to_text(&scene.sprite(0), mode).as_bytes())?;
        return Ok(());
    }
    run(&scene, mode, cli.escape)
}
