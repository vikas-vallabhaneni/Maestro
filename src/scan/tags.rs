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

pub fn read_tags(path: &Path) -> anyhow::Result<TagSnapshot> {
    let tagged_file = lofty::read_from_path(path)?;
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

    Ok(TagSnapshot {
        title,
        artist,
        album,
        track_no,
        duration,
    })
}
