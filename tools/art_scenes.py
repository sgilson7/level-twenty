"""Level Twenty: the title and the three campaign backdrops, drawn as TikZ
and compiled by tools/art.py (web/art/title.png, 25.6 x 14.4 cm;
web/art/backdrops/<name>.png, 19.2 x 10.8 cm). Never generated pixels.

The look is vagrancy/Sam_Comm_final.png's, closer here than anywhere: a
street of salmon, coral and pink buildings under a warm yellow sky with
outlined puffy clouds, vertical signs, air conditioners, pipes, wires and
antennas, a giant green mech cat in the middle distance, a girl on a
jetpack, a cream poodle, and clutter on the ground.

Facts each picture must get right (what an image generator gets wrong),
and where this file enforces each:

- no words, letters or digits: every sign is coloured blocks and bars,
  including the vertical signs that carry lettering in the inspiration
  picture. tools/art.py's `check_no_text` refuses \\node and \\text.
- the units in the title are the units in the game, drawn by the same
  functions as their portraits (art_units.figure), so a unit cannot look
  one way on the title and another in a fight.
- the title's upper-left corner (x < CALM_X, y > CALM_Y) is only sky and
  clouds, because the page lays the game's name over it (web/style.css
  .hero-text): everything after the sky is clipped out of it by `calm`.
- the backdrops keep their lower half plain enough for six fighter panels
  (web/style.css .arena puts them along the bottom): the ground is flat
  colour with sparse marks, and anything tall stands in the upper half or
  at the edges.
- each campaign backdrop shows the place its chapter names in
  data/campaign.json (`backdrop`): market stalls and awnings for "market",
  roofs with a water tower, antennas and laundry for "rooftops", scrap
  piles, tyres and the sleeping mech cat for "junkyard". `check_names`
  compares the three names with the data file.
- repeatable: every random choice comes from a seeded random.Random.
"""
import json, math, random
from pathlib import Path

from artlib import (P, f, pts, rect, rrect, circle, ellipse, ellipse_path, poly, blob, hill,
                    line, curve, leaf, limb, place, clipped, glint, sky, cloud, tube)
import artlib
import art_units

WALLS = ["wallA", "wallB", "wallC", "wallD", "wallE", "wallF"]
SIGNS = ["red", "teal", "gold", "magenta", "violet", "orange", "pink", "cyan"]


def check_names():
    data = json.loads((Path(__file__).resolve().parents[1] / "data" / "campaign.json").read_text())
    names = [c["backdrop"] for c in data["chapters"]]
    assert names == ["market", "rooftops", "junkyard"], names


def lw(w):
    return f",line width={f(w)}pt"


def fillo(path, colour, op):
    return f"\\fill[{colour}, opacity={f(op)}] {path};\n"


# --------------------------------------------------------------------------
# city pieces
# --------------------------------------------------------------------------

def windows(x0, x1, y0, y1, rng, col, pane=(0.42, 0.5), gap=(0.28, 0.32), lit=0.35):
    s = ""
    y = y1 - gap[1] - pane[1]
    while y > y0 + 0.2:
        x = x0 + gap[0]
        while x + pane[0] < x1 - 0.1:
            r = rng.random()
            if r < lit:
                fill = "window"
            elif r < lit + 0.08:
                fill = "teal!50!window"
            else:
                fill = f"{col}!60!windark"
            s += rect(x, y, x + pane[0], y + pane[1], fill, lw(0.6))
            if fill == "window" and rng.random() < 0.3:
                s += line((x + pane[0] * 0.5, y), (x + pane[0] * 0.5, y + pane[1]), lw=0.5)
            x += pane[0] + gap[0]
        y -= pane[1] + gap[1]
    return s


def ac_unit(x, y, s_=1.0):
    o = rect(x, y, x + 0.75 * s_, y + 0.5 * s_, "metallight", lw(0.8))
    o += circle(x + 0.25 * s_, y + 0.25 * s_, 0.17 * s_, "metal", lw(0.6))
    for k in range(3):
        o += line((x + 0.5 * s_, y + (0.12 + k * 0.13) * s_), (x + 0.7 * s_, y + (0.12 + k * 0.13) * s_), lw=0.5)
    return o


def vsign(x, y, w, h, rng):
    """A vertical shop sign: a coloured board with blocks for lettering."""
    c = rng.choice(SIGNS)
    o = rect(x - 0.05, y + h, x + w + 0.05, y + h + 0.12, "metaldark", lw(0.6))
    o += rect(x, y, x + w, y + h, c)
    o += rect(x + 0.08, y + 0.08, x + w - 0.08, y + h - 0.08, f"{c}!55!white", lw(0.6))
    yy = y + h - 0.25
    while yy - 0.35 > y + 0.1:
        o += rect(x + w * 0.25, yy - 0.3, x + w * 0.75, yy, c, lw(0.5))
        yy -= 0.5
    return o


def building(x0, x1, top, rng, col=None, base=0.0, signs=True, roof=True):
    col = col or rng.choice(WALLS)
    s = rect(x0, base - 0.5, x1, top, col)
    s += rect(x0 - 0.1, top, x1 + 0.1, top + 0.18, f"{col}!75!ink")
    s += windows(x0, x1, base + 0.4, top, rng, col)
    if signs and rng.random() < 0.6:
        sx = rng.choice([x0 + 0.1, x1 - 0.65])
        sy = rng.uniform(base + 1.2, max(base + 1.3, top - 3.0))
        s += vsign(sx, sy, 0.55, min(2.6, top - sy - 0.2), rng)
    if rng.random() < 0.7:
        s += ac_unit(rng.uniform(x0 + 0.1, max(x0 + 0.2, x1 - 0.9)), rng.uniform(base + 0.8, top - 1.0), 0.9)
    if rng.random() < 0.5:  # a pipe down the front
        px = rng.uniform(x0 + 0.2, x1 - 0.2)
        s += tube([[(px, top), (px, base + 0.3)]], 0.12, "metallight")
    if roof:
        for k in range(rng.randint(0, 3)):
            ax = rng.uniform(x0 + 0.2, x1 - 0.2)
            ah = rng.uniform(0.4, 1.0)
            s += line((ax, top + 0.18), (ax, top + 0.18 + ah), lw=0.9)
            s += line((ax - 0.2, top + 0.18 + ah * 0.7), (ax + 0.2, top + 0.18 + ah * 0.7), lw=0.8)
        if rng.random() < 0.4:
            tx = rng.uniform(x0 + 0.3, max(x0 + 0.4, x1 - 1.2))
            s += water_tank(tx, top + 0.18, 0.7)
    return s


def water_tank(x, y, s_):
    o = ""
    for dx in (0.15, 0.85):
        o += line((x + dx * s_, y), (x + dx * s_, y + 0.6 * s_), lw=1.0)
    o += line((x + 0.15 * s_, y + 0.3 * s_), (x + 0.85 * s_, y + 0.3 * s_), lw=0.7)
    o += rrect(x, y + 0.6 * s_, x + s_, y + 1.5 * s_, "wood", 0.08 * s_)
    for k in range(3):
        yy = y + (0.8 + k * 0.25) * s_
        o += line((x, yy), (x + s_, yy), lw=0.6)
    o += poly([(x - 0.05 * s_, y + 1.5 * s_), (x + 0.5 * s_, y + 1.85 * s_), (x + 1.05 * s_, y + 1.5 * s_)], "wooddark")
    return o


def wires(W, rng, y0, y1, n=3):
    s = ""
    for k in range(n):
        a = rng.uniform(y0, y1)
        b = rng.uniform(y0, y1)
        s += curve([(-0.3, a), (W * 0.3, a - 0.5), (W * 0.7, b - 0.5), (W + 0.3, b)], lw=0.8)
    return s


def string_lights(a, b, sag, n, rng):
    (ax, ay), (bx, by) = a, b
    s = curve([a, ((ax + bx) / 2, (ay + by) / 2 - sag), b], lw=0.8)
    for k in range(1, n):
        t = k / n
        x = ax + (bx - ax) * t
        y = ay + (by - ay) * t - sag * 4 * t * (1 - t)
        c = rng.choice(["gold", "pink", "cyan", "goldlight"])
        s += fillo(f"{P(x, y - 0.12)} circle (0.2)", c, 0.3)
        s += circle(x, y - 0.12, 0.08, c, lw(0.5))
    return s


def awning(x0, x1, y, rng, col=None):
    col = col or rng.choice(["red", "teal", "gold", "sage", "wallC"])
    s = poly([(x0, y), (x1, y), (x1 + 0.3, y - 0.8), (x0 - 0.3, y - 0.8)], col)
    n = int((x1 - x0) / 0.5)
    for k in range(n):
        if k % 2 == 0:
            a = x0 + (x1 - x0) * k / n
            b = x0 + (x1 - x0) * (k + 1) / n
            s += poly([(a, y), (b, y), (b + (b - (x0 + x1) / 2) * 0.3 / ((x1 - x0) / 2) * 1.0, y - 0.8), (a + (a - (x0 + x1) / 2) * 0.3 / ((x1 - x0) / 2) * 1.0, y - 0.8)], "white", lw(0.6))
    for k in range(n + 1):
        x = x0 - 0.3 + (x1 - x0 + 0.6) * k / n
        s += f"\\filldraw[inked,fill={col}] {P(x - 0.25, y - 0.8)} arc[start angle=180,end angle=360,radius=0.25] -- cycle;\n"
    return s


def crate(x, y, w, h, fill="wood"):
    s = rect(x, y, x + w, y + h, fill)
    s += line((x, y), (x + w, y + h), lw=0.7)
    s += rect(x, y + h * 0.42, x + w, y + h * 0.58, f"{fill}!80!ink", lw(0.6))
    return s


def fruit_stall(x, y, rng):
    s = rect(x, y, x + 2.2, y + 0.9, "woodlight")
    s += line((x, y + 0.45), (x + 2.2, y + 0.45), lw=0.6)
    for k in range(9):
        c = rng.choice(["red", "orange", "goldlight", "greenC", "pink"])
        s += circle(x + 0.25 + k * 0.22, y + 1.02 + rng.uniform(-0.04, 0.06), 0.13, c, lw(0.6))
    return s


def can(x, y, r=0.15, fill="sagelight", rot_=0):
    return f"\\filldraw[inked,fill={fill},rotate around={{{f(rot_)}:{P(x, y)}}}] {P(x - r, y - r * 1.5)} rectangle {P(x + r, y + r * 1.5)};\n"


def litter(rng, W, y0, y1, n):
    s = ""
    for k in range(n):
        x, y = rng.uniform(0, W), rng.uniform(y0, y1)
        r = rng.random()
        if r < 0.35:
            s += can(x, y, rng.uniform(0.09, 0.14), rng.choice(["sagelight", "red", "goldlight", "metallight"]), rng.uniform(-80, 80))
        elif r < 0.6:
            s += f"\\filldraw[inked,fill=cream,rotate around={{{f(rng.uniform(-30, 30))}:{P(x, y)}}}] {P(x - 0.2, y - 0.12)} rectangle {P(x + 0.2, y + 0.12)};\n"
        elif r < 0.8:
            s += crate(x, y, rng.uniform(0.4, 0.7), rng.uniform(0.3, 0.5), rng.choice(["wallA", "woodlight", "wallF"]))
        else:
            s += circle(x, y, 0.05, "ink!60!wood")
    return s


def street(W, gy, vx, rng, col="concrete"):
    """A paved street seen from the middle of the road, slabs running away."""
    s = rect(-0.5, -0.5, W + 0.5, gy, col)
    s += line((-0.5, gy), (W + 0.5, gy), lw=artlib.CUR_INK)
    for k in range(-14, 15):
        x = vx + k * 1.6
        s += f"\\draw[{col}!70!ink, line width=0.6pt] {P(vx + k * 0.25, gy)} -- {P(x * 1.0 + k * 1.2, -0.5)};\n"
    y = gy
    step = 0.25
    while y > -0.5:
        s += f"\\draw[{col}!70!ink, line width=0.6pt] {P(-0.5, y)} -- {P(W + 0.5, y)};\n"
        y -= step
        step *= 1.45
    return s


def calm(W, H, cx, cy, body):
    """Everything outside the corner x < cx, y > cy."""
    path = f"{P(cx, H + 1)} -- {P(W + 1, H + 1)} -- {P(W + 1, -1)} -- {P(-1, -1)} -- {P(-1, cy)} -- {P(cx, cy)} -- cycle"
    return clipped(path, body)


# --------------------------------------------------------------------------
# the title
# --------------------------------------------------------------------------

def title():
    W, H = 25.6, 14.4
    CALM_X, CALM_Y = 11.5, 11.6
    rng = random.Random(2026)
    base = sky(W, H)
    for (x, y, sc) in ((2.0, 13.3, 0.9), (7.0, 12.4, 0.7), (16.5, 13.4, 1.0), (23.5, 12.6, 0.85)):
        base += cloud(x, y, sc)
    s = ""
    gy = 4.2
    # far buildings in the middle of the street
    x = 7.5
    while x < 18.5:
        w = rng.uniform(1.4, 2.2)
        s += building(x, x + w, rng.uniform(8.5, 11.5), rng, col=rng.choice(["wallD", "wallB", "stone"]), base=gy, signs=False)
        x += w + 0.1
    # Chum 3000 looms over the far end
    s += place(12.8 - 3 * 1.35, 3.6, 1.35, art_units.figure("chum"))
    # near buildings both sides, tall, with signs
    for (x0, x1, top) in ((-0.5, 2.6, 12.0), (2.5, 5.2, 10.4), (5.1, 7.6, 9.0)):
        s += building(x0, x1, top, rng, base=gy - 0.3)
    for (x0, x1, top) in ((18.2, 20.6, 9.4), (20.5, 23.3, 11.2), (23.2, 26.2, 12.8)):
        s += building(x0, x1, top, rng, base=gy - 0.3)
    s += vsign(5.6, 5.6, 0.8, 3.6, rng)
    s += vsign(18.8, 5.0, 0.8, 3.4, rng)
    # billboard on the right, blank shapes
    s += rect(20.0, 11.3, 24.0, 13.4, "cream")
    s += rect(20.2, 11.5, 23.8, 13.2, "pink", lw(0.8))
    for k in range(4):
        s += circle(20.8 + k * 0.85, 12.35, 0.28, rng.choice(["red", "magenta", "gold"]), lw(0.7))
    s += line((21.0, 11.3), (21.0, 10.6), lw=1.2)
    s += line((23.0, 11.3), (23.0, 10.6), lw=1.2)
    s += wires(W, rng, 8.5, 10.5, 3)
    s += string_lights((5.2, 9.4), (18.2, 9.0), 1.2, 16, rng)
    # the street
    s += street(W, gy, 12.8, rng)
    s += litter(rng, W, 0.2, gy - 0.3, 26)
    # June on her jetpack, top right
    s += place(18.9, 7.9, 0.72, june_flying(), rot=-14)
    # the three starters at the front, and Biscuit
    s += place(1.0, -0.3, 1.25, art_units.figure("rook"))
    s += place(9.0, -0.6, 1.3, art_units.figure("vela"))
    s += place(17.2, -0.3, 1.25, art_units.figure("pip"))
    s += place(15.0, -0.2, 0.5, art_units.figure("biscuit"))
    assert CALM_X >= 0.4 * W and H - CALM_Y >= 0.18 * H
    return base + calm(W, H, CALM_X, CALM_Y, s), (W, H)


def june_flying():
    """June with her legs dangling under the bust, as she flies in the
    inspiration picture."""
    s = ""
    for (hip, knee, foot) in (((2.4, -0.2), (2.0, -1.2), (2.3, -2.1)), ((3.6, -0.2), (4.1, -1.1), (4.0, -2.2))):
        s += limb([hip, knee, foot], 0.38, 0.3, "teal")
        s += ellipse(foot[0] + 0.1, foot[1] - 0.15, 0.42, 0.25, "white")
        s += line((foot[0] - 0.25, foot[1] - 0.3), (foot[0] + 0.45, foot[1] - 0.3), lw=1.6, colour="red")
    return s + art_units.figure("june")


# --------------------------------------------------------------------------
# the three backdrops
# --------------------------------------------------------------------------

BW, BH = 19.2, 10.8


def market():
    rng = random.Random(31)
    s = sky(BW, BH)
    for (x, y, sc) in ((2.5, 10.0, 0.7), (9.5, 10.3, 0.55), (16.0, 9.9, 0.8)):
        s += cloud(x, y, sc)
    gy = 3.6
    x = 5.0
    while x < 14.2:
        w = rng.uniform(1.3, 2.0)
        s += building(x, x + w, rng.uniform(6.5, 8.5), rng, col=rng.choice(["wallD", "wallB", "stone"]), base=gy, signs=False)
        x += w + 0.1
    for (x0, x1, top) in ((-0.5, 2.4, 10.0), (2.3, 5.1, 8.6)):
        s += building(x0, x1, top, rng, base=gy - 0.2)
    for (x0, x1, top) in ((14.1, 16.8, 8.8), (16.7, 19.7, 10.2)):
        s += building(x0, x1, top, rng, base=gy - 0.2)
    # stalls under awnings along both sides
    s += awning(0.0, 4.6, 5.4, rng, "red")
    s += awning(14.6, 19.2, 5.4, rng, "teal")
    s += fruit_stall(0.6, 3.5, rng)
    s += fruit_stall(16.0, 3.5, rng)
    s += crate(3.0, 3.4, 0.9, 0.7)
    s += crate(14.9, 3.4, 0.8, 0.6, "wallA")
    s += string_lights((2.4, 7.8), (16.8, 7.6), 1.0, 14, rng)
    s += wires(BW, rng, 8.0, 9.5, 2)
    s += street(BW, gy, 9.6, rng, "concrete")
    s += litter(rng, BW, 0.4, 3.0, 8)
    return s, (BW, BH)


def rooftops():
    rng = random.Random(32)
    s = sky(BW, BH, "skyhi", "sky", "wallD")
    for (x, y, sc) in ((3.0, 9.8, 0.75), (11.0, 10.2, 0.6), (17.0, 9.6, 0.7)):
        s += cloud(x, y, sc)
    # the skyline beyond the roof
    x = -0.3
    while x < BW:
        w = rng.uniform(1.0, 1.9)
        s += building(x, x + w, rng.uniform(4.6, 7.6), rng, col=rng.choice(["wallF", "wallB", "wallD", "stone"]), base=3.2, signs=rng.random() < 0.3)
        x += w + rng.uniform(0.0, 0.3)
    # the roof we stand on: a parapet and tar
    gy = 3.4
    s += rect(-0.5, -0.5, BW + 0.5, gy, "asphalt!70!concrete")
    s += rect(-0.5, gy - 0.1, BW + 0.5, gy + 0.35, "concrete")
    for k in range(int(BW / 1.2) + 1):
        s += line((k * 1.2, gy - 0.1), (k * 1.2, gy + 0.35), lw=0.6)
    # the water tower and the antenna mast at the edges
    s += water_tank(0.4, gy + 0.35, 2.0)
    s += line((17.6, gy + 0.35), (17.6, 9.6), lw=2.0)
    for k in range(4):
        y = 5.0 + k * 1.1
        s += line((17.0 + k * 0.1, y), (18.2 - k * 0.1, y), lw=1.2)
    s += circle(17.6, 9.7, 0.12, "red", lw(0.7))
    s += ac_unit(15.2, gy + 0.35, 1.6)
    s += ac_unit(3.4, gy + 0.35, 1.2)
    # laundry on a line across the top
    s += curve([(2.4, 8.2), (9.6, 7.3), (16.8, 8.0)], lw=0.9)
    for k in range(9):
        t = (k + 0.5) / 9
        x = 2.4 + 14.4 * t
        y = 8.2 - 0.9 * math.sin(math.pi * t) + (8.0 - 8.2) * t
        c = rng.choice(["pink", "teal", "goldlight", "cream", "wallC", "sagelight"])
        if k % 3 == 0:
            s += poly([(x - 0.35, y), (x + 0.35, y), (x + 0.3, y - 0.9), (x - 0.3, y - 0.9)], c)
        elif k % 3 == 1:
            s += poly([(x - 0.4, y), (x + 0.4, y), (x + 0.5, y - 0.3), (x + 0.25, y - 0.25), (x + 0.25, y - 0.8), (x - 0.25, y - 0.8), (x - 0.25, y - 0.25), (x - 0.5, y - 0.3)], c)
        else:
            s += rect(x - 0.18, y - 0.55, x + 0.18, y, c, lw(0.8))
        s += line((x - 0.2, y + 0.05), (x - 0.2, y - 0.1), lw=0.8)
    # pigeons on the parapet
    for x in (5.4, 6.0, 12.3):
        s += ellipse(x, gy + 0.55, 0.22, 0.16, "metallight")
        s += circle(x + 0.18, gy + 0.72, 0.1, "metallight", lw(0.6))
        s += poly([(x + 0.27, gy + 0.72), (x + 0.36, gy + 0.69), (x + 0.27, gy + 0.66)], "gold", lw(0.4))
    s += litter(rng, BW, 0.4, 2.6, 5)
    return s, (BW, BH)


def tyre(x, y, r):
    return ellipse(x, y, r, r * 0.45, "ink!85!asphaltdark") + ellipse(x, y + 0.02, r * 0.5, r * 0.2, "asphalt")


def junkyard():
    rng = random.Random(33)
    s = sky(BW, BH, "skyhi", "sky", "wallD")
    for (x, y, sc) in ((4.0, 10.0, 0.7), (13.5, 10.1, 0.65)):
        s += cloud(x, y, sc)
    # a smoky chimney far off
    s += rect(1.4, 3.5, 2.0, 8.6, "wallE")
    for (x, y, r) in ((1.8, 9.0, 0.4), (2.3, 9.5, 0.5), (3.0, 9.8, 0.55)):
        s += circle(x, y, r, "metallight!60!sky", lw(0.8))
    # the crane
    s += tube([[(15.8, 3.0), (15.8, 9.0)]], 0.35, "gold")
    s += tube([[(15.8, 9.0), (11.5, 9.6)]], 0.25, "gold")
    s += line((12.0, 9.55), (12.0, 7.4), lw=0.9)
    s += ellipse(12.0, 7.2, 0.6, 0.2, "metaldark")
    s += rrect(14.9, 7.6, 16.6, 8.7, "gold", 0.1)
    s += rect(15.1, 7.9, 15.9, 8.5, "window", lw(0.7))
    # the scrap heap and Chum asleep on it
    s += hill([(4.0, 3.0), (6.0, 5.4), (9.0, 6.6), (12.0, 5.8), (14.5, 3.0)], 2.8, "metal")
    for k in range(40):
        x = rng.uniform(4.6, 13.8)
        top = 2.9 + 3.4 * math.sin(math.pi * (x - 4.0) / 10.5) ** 0.8
        y = rng.uniform(3.0, max(3.1, top - 0.3))
        r = rng.random()
        c = rng.choice(["wallE", "metallight", "sage", "wood", "wallA", "tealdark", "metaldark"])
        if r < 0.4:
            s += poly([(x, y), (x + rng.uniform(0.4, 0.9), y + rng.uniform(-0.2, 0.3)), (x + rng.uniform(0.2, 0.7), y + rng.uniform(0.3, 0.6))], c)
        elif r < 0.7:
            s += rect(x, y, x + rng.uniform(0.3, 0.7), y + rng.uniform(0.2, 0.45), c, lw(0.8))
        else:
            s += tyre(x, y, rng.uniform(0.25, 0.4))
    s += place(9.0, 5.2, 0.62, chum_asleep())
    # the fence along the back
    s += rect(-0.5, 2.9, BW + 0.5, 3.15, "metaldark")
    gy = 3.0
    s += rect(-0.5, -0.5, BW + 0.5, gy, "stonedark")
    s += line((-0.5, gy), (BW + 0.5, gy), lw=artlib.CUR_INK)
    # tyre stacks at the edges and a wrecked car
    for (x, n) in ((0.8, 4), (2.1, 3), (17.8, 5)):
        for k in range(n):
            s += tyre(x, gy - 0.4 + k * 0.42, 0.62)
    s += poly([(13.6, 2.6), (14.0, 3.6), (16.2, 3.7), (17.0, 2.8), (17.0, 2.3), (13.6, 2.3)], "teal")
    s += rect(14.3, 3.0, 15.0, 3.45, "window", lw(0.7))
    s += rect(15.2, 3.0, 16.1, 3.5, "cyan!30!white", lw(0.7))
    s += tyre(14.3, 2.3, 0.4)
    s += tyre(16.3, 2.3, 0.4)
    s += litter(rng, BW, 0.4, 2.4, 7)
    return s, (BW, BH)


def chum_asleep():
    """The mech cat curled on the heap, eyes shut, in the middle distance."""
    g, gd, gl = "chum", "chumdark", "chumlight"
    s = ellipse(1.6, 0.4, 4.2, 1.6, g)  # body
    s += limb([(4.8, 0.0), (5.6, 1.0), (5.2, 2.0), (4.4, 1.9)], 0.35, 0.25, g, cap=True)  # tail
    for k in range(4):
        s += ellipse(-0.8 + k * 1.4, 0.6, 0.3, 0.18, gl, extra=lw(0.7))
    s += poly([(-2.6, 1.6), (-2.9, 3.4), (-1.7, 2.4)], g)
    s += poly([(-0.6, 1.8), (-0.2, 3.5), (-1.3, 2.6)], g)
    s += ellipse(-1.5, 1.2, 1.7, 1.35, g)
    s += rrect(-2.2, 1.85, -0.8, 2.25, gl, 0.08)
    for ex in (-2.1, -0.9):
        s += curve([(ex - 0.32, 1.25), (ex, 1.05), (ex + 0.32, 1.25)], lw=2.2)
    s += f"\\filldraw[inked,fill=orange] {P(-2.0, 0.55)} .. controls {P(-1.7, 0.3)} and {P(-1.3, 0.3)} .. {P(-1.0, 0.55)} -- cycle;\n"
    # Zs would be letters; sleeping is said with bubbles instead
    for (x, y, r) in ((0.0, 2.6, 0.15), (0.4, 3.1, 0.22), (0.95, 3.7, 0.3)):
        s += circle(x, y, r, "white", lw(0.8))
    return s


check_names()

PICTURES = {
    "title": title,
    "backdrops/market": market,
    "backdrops/rooftops": rooftops,
    "backdrops/junkyard": junkyard,
}
