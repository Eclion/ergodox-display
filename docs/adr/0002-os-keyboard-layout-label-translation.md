# 0002. OS keyboard-layout label translation in the backend

- **Status:** Accepted
- **Date:** 2026-07-19
- **Deciders:** eclion, Claude

## Context

The keyboard sends fixed HID usages; the character each key produces is
decided by the OS input layout (the user switches between French/AZERTY and
US). The overlay displayed QMK keycode names, which assume a US layout, so
labels did not match what typing actually produced.

## Decision

The Rust backend translates printable QMK keycodes to the characters of the
active OS layout and pushes a `layout-map` event to the webview whenever the
result changes (2s poll — layout switches are rare and the computation is
milliseconds):

- **Linux (X11):** `xkbcommon` (x11 feature) + `xcb` — the server's current
  keymap and effective layout group, chars via `xkb_state` with and without
  Shift. Dead keys map through a small `dead_*` keysym → glyph table.
- **Windows:** `GetKeyboardLayout` of the foreground thread, then
  `MapVirtualKeyExW` + `ToUnicodeEx` per scan code (with the documented
  double-call flush for dead keys).
- The shared table maps each QMK keycode to its evdev keycode and set-1
  scan code; shift-alias keycodes (`KC_EXLM`, …) take the shifted character
  of their base key. A key displays uppercase only when Shift genuinely
  produces the uppercase of its character.
- The frontend overrides its US-default labels with the received map and
  rebuilds the boards; a `get_layout_map` command covers the startup race,
  same pattern as `get_kb_state`.

## Consequences

- Labels always show what the OS will actually type, including the honest
  (sometimes surprising) cases: on AZERTY the Symbols layer's `!` key types
  `1`.
- New Linux build dependencies: `libxkbcommon-dev`, `libxkbcommon-x11-dev`
  (runtime `libxkbcommon-x11-0` ships with stock GNOME). CI updated.
- Linux detection is X11-only; on Wayland the resolver returns nothing and
  labels stay at the US defaults (graceful degradation).
- Named keys (Enter, arrows, F-keys, media) are not translated (not
  layout-dependent).

## Alternatives considered

- **`navigator.keyboard.getLayoutMap()` in the webview** — Chromium-only;
  WebKitGTK (Linux side) does not implement it.
- **User-selected layout in settings + static tables** — no extra system
  deps, but manual, and wrong the moment the user switches OS language —
  which is the exact trigger the feature is for.
- **XKB event subscription instead of polling** — proper push semantics but
  meaningfully more plumbing (XKB event loop on a dedicated connection);
  a 2s poll is indistinguishable in practice for this UI.
