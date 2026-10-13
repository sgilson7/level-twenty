//! A 3v3 fight, decided in full the moment it starts. Every unit fills a
//! gauge at its speed; whoever fills it first takes a turn, uses its skill if
//! the cooldown is up and attacks otherwise. The page is handed the whole
//! fight as a list of turns, each with the state after it, and plays it back.
//!
//! The same seed and teams give the same fight on every machine: that is what
//! lets two browsers play a 1v1 by swapping teams and a seed.

use serde::Serialize;

use crate::balance::*;
use crate::content::{Content, Effect, Kind};
use crate::rng::Rng;

/// One side's entry: a unit (index into the roster) at a level.
pub type Entry = (usize, u32);

#[derive(Debug, Clone)]
struct Fighter {
    unit: usize,
    kind: Kind,
    level: u32,
    max_hp: i32,
    hp: i32,
    atk: i32,
    spd: i32,
    gauge: i32,
    cooldown: i32,
    stunned: bool,
}

impl Fighter {
    fn alive(&self) -> bool {
        self.hp > 0
    }
    /// Health left in thousandths, for comparing units of different size.
    fn share(&self) -> i32 {
        self.hp * 1000 / self.max_hp
    }
}

/// A unit as the page first draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub level: u32,
    pub hp: i32,
    pub atk: i32,
    pub spd: i32,
    pub skill: String,
}

/// A unit's state after a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct State {
    pub hp: i32,
    pub hp_pct: i32,
    pub gauge_pct: i32,
    pub alive: bool,
    pub stunned: bool,
    /// The skill fires on this unit's next turn.
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hit {
    pub side: usize,
    pub slot: usize,
    pub amount: i32,
    pub heal: bool,
    pub crit: bool,
    /// +1 with the edge, -1 against it.
    pub edge: i32,
    pub ko: bool,
    pub stun: bool,
    /// The target's turn was moved closer (Haste).
    pub hasted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Act {
    Attack,
    Skill,
    Stunned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Turn {
    pub side: usize,
    pub slot: usize,
    pub act: Act,
    /// The skill's name when `act` is skill.
    pub skill: Option<String>,
    pub effect: Option<Effect>,
    pub hits: Vec<Hit>,
    pub after: [Vec<State>; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Record {
    pub seed: u64,
    pub teams: [Vec<Card>; 2],
    pub start: [Vec<State>; 2],
    pub turns: Vec<Turn>,
    /// `None` when a fight that ran to the turn limit was dead even.
    pub winner: Option<usize>,
    pub survivors: [u32; 2],
}

impl Record {
    /// The same fight seen from the other side: side 1 becomes side 0. A
    /// joiner online plays the host's fight from its own seat.
    pub fn mirrored(&self) -> Record {
        let flip = |s: usize| 1 - s;
        let swap = |a: &[Vec<State>; 2]| [a[1].clone(), a[0].clone()];
        Record {
            seed: self.seed,
            teams: [self.teams[1].clone(), self.teams[0].clone()],
            start: swap(&self.start),
            turns: self
                .turns
                .iter()
                .map(|t| Turn {
                    side: flip(t.side),
                    hits: t
                        .hits
                        .iter()
                        .map(|h| Hit {
                            side: flip(h.side),
                            ..h.clone()
                        })
                        .collect(),
                    after: swap(&t.after),
                    ..t.clone()
                })
                .collect(),
            winner: self.winner.map(flip),
            survivors: [self.survivors[1], self.survivors[0]],
        }
    }

    /// A short fingerprint of the whole fight, so two browsers can tell
    /// whether they saw the same one (FNV-1a over its JSON).
    pub fn sum(&self) -> u32 {
        let text = serde_json::to_string(self).unwrap_or_default();
        let mut h: u32 = 0x811C_9DC5;
        for b in text.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
        h
    }
}

fn pct(part: i32, whole: i32) -> i32 {
    if whole <= 0 {
        return 0;
    }
    (part.max(0) * 100 / whole).min(100)
}

struct Fight<'a> {
    content: &'a Content,
    sides: [Vec<Fighter>; 2],
    rng: Rng,
}

impl<'a> Fight<'a> {
    fn states(&self) -> [Vec<State>; 2] {
        let one = |f: &Fighter| State {
            hp: f.hp.max(0),
            hp_pct: pct(f.hp, f.max_hp),
            gauge_pct: if f.alive() { pct(f.gauge, GAUGE_FULL) } else { 0 },
            alive: f.alive(),
            stunned: f.stunned,
            ready: f.alive() && f.cooldown == 0,
        };
        [
            self.sides[0].iter().map(one).collect(),
            self.sides[1].iter().map(one).collect(),
        ]
    }

    fn standing(&self, side: usize) -> u32 {
        self.sides[side].iter().filter(|f| f.alive()).count() as u32
    }

    /// Moves time on until somebody's gauge is full, and returns who acts:
    /// the fullest gauge, then side 0, then the lower slot.
    fn next_actor(&mut self) -> (usize, usize) {
        let mut wait = i32::MAX;
        for side in &self.sides {
            for f in side.iter().filter(|f| f.alive()) {
                let need = (GAUGE_FULL - f.gauge).max(0);
                wait = wait.min((need + f.spd - 1) / f.spd);
            }
        }
        for side in self.sides.iter_mut() {
            for f in side.iter_mut().filter(|f| f.alive()) {
                f.gauge += f.spd * wait;
            }
        }
        let mut best: Option<(usize, usize, i32)> = None;
        for (s, side) in self.sides.iter().enumerate() {
            for (k, f) in side.iter().enumerate() {
                if f.alive() && f.gauge >= GAUGE_FULL && best.map(|b| f.gauge > b.2).unwrap_or(true) {
                    best = Some((s, k, f.gauge));
                }
            }
        }
        let (s, k, _) = best.expect("a living unit always reaches a full gauge");
        (s, k)
    }

    /// The usual target: an enemy this attacker has the edge over, then the
    /// one with the least health, then the lower slot.
    fn usual_target(&self, side: usize, kind: Kind) -> Option<usize> {
        let foes = &self.sides[1 - side];
        (0..foes.len())
            .filter(|&k| foes[k].alive())
            .min_by_key(|&k| (-kind.edge(foes[k].kind), foes[k].hp, k))
    }

    fn weakest_target(&self, side: usize) -> Option<usize> {
        let foes = &self.sides[1 - side];
        (0..foes.len())
            .filter(|&k| foes[k].alive())
            .min_by_key(|&k| (foes[k].hp, k))
    }

    /// One blow from (side, slot) on the enemy in `target`.
    fn strike(&mut self, side: usize, slot: usize, target: usize, power: i32) -> Hit {
        let a = &self.sides[side][slot];
        let (atk, kind) = (a.atk, a.kind);
        let foe_kind = self.sides[1 - side][target].kind;
        let edge = kind.edge(foe_kind);
        let edge_pct = match edge {
            1 => EDGE_GOOD,
            -1 => EDGE_BAD,
            _ => EDGE_EVEN,
        };
        let crit = self.rng.chance(CRIT_CHANCE);
        let spread = self.rng.range(SPREAD_LOW, SPREAD_HIGH);
        let mut amount = atk * power / 100 * edge_pct / 100 * spread / 100;
        if crit {
            amount = amount * CRIT_POWER / 100;
        }
        let amount = amount.max(1);
        let foe = &mut self.sides[1 - side][target];
        foe.hp -= amount;
        Hit {
            side: 1 - side,
            slot: target,
            amount,
            heal: false,
            crit,
            edge,
            ko: !foe.alive(),
            stun: false,
            hasted: false,
        }
    }

    fn mend(&mut self, side: usize, slot: usize, amount: i32) -> Hit {
        let f = &mut self.sides[side][slot];
        let room = (f.max_hp - f.hp).max(0);
        let amount = amount.min(room);
        f.hp += amount;
        Hit {
            side,
            slot,
            amount,
            heal: true,
            crit: false,
            edge: 0,
            ko: false,
            stun: false,
            hasted: false,
        }
    }

    fn take_turn(&mut self, side: usize, slot: usize) -> Turn {
        self.sides[side][slot].gauge -= GAUGE_FULL;
        let unit = self.sides[side][slot].unit;
        let skill = &self.content.units[unit].skill;
        let (effect, power, cooldown) = (skill.effect, skill.power, skill.cooldown);
        let kind = self.sides[side][slot].kind;
        let mut hits = Vec::new();

        let act = if self.sides[side][slot].stunned {
            self.sides[side][slot].stunned = false;
            Act::Stunned
        } else if self.sides[side][slot].cooldown > 0 {
            self.sides[side][slot].cooldown -= 1;
            if let Some(t) = self.usual_target(side, kind) {
                hits.push(self.strike(side, slot, t, BASIC_POWER));
            }
            Act::Attack
        } else {
            self.sides[side][slot].cooldown = cooldown;
            match effect {
                Effect::Strike | Effect::Stun | Effect::Drain | Effect::Haste => {
                    if let Some(t) = self.usual_target(side, kind) {
                        let mut hit = self.strike(side, slot, t, power);
                        if effect == Effect::Stun && !hit.ko {
                            self.sides[1 - side][t].stunned = true;
                            hit.stun = true;
                        }
                        let dealt = hit.amount;
                        hits.push(hit);
                        if effect == Effect::Drain {
                            hits.push(self.mend(side, slot, dealt * DRAIN_SHARE / 100));
                        }
                        if effect == Effect::Haste {
                            for k in 0..self.sides[side].len() {
                                let f = &mut self.sides[side][k];
                                if k != slot && f.alive() {
                                    f.gauge += HASTE_GAUGE;
                                    hits.push(Hit {
                                        side,
                                        slot: k,
                                        amount: 0,
                                        heal: false,
                                        crit: false,
                                        edge: 0,
                                        ko: false,
                                        stun: false,
                                        hasted: true,
                                    });
                                }
                            }
                        }
                    }
                }
                Effect::Snipe => {
                    if let Some(t) = self.weakest_target(side) {
                        hits.push(self.strike(side, slot, t, power));
                    }
                }
                Effect::Double => {
                    for _ in 0..2 {
                        if let Some(t) = self.usual_target(side, kind) {
                            hits.push(self.strike(side, slot, t, power));
                        }
                    }
                }
                Effect::Sweep => {
                    for t in 0..self.sides[1 - side].len() {
                        if self.sides[1 - side][t].alive() {
                            hits.push(self.strike(side, slot, t, power));
                        }
                    }
                }
                Effect::Heal => {
                    let mine = &self.sides[side];
                    let t = (0..mine.len())
                        .filter(|&k| mine[k].alive())
                        .min_by_key(|&k| (mine[k].share(), k))
                        .unwrap_or(slot);
                    let amount = self.sides[side][slot].atk * power / 100;
                    hits.push(self.mend(side, t, amount));
                }
            }
            Act::Skill
        };
        let is_skill = act == Act::Skill;
        Turn {
            side,
            slot,
            act,
            skill: is_skill.then(|| self.content.units[unit].skill.name.clone()),
            effect: is_skill.then_some(effect),
            hits,
            after: self.states(),
        }
    }
}

/// The whole fight between side 0 (`a`) and side 1 (`b`). Each side has at
/// least one entry; unknown units are the caller's to refuse beforehand.
pub fn fight(content: &Content, a: &[Entry], b: &[Entry], seed: u64) -> Record {
    let make = |side: &[Entry]| -> Vec<Fighter> {
        side.iter()
            .map(|&(unit, level)| {
                let def = &content.units[unit];
                let level = clamp_unit_level(level);
                let s = def.stats(level);
                Fighter {
                    unit,
                    kind: def.kind,
                    level,
                    max_hp: s.hp,
                    hp: s.hp,
                    atk: s.atk,
                    spd: s.spd,
                    gauge: 0,
                    cooldown: def.skill.cooldown - 1,
                    stunned: false,
                }
            })
            .collect()
    };
    let mut f = Fight {
        content,
        sides: [make(a), make(b)],
        rng: Rng::new(seed),
    };
    let card = |x: &Fighter| {
        let def = &content.units[x.unit];
        Card {
            id: def.id.clone(),
            name: def.name.clone(),
            kind: def.kind,
            level: x.level,
            hp: x.max_hp,
            atk: x.atk,
            spd: x.spd,
            skill: def.skill.name.clone(),
        }
    };
    let teams = [
        f.sides[0].iter().map(card).collect(),
        f.sides[1].iter().map(card).collect(),
    ];
    let start = f.states();
    let mut turns = Vec::new();
    while f.standing(0) > 0 && f.standing(1) > 0 && (turns.len() as u32) < MAX_TURNS {
        let (s, k) = f.next_actor();
        turns.push(f.take_turn(s, k));
    }
    let survivors = [f.standing(0), f.standing(1)];
    let winner = if survivors[1] == 0 {
        Some(0)
    } else if survivors[0] == 0 {
        Some(1)
    } else {
        let total = |s: usize| f.sides[s].iter().map(|x| x.share().max(0)).sum::<i32>();
        match total(0).cmp(&total(1)) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        }
    };
    Record {
        seed,
        teams,
        start,
        turns,
        winner,
        survivors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixtures::content;

    fn ids(c: &Content, team: &[(&str, u32)]) -> Vec<Entry> {
        team.iter()
            .map(|(id, l)| (c.unit_index(id).unwrap(), *l))
            .collect()
    }

    #[test]
    fn the_same_seed_and_teams_give_the_same_fight() {
        let c = content();
        let a = ids(&c, &[("rook", 5), ("vela", 5), ("pip", 5)]);
        let b = ids(&c, &[("ed", 5), ("nan", 5), ("june", 5)]);
        assert_eq!(fight(&c, &a, &b, 42), fight(&c, &a, &b, 42));
        assert_ne!(fight(&c, &a, &b, 42).sum(), fight(&c, &a, &b, 43).sum());
    }

    #[test]
    fn every_fight_ends_with_a_side_down_or_at_the_turn_limit() {
        let c = content();
        for seed in 0..60 {
            let a = ids(&c, &[("rook", 3), ("vela", 7), ("pip", 1)]);
            let b = ids(&c, &[("chum", 9), ("nan", 2), ("tagger", 4)]);
            let r = fight(&c, &a, &b, seed);
            assert!(r.survivors[0] == 0 || r.survivors[1] == 0 || r.turns.len() as u32 == MAX_TURNS);
            let last = r.turns.last().unwrap();
            for side in 0..2 {
                let standing = last.after[side].iter().filter(|s| s.alive).count() as u32;
                assert_eq!(standing, r.survivors[side]);
            }
        }
    }

    #[test]
    fn levels_matter_twenty_beats_one_nearly_always() {
        let c = content();
        let a = ids(&c, &[("rook", 20), ("vela", 20), ("pip", 20)]);
        let b = ids(&c, &[("rook", 1), ("vela", 1), ("pip", 1)]);
        let wins = (0..50)
            .filter(|&s| fight(&c, &a, &b, s).winner == Some(0))
            .count();
        assert_eq!(wins, 50);
    }

    #[test]
    fn the_type_edge_decides_an_even_mirror_match() {
        // Three melee against three magic at the same level and similar
        // numbers: magic has the edge on every blow.
        let c = content();
        let magic = ids(&c, &[("vela", 8), ("static", 8), ("sage", 8)]);
        let melee = ids(&c, &[("rook", 8), ("ed", 8), ("biscuit", 8)]);
        let wins = (0..100)
            .filter(|&s| fight(&c, &magic, &melee, s).winner == Some(0))
            .count();
        assert!(wins >= 80, "magic won {wins} of 100");
    }

    #[test]
    fn a_hit_with_the_edge_does_more_than_one_against_it() {
        let c = content();
        // Pip (ranged) against Vela (magic): the edge. Against Rook (melee): not.
        let pip = ids(&c, &[("pip", 10)]);
        let good = fight(&c, &pip, &ids(&c, &[("vela", 10)]), 5);
        let bad = fight(&c, &pip, &ids(&c, &[("rook", 10)]), 5);
        let first = |r: &Record| {
            r.turns
                .iter()
                .find(|t| t.side == 0 && t.act == Act::Attack)
                .unwrap()
                .hits[0]
                .clone()
        };
        assert_eq!(first(&good).edge, 1);
        assert_eq!(first(&bad).edge, -1);
        assert!(first(&good).amount > first(&bad).amount * 3 / 2);
    }

    #[test]
    fn the_faster_unit_moves_first() {
        let c = content();
        let r = fight(&c, &ids(&c, &[("chum", 1)]), &ids(&c, &[("biscuit", 1)]), 1);
        assert_eq!((r.turns[0].side, r.turns[0].slot), (1, 0));
    }

    #[test]
    fn a_skill_fires_when_its_cooldown_is_up_and_waits_otherwise() {
        let c = content();
        // Rook's Haymaker has cooldown 3, so it starts two turns away.
        let r = fight(&c, &ids(&c, &[("rook", 20)]), &ids(&c, &[("chum", 20)]), 3);
        let mine: Vec<&Turn> = r.turns.iter().filter(|t| t.side == 0).collect();
        assert_eq!(mine[0].act, Act::Attack);
        assert_eq!(mine[1].act, Act::Attack);
        assert_eq!(mine[2].act, Act::Skill);
        assert_eq!(mine[2].skill.as_deref(), Some("Haymaker"));
        assert_eq!(mine[3].act, Act::Attack);
    }

    #[test]
    fn a_stunned_unit_loses_its_next_turn() {
        let c = content();
        let r = fight(&c, &ids(&c, &[("rook", 20)]), &ids(&c, &[("chum", 20)]), 3);
        let at = r
            .turns
            .iter()
            .position(|t| t.side == 0 && t.act == Act::Skill)
            .unwrap();
        assert!(r.turns[at].hits[0].stun);
        let next_foe = r.turns[at + 1..].iter().find(|t| t.side == 1).unwrap();
        assert_eq!(next_foe.act, Act::Stunned);
        assert!(next_foe.hits.is_empty());
    }

    #[test]
    fn a_sweep_hits_every_enemy_standing() {
        let c = content();
        let r = fight(
            &c,
            &ids(&c, &[("vela", 20)]),
            &ids(&c, &[("rook", 1), ("ed", 1), ("chum", 1)]),
            2,
        );
        let sweep = r
            .turns
            .iter()
            .find(|t| t.act == Act::Skill && t.side == 0)
            .unwrap();
        let before =
            sweep.after[1].iter().filter(|s| s.alive).count() + sweep.hits.iter().filter(|h| h.ko).count();
        assert_eq!(sweep.hits.len(), before);
        assert!(sweep.hits.iter().all(|h| h.side == 1 && !h.heal));
    }

    #[test]
    fn a_heal_mends_the_ally_with_least_health_left_and_never_overfills() {
        let c = content();
        let r = fight(
            &c,
            &ids(&c, &[("nan", 15), ("rook", 15)]),
            &ids(&c, &[("vela", 15), ("pip", 15)]),
            9,
        );
        for t in r
            .turns
            .iter()
            .filter(|t| t.side == 0 && t.slot == 0 && t.act == Act::Skill)
        {
            let h = &t.hits[0];
            assert!(h.heal && h.side == 0);
            for s in &t.after[0] {
                assert!(s.hp_pct <= 100);
            }
        }
        assert!(r.turns.iter().any(|t| t.side == 0 && t.act == Act::Skill));
    }

    #[test]
    fn drain_mends_the_attacker_by_half_the_damage() {
        let c = content();
        let r = fight(&c, &ids(&c, &[("chum", 20)]), &ids(&c, &[("chum", 20)]), 11);
        let t = r
            .turns
            .iter()
            .find(|t| t.act == Act::Skill && t.hits.len() == 2 && t.hits[1].amount > 0)
            .unwrap();
        assert!(!t.hits[0].heal && t.hits[1].heal);
        assert_eq!(t.hits[1].slot, t.slot);
        assert!(t.hits[1].amount <= t.hits[0].amount * DRAIN_SHARE / 100);
    }

    #[test]
    fn haste_moves_every_other_ally_closer_to_a_turn() {
        let c = content();
        let r = fight(
            &c,
            &ids(&c, &[("tagger", 10), ("rook", 10), ("pip", 10)]),
            &ids(&c, &[("chum", 20), ("ed", 20), ("nan", 20)]),
            4,
        );
        let t = r
            .turns
            .iter()
            .find(|t| t.side == 0 && t.act == Act::Skill)
            .unwrap();
        let hasted: Vec<usize> = t.hits.iter().filter(|h| h.hasted).map(|h| h.slot).collect();
        assert!(!hasted.is_empty() && !hasted.contains(&t.slot));
    }

    #[test]
    fn a_snipe_goes_for_the_enemy_with_least_health() {
        let c = content();
        let r = fight(
            &c,
            &ids(&c, &[("pip", 20)]),
            &ids(&c, &[("chum", 20), ("biscuit", 1)]),
            8,
        );
        let t = r
            .turns
            .iter()
            .find(|t| t.side == 0 && t.act == Act::Skill)
            .unwrap();
        let foes: Vec<i32> = r.turns[..r.turns.iter().position(|x| std::ptr::eq(x, t)).unwrap()]
            .last()
            .map(|p| {
                p.after[1]
                    .iter()
                    .map(|s| if s.alive { s.hp } else { i32::MAX })
                    .collect()
            })
            .unwrap_or_default();
        let weakest = (0..foes.len()).min_by_key(|&k| (foes[k], k)).unwrap();
        assert_eq!(t.hits[0].slot, weakest);
    }

    #[test]
    fn the_mirror_is_the_same_fight_from_the_other_seat() {
        let c = content();
        let a = ids(&c, &[("rook", 4), ("vela", 6), ("pip", 2)]);
        let b = ids(&c, &[("june", 5), ("nan", 3), ("ed", 4)]);
        let r = fight(&c, &a, &b, 77);
        let m = r.mirrored();
        assert_eq!(m.teams[0], r.teams[1]);
        assert_eq!(m.winner, r.winner.map(|w| 1 - w));
        assert_eq!(m.turns[0].side, 1 - r.turns[0].side);
        assert_eq!(m.mirrored(), r);
    }

    #[test]
    fn a_unit_never_shows_less_than_zero_health() {
        let c = content();
        let r = fight(
            &c,
            &ids(&c, &[("ed", 20), ("ed", 20), ("ed", 20)]),
            &ids(&c, &[("pip", 1), ("pip", 1), ("pip", 1)]),
            1,
        );
        for t in &r.turns {
            for side in &t.after {
                for s in side {
                    assert!(s.hp >= 0 && (0..=100).contains(&s.hp_pct) && (0..=100).contains(&s.gauge_pct));
                    assert_eq!(s.alive, s.hp > 0);
                }
            }
        }
    }
}
