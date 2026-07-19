#!/usr/bin/env python3
"""Generate ui/geometry.json from the QMK ergodox_ez info.json.

The LAYOUT_ergodox key order matches the order of the `keys` arrays in the
Oryx layout export, so index i in geometry.json describes the physical key
for index i of every layer's `keys` array. Each entry carries the key's
position/size in key units and its matrix (row, col) as reported by the
Oryx live-training HID events.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
info = json.loads((ROOT / "assets/qmk/ergodox_ez_info.json").read_text())
layout = info["layouts"]["LAYOUT_ergodox"]["layout"]

keys = [
    {
        "x": k["x"],
        "y": k["y"],
        "w": k.get("w", 1),
        "h": k.get("h", 1),
        "row": k["matrix"][0],
        "col": k["matrix"][1],
    }
    for k in layout
]

out = ROOT / "ui/geometry.json"
out.parent.mkdir(exist_ok=True)
out.write_text(json.dumps(keys, indent=1) + "\n")
print(f"wrote {out} ({len(keys)} keys)")
