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
pub(crate) fn clear_partial_audio(dir: &Path) -> Result<(), String> {
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
