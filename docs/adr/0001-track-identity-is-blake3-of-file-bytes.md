# Track identity is the BLAKE3 hash of file bytes

A **Track** in Maestro is identified by the BLAKE3 hash (truncated to 16 bytes) of its **File**'s byte stream — not by path, not by the `(artist, album, title, track_no)` tag tuple, and not by an acoustic fingerprint of the audio itself. This decision shapes the schema (`tracks.id BLOB PRIMARY KEY`), the scanner's identity-classification logic, the move-detection rule, and the dedup behaviour.

## Considered alternatives

- **Path as identity.** Simplest possible; the `tracks` row is keyed by the file path. Rejected because it makes Track identity fragile under reorganization: every rename or folder move appears as a delete-plus-new pair, which destroys `added_at` timestamps and (in any future version) play counts and playlist references. We *will* reorganize the music folder.

- **Tag tuple `(artist, album, title, track_no)` as identity.** Matches how humans think about music. Rejected because untagged files exist (live recordings, weird rips, transcoded files with stripped tags) and would either collide or be unaddressable. Tags are display data, not identity.

- **Acoustic fingerprint (Chromaprint / AcoustID) as identity.** The semantically correct answer to "is this the same recording?" — would let two byte-different rips of the same CD be recognized as the same Track. Rejected for V1 because it requires a 5+ MB shipped model or external service, costs CPU-seconds per File on initial scan, and adds a database to ship. The V1 cost is too high for a one-day build; the value pays off only when users have multiple rips of the same recording, which is a V2+ user need. This is a deliberate deferral, not a rejection of the idea.

## Consequences

- A Track survives renames and moves of its underlying File. The Scan detects a "moved" classification (same hash, different path) and updates the bound path in place.
- Two byte-identical Files at different paths under the Library Root resolve to a single Track, bound to whichever path the Scan saw most recently. The Library does not track multiple file locations per Track.
- A re-encode, a tag edit that mutates the File's byte stream, or any repair operation produces a *new* Track with a new id. Any persisted reference (a future playlist entry, a future play count) to the previous Track becomes orphaned. This is correct under our identity rule and is documented behaviour rather than a bug.
- The stat-cache shortcut (`(path, size, mtime)` match → skip rehash) is the only thing that makes content-hash identity affordable at scale. It is load-bearing for scan performance and must not be removed without a replacement.
