use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agentfreeze::{
    ChangeKind, Entry, ScanOptions, ScanScope, Verification, VerificationStatus, build_lockfile,
    compare, load_lockfile, scan, write_lockfile,
};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "agentfreeze",
    version,
    about = "Record and verify supported local AI agent extension state",
    long_about = "Inventory supported Claude, Codex, Cursor, Gemini, and MCP extension state without starting it; freeze the observation in agentfreeze.lock; and report additions, removals, or content changes."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Discover installed skills, plugins, and MCP servers.
    Scan(ScanCommand),
    /// Write the current extension state to agentfreeze.lock.
    #[command(alias = "lock")]
    Freeze(FreezeCommand),
    /// Compare the current extension state with agentfreeze.lock.
    Verify(VerifyCommand),
    /// Poll the user extension environment and report drift as it changes.
    Watch(WatchCommand),
}

#[derive(Clone, Debug, Args)]
struct LocationArgs {
    /// Project directory whose local extensions should be inspected.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
    /// Home directory to inspect. Defaults to the current user's home.
    #[arg(long)]
    home: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ScanCommand {
    #[command(flatten)]
    location: LocationArgs,
    /// Inventory user, workspace, or all extension locations.
    #[arg(long, value_enum, default_value = "all")]
    scope: ScanScope,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct FreezeCommand {
    #[command(flatten)]
    location: LocationArgs,
    /// Lockfile path. Defaults to <workspace>/agentfreeze.lock.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct VerifyCommand {
    #[command(flatten)]
    location: LocationArgs,
    /// Lockfile path. Defaults to <workspace>/agentfreeze.lock.
    #[arg(short, long)]
    lockfile: Option<PathBuf>,
    /// Ignore extensions that are present but not recorded.
    #[arg(long)]
    allow_untracked: bool,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
    /// Print nothing when verification succeeds.
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Debug, Args)]
struct WatchCommand {
    #[command(flatten)]
    location: LocationArgs,
    /// Lockfile path. Defaults to <workspace>/agentfreeze.lock.
    #[arg(short, long)]
    lockfile: Option<PathBuf>,
    /// Ignore extensions that are present but not recorded.
    #[arg(long)]
    allow_untracked: bool,
    /// Seconds between scans.
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..))]
    interval: u64,
    /// Stop with exit code 1 as soon as drift is observed.
    #[arg(long)]
    exit_on_drift: bool,
    /// Stop after this many scans. Useful for schedulers and smoke tests.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    max_scans: Option<u64>,
    /// Emit newline-delimited JSON when the observed state changes.
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Scan(command) => run_scan(&command),
        Command::Freeze(command) => run_freeze(command),
        Command::Verify(command) => run_verify(command),
        Command::Watch(command) => run_watch(command),
    }
}

fn run_scan(command: &ScanCommand) -> Result<ExitCode> {
    let options = scan_options(&command.location, command.scope)?;
    let entries = scan(&options)?;
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "count": entries.len(),
                "entries": entries,
            }))
            .context("cannot serialize scan output")?
        );
    } else {
        print_inventory(&entries);
    }
    Ok(ExitCode::SUCCESS)
}

fn run_freeze(command: FreezeCommand) -> Result<ExitCode> {
    let options = scan_options(&command.location, ScanScope::User)?;
    let entries = scan(&options)?;
    let lockfile = build_lockfile(entries);
    let output = command
        .output
        .unwrap_or_else(|| options.workspace.join("agentfreeze.lock"));
    let digest = write_lockfile(&output, &lockfile)?;
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "frozen",
                "path": output,
                "entries": lockfile.entries.len(),
                "digest": digest,
            }))
            .context("cannot serialize freeze output")?
        );
    } else {
        println!(
            "Frozen {} extensions in {}",
            lockfile.entries.len(),
            output.display()
        );
        println!("Lockfile {digest}");
    }
    Ok(ExitCode::SUCCESS)
}

fn run_verify(command: VerifyCommand) -> Result<ExitCode> {
    let (path, options) = verification_context(command.location, command.lockfile)?;
    let lockfile = load_lockfile(&path)?;
    let observed = scan(&options)?;
    let verification = compare(&lockfile, &observed, command.allow_untracked);

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&verification)
                .context("cannot serialize verification output")?
        );
    } else {
        print_verification(&verification, command.quiet);
    }

    Ok(verification_exit_code(&verification))
}

fn run_watch(command: WatchCommand) -> Result<ExitCode> {
    let (path, options) = verification_context(command.location, command.lockfile)?;
    let lockfile = load_lockfile(&path)?;
    if !command.json {
        println!(
            "Watching {} user extensions every {}s — Ctrl-C to stop",
            lockfile.entries.len(),
            command.interval
        );
    }

    let mut previous = None;
    let mut scans = 0_u64;
    loop {
        let observed = scan(&options)?;
        let verification = compare(&lockfile, &observed, command.allow_untracked);
        scans += 1;
        let status = verification.status;

        if previous.as_ref() != Some(&verification) {
            if command.json {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "observedAtUnix": unix_timestamp()?,
                        "scan": scans,
                        "verification": verification,
                    }))
                    .context("cannot serialize watch output")?
                );
            } else {
                print_verification(&verification, false);
            }
            previous = Some(verification.clone());
        }

        if command.exit_on_drift && status == VerificationStatus::Drift {
            return Ok(ExitCode::from(1));
        }
        if command.max_scans.is_some_and(|maximum| scans >= maximum) {
            return Ok(match status {
                VerificationStatus::Clean => ExitCode::SUCCESS,
                VerificationStatus::Drift => ExitCode::from(1),
            });
        }
        thread::sleep(Duration::from_secs(command.interval));
    }
}

fn verification_context(
    location: LocationArgs,
    lockfile: Option<PathBuf>,
) -> Result<(PathBuf, ScanOptions)> {
    let workspace = location
        .workspace
        .canonicalize()
        .with_context(|| format!("cannot resolve workspace {}", location.workspace.display()))?;
    let path = lockfile.unwrap_or_else(|| workspace.join("agentfreeze.lock"));
    let options = scan_options(
        &LocationArgs {
            workspace,
            home: location.home,
        },
        ScanScope::User,
    )?;
    Ok((path, options))
}

fn print_verification(verification: &Verification, quiet: bool) {
    if verification.changes.is_empty() {
        if !quiet {
            println!("Verified {} extensions — no drift", verification.matched);
        }
        return;
    }

    println!(
        "Drift detected: {} change{}",
        verification.changes.len(),
        if verification.changes.len() == 1 {
            ""
        } else {
            "s"
        }
    );
    for change in &verification.changes {
        let marker = match change.kind {
            ChangeKind::Missing => "-",
            ChangeKind::Modified => "~",
            ChangeKind::Untracked => "+",
        };
        println!("  {marker} {:<9} {}", change.kind, change.id);
    }
}

fn verification_exit_code(verification: &Verification) -> ExitCode {
    if verification.changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn unix_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")
        .map(|duration| duration.as_secs())
}

fn scan_options(location: &LocationArgs, scope: ScanScope) -> Result<ScanOptions> {
    match &location.home {
        Some(home) => Ok(ScanOptions::new(
            location.workspace.clone(),
            home.clone(),
            scope,
        )),
        None => ScanOptions::from_environment(location.workspace.clone(), scope),
    }
}

fn print_inventory(entries: &[Entry]) {
    let skill_count = entries
        .iter()
        .filter(|entry| entry.kind == agentfreeze::ExtensionKind::Skill)
        .count();
    let plugin_count = entries
        .iter()
        .filter(|entry| entry.kind == agentfreeze::ExtensionKind::Plugin)
        .count();
    let mcp_count = entries
        .iter()
        .filter(|entry| entry.kind == agentfreeze::ExtensionKind::McpServer)
        .count();
    println!(
        "Found {} extensions: {skill_count} skills, {plugin_count} plugins, {mcp_count} MCP servers",
        entries.len()
    );
    for entry in entries {
        println!(
            "  {:<7} {:<10} {:<9} {:<24} {}  {}",
            entry.ecosystem,
            entry.kind,
            scope_name(entry),
            truncate(&entry.name, 24),
            short_digest(&entry.digest),
            entry.source
        );
    }
}

fn scope_name(entry: &Entry) -> &'static str {
    match entry.scope {
        agentfreeze::model::EntryScope::User => "user",
        agentfreeze::model::EntryScope::Workspace => "workspace",
    }
}

fn truncate(value: &str, width: usize) -> String {
    let mut output = value.chars().take(width).collect::<String>();
    if value.chars().count() > width {
        output.pop();
        output.push('…');
    }
    output
}

fn short_digest(digest: &str) -> &str {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    hex.get(..12).unwrap_or(hex)
}
