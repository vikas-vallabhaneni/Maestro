# Maestro

Personal music server: a single Rust binary that watches a folder of audio files and serves an HTTP API plus a small web UI for playback.

## Quick start

```sh
just run --library /path/to/your/music
```

Then open http://127.0.0.1:4173.

## Flags

- `--library <PATH>` (required, or `MAESTRO_LIBRARY` env) — folder of audio files to serve.
- `--host` (default `127.0.0.1`) — bind address. Use `0.0.0.0` to expose on your LAN.
- `--port` (default `4173`).
- `--data-dir <PATH>` (optional) — where the SQLite database lives. Defaults to the platform user data dir.

## API

- `GET  /api/tracks` — list tracks.
- `POST /api/scans` — trigger a rescan of the library root.
- `GET  /api/tracks/:id/stream` — stream audio (supports HTTP Range).

## Develop

```sh
just check   # fmt + clippy + tests
just fmt     # cargo fmt
```

See [`CONTEXT.md`](CONTEXT.md) for the domain language.
