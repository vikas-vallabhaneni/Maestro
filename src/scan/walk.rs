use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "ogg", "opus", "wav"];

#[must_use]
pub fn audio_files(root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        })
        .map(|e| e.into_path())
        .collect()
}
