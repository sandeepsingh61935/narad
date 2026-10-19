//! Retained dictation audio (FR-HIS-5).

use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn write_retained_wav(dir: &Path, id: i64, samples: &[f32]) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    set_mode(dir, 0o700)?;
    let final_path = dir.join(format!("{id}.wav"));
    let partial = dir.join(format!("{id}.wav.partial"));
    fs::write(&partial, pcm16_wav(samples)).map_err(|err| err.to_string())?;
    set_mode(&partial, 0o600)?;
    if let Err(err) = fs::rename(&partial, &final_path) {
        let _ = fs::remove_file(&partial);
        return Err(err.to_string());
    }
    Ok(final_path)
}

/// Remove crash leftovers named `*.wav.partial` directly in `dir`.
/// A missing directory is already clean. Finished `*.wav` files stay.
#[allow(dead_code)]
pub fn clear_partial_audio(dir: &Path) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.to_string()),
    };
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.ends_with(".wav.partial") {
            continue;
        }
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        if file_type.is_file() {
            fs::remove_file(entry.path()).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

/// When `enabled` is false, leave the row untouched. When it is true, write
/// `<id>.wav` and store its absolute path. A failed write or path update
/// removes the new file and leaves the path empty. The caller still has a row.
pub(crate) fn attach_retained_audio(
    store: &mut narad_core::store::HistoryStore,
    enabled: bool,
    dir: &Path,
    id: i64,
    samples: &[f32],
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    let path = match write_retained_wav(dir, id, samples) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(%err, id, "retained audio was not written");
            return Ok(());
        }
    };
    let Some(path_str) = path.to_str() else {
        let _ = fs::remove_file(&path);
        tracing::warn!(id, "retained audio path is not utf-8");
        return Ok(());
    };
    if let Err(err) = store.set_audio_path(id, path_str) {
        let _ = fs::remove_file(&path);
        tracing::warn!(%err, id, "retained audio path was not stored");
    }
    Ok(())
}

fn pcm16_wav(samples: &[f32]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&16_000u32.to_le_bytes());
    out.extend_from_slice(&32_000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&pcm16(*sample).to_le_bytes());
    }
    out
}

/// argv for turning any media file into PCM16 16 kHz mono WAV on stdout.
pub(crate) fn ffmpeg_wav_args(path: &str) -> Vec<String> {
    [
        "ffmpeg",
        "-nostdin",
        "-hide_banner",
        "-v",
        "error",
        "-i",
        path,
        "-ac",
        "1",
        "-ar",
        "16000",
        "-f",
        "wav",
        "pipe:1",
    ]
    .map(str::to_string)
    .to_vec()
}

/// PCM16 16 kHz mono WAV via [`decode_pcm16_wav`], otherwise ffmpeg.
/// A missing path is `no file`. A missing ffmpeg binary is `ffmpeg is not installed`.
pub(crate) fn load_pcm_f32(path: &Path) -> Result<Vec<f32>, String> {
    if !path.is_file() {
        return Err("no file".into());
    }
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    if let Ok(pcm) = decode_pcm16_wav(&bytes) {
        return Ok(pcm16_to_f32(&pcm));
    }
    let wav = ffmpeg_wav(path)?;
    let pcm = pcm16_from_ffmpeg_stdout(&wav)?;
    Ok(pcm16_to_f32(&pcm))
}

/// ffmpeg `-f wav pipe:1` writes an unknown RIFF size and a LIST chunk before `data`.
/// [`decode_pcm16_wav`] stays strict for files this daemon wrote.
fn pcm16_from_ffmpeg_stdout(bytes: &[u8]) -> Result<Vec<i16>, String> {
    if let Ok(pcm) = decode_pcm16_wav(bytes) {
        return Ok(pcm);
    }
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("ffmpeg failed".into());
    }
    let mut offset = 12usize;
    let mut format_ok = false;
    let mut data: Option<&[u8]> = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        let start = offset + 8;
        let unknown = size == u32::MAX;
        let payload_len = if unknown {
            bytes.len().saturating_sub(start)
        } else {
            size as usize
        };
        if start + payload_len > bytes.len() {
            return Err("ffmpeg failed".into());
        }
        let payload = &bytes[start..start + payload_len];
        if id == b"fmt " {
            if payload.len() < 16 {
                return Err("ffmpeg failed".into());
            }
            let audio_format = u16::from_le_bytes(payload[0..2].try_into().unwrap());
            let channels = u16::from_le_bytes(payload[2..4].try_into().unwrap());
            let rate = u32::from_le_bytes(payload[4..8].try_into().unwrap());
            let bits = u16::from_le_bytes(payload[14..16].try_into().unwrap());
            format_ok = audio_format == 1 && channels == 1 && rate == 16_000 && bits == 16;
        } else if id == b"data" {
            data = Some(payload);
            if unknown {
                break;
            }
        }
        if unknown {
            break;
        }
        offset = start + payload_len + (payload_len % 2);
    }
    if !format_ok {
        return Err("ffmpeg failed".into());
    }
    let data = data.ok_or_else(|| "ffmpeg failed".to_string())?;
    if data.len() % 2 != 0 {
        return Err("ffmpeg failed".into());
    }
    let mut samples = Vec::with_capacity(data.len() / 2);
    for frame in data.chunks_exact(2) {
        samples.push(i16::from_le_bytes([frame[0], frame[1]]));
    }
    Ok(samples)
}

fn ffmpeg_wav(path: &Path) -> Result<Vec<u8>, String> {
    let path = path.to_str().ok_or_else(|| "no file".to_string())?;
    let args = ffmpeg_wav_args(path);
    let mut cmd = std::process::Command::new(&args[0]);
    cmd.args(&args[1..]).stdin(std::process::Stdio::null());
    match cmd.output() {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err("ffmpeg is not installed".into())
        }
        Err(err) => Err(err.to_string()),
        Ok(out) if !out.status.success() => Err("ffmpeg failed".into()),
        Ok(out) => Ok(out.stdout),
    }
}

/// Decode the PCM16 16 kHz mono WAV this daemon writes. Anything else is an error.
pub(crate) fn decode_pcm16_wav(bytes: &[u8]) -> Result<Vec<i16>, String> {
    if bytes.len() < 44 {
        return Err("wav is not pcm16 16 khz mono".into());
    }
    let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let fmt_len = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let audio_format = u16::from_le_bytes(bytes[20..22].try_into().unwrap());
    let channels = u16::from_le_bytes(bytes[22..24].try_into().unwrap());
    let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    let byte_rate = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
    let block_align = u16::from_le_bytes(bytes[32..34].try_into().unwrap());
    let bits = u16::from_le_bytes(bytes[34..36].try_into().unwrap());
    let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    let header_ok = &bytes[0..4] == b"RIFF"
        && &bytes[8..12] == b"WAVE"
        && &bytes[12..16] == b"fmt "
        && fmt_len == 16
        && audio_format == 1
        && channels == 1
        && rate == 16_000
        && byte_rate == 32_000
        && block_align == 2
        && bits == 16
        && &bytes[36..40] == b"data"
        && data_len % 2 == 0
        && riff_size == 36 + data_len
        && bytes.len() == 44 + data_len;
    if !header_ok {
        return Err("wav is not pcm16 16 khz mono".into());
    }
    let mut samples = Vec::with_capacity(data_len / 2);
    for frame in bytes[44..].chunks_exact(2) {
        samples.push(i16::from_le_bytes([frame[0], frame[1]]));
    }
    Ok(samples)
}

pub(crate) fn pcm16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| (*sample as f32 / 32767.0).clamp(-1.0, 1.0))
        .collect()
}

fn pcm16(sample: f32) -> i16 {
    let finite = if sample.is_finite() { sample } else { 0.0 };
    (finite.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|err| err.to_string())?;
    }
    let _ = (path, mode);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_args_pcm_decode_and_missing_ffmpeg() {
        assert_eq!(
            ffmpeg_wav_args("/tmp/clip.mp3"),
            [
                "ffmpeg",
                "-nostdin",
                "-hide_banner",
                "-v",
                "error",
                "-i",
                "/tmp/clip.mp3",
                "-ac",
                "1",
                "-ar",
                "16000",
                "-f",
                "wav",
                "pipe:1",
            ]
            .map(str::to_string)
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let wav = write_retained_wav(dir.path(), 1, &[0.5]).expect("wav");
        let samples = load_pcm_f32(&wav).expect("pcm");
        assert_eq!(samples[0], 16384.0 / 32767.0);

        if std::env::var_os("NARAD_ASSERT_NO_FFMPEG").is_some() {
            let mp3 = dir.path().join("clip.mp3");
            std::fs::write(&mp3, b"not a wav").expect("mp3");
            let err = load_pcm_f32(&mp3).expect_err("no ffmpeg");
            assert!(err.contains("ffmpeg is not installed"), "{err}");
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().expect("exe"))
            .env("PATH", "/nonexistent")
            .env("NARAD_ASSERT_NO_FFMPEG", "1")
            .args([
                "--exact",
                "retained::tests::ffmpeg_args_pcm_decode_and_missing_ffmpeg",
                "--test-threads=1",
            ])
            .output()
            .expect("re-exec");
        assert!(
            output.status.success(),
            "stdout {}\nstderr {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn piped_ffmpeg_wav_keeps_pcm16_after_a_list_chunk() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&16_000u32.to_le_bytes());
        bytes.extend_from_slice(&32_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"LIST");
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(b"INFO");
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&16384i16.to_le_bytes());
        let pcm = pcm16_from_ffmpeg_stdout(&bytes).expect("pcm");
        assert_eq!(pcm, vec![16384]);
        assert!(decode_pcm16_wav(&bytes).is_err());
    }

    #[test]
    fn one_half_sample_decodes_to_i16_16384() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_retained_wav(dir.path(), 7, &[0.5]).expect("wav");
        let bytes = std::fs::read(path).expect("read");
        let samples = decode_pcm16_wav(&bytes).expect("decode");
        assert_eq!(samples, vec![16384]);
    }

    #[test]
    fn one_half_sample_is_a_pcm16_wav() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_retained_wav(dir.path(), 7, &[0.5]).expect("wav");
        assert_eq!(path, dir.path().join("7.wav"));
        let bytes = std::fs::read(&path).expect("read");
        assert_eq!(
            bytes,
            [
                b'R', b'I', b'F', b'F', 38, 0, 0, 0, b'W', b'A', b'V', b'E', b'f', b'm', b't',
                b' ', 16, 0, 0, 0, 1, 0, 1, 0, 0x80, 0x3e, 0, 0, 0x00, 0x7d, 0, 0, 2, 0, 16, 0,
                b'd', b'a', b't', b'a', 2, 0, 0, 0, 0x00, 0x40,
            ]
        );
    }

    #[test]
    fn a_sample_below_minus_one_clamps_to_the_pcm_floor() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_retained_wav(dir.path(), 3, &[-2.0]).expect("wav");
        let bytes = std::fs::read(path).expect("read");
        assert_eq!(&bytes[44..], &[0x01, 0x80]);
    }

    #[test]
    fn a_non_finite_sample_is_silence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_retained_wav(dir.path(), 4, &[f32::NAN]).expect("wav");
        let bytes = std::fs::read(path).expect("read");
        assert_eq!(&bytes[44..], &[0, 0]);
    }

    #[test]
    fn the_wav_is_private_and_the_directory_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_retained_wav(dir.path(), 5, &[0.0]).expect("wav");
        let file_mode = std::fs::metadata(&path).expect("file").permissions().mode() & 0o777;
        let dir_mode = std::fs::metadata(dir.path())
            .expect("dir")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600);
        assert_eq!(dir_mode, 0o700);
    }

    #[test]
    fn startup_removes_only_partial_wavs_in_that_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let partial = dir.path().join("1.wav.partial");
        let finished = dir.path().join("2.wav");
        let other = dir.path().join("notes.txt");
        let nested = dir.path().join("nested");
        std::fs::write(&partial, b"speech").expect("partial");
        std::fs::write(&finished, b"keep").expect("wav");
        std::fs::write(&other, b"keep").expect("txt");
        std::fs::create_dir(&nested).expect("nested");
        std::fs::write(nested.join("3.wav.partial"), b"nested").expect("nested partial");

        clear_partial_audio(dir.path()).expect("clear");

        assert!(!partial.exists());
        assert_eq!(std::fs::read(&finished).expect("wav"), b"keep");
        assert_eq!(std::fs::read(&other).expect("txt"), b"keep");
        assert_eq!(
            std::fs::read(nested.join("3.wav.partial")).expect("nested"),
            b"nested"
        );
    }

    fn entry(text: &str) -> narad_core::store::NewHistoryEntry {
        narad_core::store::NewHistoryEntry {
            duration_ms: 1000,
            mode: "plain".into(),
            src_lang: "en".into(),
            dst_lang: "en".into(),
            asr_model: "parakeet-test".into(),
            mt_model: None,
            llm_model: None,
            app_id: None,
            raw_text: text.into(),
            final_text: text.into(),
            insert_status: narad_core::store::InsertStatus::Success,
            cleanup_flagged: false,
            audio_path: None,
            pinned: false,
            kind: "dictation".into(),
        }
    }

    #[test]
    fn retention_off_leaves_no_file_and_an_empty_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store =
            narad_core::store::HistoryStore::open(dir.path().join("t.db")).expect("open");
        let id = store.insert(&entry("hello")).expect("insert");

        attach_retained_audio(&mut store, false, dir.path(), id, &[0.5]).expect("attach");

        let row = store.get(id).expect("get").expect("row");
        assert!(row.audio_path.is_none());
        assert!(!dir.path().join(format!("{id}.wav")).exists());
    }

    #[test]
    fn retention_on_stores_the_absolute_wav_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let audio = dir.path().join("audio");
        let mut store =
            narad_core::store::HistoryStore::open(dir.path().join("t.db")).expect("open");
        let id = store.insert(&entry("hello")).expect("insert");

        attach_retained_audio(&mut store, true, &audio, id, &[0.5]).expect("attach");

        let row = store.get(id).expect("get").expect("row");
        let expected = audio.join(format!("{id}.wav"));
        assert_eq!(row.audio_path.as_deref(), Some(expected.to_str().unwrap()));
        let bytes = std::fs::read(&expected).expect("wav");
        assert_eq!(&bytes[44..], &[0x00, 0x40]);
    }

    #[test]
    fn a_missing_row_does_not_keep_the_new_wav() {
        let dir = tempfile::tempdir().expect("tempdir");
        let audio = dir.path().join("audio");
        let mut store =
            narad_core::store::HistoryStore::open(dir.path().join("t.db")).expect("open");

        attach_retained_audio(&mut store, true, &audio, 9, &[0.5]).expect("attach");

        assert!(store.get(9).expect("get").is_none());
        assert!(!audio.join("9.wav").exists());
    }
}
