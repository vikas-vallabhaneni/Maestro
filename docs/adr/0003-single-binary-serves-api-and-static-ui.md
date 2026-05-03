# Single Rust binary serves both the HTTP API and the static web UI

Maestro is one Rust binary. It hosts the JSON API at `/api/*` and serves the web UI's static assets (HTML, CSS, JS, vendored libraries) at `/` and `/assets/*` from a sibling `web/` directory via `tower_http::services::ServeDir`. There is no separate frontend project, no `package.json`, no Vite/webpack/build pipeline, no CDN, and no CORS layer.

## Considered alternatives

- **Separate frontend project (Vite + React/Svelte/Solid) deployed alongside the Rust API.** The dominant 2025 shape for "Rust backend + interactive frontend." Rejected for V1 because: a build pipeline is overhead felt three times in a one-day budget (`npm install` in CI, `npm run build` before testing, "is this a build error or a runtime error?" in debugging); a separate project means a CORS layer or a reverse-proxy story; and the V1 UI is ~150 lines of code, a scale at which a build pipeline does not pay for itself. We pay this cost when the UI is genuinely large, not before.

- **Embed the web assets into the binary at compile time (`include_dir!` / `rust-embed`).** Lets us ship one literal binary file. Rejected for V1 because every CSS tweak requires a recompile, which is a friction tax during the day-one build that costs more than the single-binary distribution benefit returns at this stage. The architecture is compatible with this — flipping `ServeDir::new("web")` to an embedded variant is a 5-line change. We do that in V1.1+ when distribution actually matters.

- **Write the UI in plain HTML/JS with no framework, no build step, and no embedded assets.** Considered. Rejected because the UI has component-shaped state ("which album is selected, which Track is playing") and hand-syncing the DOM in three places does not satisfy the "good code" constraint at this size. Preact + `htm` (vendored, no build step) hits the sweet spot.

## Consequences

- The API and the UI evolve together. Every commit that touches one can touch the other; there is no cross-repo coordination, no API-version drift between client and server.
- No CORS configuration is needed. The browser sees one origin.
- The V1→V2 path to a Tauri (or Electron) desktop client is straightforward: Tauri wraps the *same* `web/` directory and talks to the *same* binary's API over `127.0.0.1`. The desktop client is purely additive — it does not require splitting the project.
- Deployment is one artifact: `target/release/maestro` plus the `web/` directory. Once the embedding flip happens in V1.1+, deployment is a single file.
- A future maintainer who feels the urge to "modernize" by extracting a frontend project should read this ADR, then weigh: has the UI grown past the point where a build pipeline pays for itself (~500+ lines), and is the cross-repo coordination cost worth eating? If both yes, supersede this ADR. If either no, keep the single-binary shape.
