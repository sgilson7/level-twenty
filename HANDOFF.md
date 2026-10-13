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

## Checks run, and results

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 48 unit tests, 12 boundary tests, 1 campaign probe: all pass |
| `./scripts/check.sh` (local, three engines) | format, clippy, tests, build, WebAssembly audit, browser check in Chromium, Firefox and WebKit: pass. The online step hung on a second fight in its first run (SECOND-ORDER row 7); after the fix, `tests/browser/online.py chromium firefox webkit` passed twice |
| CI on `main` (Chromium) | check: success, deploy: success, for both pushes |
| `BUILDER_ORIGIN=https://sgilson7.github.io/level-twenty tests/browser/check.py` | first run FAILED "every portrait loaded" in all three engines (pictures still loading, no 404); after preloading and a waiting check, PASSED in Chromium, Firefox and WebKit |
| Room code against the live site, two Chromium browsers through the public relays and STUN | met in 1.3 s; one fight, one Victory and one Defeat; no console error |
| Website `make check`, `make packet`, `make gate`, then `make deploy` and `make gate-live` | all passed |
| Website `testing/engines.py`, Level Twenty alone, live | drew in Chromium, Firefox and WebKit |
| `cargo mutants -p lt_core` (on the first commit, before the online fix) | 540 mutants in 25 min: 351 caught, 156 missed, 18 unviable, 15 timeouts. Not part of `check.sh` |

## Waiting on Sam

1. **The name.** "Level Twenty" is a working title.
2. **The roster, the campaign and the balance.** Twelve units, fifteen fights and every number are first-pass (`data/units.json`, `data/campaign.json`, `crates/core/src/balance.rs`). The pacing probe says about 50 fights clear the campaign, with a wall at stage 14, the Kennel (SECOND-ORDER row 5).
3. **Online fairness.** Each player brings the crew they have, at their levels; a level-matched option is a small change in the core if wanted.
4. **Phones.** The page lays out at phone width and takes taps, but nobody has played it on a phone, so the website lists Windows, Mac and Linux only.

## Known gaps

- Mutation testing leaves survivors (row above): most are exact-number mutants in the damage formula, the percentage helpers and the view JSON, which the tests check by direction (more, less, capped) rather than by value.
- No sound.
- A joiner who changes crew while the host's Start is in flight misses that one fight (SECOND-ORDER row 13).
- Random mode pays account XP only, by design (PLAN P6).

## Where things are

`BRIEF.md` (the request, verbatim), `PLAN.md` (each decision and the words it answers), `SECOND-ORDER.md` (what came up), `crates/core/src/` (`battle.rs` the fight, `profile.rs` levels and the save, `game.rs` the door the page calls, `online.rs` the 1v1 protocol, `content.rs` the data), `tools/art*.py` (the TikZ).
