#!/usr/bin/env python3
"""Level Twenty's 2D art, drawn as TikZ and compiled, never
generated as pixels (Sam's diagrams-as-code prompt,
GearMaster2D/tikz_figure_prompt.md: write compilable standalone TikZ, list
what an image generator gets wrong, and check the output against that list).
The pipeline is Raiders of the Lost Mart's tools/art.py, from Ribbit King's;
the shared helpers and palette are in tools/artlib.py, and the pictures live
in three modules:

    art_units.py    the twelve unit portraits (web/art/units/*.png)
    art_scenes.py   the title and the three campaign backdrops
                    (web/art/title.png, web/art/backdrops/*.png)
    art_icons.py    the three kind icons and the help triangle
                    (web/art/kinds/*.png, web/art/triangle.png)

    python3 tools/art.py            # every picture
    python3 tools/art.py title      # one picture
    python3 tools/art.py units      # one folder

Each module's docstring lists the facts its pictures must get right and
where each is enforced. Every module shares two rules, enforced here:

- no words, letters or digits in any picture: `check_no_text` refuses a .tex
  containing \\node, \\text or a label before it is compiled;
- repeatable: every random choice comes from a seeded random.Random.
"""
import shutil, subprocess, sys, tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import artlib  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "web" / "art"
SRC = ROOT / "tools" / "art"
DPI = {"": 150, "backdrops": 150, "units": 150, "kinds": 150}


def pictures():
    found = {}
    for mod in ("art_units", "art_scenes", "art_icons"):
        try:
            m = __import__(mod)
        except ModuleNotFoundError as e:
            if e.name != mod:
                raise
            continue
        found.update(getattr(m, "PICTURES", {}))
    return found


def main():
    only = set(sys.argv[1:])
    for name, make in pictures().items():
        if only and not only & {name, name.split("/")[0], name.split("/")[-1]}:
            continue
        folder = name.split("/")[0] if "/" in name else ""
        artlib.CUR_INK = artlib.INK.get(folder, 1.15)
        body, size = make()
        tex = artlib.document(body, size, artlib.CUR_INK)
        artlib.check_no_text(name, tex)
        with tempfile.TemporaryDirectory() as tmp:
            t = Path(tmp)
            (t / "art.tex").write_text(tex)
            r = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "art.tex"], cwd=t, capture_output=True, text=True)
            if r.returncode != 0:
                print(r.stdout[-3000:])
                raise SystemExit(f"{name}: pdflatex failed")
            clear = ["-transp"] if folder == "kinds" or name == "triangle" else []
            subprocess.run(["pdftocairo", "-png", *clear, "-r", str(DPI.get(folder, 150)), "-singlefile", str(t / "art.pdf"), str(t / "art")], check=True)
            out = OUT / f"{name}.png"
            out.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(t / "art.png", out)
            src = SRC / f"{name}.tex"
            src.parent.mkdir(parents=True, exist_ok=True)
            src.write_text(tex)
        print("wrote", out)


if __name__ == "__main__":
    main()
