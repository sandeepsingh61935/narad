//! Phase 0 spike: sherpa-onnx offline ASR latency harness (ROADMAP item 4).

mod bench;
mod fixtures;
mod models;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use models::{load_catalog, list_models, ModelEntry};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "spike-asr-bench")]
#[command(about = "Phase 0 sherpa-onnx ASR benchmark harness")]
struct Cli {
    /// Path to models.toml
    #[arg(long, default_value = "spikes/asr-bench/models.toml")]
    manifest: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print model catalog and download URLs.
    ListModels {
        #[arg(long, default_value = "spikes/asr-bench/models")]
        models_dir: PathBuf,
    },
    /// Generate synthetic 3s/5s/10s WAV fixtures (silence + 440 Hz tone).
    Fixtures {
        #[arg(long, default_value = "spikes/asr-bench/fixtures")]
        fixtures_dir: PathBuf,
    },
    /// Run timed offline decode; requires downloaded models.
    Run {
        #[arg(long, default_value = "spikes/asr-bench/models")]
        models_dir: PathBuf,
        #[arg(long, default_value = "spikes/asr-bench/fixtures")]
        fixtures_dir: PathBuf,
        /// Repeat run for one model id (default: all in manifest).
        #[arg(long)]
        model: Option<String>,
        #[arg(long, default_value_t = default_threads())]
        num_threads: i32,
    },
}

fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4)
        .min(8)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let catalog = load_catalog(&cli.manifest)?;

    match cli.command {
        Command::ListModels { models_dir } => {
            list_models(&catalog, &models_dir);
        }
        Command::Fixtures { fixtures_dir } => {
            let paths = fixtures::generate_all(&fixtures_dir)?;
            for p in paths {
                println!("wrote {}", p.display());
            }
        }
        Command::Run {
            models_dir,
            fixtures_dir,
            model,
            num_threads,
        } => {
            let wavs = fixtures::fixture_paths(&fixtures_dir);
            for w in &wavs {
                if !w.is_file() {
                    anyhow::bail!(
                        "missing fixture {} — run: cargo run -p spike-asr-bench -- fixtures",
                        w.display()
                    );
                }
            }

            let selected: Vec<&ModelEntry> = catalog
                .iter()
                .filter(|m| model.as_ref().map(|id| id == &m.id).unwrap_or(true))
                .collect();
            if selected.is_empty() {
                anyhow::bail!("no models matched {:?}", model);
            }

            println!("model,clip,load_ms,decode_ms,num_threads");
            for entry in selected {
                let result = bench::bench_model(entry, &models_dir, &wavs, num_threads)
                    .with_context(|| format!("bench {}", entry.id))?;
                for (clip, decode_ms) in &result.rows {
                    println!(
                        "{},{},{:.2},{:.2},{}",
                        entry.id, clip, result.cold_load_ms, decode_ms, num_threads
                    );
                }
            }
        }
    }

    Ok(())
}
