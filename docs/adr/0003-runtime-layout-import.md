# 0003. Runtime layout import from the Oryx API

- **Status:** Accepted
- **Date:** 2026-07-19
- **Deciders:** eclion, Claude

## Context

The displayed layout was baked into the binary (`ui/layout.json` embedded
at compile time), so every Oryx layout revision required rebuilding the
app.

## Decision

The settings window gets an import flow: the user pastes an Oryx layout URL
or id (revision defaults to `latest`); the backend fetches the layout from
the Oryx GraphQL endpoint (`ureq`, blocking, run via `spawn_blocking`),
validates it (must be an ergodox geometry with 76-key layers), stores it as
`layout.json` in the app config dir, and emits `layout-changed`; the
frontend re-renders. The stored file uses the exact shape of the bundled
`ui/layout.json`, which stays as the fallback (and is what "Reset to
built-in" returns to). The stored layout is reloaded at startup.

## Consequences

- Layout revisions are a paste-and-click, no toolchain needed on the
  user's machine.
- New dependency: `ureq` (rustls-based, small footprint) — the app now
  makes network requests, but only to `oryx.zsa.io` and only on explicit
  user action.
- Validation rejects non-Ergodox geometries; other ZSA boards would need
  their own geometry table before display could work.
- The bundled layout can drift from the user's current Oryx revision;
  that's acceptable since imports override it persistently.

## Alternatives considered

- **Local JSON file picker** — still requires the user to run curl first;
  the id/URL is what Oryx puts in the clipboard/address bar.
- **Auto-refresh from Oryx on startup** — network dependency on every
  launch and surprising layout changes; explicit import is predictable.
