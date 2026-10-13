//! A unit in 3D: built from boxes, spheres, cones and cylinders by its
//! `look` in data/units.json, and posed by the stage (crates/core/src/stage.rs).
//! Every figure is modelled facing −z with its feet at the origin, a grown
//! person 1.7 m tall; `look.size` scales it.

use crate::content::{hex, Body, Hat, Kind, Look, Prop, Rgb};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{shade, Batch};

/// How a figure stands this frame. Every share is `0..=ONE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pose {
    /// The walk cycle's angle, and how much of a walk is in the legs.
    pub step: i32,
    pub moving: i32,
    /// The weapon arm going up and coming down.
    pub swing: i32,
    /// Both arms raised to cast.
    pub cast: i32,
    /// Knocked back by a hit.
    pub recoil: i32,
    /// Fallen over.
    pub down: i32,
    /// The idle breath, an angle.
    pub breath: i32,
    /// A white flash as a hit lands.
    pub flash: i32,
}

/// Figures are drawn a little larger than life so they read on the stage.
const STAGE_SCALE: i32 = 122;

fn scale_of(look: &Look) -> i32 {
    (look.size as i64 * STAGE_SCALE as i64 * ONE as i64 / 10_000) as i32
}

/// Hundredths of a metre.
pub const fn cm(n: i32) -> i32 {
    n * ONE / 100
}

fn c3(x: i32, y: i32, z: i32) -> V3 {
    v3(cm(x), cm(y), cm(z))
}

const INK: Rgb = [ONE * 59 / 255, ONE * 35 / 255, ONE * 64 / 255];
const WHITE: Rgb = [ONE, ONE, ONE];
const METAL: Rgb = [ONE * 141 / 255, ONE * 154 / 255, ONE * 147 / 255];
const GOLD: Rgb = [ONE, ONE * 176 / 255, 0];
const RED: Rgb = [ONE * 224 / 255, ONE * 72 / 255, ONE * 62 / 255];

/// Draws parts relative to one base transform.
struct Rig<'a> {
    b: &'a mut Batch,
    base: Affine,
    flash: i32,
}

impl<'a> Rig<'a> {
    fn colour(&self, c: Rgb) -> Rgb {
        crate::render::mix(c, WHITE, self.flash)
    }

    /// A shape of `size` centred at `at` (local), turned by `turn`.
    fn part(&mut self, mesh: Mesh, at: V3, size: V3, turn: &Affine, c: Rgb) {
        let a = self
            .base
            .then(&Affine::translate(at))
            .then(turn)
            .then(&Affine::scale(size));
        let c = self.colour(c);
        self.b.inked(mesh, &a, c, 0);
    }

    fn put(&mut self, mesh: Mesh, at: V3, size: V3, c: Rgb) {
        self.part(mesh, at, size, &Affine::IDENTITY, c);
    }

    /// A limb hanging from `pivot`, `len` long, turned by `turn` about the
    /// pivot: a box whose top sits at the pivot.
    fn limb(&mut self, pivot: V3, len: i32, w: i32, turn: &Affine, c: Rgb) -> V3 {
        let a = self
            .base
            .then(&Affine::translate(pivot))
            .then(turn)
            .then(&Affine::translate(v3(0, -len / 2, 0)))
            .then(&Affine::scale(v3(w, len, w)));
        let col = self.colour(c);
        self.b.inked(Mesh::Cube, &a, col, 0);
        // where the limb ends, in local space
        pivot.add(turn.apply(v3(0, -len, 0)))
    }

    fn glow(&mut self, at: V3, r: i32, c: Rgb, strength: i32) {
        let a = self
            .base
            .then(&Affine::translate(at))
            .then(&Affine::scale(v3(r, r, r)));
        self.b.push(Mesh::Sphere, &a, c, ONE, crate::render::PASS_SOLID);
        let halo = self
            .base
            .then(&Affine::translate(at))
            .then(&Affine::scale(v3(r * 3, r * 3, r * 3)));
        self.b.glow(Mesh::Sphere, &halo, c, strength);
    }
}

/// Where a figure's hand and chest are, for spells and arrows to leave from
/// and arrive at: (hand, chest), in world space.
pub fn anchors(look: &Look, at: V3, yaw: i32) -> (V3, V3) {
    let s = scale_of(look);
    let base = Affine::translate(at)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::scale(v3(s, s, s)));
    let (hand, chest) = match look.body {
        Body::Cat => (c3(0, 70, -70), c3(0, 75, 0)),
        Body::Dog => (c3(0, 45, -45), c3(0, 45, 0)),
        _ => (c3(30, 120, -35), c3(0, 115, 0)),
    };
    (base.apply(hand), base.apply(chest))
}

/// Height of the top of the head, in world metres.
pub fn height(look: &Look) -> i32 {
    let h = match look.body {
        Body::Cat => cm(150),
        Body::Dog => cm(95),
        _ => cm(195),
    };
    fx::mul(h, scale_of(look))
}

pub fn draw(b: &mut Batch, look: &Look, kind: Kind, at: V3, yaw: i32, pose: &Pose) {
    let s = scale_of(look);
    // fall over backwards about the heels, and rock back on a hit
    let tip = -fx::mul(deg(85), pose.down) - fx::mul(deg(14), pose.recoil);
    let lift = fx::mul(cm(4), fx::sin(pose.breath));
    let base = Affine::translate(at.add(v3(0, lift, 0)))
        .then(&Affine::rot_y(yaw))
        .then(&Affine::translate(c3(0, 0, 25)))
        .then(&Affine::rot_x(tip))
        .then(&Affine::translate(c3(0, 0, -25)))
        .then(&Affine::scale(v3(s, s, s)));
    let mut r = Rig {
        b,
        base,
        flash: pose.flash,
    };
    match look.body {
        Body::Person | Body::Raccoon => person(&mut r, look, kind, pose),
        Body::Cat => cat(&mut r, look, pose),
        Body::Dog => dog(&mut r, look, pose),
    }
}

fn person(r: &mut Rig, look: &Look, kind: Kind, pose: &Pose) {
    let (skin, top, legs, hair, accent) = (
        hex(&look.skin),
        hex(&look.top),
        hex(&look.legs),
        hex(&look.hair),
        hex(&look.accent),
    );
    let swing_leg = fx::mul(fx::mul(fx::sin(pose.step), deg(32)), pose.moving);
    // legs and shoes
    for (side, a) in [(-1, swing_leg), (1, -swing_leg)] {
        let foot = r.limb(c3(side * 13, 84, 0), cm(80), cm(21), &Affine::rot_x(a), legs);
        r.put(
            Mesh::Cube,
            foot.add(c3(0, 4, -5)),
            c3(19, 10, 28),
            shade(legs, 60),
        );
    }
    // body, leaning into a run
    let lean = Affine::rot_x(-fx::mul(deg(12), pose.moving));
    r.part(Mesh::Cube, c3(0, 114, 0), c3(58, 64, 36), &lean, top);
    r.put(Mesh::Cube, c3(0, 86, 0), c3(54, 10, 32), shade(top, 70));
    if look.body == Body::Raccoon {
        // a striped tail
        for k in 0..4 {
            let c = if k % 2 == 0 { skin } else { INK };
            r.put(
                Mesh::Sphere,
                c3(12, 80 + k * 6, 22 + k * 10),
                c3(20 - k * 2, 20 - k * 2, 20 - k * 2),
                c,
            );
        }
    }
    // arms: the weapon arm (right, +x) swings or casts, the other balances
    let up = deg(150);
    let swing = if pose.swing < ONE / 2 {
        -fx::mul(up, pose.swing * 2)
    } else {
        -fx::mul(up, (ONE - pose.swing) * 2) + fx::mul(deg(40), (pose.swing - ONE / 2) * 2)
    };
    let walk_arm = -fx::mul(fx::mul(fx::sin(pose.step), deg(30)), pose.moving);
    let cast = fx::mul(deg(160), pose.cast);
    let right = Affine::rot_x(swing + walk_arm - cast).then(&Affine::rot_z(fx::mul(deg(12), pose.cast)));
    let left = Affine::rot_x(-walk_arm - cast).then(&Affine::rot_z(-fx::mul(deg(12), pose.cast) - deg(6)));
    let rh = r.limb(c3(36, 140, 0), cm(58), cm(17), &right, top);
    let lh = r.limb(c3(-36, 140, 0), cm(58), cm(17), &left, top);
    let fist = if look.prop == Prop::Fists { 16 } else { 11 };
    r.put(Mesh::Sphere, rh, c3(fist, fist, fist), skin);
    r.put(Mesh::Sphere, lh, c3(fist, fist, fist), skin);
    // head
    r.put(Mesh::Cube, c3(0, 150, 0), c3(16, 14, 16), skin);
    r.put(Mesh::Sphere, c3(0, 172, 0), c3(50, 52, 48), skin);
    if look.body == Body::Raccoon {
        r.put(Mesh::Cube, c3(0, 176, -17), c3(40, 11, 10), INK);
        r.put(Mesh::Sphere, c3(-17, 196, 0), c3(14, 14, 8), skin);
        r.put(Mesh::Sphere, c3(17, 196, 0), c3(14, 14, 8), skin);
        r.put(Mesh::Sphere, c3(0, 166, -21), c3(9, 7, 7), INK);
    } else {
        r.put(Mesh::Sphere, c3(0, 184, 3), c3(53, 34, 52), hair);
    }
    for x in [-10, 10] {
        r.put(Mesh::Sphere, c3(x, 175, -23), c3(7, 9, 4), INK);
    }
    hat(r, look.hat, hair, accent);
    prop(r, look.prop, kind, accent, skin, rh, lh, pose);
}

fn hat(r: &mut Rig, hat: Hat, hair: Rgb, accent: Rgb) {
    match hat {
        Hat::None => {}
        Hat::Flat => r.put(Mesh::Cube, c3(0, 196, 0), c3(46, 10, 44), hair),
        Hat::Witch => {
            r.put(Mesh::Cyl, c3(0, 192, 0), c3(80, 4, 80), shade(accent, 45));
            r.put(Mesh::Cone, c3(0, 222, 0), c3(36, 58, 36), shade(accent, 45));
            r.put(Mesh::Cyl, c3(0, 197, 0), c3(38, 7, 38), accent);
        }
        Hat::Cap => {
            r.put(Mesh::Sphere, c3(0, 190, 0), c3(46, 26, 46), accent);
            r.put(Mesh::Cube, c3(0, 186, 24), c3(30, 3, 22), shade(accent, 70));
        }
        Hat::Bun => r.put(Mesh::Sphere, c3(0, 200, 6), c3(22, 22, 22), hair),
        Hat::Visor => {
            r.put(Mesh::Cube, c3(9, 176, -21), c3(30, 9, 6), shade(accent, 70));
            r.glow(
                c3(13, 176, -24),
                cm(3),
                [0, ONE * 207 / 255, ONE * 227 / 255],
                ONE / 2,
            );
        }
        Hat::Antenna => {
            for x in [-1, 1] {
                let tip = r.limb(
                    c3(x * 8, 192, 0),
                    cm(34),
                    cm(3),
                    &Affine::rot_z(x * deg(150)),
                    METAL,
                );
                r.put(Mesh::Sphere, tip, c3(8, 8, 8), RED);
            }
        }
        Hat::Goggles => {
            r.put(Mesh::Cube, c3(0, 190, 0), c3(48, 6, 46), shade(accent, 50));
            for x in [-10, 10] {
                r.put(Mesh::Cyl, c3(x, 191, -21), c3(14, 14, 6), [ONE / 2, ONE, ONE]);
            }
        }
        Hat::Chimney => {
            r.put(
                Mesh::Cube,
                c3(0, 200, 0),
                c3(46, 8, 46),
                [ONE * 217 / 255, ONE * 118 / 255, ONE * 95 / 255],
            );
            r.put(
                Mesh::Cube,
                c3(0, 222, 0),
                c3(34, 38, 34),
                [ONE * 217 / 255, ONE * 118 / 255, ONE * 95 / 255],
            );
            r.put(Mesh::Sphere, c3(4, 252, 0), c3(20, 16, 20), METAL);
        }
        Hat::Headset => {
            r.put(Mesh::Cube, c3(0, 198, 0), c3(50, 5, 8), shade(METAL, 60));
            for x in [-24, 24] {
                r.put(Mesh::Cyl, c3(x, 174, 0), c3(9, 16, 16), shade(METAL, 60));
            }
            r.put(Mesh::Sphere, c3(0, 182, 3), c3(46, 30, 44), hair);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn prop(r: &mut Rig, prop: Prop, kind: Kind, accent: Rgb, skin: Rgb, rh: V3, lh: V3, pose: &Pose) {
    match prop {
        Prop::None => {}
        Prop::Fists => {
            r.put(Mesh::Cube, rh.add(c3(0, -4, 0)), c3(20, 8, 20), accent);
            r.put(Mesh::Cube, lh.add(c3(0, -4, 0)), c3(20, 8, 20), accent);
        }
        Prop::Orb => {
            let strength = ONE / 3 + fx::mul(ONE / 2, pose.cast);
            r.glow(rh.add(c3(0, -8, -6)), cm(9), accent, strength);
        }
        Prop::Slingshot => {
            r.put(
                Mesh::Cube,
                lh.add(c3(0, 12, 0)),
                c3(5, 22, 5),
                [ONE * 185 / 255, ONE * 131 / 255, ONE * 90 / 255],
            );
            r.put(
                Mesh::Cube,
                lh.add(c3(0, 26, 0)),
                c3(22, 5, 5),
                [ONE * 185 / 255, ONE * 131 / 255, ONE * 90 / 255],
            );
        }
        Prop::Lantern => {
            r.put(Mesh::Cube, lh.add(c3(0, -6, 0)), c3(3, 14, 3), INK);
            r.glow(lh.add(c3(0, -26, 0)), cm(13), accent, ONE / 2);
        }
        Prop::Pistons => {
            r.put(Mesh::Cyl, rh.add(c3(0, 16, 0)), c3(14, 26, 14), accent);
            r.put(Mesh::Cyl, lh.add(c3(0, 16, 0)), c3(14, 26, 14), accent);
            r.put(Mesh::Cube, c3(0, 140, 18), c3(60, 12, 10), shade(accent, 70));
        }
        Prop::Jet => {
            for x in [-15, 15] {
                r.put(Mesh::Cyl, c3(x, 128, 24), c3(16, 44, 16), METAL);
                r.glow(
                    c3(x, 98, 24),
                    cm(7),
                    [ONE, ONE * 128 / 255, ONE * 58 / 255],
                    ONE / 2,
                );
            }
        }
        Prop::Drone => {
            let bob = fx::mul(cm(8), fx::sin(pose.breath * 3));
            let d = c3(40, 220, 10).add(v3(0, bob, 0));
            r.put(Mesh::Cube, d, c3(24, 8, 24), accent);
            for (x, z) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                r.put(Mesh::Cyl, d.add(c3(x * 16, 5, z * 16)), c3(18, 2, 18), WHITE);
            }
        }
        Prop::Spray => {
            r.put(Mesh::Cyl, rh.add(c3(0, 12, 0)), c3(12, 24, 12), accent);
            r.put(Mesh::Cyl, rh.add(c3(0, 27, 0)), c3(6, 6, 6), WHITE);
        }
    }
    // magic users carry a faint glow in the off hand whatever they hold
    if kind == Kind::Magic && prop != Prop::Orb {
        r.glow(
            lh.add(c3(0, 8, 0)),
            cm(5),
            [ONE * 205 / 255, ONE * 182 / 255, ONE * 240 / 255],
            ONE / 4,
        );
    }
    let _ = skin;
}

fn cat(r: &mut Rig, look: &Look, pose: &Pose) {
    let (body, dark, light, accent) = (
        hex(&look.skin),
        hex(&look.top),
        hex(&look.hair),
        hex(&look.accent),
    );
    let gait = fx::mul(fx::mul(fx::sin(pose.step), deg(30)), pose.moving);
    for (x, z, a) in [
        (-26, -30, gait),
        (26, -30, -gait),
        (-26, 34, -gait),
        (26, 34, gait),
    ] {
        let foot = r.limb(c3(x, 52, z), cm(50), cm(18), &Affine::rot_x(a), dark);
        r.put(Mesh::Sphere, foot, c3(22, 12, 26), dark);
    }
    r.put(Mesh::Sphere, c3(0, 66, 4), c3(80, 52, 108), body);
    for (x, z) in [(-28, 10), (24, -14), (6, 40)] {
        r.put(Mesh::Sphere, c3(x, 84, z), c3(22, 10, 22), light);
    }
    // tail
    let wag = fx::mul(deg(25), fx::sin(pose.breath * 2));
    let t1 = r.limb(
        c3(0, 76, 54),
        cm(40),
        cm(12),
        &Affine::rot_x(deg(140)).then(&Affine::rot_z(wag)),
        dark,
    );
    r.put(Mesh::Sphere, t1, c3(16, 16, 16), dark);
    // head, lunging with the swing
    let lunge = fx::mul(cm(30), fx::sin(fx::mul(fx::HALF, pose.swing)));
    let h = c3(0, 102, -52).add(v3(0, 0, -lunge));
    r.put(Mesh::Sphere, h, c3(78, 66, 66), body);
    for x in [-1, 1] {
        r.part(
            Mesh::Cone,
            h.add(c3(x * 24, 42, 0)),
            c3(24, 36, 16),
            &Affine::rot_z(-x * deg(12)),
            body,
        );
        r.glow(h.add(c3(x * 17, 6, -30)), cm(8), [ONE / 2, ONE, ONE], ONE / 3);
    }
    r.put(Mesh::Cube, h.add(c3(0, 26, -26)), c3(40, 12, 10), light);
    r.put(Mesh::Cube, h.add(c3(0, -18, -28)), c3(44, 10, 14), accent);
    for k in 0..4 {
        r.put(Mesh::Cone, h.add(c3(-15 + k * 10, -14, -34)), c3(6, 8, 6), WHITE);
    }
    let _ = GOLD;
}

fn dog(r: &mut Rig, look: &Look, pose: &Pose) {
    let (fluff, dark, accent) = (hex(&look.skin), hex(&look.hair), hex(&look.accent));
    let gait = fx::mul(fx::mul(fx::sin(pose.step), deg(35)), pose.moving);
    for (x, z, a) in [
        (-14, -18, gait),
        (14, -18, -gait),
        (-14, 20, -gait),
        (14, 20, gait),
    ] {
        let foot = r.limb(c3(x, 32, z), cm(30), cm(10), &Affine::rot_x(a), fluff);
        r.put(Mesh::Sphere, foot, c3(16, 14, 16), fluff);
    }
    for (x, y, z, d) in [
        (0, 44, 0, 52),
        (-12, 50, 14, 30),
        (12, 50, -12, 30),
        (0, 56, 0, 34),
    ] {
        r.put(Mesh::Sphere, c3(x, y, z), c3(d, d - 6, d + 8), fluff);
    }
    let wag = fx::mul(cm(8), fx::sin(pose.breath * 4));
    r.put(Mesh::Sphere, c3(wag, 64, 34), c3(18, 18, 18), fluff);
    let lunge = fx::mul(cm(16), fx::sin(fx::mul(fx::HALF, pose.swing)));
    let h = c3(0, 74, -30).add(v3(0, 0, -lunge));
    r.put(Mesh::Sphere, h, c3(40, 38, 38), fluff);
    r.put(Mesh::Sphere, h.add(c3(0, 22, 0)), c3(30, 24, 30), fluff);
    for x in [-1, 1] {
        r.put(Mesh::Sphere, h.add(c3(x * 20, -8, 2)), c3(16, 32, 16), dark);
        r.put(Mesh::Sphere, h.add(c3(x * 9, 4, -18)), c3(6, 7, 4), INK);
    }
    r.put(Mesh::Sphere, h.add(c3(0, -6, -22)), c3(20, 16, 18), fluff);
    r.put(Mesh::Sphere, h.add(c3(0, -2, -31)), c3(8, 6, 6), INK);
    r.put(Mesh::Cyl, c3(0, 58, -18), c3(34, 6, 30), accent);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixtures::content;
    use crate::render::Batch;

    fn count(id: &str, pose: Pose) -> usize {
        let c = content();
        let u = &c.units[c.unit_index(id).unwrap()];
        let mut b = Batch::new(INK);
        draw(&mut b, &u.look, u.kind, V3::ZERO, 0, &pose);
        b.count()
    }

    #[test]
    fn every_unit_is_drawn_from_some_shapes() {
        let c = content();
        for u in &c.units {
            assert!(count(&u.id, Pose::default()) > 20, "{}", u.id);
        }
    }

    #[test]
    fn a_bigger_look_is_a_taller_figure() {
        let c = content();
        let chum = &c.units[c.unit_index("chum").unwrap()].look;
        let pip = &c.units[c.unit_index("pip").unwrap()].look;
        let rook = &c.units[c.unit_index("rook").unwrap()].look;
        assert!(height(rook) > height(pip));
        assert!(height(chum) > cm(200));
    }

    #[test]
    fn a_fallen_figure_lies_low() {
        // the chest of a figure that is down sits near the ground
        let c = content();
        let u = &c.units[c.unit_index("rook").unwrap()];
        let mut stand = Batch::new(INK);
        let mut down = Batch::new(INK);
        draw(&mut stand, &u.look, u.kind, V3::ZERO, 0, &Pose::default());
        let fallen = Pose {
            down: ONE,
            ..Pose::default()
        };
        draw(&mut down, &u.look, u.kind, V3::ZERO, 0, &fallen);
        let top = |b: Batch| {
            let (v, _) = b.finish(crate::render::DYNAMIC_BUFFER);
            // the highest translation of any shape
            v.chunks(crate::render::INSTANCE_VALUES)
                .map(|i| i[7])
                .max()
                .unwrap()
        };
        assert!(
            top(down) < cm(90),
            "a fallen figure's highest shape is under 90 cm"
        );
        assert!(top(stand) > cm(180));
    }
}
