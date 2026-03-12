mod power;

use clap::Parser;
use power::{AssertionType, PowerAssertion};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// cocaine - Keep your machine awake. Like caffeinate, but with more kick.
#[derive(Parser, Debug)]
#[command(name = "cocaine", version, about)]
struct Cli {
    /// Prevent the display from sleeping
    #[arg(short = 'd', long = "display")]
    prevent_display_sleep: bool,

    /// Prevent the system from idle sleeping
    #[arg(short = 'i', long = "idle")]
    prevent_idle_sleep: bool,

    /// Prevent the system from sleeping (even on AC power loss)
    #[arg(short = 's', long = "system")]
    prevent_system_sleep: bool,

    /// Duration in seconds (0 = indefinite)
    #[arg(short = 't', long = "timeout", default_value_t = 0)]
    timeout: u64,

    /// Command to execute (cocaine stays active while command runs)
    #[arg(trailing_var_arg = true)]
    command: Vec<String>,
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:02}m {:02}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

fn main() {
    let cli = Cli::parse();

    let assertion_types = resolve_assertion_types(&cli);
    let assertions = create_assertions(&assertion_types);

    if assertions.is_empty() {
        eprintln!("cocaine: no power assertions could be created, exiting.");
        std::process::exit(1);
    }

    print_status(&assertions);

    if !cli.command.is_empty() {
        run_with_command(&cli.command, assertions);
    } else {
        run_until_stopped(&cli, assertions);
    }
}

fn resolve_assertion_types(cli: &Cli) -> Vec<AssertionType> {
    let mut types = Vec::new();

    // Windows'da birden fazla type için SetThreadExecutionState
    // ayrı ayrı çağrılınca flag'ler overwrite oluyor.
    // Bu yüzden display + system kullanılırsa, tek bir
    // "PreventIdleDisplaySleep" type'ı olarak birleştir.
    let has_display = cli.prevent_display_sleep;
    let has_idle = cli.prevent_idle_sleep;
    let has_system = cli.prevent_system_sleep;

    if has_display {
        // Display isteniyorsa, display + system + idle'ı birleştir
        // (ES_DISPLAY_REQUIRED hepsini kapsar aslında)
        types.push(AssertionType::PreventIdleDisplaySleep);
    } else if has_system {
        types.push(AssertionType::PreventSystemSleep);
    } else if has_idle {
        types.push(AssertionType::PreventIdleSystemSleep);
    } else {
        // Default: idle system sleep
        types.push(AssertionType::PreventIdleSystemSleep);
    }

    types
}

fn create_assertions(types: &[AssertionType]) -> Vec<PowerAssertion> {
    let mut assertions = Vec::new();

    for &t in types {
        match PowerAssertion::new(t, "cocaine: keeping your machine awake") {
            Ok(a) => assertions.push(a),
            Err(e) => eprintln!("cocaine: warning: could not prevent {t}: {e}"),
        }
    }

    assertions
}

fn print_status(assertions: &[PowerAssertion]) {
    let labels: Vec<String> = assertions
        .iter()
        .map(|a| format!("{}", a.assertion_type()))
        .collect();
    eprintln!(
        "cocaine: preventing {} — Ctrl+C to stop",
        labels.join(", ")
    );
}

fn run_with_command(command: &[String], _assertions: Vec<PowerAssertion>) {
    let status = Command::new(&command[0])
        .args(&command[1..])
        .status();

    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(1);
            eprintln!("cocaine: command exited with code {code}");
            std::process::exit(code);
        }
        Err(e) => {
            eprintln!("cocaine: failed to run command '{}': {e}", command[0]);
            std::process::exit(127);
        }
    }
}

fn run_until_stopped(cli: &Cli, _assertions: Vec<PowerAssertion>) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("cocaine: failed to set Ctrl+C handler");

    let start = Instant::now();
    let timeout = if cli.timeout > 0 {
        Some(Duration::from_secs(cli.timeout))
    } else {
        None
    };

    if let Some(t) = timeout {
        eprintln!("cocaine: will stop after {}", format_duration(t));
    }

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(250));

        if let Some(t) = timeout {
            if start.elapsed() >= t {
                break;
            }
        }
    }

    let elapsed = start.elapsed();
    eprintln!("cocaine: stopped after {}", format_duration(elapsed));
}
