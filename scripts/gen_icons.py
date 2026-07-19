#!/usr/bin/env python3
"""Generate the app icons (PNG + ICO) without external dependencies.

Draws a flat rounded-square badge with a stylized split keyboard (two key
grids) on it. ICO output embeds PNG data (supported since Windows Vista).
"""
import pathlib
import struct
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri/icons"
OUT.mkdir(parents=True, exist_ok=True)

BG = (30, 34, 42, 255)
KEY = (94, 234, 212, 255)  # teal keys
KEY_DIM = (148, 163, 184, 255)


def rounded_mask(size, x, y, w, h, r):
    def inside(px, py):
        if not (x <= px < x + w and y <= py < y + h):
            return False
        cx = min(max(px, x + r), x + w - 1 - r)
        cy = min(max(py, y + r), y + h - 1 - r)
        return (px - cx) ** 2 + (py - cy) ** 2 <= r * r or (
            x + r <= px < x + w - r or y + r <= py < y + h - r
        )

    return inside


def draw(size):
    px = [[(0, 0, 0, 0)] * size for _ in range(size)]
    m = size / 32.0
    badge = rounded_mask(size, round(1 * m), round(1 * m), round(30 * m), round(30 * m), round(7 * m))
    for yy in range(size):
        for xx in range(size):
            if badge(xx, yy):
                px[yy][xx] = BG
    # two 3x2 key grids (left / right half), slight vertical offset
    key_w = round(5 * m)
    gap = round(2 * m)
    for half, (ox, oy) in enumerate([(round(4 * m), round(8 * m)), (round(18 * m), round(11 * m))]):
        for row in range(2):
            for col in range(2):
                color = KEY if (half + row + col) % 2 == 0 else KEY_DIM
                kx = ox + col * (key_w + gap)
                ky = oy + row * (key_w + gap)
                keym = rounded_mask(size, kx, ky, key_w, key_w, max(1, round(1.5 * m)))
                for yy in range(ky, min(size, ky + key_w)):
                    for xx in range(kx, min(size, kx + key_w)):
                        if keym(xx, yy):
                            px[yy][xx] = color
    return px


def png_bytes(px):
    size = len(px)
    raw = b"".join(
        b"\x00" + b"".join(struct.pack("4B", *p) for p in row) for row in px
    )

    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c))

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


pngs = {}
for size in (32, 128, 256):
    pngs[size] = png_bytes(draw(size))

(OUT / "32x32.png").write_bytes(pngs[32])
(OUT / "128x128.png").write_bytes(pngs[128])
(OUT / "128x128@2x.png").write_bytes(pngs[256])
(OUT / "icon.png").write_bytes(pngs[256])

# ICO with embedded PNGs
entries = [(32, pngs[32]), (256, pngs[256])]
ico = struct.pack("<HHH", 0, 1, len(entries))
offset = 6 + 16 * len(entries)
body = b""
for size, data in entries:
    dim = 0 if size == 256 else size
    ico += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
    body += data
    offset += len(data)
(OUT / "icon.ico").write_bytes(ico + body)
print(f"wrote icons to {OUT}")
