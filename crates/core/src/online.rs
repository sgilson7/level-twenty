//! Online 1v1. Two browsers meet through `web/net/` (Ribbit King's
//! transport), each sends the other its team, and the host sends a seed. A
//! fight is decided in full from two teams and a seed (`battle::fight`), so
//! there is nothing to keep in step while it plays: both browsers work out
//! the same fight, and the joiner plays it from its own seat. The host also
//! sends the fight's fingerprint, so a joiner that worked out a different
//! fight says so instead of showing it.
//!
//! The transport hands the core peers, departures and messages; the core
//! leaves what to send in an outbox. Messages are JSON text.

use serde::{Deserialize, Serialize};

use crate::balance::TEAM_SIZE;
use crate::battle::{self, Entry, Record};
use crate::content::Content;
use crate::online_code;

/// Bumped when a message changes shape or a fight's rules change, so two
/// copies that would decide a fight differently never meet. 2: a unit on the
/// wire carries its chosen skill. 3: Dodge and Flank.
pub const PROTOCOL: u32 = 3;

/// A unit on the wire: its id, its level and the skill it fields.
pub type Wire = (String, u32, usize);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Msg {
    Hello {
        protocol: u32,
        build: String,
        team: Vec<Wire>,
    },
    Start {
        seed: u64,
        host: Vec<Wire>,
        joiner: Vec<Wire>,
        sum: u32,
    },
    Bye {
        why: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Off,
    Host,
    Joiner,
}

/// A fight ready for the page, from this browser's seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    pub record: Record,
    pub my_team: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub role: Role,
    pub room: String,
    build: String,
    /// The one other browser, once its link is up.
    peer: Option<u32>,
    /// Its team, once it has said hello.
    pub foe: Option<Vec<Wire>>,
    /// A key into copy.json saying what is happening.
    pub status: &'static str,
    outbox: Vec<(u32, String)>,
    ready: Option<Ready>,
    pub fights: u32,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            role: Role::Off,
            room: String::new(),
            build: String::new(),
            peer: None,
            foe: None,
            status: "online_off",
            outbox: Vec::new(),
            ready: None,
            fights: 0,
        }
    }
}

/// Reads a team someone else sent, refusing anything that is not three
/// different known units at levels 1 to 20.
pub fn read_team(content: &Content, team: &[Wire]) -> Option<Vec<Entry>> {
    if team.len() != TEAM_SIZE {
        return None;
    }
    let mut out: Vec<Entry> = Vec::new();
    for (id, level, skill) in team {
        let u = content.unit_index(id)?;
        if out.iter().any(|e| e.0 == u) || !(1..=crate::balance::MAX_UNIT_LEVEL).contains(level) {
            return None;
        }
        if !content.units[u].skill_open(*skill, *level) {
            return None;
        }
        out.push((u, *level, *skill));
    }
    Some(out)
}

pub fn write_team(content: &Content, team: &[Entry]) -> Vec<Wire> {
    team.iter()
        .map(|&(u, l, k)| (content.units[u].id.clone(), l, k))
        .collect()
}

impl Session {
    pub fn host(build: &str, seed: u64) -> Session {
        Session {
            role: Role::Host,
            room: online_code::room_code(seed),
            build: build.to_string(),
            status: "online_waiting_foe",
            ..Session::default()
        }
    }

    /// `None` if the code cannot be a room code.
    pub fn join(build: &str, code: &str) -> Option<Session> {
        Some(Session {
            role: Role::Joiner,
            room: online_code::tidy_code(code)?,
            build: build.to_string(),
            status: "online_connecting",
            ..Session::default()
        })
    }

    /// A joiner by pasted invitation has no room code.
    pub fn join_by_paste(build: &str) -> Session {
        Session {
            role: Role::Joiner,
            build: build.to_string(),
            status: "online_connecting",
            ..Session::default()
        }
    }

    fn send(&mut self, to: u32, msg: &Msg) {
        self.outbox
            .push((to, serde_json::to_string(msg).unwrap_or_default()));
    }

    fn hello(&self, team: &[Wire]) -> Msg {
        Msg::Hello {
            protocol: PROTOCOL,
            build: self.build.clone(),
            team: team.to_vec(),
        }
    }

    /// The transport has a link to another browser.
    pub fn peer(&mut self, id: u32, my_team: &[Wire]) {
        if self.role == Role::Off {
            return;
        }
        if self.peer.is_some() && self.peer != Some(id) {
            self.send(
                id,
                &Msg::Bye {
                    why: "online_full".into(),
                },
            );
            return;
        }
        self.peer = Some(id);
        self.status = "online_hello";
        let hello = self.hello(my_team);
        self.send(id, &hello);
    }

    pub fn left(&mut self, id: u32) {
        if self.peer == Some(id) {
            self.peer = None;
            self.foe = None;
            self.status = "online_left";
        }
    }

    /// My team changed: tell the other browser.
    pub fn retell(&mut self, my_team: &[Wire]) {
        if let Some(id) = self.peer {
            let hello = self.hello(my_team);
            self.send(id, &hello);
        }
    }

    pub fn message(&mut self, content: &Content, id: u32, text: &str, my_team: &[Wire]) {
        if self.peer != Some(id) {
            return;
        }
        let Ok(msg) = serde_json::from_str::<Msg>(text) else {
            self.status = "online_garbled";
            return;
        };
        match msg {
            Msg::Hello {
                protocol,
                build,
                team,
            } => {
                if protocol != PROTOCOL || build != self.build {
                    self.status = "online_wrong_build";
                    self.send(
                        id,
                        &Msg::Bye {
                            why: "online_wrong_build".into(),
                        },
                    );
                    return;
                }
                if read_team(content, &team).is_none() {
                    self.status = "online_bad_team";
                    return;
                }
                self.foe = Some(team);
                self.status = if self.role == Role::Host {
                    "online_host_ready"
                } else {
                    "online_joiner_ready"
                };
            }
            Msg::Start {
                seed,
                host,
                joiner,
                sum,
            } => {
                if self.role != Role::Joiner {
                    return;
                }
                let (Some(h), Some(j)) = (read_team(content, &host), read_team(content, &joiner)) else {
                    self.status = "online_bad_team";
                    return;
                };
                // The host fights the crew it last heard about. Its levels
                // may be a fight old (both browsers retell after a fight,
                // and the host may press Fight first), so the joiner plays
                // the fight as the host set it if the units are the same;
                // refusing it would leave the host in a fight the joiner
                // never sees. Different units mean the joiner has changed
                // its crew since: say so, and wait for the next one.
                let same_units =
                    joiner.len() == my_team.len() && joiner.iter().zip(my_team).all(|(a, b)| a.0 == b.0);
                if !same_units {
                    self.status = "online_crew_changed";
                    self.retell(my_team);
                    return;
                }
                let record = battle::fight(content, &h, &j, seed);
                if record.sum() != sum {
                    self.status = "online_desync";
                    return;
                }
                self.foe = Some(host);
                self.fights += 1;
                self.status = "online_joiner_ready";
                self.ready = Some(Ready {
                    record: record.mirrored(),
                    my_team: j.iter().map(|e| e.0).collect(),
                });
            }
            Msg::Bye { why } => {
                self.status = match why.as_str() {
                    "online_full" => "online_full",
                    "online_wrong_build" => "online_wrong_build",
                    _ => "online_left",
                };
                self.peer = None;
                self.foe = None;
            }
        }
    }

    /// The host starts a fight. False when there is nobody to fight yet.
    pub fn start(&mut self, content: &Content, mine: &[Entry], seed: u64) -> bool {
        if self.role != Role::Host {
            return false;
        }
        let (Some(id), Some(foe)) = (self.peer, self.foe.clone()) else {
            return false;
        };
        let Some(theirs) = read_team(content, &foe) else {
            return false;
        };
        let record = battle::fight(content, mine, &theirs, seed);
        let msg = Msg::Start {
            seed,
            host: write_team(content, mine),
            joiner: foe,
            sum: record.sum(),
        };
        self.send(id, &msg);
        self.fights += 1;
        self.ready = Some(Ready {
            record,
            my_team: mine.iter().map(|e| e.0).collect(),
        });
        true
    }

    pub fn take_ready(&mut self) -> Option<Ready> {
        self.ready.take()
    }

    pub fn take_outbox(&mut self) -> Vec<(u32, String)> {
        std::mem::take(&mut self.outbox)
    }

    pub fn connected(&self) -> bool {
        self.peer.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::fixtures::content;

    fn team(v: &[(&str, u32)]) -> Vec<Wire> {
        v.iter().map(|(s, l)| (s.to_string(), *l, 0)).collect()
    }

    /// Carries every message in both outboxes to the other side until
    /// neither has anything left to say.
    fn pump(c: &Content, host: &mut Session, join: &mut Session, ht: &[Wire], jt: &[Wire]) {
        loop {
            let a = host.take_outbox();
            let b = join.take_outbox();
            if a.is_empty() && b.is_empty() {
                break;
            }
            for (_, m) in a {
                join.message(c, 1, &m, jt);
            }
            for (_, m) in b {
                host.message(c, 2, &m, ht);
            }
        }
    }

    #[test]
    fn two_browsers_meet_swap_teams_and_see_the_same_fight_from_their_own_seats() {
        let c = content();
        let ht = team(&[("rook", 5), ("vela", 4), ("pip", 6)]);
        let jt = team(&[("chum", 7), ("nan", 3), ("june", 2)]);
        let mut host = Session::host("b1", 99);
        let mut join = Session::join("b1", &host.room.to_lowercase()).unwrap();
        assert_eq!(join.room, host.room);
        host.peer(2, &ht);
        join.peer(1, &jt);
        pump(&c, &mut host, &mut join, &ht, &jt);
        assert_eq!(host.foe.as_deref(), Some(&jt[..]));
        assert_eq!(join.foe.as_deref(), Some(&ht[..]));
        assert_eq!(host.status, "online_host_ready");
        let mine = read_team(&c, &ht).unwrap();
        assert!(host.start(&c, &mine, 1234));
        pump(&c, &mut host, &mut join, &ht, &jt);
        let h = host.take_ready().unwrap();
        let j = join.take_ready().unwrap();
        assert_eq!(j.record, h.record.mirrored());
        assert_eq!(j.record.teams[0][0].id, "chum");
        assert_eq!(h.my_team.len(), 3);
        assert_eq!(join.status, "online_joiner_ready");
    }

    #[test]
    fn a_fight_set_with_last_fights_levels_is_still_played() {
        // After a fight the joiner's units may level up before the host
        // hears; the next Start names the old levels and must not be refused.
        let c = content();
        let ht = team(&[("rook", 5), ("vela", 4), ("pip", 6)]);
        let old = team(&[("chum", 7), ("nan", 3), ("june", 2)]);
        let now = team(&[("chum", 8), ("nan", 4), ("june", 3)]);
        let mut join = Session::join("b", "ABCDE").unwrap();
        join.peer(1, &now);
        join.take_outbox();
        let h = read_team(&c, &ht).unwrap();
        let o = read_team(&c, &old).unwrap();
        let sum = battle::fight(&c, &h, &o, 5).sum();
        let msg = Msg::Start {
            seed: 5,
            host: ht.clone(),
            joiner: old.clone(),
            sum,
        };
        join.message(&c, 1, &serde_json::to_string(&msg).unwrap(), &now);
        assert!(join.take_ready().is_some());
        // a different crew altogether is not played
        let other = team(&[("rook", 1), ("vela", 1), ("pip", 1)]);
        join.message(&c, 1, &serde_json::to_string(&msg).unwrap(), &other);
        assert!(join.take_ready().is_none());
        assert_eq!(join.status, "online_crew_changed");
    }

    #[test]
    fn a_different_build_is_turned_away_with_a_reason() {
        let c = content();
        let t = team(&[("rook", 1), ("vela", 1), ("pip", 1)]);
        let mut host = Session::host("b1", 1);
        let mut join = Session::join("b2", &host.room).unwrap();
        host.peer(2, &t);
        join.peer(1, &t);
        pump(&c, &mut host, &mut join, &t, &t);
        assert_eq!(host.status, "online_wrong_build");
        assert_eq!(join.status, "online_wrong_build");
        assert!(host.foe.is_none());
    }

    #[test]
    fn a_team_that_cannot_be_real_is_refused() {
        let c = content();
        assert!(read_team(&c, &team(&[("rook", 21), ("vela", 1), ("pip", 1)])).is_none());
        assert!(read_team(&c, &team(&[("rook", 2), ("rook", 1), ("pip", 1)])).is_none());
        assert!(read_team(&c, &team(&[("rook", 2), ("zed", 1), ("pip", 1)])).is_none());
        assert!(read_team(&c, &team(&[("rook", 2), ("vela", 1)])).is_none());
        assert!(read_team(&c, &team(&[("rook", 20), ("vela", 1), ("pip", 1)])).is_some());
        // a skill the unit's level has not opened
        let mut t = team(&[("rook", 4), ("vela", 1), ("pip", 1)]);
        t[0].2 = 1;
        assert!(read_team(&c, &t).is_none());
        t[0].1 = 5;
        assert!(read_team(&c, &t).is_some());
    }

    #[test]
    fn a_fight_the_joiner_works_out_differently_is_reported_not_shown() {
        let c = content();
        let t = team(&[("rook", 1), ("vela", 1), ("pip", 1)]);
        let mut join = Session::join("b", "ABCDE").unwrap();
        join.peer(1, &t);
        join.take_outbox();
        let msg = Msg::Start {
            seed: 5,
            host: t.clone(),
            joiner: t.clone(),
            sum: 0,
        };
        join.message(&c, 1, &serde_json::to_string(&msg).unwrap(), &t);
        assert_eq!(join.status, "online_desync");
        assert!(join.take_ready().is_none());
    }

    #[test]
    fn a_third_browser_is_told_the_room_is_full() {
        let t = team(&[("rook", 1), ("vela", 1), ("pip", 1)]);
        let mut host = Session::host("b", 3);
        host.peer(2, &t);
        host.take_outbox();
        host.peer(3, &t);
        let out = host.take_outbox();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 3);
        assert!(out[0].1.contains("online_full"));
    }

    #[test]
    fn the_host_cannot_start_alone_and_a_leaving_foe_is_forgotten() {
        let c = content();
        let t = team(&[("rook", 1), ("vela", 1), ("pip", 1)]);
        let mine = read_team(&c, &t).unwrap();
        let mut host = Session::host("b", 3);
        assert!(!host.start(&c, &mine, 1));
        host.peer(2, &t);
        host.message(
            &c,
            2,
            &serde_json::to_string(&Msg::Hello {
                protocol: PROTOCOL,
                build: "b".into(),
                team: t.clone(),
            })
            .unwrap(),
            &t,
        );
        assert!(host.foe.is_some());
        host.left(2);
        assert!(host.foe.is_none() && !host.connected());
        assert_eq!(host.status, "online_left");
        assert!(!host.start(&c, &mine, 1));
    }

    #[test]
    fn garbage_on_the_wire_changes_nothing_but_the_status() {
        let c = content();
        let t = team(&[("rook", 1), ("vela", 1), ("pip", 1)]);
        let mut host = Session::host("b", 3);
        host.peer(2, &t);
        host.message(&c, 2, "{nope", &t);
        assert_eq!(host.status, "online_garbled");
        assert!(host.foe.is_none());
        host.message(&c, 9, "{}", &t);
        assert_eq!(host.status, "online_garbled", "a stranger's message is ignored");
    }
}
