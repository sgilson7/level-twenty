//! The fight in 3D. A fight is already decided (`battle::Record`); this
//! module says, for any moment of its playback, where everyone stands and
//! what flies between them, and where the camera looks.
//!
//! Nobody stands still for long, as in Final Fantasy XIII's battles: between
//! one turn and the next every unit that is not being hit jogs to a new spot,
//! lining itself up on the foe it means to hit next, melee close, magic and
//! ranged further off. On its own turn a melee unit runs in and swings, a
//! ranged unit shoots, a magic user raises an orb and hurls it, a healer
//! lights the ones it mends; a Flank runs round to the target's side first,
//! and a Dodge rolls clear after its blow. A blow that misses meets a
//! sidestep. Hits knock their targets back, and the fallen fall. The page
//! asks for a moment and draws what comes back.
//!
//! Playback time is in milliseconds at speed ×1; the page scales its own
//! clock for faster speeds. Each turn has a start, the moment its hits land
//! (when the page updates the tiles and plays the hit sounds) and an end.

use serde::Serialize;

use crate::battle::{Act, Record, State};
use crate::content::{hex, Content, Effect, Kind, Look, Palette, Rgb};
use crate::figure::{self, cm, Pose};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, yaw_pitch_of, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{self, mix, shade, Batch, Camera};
use crate::rng::Rng;

/// How a turn plays out on the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// Run in, swing, run back.
    Charge,
    /// Throw or fire, strafing.
    Shoot,
    /// Raise an orb and hurl it, strafing.
    Spell,
    /// Light up allies.
    Support,
    /// Shake off a stun.
    Dazed,
    /// Run round to the target's side, then attack as its kind does.
    Flank,
    /// Attack as its kind does, then roll clear.
    Dodge,
}

/// Milliseconds a turn of each style takes, and the share of it (in
/// thousandths) at which its hits land.
fn timing(style: Style) -> (i32, i32) {
    match style {
        Style::Charge => (1500, 480),
        Style::Shoot => (1100, 600),
        Style::Spell => (1300, 620),
        Style::Support => (1100, 500),
        Style::Dazed => (800, 500),
        Style::Flank => (1900, 560),
        Style::Dodge => (1700, 420),
    }
}

/// A pause before the first turn, so the stage is seen before anyone moves.
const OPENING_MS: i32 = 700;
/// How long a knocked-back target takes to recover, a fallen one to fall,
/// a burst to fade, a missed target to sidestep and come back.
const RECOIL_MS: i32 = 350;
const FALL_MS: i32 = 450;
const FLASH_MS: i32 = 160;
const BURST_MS: i32 = 450;
const RING_MS: i32 = 700;
const HOP_MS: i32 = 440;

/// The floor everyone keeps to: half its width (x) and depth (z).
const FLOOR_X: i32 = cm(760);
const FLOOR_Z: i32 = cm(580);
/// Nearest a unit comes to its own side's edge of the middle line.
const MIDDLE: i32 = cm(230);
/// Nearest two units stand to one another between turns.
const APART: i32 = cm(230);
/// Furthest a unit jogs from one turn to the next, and rolls on a Dodge.
const STRIDE: i32 = cm(420);
const ROLL: i32 = cm(340);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Beat {
    pub start: i32,
    pub hit: i32,
    pub end: i32,
    pub style: Style,
}

pub fn style_of(record: &Record, content: &Content, i: usize) -> Style {
    let t = &record.turns[i];
    let id = &record.teams[t.side][t.slot].id;
    let kind = content
        .unit_index(id)
        .map(|u| content.units[u].kind)
        .unwrap_or(Kind::Melee);
    match (t.act, t.effect) {
        (Act::Stunned, _) => Style::Dazed,
        (Act::Skill, Some(Effect::Heal | Effect::Mend)) => Style::Support,
        (Act::Skill, Some(Effect::Flank)) => Style::Flank,
        (Act::Skill, Some(Effect::Dodge)) => Style::Dodge,
        _ => match kind {
            Kind::Melee => Style::Charge,
            Kind::Ranged => Style::Shoot,
            Kind::Magic => Style::Spell,
        },
    }
}

/// When each turn starts, lands and ends.
pub fn timeline(record: &Record, content: &Content) -> Vec<Beat> {
    let mut at = OPENING_MS;
    (0..record.turns.len())
        .map(|i| {
            let style = style_of(record, content, i);
            let (len, hit) = timing(style);
            let b = Beat {
                start: at,
                hit: at + len * hit / 1000,
                end: at + len,
                style,
            };
            at += len;
            b
        })
        .collect()
}

/// What a turn's blow travels as: a shot, an orb, or nothing (a melee blow
/// or a heal).
fn delivery(style: Style, kind: Kind) -> Option<Kind> {
    match style {
        Style::Shoot => Some(Kind::Ranged),
        Style::Spell => Some(Kind::Magic),
        Style::Flank | Style::Dodge if kind != Kind::Melee => Some(kind),
        _ => None,
    }
}

/// The share of a turn at which a shot leaves the hand or an orb is fully
/// grown, given the share at which it lands.
fn launch(style: Style, kind: Kind, hs: i32) -> i32 {
    match (style, kind) {
        (Style::Flank, _) => hs * 8 / 10,
        (_, Kind::Ranged) => hs / 2,
        _ => hs * 2 / 3,
    }
}

/// A fight on the stage: the record, the looks of the six, the place, and
/// where each unit stands before every turn.
#[derive(Debug, Clone)]
pub struct Show {
    pub record: Record,
    pub beats: Vec<Beat>,
    looks: [Vec<Look>; 2],
    kinds: [Vec<Kind>; 2],
    palette: Palette,
    backdrop: String,
    /// `spots[i]`: where everyone stands as turn `i` starts; the last entry
    /// is where they end.
    spots: Vec<[Vec<V3>; 2]>,
}

/// Where a unit stands when the fight opens. Side 0 on the left (−x), side 1
/// on the right; slot 0 at the back, slot 2 at the front; the middle slot a
/// step nearer the middle, as the tiles beside the stage are.
pub fn home(side: usize, slot: usize) -> V3 {
    let x = if slot == 1 { cm(450) } else { cm(570) };
    let z = cm(320) * (slot as i32 - 1);
    v3(if side == 0 { -x } else { x }, 0, z)
}

/// Toward a side's own edge of the floor, along x.
fn own(side: usize) -> i32 {
    if side == 0 {
        -ONE
    } else {
        ONE
    }
}

fn on_floor(p: V3) -> V3 {
    v3(p.x.clamp(-FLOOR_X, FLOOR_X), 0, p.z.clamp(-FLOOR_Z, FLOOR_Z))
}

/// On the floor and on its own side of the middle.
fn on_own_side(p: V3, side: usize) -> V3 {
    let p = on_floor(p);
    let x = if side == 0 {
        p.x.min(-MIDDLE)
    } else {
        p.x.max(MIDDLE)
    };
    v3(x, 0, p.z)
}

/// Square to `d` on the ground, the same length.
fn across(d: V3) -> V3 {
    v3(-d.z, 0, d.x)
}

fn facing_yaw(from: V3, to: V3) -> i32 {
    yaw_pitch_of(v3(to.x - from.x, 0, to.z - from.z)).0
}

/// `num / den` as a share in `0..=ONE`.
fn share(num: i32, den: i32) -> i32 {
    if den <= 0 {
        return ONE;
    }
    ((num.clamp(0, den) as i64 * ONE as i64) / den as i64) as i32
}

/// Eases a share in and out.
fn ease(s: i32) -> i32 {
    // 3s² − 2s³
    let s2 = fx::mul(s, s);
    3 * s2 - 2 * fx::mul(s2, s)
}

/// A bump up and back down over a share.
fn bump(s: i32) -> i32 {
    fx::sin(fx::mul(fx::HALF, s.clamp(0, ONE)))
}

/// From `a` to `b` by share `s`, bowed out sideways by `bow` at the middle
/// and lifted by `lift` (a run round, a roll, a hop).
fn arc(a: V3, b: V3, s: i32, bow: V3, lift: i32) -> V3 {
    let p = on_floor(a.lerp(b, s).add(bow.scale(bump(s))));
    v3(p.x, a.y + fx::mul(lift, bump(s)), p.z)
}

/// Where everyone stands before each turn and after the last. A unit being
/// hit holds its ground for that turn (so blows find it); everyone else
/// standing jogs up to a stride toward a spot lined up on the foe it hits
/// next, at its kind's range and an angle of its own; a Dodge rolls aside.
fn plan_spots(record: &Record, kinds: &[Vec<Kind>; 2]) -> Vec<[Vec<V3>; 2]> {
    let n = [record.teams[0].len(), record.teams[1].len()];
    let homes = [
        (0..n[0]).map(|k| home(0, k)).collect(),
        (0..n[1]).map(|k| home(1, k)).collect(),
    ];
    let mut out: Vec<[Vec<V3>; 2]> = vec![homes];
    let mut rng = Rng::new(record.seed ^ 0x5745_A6E5);
    for (i, turn) in record.turns.iter().enumerate() {
        let prev = out[i].clone();
        let mut next = prev.clone();
        let mut moved = Vec::new();
        for side in 0..2 {
            for slot in 0..n[side] {
                let actor = turn.side == side && turn.slot == slot;
                let struck = turn.hits.iter().any(|h| h.side == side && h.slot == slot);
                if !turn.after[side][slot].alive || (struck && !actor) {
                    continue;
                }
                let p = prev[side][slot];
                let want = if actor && turn.act == Act::Skill && turn.effect == Some(Effect::Dodge) {
                    let way = if p.z > cm(200) {
                        -ONE
                    } else if p.z < -cm(200) || rng.chance(50) {
                        ONE
                    } else {
                        -ONE
                    };
                    p.add(v3(own(side) / 3, 0, way).norm().scale(ROLL))
                } else {
                    let foe = lineup(record, i, side, slot, &prev);
                    let reach = match kinds[side][slot] {
                        Kind::Melee => rng.range(380, 520),
                        Kind::Magic => rng.range(680, 860),
                        Kind::Ranged => rng.range(780, 980),
                    };
                    let a = deg(rng.range(-50, 50));
                    let dir = v3(fx::mul(fx::cos(a), own(side)), 0, fx::sin(a));
                    let w = prev[1 - side][foe].add(dir.scale(cm(reach)));
                    // keep to a lane of its own across the floor, loosely
                    let lane = cm(340) * (slot as i32 - 1) + cm(rng.range(-110, 110));
                    let w = v3(w.x, 0, (w.z + lane) / 2);
                    p.add(w.sub(p).clamp_len(STRIDE))
                };
                next[side][slot] = on_own_side(want, side);
                moved.push((side, slot));
            }
        }
        // nobody ends a jog standing in anybody
        for _ in 0..6 {
            for &(s, k) in &moved {
                let mut p = next[s][k];
                for (s2, row) in next.iter().enumerate() {
                    for (k2, &q) in row.iter().enumerate() {
                        if (s2, k2) == (s, k) {
                            continue;
                        }
                        let d = v3(p.x - q.x, 0, p.z - q.z);
                        if d.len() < APART {
                            let away = if d.len() == 0 { v3(own(s), 0, 0) } else { d.norm() };
                            p = on_own_side(q.add(away.scale(APART)), s);
                        }
                        if p.dist(q) < APART {
                            // pinned against the middle or an edge: step along it
                            let way = if p.z > q.z || q.z < -FLOOR_Z + APART {
                                ONE
                            } else {
                                -ONE
                            };
                            p = on_own_side(v3(p.x, 0, q.z + fx::mul(way, APART)), s);
                        }
                    }
                }
                next[s][k] = on_own_side(p, s);
            }
        }
        out.push(next);
    }
    out
}

/// The foe a unit lines up on after turn `i`: the one its next turn hits,
/// else the nearest standing.
fn lineup(record: &Record, i: usize, side: usize, slot: usize, at: &[Vec<V3>; 2]) -> usize {
    let standing = &record.turns[i].after[1 - side];
    record.turns[i + 1..]
        .iter()
        .find(|t| t.side == side && t.slot == slot && !t.hits.is_empty())
        .and_then(|t| t.hits.iter().find(|h| h.side != side))
        .map(|h| h.slot)
        .filter(|&k| standing[k].alive)
        .or_else(|| {
            (0..standing.len())
                .filter(|&k| standing[k].alive)
                .min_by_key(|&k| (at[side][slot].dist(at[1 - side][k]), k))
        })
        .unwrap_or(0)
}

/// Where a unit is and what its body is doing at one moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Motion {
    at: V3,
    /// `None`: facing the nearest foe standing.
    yaw: Option<i32>,
    moving: i32,
    swing: i32,
    cast: i32,
    /// Added to the yaw: a dazed unit shaking its head.
    wobble: i32,
}

/// What the page needs to draw one moment.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub view: Vec<i32>,
    pub values: Vec<i32>,
    pub draws: Vec<u32>,
    /// Each unit's head on screen: x and y in percent (Q16), side 0 then 1.
    pub marks: Vec<i32>,
}

impl Show {
    pub fn new(content: &Content, record: Record, chapter: usize) -> Show {
        let beats = timeline(&record, content);
        let look = |id: &String| {
            content
                .unit_index(id)
                .map(|u| content.units[u].look.clone())
                .unwrap_or_else(|| content.units[0].look.clone())
        };
        let kind = |id: &String| {
            content
                .unit_index(id)
                .map(|u| content.units[u].kind)
                .unwrap_or(Kind::Melee)
        };
        let ch = &content.chapters[chapter.min(content.chapters.len() - 1)];
        let kinds = [
            record.teams[0].iter().map(|c| kind(&c.id)).collect(),
            record.teams[1].iter().map(|c| kind(&c.id)).collect(),
        ];
        Show {
            looks: [
                record.teams[0].iter().map(|c| look(&c.id)).collect(),
                record.teams[1].iter().map(|c| look(&c.id)).collect(),
            ],
            spots: plan_spots(&record, &kinds),
            kinds,
            palette: ch.palette.clone(),
            backdrop: ch.backdrop.clone(),
            record,
            beats,
        }
    }

    pub fn total(&self) -> i32 {
        self.beats.last().map(|b| b.end).unwrap_or(OPENING_MS) + OPENING_MS
    }

    /// The colours the shader needs: sky, fog, fog range (near, far), light,
    /// light direction, ink.
    pub fn uniforms(&self) -> Vec<i32> {
        let p = &self.palette;
        let mut u = Vec::new();
        u.extend(hex(&p.sky));
        u.extend(hex(&p.fog));
        u.extend([fx::int(18), fx::int(60)]);
        u.extend([ONE, ONE * 97 / 100, ONE * 90 / 100]);
        let l = v3(ONE / 3, ONE, ONE / 2).norm();
        u.extend([l.x, l.y, l.z]);
        u.extend(INK);
        u
    }

    pub fn clear(&self) -> Rgb {
        hex(&self.palette.sky)
    }

    /// The scenery, which never moves: built once a fight.
    pub fn scene(&self) -> (Vec<i32>, Vec<Vec<u32>>) {
        let mut b = Batch::new(INK);
        let p = &self.palette;
        let walls: Vec<Rgb> = p.walls.iter().map(|w| hex(w)).collect();
        let trim = hex(&p.trim);
        let mut rng = Rng::new(self.backdrop.bytes().map(|c| c as u64).sum());
        b.push(
            Mesh::Ground,
            &Affine::scale(v3(fx::int(160), ONE, fx::int(160))),
            hex(&p.ground),
            0,
            render::PASS_SOLID,
        );
        // the fighting floor, a raised disc
        b.inked(
            Mesh::Cyl,
            &render::boxed(v3(0, cm(3), 0), v3(fx::int(18), cm(6), fx::int(14))),
            hex(&p.floor),
            0,
        );
        match self.backdrop.as_str() {
            "rooftops" => rooftops(&mut b, &walls, trim, &mut rng),
            "junkyard" => junkyard(&mut b, &walls, trim, &mut rng),
            _ => market(&mut b, &walls, trim, &mut rng),
        }
        b.finish(render::STATIC_BUFFER)
    }

    fn state_at(&self, t: i32) -> (usize, [Vec<State>; 2]) {
        // the turn under way (or the last), and the state the tiles show
        let mut current = 0;
        let mut states = self.record.start.clone();
        for (i, b) in self.beats.iter().enumerate() {
            if t >= b.start {
                current = i;
            }
            if t >= b.hit {
                states = self.record.turns[i].after.clone();
            }
        }
        (current, states)
    }

    /// When each unit went down, if it has by `t`.
    fn fell_at(&self, t: i32) -> [Vec<Option<i32>>; 2] {
        let mut out = [vec![None; self.looks[0].len()], vec![None; self.looks[1].len()]];
        for (i, b) in self.beats.iter().enumerate() {
            if b.hit > t {
                break;
            }
            for h in &self.record.turns[i].hits {
                if h.ko && out[h.side][h.slot].is_none() {
                    out[h.side][h.slot] = Some(b.hit);
                }
            }
        }
        out
    }

    /// The first enemy a turn hits, else its first ally, else itself.
    fn target_of(&self, i: usize) -> (usize, usize) {
        let t = &self.record.turns[i];
        t.hits
            .iter()
            .find(|h| h.side != t.side)
            .or_else(|| t.hits.first())
            .map(|h| (h.side, h.slot))
            .unwrap_or((t.side, t.slot))
    }

    /// The turn under way at `t`, if any (turns follow one another).
    fn turn_at(&self, t: i32) -> Option<usize> {
        self.beats.iter().position(|b| t >= b.start && t < b.end)
    }

    /// How many turns are over by `t`: the index of the spots to stand on.
    fn settled(&self, t: i32) -> usize {
        self.beats.iter().filter(|b| b.end <= t).count()
    }

    /// Where the enemies a turn hits stand: the one target, or the middle of
    /// several (a sweep).
    fn aim_of(&self, i: usize) -> V3 {
        let t = &self.record.turns[i];
        let mut seen: Vec<(usize, usize)> = Vec::new();
        for h in t.hits.iter().filter(|h| h.side != t.side) {
            if !seen.contains(&(h.side, h.slot)) {
                seen.push((h.side, h.slot));
            }
        }
        let n = seen.len() as i32;
        let sum = seen
            .iter()
            .fold(V3::ZERO, |a, &(s, k)| a.add(self.spots[i][s][k]));
        if n == 0 {
            let (s, k) = self.target_of(i);
            return self.spots[i][s][k];
        }
        v3(sum.x / n, 0, sum.z / n)
    }

    /// The spot beside the target a Flank runs to: square to the line
    /// between them, close for melee and at range for the others, on
    /// whichever side has more floor.
    fn flank_spot(&self, from: V3, to: V3, reach: i32) -> V3 {
        let side_on = across(v3(from.x - to.x, 0, from.z - to.z).norm());
        let a = on_floor(to.add(side_on.scale(reach)));
        let b = on_floor(to.sub(side_on.scale(reach)));
        if a.z.abs() <= b.z.abs() {
            a
        } else {
            b
        }
    }

    fn motion(&self, side: usize, slot: usize, t: i32) -> Motion {
        let rest = Motion {
            at: self.spots[self.settled(t)][side][slot],
            ..Motion::default()
        };
        let Some(i) = self.turn_at(t) else {
            return rest;
        };
        let bt = self.beats[i];
        let f = share(t - bt.start, bt.end - bt.start);
        let (p0, p1) = (self.spots[i][side][slot], self.spots[i + 1][side][slot]);
        let turn = &self.record.turns[i];
        if turn.side == side && turn.slot == slot {
            return self.act(i, t, p0, p1);
        }
        let mut m = Motion { at: p0, ..rest };
        // jogging to the next spot, done well before the turn ends
        if p0 != p1 {
            let s = share(f, ONE * 8 / 10);
            m.at = p0.lerp(p1, ease(s));
            if s < ONE {
                m.moving = ONE;
                m.yaw = Some(facing_yaw(p0, p1));
            }
        }
        // a blow that misses: a hop aside, and back
        if turn
            .hits
            .iter()
            .any(|h| h.side == side && h.slot == slot && h.missed)
        {
            let from = self.spots[i][turn.side][turn.slot];
            let mut aside = across(p0.sub(from)).norm();
            if p0.add(aside).len_xz() > p0.sub(aside).len_xz() {
                aside = aside.neg();
            }
            let s = share(t - (bt.hit - HOP_MS / 2), HOP_MS);
            m.at = arc(p0, p0, s, aside.scale(cm(160)), cm(45));
        }
        m
    }

    /// The unit taking turn `i`, at `t`, from `p0` to `p1`.
    fn act(&self, i: usize, t: i32, p0: V3, p1: V3) -> Motion {
        let bt = self.beats[i];
        let turn = &self.record.turns[i];
        let (side, slot) = (turn.side, turn.slot);
        let kind = self.kinds[side][slot];
        let look = &self.looks[side][slot];
        let f = share(t - bt.start, bt.end - bt.start);
        let hs = share(bt.hit - bt.start, bt.end - bt.start);
        let aim = self.aim_of(i);
        let mut m = Motion {
            at: p0,
            ..Motion::default()
        };
        let melee = bt.style == Style::Charge
            || (kind == Kind::Melee && matches!(bt.style, Style::Flank | Style::Dodge));
        let gap = if look.size > 140 { cm(220) } else { cm(130) };
        // where the blow is struck from, if not from where it stands
        let spot = if bt.style == Style::Flank {
            Some(self.flank_spot(p0, aim, if melee { gap } else { cm(470) }))
        } else if melee {
            Some(aim.add(v3(p0.x - aim.x, 0, p0.z - aim.z).norm().scale(gap)))
        } else {
            None
        };
        let run = if bt.style == Style::Flank {
            hs * 6 / 10
        } else {
            hs * 7 / 10
        };
        let back = hs + (ONE - hs) / if bt.style == Style::Dodge { 5 } else { 3 };
        let from = spot.unwrap_or(p0);
        if let Some(spot) = spot.filter(|_| f < run) {
            // running there, round the side for a flank
            let s = ease(share(f, run));
            let mut bow = across(spot.sub(p0)).norm().scale(cm(150));
            if bow.dot(p0.lerp(spot, ONE / 2).sub(aim)) < 0 {
                bow = bow.neg();
            }
            let bow = if bt.style == Style::Flank { bow } else { V3::ZERO };
            m.at = arc(p0, spot, s, bow, 0);
            m.moving = ONE;
            m.yaw = Some(facing_yaw(p0, spot));
        } else if f < back {
            // striking
            m.at = if spot.is_some() {
                from
            } else {
                p0.lerp(p1, ease(f) / 3)
            };
            m.yaw = Some(facing_yaw(m.at, aim));
            if melee {
                m.swing = share(f - run, back - run);
            }
        } else {
            // going on to the next spot: a roll for a dodge, a run otherwise
            let s = ease(share(f - back, ONE - back));
            let start = if spot.is_some() {
                from
            } else {
                p0.lerp(p1, ease(back) / 3)
            };
            if bt.style == Style::Dodge {
                let bow = across(p1.sub(start)).norm().scale(cm(60));
                m.at = arc(start, p1, s, bow, cm(70));
            } else {
                m.at = start.lerp(p1, s);
            }
            if start.dist(p1) > cm(30) {
                m.moving = ONE - s / 2;
                m.yaw = Some(if s < ONE * 9 / 10 {
                    facing_yaw(start, p1)
                } else {
                    facing_yaw(p1, aim)
                });
            }
        }
        // the arms: a shot drawn and loosed, an orb raised and thrown
        let up = launch(bt.style, kind, hs);
        match (bt.style, delivery(bt.style, kind)) {
            (Style::Support, _) => {
                let rise = if f < hs {
                    share(f, up)
                } else {
                    ONE - share(f - hs, ONE - hs)
                };
                m.cast = ease(rise);
                let (s, k) = self.target_of(i);
                let ally = self.spots[i][s][k];
                m.yaw = Some(if ally == p0 {
                    facing_yaw(p0, p0.add(v3(own(1 - side), 0, 0)))
                } else {
                    facing_yaw(m.at, ally)
                });
            }
            (Style::Dazed, _) => {
                m.yaw = None;
                m.wobble = fx::mul(deg(18), fx::sin(fx::TURN * (t % 300) / 300));
            }
            (_, Some(Kind::Ranged)) if f >= run || spot.is_none() => {
                m.swing = share(f - up, hs);
            }
            (_, Some(Kind::Magic)) => {
                let rise = if f < hs {
                    share(f, up)
                } else {
                    ONE - share(f - hs, ONE - hs)
                };
                m.cast = ease(rise);
            }
            _ => {}
        }
        m
    }

    /// Where a shot or orb starts from at `t`: the hand, or above the head.
    fn launcher(&self, i: usize, t: i32, kind: Kind) -> V3 {
        let tn = &self.record.turns[i];
        let look = &self.looks[tn.side][tn.slot];
        let m = self.motion(tn.side, tn.slot, t);
        let aim = self.aim_of(i);
        if kind == Kind::Magic {
            return m.at.add(v3(0, figure::height(look) + cm(70), 0));
        }
        figure::anchors(look, m.at, m.yaw.unwrap_or_else(|| facing_yaw(m.at, aim))).0
    }

    pub fn frame(&self, t: i32, aspect: i32) -> Frame {
        let mut b = Batch::new(INK);
        let (_, states) = self.state_at(t);
        let fell = self.fell_at(t);
        let cur = self.turn_at(t);
        let n = [self.looks[0].len(), self.looks[1].len()];
        let motions: [Vec<Motion>; 2] = [
            (0..n[0]).map(|k| self.motion(0, k, t)).collect(),
            (0..n[1]).map(|k| self.motion(1, k, t)).collect(),
        ];
        let mut heads = Vec::new();
        let mut focus = v3(0, ONE, 0);
        let mut focus_w = 0;
        let mut actor_side_sign = 0;
        if let Some(i) = cur {
            let bt = self.beats[i];
            let tn = &self.record.turns[i];
            focus = motions[tn.side][tn.slot].at.lerp(self.aim_of(i), ONE / 2);
            focus_w = bump(share(t - bt.start, bt.end - bt.start));
            actor_side_sign = own(tn.side) / ONE;
        }
        for side in 0..2 {
            for slot in 0..n[side] {
                let look = &self.looks[side][slot];
                let kind = self.kinds[side][slot];
                let m = motions[side][slot];
                let st = &states[side][slot];
                let at = m.at;
                // between blows, everyone keeps an eye on the nearest foe
                let yaw = m.yaw.unwrap_or_else(|| {
                    (0..n[1 - side])
                        .filter(|&k| states[1 - side][k].alive)
                        .min_by_key(|&k| (at.dist(motions[1 - side][k].at), k))
                        .map(|k| facing_yaw(at, motions[1 - side][k].at))
                        .unwrap_or(facing_yaw(at, v3(0, 0, at.z)))
                }) + m.wobble;
                let mut pose = Pose {
                    breath: fx::TURN * ((t + 700 * (side * 3 + slot) as i32) % 2400) / 2400,
                    moving: m.moving,
                    swing: m.swing,
                    cast: m.cast,
                    step: if m.moving > 0 {
                        fx::TURN * (t % 500) / 500
                    } else {
                        0
                    },
                    ..Pose::default()
                };
                if let Some(when) = fell[side][slot] {
                    pose.down = ease(share(t - when, FALL_MS));
                    pose.breath = 0;
                }
                // hits that landed on this unit lately
                for (i, bt) in self.beats.iter().enumerate() {
                    if bt.hit > t || t - bt.hit > RECOIL_MS.max(FLASH_MS) {
                        continue;
                    }
                    for h in self.record.turns[i]
                        .hits
                        .iter()
                        .filter(|h| h.side == side && h.slot == slot)
                    {
                        let dt = t - bt.hit;
                        if !h.heal && !h.hasted && !h.missed {
                            pose.recoil = pose.recoil.max(
                                bump(share(dt, RECOIL_MS)) * if h.crit { 2 } else { 1 }
                                    / if h.edge < 0 { 2 } else { 1 },
                            );
                            pose.flash = pose.flash.max(ONE - share(dt, FLASH_MS));
                        }
                    }
                }
                figure::draw(&mut b, look, kind, at, yaw, &pose);
                let standing = figure::height(look) + cm(30);
                let top = at.add(v3(0, standing - fx::mul(standing * 4 / 5, pose.down), 0));
                // whose side it is on: a ring of its team's colour at its
                // feet and a marker over its head, as FF13 marks friend and foe
                if pose.down < ONE / 2 {
                    let team = TEAM[side];
                    b.push(
                        Mesh::Cyl,
                        &render::boxed(at.add(v3(0, cm(6), 0)), v3(cm(130), cm(4), cm(130))),
                        team,
                        ONE / 3,
                        render::PASS_SOLID,
                    );
                    b.inked(
                        Mesh::Ring,
                        &render::boxed(at.add(v3(0, cm(10), 0)), v3(cm(170), cm(16), cm(170))),
                        team,
                        ONE / 3,
                    );
                    let bob = fx::mul(cm(8), fx::sin(fx::TURN * ((t + 300 * slot as i32) % 1400) / 1400));
                    let spin = fx::TURN * (t % 3000) / 3000;
                    let over = top.add(v3(0, cm(45) + bob, 0));
                    let a = Affine::translate(over)
                        .then(&Affine::rot_y(spin))
                        .then(&Affine::scale(v3(cm(30), cm(44), cm(30))));
                    b.inked(Mesh::Octa, &a, team, ONE / 3);
                }
                // a stunned unit has stars round its head
                if st.stunned && st.alive {
                    for k in 0..3 {
                        let a = fx::TURN * k / 3 + fx::TURN * (t % 1200) / 1200;
                        let p = top.add(v3(fx::mul(cm(35), fx::cos(a)), 0, fx::mul(cm(35), fx::sin(a))));
                        b.inked(
                            Mesh::Octa,
                            &render::boxed(p, v3(cm(14), cm(14), cm(14))),
                            GOLD,
                            ONE / 2,
                        );
                    }
                }
                // a unit set to dodge has wisps circling its waist
                if st.evading && st.alive {
                    let waist = at.add(v3(0, figure::height(look) / 2, 0));
                    for k in 0..4 {
                        let a = fx::TURN * k / 4 - fx::TURN * (t % 900) / 900;
                        let p = waist.add(v3(
                            fx::mul(cm(75), fx::cos(a)),
                            fx::mul(cm(20), fx::sin(a * 2)),
                            fx::mul(cm(75), fx::sin(a)),
                        ));
                        b.glow(
                            Mesh::Sphere,
                            &render::boxed(p, v3(cm(26), cm(26), cm(26))),
                            DODGE,
                            ONE / 2,
                        );
                    }
                }
                // a ready skill glows at the feet
                if st.ready && st.alive {
                    let pulse = ONE / 4 + fx::mul(ONE / 6, fx::sin(fx::TURN * (t % 1000) / 1000));
                    b.glow(
                        Mesh::Cyl,
                        &render::boxed(at.add(v3(0, cm(8), 0)), v3(cm(150), cm(4), cm(150))),
                        hex(&look.accent),
                        pulse,
                    );
                }
                // dust kicked up by a run
                if m.moving > ONE / 2 && pose.down == 0 {
                    let puff = share(t % 250, 250);
                    b.glow(
                        Mesh::Sphere,
                        &render::boxed(
                            at.add(v3(0, cm(15), 0)),
                            v3(cm(60), cm(25), cm(60)).scale(ONE / 2 + puff / 2),
                        ),
                        DUST,
                        (ONE - puff) / 3,
                    );
                }
                heads.push(top);
            }
        }
        // what flies, and what bursts
        for (i, bt) in self.beats.iter().enumerate() {
            if t < bt.start || t > bt.end + RING_MS {
                continue;
            }
            let tn = &self.record.turns[i];
            let look = &self.looks[tn.side][tn.slot];
            let kind = self.kinds[tn.side][tn.slot];
            let accent = hex(&look.accent);
            let f = share(t - bt.start, bt.end - bt.start);
            let hs = share(bt.hit - bt.start, bt.end - bt.start);
            let up = launch(bt.style, kind, hs);
            let flies = delivery(bt.style, kind);
            let mut seen: Vec<(usize, usize)> = Vec::new();
            for h in &tn.hits {
                if seen.contains(&(h.side, h.slot)) && !h.heal {
                    continue;
                }
                seen.push((h.side, h.slot));
                let tlook = &self.looks[h.side][h.slot];
                let spot = self.spots[i][h.side][h.slot];
                let (_, chest) = figure::anchors(tlook, spot, 0);
                let hostile = h.side != tn.side;
                if hostile && t < bt.hit {
                    match flies {
                        Some(Kind::Ranged) if f >= up => {
                            let hand = self.launcher(i, bt.start + fx::mul(bt.end - bt.start, up), kind);
                            let s = share(f - up, hs - up);
                            let lob = fx::mul(cm(120), bump(s));
                            let p = hand.lerp(chest, s).add(v3(0, lob, 0));
                            let dir = chest.sub(hand).norm();
                            let a = Affine::translate(p)
                                .then(&Affine::looking(dir))
                                .then(&Affine::scale(v3(cm(10), cm(10), cm(40))));
                            b.inked(Mesh::Cube, &a, accent, ONE / 3);
                            b.glow(
                                Mesh::Sphere,
                                &render::boxed(p, v3(cm(40), cm(40), cm(40))),
                                accent,
                                ONE / 2,
                            );
                        }
                        Some(Kind::Magic) => {
                            let (p, r) = if f < up {
                                (self.launcher(i, t, kind), fx::mul(cm(45), share(f, up)))
                            } else {
                                let lift = self.launcher(i, bt.start + fx::mul(bt.end - bt.start, up), kind);
                                (lift.lerp(chest, ease(share(f - up, hs - up))), cm(45))
                            };
                            b.push(
                                Mesh::Sphere,
                                &render::boxed(p, v3(r, r, r)),
                                mix(accent, [ONE; 3], ONE / 3),
                                ONE,
                                render::PASS_SOLID,
                            );
                            b.glow(
                                Mesh::Sphere,
                                &render::boxed(p, v3(r * 3, r * 3, r * 3)),
                                accent,
                                ONE * 2 / 3,
                            );
                        }
                        _ => {}
                    }
                }
                if t >= bt.hit {
                    let dt = t - bt.hit;
                    let at_floor = v3(spot.x, cm(10), spot.z);
                    if hostile && !h.missed && dt < BURST_MS {
                        let s = share(dt, BURST_MS);
                        let r = fx::mul(cm(if h.crit { 160 } else { 100 }), ease(s));
                        let c = if h.flank {
                            DODGE
                        } else if h.edge > 0 {
                            ORANGE
                        } else if h.edge < 0 {
                            shade(accent, 60)
                        } else {
                            accent
                        };
                        b.glow(Mesh::Sphere, &render::boxed(chest, v3(r, r, r)), c, ONE - s);
                    }
                    if dt < RING_MS {
                        let s = share(dt, RING_MS);
                        let ring = if h.heal {
                            Some(HEAL)
                        } else if h.hasted {
                            Some(HASTE)
                        } else if h.slowed {
                            Some(SLOW)
                        } else if h.stun {
                            Some(GOLD)
                        } else if h.missed {
                            Some(DODGE)
                        } else {
                            None
                        };
                        if let Some(c) = ring {
                            let rise = fx::mul(cm(160), s);
                            let r = cm(90) + fx::mul(cm(30), s);
                            b.glow(
                                Mesh::Ring,
                                &render::boxed(at_floor.add(v3(0, rise, 0)), v3(r * 2, cm(30), r * 2)),
                                c,
                                ONE - s,
                            );
                        }
                    }
                }
            }
        }
        // the camera: the whole stage, drifting, leaning in on the action
        let drift = fx::mul(deg(9), fx::sin(fx::TURN * (t % 16000) / 16000));
        let centre = v3(0, cm(60), -cm(60));
        let look_at = centre.lerp(focus.add(v3(0, cm(60), 0)), focus_w / 2);
        // far enough back that everyone standing fits a narrow stage
        let tan_half_fov = ONE * 364 / 1000;
        let fit = fx::div(cm(990), fx::mul(tan_half_fov, aspect.max(ONE / 2))) - fx::int(2);
        let base = fit.clamp(fx::int(15), fx::int(27));
        let dist = base - fx::mul(cm(250), focus_w);
        let height = fx::mul(cm(640), fx::div(base, fx::int(13)));
        let side_lean = fx::mul(cm(70), focus_w) * actor_side_sign;
        let eye = look_at.add(v3(
            fx::mul(fx::sin(drift), dist) + side_lean,
            height,
            fx::mul(fx::cos(drift), dist),
        ));
        let cam = Camera {
            eye,
            fwd: look_at.sub(eye).norm(),
            fov: deg(40),
        };
        let marks = heads
            .iter()
            .flat_map(|p| {
                cam.screen(aspect, *p)
                    .map(|(x, y)| [x, y])
                    .unwrap_or([-ONE, -ONE])
            })
            .collect();
        let (values, draws) = b.finish(render::DYNAMIC_BUFFER);
        Frame {
            view: cam.values(aspect),
            values,
            draws: draws.concat(),
            marks,
        }
    }
}

const INK: Rgb = [ONE * 59 / 255, ONE * 35 / 255, ONE * 64 / 255];
const GOLD: Rgb = [ONE, ONE * 176 / 255, 0];
const ORANGE: Rgb = [ONE, ONE * 120 / 255, ONE * 40 / 255];
const HEAL: Rgb = [ONE * 120 / 255, ONE * 230 / 255, ONE * 110 / 255];
const HASTE: Rgb = [ONE * 80 / 255, ONE * 170 / 255, ONE];
const SLOW: Rgb = [ONE * 170 / 255, ONE * 90 / 255, ONE];
const DODGE: Rgb = [ONE * 120 / 255, ONE * 240 / 255, ONE * 235 / 255];
/// The two sides' colours, the same as their tiles: blue for you, red for the foe.
const TEAM: [Rgb; 2] = [
    [ONE * 60 / 255, ONE * 140 / 255, ONE * 235 / 255],
    [ONE * 225 / 255, ONE * 55 / 255, ONE * 50 / 255],
];
const DUST: Rgb = [ONE * 230 / 255, ONE * 215 / 255, ONE * 190 / 255];

// ---- scenery ----------------------------------------------------------------

/// A block standing on the ground at `foot` (x and z), `size` wide, tall and deep.
fn building(b: &mut Batch, foot: V3, size: V3, c: Rgb, rng: &mut Rng) {
    let (x, z) = (foot.x, foot.z);
    let (w, h, d) = (size.x, size.y, size.z);
    b.inked(Mesh::Cube, &render::boxed(v3(x, h / 2, z), v3(w, h, d)), c, 0);
    b.inked(
        Mesh::Cube,
        &render::boxed(v3(x, h + cm(15), z), v3(w + cm(30), cm(30), d + cm(30))),
        shade(c, 80),
        0,
    );
    // windows on the face toward the stage (+z when behind it, toward x=0 at the sides)
    let face_z = z.abs() > x.abs() / 2;
    let rows = (h - fx::int(2)) / fx::int(3);
    let cols = if face_z { w / cm(250) } else { d / cm(250) };
    for r in 0..rows.max(0) {
        for k in 0..cols.max(0) {
            let lit = rng.chance(40);
            let col = if lit { WINDOW } else { shade(c, 55) };
            let y = fx::int(3) + r * fx::int(3);
            let along = -(if face_z { w } else { d }) / 2 + cm(125) + k * cm(250);
            let (p, s) = if face_z {
                let zz = if z < 0 { z + d / 2 } else { z - d / 2 };
                (v3(x + along, y, zz), v3(cm(110), cm(150), cm(20)))
            } else {
                let xx = if x < 0 { x + w / 2 } else { x - w / 2 };
                (v3(xx, y, z + along), v3(cm(20), cm(150), cm(110)))
            };
            b.push(
                Mesh::Cube,
                &render::boxed(p, s),
                col,
                if lit { ONE / 3 } else { 0 },
                render::PASS_SOLID,
            );
        }
    }
    // a sign or an air conditioner
    if rng.chance(50) {
        let sign = SIGNS[rng.range(0, SIGNS.len() as i32 - 1) as usize];
        let zz = if z < 0 { z + d / 2 + cm(30) } else { z };
        b.inked(
            Mesh::Cube,
            &render::boxed(v3(x + w / 3, fx::int(5), zz), v3(cm(70), fx::int(4), cm(20))),
            sign,
            ONE / 4,
        );
    }
}

const WINDOW: Rgb = [ONE * 246 / 255, ONE * 231 / 255, ONE * 160 / 255];
const SIGNS: [Rgb; 4] = [
    [ONE * 255 / 255, ONE * 46 / 255, ONE * 136 / 255],
    [ONE * 19 / 255, ONE * 207 / 255, ONE * 227 / 255],
    [ONE, ONE * 176 / 255, 0],
    [ONE * 138 / 255, ONE * 91 / 255, ONE * 214 / 255],
];

fn street_ring(b: &mut Batch, walls: &[Rgb], rng: &mut Rng, base: i32) {
    // a street's worth of buildings behind the stage and down both sides
    let mut x = -fx::int(40);
    let mut k = 0;
    while x < fx::int(40) {
        let w = fx::int(rng.range(5, 9));
        let h = fx::int(rng.range(9, 20));
        let foot = v3(x + w / 2, 0, -fx::int(22));
        building(b, foot, v3(w, h + base, fx::int(8)), walls[k % walls.len()], rng);
        x += w + cm(30);
        k += 1;
    }
    for side in [-1, 1] {
        let mut z = -fx::int(16);
        while z < fx::int(10) {
            let d = fx::int(rng.range(5, 8));
            let h = fx::int(rng.range(8, 16));
            let foot = v3(side * fx::int(24), 0, z + d / 2);
            building(b, foot, v3(fx::int(8), h + base, d), walls[k % walls.len()], rng);
            z += d + cm(30);
            k += 1;
        }
    }
}

fn market(b: &mut Batch, walls: &[Rgb], trim: Rgb, rng: &mut Rng) {
    street_ring(b, walls, rng, 0);
    // stalls with striped awnings along the back
    for k in 0..6 {
        let x = -fx::int(15) + k * fx::int(6);
        let z = -fx::int(15);
        b.inked(
            Mesh::Cube,
            &render::boxed(v3(x, cm(50), z), v3(fx::int(3), ONE, cm(120))),
            WOOD,
            0,
        );
        let c = if k % 2 == 0 { trim } else { [ONE; 3] };
        let a = Affine::translate(v3(x, cm(260), z + cm(60)))
            .then(&Affine::rot_y(fx::HALF))
            .then(&Affine::scale(v3(fx::int(3), cm(50), cm(180))));
        b.inked(Mesh::Wedge, &a, c, 0);
        for f in 0..4 {
            b.inked(
                Mesh::Sphere,
                &render::boxed(
                    v3(x - cm(100) + f * cm(65), cm(115), z),
                    v3(cm(30), cm(30), cm(30)),
                ),
                SIGNS[(f % 4) as usize],
                0,
            );
        }
    }
    // string lights over the stage
    for k in 0..16 {
        let x = -fx::int(12) + k * cm(160);
        let s = share(k, 15);
        let y = fx::int(9) - fx::mul(fx::int(2), bump(s));
        b.glow(
            Mesh::Sphere,
            &render::boxed(v3(x, y, -fx::int(6)), v3(cm(50), cm(50), cm(50))),
            SIGNS[(k % 4) as usize],
            ONE / 2,
        );
        b.push(
            Mesh::Sphere,
            &render::boxed(v3(x, y, -fx::int(6)), v3(cm(16), cm(16), cm(16))),
            SIGNS[(k % 4) as usize],
            ONE,
            render::PASS_SOLID,
        );
    }
    crates(b, rng, 6);
}

fn rooftops(b: &mut Batch, walls: &[Rgb], trim: Rgb, rng: &mut Rng) {
    // the street is below us: buildings round the roof, cut off low
    street_ring(b, walls, rng, -fx::int(6));
    // a parapet round the roof
    for (x, z, w, d) in [
        (0, -fx::int(12), fx::int(36), cm(50)),
        (-fx::int(18), -fx::int(3), cm(50), fx::int(18)),
        (fx::int(18), -fx::int(3), cm(50), fx::int(18)),
    ] {
        b.inked(
            Mesh::Cube,
            &render::boxed(v3(x, cm(50), z), v3(w, ONE, d)),
            CONCRETE,
            0,
        );
    }
    // a water tower and an antenna mast
    let wt = v3(-fx::int(12), 0, -fx::int(9));
    for (dx, dz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        b.inked(
            Mesh::Cube,
            &render::boxed(
                wt.add(v3(dx * cm(90), fx::int(2), dz * cm(90))),
                v3(cm(15), fx::int(4), cm(15)),
            ),
            WOOD,
            0,
        );
    }
    b.inked(
        Mesh::Cyl,
        &render::boxed(
            wt.add(v3(0, fx::int(5), 0)),
            v3(fx::int(3), fx::int(3), fx::int(3)),
        ),
        WOOD,
        0,
    );
    b.inked(
        Mesh::Cone,
        &render::boxed(wt.add(v3(0, cm(750), 0)), v3(cm(340), cm(140), cm(340))),
        shade(WOOD, 75),
        0,
    );
    let mast = v3(fx::int(13), 0, -fx::int(8));
    b.inked(
        Mesh::Cube,
        &render::boxed(mast.add(v3(0, fx::int(5), 0)), v3(cm(20), fx::int(10), cm(20))),
        METAL,
        0,
    );
    for k in 0..3 {
        b.inked(
            Mesh::Cube,
            &render::boxed(
                mast.add(v3(0, fx::int(6) + k * fx::int(1), 0)),
                v3(cm(220) - k * cm(50), cm(10), cm(10)),
            ),
            METAL,
            0,
        );
    }
    b.glow(
        Mesh::Sphere,
        &render::boxed(mast.add(v3(0, fx::int(10), 0)), v3(cm(60), cm(60), cm(60))),
        [ONE, 0, 0],
        ONE / 2,
    );
    // air conditioners
    for (x, z) in [
        (fx::int(9), -fx::int(10)),
        (-fx::int(16), fx::int(4)),
        (fx::int(15), fx::int(3)),
    ] {
        b.inked(
            Mesh::Cube,
            &render::boxed(v3(x, cm(60), z), v3(fx::int(2), cm(120), cm(140))),
            METAL,
            0,
        );
        b.inked(
            Mesh::Cyl,
            &render::boxed(v3(x, cm(125), z), v3(cm(110), cm(10), cm(110))),
            shade(METAL, 70),
            0,
        );
    }
    // laundry across the back
    for k in 0..9 {
        let x = -fx::int(10) + k * cm(250);
        let c = [trim, [ONE; 3], walls[k as usize % walls.len()]][k as usize % 3];
        b.inked(
            Mesh::Cube,
            &render::boxed(v3(x, fx::int(6), -fx::int(11)), v3(cm(90), cm(110), cm(6))),
            c,
            0,
        );
    }
    let _ = rng;
}

fn junkyard(b: &mut Batch, walls: &[Rgb], trim: Rgb, rng: &mut Rng) {
    // a fence
    b.inked(
        Mesh::Cube,
        &render::boxed(
            v3(0, fx::int(2), -fx::int(24)),
            v3(fx::int(80), fx::int(4), cm(20)),
        ),
        METAL,
        0,
    );
    // scrap heaps: mounds of boxes and wedges
    for (cx, cz, r) in [
        (-fx::int(12), -fx::int(14), 6),
        (fx::int(10), -fx::int(16), 7),
        (-fx::int(20), fx::int(2), 4),
        (fx::int(20), -fx::int(2), 5),
    ] {
        b.inked(
            Mesh::Sphere,
            &render::boxed(v3(cx, 0, cz), v3(fx::int(r * 2), fx::int(r), fx::int(r * 2))),
            shade(METAL, 85),
            0,
        );
        for _ in 0..14 {
            let a = fx::TURN * rng.range(0, 360) / 360;
            let d = fx::mul(fx::int(r), ONE * rng.range(20, 95) / 100);
            let p = v3(
                cx + fx::mul(fx::cos(a), d),
                fx::int(r) / 3 + ONE * rng.range(0, 140) / 100,
                cz + fx::mul(fx::sin(a), d),
            );
            let c = walls[rng.range(0, walls.len() as i32 - 1) as usize];
            let turn = Affine::rot_y(a).then(&Affine::rot_x(deg(rng.range(-40, 40))));
            let s = v3(
                ONE * rng.range(60, 200) / 100,
                ONE * rng.range(40, 120) / 100,
                ONE * rng.range(60, 200) / 100,
            );
            let mesh = if rng.chance(50) { Mesh::Cube } else { Mesh::Wedge };
            b.inked(
                mesh,
                &Affine::translate(p).then(&turn).then(&Affine::scale(s)),
                c,
                0,
            );
        }
    }
    // tyre stacks
    for (x, z, n) in [
        (-fx::int(9), fx::int(7), 4),
        (fx::int(10), fx::int(7), 3),
        (fx::int(15), -fx::int(8), 5),
    ] {
        for k in 0..n {
            b.inked(
                Mesh::Cyl,
                &render::boxed(v3(x, cm(25) + k * cm(45), z), v3(cm(160), cm(42), cm(160))),
                TYRE,
                0,
            );
        }
    }
    // the crane
    let base = v3(fx::int(16), 0, -fx::int(12));
    b.inked(
        Mesh::Cube,
        &render::boxed(base.add(v3(0, fx::int(8), 0)), v3(cm(80), fx::int(16), cm(80))),
        trim,
        0,
    );
    b.inked(
        Mesh::Cube,
        &render::boxed(
            base.add(v3(-fx::int(5), fx::int(16), 0)),
            v3(fx::int(11), cm(60), cm(60)),
        ),
        trim,
        0,
    );
    b.inked(
        Mesh::Cyl,
        &render::boxed(
            base.add(v3(-fx::int(10), fx::int(11), 0)),
            v3(fx::int(2), cm(40), fx::int(2)),
        ),
        METAL,
        0,
    );
    // the mech cat, asleep on the far heap
    let pose = Pose {
        down: ONE / 2,
        ..Pose::default()
    };
    let look = Look {
        body: crate::content::Body::Cat,
        skin: "#6FB07A".into(),
        top: "#4E8A5E".into(),
        legs: "#4E8A5E".into(),
        hair: "#A8D49A".into(),
        hat: crate::content::Hat::None,
        prop: crate::content::Prop::None,
        accent: "#F2803A".into(),
        size: 400,
    };
    figure::draw(
        b,
        &look,
        Kind::Melee,
        v3(fx::int(10), fx::int(4), -fx::int(17)),
        deg(200),
        &pose,
    );
}

fn crates(b: &mut Batch, rng: &mut Rng, n: i32) {
    for _ in 0..n {
        let x = fx::int(rng.range(-16, 16));
        let z = fx::int(rng.range(6, 10)) * if rng.chance(50) { 1 } else { -1 };
        if x.abs() < fx::int(9) && z.abs() < fx::int(7) {
            continue;
        }
        let s = cm(rng.range(60, 110));
        b.inked(
            Mesh::Cube,
            &Affine::translate(v3(x, s / 2, z))
                .then(&Affine::rot_y(deg(rng.range(0, 90))))
                .then(&Affine::scale(v3(s, s, s))),
            WOOD,
            0,
        );
    }
}

const WOOD: Rgb = [ONE * 185 / 255, ONE * 131 / 255, ONE * 90 / 255];
const METAL: Rgb = [ONE * 141 / 255, ONE * 154 / 255, ONE * 147 / 255];
const CONCRETE: Rgb = [ONE * 216 / 255, ONE * 195 / 255, ONE * 164 / 255];
const TYRE: Rgb = [ONE * 70 / 255, ONE * 58 / 255, ONE * 72 / 255];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::fight;
    use crate::content::fixtures::content;

    fn show(seed: u64) -> Show {
        let c = content();
        let u = |id: &str, l: u32, k: usize| (c.unit_index(id).unwrap(), l, k);
        let r = fight(
            &c,
            &[u("rook", 6, 1), u("vela", 6, 1), u("pip", 6, 1)],
            &[u("nan", 6, 1), u("june", 6, 1), u("chum", 6, 0)],
            seed,
        );
        Show::new(&c, r, 0)
    }

    #[test]
    fn turns_follow_one_another_and_land_inside_themselves() {
        let s = show(1);
        let mut last = 0;
        for b in &s.beats {
            assert!(b.start >= last && b.start < b.hit && b.hit < b.end, "{b:?}");
            last = b.end;
        }
        assert!(s.total() > last);
    }

    #[test]
    fn each_kind_moves_in_its_own_way() {
        let c = content();
        let s = show(2);
        for (i, t) in s.record.turns.iter().enumerate() {
            let id = &s.record.teams[t.side][t.slot].id;
            let kind = c.units[c.unit_index(id).unwrap()].kind;
            let style = s.beats[i].style;
            match t.act {
                Act::Stunned => assert_eq!(style, Style::Dazed),
                Act::Attack => assert_eq!(
                    style,
                    match kind {
                        Kind::Melee => Style::Charge,
                        Kind::Ranged => Style::Shoot,
                        Kind::Magic => Style::Spell,
                    }
                ),
                Act::Skill => {}
            }
        }
    }

    #[test]
    fn a_charging_unit_is_next_to_its_target_when_the_blow_lands() {
        let s = show(3);
        let i = s
            .beats
            .iter()
            .position(|b| b.style == Style::Charge)
            .expect("someone charges");
        let t = &s.record.turns[i];
        let (ts, tk) = s.target_of(i);
        let at_hit = s.frame(s.beats[i].hit, ONE);
        let at_start = s.frame(s.beats[i].start, ONE);
        // the attacker's head mark has moved toward the target's
        let k = |side: usize, slot: usize| (side * 3 + slot) * 2;
        let d = |f: &Frame, a: usize, b: usize| (f.marks[a] - f.marks[b]).abs();
        assert!(d(&at_hit, k(t.side, t.slot), k(ts, tk)) < d(&at_start, k(t.side, t.slot), k(ts, tk)));
    }

    #[test]
    fn a_frame_draws_everyone_and_marks_six_heads_on_screen() {
        let s = show(4);
        for t in [0, s.total() / 3, s.total()] {
            let f = s.frame(t, ONE * 16 / 9);
            assert_eq!(f.marks.len(), 12);
            for m in &f.marks {
                assert!((0..=100 * ONE).contains(m), "{m}");
            }
            assert!(f.values.len() / render::INSTANCE_VALUES > 150);
            assert_eq!(f.draws.len() % render::DRAW_VALUES, 0);
        }
    }

    #[test]
    fn every_standing_unit_wears_its_own_sides_colour_and_never_the_other() {
        let s = show(4);
        for t in [0, s.total() / 2] {
            let f = s.frame(t, ONE * 16 / 9);
            let (_, states) = s.state_at(t);
            for side in 0..2 {
                let worn: Vec<V3> = f
                    .values
                    .chunks(render::INSTANCE_VALUES)
                    .filter(|v| v[12..15] == TEAM[side])
                    .map(|v| v3(v[3], v[7], v[11]))
                    .collect();
                for (slot, st) in states[side].iter().enumerate() {
                    let at = s.motion(side, slot, t).at;
                    let near = |p: &V3| v3(p.x - at.x, 0, p.z - at.z).dist(v3(0, 0, 0)) < cm(20);
                    let n = worn.iter().filter(|p| near(p)).count();
                    if st.alive {
                        assert_eq!(n, 3, "side {side} slot {slot} at {t}");
                    }
                    let other = s.motion(1 - side, slot, t).at;
                    assert!(
                        !worn
                            .iter()
                            .any(|p| v3(p.x - other.x, 0, p.z - other.z).dist(v3(0, 0, 0)) < cm(20)),
                        "side {side} colour on the other side's slot {slot}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_fallen_are_lower_at_the_end_than_at_the_start() {
        let s = show(5);
        let end = s.frame(s.total(), ONE);
        let start = s.frame(0, ONE);
        let last = s.record.turns.last().unwrap();
        for side in 0..2 {
            for slot in 0..3 {
                if !last.after[side][slot].alive {
                    // a lower head is further down the screen
                    let k = (side * 3 + slot) * 2 + 1;
                    assert!(end.marks[k] > start.marks[k], "side {side} slot {slot}");
                }
            }
        }
    }

    /// Every unit's place, sampled through the whole fight.
    fn walk(s: &Show, step: usize) -> Vec<[Vec<V3>; 2]> {
        (0..=s.total())
            .step_by(step)
            .map(|t| {
                [
                    (0..3).map(|k| s.motion(0, k, t).at).collect(),
                    (0..3).map(|k| s.motion(1, k, t).at).collect(),
                ]
            })
            .collect()
    }

    #[test]
    fn nobody_jumps_and_everyone_keeps_to_the_floor() {
        for seed in 0..6 {
            let s = show(seed);
            let w = walk(&s, 16);
            for pair in w.windows(2) {
                for (was, now) in pair[0].iter().flatten().zip(pair[1].iter().flatten()) {
                    {
                        let (a, b) = (*was, *now);
                        assert!(a.dist(b) < cm(60), "seed {seed}: {a:?} to {b:?}");
                        assert!(b.x.abs() <= FLOOR_X && b.z.abs() <= FLOOR_Z, "{b:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn units_keep_moving_between_turns_and_never_stand_in_each_other() {
        let s = show(7);
        let last = s.record.turns.last().unwrap();
        for side in 0..2 {
            for k in 0..3 {
                let mut places: Vec<V3> = s.spots.iter().map(|sp| sp[side][k]).collect();
                places.dedup();
                if last.after[side][k].alive {
                    assert!(
                        places.len() > s.record.turns.len() / 3,
                        "side {side} slot {k}: {}",
                        places.len()
                    );
                }
            }
        }
        for sp in &s.spots {
            let all: Vec<V3> = sp.iter().flatten().copied().collect();
            for (a, p) in all.iter().enumerate() {
                for q in &all[a + 1..] {
                    assert!(p.dist(*q) >= APART - cm(5), "{p:?} {q:?}");
                }
            }
        }
    }

    #[test]
    fn a_flank_strikes_from_the_side() {
        let mut seen = 0;
        for seed in 0..10 {
            let s = show(seed);
            for (i, bt) in s
                .beats
                .iter()
                .enumerate()
                .filter(|(_, b)| b.style == Style::Flank)
            {
                let tn = &s.record.turns[i];
                let aim = s.aim_of(i);
                let p0 = s.spots[i][tn.side][tn.slot];
                let at = s.motion(tn.side, tn.slot, bt.hit).at;
                let (a, b) = (
                    v3(at.x - aim.x, 0, at.z - aim.z).norm(),
                    v3(p0.x - aim.x, 0, p0.z - aim.z).norm(),
                );
                // at least 50 degrees round from where it started
                assert!(a.dot(b) < ONE * 64 / 100, "seed {seed} turn {i}: {}", a.dot(b));
                seen += 1;
            }
        }
        assert!(seen > 5);
    }

    #[test]
    fn a_dodged_blow_finds_its_target_stepped_aside() {
        let mut seen = 0;
        for seed in 0..20 {
            let s = show(seed);
            for (i, bt) in s.beats.iter().enumerate() {
                for h in s.record.turns[i].hits.iter().filter(|h| h.missed) {
                    let spot = s.spots[i][h.side][h.slot];
                    assert!(s.motion(h.side, h.slot, bt.hit).at.dist(spot) > cm(100));
                    let after = (bt.hit + HOP_MS / 2 + 1).min(bt.end - 1);
                    assert!(s.motion(h.side, h.slot, after).at.dist(spot) < cm(5));
                    seen += 1;
                }
            }
        }
        assert!(seen > 3, "{seen}");
    }

    #[test]
    fn a_dodge_rolls_clear_of_where_it_stood() {
        let s = show(2);
        let mut seen = 0;
        for (i, _) in s
            .beats
            .iter()
            .enumerate()
            .filter(|(_, b)| b.style == Style::Dodge)
        {
            let tn = &s.record.turns[i];
            let (p0, p1) = (s.spots[i][tn.side][tn.slot], s.spots[i + 1][tn.side][tn.slot]);
            assert!(p0.dist(p1) > cm(100), "{p0:?} {p1:?}");
            seen += 1;
        }
        assert!(seen > 0);
    }

    #[test]
    fn the_scene_has_buildings_and_each_chapter_its_own() {
        let c = content();
        let r = show(1).record;
        let counts: Vec<usize> = (0..3)
            .map(|ch| Show::new(&c, r.clone(), ch).scene().0.len())
            .collect();
        assert!(counts.iter().all(|n| *n > 100 * render::INSTANCE_VALUES));
        assert!(counts[0] != counts[1] && counts[1] != counts[2]);
    }
}
