"""Shared TikZ helpers for Level Twenty's 2D art.

Copied unchanged from Raiders of the Lost Mart's tools/artlib.py, which took
it from Ribbit King's tools/art.py (NickHGame), which follows
LongTimeComin/tools/art.py: every picture is TikZ text compiled with
pdflatex, never generated pixels (Sam's diagrams-as-code prompt,
GearMaster2D/tikz_figure_prompt.md). The look follows
vagrancy/Sam_Comm_final.png: a warm yellow sky with outlined puffy clouds,
salmon, coral and pink buildings, sage and teal machines, flat fills, every
shape outlined in one plum ink with round joins, and lots of small clutter.

Picture modules (art_units.py, art_scenes.py, art_icons.py) import this
file and register pictures in their own PICTURES dict; tools/art.py builds
them all.
"""
import math, random

COLOURS = {
    # Sam_Comm_final.png by eye
    "ink": "3B2340", "sky": "EFC46A", "skyhi": "F5DA92", "skylow": "E9B060", "cloud": "F2D58A",
    "paper": "FBEFD0", "cream": "FFF6E0", "white": "FFFFFF",
    "wallA": "E88A73", "wallB": "EE9F7E", "wallC": "E47A7E", "wallD": "F0B07E", "wallE": "D9765F", "wallF": "E99A8A",
    "greenA": "8DBB7E", "greenB": "6FA77A", "greenC": "A9C97F", "greenD": "5E9C6A", "greenE": "4A8060",
    "sage": "9DB98A", "sagedark": "6F8F66", "sagelight": "C2D6A6",
    "window": "F6E7A0", "windark": "8C5A64", "metal": "8D9A93", "metaldark": "5E6B66", "metallight": "B9C4BC",
    "wood": "B9835A", "wooddark": "8C5A3C", "woodlight": "D9A66F",
    "stone": "E7C9A0", "stonedark": "C9A47E", "concrete": "D8C3A4", "concretedark": "B8A084",
    "asphalt": "8E7A86", "asphaltdark": "6E5C68", "roadline": "F6E7A0", "kerb": "D8B48C",
    "red": "E0483E", "redlight": "F27A6A", "gold": "FFB000", "goldlight": "FFD54F",
    "teal": "4FA3A0", "tealdark": "3C8582", "magenta": "FF2E88", "cyan": "13CFE3", "violet": "8A5BD6",
    "orange": "F2803A", "pink": "F48FB1",
    # weak spots glow; armour is dull
    "glow": "4FE6E0", "glowhi": "C8FFF9", "hot": "FF6A1A", "hothi": "FFD07A",
    "armour": "7E7A86", "armourdark": "5A5466", "armourlight": "A8A2B0",
    # the drones (data/drones.json carries the same values)
    "wasp": "E0A82E", "waspdark": "B07E1A", "moth": "F2E6C8", "mothdark": "D2C09A",
    "hornet": "3C8582", "hornetdark": "2C6260", "warden": "E47A7E", "wardendark": "B85A62",
    "tick": "8DBB7E", "tickdark": "5E8A5A", "strider": "D9765F", "striderdark": "A8543F",
    "queen": "7E5C94", "queendark": "4C3A60",
    # the raiders
    "jacket": "6FA77A", "jacketb": "4FA3A0", "jacketc": "E88A73", "jacketd": "F0B07E",
    "skin": "F2C49A", "skinb": "C98C6A", "skinc": "8C5A3C", "hair": "3B2340",
    "goo": "B6E35A", "goodark": "7FB23A",
    # Level Twenty: the three kinds (web/style.css carries the same values)
    "melee": "E0483E", "meleesoft": "F6B3A4", "magic": "8A5BD6", "magicsoft": "CDB6F0",
    "ranged": "2F9A8A", "rangedsoft": "A6DCCF",
    "chum": "6FB07A", "chumdark": "4E8A5E", "chumlight": "A8D49A", "poodle": "F6DDB0", "poodledark": "E2BF86",
    "hairA": "3B2340", "hairB": "6E3F2C", "hairC": "C9C2CF", "hairD": "D9A04A", "lips": "C25A5A",
}

PREAMBLE = r"""\documentclass[tikz,border=0pt]{standalone}
\usepackage{xcolor}
\usetikzlibrary{calc}
% Palette: Sam_Comm_final.png by eye, and the drones and raiders.
""" + "\n".join(f"\\definecolor{{{k}}}{{HTML}}{{{v}}}" for k, v in COLOURS.items()) + "\n"

# The ink's width, in pt, for each folder: small pictures get a bolder line.
INK = {"": 1.15, "backdrops": 1.15, "units": 1.5, "kinds": 1.8}
CUR_INK = 1.15
PT_PER_CM = 28.4528


def f(x):
    return f"{x:.3f}".rstrip("0").rstrip(".") if abs(x) > 0.0005 else "0"


def P(x, y):
    return f"({f(x)},{f(y)})"


def pts(points):
    return " ".join(P(x, y) for x, y in points)


def rect(x0, y0, x1, y1, fill, extra=""):
    return f"\\filldraw[inked,fill={fill}{extra}] {P(x0, y0)} rectangle {P(x1, y1)};\n"


def rrect(x0, y0, x1, y1, fill, r=0.15, extra=""):
    return f"\\filldraw[inked,fill={fill},rounded corners={f(r)}cm{extra}] {P(x0, y0)} rectangle {P(x1, y1)};\n"


def circle(x, y, r, fill, extra=""):
    return f"\\filldraw[inked,fill={fill}{extra}] {P(x, y)} circle ({f(r)});\n"


def ellipse(x, y, rx, ry, fill, rot=0, extra=""):
    turn = f",rotate around={{{f(rot)}:{P(x, y)}}}" if rot else ""
    return f"\\filldraw[inked,fill={fill}{turn}{extra}] {P(x, y)} ellipse ({f(rx)} and {f(ry)});\n"


def ellipse_path(x, y, rx, ry, rot=0):
    turn = f"[rotate around={{{f(rot)}:{P(x, y)}}}]" if rot else ""
    return f"{turn} {P(x, y)} ellipse ({f(rx)} and {f(ry)})"


def poly(points, fill, extra=""):
    path = " -- ".join(P(x, y) for x, y in points)
    return f"\\filldraw[inked,fill={fill}{extra}] {path} -- cycle;\n"


def blob_path(points, tension=0.7):
    return f"plot[smooth cycle, tension={tension}] coordinates {{{pts(points)}}}"


def blob(points, fill, extra="", tension=0.7):
    return f"\\filldraw[inked,fill={fill}{extra}] {blob_path(points, tension)};\n"


def hill(top, base, fill, extra="", tension=0.6):
    """A shape whose top edge is a smooth curve and whose foot is flat."""
    x0, x1 = top[0][0], top[-1][0]
    return f"\\filldraw[inked,fill={fill}{extra}] plot[smooth, tension={tension}] coordinates {{{pts(top)}}} -- {P(x1, base)} -- {P(x0, base)} -- cycle;\n"


def line(a, b, lw=1.1, colour="ink", extra=""):
    return f"\\draw[{colour}, line width={f(lw)}pt, line cap=round{extra}] {P(*a)} -- {P(*b)};\n"


def curve(points, lw=1.1, colour="ink", extra="", tension=0.6):
    return f"\\draw[{colour}, line width={f(lw)}pt, line cap=round, line join=round{extra}] plot[smooth, tension={tension}] coordinates {{{pts(points)}}};\n"


def leaf_path(b, t, width, bend=0.0):
    """A leaf, petal or blade from base b to tip t: two curves through
    control points either side of the (bent) midline."""
    (bx, by), (tx, ty) = b, t
    dx, dy = tx - bx, ty - by
    L = math.hypot(dx, dy) or 1.0
    nx, ny = -dy / L, dx / L
    mx, my = (bx + tx) / 2 + nx * bend, (by + ty) / 2 + ny * bend
    c1 = (mx + nx * width, my + ny * width)
    c2 = (mx - nx * width, my - ny * width)
    return f"{P(bx, by)} .. controls {P(*c1)} .. {P(tx, ty)} .. controls {P(*c2)} .. {P(bx, by)} -- cycle"


def leaf(b, t, width, fill, bend=0.0, extra=""):
    return f"\\filldraw[inked,fill={fill}{extra}] {leaf_path(b, t, width, bend)};\n"


def limb_pts(path, hw0, hw1, cap=False):
    """The outline of a tapering limb along a polyline, as points for a
    smooth closed curve; with cap, the far end is rounded."""
    n = len(path)
    left, right = [], []
    for i, (x, y) in enumerate(path):
        if i == 0:
            dx, dy = path[1][0] - x, path[1][1] - y
        elif i == n - 1:
            dx, dy = x - path[i - 1][0], y - path[i - 1][1]
        else:
            dx, dy = path[i + 1][0] - path[i - 1][0], path[i + 1][1] - path[i - 1][1]
        L = math.hypot(dx, dy) or 1.0
        nx, ny = -dy / L, dx / L
        hw = hw0 + (hw1 - hw0) * i / (n - 1)
        left.append((x + nx * hw, y + ny * hw))
        right.append((x - nx * hw, y - ny * hw))
    if cap:
        (x, y), (px, py) = path[-1], path[-2]
        L = math.hypot(x - px, y - py) or 1.0
        ux, uy = (x - px) / L, (y - py) / L
        left.append((x + ux * hw1 * 0.95, y + uy * hw1 * 0.95))
    return left + right[::-1]


def limb(path, hw0, hw1, fill, extra="", cap=False):
    return blob(limb_pts(path, hw0, hw1, cap), fill, extra, tension=0.5)


def tube(paths, w, fill, tension=0.6):
    """Round-capped tubes w cm wide, outlined as one shape: every ink
    stroke goes down before any fill (cacti, stalks, poles, trunks).
    Absolute coordinates only (a scaled scope would not scale the width)."""
    wpt = w * PT_PER_CM
    out = ""
    for p in paths:
        out += f"\\draw[ink, line width={f(wpt + 2 * CUR_INK)}pt, line cap=round, line join=round] plot[smooth, tension={tension}] coordinates {{{pts(p)}}};\n"
    for p in paths:
        out += f"\\draw[{fill}, line width={f(wpt)}pt, line cap=round, line join=round] plot[smooth, tension={tension}] coordinates {{{pts(p)}}};\n"
    return out


def place(x, y, s, body, rot=0.0, flip=False):
    """Put a sub-picture drawn about its own origin at (x, y), scaled by s,
    turned by rot degrees, mirrored if flip."""
    xs = -s if flip else s
    return f"\\begin{{scope}}[shift={{{P(x, y)}}}, rotate={f(rot)}, xscale={f(xs)}, yscale={f(s)}]\n{body}\\end{{scope}}\n"


def clipped(path, body):
    return f"\\begin{{scope}}\\clip {path};\n{body}\\end{{scope}}\n"


def glint(x, y, r, opacity=0.9):
    return f"\\fill[white, opacity={f(opacity)}] {P(x, y)} circle ({f(r)});\n"


def rot(p, a, o=(0.0, 0.0)):
    c, s = math.cos(math.radians(a)), math.sin(math.radians(a))
    x, y = p[0] - o[0], p[1] - o[1]
    return (o[0] + x * c - y * s, o[1] + x * s + y * c)




def sky(W, H, top="skyhi", mid="sky", low="skylow", a=0.78, b=0.3):
    s = f"\\fill[{mid}] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[{top}] (0,{f(H * a)}) rectangle {P(W, H)};\n"
    s += f"\\fill[{low}] (0,0) rectangle {P(W, H * b)};\n"
    return s


def cloud(x, y, s, fill="cloud"):
    """Puffs, outlined, as the clouds over Sam_Comm_final's city."""
    puffs = [(-1.0, 0, 0.55), (-0.45, 0.25, 0.7), (0.2, 0.35, 0.8), (0.85, 0.15, 0.6), (1.35, -0.05, 0.45)]
    out = ""
    for dx, dy, r in puffs:
        out += circle(x + dx * s, y + dy * s, r * s, fill)
    out += f"\\fill[{fill}] {P(x - 1.0 * s, y - 0.15 * s)} rectangle {P(x + 1.35 * s, y + 0.05 * s)};\n"
    out += line((x - 1.25 * s, y - 0.15 * s), (x + 1.6 * s, y - 0.15 * s), lw=CUR_INK)
    return out


def tuft(x, y, s, fill="greenB"):
    out = ""
    for a, l in ((-28, 0.55), (-8, 0.8), (14, 0.7), (34, 0.5)):
        tip = (x + math.sin(math.radians(a)) * l * s, y + math.cos(math.radians(a)) * l * s)
        out += leaf((x + a * 0.004 * s, y), tip, 0.07 * s, fill, bend=0.05 * s * (1 if a > 0 else -1))
    return out




def check_no_text(name, tex):
    """The no-words rule, checked on the source before it is compiled."""
    for bad in ("\\node", "\\text", "label=", "\\pgftext", "\\put"):
        if bad in tex:
            raise SystemExit(f"{name}: {bad} found; pictures carry no words")


def document(body, size, ink):
    W, H = size
    head = PREAMBLE + f"\\newcommand{{\\Ink}}{{{ink}pt}}\n\\tikzset{{inked/.style={{draw=ink, line width=\\Ink, line join=round, line cap=round}}}}\n"
    return head + "\\begin{document}\n\\begin{tikzpicture}\n" + f"\\clip (0,0) rectangle {P(W, H)};\n" + body + "\\end{tikzpicture}\n\\end{document}\n"


