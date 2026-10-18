//! Integration tests for Narad hardware profiling and universal BYOM model inspection (FR-ASR-8, ADR-002).

use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

use naradd::hardware::{
    inspect_model_directory, DetectedModelPack, HardwareError, HardwareProfile, ModelArchitecture,
};

#[test]
fn test_hardware_profile_detection_host() {
    let profile = HardwareProfile::detect();

    // Verify basic sanity of detected cores
    assert!(profile.physical_cores() >= 1, "Physical cores must be >= 1");
    assert!(
        profile.logical_cores() >= profile.physical_cores(),
        "Logical cores ({}) must be >= physical cores ({})",
        profile.logical_cores(),
        profile.physical_cores()
    );
    assert!(
        !profile.platform_arch().is_empty(),
        "Platform architecture must be detected"
    );

    // Default trait should match detect
    let default_profile = HardwareProfile::default();
    assert_eq!(default_profile.platform_arch(), profile.platform_arch());
    assert_eq!(default_profile.logical_cores(), profile.logical_cores());
    assert_eq!(default_profile.physical_cores(), profile.physical_cores());

    // Getter methods
    let _ = profile.avx2();
    let _ = profile.avx512();
    let _ = profile.amx();
    let _ = profile.neon();
    let _ = profile.total_ram_mb();
    let _ = profile.available_ram_mb();
}

#[test]
fn test_recommended_threads_synthetic_profiles() {
    // 1. Single-core constrained CPU (1 physical, 1 logical, 2GB RAM)
    let single_core = HardwareProfile::new(false, false, false, false, 1, 1, 2048, 1024, "x86_64");
    assert_eq!(single_core.recommended_threads("whisper"), 1);
    assert_eq!(single_core.recommended_threads("streaming_transducer"), 1);
    assert_eq!(single_core.recommended_threads("zipformer"), 1);
    assert_eq!(single_core.recommended_threads("ctc"), 1);
    assert_eq!(single_core.recommended_threads("moonshine"), 1);
    assert_eq!(single_core.recommended_threads("sensevoice"), 1);

    // 2. Dual-core laptop CPU (2 physical, 4 logical, 8GB RAM, 4GB available)
    let dual_core = HardwareProfile::new(true, false, false, false, 2, 4, 8192, 4096, "x86_64");
    assert_eq!(dual_core.recommended_threads("whisper"), 2);
    assert_eq!(dual_core.recommended_threads("transducer"), 2);
    assert_eq!(dual_core.recommended_threads("ctc"), 2);
    assert_eq!(dual_core.recommended_threads("moonshine"), 2);
    assert_eq!(dual_core.recommended_threads("sense_voice"), 2);

    // 3. Quad-core desktop CPU (4 physical, 8 logical, 16GB RAM, 8GB available)
    let quad_core = HardwareProfile::new(true, false, false, false, 4, 8, 16384, 8192, "x86_64");
    assert_eq!(quad_core.recommended_threads("whisper"), 4);
    assert_eq!(quad_core.recommended_threads("streaming_transducer"), 2);
    assert_eq!(quad_core.recommended_threads("zipformer"), 2);
    assert_eq!(quad_core.recommended_threads("ctc"), 2);
    assert_eq!(quad_core.recommended_threads("moonshine"), 4);
    assert_eq!(quad_core.recommended_threads("sensevoice"), 2);

    // 4. High-performance workstation CPU (16 physical, 32 logical, 64GB RAM, 32GB available)
    let workstation = HardwareProfile::new(true, true, true, false, 16, 32, 65536, 32768, "x86_64");
    assert_eq!(workstation.recommended_threads("whisper"), 8);
    assert_eq!(workstation.recommended_threads("transducer"), 4);
    assert_eq!(workstation.recommended_threads("streaming_zipformer"), 4);
    assert_eq!(workstation.recommended_threads("ctc"), 4);
    assert_eq!(workstation.recommended_threads("moonshine"), 4);
    assert_eq!(workstation.recommended_threads("sensevoice"), 4);

    // 5. Constrained RAM scenario (8 physical cores, but < 2GB available RAM)
    let low_ram = HardwareProfile::new(true, false, false, false, 8, 16, 4096, 1024, "x86_64");
    assert_eq!(
        low_ram.recommended_threads("whisper"),
        2,
        "Low RAM environment must cap threads to at most 2"
    );
    assert_eq!(
        low_ram.recommended_threads("moonshine"),
        2,
        "Low RAM environment must cap threads to at most 2"
    );

    // 6. Case insensitivity and general model fallback
    assert_eq!(quad_core.recommended_threads("WHISPER-BASE"), 4);
    assert_eq!(quad_core.recommended_threads("Whisper"), 4);
    assert_eq!(quad_core.recommended_threads("Streaming"), 2);
    assert_eq!(quad_core.recommended_threads("Zipformer-2023"), 2);
    assert_eq!(quad_core.recommended_threads("custom_asr_unknown"), 4);
}

#[test]
fn test_model_detection_architecture_1_streaming_transducer() {
    let dir = tempdir().expect("create tempdir");
    let model_path = dir.path().join("zipformer-en-streaming");
    fs::create_dir_all(&model_path).expect("create model dir");

    File::create(model_path.join("encoder.int8.onnx")).expect("file");
    File::create(model_path.join("decoder.int8.onnx")).expect("file");
    File::create(model_path.join("joiner.int8.onnx")).expect("file");
    File::create(model_path.join("tokens.txt")).expect("file");

    let pack = inspect_model_directory(&model_path).expect("inspect model directory");
    let _pack: &DetectedModelPack = &pack;
    assert_eq!(pack.architecture(), ModelArchitecture::Transducer);
    assert_eq!(pack.architecture().as_str(), "transducer");
    assert!(pack.architecture().is_streaming());
    assert!(pack.is_manifest_generated());
    assert_eq!(pack.model_id(), "zipformer-en-streaming");

    // Manifest content assertions
    let manifest = pack.manifest_content();
    assert!(manifest.contains("runtime = \"sherpa-onnx\""));
    assert!(manifest.contains("architecture = \"transducer\""));
    assert!(manifest.contains("type = \"asr\""));
    assert!(manifest.contains("encoder.int8.onnx"));
    assert!(manifest.contains("decoder.int8.onnx"));
    assert!(manifest.contains("joiner.int8.onnx"));
    assert!(manifest.contains("tokens.txt"));

    // Also verify encoder*.onnx + joiner*.onnx (without decoder)
    let dir2 = tempdir().expect("tempdir");
    let model_path2 = dir2.path().join("zipformer-encoder-joiner");
    fs::create_dir_all(&model_path2).expect("create dir");
    File::create(model_path2.join("encoder-epoch-99.onnx")).expect("file");
    File::create(model_path2.join("joiner-epoch-99.onnx")).expect("file");
    let pack2 = inspect_model_directory(&model_path2).expect("inspect");
    assert_eq!(pack2.architecture(), ModelArchitecture::Transducer);
}

#[test]
fn test_model_detection_architecture_2_streaming_ctc() {
    let dir = tempdir().expect("create tempdir");
    let model_path = dir.path().join("parakeet-ctc-110m-int8");
    fs::create_dir_all(&model_path).expect("create model dir");

    File::create(model_path.join("model.int8.onnx")).expect("file");
    File::create(model_path.join("tokens.txt")).expect("file");

    let pack = inspect_model_directory(&model_path).expect("inspect model directory");
    assert_eq!(pack.architecture(), ModelArchitecture::Ctc);
    assert_eq!(pack.architecture().as_str(), "ctc");
    assert!(pack.architecture().is_streaming());
    assert!(pack.is_manifest_generated());
    assert_eq!(pack.model_id(), "parakeet-ctc-110m-int8");

    let manifest = pack.manifest_content();
    assert!(manifest.contains("runtime = \"sherpa-onnx\""));
    assert!(manifest.contains("architecture = \"ctc\""));
    assert!(manifest.contains("model.int8.onnx"));
    assert!(manifest.contains("tokens.txt"));
}

#[test]
fn test_model_detection_architecture_3_moonshine() {
    let dir = tempdir().expect("create tempdir");
    let model_path = dir.path().join("moonshine-base-int8");
    fs::create_dir_all(&model_path).expect("create model dir");

    File::create(model_path.join("preprocess.onnx")).expect("file");
    File::create(model_path.join("encode.int8.onnx")).expect("file");
    File::create(model_path.join("uncached_decode.int8.onnx")).expect("file");
    File::create(model_path.join("tokens.txt")).expect("file");

    let pack = inspect_model_directory(&model_path).expect("inspect model directory");
    assert_eq!(pack.architecture(), ModelArchitecture::Moonshine);
    assert_eq!(pack.architecture().as_str(), "moonshine");
    assert!(!pack.architecture().is_streaming());
    assert!(pack.is_manifest_generated());
    assert_eq!(pack.model_id(), "moonshine-base-int8");

    let manifest = pack.manifest_content();
    assert!(manifest.contains("runtime = \"sherpa-onnx\""));
    assert!(manifest.contains("architecture = \"moonshine\""));
    assert!(manifest.contains("preprocess.onnx"));
    assert!(manifest.contains("encode.int8.onnx"));
}

#[test]
fn test_model_detection_architecture_4_whisper() {
    let dir = tempdir().expect("create tempdir");
    let model_path = dir.path().join("whisper-tiny-en-int8");
    fs::create_dir_all(&model_path).expect("create model dir");

    File::create(model_path.join("tiny.en-encoder.int8.onnx")).expect("file");
    File::create(model_path.join("tiny.en-decoder.int8.onnx")).expect("file");
    File::create(model_path.join("tiny.en-tokens.txt")).expect("file");

    let pack = inspect_model_directory(&model_path).expect("inspect model directory");
    assert_eq!(pack.architecture(), ModelArchitecture::Whisper);
    assert_eq!(pack.architecture().as_str(), "whisper");
    assert!(!pack.architecture().is_streaming());
    assert!(pack.is_manifest_generated());
    assert_eq!(pack.model_id(), "whisper-tiny-en-int8");

    let manifest = pack.manifest_content();
    assert!(manifest.contains("runtime = \"sherpa-onnx\""));
    assert!(manifest.contains("architecture = \"whisper\""));
    assert!(manifest.contains("tiny.en-decoder.int8.onnx"));
    assert!(manifest.contains("tiny.en-tokens.txt"));
}

#[test]
fn test_model_detection_architecture_5_sensevoice() {
    let dir = tempdir().expect("create tempdir");
    let model_path = dir.path().join("sensevoice-small-int8");
    fs::create_dir_all(&model_path).expect("create model dir");

    File::create(model_path.join("model.int8.onnx")).expect("file");
    File::create(model_path.join("bpe.model")).expect("file");

    let pack = inspect_model_directory(&model_path).expect("inspect model directory");
    assert_eq!(pack.architecture(), ModelArchitecture::SenseVoice);
    assert_eq!(pack.architecture().as_str(), "sense_voice");
    assert!(!pack.architecture().is_streaming());
    assert!(pack.is_manifest_generated());
    assert_eq!(pack.model_id(), "sensevoice-small-int8");

    let manifest = pack.manifest_content();
    assert!(manifest.contains("runtime = \"sherpa-onnx\""));
    assert!(manifest.contains("architecture = \"sense_voice\""));
    assert!(manifest.contains("model.int8.onnx"));
    assert!(manifest.contains("bpe.model"));
}

#[test]
fn test_pre_existing_manifest_preservation() {
    let dir = tempdir().expect("create tempdir");
    let model_path = dir.path().join("custom-ctc-pack");
    fs::create_dir_all(&model_path).expect("create model dir");

    File::create(model_path.join("model.onnx")).expect("file");
    File::create(model_path.join("tokens.txt")).expect("file");

    let custom_manifest = r#"id = "pinned-custom-id"
type = "asr"
runtime = "sherpa-onnx"
languages = ["en"]
files = ["model.onnx", "tokens.txt"]
"#;
    let mut manifest_file = File::create(model_path.join("manifest.toml")).expect("manifest file");
    manifest_file.write_all(custom_manifest.as_bytes()).expect("write");

    let pack = inspect_model_directory(&model_path).expect("inspect model directory");
    assert_eq!(pack.architecture(), ModelArchitecture::Ctc);
    assert!(!pack.is_manifest_generated(), "Existing manifest must not be overwritten");
    assert_eq!(pack.model_id(), "pinned-custom-id");
    assert_eq!(pack.manifest_content(), custom_manifest);
}

#[test]
fn test_error_cases() {
    let dir = tempdir().expect("tempdir");

    // 1. Non-existent directory
    let non_existent = dir.path().join("does_not_exist");
    match inspect_model_directory(&non_existent) {
        Err(HardwareError::NotADirectory(p)) => assert_eq!(p, non_existent),
        other => panic!("Expected NotADirectory, got: {:?}", other),
    }

    // 2. Empty directory
    let empty_dir = dir.path().join("empty_folder");
    fs::create_dir_all(&empty_dir).expect("create empty");
    match inspect_model_directory(&empty_dir) {
        Err(HardwareError::EmptyDirectory(p)) => assert_eq!(p, empty_dir),
        other => panic!("Expected EmptyDirectory, got: {:?}", other),
    }

    // 3. Unknown model files
    let unknown_dir = dir.path().join("unknown_folder");
    fs::create_dir_all(&unknown_dir).expect("create unknown");
    File::create(unknown_dir.join("readme.md")).expect("file");
    File::create(unknown_dir.join("data.bin")).expect("file");
    match inspect_model_directory(&unknown_dir) {
        Err(HardwareError::UnknownArchitecture(p)) => assert_eq!(p, unknown_dir),
        other => panic!("Expected UnknownArchitecture, got: {:?}", other),
    }
}
