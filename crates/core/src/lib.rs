//! lt_core: every rule of Level Twenty.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `game.rs` is the one door the shim calls through.

pub mod balance;
pub mod battle;
pub mod content;
pub mod game;
pub mod online;
pub mod online_code;
pub mod profile;
pub mod rng;
