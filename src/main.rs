mod power;

use clap::{Parser, Subcommand};
use power::{AssertionType, PowerAssertion};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// cocaine - Keep your machine awake. Like caffeinate, but with more kick.
#[derive(Parser, Debug)]
#[command(name = "cocaine", version, about)]
struct Cli {
    /// Prevent the display from sleeping
    #[arg(short = 'd', long = "display", global = true)]
    prevent_display_sleep: bool,

    /// Prevent the system from idle sleeping
    #[arg(short = 'i', long = "idle", global = true)]
    prevent_idle_sleep: bool,

    /// Prevent the system from sleeping (even on AC power loss)
    #[arg(short = 's', long = "system", global = true)]
    prevent_system_sleep: bool,

    /// Duration in seconds (0 = indefinite)
    #[arg(short = 't', long = "timeout", default_value_t = 0, global = true)]
    timeout: u64,

    /// Subcommand (daemon, etc.)
    #[command(subcommand)]
    subcommand: Option<Subcommands>,

    /// Command to execute (cocaine stays active while command runs)
    #[arg(trailing_var_arg = true)]
    command: Vec<String>,

    /// Internal flag: run as background worker (do not use directly)
    #[arg(long, hide = true)]
    daemon_worker: bool,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Run cocaine as a background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand, Debug)]
enum DaemonAction {
    /// Start the daemon
    Start,
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
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

    // Internal daemon worker mode — called by `daemon start`
    if cli.daemon_worker {
        run_daemon_worker(&cli);
        return;
    }

    if let Some(Subcommands::Daemon { action }) = &cli.subcommand {
        match action {
            DaemonAction::Start => daemon_start(&cli),
            DaemonAction::Stop => daemon_stop(),
            DaemonAction::Status => daemon_status(),
        }
        return;
    }

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

// ── Daemon helpers ────────────────────────────────────────────────────────────

fn pid_file_path() -> PathBuf {
    #[cfg(windows)]
    {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("cocaine").join("cocaine.pid")
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var("XDG_DATA_HOME")
            .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.local/share")))
            .unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(base).join("cocaine").join("cocaine.pid")
    }
}

fn read_pid() -> Option<u32> {
    let path = pid_file_path();
    let content = fs::read_to_string(&path).ok()?;
    content.trim().parse().ok()
}

fn write_pid(pid: u32) -> io::Result<()> {
    let path = pid_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(&path)?;
    write!(f, "{pid}")
}

fn remove_pid() {
    let _ = fs::remove_file(pid_file_path());
}

fn is_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill -0 just checks existence without sending a signal
        libc_kill(pid as i32, 0) == 0
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        // Try to open the process; simplest cross-platform check via tasklist
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        if let Ok(o) = out {
            let stdout = String::from_utf8_lossy(&o.stdout);
            return stdout.contains(&pid.to_string());
        }
        false
    }
}

#[cfg(unix)]
fn libc_kill(pid: i32, sig: i32) -> i32 {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe { kill(pid, sig) }
}

fn build_worker_args(cli: &Cli) -> Vec<String> {
    let mut args = vec!["--daemon-worker".to_string()];
    if cli.prevent_display_sleep {
        args.push("-d".to_string());
    }
    if cli.prevent_idle_sleep {
        args.push("-i".to_string());
    }
    if cli.prevent_system_sleep {
        args.push("-s".to_string());
    }
    if cli.timeout > 0 {
        args.push("-t".to_string());
        args.push(cli.timeout.to_string());
    }
    args
}

fn daemon_start(cli: &Cli) {
    if let Some(pid) = read_pid() {
        if is_running(pid) {
            eprintln!("cocaine: daemon already running (pid {pid})");
            std::process::exit(1);
        }
        remove_pid();
    }

    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cocaine"));
    let args = build_worker_args(cli);

    let child = Command::new(&exe)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match child {
        Ok(c) => {
            let pid = c.id();
            if let Err(e) = write_pid(pid) {
                eprintln!("cocaine: daemon started (pid {pid}) but failed to write PID file: {e}");
            } else {
                eprintln!("cocaine: daemon started (pid {pid})");
                eprintln!("cocaine: PID file: {}", pid_file_path().display());
            }
        }
        Err(e) => {
            eprintln!("cocaine: failed to start daemon: {e}");
            std::process::exit(1);
        }
    }
}

fn daemon_stop() {
    let Some(pid) = read_pid() else {
        eprintln!("cocaine: daemon is not running (no PID file found)");
        std::process::exit(1);
    };

    if !is_running(pid) {
        eprintln!("cocaine: daemon (pid {pid}) is no longer running, cleaning up");
        remove_pid();
        return;
    }

    #[cfg(unix)]
    {
        let ret = libc_kill(pid as i32, 15); // SIGTERM
        if ret == 0 {
            eprintln!("cocaine: daemon stopped (pid {pid})");
            remove_pid();
        } else {
            eprintln!("cocaine: failed to stop daemon (pid {pid})");
            std::process::exit(1);
        }
    }
    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();
        match status {
            Ok(s) if s.success() => {
                eprintln!("cocaine: daemon stopped (pid {pid})");
                remove_pid();
            }
            _ => {
                eprintln!("cocaine: failed to stop daemon (pid {pid})");
                std::process::exit(1);
            }
        }
    }
}

fn daemon_status() {
    match read_pid() {
        Some(pid) if is_running(pid) => {
            eprintln!("cocaine: daemon is running (pid {pid})");
            eprintln!("cocaine: PID file: {}", pid_file_path().display());
        }
        Some(pid) => {
            eprintln!("cocaine: daemon (pid {pid}) is not running (stale PID file)");
            std::process::exit(1);
        }
        None => {
            eprintln!("cocaine: daemon is not running");
            std::process::exit(1);
        }
    }
}

fn run_daemon_worker(cli: &Cli) {
    let assertion_types = resolve_assertion_types(cli);
    let assertions = create_assertions(&assertion_types);

    if assertions.is_empty() {
        std::process::exit(1);
    }

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

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(500));
        if let Some(t) = timeout {
            if start.elapsed() >= t {
                break;
            }
        }
    }

    remove_pid();
}
