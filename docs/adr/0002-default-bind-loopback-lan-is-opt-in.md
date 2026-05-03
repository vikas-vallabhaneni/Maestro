# Default bind is 127.0.0.1; LAN exposure is an explicit, unauth'd opt-in

Maestro defaults `--host` to `127.0.0.1` (loopback only). Reaching the server from another device on the LAN requires the user to explicitly pass `--host 0.0.0.0` (or another non-loopback address). When they do, Maestro logs a `warn!` line at startup stating that there is no authentication and anyone on the network can read the user's music. This is the day-one security invariant for a single-user, no-auth service.

## Considered alternatives

- **Default to `0.0.0.0` (LAN-reachable out of the box).** This is what every comparable home-server tool does — Plex, Jellyfin, Navidrome, Subsonic. It is the user-friendliest default for a "personal music server you run on a home server." Rejected for V1 because Maestro on day one has no authentication: defaulting to LAN-reachable means defaulting to "anyone on your home network can read your library by visiting `http://<your-ip>:4173/`." That is a different security posture than the user typically expects from "running a binary on my laptop," and we should not silently move them from one to the other.

- **Default to `0.0.0.0` but require a `--i-know-this-is-unauth` flag the first time.** Rejected as too much friction for a personal tool. The cost (one-time confirmation) outweighs the benefit (deliberate consent) when the alternative — defaulting to loopback — gives the same consent property without the friction.

- **Default to `127.0.0.1` and be silent about non-loopback binds.** Rejected because the user who passes `--host 0.0.0.0` may not have read this ADR or the `--help` text closely. The startup `warn!` line is the cheapest possible way to surface the security trade-off at exactly the moment it becomes relevant.

## Consequences

- The streaming endpoint's path canonicalize-and-startswith guard is *not* the only line of defence; the bind address is the outer perimeter. Both must be correct, and neither alone is sufficient.
- Any V1+ feature that adds authentication (token-based, OS-keychain-backed, OAuth, etc.) supersedes this ADR or amends it. When that day comes, the default may flip to `0.0.0.0` because the auth layer has taken over the security guarantee. Until then, this ADR is the authority.
- A future maintainer tempted to "make Maestro work like Plex out of the box" by flipping the default needs to read this first and either add auth in the same PR or keep the default.
- The `--host` flag's `--help` text documents the trade-off inline. The startup log line documents it at runtime. The ADR documents it in the codebase. Three places, on purpose: each catches a different reader.
