// Plays back a fight the core has already decided (crates/core/src/battle.rs).
// Each turn names who acted, what it hit and the state of every unit after
// it; this file only moves those values onto the screen, at a pace from
// data/display.json. Nothing here changes how a fight comes out.
import { t, portrait, kindIcon, backdropArt } from "./app.js";

const el = (id) => document.getElementById(id);

let timer = null;
let skipTo = null;

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
  const tags = make("div", "f-tags");
  f.append(art, kind, name, hp, hpText, gauge, tags);
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

function float(hit, display) {
  const f = el("f-" + hit.side + "-" + hit.slot);
  if (!f) return;
  let text = String(hit.amount);
  let cls = "float";
  if (hit.hasted) {
    text = "»";
    cls += " haste";
  } else if (hit.heal) {
    text = "+" + hit.amount;
    cls += " heal";
  } else {
    if (hit.crit) text = t("crit") + " " + text;
    if (hit.edge > 0) cls += " good";
    if (hit.edge < 0) cls += " bad";
    f.classList.remove("hit");
    void f.offsetWidth;
    f.classList.add("hit");
  }
  const n = make("span", cls, text);
  if (!hit.heal && !hit.hasted && hit.edge !== 0) n.append(make("small", "", hit.edge > 0 ? t("good_hit") : t("bad_hit")));
  f.append(n);
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
      current = s;
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

export function playFight(view, display, done) {
  if (timer) clearTimeout(timer);
  const rec = view.record;
  el("backdrop").src = backdropArt(view.backdrop);
  el("a-title").textContent = view.title;
  el("a-banner").hidden = true;
  el("a-log").textContent = "";
  for (const s of [0, 1]) {
    const side = el("side-" + s);
    side.replaceChildren(make("div", "side-label", s ? t("battle_foe") : t("battle_you")), ...rec.teams[s].map((c, k) => fighter(c, s, k)));
  }
  paint(rec.start, rec.teams);
  let speed = speeds(display, (s) => (speed = s));
  let i = 0;
  let finished = false;

  function finish() {
    if (finished) return;
    finished = true;
    if (timer) clearTimeout(timer);
    const last = rec.turns[rec.turns.length - 1];
    if (last) paint(last.after, rec.teams);
    const banner = el("a-banner");
    const word = rec.winner === null ? t("drawn") : rec.winner === 0 ? t("won") : t("lost");
    banner.textContent = word;
    banner.className = "banner " + (rec.winner === 0 ? "good" : rec.winner === null ? "" : "bad");
    banner.hidden = false;
    if (rec.survivors[0] && rec.survivors[1]) el("a-log").textContent = t("turn_limit");
    document.body.dataset.fight = "over";
    timer = setTimeout(done, display.result_delay_ms);
  }
  skipTo = finish;
  el("a-skip").onclick = () => skipTo && skipTo();

  function step() {
    if (finished) return;
    if (i >= rec.turns.length) {
      finish();
      return;
    }
    const turn = rec.turns[i];
    i = i + 1;
    for (const f of document.querySelectorAll(".fighter.acting")) f.classList.remove("acting");
    const actor = el("f-" + turn.side + "-" + turn.slot);
    if (actor) actor.classList.add("acting");
    if (turn.act === "skill" && actor) {
      const call = make("span", "skill-call", turn.skill);
      actor.append(call);
      setTimeout(() => call.remove(), display.float_ms);
    }
    el("a-log").textContent = logLine(turn, rec.teams);
    timer = setTimeout(() => {
      if (finished) return;
      for (const h of turn.hits) float(h, display);
      paint(turn.after, rec.teams);
      timer = setTimeout(step, display.turn_ms / speed - display.hit_ms / speed);
    }, display.hit_ms / speed);
  }
  document.body.dataset.fight = "playing";
  timer = setTimeout(step, display.hit_ms);
}
