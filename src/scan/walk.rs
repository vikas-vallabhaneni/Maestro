use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "ogg", "opus", "wav"];
const MAX_FILE_SIZE: u64 = 1_073_741_824; // 1 GiB

fn should_skip_entry(name: &str) -> bool {
    name.starts_with('.')
        || Path::new(name).extension().is_some_and(|ext| {
            ext.eq_ignore_ascii_case("musiclibrary") || ext.eq_ignore_ascii_case("app")
        })
        || name == "@eaDir"
}

#[must_use]
pub fn audio_files(root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(16)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !should_skip_entry(&name)
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        })
        .filter(|e| {
            let size = e.metadata().map_or(0, |m| m.len());
            if size == 0 {
                tracing::warn!(path = %e.path().display(), "skipping zero-byte file");
                return false;
            }
            if size > MAX_FILE_SIZE {
                tracing::warn!(path = %e.path().display(), size, "skipping file larger than 1 GiB");
                return false;
            }
            true
        })
        .map(walkdir::DirEntry::into_path)
        .collect()
}
