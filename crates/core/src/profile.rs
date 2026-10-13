//! A player's progress: the account's level, which decides which units may
//! be fielded, each unit's level and experience, the team, and the campaign's
//! stars. The page keeps the saved text in the browser and hands it back; the
//! core reads it, and a save it cannot read is a new player rather than an
//! error, because a broken save must never lock somebody out of the game.

use serde::{Deserialize, Serialize};

use crate::balance::*;
use crate::content::Content;

pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UnitProgress {
    pub id: String,
    pub level: u32,
    pub xp: u32,
    /// Which of the unit's three skills it fields.
    #[serde(default)]
    pub skill: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Profile {
    pub version: u32,
    pub level: u32,
    /// Experience into the current account level.
    pub xp: u32,
    /// One entry per roster unit, in roster order.
    pub units: Vec<UnitProgress>,
    /// Stars per campaign stage, 0 for not yet won.
    pub stars: Vec<u8>,
    pub team: Vec<String>,
    #[serde(default)]
    pub wins: u32,
    #[serde(default)]
    pub losses: u32,
}

/// What one fight did to the profile, for the results screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reward {
    pub won: bool,
    pub account_xp: u32,
    pub level_before: u32,
    pub level_after: u32,
    pub xp_pct: i32,
    /// Units the account's new level lets the player field.
    pub unlocked: Vec<String>,
    pub units: Vec<UnitReward>,
    /// Skills that a level-up in this fight opened: (unit name, skill name).
    pub skills_opened: Vec<(String, String)>,
    pub stars: u8,
    /// Three less the stars, so the page can draw the empty ones.
    pub stars_missing: u8,
    pub first_clear: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitReward {
    pub id: String,
    pub name: String,
    pub xp: u32,
    pub level_before: u32,
    pub level_after: u32,
    pub xp_pct: i32,
}

pub fn xp_pct(xp: u32, need: u32) -> i32 {
    if need == 0 {
        return 100;
    }
    (xp.min(need) * 100 / need) as i32
}

impl Profile {
    /// A new player: account level 1, every unit at level 1, the three
    /// starters on the team.
    pub fn new(content: &Content) -> Profile {
        Profile {
            version: SAVE_VERSION,
            level: 1,
            xp: 0,
            units: content
                .units
                .iter()
                .map(|u| UnitProgress {
                    id: u.id.clone(),
                    level: 1,
                    xp: 0,
                    skill: 0,
                })
                .collect(),
            stars: vec![0; content.stage_count()],
            team: content
                .units
                .iter()
                .filter(|u| u.unlock == 1)
                .take(TEAM_SIZE)
                .map(|u| u.id.clone())
                .collect(),
            wins: 0,
            losses: 0,
        }
    }

    /// Reads a save, keeping what still fits the roster and campaign: a unit
    /// added since gets level 1, one removed since is dropped, levels are
    /// clamped, and a team that no longer works is replaced by the starters.
    pub fn load(content: &Content, saved: &str) -> Profile {
        let fresh = Profile::new(content);
        let Ok(old) = serde_json::from_str::<Profile>(saved) else {
            return fresh;
        };
        if old.version != SAVE_VERSION {
            return fresh;
        }
        let mut p = fresh.clone();
        p.level = old.level.clamp(1, MAX_ACCOUNT_LEVEL);
        p.xp = old.xp.min(account_xp_to_next(p.level));
        for (k, u) in p.units.iter_mut().enumerate() {
            if let Some(o) = old.units.iter().find(|o| o.id == u.id) {
                u.level = clamp_unit_level(o.level);
                u.xp = o.xp.min(unit_xp_to_next(u.level));
                // a skill the unit's level does not open goes back to the first
                u.skill = if content.units[k].skill_open(o.skill, u.level) {
                    o.skill
                } else {
                    0
                };
            }
        }
        for (i, s) in old.stars.iter().take(p.stars.len()).enumerate() {
            p.stars[i] = (*s).min(3);
        }
        p.wins = old.wins;
        p.losses = old.losses;
        p.team = old.team;
        if p.check_team(content, &p.team.clone()).is_err() {
            p.team = fresh.team;
        }
        p
    }

    pub fn save(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn unlocked(&self, content: &Content, unit: usize) -> bool {
        content.units[unit].unlock <= self.level
    }

    pub fn unit_level(&self, unit: usize) -> u32 {
        self.units[unit].level
    }

    /// A team is three different units the account has unlocked. The error
    /// is a key into `data/copy.json`.
    pub fn check_team(&self, content: &Content, ids: &[String]) -> Result<Vec<usize>, &'static str> {
        if ids.len() != TEAM_SIZE {
            return Err("team_need_three");
        }
        let mut out = Vec::new();
        for id in ids {
            let Some(u) = content.unit_index(id) else {
                return Err("team_unknown");
            };
            if out.contains(&u) {
                return Err("team_twice");
            }
            if !self.unlocked(content, u) {
                return Err("team_locked");
            }
            out.push(u);
        }
        Ok(out)
    }

    pub fn team_entries(&self, content: &Content) -> Vec<crate::battle::Entry> {
        self.check_team(content, &self.team)
            .unwrap_or_default()
            .into_iter()
            .map(|u| (u, self.unit_level(u), self.units[u].skill))
            .collect()
    }

    /// Fields skill `k` on unit `u`. The error is a key into copy.json.
    pub fn set_skill(&mut self, content: &Content, u: usize, k: usize) -> Result<(), &'static str> {
        if !content.units[u].skill_open(k, self.units[u].level) {
            return Err("skill_locked");
        }
        self.units[u].skill = k;
        Ok(())
    }

    /// Whether a stage may be played: the first always, each later one once
    /// the one before it is won.
    pub fn stage_open(&self, stage: usize) -> bool {
        stage == 0 || self.stars.get(stage - 1).map(|s| *s > 0).unwrap_or(false)
    }

    /// Adds experience to the account and to the given units, levelling up as
    /// often as it reaches, and says what changed.
    pub fn grant(
        &mut self,
        content: &Content,
        won: bool,
        account_xp: u32,
        units: &[usize],
        unit_xp: u32,
    ) -> Reward {
        let level_before = self.level;
        self.xp += account_xp;
        while self.level < MAX_ACCOUNT_LEVEL && self.xp >= account_xp_to_next(self.level) {
            self.xp -= account_xp_to_next(self.level);
            self.level += 1;
        }
        if self.level == MAX_ACCOUNT_LEVEL {
            self.xp = 0;
        }
        let unlocked = content
            .units
            .iter()
            .filter(|u| u.unlock > level_before && u.unlock <= self.level)
            .map(|u| u.id.clone())
            .collect();
        let mut rewards = Vec::new();
        let mut opened = Vec::new();
        for &u in units {
            let p = &mut self.units[u];
            let before = p.level;
            let gained = if p.level < MAX_UNIT_LEVEL { unit_xp } else { 0 };
            p.xp += gained;
            while p.level < MAX_UNIT_LEVEL && p.xp >= unit_xp_to_next(p.level) {
                p.xp -= unit_xp_to_next(p.level);
                p.level += 1;
            }
            if p.level == MAX_UNIT_LEVEL {
                p.xp = 0;
            }
            for sk in content.units[u]
                .skills
                .iter()
                .filter(|s| s.unlock > before && s.unlock <= p.level)
            {
                opened.push((content.units[u].name.clone(), sk.name.clone()));
            }
            rewards.push(UnitReward {
                id: p.id.clone(),
                name: content.units[u].name.clone(),
                xp: gained,
                level_before: before,
                level_after: p.level,
                xp_pct: if p.level == MAX_UNIT_LEVEL {
                    100
                } else {
                    xp_pct(p.xp, unit_xp_to_next(p.level))
                },
            });
        }
        if won {
            self.wins += 1;
        } else {
            self.losses += 1;
        }
        Reward {
            won,
            account_xp,
            level_before,
            level_after: self.level,
            xp_pct: self.account_xp_pct(),
            unlocked,
            units: rewards,
            skills_opened: opened,
            stars: 0,
            stars_missing: 0,
            first_clear: false,
        }
    }

    pub fn account_xp_pct(&self) -> i32 {
        if self.level == MAX_ACCOUNT_LEVEL {
            100
        } else {
            xp_pct(self.xp, account_xp_to_next(self.level))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixtures::content;

    #[test]
    fn a_new_player_fields_the_three_starters_at_level_one() {
        let c = content();
        let p = Profile::new(&c);
        assert_eq!(p.team, vec!["rook", "vela", "pip"]);
        assert_eq!(p.level, 1);
        assert!(p.units.iter().all(|u| u.level == 1 && u.xp == 0));
        assert_eq!(p.stars.len(), 15);
    }

    #[test]
    fn a_team_must_be_three_different_unlocked_units() {
        let c = content();
        let p = Profile::new(&c);
        let t = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(p.check_team(&c, &t(&["rook", "vela", "pip"])).is_ok());
        assert_eq!(p.check_team(&c, &t(&["rook", "vela"])), Err("team_need_three"));
        assert_eq!(p.check_team(&c, &t(&["rook", "rook", "pip"])), Err("team_twice"));
        assert_eq!(p.check_team(&c, &t(&["rook", "chum", "pip"])), Err("team_locked"));
        assert_eq!(p.check_team(&c, &t(&["rook", "zed", "pip"])), Err("team_unknown"));
    }

    #[test]
    fn the_account_level_unlocks_units() {
        let c = content();
        let mut p = Profile::new(&c);
        let chum = c.unit_index("chum").unwrap();
        assert!(!p.unlocked(&c, chum));
        let r = p.grant(&c, true, 50 + 100 + 150 + 200, &[], 0);
        assert_eq!(r.level_after, 5);
        assert!(p.unlocked(&c, chum));
        assert_eq!(r.unlocked, vec!["nan", "ed", "static", "chum"]);
    }

    #[test]
    fn experience_levels_a_unit_up_and_stops_at_twenty() {
        let c = content();
        let mut p = Profile::new(&c);
        let r = p.grant(&c, true, 0, &[0], unit_xp_to_next(1) + 5);
        assert_eq!((r.units[0].level_before, r.units[0].level_after), (1, 2));
        assert_eq!(p.units[0].xp, 5);
        let r = p.grant(&c, true, 0, &[0], 1_000_000);
        assert_eq!(r.units[0].level_after, MAX_UNIT_LEVEL);
        assert_eq!(p.units[0].xp, 0);
        let r = p.grant(&c, true, 0, &[0], 100);
        assert_eq!(r.units[0].xp, 0, "a level 20 unit gains nothing");
    }

    #[test]
    fn a_save_round_trips_and_a_broken_one_is_a_new_player() {
        let c = content();
        let mut p = Profile::new(&c);
        p.grant(&c, true, 400, &[1, 2], 300);
        p.stars[0] = 3;
        assert_eq!(Profile::load(&c, &p.save()), p);
        assert_eq!(Profile::load(&c, "not json"), Profile::new(&c));
        assert_eq!(Profile::load(&c, ""), Profile::new(&c));
    }

    #[test]
    fn a_tampered_save_is_clamped_and_a_locked_team_is_replaced() {
        let c = content();
        let mut p = Profile::new(&c);
        p.units[0].level = 99;
        p.level = 1;
        p.team = vec!["chum".into(), "rook".into(), "pip".into()];
        let back = Profile::load(&c, &p.save());
        assert_eq!(back.units[0].level, MAX_UNIT_LEVEL);
        assert_eq!(back.team, vec!["rook", "vela", "pip"]);
    }

    #[test]
    fn a_skill_can_be_fielded_only_once_its_level_is_reached() {
        let c = content();
        let mut p = Profile::new(&c);
        assert_eq!(p.set_skill(&c, 0, 1), Err("skill_locked"));
        let r = p.grant(&c, true, 0, &[0], 1_000);
        assert!(p.units[0].level >= 5);
        assert!(r.skills_opened.iter().any(|(_, s)| s == "Two-Piece"));
        assert_eq!(p.set_skill(&c, 0, 1), Ok(()));
        assert_eq!(p.team_entries(&c)[0], (0, p.units[0].level, 1));
        assert_eq!(p.set_skill(&c, 0, 2), Err("skill_locked"));
        assert_eq!(p.set_skill(&c, 0, 3), Err("skill_locked"));
    }

    #[test]
    fn a_saved_skill_the_level_does_not_open_falls_back_to_the_first() {
        let c = content();
        let mut p = Profile::new(&c);
        p.units[0].skill = 2;
        assert_eq!(Profile::load(&c, &p.save()).units[0].skill, 0);
        p.units[0].level = 12;
        assert_eq!(Profile::load(&c, &p.save()).units[0].skill, 2);
    }

    #[test]
    fn a_stage_opens_once_the_one_before_is_won() {
        let c = content();
        let mut p = Profile::new(&c);
        assert!(p.stage_open(0) && !p.stage_open(1));
        p.stars[0] = 1;
        assert!(p.stage_open(1) && !p.stage_open(2));
    }
}
