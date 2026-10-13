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
    /// Mends every ally standing, the caster included.
    Mend,
    /// A hit, and the target's turn is pushed back.
    Slow,
    /// A hit, and the caster steps aside: the next blow aimed at it misses.
    Dodge,
    /// A hit from the target's flank, counted as with the edge whatever the
    /// kinds.
    Flank,
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
    /// The unit level at which the skill may be chosen.
    pub unlock: u32,
}

/// How a unit is built in the 3D fight (crates/core/src/figure.rs). Colours
/// are `#RRGGBB`; `size` is percent of a grown person's height.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    pub body: Body,
    pub skin: String,
    pub top: String,
    pub legs: String,
    pub hair: String,
    pub hat: Hat,
    pub prop: Prop,
    pub accent: String,
    pub size: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Body {
    Person,
    Cat,
    Dog,
    Raccoon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Hat {
    None,
    Flat,
    Witch,
    Cap,
    Bun,
    Visor,
    Antenna,
    Goggles,
    Chimney,
    Headset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Prop {
    None,
    Fists,
    Orb,
    Slingshot,
    Lantern,
    Pistons,
    Jet,
    Drone,
    Spray,
}

/// `#RRGGBB` to a colour with each channel in Q16; anything else is ink.
pub type Rgb = [i32; 3];

pub fn hex(s: &str) -> Rgb {
    let t = s.trim_start_matches('#');
    if t.len() != 6 || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return [0; 3];
    }
    let ch = |i: usize| i32::from_str_radix(&t[i..i + 2], 16).unwrap_or(0) * crate::fx::ONE / 255;
    [ch(0), ch(2), ch(4)]
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
    /// Three, unlocking as the unit levels; the player picks one to field.
    pub skills: Vec<Skill>,
    pub look: Look,
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

    /// Whether skill `k` may be fielded at `level`.
    pub fn skill_open(&self, k: usize, level: u32) -> bool {
        self.skills.get(k).map(|s| s.unlock <= level).unwrap_or(false)
    }

    /// The last skill a unit at `level` has unlocked: what a campaign enemy
    /// fields.
    pub fn best_skill(&self, level: u32) -> usize {
        (0..self.skills.len())
            .rev()
            .find(|&k| self.skill_open(k, level))
            .unwrap_or(0)
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
    /// The 3D stage's colours for this part of town.
    pub palette: Palette,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub sky: String,
    pub fog: String,
    pub ground: String,
    pub floor: String,
    pub walls: Vec<String>,
    pub trim: String,
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
            if u.skills.len() != balance::SKILLS {
                return Err(format!("units.json: {} needs three skills", u.id));
            }
            for (k, sk) in u.skills.iter().enumerate() {
                if sk.power <= 0 || sk.cooldown < 1 {
                    return Err(format!(
                        "units.json: {}'s {} needs power and a cooldown",
                        u.id, sk.name
                    ));
                }
                let first = k == 0 && sk.unlock != 1;
                let backwards = k > 0 && sk.unlock <= u.skills[k - 1].unlock;
                if first || backwards || sk.unlock > balance::MAX_UNIT_LEVEL {
                    return Err(format!(
                        "units.json: {}'s skills must unlock at 1, then later, by 20",
                        u.id
                    ));
                }
            }
            for c in [
                &u.look.skin,
                &u.look.top,
                &u.look.legs,
                &u.look.hair,
                &u.look.accent,
            ] {
                if c.len() != 7 || hex(c) == [0; 3] && c != "#000000" {
                    return Err(format!(
                        "units.json: {} has a colour {c} that is not #RRGGBB",
                        u.id
                    ));
                }
            }
            if !(40..=250).contains(&u.look.size) {
                return Err(format!("units.json: {}'s size is out of range", u.id));
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
    fn every_unit_has_three_skills_unlocking_at_one_five_and_ten() {
        let c = content();
        for u in &c.units {
            let at: Vec<u32> = u.skills.iter().map(|s| s.unlock).collect();
            assert_eq!(at, vec![1, 5, 10], "{}", u.id);
            assert_eq!(u.best_skill(1), 0);
            assert_eq!(u.best_skill(5), 1);
            assert_eq!(u.best_skill(20), 2);
            assert!(u.skill_open(0, 1) && !u.skill_open(1, 4) && u.skill_open(1, 5) && !u.skill_open(3, 20));
        }
    }

    #[test]
    fn skills_out_of_order_are_refused() {
        let bad = UNITS.replacen("\"unlock\": 5}", "\"unlock\": 1}", 1);
        assert!(Content::parse(&bad, CAMPAIGN)
            .unwrap_err()
            .contains("unlock at 1"));
    }

    #[test]
    fn a_colour_is_read_as_sixteen_bit_channels() {
        assert_eq!(hex("#FF0000"), [crate::fx::ONE, 0, 0]);
        assert_eq!(hex("#808080")[1], 128 * crate::fx::ONE / 255);
        assert_eq!(hex("nope"), [0; 3]);
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
        assert_eq!(c.stage(0).unwrap().1.name, "The Fruit Stall");
        assert_eq!(c.stage(5).unwrap().0, 1);
        assert_eq!(c.stage(14).unwrap().1.name, "Chum Wakes Up");
        assert!(c.stage(15).is_none());
    }
}
