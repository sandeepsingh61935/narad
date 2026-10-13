use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use narad_dbus::{DaemonProxy, HistoryEntry};
use zbus::Connection;

const DEFAULT_MODE: &str = "plain";
const DEFAULT_HISTORY_LIMIT: u32 = 20;

#[derive(Parser)]
#[command(name = "narad", about = "Narad voice-to-text CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Daemon mode, pause flag, and HUD visibility
    Status,
    /// Begin dictation (stub session state)
    Start {
        #[arg(long, default_value = DEFAULT_MODE)]
        mode: String,
    },
    /// End dictation session
    Stop,
    /// Discard in-flight session
    Cancel,
    /// Start if idle, otherwise stop
    Toggle {
        #[arg(long, default_value = DEFAULT_MODE)]
        mode: String,
    },
    /// List history rows (optional text query)
    History {
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long, default_value_t = DEFAULT_HISTORY_LIMIT)]
        limit: u32,
    },
    /// Delete history entries by id
    DeleteEntries {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<i64>,
    },
    /// Re-insert a history entry into the focused app
    Reinsert {
        id: i64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let connection = Connection::session()
        .await
        .context("connect to session D-Bus (is naradd running?)")?;
    let proxy = DaemonProxy::new(&connection)
        .await
        .context("resolve dev.narad.Daemon on session bus")?;

    match cli.command {
        Commands::Status => {
            let mode = proxy.mode().await.context("read Mode property")?;
            let paused = proxy.paused().await.context("read Paused property")?;
            let hud_visible = proxy
                .hud_visible()
                .await
                .context("read HudVisible property")?;
            println!("mode: {mode}");
            println!("paused: {paused}");
            println!("hud_visible: {hud_visible}");
        }
        Commands::Start { mode } => {
            proxy.start(&mode).await.context("Start")?;
            println!("started (mode: {mode})");
        }
        Commands::Stop => {
            proxy.stop().await.context("Stop")?;
            println!("stopped");
        }
        Commands::Cancel => {
            proxy.cancel().await.context("Cancel")?;
            println!("cancelled");
        }
        Commands::Toggle { mode } => {
            proxy.toggle(&mode).await.context("Toggle")?;
            println!("toggled (mode: {mode})");
        }
        Commands::History {
            query,
            offset,
            limit,
        } => {
            let rows = proxy
                .history(&query, offset, limit, "", "", "", "", "")
                .await
                .context("History")?;
            print_history(&rows);
        }
        Commands::DeleteEntries { ids } => {
            proxy
                .delete_entries(&ids)
                .await
                .context("DeleteEntries")?;
            println!("deleted {} entr{}", ids.len(), if ids.len() == 1 { "y" } else { "ies" });
        }
        Commands::Reinsert { id } => match proxy.reinsert_entry(id).await {
            Ok(()) => println!("reinserted entry {id}"),
            Err(e) if is_not_supported(&e) => {
                bail!("reinsert not supported yet (insert path not ready): {e}");
            }
            Err(e) => return Err(e).context("ReinsertEntry"),
        },
    }
    Ok(())
}

fn print_history(rows: &[HistoryEntry]) {
    if rows.is_empty() {
        println!("(no entries)");
        return;
    }
    println!(
        "{:<6} {:<14} {:<8} {:<10} {:<8} {}",
        "id", "created_at", "dur_ms", "mode", "status", "text"
    );
    for e in rows {
        let text = e.final_text.replace('\n', " ");
        println!(
            "{:<6} {:<14} {:<8} {:<10} {:<8} {}",
            e.id, e.created_at, e.duration_ms, e.mode, e.insert_status, text
        );
    }
}

fn is_not_supported(err: &zbus::Error) -> bool {
    match err {
        zbus::Error::FDO(e) => matches!(e.as_ref(), zbus::fdo::Error::NotSupported(_)),
        zbus::Error::MethodError(name, _, _) => {
            name.as_str() == "org.freedesktop.DBus.Error.NotSupported"
        }
        _ => false,
    }
}
