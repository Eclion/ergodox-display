# Changelog

2026-07-19 - 4afda1e..9b3e750 - Ergodox EZ layout overlay app: transparent borderless always-on-top Tauri window rendering the Symbols and Base layers stacked, live keypress + layer highlighting over ZSA's Oryx raw-HID protocol, tray icon with a settings window switching between click-through (overlay) and draggable (movable) modes, mode/position persistence, Linux udev rule.

2026-07-19 - e8868b9..e8868b9 - Key highlights are always cleared on release: the release now targets the exact element the press lit (correct across mid-press layer changes) and the lost-keyup fallback dropped from 3s to 500ms.

2026-07-19 - f50c841..f50c841 - GitHub Actions workflow building deb/AppImage (ubuntu-22.04) and msi/nsis (windows) bundles; artifacts on every push, GitHub release on v* tags.

2026-07-19 - 64efb21..64efb21 - Key labels adapt to the active OS keyboard layout (AZERTY etc.): xkbcommon on Linux/X11, ToUnicodeEx on Windows, live re-render within ~2s of switching input language.
