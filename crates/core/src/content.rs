//! The roster and the campaign, read from `data/units.json` and
//! `data/campaign.json` and checked before anything uses them. A unit's
//! numbers live in the data; what they mean lives in `balance.rs` and
//! `battle.rs`.

use serde::{Deserialize, Serialize};

use crate::balance;

/// How a unit attacks. Melee is weak to magic, magic is weak to ranged, and
/// ranged is weak to melee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Melee,
    Magic,
    Ranged,
}

impl Kind {
    /// The kind this one hits hard.
    pub fn beats(self) -> Kind {
        match self {
            Kind::Melee => Kind::Ranged,
            Kind::Ranged => Kind::Magic,
            Kind::Magic => Kind::Melee,
        }
    }

    /// +1 when `self` has the edge over `other`, -1 when `other` has it, 0
    /// for the same kind.
    pub fn edge(self, other: Kind) -> i32 {
        if self.beats() == other {
            1
        } else if other.beats() == self {
            -1
        } else {
            0
        }
    }

    pub const ALL: [Kind; 3] = [Kind::Melee, Kind::Magic, Kind::Ranged];
}

/// What a unit's skill does when its cooldown is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    /// One heavy hit on the usual target.
    Strike,
    /// A hit, and the target loses its next turn.
    Stun,
    /// A lighter hit on every enemy standing.
    Sweep,
    /// Mends the ally with the smallest share of health left.
    Heal,
    /// A hit, and the attacker mends by half the damage done.
    Drain,
    /// A hit on whichever enemy has the least health, whatever its kind.
    Snipe,
    /// Two hits.
    Double,
    /// A hit, and every other ally's turn comes sooner.
    Haste,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub name: String,
    pub effect: Effect,
    /// Percent of attack.
    pub power: i32,
    /// Own turns between uses.
    pub cooldown: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnitDef {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// The account level at which a player may field this unit.
    pub unlock: u32,
    pub title: String,
    pub blurb: String,
    /// [at level 1, added per level]
    pub hp: [i32; 2],
    pub atk: [i32; 2],
    pub spd: [i32; 2],
    pub skill: Skill,
}

/// A unit's three numbers at one level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Stats {
    pub hp: i32,
    pub atk: i32,
    pub spd: i32,
}

impl UnitDef {
    pub fn stats(&self, level: u32) -> Stats {
        let l = balance::clamp_unit_level(level) as i32 - 1;
        Stats {
            hp: self.hp[0] + self.hp[1] * l,
            atk: self.atk[0] + self.atk[1] * l,
            spd: self.spd[0] + self.spd[1] * l,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitsFile {
    units: Vec<UnitDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StageDef {
    pub name: String,
    /// Three (unit id, level) pairs.
    pub enemies: Vec<(String, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterDef {
    pub name: String,
    pub backdrop: String,
    pub blurb: String,
    pub stages: Vec<StageDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignFile {
    chapters: Vec<ChapterDef>,
}

/// Everything read from data, checked.
#[derive(Debug, Clone)]
pub struct Content {
    pub units: Vec<UnitDef>,
    pub chapters: Vec<ChapterDef>,
}

impl Content {
    pub fn parse(units_json: &str, campaign_json: &str) -> Result<Content, String> {
        let units: UnitsFile = serde_json::from_str(units_json).map_err(|e| format!("units.json: {e}"))?;
        let campaign: CampaignFile =
            serde_json::from_str(campaign_json).map_err(|e| format!("campaign.json: {e}"))?;
        let c = Content {
            units: units.units,
            chapters: campaign.chapters,
        };
        c.check()?;
        Ok(c)
    }

    fn check(&self) -> Result<(), String> {
        if self.units.len() < balance::TEAM_SIZE {
            return Err("units.json: fewer units than a team needs".into());
        }
        for (i, u) in self.units.iter().enumerate() {
            if self.units[..i].iter().any(|v| v.id == u.id) {
                return Err(format!("units.json: {} appears twice", u.id));
            }
            if u.hp[0] <= 0 || u.atk[0] <= 0 || u.spd[0] <= 0 {
                return Err(format!("units.json: {} has a stat that is not positive", u.id));
            }
            if u.hp[1] < 0 || u.atk[1] < 0 || u.spd[1] < 0 {
                return Err(format!("units.json: {} shrinks as it levels", u.id));
            }
            if u.skill.power <= 0 || u.skill.cooldown < 1 {
                return Err(format!("units.json: {}'s skill needs power and a cooldown", u.id));
            }
            if u.unlock < 1 || u.unlock > balance::MAX_ACCOUNT_LEVEL {
                return Err(format!("units.json: {} unlocks at an unreachable level", u.id));
            }
        }
        let starters = self.units.iter().filter(|u| u.unlock == 1).count();
        if starters < balance::TEAM_SIZE {
            return Err("units.json: a new player needs three units at level 1".into());
        }
        if self.stage_count() == 0 {
            return Err("campaign.json: no stages".into());
        }
        for ch in &self.chapters {
            for st in &ch.stages {
                if st.enemies.len() != balance::TEAM_SIZE {
                    return Err(format!("campaign.json: {} does not field three", st.name));
                }
                for (id, level) in &st.enemies {
                    if self.unit_index(id).is_none() {
                        return Err(format!("campaign.json: {} names unknown unit {id}", st.name));
                    }
                    if *level < 1 || *level > balance::MAX_UNIT_LEVEL {
                        return Err(format!("campaign.json: {} has {id} at level {level}", st.name));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn unit_index(&self, id: &str) -> Option<usize> {
        self.units.iter().position(|u| u.id == id)
    }

    pub fn stage_count(&self) -> usize {
        self.chapters.iter().map(|c| c.stages.len()).sum()
    }

    /// The stage at a flat index, with its chapter's index.
    pub fn stage(&self, index: usize) -> Option<(usize, &StageDef)> {
        let mut k = index;
        for (ci, ch) in self.chapters.iter().enumerate() {
            if k < ch.stages.len() {
                return Some((ci, &ch.stages[k]));
            }
            k -= ch.stages.len();
        }
        None
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::Content;

    pub const UNITS: &str = include_str!("../../../data/units.json");
    pub const CAMPAIGN: &str = include_str!("../../../data/campaign.json");

    pub fn content() -> Content {
        Content::parse(UNITS, CAMPAIGN).expect("the shipped data parses")
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn melee_is_weak_to_magic_magic_to_ranged_and_ranged_to_melee() {
        assert_eq!(Kind::Magic.edge(Kind::Melee), 1);
        assert_eq!(Kind::Melee.edge(Kind::Magic), -1);
        assert_eq!(Kind::Ranged.edge(Kind::Magic), 1);
        assert_eq!(Kind::Magic.edge(Kind::Ranged), -1);
        assert_eq!(Kind::Melee.edge(Kind::Ranged), 1);
        assert_eq!(Kind::Ranged.edge(Kind::Melee), -1);
        for k in Kind::ALL {
            assert_eq!(k.edge(k), 0);
        }
    }

    #[test]
    fn the_shipped_roster_and_campaign_parse_and_check() {
        let c = content();
        assert_eq!(c.units.len(), 12);
        for k in Kind::ALL {
            assert_eq!(c.units.iter().filter(|u| u.kind == k).count(), 4, "{k:?}");
        }
        assert_eq!(c.stage_count(), 15);
    }

    #[test]
    fn each_level_adds_to_every_stat_and_level_twenty_is_the_ceiling() {
        let c = content();
        let rook = &c.units[c.unit_index("rook").unwrap()];
        let (a, b) = (rook.stats(1), rook.stats(2));
        assert!(b.hp > a.hp && b.atk > a.atk && b.spd > a.spd);
        assert_eq!(rook.stats(20), rook.stats(99));
        assert_eq!(rook.stats(1), rook.stats(0));
    }

    #[test]
    fn a_stage_naming_an_unknown_unit_is_refused() {
        let bad = CAMPAIGN.replace("\"rook\", 1]", "\"nobody\", 1]");
        let e = Content::parse(UNITS, &bad).unwrap_err();
        assert!(e.contains("nobody"), "{e}");
    }

    #[test]
    fn a_roster_without_three_starters_is_refused() {
        let bad = UNITS.replace(
            "\"kind\": \"magic\", \"unlock\": 1",
            "\"kind\": \"magic\", \"unlock\": 4",
        );
        assert!(Content::parse(&bad, CAMPAIGN)
            .unwrap_err()
            .contains("three units"));
    }

    #[test]
    fn flat_stage_indexes_walk_through_the_chapters_in_order() {
        let c = content();
        assert_eq!(c.stage(0).unwrap().1.name, "The Corner Store");
        assert_eq!(c.stage(5).unwrap().0, 1);
        assert_eq!(c.stage(14).unwrap().1.name, "Chum Wakes Up");
        assert!(c.stage(15).is_none());
    }
}
