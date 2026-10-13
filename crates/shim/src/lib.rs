//! lt_shim: moves values between the page and `lt_core::game`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion of its arguments. The page's clock arrives
//! as a float of milliseconds and becomes the core's whole-number seed.
//! `crates/core/tests/boundary.rs` fails if a branch, a loop or the `?`
//! operator appears here.

use lt_core::game;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Game {
    inner: game::Game,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(units: &str, campaign: &str, saved: &str) -> Result<Game, JsValue> {
        game::Game::new(units, campaign, saved)
            .map(|inner| Game { inner })
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn save(&self) -> String {
        self.inner.save()
    }

    pub fn reset(&mut self) -> String {
        self.inner.reset()
    }

    pub fn home(&self) -> String {
        self.inner.home()
    }

    pub fn roster(&self) -> String {
        self.inner.roster()
    }

    pub fn set_team(&mut self, ids_json: &str) -> String {
        self.inner.set_team(ids_json)
    }

    pub fn campaign(&self) -> String {
        self.inner.campaign()
    }

    pub fn fight_campaign(&mut self, stage: u32, seed: f64) -> String {
        self.inner.fight_campaign(stage as usize, seed as u64)
    }

    pub fn fight_random(&mut self, seed: f64) -> String {
        self.inner.fight_random(seed as u64)
    }

    pub fn host_online(&mut self, build: &str, seed: f64) -> String {
        self.inner.host_online(build, seed as u64)
    }

    pub fn join_online(&mut self, build: &str, code: &str) -> bool {
        self.inner.join_online(build, code)
    }

    pub fn leave_online(&mut self) {
        self.inner.leave_online()
    }

    pub fn room(&self) -> String {
        self.inner.room()
    }

    pub fn net_peer(&mut self, id: u32) {
        self.inner.net_peer(id)
    }

    pub fn net_left(&mut self, id: u32) {
        self.inner.net_left(id)
    }

    pub fn net_message(&mut self, id: u32, text: &str) {
        self.inner.net_message(id, text)
    }

    pub fn net_error(&mut self, key: &str) {
        self.inner.net_error(key)
    }

    pub fn net_outbox(&mut self) -> String {
        self.inner.net_outbox()
    }

    pub fn online_view(&self) -> String {
        self.inner.online_view()
    }

    pub fn online_start(&mut self, seed: f64) -> String {
        self.inner.online_start(seed as u64)
    }

    pub fn online_take(&mut self) -> String {
        self.inner.online_take()
    }
}
