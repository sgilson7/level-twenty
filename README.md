# Level Twenty

A three-against-three auto battler that runs in the browser. You field a crew of three from a city's worth of brawlers, witches, slingshot kids, a mech cat and a very good dog; the fight plays itself, and what it pays levels your crew up, to level 20. Your account levels up too, and your account level decides which units you can field.

**Play:** https://sgilson7.github.io/level-twenty/

## The game

- **Three kinds.** Every unit attacks with melee, magic or ranged. Melee is weak to magic, magic is weak to ranged, and ranged is weak to melee: a hit with the edge does half again as much, a hit against it a bit over two thirds.
- **Turns from speed.** Each unit's turn bar fills at its speed, as in Summoners War. When it fills, the unit goes for an enemy it has the edge over, or the one with least health; every few turns it uses its skill instead: a stun, a sweep, a heal, a mend, a drain, a snipe, a double hit, a haste or a slow.
- **Three skills each.** Every unit learns a skill at levels 1, 5 and 10, and you pick the one it fields.
- **A 3D stage.** Between your crew's tiles and theirs, the fight plays out in 3D: melee units run in and swing, ranged units shoot, magic users hurl what they conjure, and the camera leans in on each turn. With sound.
- **Levels.** Units earn XP from fights and level up to 20; each level adds HP, attack and speed. The account earns XP too, and unlocks a new unit at each level from 2 to 10.
- **Campaign.** Fifteen set fights across Market Street, the Rooftops and Chum's Junkyard. Stars count the units you had standing at the end.
- **Random.** Two crews dealt from the whole roster, locked units included, near your account level.
- **Online 1v1.** Your crew against a friend's, by room code or by pasted codes. Both browsers work out the same fight from a shared seed.

## How it is built

A [Builder](https://github.com/sgilson7/builder-setting) project: every rule is in a Rust core (`crates/core`) in integer maths, the 3D stage included (`stage.rs`, `figure.rs`), compiled to WebAssembly through a shim that decides nothing (`crates/shim`), and a static page (`web/`) draws what the core returns. The online transport is Ribbit King's (`web/net/`). Every picture is drawn in TikZ (`tools/art.py`), after vagrancy's `Sam_Comm_final.png`.

    ./scripts/setup.sh         # once: Playwright and three browser engines
    ./scripts/check.sh         # format, lint, tests, build, WebAssembly audit, browsers, online
    python3 scripts/serve.py   # then open http://127.0.0.1:8000
    python3 tools/art.py       # redraw web/art/ (needs pdflatex and pdftocairo)

`BRIEF.md` has the request as it was given, `PLAN.md` the decisions taken from it, `SECOND-ORDER.md` what came up while building, and `HANDOFF.md` what was checked.

MIT licence; see `LICENSES.md`.
