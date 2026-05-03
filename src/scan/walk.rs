use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "ogg", "opus", "wav"];

#[must_use]
pub fn audio_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let walker = walkdir::WalkDir::new(root).follow_links(false);
    for entry in walker {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let matches = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()));
        if matches {
            files.push(path.to_path_buf());
        }
    }
    files
}
