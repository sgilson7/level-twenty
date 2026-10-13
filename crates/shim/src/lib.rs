//! lt_shim: moves values between the page and `lt_core::game`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion of its arguments. The page's clock arrives
//! as a float of milliseconds and becomes the core's whole-number seed. The
//! core speaks Q16.16 integers; WebGL wants `f32`, so the stage's values on
//! the way out are divided by the core's own `ONE`.
//! `crates/core/tests/boundary.rs` fails if a branch, a loop or the `?`
//! operator appears here.

use lt_core::{fx, game};
use wasm_bindgen::prelude::*;

const ONE: f32 = fx::ONE as f32;

fn floats(v: Vec<i32>) -> Vec<f32> {
    v.into_iter().map(|x| x as f32 / ONE).collect()
}

/// The renderer's sizes, strides and offsets.
#[wasm_bindgen]
pub fn stage_numbers() -> String {
    game::stage_numbers()
}

#[wasm_bindgen]
pub fn mesh_vertices(id: u32) -> Vec<f32> {
    floats(game::mesh_vertices(id))
}

/// White noise for the sound, `-1..1`.
#[wasm_bindgen]
pub fn noise(n: u32, seed: u32) -> Vec<f32> {
    floats(game::noise(n, seed))
}

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

    pub fn set_skill(&mut self, id: &str, k: u32) -> String {
        self.inner.set_skill(id, k)
    }

    pub fn stage_begin(&self) -> String {
        self.inner.stage_begin()
    }

    pub fn stage_scene_values(&self) -> Vec<f32> {
        floats(self.inner.stage_scene().0)
    }

    pub fn stage_scene_draws(&self) -> Vec<u32> {
        self.inner.stage_scene().1
    }

    pub fn stage_colours(&self) -> Vec<f32> {
        floats(self.inner.stage_colours())
    }

    /// The page's playback clock in milliseconds, and the canvas size.
    pub fn stage_frame(&mut self, t_ms: f64, width: u32, height: u32) {
        self.inner.stage_frame(t_ms as i32, width, height)
    }

    pub fn stage_view(&self) -> Vec<f32> {
        floats(self.inner.stage_view())
    }

    pub fn stage_values(&self) -> Vec<f32> {
        floats(self.inner.stage_values())
    }

    pub fn stage_draws(&self) -> Vec<u32> {
        self.inner.stage_draws()
    }

    /// Each unit's head on screen, in percent from the left and the top.
    pub fn stage_marks(&self) -> Vec<f32> {
        floats(self.inner.stage_marks())
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
