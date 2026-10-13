//! The fight in 3D. A fight is already decided (`battle::Record`); this
//! module says, for any moment of its playback, where everyone stands and
//! what flies between them, and where the camera looks. Melee units run in
//! and swing, ranged units shoot, magic users raise an orb and throw it,
//! healers light the ones they mend, hits knock their targets back, and the
//! fallen fall. The page asks for a moment and draws what comes back.
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
    /// Throw or fire from where it stands.
    Shoot,
    /// Raise an orb and hurl it.
    Spell,
    /// Light up allies from where it stands.
    Support,
    /// Shake off a stun.
    Dazed,
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
    }
}

/// A pause before the first turn, so the stage is seen before anyone moves.
const OPENING_MS: i32 = 700;
/// How long a knocked-back target takes to recover, a fallen one to fall,
/// a burst to fade.
const RECOIL_MS: i32 = 350;
const FALL_MS: i32 = 450;
const FLASH_MS: i32 = 160;
const BURST_MS: i32 = 450;
const RING_MS: i32 = 700;

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

/// A fight on the stage: the record, the looks of the six, and the place.
#[derive(Debug, Clone)]
pub struct Show {
    pub record: Record,
    pub beats: Vec<Beat>,
    looks: [Vec<Look>; 2],
    kinds: [Vec<Kind>; 2],
    palette: Palette,
    backdrop: String,
}

/// Where a unit stands between turns. Side 0 on the left (−x), side 1 on
/// the right; slot 0 at the back, slot 2 at the front; the middle slot a
/// step nearer the middle, as the tiles beside the stage are.
pub fn home(side: usize, slot: usize) -> V3 {
    let x = if slot == 1 { cm(330) } else { cm(420) };
    let z = cm(230) * (slot as i32 - 1);
    v3(if side == 0 { -x } else { x }, 0, z)
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
        Show {
            looks: [
                record.teams[0].iter().map(|c| look(&c.id)).collect(),
                record.teams[1].iter().map(|c| look(&c.id)).collect(),
            ],
            kinds: [
                record.teams[0].iter().map(|c| kind(&c.id)).collect(),
                record.teams[1].iter().map(|c| kind(&c.id)).collect(),
            ],
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
            &render::boxed(v3(0, cm(3), 0), v3(fx::int(13), cm(6), fx::int(10))),
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

    pub fn frame(&self, t: i32, aspect: i32) -> Frame {
        let mut b = Batch::new(INK);
        let (cur, states) = self.state_at(t);
        let fell = self.fell_at(t);
        let beat = self.beats.get(cur).copied();
        let turn = self.record.turns.get(cur);
        let active = beat.map(|bt| t >= bt.start && t < bt.end).unwrap_or(false);
        let f = beat.map(|bt| share(t - bt.start, bt.end - bt.start)).unwrap_or(0);
        let hit_share = beat
            .map(|bt| share(bt.hit - bt.start, bt.end - bt.start))
            .unwrap_or(ONE / 2);
        let (tside, tslot) = if turn.is_some() {
            self.target_of(cur)
        } else {
            (1, 0)
        };
        let mut heads = Vec::new();
        let mut focus = v3(0, ONE, 0);
        let mut focus_w = 0;
        let mut actor_side_sign = 0;
        for side in 0..2 {
            for slot in 0..self.looks[side].len() {
                let look = &self.looks[side][slot];
                let kind = self.kinds[side][slot];
                let me = home(side, slot);
                let foe_home = home(1 - side, slot.min(self.looks[1 - side].len().saturating_sub(1)));
                let mut at = me;
                let mut yaw = facing_yaw(me, v3(foe_home.x, 0, me.z));
                let mut pose = Pose {
                    breath: fx::TURN * ((t + 700 * (side * 3 + slot) as i32) % 2400) / 2400,
                    ..Pose::default()
                };
                if let Some(when) = fell[side][slot] {
                    pose.down = ease(share(t - when, FALL_MS));
                    pose.breath = 0;
                }
                let acting = active && turn.map(|x| x.side == side && x.slot == slot).unwrap_or(false);
                if acting {
                    let bt = beat.unwrap_or(Beat {
                        start: 0,
                        hit: 0,
                        end: 1,
                        style: Style::Dazed,
                    });
                    let target = home(tside, tslot);
                    actor_side_sign = if side == 0 { -1 } else { 1 };
                    let sweep = turn
                        .map(|x| {
                            x.hits
                                .iter()
                                .filter(|h| h.side != side && !h.heal && !h.hasted)
                                .count()
                                > 1
                        })
                        .unwrap_or(false);
                    match bt.style {
                        Style::Charge => {
                            // a spot just in front of the target (the middle of
                            // their line for a sweep)
                            let aim = if sweep { v3(home(tside, 1).x, 0, 0) } else { target };
                            let toward = me.sub(aim);
                            let gap = if look.size > 140 { cm(220) } else { cm(130) };
                            let spot = aim.add(v3(toward.x, 0, toward.z).norm().scale(gap));
                            let run = hit_share * 7 / 10;
                            let back = hit_share + (ONE - hit_share) / 3;
                            if f < run {
                                let s = ease(share(f, run));
                                at = me.lerp(spot, s);
                                pose.moving = ONE;
                                yaw = facing_yaw(me, spot);
                            } else if f < back {
                                at = spot;
                                pose.swing = share(f - run, back - run);
                                yaw = facing_yaw(spot, aim);
                            } else {
                                let s = ease(share(f - back, ONE - back));
                                at = spot.lerp(me, s);
                                pose.moving = ONE - s / 2;
                                yaw = if s < ONE * 9 / 10 {
                                    facing_yaw(spot, me)
                                } else {
                                    yaw
                                };
                            }
                            pose.step = fx::TURN * (t % 500) / 500;
                        }
                        Style::Shoot => {
                            yaw = facing_yaw(me, target);
                            pose.swing = share(f - hit_share / 2, hit_share);
                        }
                        Style::Spell | Style::Support => {
                            yaw = facing_yaw(
                                me,
                                if bt.style == Style::Support {
                                    v3(me.x, 0, me.z - ONE)
                                } else {
                                    target
                                },
                            );
                            let up = if f < hit_share {
                                share(f, hit_share * 2 / 3)
                            } else {
                                ONE - share(f - hit_share, ONE - hit_share)
                            };
                            pose.cast = ease(up);
                        }
                        Style::Dazed => {
                            yaw += fx::mul(deg(18), fx::sin(fx::TURN * (t % 300) / 300));
                        }
                    }
                    focus = at.lerp(target, ONE / 2);
                    focus_w = bump(f);
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
                        if !h.heal && !h.hasted {
                            pose.recoil = pose.recoil.max(
                                bump(share(dt, RECOIL_MS)) * if h.crit { 2 } else { 1 }
                                    / if h.edge < 0 { 2 } else { 1 },
                            );
                            pose.flash = pose.flash.max(ONE - share(dt, FLASH_MS));
                        }
                    }
                }
                figure::draw(&mut b, look, kind, at, yaw, &pose);
                let st = &states[side][slot];
                let standing = figure::height(look) + cm(30);
                let top = at.add(v3(0, standing - fx::mul(standing * 4 / 5, pose.down), 0));
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
                // a ready skill glows at the feet
                if st.ready && st.alive {
                    let pulse = ONE / 4 + fx::mul(ONE / 6, fx::sin(fx::TURN * (t % 1000) / 1000));
                    b.glow(
                        Mesh::Cyl,
                        &render::boxed(me.add(v3(0, cm(8), 0)), v3(cm(150), cm(4), cm(150))),
                        hex(&look.accent),
                        pulse,
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
            let accent = hex(&look.accent);
            let me = home(tn.side, tn.slot);
            let yaw = facing_yaw(me, home(1 - tn.side, 1));
            let (hand, _) = figure::anchors(look, me, yaw);
            let f = share(t - bt.start, bt.end - bt.start);
            let hs = share(bt.hit - bt.start, bt.end - bt.start);
            let mut seen: Vec<(usize, usize)> = Vec::new();
            for h in &tn.hits {
                if seen.contains(&(h.side, h.slot)) && !h.heal {
                    continue;
                }
                seen.push((h.side, h.slot));
                let tlook = &self.looks[h.side][h.slot];
                let thome = home(h.side, h.slot);
                let (_, chest) = figure::anchors(tlook, thome, 0);
                let hostile = h.side != tn.side;
                if hostile && t < bt.hit {
                    match bt.style {
                        Style::Shoot => {
                            let from = hs / 2;
                            if f >= from {
                                let s = share(f - from, hs - from);
                                let arc = fx::mul(cm(120), bump(s));
                                let p = hand.lerp(chest, s).add(v3(0, arc, 0));
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
                        }
                        Style::Spell => {
                            let lift = me.add(v3(0, figure::height(look) + cm(70), 0));
                            let grow = hs * 2 / 3;
                            let (p, r) = if f < grow {
                                (lift, fx::mul(cm(45), share(f, grow)))
                            } else {
                                let s = ease(share(f - grow, hs - grow));
                                (lift.lerp(chest, s), cm(45))
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
                    let at_floor = v3(thome.x, cm(10), thome.z);
                    if hostile && dt < BURST_MS {
                        let s = share(dt, BURST_MS);
                        let r = fx::mul(cm(if h.crit { 160 } else { 100 }), ease(s));
                        let c = if h.edge > 0 {
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
        let centre = v3(0, cm(60), cm(80));
        let look_at = centre.lerp(focus.add(v3(0, cm(60), 0)), focus_w / 2);
        let dist = fx::int(13) - fx::mul(cm(250), focus_w);
        let height = cm(640);
        let side_lean = fx::mul(cm(150), focus_w) * actor_side_sign;
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
            &[u("rook", 6, 1), u("vela", 6, 1), u("pip", 6, 0)],
            &[u("nan", 6, 1), u("june", 6, 0), u("chum", 6, 0)],
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
