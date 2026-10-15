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

#[derive(Debug, Subcommand)]
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
        /// Text query. `--query` is an alias of `--search`.
        #[arg(long = "search", visible_alias = "query", default_value = "")]
        query: String,
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long, default_value_t = DEFAULT_HISTORY_LIMIT)]
        limit: u32,
        /// Print a JSON array instead of the table
        #[arg(long)]
        json: bool,
    },
    /// Newest history row
    Last,
    /// Transcribe one audio or video file
    Transcribe {
        #[arg(value_name = "FILE")]
        path: String,
        /// Empty uses the current session mode
        #[arg(long, default_value = "")]
        mode: String,
        /// Destination language; non-empty is refused until Phase 3
        #[arg(long, default_value = "")]
        to: String,
    },
    /// Delete history entries by id
    DeleteEntries {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<i64>,
    },
    /// Re-insert a history entry into the focused app
    Reinsert { id: i64 },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Commands::Transcribe { to, .. } = &cli.command {
        refuse_translation(to)?;
    }
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
            json,
        } => {
            let rows = proxy
                .history(&query, offset, limit, "", "", "", "", "")
                .await
                .context("History")?;
            if json {
                println!("{}", history_json(&rows));
            } else {
                print_history(&rows);
            }
        }
        Commands::Last => {
            let rows = proxy
                .history("", 0, 1, "", "", "", "", "")
                .await
                .context("History")?;
            print_history(&rows);
        }
        Commands::Transcribe { path, mode, .. } => {
            let id = proxy
                .transcribe(&path, mode.trim(), "")
                .await
                .context("Transcribe")?;
            println!("transcribed {id}");
        }
        Commands::DeleteEntries { ids } => {
            proxy.delete_entries(&ids).await.context("DeleteEntries")?;
            println!(
                "deleted {} entr{}",
                ids.len(),
                if ids.len() == 1 { "y" } else { "ies" }
            );
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

fn refuse_translation(to: &str) -> Result<()> {
    if !to.trim().is_empty() {
        bail!("translation is a Phase 3 feature");
    }
    Ok(())
}

fn history_json(rows: &[HistoryEntry]) -> String {
    let mut out = String::from("[");
    for (i, entry) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_history_entry(&mut out, entry);
    }
    out.push(']');
    out
}

fn push_history_entry(out: &mut String, entry: &HistoryEntry) {
    out.push('{');
    push_json_key(out, "id");
    out.push_str(&entry.id.to_string());
    push_json_key(out, "created_at");
    out.push_str(&entry.created_at.to_string());
    push_json_key(out, "duration_ms");
    out.push_str(&entry.duration_ms.to_string());
    push_json_key(out, "mode");
    push_json_string(out, &entry.mode);
    push_json_key(out, "src_lang");
    push_json_string(out, &entry.src_lang);
    push_json_key(out, "dst_lang");
    push_json_string(out, &entry.dst_lang);
    push_json_key(out, "asr_model");
    push_json_string(out, &entry.asr_model);
    push_json_key(out, "mt_model");
    push_json_opt(out, &entry.mt_model);
    push_json_key(out, "llm_model");
    push_json_opt(out, &entry.llm_model);
    push_json_key(out, "app_id");
    push_json_opt(out, &entry.app_id);
    push_json_key(out, "raw_text");
    push_json_string(out, &entry.raw_text);
    push_json_key(out, "final_text");
    push_json_string(out, &entry.final_text);
    push_json_key(out, "insert_status");
    push_json_string(out, &entry.insert_status);
    push_json_key(out, "cleanup_flagged");
    push_json_bool(out, entry.cleanup_flagged);
    push_json_key(out, "audio_path");
    push_json_opt(out, &entry.audio_path);
    push_json_key(out, "pinned");
    push_json_bool(out, entry.pinned);
    push_json_key(out, "kind");
    push_json_string(out, &entry.kind);
    push_json_key(out, "tags");
    out.push('[');
    for (i, tag) in entry.tags.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_json_string(out, tag);
    }
    out.push_str("]}");
}

fn push_json_key(out: &mut String, key: &str) {
    if !out.ends_with('{') {
        out.push(',');
    }
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn push_json_opt(out: &mut String, value: &Option<String>) {
    match value {
        Some(text) => push_json_string(out, text),
        None => out.push_str("null"),
    }
}

fn push_json_bool(out: &mut String, value: bool) {
    out.push_str(if value { "true" } else { "false" });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Commands {
        Cli::try_parse_from(args).expect("parse").command
    }

    #[test]
    fn history_search_json_and_query_alias() {
        match parse(&["narad", "history", "--search", "hello", "--json"]) {
            Commands::History { query, json, .. } => {
                assert_eq!(query, "hello");
                assert!(json);
            }
            other => panic!("expected history, got {other:?}"),
        }
        match parse(&["narad", "history", "--query", "hello"]) {
            Commands::History { query, json, .. } => {
                assert_eq!(query, "hello");
                assert!(!json);
            }
            other => panic!("expected history, got {other:?}"),
        }
    }

    #[test]
    fn history_json_one_row_matches_literal() {
        let row = HistoryEntry {
            id: 7,
            created_at: 86_400_000,
            duration_ms: 1000,
            mode: "plain".into(),
            src_lang: "en".into(),
            dst_lang: "en".into(),
            asr_model: "parakeet-ctc-110m-int8".into(),
            mt_model: None,
            llm_model: None,
            app_id: Some("org.gnome.TextEditor".into()),
            raw_text: "raw".into(),
            final_text: "say \"hi\"".into(),
            insert_status: "success".into(),
            cleanup_flagged: false,
            audio_path: Some("/tmp/7.wav".into()),
            pinned: true,
            kind: "dictation".into(),
            tags: vec!["a".into(), "A".into()],
        };
        let expected = concat!(
            r#"[{"id":7,"created_at":86400000,"duration_ms":1000,"mode":"plain","#,
            r#""src_lang":"en","dst_lang":"en","asr_model":"parakeet-ctc-110m-int8","#,
            r#""mt_model":null,"llm_model":null,"app_id":"org.gnome.TextEditor","#,
            r#""raw_text":"raw","final_text":"say \"hi\"","insert_status":"success","#,
            r#""cleanup_flagged":false,"audio_path":"/tmp/7.wav","pinned":true,"#,
            r#""kind":"dictation","tags":["a","A"]}]"#
        );
        assert_eq!(history_json(&[row]), expected);
    }

    #[test]
    fn transcribe_to_refused_without_bus_and_mode_parsed() {
        match parse(&["narad", "transcribe", "clip.wav", "--to", "hi"]) {
            Commands::Transcribe { path, mode, to } => {
                assert_eq!(path, "clip.wav");
                assert_eq!(mode, "");
                let err = refuse_translation(&to).expect_err("phase 3");
                assert!(
                    err.to_string().contains("translation is a Phase 3 feature"),
                    "{err}"
                );
            }
            other => panic!("expected transcribe, got {other:?}"),
        }
        match parse(&["narad", "transcribe", "clip.wav"]) {
            Commands::Transcribe { path, mode, .. } => {
                assert_eq!(path, "clip.wav");
                assert_eq!(mode, "");
            }
            other => panic!("expected transcribe, got {other:?}"),
        }
        match parse(&["narad", "transcribe", "clip.wav", "--mode", "code"]) {
            Commands::Transcribe { path, mode, .. } => {
                assert_eq!(path, "clip.wav");
                assert_eq!(mode, "code");
            }
            other => panic!("expected transcribe, got {other:?}"),
        }
    }

    #[test]
    fn last_selects_last_command() {
        assert!(matches!(parse(&["narad", "last"]), Commands::Last));
    }
}
