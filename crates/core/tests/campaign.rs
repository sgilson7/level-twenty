//! The campaign's pacing, measured: a player who fields the three starters,
//! swaps in units as the account unlocks them, and replays each stage until
//! it is won. The numbers it prints are the ones to read when tuning
//! `data/campaign.json` or `balance.rs`.

use lt_core::game::Game;
use serde_json::Value;

const UNITS: &str = include_str!("../../../data/units.json");
const CAMPAIGN: &str = include_str!("../../../data/campaign.json");

fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

/// Plays the whole campaign; returns the fights each stage took.
fn play(seed0: u64, team_of: fn(&Value) -> Vec<String>) -> Vec<u32> {
    let mut g = Game::new(UNITS, CAMPAIGN, "").unwrap();
    let mut seed = seed0;
    let mut tries = Vec::new();
    for stage in 0..15 {
        let mut n = 0;
        loop {
            n += 1;
            seed += 1;
            let roster = parse(&g.roster());
            let team = team_of(&roster["ok"]);
            g.set_team(&serde_json::to_string(&team).unwrap());
            let f = parse(&g.fight_campaign(stage, seed));
            if f["ok"]["reward"]["won"] == true || n >= 200 {
                break;
            }
        }
        tries.push(n);
    }
    tries
}

/// The three highest-levelled unlocked units, starters first on a tie.
fn best_three(roster: &Value) -> Vec<String> {
    let mut open: Vec<&Value> = roster
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| u["locked"] == false)
        .collect();
    open.sort_by_key(|u| std::cmp::Reverse(u["level"].as_u64().unwrap()));
    open.iter()
        .take(3)
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect()
}

fn starters(_: &Value) -> Vec<String> {
    vec!["rook".into(), "vela".into(), "pip".into()]
}

#[test]
fn the_campaign_can_be_finished_and_asks_for_some_replays() {
    for (name, team) in [
        ("starters", starters as fn(&Value) -> Vec<String>),
        ("best three", best_three),
    ] {
        let runs: Vec<Vec<u32>> = (0..5).map(|k| play(k * 1000, team)).collect();
        for r in &runs {
            println!("{name}: {:?} total {}", r, r.iter().sum::<u32>());
        }
        for r in &runs {
            assert!(
                r.iter().all(|&n| n < 200),
                "{name}: a stage could not be won in 200 tries: {r:?}"
            );
            let total: u32 = r.iter().sum();
            assert!(total >= 20, "{name}: the campaign is a walkover ({total} fights)");
            assert!(total <= 120, "{name}: the campaign is a grind ({total} fights)");
        }
    }
}
