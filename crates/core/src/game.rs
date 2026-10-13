//! The one door the shim calls through. A `Game` holds the roster, the
//! campaign, the player's profile and the online session; every method takes
//! plain values and returns JSON text, `{"ok": ...}` or `{"error": "<key>"}`
//! where the key names a sentence in `data/copy.json`. Every number the page
//! draws, percentages for bars included, is worked out here.

use serde::Serialize;
use serde_json::{json, Value};

use crate::balance::*;
use crate::battle::{self, Entry, Record};
use crate::content::{Content, Effect, Kind, Stats};
use crate::online::{self, Role, Session};
use crate::profile::{xp_pct, Profile, Reward};
use crate::rng::Rng;

fn ok<T: Serialize>(v: &T) -> String {
    json!({ "ok": v }).to_string()
}

fn err(key: &str) -> String {
    json!({ "error": key }).to_string()
}

#[derive(Debug, Clone, Serialize)]
struct SkillView {
    name: String,
    effect: Effect,
    power: i32,
    cooldown: i32,
}

#[derive(Debug, Clone, Serialize)]
struct UnitView {
    id: String,
    name: String,
    title: String,
    blurb: String,
    kind: Kind,
    /// The kind this unit hits hard, and the kind that hits it hard.
    beats: Kind,
    weak_to: Kind,
    level: u32,
    xp: u32,
    need: u32,
    xp_pct: i32,
    maxed: bool,
    stats: Stats,
    /// Stats one level up, or the same at twenty.
    next: Stats,
    skill: SkillView,
    locked: bool,
    unlock: u32,
    in_team: bool,
}

#[derive(Debug, Clone, Serialize)]
struct FoeView {
    id: String,
    name: String,
    kind: Kind,
    level: u32,
}

/// A fight for the page to play, and what it paid.
#[derive(Debug, Clone, Serialize)]
struct FightView {
    mode: &'static str,
    backdrop: String,
    title: String,
    record: Record,
    reward: Reward,
    /// The campaign stage, when there is one, so the page can offer the next.
    stage: Option<usize>,
    next_stage_open: bool,
}

pub struct Game {
    content: Content,
    profile: Profile,
    online: Session,
}

impl Game {
    pub fn new(units_json: &str, campaign_json: &str, saved: &str) -> Result<Game, String> {
        let content = Content::parse(units_json, campaign_json)?;
        let profile = Profile::load(&content, saved);
        Ok(Game {
            content,
            profile,
            online: Session::default(),
        })
    }

    pub fn save(&self) -> String {
        self.profile.save()
    }

    /// Back to a new player.
    pub fn reset(&mut self) -> String {
        self.profile = Profile::new(&self.content);
        self.home()
    }

    fn unit_view(&self, u: usize) -> UnitView {
        let def = &self.content.units[u];
        let p = &self.profile.units[u];
        let maxed = p.level >= MAX_UNIT_LEVEL;
        let need = if maxed { 0 } else { unit_xp_to_next(p.level) };
        UnitView {
            id: def.id.clone(),
            name: def.name.clone(),
            title: def.title.clone(),
            blurb: def.blurb.clone(),
            kind: def.kind,
            beats: def.kind.beats(),
            weak_to: def.kind.beats().beats(),
            level: p.level,
            xp: p.xp,
            need,
            xp_pct: if maxed { 100 } else { xp_pct(p.xp, need) },
            maxed,
            stats: def.stats(p.level),
            next: def.stats(p.level + 1),
            skill: SkillView {
                name: def.skill.name.clone(),
                effect: def.skill.effect,
                power: def.skill.power,
                cooldown: def.skill.cooldown,
            },
            locked: !self.profile.unlocked(&self.content, u),
            unlock: def.unlock,
            in_team: self.profile.team.contains(&def.id),
        }
    }

    fn foe_view(&self, (u, level): Entry) -> FoeView {
        let def = &self.content.units[u];
        FoeView {
            id: def.id.clone(),
            name: def.name.clone(),
            kind: def.kind,
            level,
        }
    }

    pub fn home(&self) -> String {
        let p = &self.profile;
        let next_unlock = self
            .content
            .units
            .iter()
            .filter(|u| u.unlock > p.level)
            .min_by_key(|u| u.unlock)
            .map(|u| json!({ "id": u.id, "name": u.name, "level": u.unlock }));
        let team: Vec<UnitView> = self
            .profile
            .team_entries(&self.content)
            .iter()
            .map(|e| self.unit_view(e.0))
            .collect();
        let maxed = p.level >= MAX_ACCOUNT_LEVEL;
        ok(&json!({
            "level": p.level,
            "xp": p.xp,
            "need": if maxed { 0 } else { account_xp_to_next(p.level) },
            "xp_pct": p.account_xp_pct(),
            "maxed": maxed,
            "next_unlock": next_unlock,
            "team": team,
            "wins": p.wins,
            "losses": p.losses,
            "stars": p.stars.iter().map(|s| *s as u32).sum::<u32>(),
            "stars_max": p.stars.len() * 3,
            "unlocked": (0..self.content.units.len()).filter(|&u| p.unlocked(&self.content, u)).count(),
            "units": self.content.units.len(),
        }))
    }

    pub fn roster(&self) -> String {
        let all: Vec<UnitView> = (0..self.content.units.len()).map(|u| self.unit_view(u)).collect();
        ok(&all)
    }

    /// `ids_json` is a JSON array of three unit ids.
    pub fn set_team(&mut self, ids_json: &str) -> String {
        let Ok(ids) = serde_json::from_str::<Vec<String>>(ids_json) else {
            return err("team_need_three");
        };
        match self.profile.check_team(&self.content, &ids) {
            Ok(_) => {
                self.profile.team = ids;
                let mine = self.my_team_wire();
                self.online.retell(&mine);
                self.home()
            }
            Err(key) => err(key),
        }
    }

    pub fn campaign(&self) -> String {
        let mut index = 0;
        let mut chapters = Vec::new();
        for ch in &self.content.chapters {
            let mut stages = Vec::new();
            for st in &ch.stages {
                let enemies: Vec<FoeView> = st
                    .enemies
                    .iter()
                    .map(|(id, l)| self.foe_view((self.content.unit_index(id).unwrap_or(0), *l)))
                    .collect();
                let stars = self.profile.stars[index];
                stages.push(json!({
                    "index": index,
                    "name": st.name,
                    "stars": stars,
                    "stars_missing": 3 - stars.min(3),
                    "open": self.profile.stage_open(index),
                    "cleared": stars > 0,
                    "enemies": enemies,
                    "unit_xp": stage_unit_xp(index, true),
                    "account_xp": stage_account_xp(index, true, stars == 0),
                }));
                index += 1;
            }
            let open = stages.iter().any(|s| s["open"] == Value::Bool(true));
            chapters.push(json!({
                "name": ch.name,
                "backdrop": ch.backdrop,
                "blurb": ch.blurb,
                "open": open,
                "stages": stages,
            }));
        }
        ok(&chapters)
    }

    pub fn fight_campaign(&mut self, stage: usize, seed: u64) -> String {
        let Some((chapter, def)) = self.content.stage(stage) else {
            return err("stage_unknown");
        };
        if !self.profile.stage_open(stage) {
            return err("stage_locked");
        }
        let foes: Vec<Entry> = def
            .enemies
            .iter()
            .filter_map(|(id, l)| self.content.unit_index(id).map(|u| (u, *l)))
            .collect();
        let title = def.name.clone();
        let backdrop = self.content.chapters[chapter].backdrop.clone();
        let mine = self.profile.team_entries(&self.content);
        let record = battle::fight(&self.content, &mine, &foes, seed);
        let won = record.winner == Some(0);
        let first_clear = won && self.profile.stars[stage] == 0;
        let units: Vec<usize> = mine.iter().map(|e| e.0).collect();
        let mut reward = self.profile.grant(
            &self.content,
            won,
            stage_account_xp(stage, won, first_clear),
            &units,
            stage_unit_xp(stage, won),
        );
        if won {
            let s = stars(record.survivors[0]);
            reward.stars = s;
            reward.stars_missing = 3 - s;
            self.profile.stars[stage] = self.profile.stars[stage].max(s);
        }
        reward.first_clear = first_clear;
        let next = stage + 1;
        ok(&FightView {
            mode: "campaign",
            backdrop,
            title,
            record,
            reward,
            stage: Some(stage),
            next_stage_open: next < self.content.stage_count() && self.profile.stage_open(next),
        })
    }

    /// Two teams of three drawn at random from the whole roster, locked
    /// units included, at levels around the account's.
    pub fn fight_random(&mut self, seed: u64) -> String {
        let mut rng = Rng::new(seed);
        let n = self.content.units.len();
        let centre = clamp_unit_level(self.profile.level) as i32;
        let draw = |rng: &mut Rng| -> Vec<Entry> {
            let mut picked: Vec<usize> = Vec::new();
            while picked.len() < TEAM_SIZE {
                let u = rng.range(0, n as i32 - 1) as usize;
                if !picked.contains(&u) {
                    picked.push(u);
                }
            }
            picked
                .into_iter()
                .map(|u| {
                    let l = rng.range(centre - RANDOM_LEVEL_SPREAD, centre + RANDOM_LEVEL_SPREAD);
                    (u, clamp_unit_level(l.max(1) as u32))
                })
                .collect()
        };
        let mine = draw(&mut rng);
        let foes = draw(&mut rng);
        let backdrops: Vec<String> = self.content.chapters.iter().map(|c| c.backdrop.clone()).collect();
        let backdrop = backdrops[rng.range(0, backdrops.len() as i32 - 1) as usize].clone();
        let record = battle::fight(&self.content, &mine, &foes, rng.next());
        let won = record.winner == Some(0);
        let reward = self
            .profile
            .grant(&self.content, won, random_account_xp(won), &[], 0);
        ok(&FightView {
            mode: "random",
            backdrop,
            title: String::new(),
            record,
            reward,
            stage: None,
            next_stage_open: false,
        })
    }

    // ---- online ------------------------------------------------------------

    fn my_team_wire(&self) -> Vec<(String, u32)> {
        online::write_team(&self.content, &self.profile.team_entries(&self.content))
    }

    pub fn host_online(&mut self, build: &str, seed: u64) -> String {
        self.online = Session::host(build, seed);
        self.online.room.clone()
    }

    /// False if the code cannot be a room code. An empty code is a joiner by
    /// pasted invitation.
    pub fn join_online(&mut self, build: &str, code: &str) -> bool {
        if code.trim().is_empty() {
            self.online = Session::join_by_paste(build);
            return true;
        }
        match Session::join(build, code) {
            Some(s) => {
                self.online = s;
                true
            }
            None => false,
        }
    }

    pub fn leave_online(&mut self) {
        self.online = Session::default();
    }

    pub fn room(&self) -> String {
        self.online.room.clone()
    }

    pub fn net_peer(&mut self, id: u32) {
        let mine = self.my_team_wire();
        self.online.peer(id, &mine);
    }

    pub fn net_left(&mut self, id: u32) {
        self.online.left(id);
    }

    pub fn net_message(&mut self, id: u32, text: &str) {
        let mine = self.my_team_wire();
        self.online.message(&self.content, id, text, &mine);
    }

    /// A transport error, by its key in copy.json.
    pub fn net_error(&mut self, key: &str) {
        self.online.status = match key {
            "online_no_relay" => "online_no_relay",
            "online_no_path" => "online_no_path",
            _ => "online_left",
        };
    }

    pub fn net_outbox(&mut self) -> String {
        let out = self.online.take_outbox();
        serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
    }

    pub fn online_view(&self) -> String {
        let s = &self.online;
        let foe: Vec<FoeView> = s
            .foe
            .as_ref()
            .and_then(|t| online::read_team(&self.content, t))
            .unwrap_or_default()
            .into_iter()
            .map(|e| self.foe_view(e))
            .collect();
        ok(&json!({
            "role": s.role,
            "room": s.room,
            "status": s.status,
            "connected": s.connected(),
            "foe": foe,
            "can_start": s.role == Role::Host && s.connected() && !foe.is_empty(),
            "fights": s.fights,
        }))
    }

    /// The host starts a fight against the browser it is connected to.
    pub fn online_start(&mut self, seed: u64) -> String {
        let mine = self.profile.team_entries(&self.content);
        if self.online.start(&self.content, &mine, seed) {
            ok(&true)
        } else {
            err("online_no_foe")
        }
    }

    /// A fight that has arrived, with its reward applied, or `null`.
    pub fn online_take(&mut self) -> String {
        let Some(ready) = self.online.take_ready() else {
            return ok(&Value::Null);
        };
        let won = ready.record.winner == Some(0);
        let reward = self.profile.grant(
            &self.content,
            won,
            online_account_xp(won),
            &ready.my_team,
            online_unit_xp(won),
        );
        // The fight may have levelled units up: tell the other browser.
        let mine = self.my_team_wire();
        self.online.retell(&mine);
        let backdrops: Vec<String> = self.content.chapters.iter().map(|c| c.backdrop.clone()).collect();
        let backdrop = backdrops[(ready.record.seed % backdrops.len() as u64) as usize].clone();
        ok(&FightView {
            mode: "online",
            backdrop,
            title: String::new(),
            record: ready.record,
            reward,
            stage: None,
            next_stage_open: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixtures::{CAMPAIGN, UNITS};

    fn game() -> Game {
        Game::new(UNITS, CAMPAIGN, "").unwrap()
    }

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn a_new_game_opens_on_account_level_one_with_three_of_twelve_units() {
        let g = game();
        let h = parse(&g.home());
        assert_eq!(h["ok"]["level"], 1);
        assert_eq!(h["ok"]["unlocked"], 3);
        assert_eq!(h["ok"]["units"], 12);
        assert_eq!(h["ok"]["next_unlock"]["id"], "nan");
        let r = parse(&g.roster());
        let locked = r["ok"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|u| u["locked"] == true)
            .count();
        assert_eq!(locked, 9);
    }

    #[test]
    fn a_locked_unit_cannot_join_the_team() {
        let mut g = game();
        assert_eq!(
            parse(&g.set_team(r#"["rook","chum","pip"]"#))["error"],
            "team_locked"
        );
        assert_eq!(parse(&g.set_team("nonsense"))["error"], "team_need_three");
        assert!(parse(&g.set_team(r#"["pip","rook","vela"]"#))["ok"].is_object());
    }

    #[test]
    fn a_locked_stage_cannot_be_fought_and_winning_one_opens_the_next() {
        let mut g = game();
        assert_eq!(parse(&g.fight_campaign(1, 1))["error"], "stage_locked");
        assert_eq!(parse(&g.fight_campaign(99, 1))["error"], "stage_unknown");
        let mut seed = 0;
        loop {
            let f = parse(&g.fight_campaign(0, seed));
            if f["ok"]["reward"]["won"] == true {
                assert!(f["ok"]["next_stage_open"] == true);
                assert!(f["ok"]["reward"]["stars"].as_u64().unwrap() >= 1);
                assert_eq!(f["ok"]["reward"]["first_clear"], true);
                break;
            }
            seed += 1;
            assert!(seed < 50, "the first stage should be winnable");
        }
        assert!(parse(&g.fight_campaign(1, 1))["ok"].is_object());
    }

    #[test]
    fn a_fight_pays_the_team_experience_and_the_save_keeps_it() {
        let mut g = game();
        let f = parse(&g.fight_campaign(0, 3));
        let units = f["ok"]["reward"]["units"].as_array().unwrap();
        assert_eq!(units.len(), 3);
        assert!(units.iter().all(|u| u["xp"].as_u64().unwrap() > 0));
        let saved = g.save();
        let again = Game::new(UNITS, CAMPAIGN, &saved).unwrap();
        assert_eq!(again.save(), saved);
    }

    #[test]
    fn random_mode_deals_two_teams_of_three_and_pays_only_the_account() {
        let mut g = game();
        let f = parse(&g.fight_random(17));
        let teams = f["ok"]["record"]["teams"].as_array().unwrap();
        assert_eq!(teams[0].as_array().unwrap().len(), 3);
        assert_eq!(teams[1].as_array().unwrap().len(), 3);
        assert!(f["ok"]["reward"]["units"].as_array().unwrap().is_empty());
        assert!(f["ok"]["reward"]["account_xp"].as_u64().unwrap() > 0);
        assert_ne!(parse(&g.fight_random(18))["ok"]["record"], f["ok"]["record"]);
    }

    #[test]
    fn random_mode_can_deal_units_the_account_has_not_unlocked() {
        let mut g = game();
        let starters = ["rook", "vela", "pip"];
        let any_locked = (0..40).any(|s| {
            let f = parse(&g.fight_random(s));
            f["ok"]["record"]["teams"][0]
                .as_array()
                .unwrap()
                .iter()
                .any(|u| !starters.contains(&u["id"].as_str().unwrap()))
        });
        assert!(any_locked);
    }

    #[test]
    fn two_games_play_an_online_fight_and_each_is_paid_from_its_own_seat() {
        let mut a = game();
        let mut b = game();
        let room = a.host_online("x", 5);
        assert!(b.join_online("x", &room));
        a.net_peer(2);
        b.net_peer(1);
        for _ in 0..3 {
            let out: Vec<(u32, String)> = serde_json::from_str(&a.net_outbox()).unwrap();
            for (_, m) in out {
                b.net_message(1, &m);
            }
            let out: Vec<(u32, String)> = serde_json::from_str(&b.net_outbox()).unwrap();
            for (_, m) in out {
                a.net_message(2, &m);
            }
        }
        assert_eq!(parse(&a.online_view())["ok"]["can_start"], true);
        assert_eq!(parse(&b.online_view())["ok"]["can_start"], false);
        assert!(parse(&a.online_start(9))["ok"] == true);
        let out: Vec<(u32, String)> = serde_json::from_str(&a.net_outbox()).unwrap();
        for (_, m) in out {
            b.net_message(1, &m);
        }
        let fa = parse(&a.online_take());
        let fb = parse(&b.online_take());
        let wa = fa["ok"]["reward"]["won"].as_bool().unwrap();
        let wb = fb["ok"]["reward"]["won"].as_bool().unwrap();
        let drawn = fa["ok"]["record"]["winner"].is_null();
        assert!(drawn || wa != wb, "one browser wins and the other loses");
        assert_eq!(fa["ok"]["record"]["teams"][0], fb["ok"]["record"]["teams"][1]);
        assert!(parse(&a.online_take())["ok"].is_null());
    }

    #[test]
    fn a_typed_room_code_that_cannot_be_one_is_refused() {
        let mut g = game();
        assert!(!g.join_online("x", "a!"));
        assert!(g.join_online("x", ""));
    }
}
