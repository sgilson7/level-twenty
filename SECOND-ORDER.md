# Second-order notebook

One row per assumption (A), surprise or finding (F), divergence from the plan (D), or item for Sam (W), written as it came up.

| # | Kind | Row |
| --- | --- | --- |
| 1 | A | Raiders of the Lost Mart (11 October) was built from an almost identical request, so its layout is the starting point: Builder template, Raiders' `scripts/build.sh` (cache busting, vendored-bundle hash check), `tools/art.py` and `artlib.py`, `web/net/` and the boundary test that fences it. |
| 2 | A | The brief names three stats only (attack, HP, speed), so there is no defence. Type edge, crits and a ±8% spread make fights vary. |
| 3 | A | "A level out of combat that determines which units you can use" is read as an account level that unlocks units one per level, 2 to 10, three starters at 1. |
| 4 | D | The brief says "could have" online 1v1. Built, because the transport existed and a fight is a pure function of crews and seed (PLAN P9): no lockstep or rollback was needed, unlike Raiders. |
| 5 | F | Campaign pacing, measured by `crates/core/tests/campaign.rs` (a player who keeps the starters and replays until a win): about 50 fights for all fifteen stages, most stages won in 1 to 3 tries, with a wall at The Kennel (12 to 19 tries) where the starters are melee/magic-heavy against Biscuit and Rook at 14. Enemy levels were tuned twice from the first numbers (Scrap Gate 11 → 10, the Kennel 16 → 14, the final fight 18 → 19 for its support). |
| 6 | F | The page may hold no numeric literal but 0 and 1 (Builder D3). The star display needed "3 − stars", so the core sends `stars_missing`. |
| 7 | F | In the first full `check.sh` the online test hung on the second fight. Cause: after fight 1 the joiner's units levelled up, the host's Start named the old levels, and the joiner refused a crew that "no longer matched", leaving the host in a fight the joiner never saw. Fixed: the joiner plays a Start whose units match its crew whatever the levels, and both browsers retell their crew after each fight. Test `a_fight_set_with_last_fights_levels_is_still_played`, broken once by restoring the strict comparison: it failed with `assertion failed: join.take_ready().is_some()`. |
| 8 | F | The first portrait pass hid Nan's lantern behind her arm and drew flat-top hair as a box; the title's mech cat was placed by its corner, not its centre. Each was seen in a rendered contact sheet and fixed. The help triangle's arrows first bowed inward. |
| 9 | F | On a phone the battle arena clipped the last fighter: the 16:9 ratio survived the media query. Fixed with `aspect-ratio: unset` and auto height. |
| 10 | D | Online fights do not level-match crews: each player brings what they have. A player with a level-20 crew will beat a new one. |
| 11 | W | The name ("Level Twenty"), the roster's names and blurbs, the campaign's fights and every balance number are first-pass and Sam's to change (`data/*.json`, `balance.rs`). |
| 12 | W | The room-code path (public relays and STUN) is not exercised by the automated test, which uses pasted codes with `?ice=none`; it is the transport Ribbit King and Raiders use unchanged. |
| 13 | F | A joiner who changes crew between the host's Start and its arrival sees "online_crew_changed" and the host plays that one fight alone (it is still paid). Rare, and recorded rather than fixed. |
| 14 | F | `cargo mutants -p lt_core`: 540 mutants, 351 caught, 156 missed, 18 unviable, 15 timeouts. The misses cluster in `battle.rs` (46: the damage formula's exact numbers), `game.rs` (35: view fields such as `room`, `net_left`, `xp_pct`), `content.rs` (23: validation messages) and `balance.rs` (15: reward amounts). The tests check direction and bounds, not values; pinning values is the next test pass. |
| 15 | F | Against the live site the browser check failed "every portrait loaded" in all three engines with no 404 and no console error: pictures still in flight when the check looked. The page now preloads every picture after the menu draws, and the check waits up to ten seconds. Passed live in all three after the change. |
