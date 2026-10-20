#!/usr/bin/env python3
"""Render the MacClip app icon (1024x1024 PNG) using only the standard library."""
import math
import struct
import sys
import zlib

N = 1024


def rrect_cov(x, y, cx, cy, hx, hy, r):
    qx = max(abs(x - cx) - (hx - r), 0.0)
    qy = max(abs(y - cy) - (hy - r), 0.0)
    d = math.hypot(qx, qy) - r
    return min(max(0.5 - d, 0.0), 1.0)


def over(dst, src, a):
    return tuple(d * (1 - a) + s * a for d, s in zip(dst, src))


def main(out):
    rows = []
    for yi in range(N):
        row = bytearray([0])
        t = yi / (N - 1)
        top, bot = (96, 165, 255), (82, 72, 226)
        grad = tuple(top[i] * (1 - t) + bot[i] * t for i in range(3))
        for xi in range(N):
            x, y = xi + 0.5, yi + 0.5
            bg = rrect_cov(x, y, 512, 512, 412, 412, 185)
            if bg <= 0:
                row += b"\x00\x00\x00\x00"
                continue
            c = grad
            # clipboard board
            c = over(c, (248, 249, 255), rrect_cov(x, y, 512, 548, 215, 265, 44))
            # lines of text
            for ly, hw in ((470, 130), (560, 130), (650, 90)):
                c = over(c, (150, 160, 235), rrect_cov(x, y, 512 - (130 - hw), ly, hw, 19, 19))
            # metal clip
            c = over(c, (70, 62, 190), rrect_cov(x, y, 512, 290, 112, 52, 28))
            c = over(c, (248, 249, 255), rrect_cov(x, y, 512, 290, 44, 20, 12))
            row += bytes((int(c[0]), int(c[1]), int(c[2]), int(bg * 255)))
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    with open(out, "wb") as f:
        f.write(png)


if __name__ == "__main__":
    main(sys.argv[1])
