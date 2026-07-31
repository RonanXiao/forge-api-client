//! Forge CLI — manage local Forge app data (collections, envs, cookies, history).
//! Bundled inside Forge.app; install via app menu "Install forge-cli" (symlink to PATH).

use clap::{Parser, Subcommand};
use forge_lib::storage;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "forge",
    version = env!("CARGO_PKG_VERSION"),
    about = "Forge CLI — manage Forge API client data (local, offline)",
    long_about = "Commands operate on the same data directory as the Forge desktop app \
(e.g. ~/Library/Application Support/Forge on macOS)."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print Forge data / workspace paths
    Path {
        #[arg(long)]
        workspace: bool,
    },
    /// Open data directory in the system file manager
    Open,
    /// List collections
    Collections {
        #[command(subcommand)]
        action: Option<CollectionsCmd>,
    },
    /// List environments and variables
    Env {
        #[command(subcommand)]
        action: Option<EnvCmd>,
    },
    /// History helpers
    History {
        #[command(subcommand)]
        action: Option<HistoryCmd>,
    },
    /// Cookie jar helpers
    Cookies {
        #[command(subcommand)]
        action: Option<CookiesCmd>,
    },
    /// Print app + CLI version
    Version,
}

#[derive(Subcommand)]
enum CollectionsCmd {
    /// List all collections (default)
    List,
    /// Show one collection as JSON
    Show { id: String },
    /// Delete a collection by id
    Delete { id: String },
}

#[derive(Subcommand)]
enum EnvCmd {
    /// List environments (default)
    List,
    /// Show variables for an environment (by name or id)
    Show { name_or_id: String },
}

#[derive(Subcommand)]
enum HistoryCmd {
    List {
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
    Clear,
}

#[derive(Subcommand)]
enum CookiesCmd {
    List,
    Clear,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Version => {
            println!("forge-cli {}", env!("CARGO_PKG_VERSION"));
            println!("data: {}", storage::app_data_dir()?.display());
        }
        Commands::Path { workspace } => {
            if workspace {
                println!("{}", storage::get_workspace_path()?);
            } else {
                println!("{}", storage::app_data_dir()?.display());
            }
        }
        Commands::Open => {
            let dir = storage::app_data_dir()?;
            open_path(&dir)?;
            println!("Opened {}", dir.display());
        }
        Commands::Collections { action } => match action.unwrap_or(CollectionsCmd::List) {
            CollectionsCmd::List => {
                let cols = storage::list_collections()?;
                if cols.is_empty() {
                    println!("(no collections)");
                }
                for c in cols {
                    let n = count_requests(&c.items);
                    println!("{}\t{}\t{} request(s)", c.id, c.name, n);
                }
            }
            CollectionsCmd::Show { id } => {
                let cols = storage::list_collections()?;
                let c = cols
                    .into_iter()
                    .find(|c| c.id == id || c.name == id)
                    .ok_or_else(|| format!("collection not found: {id}"))?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&c).map_err(|e| e.to_string())?
                );
            }
            CollectionsCmd::Delete { id } => {
                storage::delete_collection(&id)?;
                println!("deleted collection {id}");
            }
        },
        Commands::Env { action } => match action.unwrap_or(EnvCmd::List) {
            EnvCmd::List => {
                let file = storage::load_environments()?;
                let active = file
                    .active_id
                    .clone()
                    .or_else(|| storage::load_config().ok().and_then(|c| c.active_env_id));
                if file.environments.is_empty() {
                    println!("(no environments)");
                }
                for e in &file.environments {
                    let mark = if Some(&e.id) == active.as_ref() {
                        "*"
                    } else {
                        " "
                    };
                    let n = e.variables.iter().filter(|v| !v.key.trim().is_empty()).count();
                    println!("{mark} {}\t{}\t{} var(s)", e.id, e.name, n);
                }
            }
            EnvCmd::Show { name_or_id } => {
                let file = storage::load_environments()?;
                let env = file
                    .environments
                    .iter()
                    .find(|e| e.id == name_or_id || e.name == name_or_id)
                    .ok_or_else(|| format!("environment not found: {name_or_id}"))?;
                for v in &env.variables {
                    if v.key.trim().is_empty() {
                        continue;
                    }
                    let on = if v.enabled { "on" } else { "off" };
                    println!("{}\t{}\t[{on}]", v.key, v.value);
                }
            }
        },
        Commands::History { action } => match action.unwrap_or(HistoryCmd::List { limit: 20 }) {
            HistoryCmd::List { limit } => {
                let hist = storage::load_history()?;
                for h in hist.into_iter().take(limit) {
                    println!(
                        "{}\t{}\t{}\t{}",
                        h.timestamp,
                        h.method,
                        h.status.map(|s| s.to_string()).unwrap_or_else(|| "-".into()),
                        h.url
                    );
                }
            }
            HistoryCmd::Clear => {
                storage::clear_history()?;
                println!("history cleared");
            }
        },
        Commands::Cookies { action } => match action.unwrap_or(CookiesCmd::List) {
            CookiesCmd::List => {
                let jar = storage::load_cookies()?;
                if jar.is_empty() {
                    println!("(no cookies)");
                }
                for c in jar {
                    println!("{}\t{}\t{}{}", c.domain, c.name, c.path, if c.secure { "\tsecure" } else { "" });
                }
            }
            CookiesCmd::Clear => {
                storage::save_cookies(&[])?;
                println!("cookies cleared");
            }
        },
    }
    Ok(())
}

fn count_requests(items: &[forge_lib::models::CollectionItem]) -> usize {
    let mut n = 0;
    for it in items {
        match it.item_type.as_str() {
            "request" => n += 1,
            "folder" => {
                if let Some(ref ch) = it.children {
                    n += count_requests(ch);
                }
            }
            _ => {
                if it.request.is_some() {
                    n += 1;
                } else if let Some(ref ch) = it.children {
                    n += count_requests(ch);
                }
            }
        }
    }
    n
}

fn open_path(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .status()
            .map_err(|e| format!("open failed: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|e| format!("xdg-open failed: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .status()
            .map_err(|e| format!("explorer failed: {e}"))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = path;
        Err("open not supported on this platform".into())
    }
}
