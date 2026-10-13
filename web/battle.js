// Plays back a fight the core has already decided (crates/core/src/battle.rs)
// on the 3D stage (crates/core/src/stage.rs) and the tiles either side of it.
// One clock runs at the chosen speed; the core's timeline says when each turn
// starts and when its hits land, and this file moves those values onto the
// screen and plays the sounds as the clock passes them. Nothing here changes
// how a fight comes out.
import { t, portrait, kindIcon, backdropArt } from "./app.js";
import * as sound from "./sound.js";

const el = (id) => document.getElementById(id);

let run = 0; // the current playback; an older loop that sees a newer run stops

function make(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

function fighter(card, side, slot) {
  const f = make("div", "fighter " + card.kind);
  f.id = "f-" + side + "-" + slot;
  const art = make("img", "f-art");
  art.src = portrait(card.id);
  art.alt = "";
  const kind = make("img", "f-kind");
  kind.src = kindIcon(card.kind);
  kind.alt = "";
  const name = make("div", "f-name");
  name.append(make("b", "", card.name), make("span", "badge", t("level", { level: card.level })));
  const hp = make("div", "bar hp");
  hp.append(make("i"));
  const hpText = make("div", "f-hp small");
  const gauge = make("div", "bar gauge");
  gauge.append(make("i"));
  const skill = make("div", "f-skill small", card.skill);
  const tags = make("div", "f-tags");
  f.append(art, kind, name, hp, hpText, gauge, skill, tags);
  return f;
}

function paint(states, teams) {
  states.forEach((side, s) => {
    side.forEach((st, k) => {
      const f = el("f-" + s + "-" + k);
      if (!f) return;
      f.querySelector(".hp i").style.width = st.hp_pct + "%";
      f.querySelector(".gauge i").style.width = st.gauge_pct + "%";
      f.querySelector(".f-hp").textContent = st.hp + " / " + teams[s][k].hp;
      f.classList.toggle("down", !st.alive);
      f.classList.toggle("ready", st.ready);
      const tags = f.querySelector(".f-tags");
      tags.replaceChildren();
      if (st.stunned) tags.append(make("span", "tag stun", t("stunned_tag")));
      if (st.ready) tags.append(make("span", "tag ready", t("ready_tag")));
    });
  });
}

// The words and class of a number that pops up for a hit.
function hitText(hit) {
  if (hit.hasted) return ["»", "float haste"];
  if (hit.heal) return ["+" + hit.amount, "float heal"];
  let cls = "float";
  if (hit.edge > 0) cls += " good";
  if (hit.edge < 0) cls += " bad";
  return [(hit.crit ? t("crit") + " " : "") + hit.amount, cls];
}

function pop(parent, hit, display, at) {
  const [text, cls] = hitText(hit);
  const n = make("span", cls, text);
  if (!hit.heal && !hit.hasted && hit.edge !== 0) n.append(make("small", "", hit.edge > 0 ? t("good_hit") : t("bad_hit")));
  if (hit.slowed) n.append(make("small", "", t("slowed_tag")));
  if (at) {
    n.style.left = at[0] + "%";
    n.style.top = at[1] + "%";
  }
  parent.append(n);
  setTimeout(() => n.remove(), display.float_ms);
}

function logLine(turn, teams) {
  const actor = teams[turn.side][turn.slot].name;
  if (turn.act === "stunned") return t("log_stunned", { actor });
  if (turn.act === "skill") return t("log_skill", { actor, skill: turn.skill });
  const hit = turn.hits[0];
  if (!hit) return actor;
  const line = t("log_attack", { actor, target: teams[hit.side][hit.slot].name, amount: hit.amount });
  return hit.ko ? line + " · " + t("log_ko", { target: teams[hit.side][hit.slot].name }) : line;
}

function speeds(display, onChange) {
  const box = el("a-speeds");
  box.replaceChildren();
  let current = display.speeds[0];
  try {
    const saved = localStorage.getItem(display.speed_key);
    if (display.speeds.map(String).includes(saved)) current = display.speeds[display.speeds.map(String).indexOf(saved)];
  } catch (e) {
    // no storage: the first speed
  }
  for (const s of display.speeds) {
    const b = make("button", "speed" + (s === current ? " on" : ""), "×" + s);
    b.addEventListener("click", () => {
      for (const o of box.children) o.classList.toggle("on", o === b);
      try {
        localStorage.setItem(display.speed_key, String(s));
      } catch (e) {
        // fine
      }
      onChange(s);
    });
    box.append(b);
  }
  return current;
}

const START_SOUND = { charge: "swing", shoot: "shoot", spell: "cast", support: "cast", dazed: "stun" };

function hitSound(hit) {
  if (hit.heal) return "heal";
  if (hit.hasted) return "haste";
  if (hit.slowed) return "slow";
  if (hit.stun) return "stun";
  if (hit.crit) return "crit";
  if (hit.edge > 0) return "strong";
  if (hit.edge < 0) return "weak";
  return "punch";
}

// `stage` is { renderer, N, game } or null when the browser has no WebGL 2.
export function playFight(view, display, stage, done) {
  run = run + 1;
  const me = run;
  const rec = view.record;
  el("backdrop").src = backdropArt(view.backdrop);
  el("a-title").textContent = view.title;
  el("a-banner").hidden = true;
  el("a-call").hidden = true;
  el("a-log").textContent = "";
  el("stage-floats").replaceChildren();
  for (const s of [0, 1]) {
    const side = el("side-" + s);
    side.replaceChildren(make("div", "side-label", s ? t("battle_foe") : t("battle_you")), ...rec.teams[s].map((c, k) => fighter(c, s, k)));
  }
  paint(rec.start, rec.teams);
  let speed = speeds(display, (s) => (speed = s));
  const timeline = JSON.parse(stage.game.stage_begin()).ok;
  const beats = timeline.beats;
  const canvas = el("stage");
  const gl = stage.renderer;
  canvas.hidden = !gl;
  if (gl) gl.setScene(stage.game.stage_scene_values(), stage.game.stage_scene_draws(), stage.game.stage_colours());

  let clock = 0;
  let last = performance.now();
  let started = 0;
  let landed = 0;
  let finished = false;
  let quiet = false;
  let callTimer = null;

  function markAt(side, slot, marks) {
    const at = ((side ? rec.teams[0].length : 0) + slot) * stage.N.marks;
    return [marks[at], marks[at + 1]];
  }

  function onStart(i) {
    const turn = rec.turns[i];
    for (const f of document.querySelectorAll(".fighter.acting")) f.classList.remove("acting");
    const actor = el("f-" + turn.side + "-" + turn.slot);
    if (actor) actor.classList.add("acting");
    el("a-log").textContent = logLine(turn, rec.teams);
    if (quiet) return;
    sound.play(START_SOUND[beats[i].style]);
    if (turn.act === "skill") {
      sound.play("skill");
      const call = el("a-call");
      call.textContent = t("callout", { actor: rec.teams[turn.side][turn.slot].name, skill: turn.skill });
      call.className = "callout " + (turn.side ? "foe" : "mine");
      call.hidden = false;
      if (callTimer) clearTimeout(callTimer);
      callTimer = setTimeout(() => (call.hidden = true), display.callout_ms / speed);
    }
  }

  function onLand(i, marks) {
    const turn = rec.turns[i];
    paint(turn.after, rec.teams);
    if (quiet) return;
    if (beats[i].style === "spell") sound.play("burst");
    for (const h of turn.hits) {
      const f = el("f-" + h.side + "-" + h.slot);
      if (f) {
        pop(f, h, display, null);
        if (!h.heal && !h.hasted) {
          f.classList.remove("hit");
          void f.offsetWidth;
          f.classList.add("hit");
        }
      }
      if (marks) pop(el("stage-floats"), h, display, markAt(h.side, h.slot, marks));
      sound.play(hitSound(h));
      if (h.ko) sound.play("ko");
    }
  }

  function finish() {
    if (finished) return;
    finished = true;
    const final = rec.turns.length ? rec.turns[rec.turns.length - 1].after : rec.start;
    paint(final, rec.teams);
    for (const f of document.querySelectorAll(".fighter.acting")) f.classList.remove("acting");
    const banner = el("a-banner");
    banner.textContent = rec.winner === null ? t("drawn") : rec.winner === 0 ? t("won") : t("lost");
    banner.className = "banner " + (rec.winner === 0 ? "good" : rec.winner === null ? "" : "bad");
    banner.hidden = false;
    sound.play(rec.winner === 0 ? "victory" : "defeat");
    if (rec.survivors[0] && rec.survivors[1]) el("a-log").textContent = t("turn_limit");
    document.body.dataset.fight = "over";
    setTimeout(() => me === run && done(), display.result_delay_ms);
  }

  function draw() {
    if (!gl) return null;
    const ratio = window.devicePixelRatio || 1;
    const w = canvas.clientWidth * ratio;
    const h = canvas.clientHeight * ratio;
    if (canvas.width !== w || canvas.height !== h) gl.resize(w, h);
    stage.game.stage_frame(clock, w, h);
    gl.frame(stage.game.stage_view(), stage.game.stage_values(), stage.game.stage_draws());
    document.body.dataset.stage = "drawn";
    return stage.game.stage_marks();
  }

  function tick(now) {
    if (me !== run || finished) return;
    clock = clock + (now - last) * speed;
    last = now;
    const marks = draw();
    while (started < beats.length && clock >= beats[started].start) onStart((started = started + 1) - 1);
    while (landed < beats.length && clock >= beats[landed].hit) onLand((landed = landed + 1) - 1, marks);
    if (clock >= timeline.total) {
      finish();
      return;
    }
    requestAnimationFrame(tick);
  }

  el("a-skip").onclick = () => {
    if (finished) return;
    quiet = true;
    clock = timeline.total;
    while (started < beats.length) onStart((started = started + 1) - 1);
    while (landed < beats.length) onLand((landed = landed + 1) - 1, null);
    draw();
    finish();
  };
  document.body.dataset.fight = "playing";
  requestAnimationFrame(tick);
}
