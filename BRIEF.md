# Brief

## The request (Sam, 12 October 2026)

> Please use the Builder repo as a base in the directory above, and use the work and repos of Sam gilson at sgilson7.github.io, which can all be found one directory up. please build out the MVP and deploy it to my website. you have approval for creating a public github repo, github pages, and approval to deploy to my website when its ready. any time you need to make 2D art and any kind of asset for the game, use the tikz latex prompt you can find in other repos, and use sam comm final as an inspiration image, alongside the menu screens of all of the sgilson7 games which use a similar build pipeline for images. now build the mvp for the game and deploy it, dont stop till its deployed, good luck. auto battler with a campaign style battle mode, or a completely random game mode. 3v3 fights. you have a level out of combat for that determines which units you can use. all units have xp and levels. levels give more stats (attack, hp, speed). units attacks are determined by Magic, Range, or Melee typing. Melee weak to magic, magic weak to ranged, ranged weak to melee. Kind of like Summoners wars except for levels on your units that could stack to lvl 20. Could have online 1v1 mode as well

## The game

A browser auto battler. A player fields a crew of three; fights play themselves, three against three, and the player watches, then spends what the fight paid. Played with a mouse or a finger, on a laptop or a phone.

## Done means

A Builder project whose `./scripts/check.sh` passes, deployed on GitHub Pages and listed on sgilson7.github.io, in which:

- fights are three against three and play themselves;
- there is a campaign mode (set fights in order) and a completely random mode;
- the player has an account level, earned out of combat, that decides which units they may field;
- every unit has XP and a level up to 20, and each level adds attack, HP and speed;
- every unit attacks by melee, magic or ranged; melee is weak to magic, magic to ranged, ranged to melee;
- turns come from speed, as in Summoners War's attack bar;
- an online 1v1 mode ("could have": built, because Ribbit King's transport already exists);
- every 2D picture is TikZ, in the look of vagrancy/Sam_Comm_final.png and the other sgilson7 games' menus.

## Decisions left with Sam

- The name. "Level Twenty" is a working title, taken from the level cap.
- The roster, its names and blurbs (twelve units, four of each kind, written for this build), and the campaign's fifteen fights. All are in `data/` and change without touching a rule.
- Balance. The numbers are first-pass and measured by one probe (`crates/core/tests/campaign.rs`); a playtest decides them.
- Whether online fights should level-match the two crews. They do not: each player brings the crew they have.

## Autonomy

Sam approved in advance: a public GitHub repository, GitHub Pages deploys from it, and deploying the website listing when the game is ready. The plan is approved in advance ("dont stop till its deployed").

## Allowed core dependencies

`serde`, `serde_json` (reference default).
