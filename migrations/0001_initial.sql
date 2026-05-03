CREATE TABLE IF NOT EXISTS tracks (
    id          BLOB    PRIMARY KEY NOT NULL,
    path        TEXT    UNIQUE NOT NULL,
    size        INTEGER NOT NULL,
    mtime       INTEGER NOT NULL,
    duration_ms INTEGER,
    title       TEXT,
    artist      TEXT,
    album       TEXT,
    track_no    INTEGER,
    added_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_tracks_artist_album_trackno
    ON tracks (artist, album, track_no);

CREATE TABLE IF NOT EXISTS scan_runs (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    finished_at    TEXT,
    files_seen     INTEGER NOT NULL DEFAULT 0,
    files_new      INTEGER NOT NULL DEFAULT 0,
    files_moved    INTEGER NOT NULL DEFAULT 0,
    error_message  TEXT
);
