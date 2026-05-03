# Maestro

Personal music server: a single Rust binary that watches a folder of audio files, organizes them into a browsable library, and serves both an HTTP API and a small web UI for playback.

## Language

**Track**:
A single addressable audio recording in the library, identified by the BLAKE3 hash of its file's byte stream. A Track survives renames and moves; it does not survive re-encoding or any tag edit that mutates the file's bytes — those produce a new Track.
_Avoid_: song, item, audio, recording (when used loosely)

**File**:
An on-disk audio file under the Library Root. A File contains exactly one Track. The File's path is the Track's current location, not its identity.
_Avoid_: source, asset, blob

**Library**:
The set of Tracks known to the server, persisted in the on-disk SQLite database. The Library is a derived projection of the Library Root; it can lag the Library Root between Scans.
_Avoid_: collection, catalog, index

**Library Root**:
The absolute, canonicalized filesystem path passed to `--library` at startup. Immutable for the process lifetime. Serves as both the Scan walk root and the security boundary for the streaming endpoint.
_Avoid_: music folder, music dir, root path

**Scan**:
The operation of walking the Library Root, reading File tags, and updating the Library to match. May be triggered automatically at startup or requested via the API. There is no semantic distinction between the first Scan and any subsequent one. Requesting a Scan while one is in flight is a no-op that returns the in-flight Scan Run, not an error.
_Avoid_: rescan, refresh, reindex (as domain nouns; "rescan" is acceptable as user-facing English on a button label, but the domain has only Scans)

**Scan Run**:
The persistent record of a single execution of a Scan: when it started, when it finished, how many Files it processed, and any error message. Each Scan produces exactly one Scan Run, stored in the `scan_runs` table.
_Avoid_: scan job, scan instance, scan record

## Relationships

- A **Library Root** contains many **Files**
- A **File** contains exactly one **Track**
- A **Library** contains many **Tracks**
- A **Track** is bound to exactly one current **File** at any time, even if byte-identical content exists at multiple paths (last-seen path wins)
- A **Scan** produces exactly one **Scan Run**
- At most one **Scan** is in flight at any time

## Example dialogue

> **Dev:** "If I rename a file, does the Track survive?"
> **Maintainer:** "Yes — Track identity is the BLAKE3 hash of the file's bytes, not the path. The Scan detects the rename as a move and updates the Track's bound path."
>
> **Dev:** "What if I re-rip the same CD at a different bitrate?"
> **Maintainer:** "Different bytes, different hash, different Track. V1 has no concept of 'these two Tracks are the same recording.'"
>
> **Dev:** "What if the same audio file exists in two places under the Library Root?"
> **Maintainer:** "One Track, bound to whichever path the Scan saw most recently. The Library doesn't track multiple file locations per Track."
>
> **Dev:** "What's the difference between a Scan and a rescan?"
> **Maintainer:** "Nothing — they're the same operation. The domain has only Scans. The 'rescan' button in the UI is informal English; the API endpoint is `POST /api/scans`."

## Flagged ambiguities

- "library" was used to mean both **Library** (the set of Tracks) and **Library Root** (the on-disk folder) — resolved: these are distinct, and the Library is a derived projection of the Library Root.
- "track" was used to mean both **Track** (hash-identified entity) and **File** (path on disk) — resolved: a File contains a Track; the File can move while the Track stays the same.
- "scan" / "rescan" / "scan run" were used interchangeably — resolved: a **Scan** is the operation, a **Scan Run** is the persistent record of one execution, and "rescan" is not a domain term.
