"""Level Twenty: the twelve unit portraits, drawn as TikZ and compiled by
tools/art.py (web/art/units/<id>.png, 6 x 6 cm at 150 dpi). Never generated
pixels.

Each portrait is a head-and-shoulders figure in front of a scrap of city.
People share one body (`bust`), one face (`face`) and a set of hair styles
(`hair_back`, `hair_front`); each unit's function adds what makes it that
unit. Chum 3000 and Biscuit are not people and are drawn whole. The look is
vagrancy/Sam_Comm_final.png's: flat fills, plum ink round everything, a
warm yellow sky, salmon walls, sage machines, and small clutter.

Facts each portrait must get right (what an image generator gets wrong),
and where this file enforces each:

- no words, letters or digits: Chum 3000's forehead carries the coloured
  plate from the inspiration picture but no lettering, and every sign is a
  coloured bar. tools/art.py's `check_no_text` refuses a .tex with \\node
  or \\text before compiling.
- the kind is readable at a glance from the background tint: melee warm
  coral walls, magic violet dusk, ranged teal and sage (`BACK`), matching
  web/style.css's --melee/--magic/--ranged and data/units.json's kinds,
  which `KIND` repeats and `check_kinds` compares with the data file.
- each unit holds the thing its skill names: Rook's taped fists, Vela's
  neon sign shapes, Pip's slingshot, Nan's paper lantern, Ed's exosuit
  pistons, Static's antenna and zigzags, Chum's riveted cat head and teeth,
  June's twin jet engines, the Sage's smoke, Biscuit's curls, Dee's
  quadcopter (exactly four rotors, asserted), Tagger's spray can.
- people have exactly two eyes and two ears; hands have no fingers counted
  wrong because hands are mittens and fists (no fingers drawn).
- repeatable: every random choice comes from random.Random(seed) per
  picture.
"""
import json, math, random
from pathlib import Path

from artlib import (P, f, pts, rect, rrect, circle, ellipse, ellipse_path, poly, blob, hill,
                    line, curve, leaf, limb, place, clipped, glint, rot, sky, cloud, tube)
import artlib

W = H = 6.0

KIND = {
    "rook": "melee", "ed": "melee", "chum": "melee", "biscuit": "melee",
    "vela": "magic", "nan": "magic", "static": "magic", "sage": "magic",
    "pip": "ranged", "june": "ranged", "dee": "ranged", "tagger": "ranged",
}

# sky (top, middle, low) and three wall colours per kind
BACK = {
    "melee": (("skyhi", "sky", "skylow"), ["wallA", "wallE", "wallC"]),
    "magic": (("magicsoft!55!skyhi", "magicsoft!70!sky", "violet!45!wallC"), ["violet!45!wallF", "magicsoft!80!wallC", "violet!35!wallA"]),
    "ranged": (("skyhi", "rangedsoft!55!sky", "rangedsoft"), ["teal!55!sage", "sage", "rangedsoft!70!tealdark"]),
}


def check_kinds():
    data = json.loads((Path(__file__).resolve().parents[1] / "data" / "units.json").read_text())
    got = {u["id"]: u["kind"] for u in data["units"]}
    assert got == KIND, f"art_units.KIND disagrees with data/units.json: {got}"


def lw(w):
    return f",line width={f(w)}pt"


def fillo(path, colour, op):
    return f"\\fill[{colour}, opacity={f(op)}] {path};\n"


# --------------------------------------------------------------------------
# the scrap of city behind each portrait
# --------------------------------------------------------------------------

BARE = [False]


def figure(name):
    """A unit without its scrap of city, for placing in a scene."""
    BARE[0] = True
    try:
        return PICTURES[f"units/{name}"]()[0]
    finally:
        BARE[0] = False


def backdrop(kind, rng):
    if BARE[0]:
        return ""
    (top, mid, low), walls = BACK[kind]
    s = sky(W, H, top, mid, low, a=0.7, b=0.25)
    s += cloud(rng.uniform(0.8, 1.6), rng.uniform(5.1, 5.5), 0.45)
    s += cloud(rng.uniform(4.4, 5.2), rng.uniform(4.5, 5.0), 0.32)
    x = -0.3
    k = 0
    while x < W:
        w = rng.uniform(1.1, 1.8)
        h = rng.uniform(2.3, 4.2)
        col = walls[k % len(walls)]
        s += rect(x, -0.2, x + w, h, col)
        s += rect(x - 0.06, h, x + w + 0.06, h + 0.14, f"{col}!80!ink")
        # windows: a grid of small panes, some lit
        wy = h - 0.55
        while wy > 0.3:
            wx = x + 0.2
            while wx + 0.3 < x + w - 0.1:
                lit = rng.random() < 0.35
                s += rect(wx, wy, wx + 0.28, wy + 0.32, "window" if lit else f"{col}!65!windark", lw(0.6))
                wx += 0.45
            wy -= 0.6
        # a sign bar or an air conditioner on some
        if rng.random() < 0.5:
            sy = rng.uniform(1.2, h - 0.8)
            s += rect(x + w - 0.25, sy, x + w + 0.05, sy + 0.9, rng.choice(["gold", "teal", "red", "pink"]), lw(0.8))
        else:
            ay = rng.uniform(1.0, h - 0.8)
            s += rect(x + 0.1, ay, x + 0.55, ay + 0.35, "metallight", lw(0.7))
            s += circle(x + 0.33, ay + 0.175, 0.11, "metal", lw(0.5))
        x += w + rng.uniform(-0.05, 0.15)
        k += 1
    # wires across
    s += curve([(-0.2, 4.6), (2.0, 4.1), (4.2, 4.3), (6.2, 4.75)], lw=0.8)
    return s


# --------------------------------------------------------------------------
# people
# --------------------------------------------------------------------------

def bust(top, collar=None, stripes=None, shoulders=1.0):
    """Shoulders and chest, the neck sitting at (3, 2.2)."""
    a = 2.1 * shoulders
    path = [(3 - a - 0.25, -0.4), (3 - a - 0.05, 1.3), (3 - a + 0.45, 2.05), (3 - 0.55, 2.3),
            (3 + 0.55, 2.3), (3 + a - 0.45, 2.05), (3 + a + 0.05, 1.3), (3 + a + 0.25, -0.4)]
    s = f"\\filldraw[inked,fill={top}] plot[smooth, tension=0.35] coordinates {{{pts(path)}}} -- cycle;\n"
    if stripes:
        body = f"plot[smooth, tension=0.35] coordinates {{{pts(path)}}} -- cycle"
        inner = ""
        y = 1.95
        while y > -0.4:
            inner += f"\\fill[{stripes}] (0,{f(y)}) rectangle (6,{f(y - 0.17)});\n"
            y -= 0.38
        s += clipped(body, inner)
        s += f"\\draw[ink, line width=\\Ink, line join=round] {body};\n"
    if collar:
        s += poly([(2.45, 2.28), (3.0, 1.55), (3.55, 2.28), (3.25, 2.32), (3.0, 1.95), (2.75, 2.32)], collar)
    return s


def neck(skin):
    return rect(2.62, 1.9, 3.38, 2.75, skin) + curve([(2.7, 2.05), (3.0, 1.95), (3.3, 2.05)], lw=0.7)


def head(skin, ears=True, cy=3.55):
    s = ""
    if ears:
        s += ellipse(1.88, cy - 0.05, 0.2, 0.27, skin)
        s += ellipse(4.12, cy - 0.05, 0.2, 0.27, skin)
        s += curve([(1.84, cy + 0.07), (1.92, cy - 0.12)], lw=0.6)
        s += curve([(4.16, cy + 0.07), (4.08, cy - 0.12)], lw=0.6)
    s += ellipse(3.0, cy, 1.12, 1.24, skin)
    return s


def face(cy=3.55, eyes="dot", mouth="smile", brows="flat", blush=True, nose=True, glasses=False, sweat=False):
    """Two eyes, brows, a nose and a mouth on a head centred at (3, cy)."""
    s = ""
    ey = cy - 0.05
    for ex in (2.55, 3.45):
        if eyes == "dot":
            s += ellipse(ex, ey, 0.12, 0.17, "ink", extra=lw(0.4))
            s += glint(ex + 0.04, ey + 0.06, 0.045)
        elif eyes == "big":
            s += ellipse(ex, ey, 0.2, 0.25, "white", extra=lw(0.9))
            s += ellipse(ex, ey - 0.03, 0.12, 0.16, "ink", extra=lw(0.4))
            s += glint(ex + 0.04, ey + 0.04, 0.045)
        elif eyes == "shut":
            s += curve([(ex - 0.15, ey), (ex, ey - 0.08), (ex + 0.15, ey)], lw=1.3)
        elif eyes == "glow":
            s += ellipse(ex, ey, 0.17, 0.12, "cyan", extra=lw(0.8))
            s += glint(ex, ey, 0.05)
        elif eyes == "squint":
            s += line((ex - 0.16, ey), (ex + 0.16, ey), lw=1.6)
    by = cy + 0.28
    if brows == "flat":
        s += line((2.38, by), (2.7, by + 0.02), lw=1.6)
        s += line((3.3, by + 0.02), (3.62, by), lw=1.6)
    elif brows == "angry":
        s += line((2.36, by + 0.08), (2.72, by - 0.06), lw=2.0)
        s += line((3.28, by - 0.06), (3.64, by + 0.08), lw=2.0)
    elif brows == "up":
        s += curve([(2.38, by), (2.55, by + 0.1), (2.72, by + 0.04)], lw=1.4)
        s += curve([(3.28, by + 0.04), (3.45, by + 0.1), (3.62, by)], lw=1.4)
    elif brows == "worried":
        s += line((2.38, by - 0.02), (2.72, by + 0.1), lw=1.5)
        s += line((3.28, by + 0.1), (3.62, by - 0.02), lw=1.5)
    if glasses:
        s += f"\\draw[ink, line width=1.1pt] {P(2.55, ey)} circle (0.27);\n"
        s += f"\\draw[ink, line width=1.1pt] {P(3.45, ey)} circle (0.27);\n"
        s += line((2.82, ey + 0.02), (3.18, ey + 0.02), lw=1.0)
    if nose:
        s += curve([(3.02, cy - 0.12), (3.1, cy - 0.32), (2.96, cy - 0.38)], lw=1.0)
    my = cy - 0.62
    if mouth == "smile":
        s += curve([(2.7, my + 0.06), (3.0, my - 0.08), (3.3, my + 0.06)], lw=1.3)
    elif mouth == "grin":
        s += f"\\filldraw[inked,fill=white] {P(2.62, my + 0.08)} .. controls {P(2.8, my - 0.32)} and {P(3.2, my - 0.32)} .. {P(3.38, my + 0.08)} -- cycle;\n"
        s += line((2.66, my + 0.03), (3.34, my + 0.03), lw=0.6)
    elif mouth == "open":
        s += ellipse(3.0, my - 0.02, 0.17, 0.13, "lips", extra=lw(0.9))
    elif mouth == "flat":
        s += line((2.78, my), (3.22, my - 0.02), lw=1.4)
    elif mouth == "smirk":
        s += curve([(2.75, my), (3.05, my - 0.04), (3.3, my + 0.1)], lw=1.4)
    elif mouth == "teeth":
        s += rrect(2.68, my - 0.16, 3.32, my + 0.08, "white", 0.06)
        s += line((2.68, my - 0.04), (3.32, my - 0.04), lw=0.6)
    if blush:
        s += fillo(ellipse_path(2.2, cy - 0.32, 0.22, 0.12), "pink", 0.55)
        s += fillo(ellipse_path(3.8, cy - 0.32, 0.22, 0.12), "pink", 0.55)
    if sweat:
        for (x, y, r) in ((1.95, cy + 0.6, 0.11), (4.12, cy + 0.45, 0.09), (3.85, cy + 0.95, 0.07), (2.2, cy - 0.65, 0.07)):
            s += f"\\filldraw[inked,fill=cyan!25!white{lw(0.8)}] {P(x, y + r * 1.8)} .. controls {P(x + r, y + r * 0.3)} and {P(x + r, y - r)} .. {P(x, y - r)} .. controls {P(x - r, y - r)} and {P(x - r, y + r * 0.3)} .. {P(x, y + r * 1.8)} -- cycle;\n"
    return s


def hair_back(style, col, cy=3.55):
    if style == "bob":
        return blob([(1.65, 2.55), (1.6, 3.6), (1.95, 4.6), (3.0, 4.95), (4.05, 4.6), (4.4, 3.6), (4.35, 2.55), (3.0, 2.45)], col, tension=0.6)
    if style == "long":
        return blob([(1.55, 1.2), (1.5, 3.4), (1.9, 4.65), (3.0, 5.0), (4.1, 4.65), (4.5, 3.4), (4.45, 1.2), (3.0, 1.6)], col, tension=0.55)
    if style == "bun":
        return circle(3.0, 5.0, 0.48, col) + curve([(2.7, 5.05), (3.0, 5.25), (3.3, 5.05)], lw=0.8)
    return ""


def hair_front(style, col, cy=3.55):
    t = cy + 1.24
    if style == "flattop":
        s = f"\\filldraw[inked,fill={col}] {P(1.9, cy + 0.45)} -- {P(1.95, t + 0.12)} -- {P(4.05, t + 0.12)} -- {P(4.1, cy + 0.45)} .. controls {P(3.6, cy + 0.75)} and {P(2.4, cy + 0.75)} .. cycle;\n"
        for k in range(6):
            x = 2.2 + k * 0.32
            s += line((x, t + 0.05), (x + 0.05, cy + 0.85), lw=0.6, colour="paper!35!" + col)
        return s
    if style == "bob":
        return f"\\filldraw[inked,fill={col}] {P(1.85, cy + 0.15)} .. controls {P(1.8, t + 0.25)} and {P(4.2, t + 0.25)} .. {P(4.15, cy + 0.15)} -- {P(4.0, cy + 0.45)} -- {P(3.6, cy + 0.42)} -- {P(3.3, cy + 0.55)} -- {P(3.0, cy + 0.42)} -- {P(2.65, cy + 0.55)} -- {P(2.3, cy + 0.42)} -- {P(2.0, cy + 0.5)} -- cycle;\n"
    if style == "long":
        return f"\\filldraw[inked,fill={col}] {P(1.82, cy + 0.0)} .. controls {P(1.7, t + 0.3)} and {P(4.3, t + 0.3)} .. {P(4.18, cy + 0.0)} .. controls {P(3.9, cy + 0.7)} and {P(3.4, cy + 0.5)} .. {P(2.9, cy + 0.85)} .. controls {P(2.5, cy + 0.55)} and {P(2.1, cy + 0.6)} .. cycle;\n"
    if style == "bun":
        return f"\\filldraw[inked,fill={col}] {P(1.88, cy + 0.2)} .. controls {P(1.85, t + 0.2)} and {P(4.15, t + 0.2)} .. {P(4.12, cy + 0.2)} .. controls {P(3.6, cy + 0.7)} and {P(2.4, cy + 0.7)} .. cycle;\n" + curve([(2.3, cy + 0.85), (3.0, t + 0.05), (3.7, cy + 0.85)], lw=0.6)
    if style == "spiky":
        spikes = [(1.85, cy + 0.3), (1.7, cy + 1.0), (2.1, cy + 1.1), (2.0, t + 0.35), (2.55, t + 0.05), (2.8, t + 0.55), (3.2, t + 0.08), (3.55, t + 0.5), (3.75, t), (4.25, t + 0.15), (4.0, cy + 0.9), (4.3, cy + 0.8), (4.15, cy + 0.3),
                  (3.7, cy + 0.6), (3.3, cy + 0.45), (2.9, cy + 0.62), (2.5, cy + 0.45), (2.15, cy + 0.6)]
        return poly(spikes, col)
    if style == "bald":
        return glint(2.6, cy + 0.95, 0.12, 0.6)
    return ""


def fist(x, y, r, fill, tape=None, rot_=0):
    s = f"\\filldraw[inked,fill={fill},rounded corners=0.12cm] {P(x - r, y - r * 0.85)} rectangle {P(x + r, y + r * 0.85)};\n"
    for k in range(3):
        kx = x - r * 0.62 + k * r * 0.62
        s += curve([(kx - r * 0.25, y + r * 0.85), (kx, y + r * 0.6), (kx + r * 0.25, y + r * 0.85)], lw=0.8)
    s += curve([(x - r * 0.9, y - r * 0.1), (x - r * 0.3, y - r * 0.2), (x + r * 0.3, y - r * 0.05)], lw=0.9)
    if tape:
        s += rect(x - r, y - r * 0.85, x + r, y - r * 0.35, tape, lw(0.8))
        s += line((x - r * 0.4, y - r * 0.85), (x - r * 0.3, y - r * 0.35), lw=0.5)
    return s


def mitten(x, y, r, fill):
    return ellipse(x, y, r, r * 0.85, fill) + curve([(x - r * 0.6, y + r * 0.3), (x - r * 0.9, y + r * 0.6)], lw=0.8)


# --------------------------------------------------------------------------
# the twelve
# --------------------------------------------------------------------------

def rook():
    rng = random.Random(101)
    skin, hair = "skinc", "ink!80!wooddark"
    s = backdrop("melee", rng)
    s += bust("redlight", collar=None, shoulders=1.12)
    # tank top over a broad chest
    s += poly([(2.2, -0.4), (2.25, 1.7), (2.55, 2.25), (3.45, 2.25), (3.75, 1.7), (3.8, -0.4)], "cream")
    s += curve([(2.6, 2.2), (3.0, 1.75), (3.4, 2.2)], lw=1.0)
    s += neck(skin)
    s += head(skin)
    s += face(eyes="dot", mouth="smirk", brows="angry")
    s += hair_front("flattop", hair)
    # a plaster across the nose
    s += f"\\filldraw[inked,fill=woodlight,rotate around={{-20:{P(3.0, 3.35)}}}{lw(0.8)}] {P(2.75, 3.28)} rectangle {P(3.25, 3.42)};\n"
    # two taped fists up in guard
    s += limb([(1.2, -0.4), (1.25, 0.6), (1.75, 1.15)], 0.42, 0.36, skin)
    s += limb([(4.8, -0.4), (4.75, 0.6), (4.25, 1.15)], 0.42, 0.36, skin)
    s += fist(1.75, 1.45, 0.5, skin, tape="cream")
    s += fist(4.25, 1.45, 0.5, skin, tape="cream")
    return s, (W, H)


def vela():
    rng = random.Random(202)
    skin, hair = "skin", "hairA"
    s = backdrop("magic", rng)
    # neon sign shapes floating behind her
    for (x, y, w, h, c) in ((0.5, 3.2, 0.5, 1.4, "magenta"), (4.9, 2.6, 0.45, 1.2, "cyan"), (4.6, 4.4, 1.0, 0.35, "gold")):
        s += fillo(f"{P(x - 0.15, y - 0.15)} rectangle {P(x + w + 0.15, y + h + 0.15)}", c, 0.25)
        s += rrect(x, y, x + w, y + h, f"{c}!40!white", 0.08)
    s += hair_back("long", hair)
    s += bust("violet", collar="magicsoft")
    s += neck(skin)
    s += head(skin)
    s += face(eyes="big", mouth="smile", brows="up")
    s += hair_front("long", hair)
    # the witch's hat: a wide brim and a bent point
    s += ellipse(3.0, 4.55, 1.85, 0.32, "violet!80!ink")
    s += f"\\filldraw[inked,fill=violet!80!ink] {P(1.95, 4.6)} .. controls {P(2.3, 5.3)} and {P(2.7, 5.8)} .. {P(3.7, 5.85)} .. controls {P(3.5, 5.5)} and {P(3.9, 5.1)} .. {P(4.05, 4.6)} -- cycle;\n"
    s += rect(2.05, 4.55, 3.95, 4.82, "magenta", lw(0.9))
    s += circle(3.7, 5.85, 0.12, "gold", lw(0.7))
    # one hand up with a spark of sign light
    s += limb([(4.7, -0.4), (5.0, 0.6), (4.8, 1.3)], 0.38, 0.3, "violet")
    s += mitten(4.8, 1.5, 0.3, skin)
    s += fillo(f"{P(4.8, 2.1)} circle (0.55)", "magenta", 0.25)
    s += star(4.8, 2.1, 0.32, "magenta!30!white")
    return s, (W, H)


def star(x, y, r, fill):
    pts_ = []
    for k in range(8):
        a = math.radians(90 + k * 45)
        rr = r if k % 2 == 0 else r * 0.38
        pts_.append((x + rr * math.cos(a), y + rr * math.sin(a)))
    return poly(pts_, fill)


def pip():
    rng = random.Random(303)
    skin, hair = "skinb", "hairB"
    s = backdrop("ranged", rng)
    s += bust("goldlight", stripes="orange")
    s += neck(skin)
    s += head(skin)
    s += face(eyes="dot", mouth="grin", brows="up")
    # freckles
    for (x, y) in ((2.25, 3.25), (2.4, 3.18), (2.3, 3.08), (3.6, 3.18), (3.75, 3.25), (3.7, 3.08)):
        s += f"\\fill[wooddark] {P(x, y)} circle (0.035);\n"
    # a cap on backwards: dome and the brim behind
    s += f"\\filldraw[inked,fill=teal] {P(1.85, 4.05)} .. controls {P(1.9, 5.05)} and {P(4.1, 5.05)} .. {P(4.15, 4.05)} -- cycle;\n"
    s += f"\\filldraw[inked,fill=tealdark] {P(3.9, 4.25)} -- {P(4.75, 4.05)} -- {P(4.7, 4.3)} -- {P(4.05, 4.6)} -- cycle;\n"
    s += line((3.0, 4.82), (3.0, 4.1), lw=0.6)
    s += circle(3.0, 4.82, 0.07, "tealdark", lw(0.5))
    # tufts of hair under it
    s += poly([(1.88, 4.05), (1.75, 3.7), (2.05, 3.9), (2.0, 3.6), (2.3, 4.05)], hair)
    # the slingshot, drawn back
    s += limb([(1.0, -0.4), (1.1, 0.5), (1.55, 1.05)], 0.36, 0.3, "goldlight")
    s += tube([[(1.55, 0.9), (1.6, 1.7)], [(1.6, 1.7), (1.25, 2.4)], [(1.6, 1.7), (2.0, 2.45)]], 0.13, "wood")
    s += mitten(1.55, 1.0, 0.28, skin)
    s += line((1.25, 2.4), (3.9, 1.6), lw=0.9, colour="redlight")
    s += line((2.0, 2.45), (3.9, 1.6), lw=0.9, colour="redlight")
    s += circle(3.95, 1.58, 0.16, "metallight", lw(0.9))
    s += limb([(5.0, -0.4), (4.8, 0.6), (4.15, 1.45)], 0.36, 0.3, "goldlight")
    s += mitten(4.1, 1.5, 0.28, skin)
    return s, (W, H)


def nan():
    rng = random.Random(404)
    skin, hair = "skin", "hairC"
    s = backdrop("magic", rng)
    s += hair_back("bun", hair)
    s += bust("wallC", collar=None)
    # a knitted shawl
    s += f"\\filldraw[inked,fill=goldlight] {P(1.0, 1.75)} .. controls {P(2.0, 2.35)} and {P(4.0, 2.35)} .. {P(5.0, 1.75)} -- {P(3.0, 0.2)} -- cycle;\n"
    for k in range(5):
        s += curve([(1.6 + k * 0.7, 1.95), (1.9 + k * 0.6, 1.2), (3.0, 0.4)], lw=0.5, colour="ink!60!goldlight")
    s += neck(skin)
    s += head(skin)
    s += face(eyes="shut", mouth="smile", brows="flat", glasses=True)
    s += curve([(2.2, 3.0), (2.35, 2.85)], lw=0.6)
    s += curve([(3.8, 3.0), (3.65, 2.85)], lw=0.6)
    s += hair_front("bun", hair)
    # the paper lantern, glowing, held in both hands in front of her
    lx, ly = 3.0, 1.05
    s += fillo(f"{P(lx, ly)} circle (1.25)", "gold", 0.2)
    s += fillo(f"{P(lx, ly)} circle (0.9)", "goldlight", 0.3)
    s += rect(lx - 0.25, ly + 0.82, lx + 0.25, ly + 0.97, "ink!70!red", lw(0.8))
    s += ellipse(lx, ly, 0.62, 0.85, "red!85!orange")
    for dy in (-0.5, -0.17, 0.17, 0.5):
        half = 0.6 * math.sqrt(max(0.0, 1 - (dy / 0.85) ** 2))
        s += curve([(lx - half, ly + dy), (lx, ly + dy - 0.05), (lx + half, ly + dy)], lw=0.6)
    s += rect(lx - 0.25, ly - 0.97, lx + 0.25, ly - 0.82, "ink!70!red", lw(0.8))
    s += glint(lx - 0.2, ly + 0.35, 0.08)
    s += mitten(lx - 0.72, ly - 0.25, 0.27, skin)
    s += mitten(lx + 0.72, ly - 0.25, 0.27, skin)
    return s, (W, H)


def ed():
    rng = random.Random(505)
    skin, hair = "skin", "hairA"
    s = backdrop("melee", rng)
    # the exosuit's frame behind the shoulders
    s += tube([[(0.7, -0.4), (0.8, 2.3)], [(5.3, -0.4), (5.2, 2.3)]], 0.32, "chumlight")
    s += rrect(0.4, 2.1, 1.3, 2.75, "chum")
    s += rrect(4.7, 2.1, 5.6, 2.75, "chum")
    s += bust("orange", shoulders=1.1)
    # open shirt over the tee
    s += poly([(1.0, -0.4), (1.2, 1.6), (1.8, 2.15), (2.5, 2.3), (2.6, -0.4)], "goldlight")
    s += poly([(5.0, -0.4), (4.8, 1.6), (4.2, 2.15), (3.5, 2.3), (3.4, -0.4)], "goldlight")
    s += rect(2.6, 0.4, 3.4, 0.6, "ink!70!wooddark", lw(0.7))
    s += rect(2.9, 0.35, 3.1, 0.65, "gold", lw(0.6))
    s += neck(skin)
    s += head(skin)
    s += face(eyes="squint", mouth="grin", brows="worried", sweat=True)
    s += hair_front("long", hair)
    # a visor plate with one glowing lens, as in the inspiration picture
    s += rrect(3.1, 3.35, 4.25, 3.75, "chumdark", 0.1)
    s += circle(3.75, 3.55, 0.15, "cyan", lw(0.8))
    s += line((4.25, 3.55), (4.4, 3.9), lw=1.2)
    # pistons on the forearms
    for x0, sgn in ((1.0, 1), (5.0, -1)):
        s += rrect(x0 - 0.35, 0.0, x0 + 0.35, 1.2, "chum", 0.1)
        s += rect(x0 - 0.12, 1.2, x0 + 0.12, 1.75, "metallight", lw(0.9))
        s += circle(x0, 0.35, 0.1, "metal", lw(0.6))
        s += circle(x0, 0.85, 0.1, "metal", lw(0.6))
    return s, (W, H)


def static():
    rng = random.Random(606)
    skin, hair = "skinb", "hairD"
    s = backdrop("magic", rng)
    # static zigzags in the air
    for (x0, y0, c) in ((0.3, 4.2, "cyan"), (4.6, 4.9, "gold"), (4.9, 1.9, "cyan")):
        zz = [(x0 + k * 0.2, y0 + (0.18 if k % 2 else -0.18)) for k in range(6)]
        s += f"\\draw[{c}, line width=2.4pt, line join=round] {' -- '.join(P(*p) for p in zz)};\n"
    s += bust("tealdark", collar=None)
    # a hoodie's drawstrings and pocket line
    s += line((2.7, 2.2), (2.65, 1.3), lw=1.0)
    s += line((3.3, 2.2), (3.35, 1.3), lw=1.0)
    s += curve([(1.8, 0.4), (3.0, 0.65), (4.2, 0.4)], lw=0.9)
    s += neck(skin)
    s += head(skin)
    s += face(eyes="big", mouth="open", brows="up")
    s += hair_front("spiky", hair)
    # rabbit-ear antenna on a headband
    s += curve([(1.9, 4.2), (3.0, 4.95), (4.1, 4.2)], lw=2.2, colour="metaldark")
    s += line((2.6, 4.85), (1.9, 5.9), lw=1.8, colour="metaldark")
    s += line((3.4, 4.85), (4.2, 5.85), lw=1.8, colour="metaldark")
    s += circle(1.9, 5.9, 0.12, "red", lw(0.7))
    s += circle(4.2, 5.85, 0.12, "red", lw(0.7))
    s += rrect(2.75, 4.75, 3.25, 5.0, "metal", 0.05, lw(0.8))
    return s, (W, H)


def sage():
    rng = random.Random(707)
    skin = "skinb"
    s = backdrop("magic", rng)
    # smoke curling up behind
    for (x, y, r) in ((0.9, 2.6, 0.45), (0.6, 3.4, 0.55), (1.0, 4.3, 0.5), (5.2, 3.0, 0.5), (5.4, 3.9, 0.45), (5.0, 4.8, 0.55)):
        s += circle(x, y, r, "metallight!70!magicsoft", lw(1.0))
    s += bust("asphaltdark", collar=None)
    s += neck(skin)
    s += head(skin, cy=3.4)
    s += face(cy=3.4, eyes="shut", mouth="flat", brows="flat", blush=False)
    # long beard over the chest
    s += f"\\filldraw[inked,fill=hairC] {P(2.0, 3.0)} .. controls {P(1.9, 1.5)} and {P(2.6, 0.4)} .. {P(3.0, -0.1)} .. controls {P(3.4, 0.4)} and {P(4.1, 1.5)} .. {P(4.0, 3.0)} .. controls {P(3.6, 2.6)} and {P(2.4, 2.6)} .. cycle;\n"
    s += curve([(2.6, 2.2), (2.8, 1.3), (3.0, 0.7)], lw=0.6)
    s += curve([(3.4, 2.2), (3.2, 1.3), (3.0, 0.7)], lw=0.6)
    s += f"\\filldraw[inked,fill=hairC] {P(2.45, 2.85)} .. controls {P(2.7, 3.1)} and {P(3.3, 3.1)} .. {P(3.55, 2.85)} .. controls {P(3.2, 2.7)} and {P(2.8, 2.7)} .. cycle;\n"
    # a chimney of a hat, smoking
    s += rect(2.1, 4.35, 3.9, 4.6, "wallE")
    s += rect(2.35, 4.6, 3.65, 5.5, "wallE")
    for y in (4.85, 5.15):
        s += line((2.35, y), (3.65, y), lw=0.6)
    s += line((3.0, 4.6), (3.0, 4.85), lw=0.6)
    s += line((2.7, 4.85), (2.7, 5.15), lw=0.6)
    s += line((3.3, 4.85), (3.3, 5.15), lw=0.6)
    s += circle(3.2, 5.75, 0.25, "metallight", lw(0.9))
    s += circle(3.55, 5.95, 0.2, "metallight", lw(0.9))
    return s, (W, H)


def june():
    rng = random.Random(808)
    skin, hair = "skin", "hairA"
    s = backdrop("ranged", rng)
    # twin jet engines over the shoulders, firing
    for x0 in (1.0, 5.0):
        s += leaf((x0, 2.3), (x0 + (0.2 if x0 < 3 else -0.2), 1.2), 0.25, "orange")
        s += leaf((x0, 2.3), (x0 + (0.12 if x0 < 3 else -0.12), 1.55), 0.13, "goldlight")
        s += rrect(x0 - 0.38, 2.3, x0 + 0.38, 4.0, "metallight", 0.18)
        s += rect(x0 - 0.42, 2.2, x0 + 0.42, 2.45, "metaldark", lw(0.9))
        s += circle(x0, 3.7, 0.18, "metal", lw(0.7))
        s += line((x0 - 0.38, 3.1), (x0 + 0.38, 3.1), lw=0.7)
    s += hair_back("bob", hair)
    s += bust("greenB", stripes="ink!80!greenE")
    # harness straps
    s += line((2.3, 2.2), (2.0, -0.4), lw=2.6, colour="ink")
    s += line((3.7, 2.2), (4.0, -0.4), lw=2.6, colour="ink")
    s += rect(1.8, 0.7, 4.2, 0.95, "metaldark", lw(0.8))
    s += neck(skin)
    s += head(skin)
    s += face(eyes="big", mouth="open", brows="worried")
    s += hair_front("bob", hair)
    # goggles pushed up
    s += rect(1.95, 4.2, 4.05, 4.35, "wooddark", lw(0.8))
    s += circle(2.55, 4.35, 0.27, "cyan!40!white")
    s += circle(3.45, 4.35, 0.27, "cyan!40!white")
    # a bottle rocket in hand
    s += mitten(4.6, 1.2, 0.28, skin)
    s += line((4.6, 1.1), (5.3, 2.9), lw=0.9, colour="wood")
    s += rrect(4.95, 2.5, 5.4, 3.4, "red", 0.05)
    s += poly([(4.95, 3.4), (5.175, 3.75), (5.4, 3.4)], "gold")
    return s, (W, H)


def dee():
    rng = random.Random(909)
    skin, hair = "skinc", "hairA"
    s = backdrop("ranged", rng)
    s += bust("sagedark", collar="sage")
    s += rrect(3.6, 0.8, 4.3, 1.5, "wood", 0.06)  # a pocket of spare props
    s += neck(skin)
    s += head(skin)
    s += hair_back("bun", hair)
    s += face(eyes="dot", mouth="smile", brows="flat")
    s += hair_front("bun", hair)
    # headset and boom mic
    s += curve([(1.75, 3.6), (3.0, 5.05), (4.25, 3.6)], lw=2.4, colour="metaldark")
    s += rrect(1.55, 3.2, 1.95, 3.85, "metaldark", 0.12)
    s += rrect(4.05, 3.2, 4.45, 3.85, "metaldark", 0.12)
    s += curve([(1.75, 3.25), (2.0, 2.8), (2.55, 2.85)], lw=1.4, colour="metaldark")
    s += circle(2.6, 2.86, 0.08, "ink")
    # the controller in two hands
    s += rrect(1.9, 0.35, 4.1, 1.05, "metallight", 0.25)
    s += circle(2.4, 0.7, 0.18, "metal", lw(0.7))
    s += circle(3.6, 0.7, 0.18, "metal", lw(0.7))
    s += line((3.0, 1.05), (3.15, 1.6), lw=1.0)
    s += mitten(1.95, 0.6, 0.28, skin)
    s += mitten(4.05, 0.6, 0.28, skin)
    # the quadcopter over the shoulder: four rotors, a pellet gun underneath
    s += quad(4.8, 4.9, 0.85)
    return s, (W, H)


ROTORS = 4


def quad(x, y, sc):
    n = 0
    s = ""
    for dx in (-1, 1):
        s += line((x, y), (x + dx * 0.75 * sc, y + 0.12 * sc), lw=2.0, colour="ink")
        s += line((x, y), (x + dx * 0.45 * sc, y + 0.3 * sc), lw=2.0, colour="ink")
    for (dx, dy) in ((-0.75, 0.12), (0.75, 0.12), (-0.45, 0.3), (0.45, 0.3)):
        rx, ry = x + dx * sc, y + dy * sc
        s += ellipse(rx, ry + 0.08 * sc, 0.38 * sc, 0.07 * sc, "white", extra=f",opacity=0.85{lw(0.7)}")
        s += rect(rx - 0.04 * sc, ry - 0.05 * sc, rx + 0.04 * sc, ry + 0.08 * sc, "metaldark", lw(0.5))
        n += 1
    assert n == ROTORS, "a quadcopter has four rotors"
    s += rrect(x - 0.35 * sc, y - 0.2 * sc, x + 0.35 * sc, y + 0.15 * sc, "goldlight", 0.08)
    s += circle(x, y - 0.03 * sc, 0.09 * sc, "red", lw(0.5))
    s += rect(x - 0.08 * sc, y - 0.45 * sc, x + 0.3 * sc, y - 0.2 * sc, "metaldark", lw(0.6))
    return s


def tagger():
    rng = random.Random(1010)
    fur, belly = "metal", "metallight"
    s = backdrop("ranged", rng)
    # paint splats on the wall behind
    for (x, y, r, c) in ((0.8, 3.6, 0.45, "magenta"), (5.2, 4.2, 0.4, "gold"), (5.0, 1.9, 0.35, "cyan")):
        pts_ = [(x + r * (1 + 0.35 * math.sin(k * 2.3)) * math.cos(k * math.pi / 5), y + r * (1 + 0.35 * math.sin(k * 2.3)) * math.sin(k * math.pi / 5)) for k in range(10)]
        s += blob(pts_, c, tension=0.8)
    # hoodie
    s += bust("wallF", collar=None)
    s += f"\\filldraw[inked,fill=wallF!80!ink] {P(1.6, 2.3)} .. controls {P(1.4, 4.3)} and {P(4.6, 4.3)} .. {P(4.4, 2.3)} -- cycle;\n"
    # raccoon head: round ears, a face mask, a striped tail behind the shoulder
    tail = [(4.6, 0.2), (5.3, 1.0), (5.6, 2.2), (5.4, 3.0)]
    s += limb(tail, 0.35, 0.25, fur, cap=True)
    for k in range(3):
        a, b = tail[k], tail[k + 1]
        m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        s += circle(m[0], m[1], 0.22, "ink!80!metaldark", lw(0.5))
    s += circle(2.15, 4.55, 0.38, fur)
    s += circle(3.85, 4.55, 0.38, fur)
    s += circle(2.15, 4.55, 0.2, "ink!80!metaldark", lw(0.5))
    s += circle(3.85, 4.55, 0.2, "ink!80!metaldark", lw(0.5))
    s += ellipse(3.0, 3.5, 1.15, 1.1, fur)
    s += f"\\filldraw[inked,fill=ink!80!metaldark] {P(1.95, 3.6)} .. controls {P(2.2, 4.0)} and {P(2.8, 3.9)} .. {P(3.0, 3.7)} .. controls {P(3.2, 3.9)} and {P(3.8, 4.0)} .. {P(4.05, 3.6)} .. controls {P(3.8, 3.15)} and {P(3.25, 3.25)} .. {P(3.0, 3.4)} .. controls {P(2.75, 3.25)} and {P(2.2, 3.15)} .. cycle;\n"
    s += f"\\filldraw[inked,fill={belly}] {P(2.35, 3.15)} .. controls {P(2.5, 2.4)} and {P(3.5, 2.4)} .. {P(3.65, 3.15)} .. controls {P(3.3, 3.35)} and {P(2.7, 3.35)} .. cycle;\n"
    for ex in (2.55, 3.45):
        s += circle(ex, 3.58, 0.13, "white", lw(0.5))
        s += circle(ex, 3.56, 0.07, "ink")
    s += ellipse(3.0, 3.02, 0.14, 0.09, "ink")
    s += curve([(2.8, 2.78), (3.0, 2.7), (3.2, 2.78)], lw=1.0)
    # the spray can, mid-spray
    s += fillo(f"{P(1.0, 3.1)} circle (0.6)", "magenta", 0.3)
    s += fillo(f"{P(0.7, 3.3)} circle (0.35)", "magenta", 0.35)
    s += rrect(1.2, 1.4, 1.75, 2.75, "magenta", 0.08)
    s += rect(1.3, 2.75, 1.65, 2.95, "metallight", lw(0.7))
    s += rect(1.38, 2.95, 1.57, 3.08, "ink")
    s += rect(1.2, 1.8, 1.75, 2.05, "white", lw(0.6))
    s += mitten(1.5, 1.4, 0.3, fur)
    return s, (W, H)


def chum():
    """The mech cat's head, from the inspiration picture: green riveted
    plates, tall ears, a forehead plate with no lettering, round segmented
    eyes and an orange mouth."""
    rng = random.Random(1111)
    s = backdrop("melee", rng)
    # scrap at the bottom
    for k in range(0 if BARE[0] else 9):
        x = rng.uniform(0, 6)
        s += poly([(x, -0.2), (x + rng.uniform(0.3, 0.8), rng.uniform(0.3, 0.9)), (x + rng.uniform(0.9, 1.5), -0.2)], rng.choice(["wallE", "metal", "wood", "sagedark"]))
    g, gd, gl = "chum", "chumdark", "chumlight"
    # ears
    s += poly([(1.05, 3.2), (0.6, 5.85), (2.35, 4.6)], g)
    s += poly([(1.25, 3.6), (0.95, 5.25), (2.0, 4.5)], gd)
    s += poly([(4.95, 3.2), (5.4, 5.85), (3.65, 4.6)], g)
    s += poly([(4.75, 3.6), (5.05, 5.25), (4.0, 4.5)], gd)
    # head
    s += f"\\filldraw[inked,fill={g}] {P(0.7, 2.9)} .. controls {P(0.6, 4.9)} and {P(5.4, 4.9)} .. {P(5.3, 2.9)} .. controls {P(5.2, 1.0)} and {P(4.0, 0.1)} .. {P(3.0, 0.1)} .. controls {P(2.0, 0.1)} and {P(0.8, 1.0)} .. cycle;\n"
    # plate seams and camo patches
    for (x, y, rx, ry) in ((1.3, 2.4, 0.35, 0.22), (4.6, 2.2, 0.3, 0.2), (2.2, 0.9, 0.3, 0.18), (4.0, 0.9, 0.25, 0.17), (3.9, 4.15, 0.3, 0.15)):
        s += ellipse(x, y, rx, ry, gl, extra=lw(0.7))
    s += curve([(0.9, 3.4), (2.0, 3.0), (3.0, 2.95), (4.0, 3.0), (5.1, 3.4)], lw=0.9)
    # forehead plate, bolted, blank
    s += rrect(2.0, 3.55, 4.0, 4.2, gl, 0.1)
    for (x, y) in ((2.12, 3.67), (3.88, 3.67), (2.12, 4.08), (3.88, 4.08)):
        s += circle(x, y, 0.06, "metal", lw(0.5))
    s += rect(2.35, 3.8, 3.65, 3.95, "orange", lw(0.6))
    # eyes: round, segmented
    for ex in (1.85, 4.15):
        s += circle(ex, 2.45, 0.62, gl)
        s += circle(ex, 2.45, 0.42, "cyan!35!white")
        s += line((ex - 0.42, 2.45), (ex + 0.42, 2.45), lw=0.8)
        s += line((ex, 2.03), (ex, 2.87), lw=0.8)
        s += circle(ex, 2.45, 0.14, "ink")
        s += glint(ex + 0.12, 2.6, 0.06)
    # nose vent and the orange mouth with teeth
    for k in range(3):
        s += line((2.75, 2.15 - k * 0.13), (3.25, 2.15 - k * 0.13), lw=1.2)
    s += f"\\filldraw[inked,fill=orange] {P(1.75, 1.35)} .. controls {P(2.4, 0.55)} and {P(3.6, 0.55)} .. {P(4.25, 1.35)} .. controls {P(3.6, 1.15)} and {P(2.4, 1.15)} .. cycle;\n"
    for k in range(5):
        x = 2.15 + k * 0.42
        s += poly([(x - 0.13, 1.22), (x, 0.98), (x + 0.13, 1.22)], "white")
    # rivets round the rim
    for k in range(14):
        a = math.radians(200 + k * (140 / 13))
        s += circle(3.0 + 2.05 * math.cos(a) * -1, 2.6 + 1.9 * math.sin(a) * -1, 0.06, "metal", lw(0.5))
    # a hose to the side
    s += curve([(5.2, 1.6), (5.7, 1.2), (5.8, 0.4)], lw=5.0, colour="ink")
    s += curve([(5.2, 1.6), (5.7, 1.2), (5.8, 0.4)], lw=3.2, colour="wallE")
    return s, (W, H)


def biscuit():
    """A cream poodle, all curls, from the corner of the inspiration picture."""
    rng = random.Random(1212)
    s = backdrop("melee", rng)
    c, cd = "poodle", "poodledark"

    def puffs(cx, cy, rx, ry, n, r):
        o = ""
        for k in range(n):
            a = 2 * math.pi * k / n
            o += circle(cx + rx * math.cos(a), cy + ry * math.sin(a), r, c)
        o += f"\\fill[{c}] {P(cx, cy)} ellipse ({f(rx)} and {f(ry)});\n"
        return o

    # body and chest
    s += puffs(3.0, 0.6, 1.8, 0.9, 12, 0.5)
    # ears hang either side
    s += puffs(1.75, 2.6, 0.35, 0.75, 7, 0.32)
    s += puffs(4.25, 2.6, 0.35, 0.75, 7, 0.32)
    # head and topknot
    s += puffs(3.0, 3.1, 0.95, 0.9, 11, 0.42)
    s += puffs(3.0, 4.35, 0.55, 0.4, 8, 0.33)
    # muzzle
    s += ellipse(3.0, 2.55, 0.62, 0.45, c)
    s += curve([(2.6, 2.35), (2.8, 2.25), (3.0, 2.38), (3.2, 2.25), (3.4, 2.35)], lw=1.0)
    s += ellipse(3.0, 2.78, 0.17, 0.12, "ink")
    s += glint(3.05, 2.82, 0.04)
    # tongue
    s += f"\\filldraw[inked,fill=pink] {P(2.85, 2.28)} .. controls {P(2.85, 1.95)} and {P(3.15, 1.95)} .. {P(3.15, 2.28)} -- cycle;\n"
    for ex in (2.6, 3.4):
        s += circle(ex, 3.25, 0.13, "ink", lw(0.5))
        s += glint(ex + 0.04, 3.3, 0.045)
    s += fillo(ellipse_path(2.3, 2.85, 0.2, 0.1), "pink", 0.5)
    s += fillo(ellipse_path(3.7, 2.85, 0.2, 0.1), "pink", 0.5)
    # a collar with a tag
    s += f"\\filldraw[inked,fill=red] {P(2.0, 1.6)} .. controls {P(2.5, 1.35)} and {P(3.5, 1.35)} .. {P(4.0, 1.6)} -- {P(4.0, 1.85)} .. controls {P(3.5, 1.6)} and {P(2.5, 1.6)} .. {P(2.0, 1.85)} -- cycle;\n"
    s += circle(3.0, 1.32, 0.17, "gold", lw(0.8))
    # outline the curls last, lightly, so the fluff reads as one shape
    for (cx, cy, rx, ry, n, r) in ((3.0, 4.35, 0.55, 0.4, 8, 0.33),):
        for k in range(n):
            a = 2 * math.pi * k / n
            if math.sin(a) > -0.2:
                s += f"\\draw[ink, line width=0.8pt] {P(cx + rx * math.cos(a), cy + ry * math.sin(a))} circle ({f(r)});\n"
    for k in range(10):
        x, y = rng.uniform(1.4, 4.6), rng.uniform(0.0, 1.2)
        s += curve([(x - 0.12, y), (x, y + 0.1), (x + 0.12, y)], lw=0.6, colour=cd)
    return s, (W, H)


check_kinds()

PICTURES = {f"units/{k}": v for k, v in {
    "rook": rook, "vela": vela, "pip": pip, "nan": nan, "ed": ed, "static": static,
    "chum": chum, "june": june, "sage": sage, "biscuit": biscuit, "dee": dee, "tagger": tagger,
}.items()}
