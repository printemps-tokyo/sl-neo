use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
                  When the output is not a terminal (or TERM=dumb), or with --still, the train is printed once."
)]
struct Cli {
    /// Train to run (see --list); a random one when omitted
    #[arg(short = 't', long, value_name = "NAME")]
    train: Option<String>,

    /// Run a train from a file in the vehicles/ format
    #[arg(long, value_name = "PATH", conflicts_with_all = ["train", "linear"])]
    file: Option<PathBuf>,

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

/// A terminal that can take cursor moves and colors: a TTY whose TERM is not "dumb".
fn capable_terminal() -> bool {
    io::stdout().is_terminal() && env("TERM").as_deref() != Some("dumb")
}

fn color_mode(arg: ColorArg, capable: bool) -> ColorMode {
    let rich = || match env("COLORTERM").as_deref() {
        Some("truecolor") | Some("24bit") => ColorMode::TrueColor,
        _ => ColorMode::Ansi256,
    };
    match arg {
        ColorArg::Never => ColorMode::None,
        ColorArg::Always => rich(),
        ColorArg::Auto => {
            if env("NO_COLOR").is_some_and(|v| !v.is_empty()) || !capable {
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

/// Puts the terminal back however the run ends: a normal end, an error, a
/// panic (which unwinds) or a termination signal (turned into a flag below).
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

/// Signals that would otherwise kill the process with the terminal still in
/// raw mode. Each sets a flag the loop checks, so the guard can restore the
/// terminal before the process exits with the usual 128 + signal status.
struct Signals {
    flags: Vec<(i32, Arc<AtomicBool>)>,
}

impl Signals {
    #[cfg(unix)]
    fn register() -> Result<Signals> {
        use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
        let mut flags = Vec::new();
        for sig in [SIGTERM, SIGHUP, SIGINT] {
            let flag = Arc::new(AtomicBool::new(false));
            signal_hook::flag::register(sig, Arc::clone(&flag))
                .with_context(|| format!("cannot handle signal {sig}"))?;
            flags.push((sig, flag));
        }
        Ok(Signals { flags })
    }

    #[cfg(not(unix))]
    fn register() -> Result<Signals> {
        Ok(Signals { flags: Vec::new() })
    }

    /// The signal that should end the run, if one arrived. SIGINT (sent by
    /// another process; the keyboard's Ctrl+C is a key in raw mode) is ignored
    /// like the original sl unless `escape` is set.
    fn pending(&self, escape: bool) -> Option<i32> {
        #[cfg(unix)]
        let int = signal_hook::consts::SIGINT;
        #[cfg(not(unix))]
        let int = 2;
        self.flags.iter().find_map(|(sig, flag)| {
            let raised = flag.swap(false, Ordering::Relaxed);
            (raised && (*sig != int || escape)).then_some(*sig)
        })
    }
}

fn is_quit(k: &KeyEvent) -> bool {
    k.kind == KeyEventKind::Press
        && (matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
            || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL)))
}

enum Stop {
    Key,
    Signal(i32),
}

/// Wait until `deadline`, reading keys and checking signals; always looks at
/// least once, even when the deadline has already passed.
fn wait(deadline: Instant, escape: bool, signals: &Signals) -> Result<Option<Stop>> {
    loop {
        if let Some(sig) = signals.pending(escape) {
            return Ok(Some(Stop::Signal(sig)));
        }
        let left = deadline.saturating_duration_since(Instant::now());
        match event::poll(left) {
            // Keys are read even when they are ignored, so they do not pile
            // up and spill into the shell afterwards.
            Ok(true) => {
                if let Event::Key(k) = event::read()? {
                    if escape && is_quit(&k) {
                        return Ok(Some(Stop::Key));
                    }
                }
            }
            Ok(false) => {}
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
    }
}

/// Run the scene. Returns the signal that stopped it, if any.
fn run(scene: &Scene, mode: ColorMode, escape: bool, signals: &Signals) -> Result<Option<i32>> {
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
            return Ok(None);
        }
        let grid = scene.render(step, w, h, after);
        out.write_all(grid_to_ansi(&grid, mode).as_bytes())?;
        out.flush()?;
        // When drawing fell behind (a slow terminal, or the process was
        // stopped), start again from now instead of drawing every missed frame.
        next = (next + STEP).max(Instant::now());
        match wait(next, escape, signals)? {
            None => {}
            Some(Stop::Key) => return Ok(None),
            Some(Stop::Signal(sig)) => return Ok(Some(sig)),
        }
        step += 1;
    }
}

fn load(cli: &Cli, rng: &mut Rng) -> Result<Vehicle> {
    if let Some(path) = &cli.file {
        // {:?} escapes the path, so a strange file name cannot drive the terminal.
        let src = std::fs::read_to_string(path).with_context(|| format!("cannot read {path:?}"))?;
        return vehicle::parse(&src).with_context(|| format!("{path:?}"));
    }
    let name = if cli.linear {
        "maglev".to_string()
    } else if let Some(t) = &cli.train {
        t.clone()
    } else {
        let pool: Vec<&str> = BUILTIN
            .iter()
            .filter(|(n, _)| vehicle::builtin(n).is_some_and(|v| v.random))
            .map(|(n, _)| *n)
            .collect();
        pool[rng.below(pool.len() as u64) as usize].to_string()
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
    let v = load(&cli, &mut rng)?;
    let lang = match cli.lang {
        LangArg::Ja => Lang::Ja,
        LangArg::En => Lang::En,
        LangArg::Auto => Lang::from_env(env),
    };
    let capable = capable_terminal();
    let mode = color_mode(cli.color, capable);
    let minutes = 1 + rng.below(15) as u32;
    let scene = Scene {
        vehicle: &v,
        cars: cli.cars.map_or(v.cars, usize::from),
        speed: cli.speed.map_or(v.speed, usize::from),
        fly: cli.fly,
        board: cli.delay.then(|| delay_message(&v, minutes, lang)),
        after: match lang {
            Lang::Ja => v.after_ja.clone(),
            Lang::En => v.after.clone(),
        },
    };
    if cli.still || !capable {
        let mut out = io::stdout().lock();
        if let Some(board) = &scene.board {
            writeln!(out, "{board}")?;
        }
        out.write_all(sprite_to_text(&scene.sprite(0), mode).as_bytes())?;
        return Ok(());
    }
    let signals = Signals::register()?;
    if let Some(sig) = run(&scene, mode, cli.escape, &signals)? {
        // The guard has restored the terminal by now.
        std::process::exit(128 + sig);
    }
    Ok(())
}
