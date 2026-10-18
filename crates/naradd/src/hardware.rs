//! Hardware profiling and universal model inspector (FR-ASR-8, ADR-002).
//!
//! Provides CPU SIMD feature detection (AVX2, AVX-512, AMX, NEON), core count profiling,
//! available memory sizing, inference thread allocation tuning, and Bring-Your-Own-Model (BYOM)
//! directory inspection with automatic manifest.toml generation.

use std::fmt;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Supported ASR model architectures detected by the inspector (ADR-002, FR-ASR-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelArchitecture {
    /// Streaming Transducer / Zipformer (encoder, decoder, joiner).
    #[serde(rename = "transducer", alias = "streaming_transducer", alias = "zipformer")]
    Transducer,
    /// Streaming CTC / Zipformer2 CTC (model, tokens).
    #[serde(rename = "ctc", alias = "streaming_ctc", alias = "zipformer2_ctc")]
    Ctc,
    /// Moonshine sliding-window encoder-decoder (preprocess, encode, uncached decode).
    #[serde(rename = "moonshine")]
    Moonshine,
    /// Whisper sequence-to-sequence model (encoder, decoder, tokens).
    #[serde(rename = "whisper")]
    Whisper,
    /// SenseVoice non-autoregressive multi-lingual model (model, bpe.model).
    #[serde(rename = "sense_voice", alias = "sensevoice")]
    SenseVoice,
}

impl ModelArchitecture {
    /// Canonical short identifier for the architecture.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Transducer => "transducer",
            Self::Ctc => "ctc",
            Self::Moonshine => "moonshine",
            Self::Whisper => "whisper",
            Self::SenseVoice => "sense_voice",
        }
    }

    /// Descriptive human-readable architecture label.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Transducer => "Streaming Transducer / Zipformer",
            Self::Ctc => "Streaming CTC / Zipformer2 CTC",
            Self::Moonshine => "Moonshine",
            Self::Whisper => "Whisper",
            Self::SenseVoice => "SenseVoice",
        }
    }

    /// Returns true if the architecture operates as a low-latency chunk streaming engine.
    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::Transducer | Self::Ctc)
    }
}

impl fmt::Display for ModelArchitecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Hardware profile containing CPU SIMD features, core topology, RAM, and platform arch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// Intel/AMD AVX2 SIMD support (256-bit vector operations).
    pub avx2: bool,
    /// Intel AVX-512 foundation / SIMD support (512-bit vector operations).
    pub avx512: bool,
    /// Intel AMX (Advanced Matrix Extensions) tile/int8/bf16 accelerator support.
    pub amx: bool,
    /// ARM Advanced SIMD / NEON vector support.
    pub neon: bool,
    /// Count of physical CPU execution cores.
    pub physical_cores: usize,
    /// Count of logical CPU threads (including hyperthreads / SMT).
    pub logical_cores: usize,
    /// Total system RAM in megabytes.
    pub total_ram_mb: u64,
    /// Available / free system RAM in megabytes.
    pub available_ram_mb: u64,
    /// Host platform architecture (e.g. "x86_64", "aarch64").
    pub platform_arch: String,
}

impl HardwareProfile {
    /// Construct a profile manually with explicit parameters (useful for testing).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        avx2: bool,
        avx512: bool,
        amx: bool,
        neon: bool,
        physical_cores: usize,
        logical_cores: usize,
        total_ram_mb: u64,
        available_ram_mb: u64,
        platform_arch: impl Into<String>,
    ) -> Self {
        let logical = logical_cores.max(1);
        let physical = physical_cores.clamp(1, logical);
        Self {
            avx2,
            avx512,
            amx,
            neon,
            physical_cores: physical,
            logical_cores: logical,
            total_ram_mb,
            available_ram_mb,
            platform_arch: platform_arch.into(),
        }
    }

    /// Detect host hardware capabilities at runtime.
    pub fn detect() -> Self {
        let mut avx2 = false;
        let mut avx512 = false;
        let mut amx = false;
        let mut neon = false;

        #[cfg(target_arch = "x86_64")]
        {
            if std::is_x86_feature_detected!("avx2") {
                avx2 = true;
            }
            if std::is_x86_feature_detected!("avx512f") {
                avx512 = true;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                neon = true;
            }
        }

        let logical_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let mut physical_cores = 0;
        let mut total_ram_mb = 0;
        let mut available_ram_mb = 0;

        #[cfg(target_os = "linux")]
        {
            detect_linux_cpuinfo(&mut avx2, &mut avx512, &mut amx, &mut neon, &mut physical_cores);
            detect_linux_meminfo(&mut total_ram_mb, &mut available_ram_mb);
        }

        if physical_cores == 0 {
            physical_cores = if logical_cores > 1 {
                logical_cores / 2
            } else {
                1
            };
        }

        // Clamp physical cores so it never exceeds logical cores and is at least 1.
        physical_cores = physical_cores.clamp(1, logical_cores);

        Self {
            avx2,
            avx512,
            amx,
            neon,
            physical_cores,
            logical_cores,
            total_ram_mb,
            available_ram_mb,
            platform_arch: std::env::consts::ARCH.to_string(),
        }
    }

    /// Calculate safe, optimal thread count for inference given a model type.
    ///
    /// Thread allocations balance throughput against cache contention and thread-barrier
    /// latency:
    /// - Streaming models (Zipformer transducer/CTC) operate on 50ms chunk frames; excessive
    ///   threads introduce synchronization penalties. Clamped to 2-4 threads.
    /// - Whisper models are compute-heavy GEMM transformers and scale up to min(physical-1, 8).
    /// - Moonshine and SenseVoice models use moderate thread allocation (2-4 threads).
    /// - Low available RAM (<2GB) caps threads to at most 2 to avoid memory bloat.
    pub fn recommended_threads(&self, model_type: &str) -> usize {
        let phys = self.physical_cores.max(1);
        let logi = self.logical_cores.max(phys);
        let model = model_type.trim().to_lowercase();

        let threads = if model.contains("whisper") {
            if phys <= 2 {
                phys
            } else if phys <= 4 {
                phys
            } else {
                (phys - 1).min(8)
            }
        } else if model.contains("moonshine") {
            if phys <= 2 {
                phys
            } else if phys <= 4 {
                phys.min(4)
            } else {
                4
            }
        } else if model.contains("transducer")
            || model.contains("zipformer")
            || model.contains("ctc")
            || model.contains("streaming")
        {
            if phys <= 2 {
                phys
            } else if phys <= 4 {
                2
            } else {
                4
            }
        } else if model.contains("sensevoice") || model.contains("sense_voice") {
            if phys <= 2 {
                phys
            } else if phys <= 4 {
                2
            } else {
                4
            }
        } else {
            // General safe default
            phys.clamp(1, 4)
        };

        // Guard against memory exhaustion under constrained RAM environments.
        let threads = if self.available_ram_mb > 0 && self.available_ram_mb < 2048 {
            threads.min(2)
        } else {
            threads
        };

        threads.clamp(1, logi)
    }

    pub fn avx2(&self) -> bool {
        self.avx2
    }

    pub fn avx512(&self) -> bool {
        self.avx512
    }

    pub fn amx(&self) -> bool {
        self.amx
    }

    pub fn neon(&self) -> bool {
        self.neon
    }

    pub fn physical_cores(&self) -> usize {
        self.physical_cores
    }

    pub fn logical_cores(&self) -> usize {
        self.logical_cores
    }

    pub fn total_ram_mb(&self) -> u64 {
        self.total_ram_mb
    }

    pub fn available_ram_mb(&self) -> u64 {
        self.available_ram_mb
    }

    pub fn platform_arch(&self) -> &str {
        &self.platform_arch
    }
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self::detect()
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_cpuinfo(
    avx2: &mut bool,
    avx512: &mut bool,
    amx: &mut bool,
    neon: &mut bool,
    physical_cores: &mut usize,
) {
    let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") else {
        return;
    };

    use std::collections::HashSet;
    let mut physical_core_set = HashSet::new();
    let mut current_socket: Option<u32> = None;
    let mut current_core: Option<u32> = None;
    let mut parsed_cpu_cores: Option<usize> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some(core) = current_core {
                let socket = current_socket.unwrap_or(0);
                physical_core_set.insert((socket, core));
            }
            current_socket = None;
            current_core = None;
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "flags" | "Features" => {
                    for flag in value.split_whitespace() {
                        match flag {
                            "avx2" => *avx2 = true,
                            f if f.starts_with("avx512") => *avx512 = true,
                            f if f.starts_with("amx_") || f == "amx" => *amx = true,
                            "neon" | "asimd" => *neon = true,
                            _ => {}
                        }
                    }
                }
                "physical id" => {
                    if let Ok(id) = value.parse::<u32>() {
                        current_socket = Some(id);
                    }
                }
                "core id" => {
                    if let Ok(id) = value.parse::<u32>() {
                        current_core = Some(id);
                    }
                }
                "cpu cores" => {
                    if let Ok(cores) = value.parse::<usize>() {
                        parsed_cpu_cores = Some(cores);
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(core) = current_core {
        let socket = current_socket.unwrap_or(0);
        physical_core_set.insert((socket, core));
    }

    if !physical_core_set.is_empty() {
        *physical_cores = physical_core_set.len();
    } else if let Some(cores) = parsed_cpu_cores {
        *physical_cores = cores;
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_meminfo(total_ram_mb: &mut u64, available_ram_mb: &mut u64) {
    let Ok(content) = std::fs::read_to_string("/proc/meminfo") else {
        return;
    };

    for line in content.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            if key == "MemTotal" {
                if let Some(kb_str) = value.split_whitespace().next() {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        *total_ram_mb = kb / 1024;
                    }
                }
            } else if key == "MemAvailable" {
                if let Some(kb_str) = value.split_whitespace().next() {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        *available_ram_mb = kb / 1024;
                    }
                }
            }
        }
    }
}

/// Metadata and file layout detected for a model directory pack (BYOM).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedModelPack {
    /// Model identifier (from manifest.toml if present, or directory name).
    pub model_id: String,
    /// Inferred architecture type.
    pub architecture: ModelArchitecture,
    /// List of model files discovered in the directory.
    pub files: Vec<String>,
    /// Content of existing manifest.toml or freshly auto-generated manifest.
    pub manifest_content: String,
    /// True if manifest.toml was missing and synthesized dynamically.
    pub is_manifest_generated: bool,
}

impl DetectedModelPack {
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn architecture(&self) -> ModelArchitecture {
        self.architecture
    }

    pub fn files(&self) -> &[String] {
        &self.files
    }

    pub fn manifest_content(&self) -> &str {
        &self.manifest_content
    }

    pub fn is_manifest_generated(&self) -> bool {
        self.is_manifest_generated
    }
}

/// Errors occurring during model folder inspection or hardware profiling.
#[derive(Debug, thiserror::Error)]
pub enum HardwareError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path '{0}' is not a directory or does not exist")]
    NotADirectory(PathBuf),

    #[error("Directory '{0}' contains no model files")]
    EmptyDirectory(PathBuf),

    #[error("Unknown or unsupported model architecture in '{0}'")]
    UnknownArchitecture(PathBuf),

    #[error("Manifest parse error: {0}")]
    ManifestParse(String),
}

/// Inspect a directory containing speech model files and auto-detect architecture.
///
/// Pattern rules:
/// - `encoder*.onnx` + `decoder*.onnx` / `joiner*.onnx` -> Streaming Transducer / Zipformer
/// - `model*.onnx` + `tokens.txt` -> Streaming CTC / Zipformer2 CTC
/// - `preprocess*.onnx` + `encode*.onnx` -> Moonshine
/// - `*decoder*.onnx` + `*tokens.txt` -> Whisper
/// - `model.onnx` + `bpe.model` -> SenseVoice
///
/// If `manifest.toml` is absent, synthesizes a valid manifest TOML string.
pub fn inspect_model_directory(dir: &Path) -> Result<DetectedModelPack, HardwareError> {
    if !dir.exists() || !dir.is_dir() {
        return Err(HardwareError::NotADirectory(dir.to_path_buf()));
    }

    let entries = std::fs::read_dir(dir).map_err(HardwareError::Io)?;
    let mut file_names: Vec<String> = Vec::new();
    let mut total_bytes: u64 = 0;

    for entry in entries {
        let entry = entry.map_err(HardwareError::Io)?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') {
                    file_names.push(name.to_string());
                    if let Ok(metadata) = entry.metadata() {
                        total_bytes += metadata.len();
                    }
                }
            }
        }
    }

    if file_names.is_empty() {
        return Err(HardwareError::EmptyDirectory(dir.to_path_buf()));
    }

    file_names.sort();
    let lowered: Vec<String> = file_names.iter().map(|f| f.to_lowercase()).collect();
    let dir_name_lower = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let has_bpe_model = lowered
        .iter()
        .any(|f| f == "bpe.model" || f.ends_with(".bpe.model") || f.ends_with("-bpe.model"));
    let has_model_onnx = lowered
        .iter()
        .any(|f| f.starts_with("model") && f.ends_with(".onnx"));
    let has_tokens = lowered
        .iter()
        .any(|f| f == "tokens.txt" || f.ends_with("tokens.txt"));
    let has_preprocess = lowered
        .iter()
        .any(|f| (f.starts_with("preprocess") || f.starts_with("preprocessor")) && f.ends_with(".onnx"));
    let has_encode = lowered
        .iter()
        .any(|f| (f.starts_with("encode") || f.starts_with("encoder")) && f.ends_with(".onnx"));
    let has_encoder_prefix = lowered
        .iter()
        .any(|f| f.starts_with("encoder") && f.ends_with(".onnx"));
    let has_decoder = lowered
        .iter()
        .any(|f| f.contains("decoder") && f.ends_with(".onnx"));
    let has_decoder_prefix = lowered
        .iter()
        .any(|f| f.starts_with("decoder") && f.ends_with(".onnx"));
    let has_joiner = lowered
        .iter()
        .any(|f| f.contains("joiner") && f.ends_with(".onnx"));
    let has_whisper_keyword = lowered.iter().any(|f| f.contains("whisper")) || dir_name_lower.contains("whisper");

    let arch = if has_bpe_model && has_model_onnx {
        // Pattern 5: model.onnx + bpe.model -> SenseVoice
        ModelArchitecture::SenseVoice
    } else if has_preprocess && has_encode {
        // Pattern 3: preprocess*.onnx + encode*.onnx -> Moonshine
        ModelArchitecture::Moonshine
    } else if has_joiner || (has_encoder_prefix && (has_decoder_prefix || has_joiner || (has_decoder && !has_whisper_keyword))) {
        // Pattern 1: encoder*.onnx + decoder*.onnx / joiner*.onnx -> Streaming Transducer / Zipformer
        ModelArchitecture::Transducer
    } else if has_decoder && has_tokens {
        // Pattern 4: *decoder*.onnx + *tokens.txt -> Whisper
        ModelArchitecture::Whisper
    } else if has_model_onnx && has_tokens {
        // Pattern 2: model*.onnx + tokens.txt -> Streaming CTC / Zipformer2 CTC
        ModelArchitecture::Ctc
    } else {
        return Err(HardwareError::UnknownArchitecture(dir.to_path_buf()));
    };

    let manifest_path = dir.join("manifest.toml");
    let (manifest_content, is_manifest_generated) = if manifest_path.is_file() {
        let content = std::fs::read_to_string(&manifest_path).map_err(HardwareError::Io)?;
        (content, false)
    } else {
        let model_id = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{}-model", arch.as_str()));
        let size_mb = (total_bytes + 1024 * 1024 - 1) / (1024 * 1024);
        let size_mb = size_mb.max(1);

        let files_formatted = file_names
            .iter()
            .map(|f| format!("    \"{}\"", f))
            .collect::<Vec<_>>()
            .join(",\n");

        let generated = format!(
            r#"id = "{id}"
type = "asr"
architecture = "{arch}"
runtime = "sherpa-onnx"
languages = ["en"]
size_mb = {size_mb}
files = [
{files}
]
"#,
            id = model_id,
            arch = arch.as_str(),
            size_mb = size_mb,
            files = files_formatted,
        );
        (generated, true)
    };

    let parsed_id = toml::from_str::<toml::Value>(&manifest_content)
        .ok()
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(|s| s.to_string())));

    let model_id = parsed_id.unwrap_or_else(|| {
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{}-model", arch.as_str()))
    });

    Ok(DetectedModelPack {
        model_id,
        architecture: arch,
        files: file_names,
        manifest_content,
        is_manifest_generated,
    })
}
