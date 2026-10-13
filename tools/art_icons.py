"""Level Twenty: the three kind icons (web/art/kinds/*.png, 2 x 2 cm,
transparent) and the help screen's triangle (web/art/triangle.png).

Facts these must get right, and where they are enforced:

- each kind keeps one colour everywhere: melee red, magic violet, ranged
  teal, the same values as web/style.css and art_units.BACK (`melee`,
  `magic`, `ranged` in artlib.COLOURS).
- each kind keeps one emblem everywhere: a sword for melee, a four-point
  spark for magic, an arrow through a target for ranged. The triangle draws
  the icons through `emblem`, the same function the icons use.
- the triangle's arrows point from a kind to the kind it is strong
  against, and only that way: melee to ranged, ranged to magic, magic to
  melee. `BEATS` holds the three, and `check_beats` compares them with the
  rule in crates/core/src/content.rs (`Kind::beats`) by reading the source.
- no words, letters or digits (tools/art.py's `check_no_text`).
"""
import math, re
from pathlib import Path

from artlib import P, f, rect, rrect, circle, ellipse, poly, line, curve, place, glint, tube
import artlib

BEATS = {"melee": "ranged", "ranged": "magic", "magic": "melee"}


def check_beats():
    src = (Path(__file__).resolve().parents[1] / "crates" / "core" / "src" / "content.rs").read_text()
    found = dict((a.lower(), b.lower()) for a, b in re.findall(r"Kind::(\w+) => Kind::(\w+)", src)[:3])
    assert found == BEATS, f"the triangle disagrees with Kind::beats: {found}"


def lw(w):
    return f",line width={f(w)}pt"


def emblem(kind):
    """The kind's badge, centred on the origin, radius 0.9."""
    s = circle(0, 0, 0.9, kind)
    s += circle(0, 0, 0.72, f"{kind}soft", lw(1.0))
    if kind == "melee":
        # a sword, point up and to the right
        blade = [(-0.12, -0.18), (0.4, 0.34), (0.55, 0.55), (0.34, 0.4), (-0.18, -0.12)]
        s += poly(blade, "white")
        s += line((-0.15, -0.15), (0.47, 0.47), lw=0.7, colour="metallight")
        s += poly([(-0.4, 0.0), (0.0, -0.4), (0.05, -0.32), (-0.32, 0.05)], "gold")
        s += poly([(-0.2, -0.28), (-0.28, -0.2), (-0.47, -0.39), (-0.39, -0.47)], "wood")
        s += circle(-0.5, -0.5, 0.1, "gold", lw(0.8))
    elif kind == "magic":
        pts = []
        for k in range(8):
            a = math.radians(90 + k * 45)
            r = 0.58 if k % 2 == 0 else 0.18
            pts.append((r * math.cos(a), r * math.sin(a)))
        s += poly(pts, "white")
        s += circle(0.42, 0.42, 0.09, "goldlight", lw(0.7))
        s += circle(-0.4, -0.38, 0.07, "goldlight", lw(0.7))
    else:
        for r, c in ((0.55, "white"), (0.36, "ranged"), (0.17, "white")):
            s += circle(0.08, -0.08, r, c, lw(0.9))
        s += line((-0.6, 0.6), (0.08, -0.08), lw=2.2, colour="ink")
        s += line((-0.6, 0.6), (0.08, -0.08), lw=1.2, colour="wood")
        s += poly([(-0.6, 0.6), (-0.75, 0.52), (-0.56, 0.42)], "red")
        s += poly([(-0.6, 0.6), (-0.52, 0.75), (-0.42, 0.56)], "red")
    return s


def icon(kind):
    def make():
        return place(1.0, 1.0, 1.0, emblem(kind)), (2.0, 2.0)
    return make


def arrow(a, b, bend, colour="gold"):
    """A fat curved arrow from a to b, outlined."""
    (ax, ay), (bx, by) = a, b
    mx, my = (ax + bx) / 2, (ay + by) / 2
    dx, dy = bx - ax, by - ay
    L = math.hypot(dx, dy)
    nx, ny = -dy / L, dx / L
    c = (mx + nx * bend, my + ny * bend)
    # stop short of b for the head
    tx, ty = bx - c[0], by - c[1]
    tl = math.hypot(tx, ty)
    ux, uy = tx / tl, ty / tl
    end = (bx - ux * 0.5, by - uy * 0.5)
    path = [a, (ax * 0.25 + c[0] * 0.75, ay * 0.25 + c[1] * 0.75), (end[0] * 0.25 + c[0] * 0.75, end[1] * 0.25 + c[1] * 0.75), end]
    s = tube([path], 0.28, colour, tension=0.7)
    px, py = -uy, ux
    s += poly([(bx, by), (end[0] + px * 0.38, end[1] + py * 0.38), (end[0] - px * 0.38, end[1] - py * 0.38)], colour)
    return s


def triangle():
    check_beats()
    W, H = 8.0, 7.2
    pos = {"magic": (4.0, 6.0), "melee": (1.2, 1.2), "ranged": (6.8, 1.2)}
    s = ""
    centre = (4.0, 2.8)
    for k, v in BEATS.items():
        a, b = pos[k], pos[v]
        # leave the badge and arrive at the next badge's rim
        ax, ay = a
        bx, by = b
        L = math.hypot(bx - ax, by - ay)
        ux, uy = (bx - ax) / L, (by - ay) / L
        start = (ax + ux * 1.05, ay + uy * 1.05)
        stop = (bx - ux * 1.05, by - uy * 1.05)
        # bend outward, away from the centre
        mx, my = (ax + bx) / 2, (ay + by) / 2
        out = (mx - centre[0]) * -uy + (my - centre[1]) * ux
        s += arrow(start, stop, -0.45 if out > 0 else 0.45)
    for k, (x, y) in pos.items():
        s += place(x, y, 1.15, emblem(k))
    return s, (W, H)


PICTURES = {
    "kinds/melee": icon("melee"),
    "kinds/magic": icon("magic"),
    "kinds/ranged": icon("ranged"),
    "triangle": triangle,
}
