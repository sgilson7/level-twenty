# Instructions for a coding agent in a Builder project

`CLAUDE.md` holds the same rules for agents that read that file instead.

1. Read `BRIEF.md`, `PLAN.md` and `BUILDER-SPEC.md` before changing code.
2. The core (`crates/core`) owns every application decision and performs no input or output.
3. The shim (`crates/shim`) translates values and decides no application behaviour.
4. The page (`web/`) draws what the core returns and does not reproduce a core rule. No numeric literal but 0 and 1 in page JavaScript: timings live in `data/display.json`, and every number drawn comes from the core.
5. Run `cargo test` after each meaningful core change.
6. For each new behaviour test, break the behaviour it covers once, watch the test fail, then restore it. Note the failure message in `SECOND-ORDER.md`.
7. Keep `SECOND-ORDER.md` while you work: one row per assumption, surprise, divergence from the plan, or item the person must do.
8. Deploying is the person's decision. Sam approved a public repository, Pages deploys and the website listing for this project in advance (BRIEF.md, Autonomy): a push to `main` deploys after the check passes.
9. Before handoff, run `./scripts/check.sh` and record the result in `HANDOFF.md`.

Feel numbers live in `crates/core/src/balance.rs`. Units are in `data/units.json`, the campaign in `data/campaign.json`, strings in `data/copy.json`, playback timings in `data/display.json`. `cargo test --release --test campaign -- --nocapture` prints how many fights a simulated player needs per campaign stage. The 3D stage is `crates/core/src/stage.rs` (timeline, choreography, camera, scenery) and `figure.rs` (each unit's model from its `look` in `data/units.json`); `web/gl.js` only draws what they return. Sounds are recipes in `data/sounds.json`. Art is TikZ: `python3 tools/art.py` writes `web/art/` from `tools/art_units.py`, `tools/art_scenes.py` and `tools/art_icons.py`.
