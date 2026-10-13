//! Every feel number in one place. A unit's own numbers are in
//! `data/units.json`; these are the rules they are read through.

pub const TEAM_SIZE: usize = 3;
/// Skills per unit; the player fields one.
pub const SKILLS: usize = 3;

pub const MAX_UNIT_LEVEL: u32 = 20;
pub const MAX_ACCOUNT_LEVEL: u32 = 30;

/// A turn comes when a unit's gauge reaches this; each moment of the fight
/// adds the unit's speed to it (Summoners War's attack bar).
pub const GAUGE_FULL: i32 = 1000;
/// A fight that has not ended after this many turns is decided on health.
pub const MAX_TURNS: u32 = 200;

/// Damage in percent: with the edge, against it, and between two of a kind.
pub const EDGE_GOOD: i32 = 150;
pub const EDGE_BAD: i32 = 70;
pub const EDGE_EVEN: i32 = 100;
/// A basic attack is this percent of attack.
pub const BASIC_POWER: i32 = 100;
/// Each hit lands between these percents of its damage.
pub const SPREAD_LOW: i32 = 92;
pub const SPREAD_HIGH: i32 = 108;
pub const CRIT_CHANCE: i32 = 15;
pub const CRIT_POWER: i32 = 150;
/// How far Haste moves every other ally's gauge.
pub const HASTE_GAUGE: i32 = 300;
/// How far Slow pushes the target's gauge back.
pub const SLOW_GAUGE: i32 = 350;
/// Drain mends the attacker by this percent of the damage it did.
pub const DRAIN_SHARE: i32 = 50;

/// Experience a unit needs to go from `level` to the next.
pub fn unit_xp_to_next(level: u32) -> u32 {
    30 + 20 * level
}

/// Experience the account needs to go from `level` to the next.
pub fn account_xp_to_next(level: u32) -> u32 {
    50 * level
}

pub fn clamp_unit_level(level: u32) -> u32 {
    level.clamp(1, MAX_UNIT_LEVEL)
}

/// What a campaign stage pays, by its flat index (0 is the first stage).
pub fn stage_unit_xp(stage: usize, won: bool) -> u32 {
    let full = 40 + 12 * (stage as u32 + 1);
    if won {
        full
    } else {
        full / 4
    }
}

pub fn stage_account_xp(stage: usize, won: bool, first_clear: bool) -> u32 {
    let full = 40 + 15 * (stage as u32 + 1);
    match (won, first_clear) {
        (true, true) => full * 2,
        (true, false) => full,
        (false, _) => 10,
    }
}

/// Random mode pays the account only: the units in it are not yours.
pub fn random_account_xp(won: bool) -> u32 {
    if won {
        45
    } else {
        15
    }
}
/// Random mode deals levels this far either side of the account's level.
pub const RANDOM_LEVEL_SPREAD: i32 = 2;

pub fn online_account_xp(won: bool) -> u32 {
    if won {
        60
    } else {
        20
    }
}

pub fn online_unit_xp(won: bool) -> u32 {
    if won {
        80
    } else {
        25
    }
}

/// Stars for a campaign win: one for each of your units still standing.
pub fn stars(survivors: u32) -> u8 {
    survivors.min(3) as u8
}
