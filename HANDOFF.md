# Handoff

12 October 2026. Level Twenty, the MVP.

## What exists

| What | Where |
| --- | --- |
| The game, live | https://sgilson7.github.io/level-twenty/ |
| Repository (public, MIT) | https://github.com/sgilson7/level-twenty |
| Website listing, live | https://sgilson7.github.io/projects/level-twenty/ and a tile with Play game on /projects/ (website commit `b659733`, build `92e61b68`) |
| Deploys | `.github/workflows/deploy.yml`: a push to `main` runs `./scripts/check.sh` (Chromium) and deploys `dist/` to Pages only if it passes |

Everything the request asked for is in: 3v3 fights that play themselves; a campaign (fifteen fights in three parts of town) and a completely random mode; an account level, earned from fights, that decides which units you can field (three starters, then one unlock per level from 2 to 10); twelve units, each with XP and a level to 20, every level adding attack, HP and speed; melee, magic and ranged, each weak to one; turns from speed on a Summoners War attack bar; online 1v1; TikZ art for every picture, after `Sam_Comm_final.png`.

## Second round (same day): sound, a 3D stage, three skills per unit

- **Three skills each** (PLAN P12): learned at levels 1, 5 and 10; picked per unit on the Crew screen; the results screen says when a unit learns one. Two new effects, Mend and Slow.
- **The 3D stage** (P13, P14): between the tiles, WebGL 2. Units run in and swing, shoot, or raise and hurl an orb; hits knock back and flash; the fallen fall; stunned units have stars; a ready skill glows at the feet; skill callouts over the stage; floating numbers over the units' heads as well as on the tiles. Each part of town has its own 3D scenery (market stalls and string lights; a roof with a water tower, a mast and laundry; scrap heaps, tyres, a crane and the mech cat asleep).
- **Sound** (P15): synthesized, with a mute button.
- Online protocol is now 2 (a unit carries its skill).

Checks for this round are in the table below, marked "round 2".

## Third round (13 October): moving fighters, Dodge and Flank, the crew from the campaign

- **Tiles no longer run under the stage** (PLAN P16); the browser check now measures it.
- **Fighters keep moving**, after Final Fantasy XIII (P17): between turns everyone not being hit jogs to a new spot lined up on the foe it will hit next, ranged and magic strafe as they shoot, and the camera fits the whole floor.
- **Dodge and Flank** (P18): new skills for nine units. Dodge: a hit, then the next blow on the caster misses. Flank: a run to the target's side, and the hit counts as strong. Online protocol is now 3.
- **Crew from the campaign** (P19).

Checks for this round are marked "round 3".

## Checks run, and results

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 48 unit tests, 12 boundary tests, 1 campaign probe: all pass |
| `./scripts/check.sh` (local, three engines) | format, clippy, tests, build, WebAssembly audit, browser check in Chromium, Firefox and WebKit: pass. The online step hung on a second fight in its first run (SECOND-ORDER row 7); after the fix, `tests/browser/online.py chromium firefox webkit` passed twice |
| CI on `main` (Chromium) | check: success, deploy: success, for both pushes |
| `BUILDER_ORIGIN=https://sgilson7.github.io/level-twenty tests/browser/check.py` | first run FAILED "every portrait loaded" in all three engines (pictures still loading, no 404); after preloading and a waiting check, PASSED in Chromium, Firefox and WebKit |
| Room code against the live site, two Chromium browsers through the public relays and STUN | met in 1.3 s; one fight, one Victory and one Defeat; no console error |
| Website `make check`, `make packet`, `make gate`, then `make deploy` and `make gate-live` | all passed |
| Website `testing/engines.py`, Level Twenty alone, live | drew in Chromium, Firefox and WebKit (both rounds) |
| round 2: live browser check, and the website listing (`make check`, `packet`, `gate`, `deploy`, `gate-live`) | passed; the listing describes the stage and skills, with a new tile and clip of the 3D stage |
| round 2: `cargo test --workspace` | 86 unit tests (skills, Mend, Slow, figures, stage timeline, choreography, camera marks, scenery), 12 boundary tests, 1 campaign probe: all pass. Campaign probe with enemies on their best skills: 41 to 54 fights |
| round 2: `./scripts/check.sh` (local, three engines) | BUILDER CHECK PASSED: browser check (now also "the 3D stage drew the fight" and the skill picker) and the two-browser online check in Chromium, Firefox and WebKit |
| round 3: `cargo test --workspace` | 93 unit tests (new: Dodge, Flank, nobody jumps or leaves the floor, units keep moving and never stand in each other, a flank strikes from the side, a dodged blow finds its target stepped aside, a Dodge rolls clear), 12 boundary tests, 1 campaign probe (41 to 54 fights): all pass |
| round 3: `./scripts/check.sh` (local, three engines) | BUILDER CHECK PASSED, with the new browser checks (no tile under the stage; the crew screen opens from the campaign and Back returns there) |
| `cargo mutants -p lt_core` (on the first commit, before the online fix) | 540 mutants in 25 min: 351 caught, 156 missed, 18 unviable, 15 timeouts. Not part of `check.sh` |

## Waiting on Sam

1. **The name.** "Level Twenty" is a working title.
2. **The roster, the campaign and the balance.** Twelve units, fifteen fights and every number are first-pass (`data/units.json`, `data/campaign.json`, `crates/core/src/balance.rs`). The pacing probe says about 50 fights clear the campaign, with a wall at stage 14, the Kennel (SECOND-ORDER row 5).
3. **Online fairness.** Each player brings the crew they have, at their levels; a level-matched option is a small change in the core if wanted.
4. **Phones.** The page lays out at phone width and takes taps, but nobody has played it on a phone, so the website lists Windows, Mac and Linux only.

## Known gaps

- Mutation testing leaves survivors (row above): most are exact-number mutants in the damage formula, the percentage helpers and the view JSON, which the tests check by direction (more, less, capped) rather than by value.
- A joiner who changes crew while the host's Start is in flight misses that one fight (SECOND-ORDER row 13).
- Random mode pays account XP only, by design (PLAN P6).

## Where things are

`BRIEF.md` (the request, verbatim), `PLAN.md` (each decision and the words it answers), `SECOND-ORDER.md` (what came up), `crates/core/src/` (`battle.rs` the fight, `profile.rs` levels and the save, `game.rs` the door the page calls, `online.rs` the 1v1 protocol, `content.rs` the data), `tools/art*.py` (the TikZ).
