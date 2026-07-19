# 0001. Tauri desktop shell with Oryx raw-HID keypress capture

- **Status:** Accepted
- **Date:** 2026-07-19
- **Deciders:** eclion, Claude

## Context

The app must show the Ergodox EZ layout layers (Symbols above Base) as a
borderless, transparent, always-on-top overlay on Linux (X11/GNOME) and
Windows, and highlight keys as they are physically pressed — including
knowing which layer is active. OS-level keyboard hooks cannot reliably
distinguish layers or map layer-shifted keycodes back to physical keys, and
behave differently across platforms.

## Decision

- **Tauri v2 (Rust backend + static vanilla-JS webview frontend)** for the
  desktop shell. No frontend framework or bundler: `ui/` is served as-is via
  `frontendDist`, keeping the toolchain to `cargo` only.
- **Keypress capture via ZSA's Oryx raw-HID live-training protocol** (the
  one Keymapp uses): the backend opens the keyboard's raw HID interface
  (vendor `0x3297`, usage page `0xFF60`, usage `0x61`), sends
  `ORYX_CMD_PAIRING_INIT`, and receives 32-byte keydown/keyup packets with
  matrix `(col, row)` plus active-layer events. Protocol constants come from
  `keyboards/zsa/common/oryx.[ch]` in ZSA's QMK fork (firmware24).
- **Geometry as generated data**: `scripts/gen_geometry.py` derives
  `ui/geometry.json` (x/y/w/h + matrix row/col per key) from QMK's
  `LAYOUT_ergodox` definition. Index order matches the Oryx layout export's
  `keys` arrays, which was verified against the user's Base layer.
- **Window modes** (click-through "overlay" vs draggable "movable") are
  applied backend-side via `set_ignore_cursor_events`, toggled from a tray
  settings window, persisted as JSON in the app config dir.
- The keyboard is matched by vendor id + usage page only (no product id):
  the base/shine/glow variants report different PIDs (0x4974–0x4976).

## Consequences

- Exact physical key + layer fidelity, independent of OS keyboard state;
  works identically on Linux and Windows through `hidapi`.
- Linux requires a one-time udev rule (`assets/udev/50-zsa.rules`) for
  hidraw access; without it the app shows a "cannot open HID device" status.
- Capture only works with ZSA keyboards running Oryx-flavoured firmware; a
  non-ZSA keyboard would need a different capture backend.
- Raw-HID packets are sent best-effort by the firmware (notably on the
  atmega32u4), so the UI treats keyups as lossy and auto-clears highlights
  after a fallback delay; events emitted before the webview loads are
  recovered via a `get_kb_state` catch-up command.
- Tauri's first build compiles ~500 crates (minutes); incremental builds
  are seconds. Bundling installers needs `tauri-cli` (not yet a dev
  dependency).

## Alternatives considered

- **OS global key hook (rdev/iohook)** — cannot tell layers apart, maps
  layer-shifted keycodes ambiguously, and needs elevated access/Wayland
  workarounds. Rejected.
- **Electron** — heavier runtime (~100 MB+, Chromium per app) for the same
  webview needs; no benefit since the backend must be native for HID anyway.
- **Native Rust GUI (egui/iced)** — lightest runtime but transparent
  always-on-top windows and rich key styling are considerably more work;
  the web layer makes the board rendering trivial.
- **Parsing the compiled firmware hex for the layout** — fragile
  reverse-engineering; the Oryx GraphQL API provides the exact layout JSON
  for the layout id embedded in the hex filename.
