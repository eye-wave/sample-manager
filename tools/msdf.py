#!/usr/bin/env python3
"""Generates assets/font.rgb (MSDF atlas) and assets/font.rs (glyph table) for printable ASCII.
Needs fontTools + numpy.   usage: python3 tools/msdf.py DejaVuSans.ttf
Outlines are flattened to line segments; edges are coloured at corners (msdfgen's scheme) and each
channel keeps the pseudo-distance to the nearest edge of its colours, signed by the winding number.
Only error correction: where the median disagrees with inside/outside, the plain distance is used."""
import math, sys
import numpy as np
from fontTools.pens.basePen import BasePen
from fontTools.ttLib import TTFont

EM = 32        # atlas px per em
RANGE = 4      # distance range in px; 0.5 +- RANGE/2 px maps to 0..1 (shader.wgsl PX_RANGE)
PAD = 3        # px around each glyph, >= RANGE/2
ATLAS_W = 256
FLATTEN = 8    # line segments per curve
C = [6, 5, 3]  # two-channel colours (G+B, R+B, R+G); 7 = all channels


class Pen(BasePen):
    def __init__(s, gs):
        super().__init__(gs)
        s.contours, s.edges = [], []

    def _moveTo(s, p): s.edges, s.start, s.last = [], p, p
    def _lineTo(s, p):
        if p != s.last: s.edges.append([s.last, p])
        s.last = p
    def _qCurveToOne(s, c, p): s.edges.append([s.last, c, p]); s.last = p
    def _curveToOne(s, a, b, p): s.edges.append([s.last, a, b, p]); s.last = p
    def _closePath(s):
        s._lineTo(s.start)
        if s.edges: s.contours.append(s.edges)
        s.edges = []
    _endPath = _closePath


def bezier(pts, n):
    t, p = np.linspace(0, 1, n + 1)[:, None], np.array(pts, float)
    m = len(p) - 1
    return sum(math.comb(m, i) * (1 - t) ** (m - i) * t ** i * p[i] for i in range(m + 1))


def tangents(e):
    p = [np.array(x, float) for x in e]
    a = next(q - p[0] for q in p[1:] if np.linalg.norm(q - p[0]) > 1e-9)
    b = next(p[-1] - q for q in reversed(p[:-1]) if np.linalg.norm(p[-1] - q) > 1e-9)
    return a, b


def is_corner(a, b):
    a, b = a / np.linalg.norm(a), b / np.linalg.norm(b)
    return a @ b <= 0 or abs(a[0] * b[1] - a[1] * b[0]) > math.sin(3.0)


def colours(flags):
    n = len(flags)
    corners = [i for i, f in enumerate(flags) if f]
    if not corners: return [7] * n
    if len(corners) == 1:  # teardrop: three parts so the corner still gets two colours
        return [[C[0], 7, C[1]][((k - corners[0]) % n) * 3 // n] for k in range(n)]
    out, k = [0] * n, len(corners)
    for m in range(k):
        col = C[m % 3]
        if m == k - 1 and col == C[0]:
            col = next(c for c in C if c not in (C[(k - 2) % 3], C[0]))
        s, e = corners[m], corners[(m + 1) % k]
        for i in range(s, s + ((e - s) % n or n)): out[i % n] = col
    return out


def segments(contours, k):
    A, B, col = [], [], []
    for edges in contours:
        edges = [[(x * k, y * k) for x, y in e] for e in edges]
        segs, flags = [], []
        for i, e in enumerate(edges):
            flags_here = is_corner(tangents(edges[i - 1])[1], tangents(e)[0])
            poly = bezier(e, 1 if len(e) == 2 else FLATTEN)
            for j in range(len(poly) - 1):
                segs.append((poly[j], poly[j + 1])); flags.append(flags_here and j == 0)
        for (a, b), c in zip(segs, colours(flags)): A.append(a); B.append(b); col.append(c)
    return np.array(A), np.array(B), np.array(col)


def msdf(A, B, col, x0, ytop, w, h):
    P = np.stack(np.meshgrid(x0 + np.arange(w) + .5, ytop - np.arange(h) - .5), -1).reshape(-1, 2)
    N, ab = len(P), B - A
    L2 = np.maximum((ab ** 2).sum(1), 1e-12)
    ap = P[:, None, :] - A[None]
    t = (ap * ab[None]).sum(2) / L2
    q = ap - np.clip(t, 0, 1)[..., None] * ab[None]
    d = np.sqrt((q ** 2).sum(2))
    perp = np.abs(ap[..., 0] * ab[None, :, 1] - ap[..., 1] * ab[None, :, 0]) / np.sqrt(L2)
    pseudo = np.where((t < 0) | (t > 1), perp, d)  # past an end: distance to the edge's line
    # on a tie (the pixel is nearest to a shared corner) the edge it approaches most
    # perpendicularly wins, as in msdfgen
    orth = np.abs((q * (ab / np.sqrt(L2)[:, None])[None]).sum(2)) / np.maximum(d, 1e-9)
    dsel = d + 1e-6 * orth
    cr = ab[None, :, 0] * ap[..., 1] - ap[..., 0] * ab[None, :, 1]
    ay, by, py = A[None, :, 1], B[None, :, 1], P[:, 1:2]
    wn = ((ay <= py) & (by > py) & (cr > 0)).sum(1) - ((ay > py) & (by <= py) & (cr < 0)).sum(1)
    sign = np.where(wn != 0, 1.0, -1.0)
    out = np.empty((N, 3))
    for c in range(3):
        idx = np.where((col >> c) & 1)[0]
        sel = idx[np.argmin(dsel[:, idx], axis=1)]
        out[:, c] = pseudo[np.arange(N), sel]
    out = sign[:, None] * out
    med = np.median(out, axis=1)
    bad = (med > 0) != (sign > 0)  # median says inside/outside wrongly: use the plain distance
    out[bad] = (sign * d.min(1))[bad, None]
    out = np.clip(0.5 + out / RANGE, 0, 1)
    return (out * 255 + .5).astype(np.uint8).reshape(h, w, 3)


def main(path):
    font = TTFont(path)
    gs, cmap, upem = font.getGlyphSet(), font.getBestCmap(), font["head"].unitsPerEm
    k = EM / upem
    cells, x, y, row = [], 0, 0, 0
    for cp in range(32, 127):
        name = cmap[cp]
        pen = Pen(gs); gs[name].draw(pen)
        adv = font["hmtx"][name][0] / upem
        if not pen.contours:
            cells.append((adv, [0, 0, 0, 0], [0, 0, 0, 0], None)); continue
        A, B, col = segments(pen.contours, k)
        pts = np.concatenate([A, B])
        gx0, gy0 = math.floor(pts[:, 0].min()) - PAD, math.floor(pts[:, 1].min()) - PAD
        gx1, gy1 = math.ceil(pts[:, 0].max()) + PAD, math.ceil(pts[:, 1].max()) + PAD
        w, h = gx1 - gx0, gy1 - gy0
        if x + w > ATLAS_W: x, y, row = 0, y + row + 1, 0
        img = msdf(A, B, col, gx0, gy1, w, h)
        cells.append((adv, [gx0 / EM, gy0 / EM, gx1 / EM, gy1 / EM], [x, y, w, h], img))
        x, row = x + w + 1, max(row, h)
    H = y + row
    atlas = np.zeros((H, ATLAS_W, 3), np.uint8)
    for _, _, (cx, cy, w, h), img in cells:
        if img is not None: atlas[cy:cy + h, cx:cx + w] = img
    atlas.tofile("assets/font.rgb")
    with open("assets/font.rs", "w") as f:
        f.write(f"// generated by tools/msdf.py from {path.split('/')[-1]}: do not edit\n")
        f.write(f"pub const ATLAS_W: u32 = {ATLAS_W};\npub const ATLAS_H: u32 = {H};\n")
        f.write(f"pub const PX_RANGE: f32 = {RANGE:.1f};\n")
        f.write(f"/// ASCII 32..=126\npub static GLYPHS: [Glyph; {len(cells)}] = [\n")
        for adv, p, r, _ in cells:
            f.write(f"    Glyph {{ adv: {adv:.5f}, plane: [{p[0]:.5f}, {p[1]:.5f}, {p[2]:.5f}, {p[3]:.5f}], rect: {r} }},\n")
        f.write("];\n")
    print(f"atlas {ATLAS_W}x{H}, {len(cells)} glyphs")


main(sys.argv[1])
