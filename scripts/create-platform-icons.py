#!/usr/bin/env python3
"""Create the macOS ICNS container from Forge's small RGBA PNG, using only Python stdlib."""
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read_rgba(path):
    data = path.read_bytes()
    offset = 8
    chunks = []
    width = height = color_type = bit_depth = interlace = None
    while offset < len(data):
        length = struct.unpack(">I", data[offset:offset + 4])[0]
        kind = data[offset + 4:offset + 8]
        chunk = data[offset + 8:offset + 8 + length]
        offset += 12 + length
        if kind == b"IHDR":
            width, height, bit_depth, color_type, _, _, interlace = struct.unpack(">IIBBBBB", chunk)
        elif kind == b"IDAT":
            chunks.append(chunk)
    if bit_depth != 8 or color_type != 6 or interlace != 0:
        raise ValueError("Forge icon source must be a non-interlaced 8-bit RGBA PNG")
    raw = zlib.decompress(b"".join(chunks))
    stride = width * 4
    rows, pos = [], 0
    prev = bytearray(stride)
    for _ in range(height):
        mode = raw[pos]
        scan = bytearray(raw[pos + 1:pos + 1 + stride])
        pos += stride + 1
        for i in range(stride):
            a = scan[i - 4] if i >= 4 else 0
            b = prev[i]
            c = prev[i - 4] if i >= 4 else 0
            if mode == 1: scan[i] = (scan[i] + a) & 255
            elif mode == 2: scan[i] = (scan[i] + b) & 255
            elif mode == 3: scan[i] = (scan[i] + ((a + b) // 2)) & 255
            elif mode == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                scan[i] = (scan[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
            elif mode != 0: raise ValueError(f"unsupported PNG filter {mode}")
        rows.append(bytes(scan))
        prev = scan
    return width, height, rows


def png_for_size(source, target):
    width, height, rows = source
    raw = bytearray()
    for y in range(target):
        sy = min(height - 1, y * height // target)
        raw.append(0)
        row = rows[sy]
        for x in range(target):
            sx = min(width - 1, x * width // target)
            raw.extend(row[sx * 4:sx * 4 + 4])
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xffffffff)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", target, target, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def main():
    source = read_rgba(ROOT / "icons" / "icon.png")
    entries = [(b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128),
               (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024), (b"ic11", 64),
               (b"ic12", 128), (b"ic13", 512), (b"ic14", 1024)]
    body = bytearray()
    for kind, size in entries:
        image = png_for_size(source, size)
        body.extend(kind + struct.pack(">I", len(image) + 8) + image)
    output = b"icns" + struct.pack(">I", len(body) + 8) + body
    (ROOT / "icons" / "icon.icns").write_bytes(output)
    print(f"Generated icons/icon.icns ({len(output)} bytes)")


if __name__ == "__main__":
    main()
