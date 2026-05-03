use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::tag::Accessor;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub struct TagSnapshot {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_no: Option<u32>,
    pub duration: Duration,
}

pub fn read_tags(path: &Path) -> TagSnapshot {
    match lofty::read_from_path(path) {
        Ok(tagged_file) => {
            let properties = tagged_file.properties();
            let duration = properties.duration();

            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());

            let (title, artist, album, track_no) = tag.map_or((None, None, None, None), |tag| {
                (
                    tag.title().map(|s| s.to_string()),
                    tag.artist().map(|s| s.to_string()),
                    tag.album().map(|s| s.to_string()),
                    tag.track(),
                )
            });

            let title = title.or_else(|| stem_fallback(path));
            let album = album.or_else(|| parent_fallback(path));
            let artist = artist.or_else(|| grandparent_fallback(path));

            TagSnapshot {
                title,
                artist,
                album,
                track_no,
                duration,
            }
        }
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "failed to read tags; inserting with fallbacks");
            TagSnapshot {
                title: stem_fallback(path),
                artist: grandparent_fallback(path),
                album: parent_fallback(path),
                track_no: None,
                duration: Duration::ZERO,
            }
        }
    }
}

fn stem_fallback(path: &Path) -> Option<String> {
    path.file_stem().and_then(|s| s.to_str()).map(String::from)
}

fn parent_fallback(path: &Path) -> Option<String> {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map(String::from)
}

fn grandparent_fallback(path: &Path) -> Option<String> {
    path.parent()
        .and_then(Path::parent)
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map(String::from)
}
