// The page: fetches the data, hands it to the core, and draws what the core
// returns. Which screen is showing, which slot is picked and how fast a fight
// plays back are the page's own business; every rule, number and percentage
// comes from the core (crates/core/src/game.rs).
import init, { Game } from "./pkg/lt_shim.js";
import { makeOnline } from "./online.js";
import { playFight } from "./battle.js";

const BUILD = "__BUILD__";
const el = (id) => document.getElementById(id);

let game = null;
let COPY = {};
let DISPLAY = {};
let online = null;
let pollTimer = null;
let lastFight = null; // what "fight again" repeats
let rosterSlot = 0;
let rosterPick = null;
let chapterShown = 0;

// ---- text -----------------------------------------------------------------

export function t(key, vars) {
  let s = COPY[key] === undefined ? key : COPY[key];
  for (const k in vars || {}) s = s.split("{" + k + "}").join(String(vars[k]));
  return s;
}

function call(text) {
  return JSON.parse(text);
}

function make(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

function img(src, cls) {
  const i = make("img", cls);
  i.src = src;
  i.alt = "";
  i.decoding = "async";
  return i;
}

export const portrait = (id) => "art/units/" + id + ".png";
export const kindIcon = (kind) => "art/kinds/" + kind + ".png";
export const backdropArt = (name) => "art/backdrops/" + name + ".png";

function kindTag(kind) {
  const s = make("span", "kind " + kind);
  s.append(img(kindIcon(kind), "kind-icon"), make("span", "", t("kind_" + kind)));
  return s;
}

function bar(cls, pct) {
  const b = make("div", "bar " + cls);
  const i = make("i");
  i.style.width = pct + "%";
  b.append(i);
  return b;
}

function save() {
  try {
    localStorage.setItem(DISPLAY.save_key, game.save());
  } catch (e) {
    // a browser that refuses storage still plays; it just forgets
  }
}

// ---- screens ----------------------------------------------------------------

const SCREENS = ["menu", "roster", "campaign", "online", "battle", "result", "help"];

function show(mode) {
  document.body.dataset.mode = mode;
  for (const s of SCREENS) el(s).hidden = s !== mode;
  el("loading").hidden = true;
  window.scrollTo(0, 0);
  if (mode !== "online" && mode !== "battle" && mode !== "result") stopPolling();
}

function miniUnit(u) {
  const m = make("div", "mini " + u.kind);
  m.append(img(portrait(u.id), "mini-art"));
  const cap = make("div", "mini-cap");
  cap.append(make("b", "", u.name), make("span", "badge", t("level", { level: u.level })));
  m.append(cap, img(kindIcon(u.kind), "mini-kind"));
  return m;
}

function renderMenu() {
  const h = call(game.home()).ok;
  el("m-level").textContent = h.level;
  el("m-xp").textContent = h.maxed ? t("max") : h.xp + " / " + h.need + " " + t("xp");
  el("m-xpbar").style.width = h.xp_pct + "%";
  el("m-next").textContent = h.next_unlock ? t("next_unlock", h.next_unlock) : t("all_unlocked");
  el("m-record").textContent = t("record", h);
  el("m-team").replaceChildren(...h.team.map(miniUnit));
  show("menu");
}

// ---- crew -----------------------------------------------------------------

function statLine(label, now, next, maxed) {
  const r = make("div", "stat");
  r.append(make("span", "", label), make("b", "", String(now)));
  if (!maxed) r.append(make("span", "up", "→ " + next));
  return r;
}

function renderDetail(u) {
  const d = el("r-detail");
  d.replaceChildren();
  if (!u) return;
  d.append(img(portrait(u.id), "detail-art " + u.kind));
  const head = make("div", "detail-head");
  head.append(make("h3", "", u.name), make("span", "badge", t("level", { level: u.level })));
  d.append(head, make("p", "muted", u.title), kindTag(u.kind));
  const edge = make("p", "small");
  edge.append(make("span", "good", t("beats", { kind: t("kind_" + u.beats) })), make("span", "", " · "), make("span", "bad", t("weak", { kind: t("kind_" + u.weak_to) })));
  d.append(edge);
  if (u.locked) d.append(make("p", "locked-note", t("locked", { level: u.unlock })));
  d.append(make("p", "", u.blurb));
  const xp = make("div", "xp-line");
  xp.append(make("span", "small muted", u.maxed ? t("unit_max") : u.xp + " / " + u.need + " " + t("xp")), bar("xp", u.xp_pct));
  d.append(xp);
  const stats = make("div", "stats");
  stats.append(
    statLine(t("stat_hp"), u.stats.hp, u.next.hp, u.maxed),
    statLine(t("stat_atk"), u.stats.atk, u.next.atk, u.maxed),
    statLine(t("stat_spd"), u.stats.spd, u.next.spd, u.maxed),
  );
  if (!u.maxed) stats.append(make("p", "muted small", "→ " + t("next_level")));
  d.append(stats);
  const sk = make("div", "skill");
  sk.append(make("b", "", u.skill.name), make("span", "muted small", " · " + t("skill_cooldown", { n: u.skill.cooldown })));
  sk.append(make("p", "small", t("effect_" + u.skill.effect, u.skill)));
  d.append(sk);
}

function renderRoster() {
  const all = call(game.roster()).ok;
  const team = call(game.home()).ok.team.map((u) => u.id);
  const byId = {};
  for (const u of all) byId[u.id] = u;
  const slots = el("r-slots");
  slots.replaceChildren();
  team.forEach((id, i) => {
    const b = make("button", "slot" + (i === rosterSlot ? " picked" : ""));
    b.append(miniUnit(byId[id]));
    b.addEventListener("click", () => {
      rosterSlot = i;
      rosterPick = id;
      renderRoster();
    });
    slots.append(b);
  });
  const grid = el("r-grid");
  grid.replaceChildren();
  for (const u of all) {
    const c = make("button", "unit-card " + u.kind + (u.locked ? " locked" : "") + (u.in_team ? " chosen" : "") + (u.id === rosterPick ? " looking" : ""));
    c.dataset.unit = u.id;
    c.append(img(portrait(u.id), "card-art"));
    const cap = make("div", "card-cap");
    cap.append(make("b", "", u.name), make("span", "badge" + (u.locked ? " lock" : ""), u.locked ? t("locked_badge", { level: u.unlock }) : t("level", { level: u.level })));
    c.append(cap, img(kindIcon(u.kind), "card-kind"));
    if (!u.locked) c.append(bar("xp thin", u.xp_pct));
    c.addEventListener("click", () => pickUnit(u, team));
    grid.append(c);
  }
  renderDetail(byId[rosterPick] || byId[team[rosterSlot]]);
  show("roster");
}

function pickUnit(u, team) {
  rosterPick = u.id;
  el("r-error").textContent = "";
  if (!u.locked && !u.in_team) {
    const next = team.slice();
    next[rosterSlot] = u.id;
    const r = call(game.set_team(JSON.stringify(next)));
    if (r.error) el("r-error").textContent = t(r.error);
    else save();
  } else if (u.locked) {
    el("r-error").textContent = t("team_locked");
  }
  renderRoster();
}

// ---- campaign -----------------------------------------------------------------

function renderCampaign() {
  const chapters = call(game.campaign()).ok;
  const tabs = el("c-tabs");
  tabs.replaceChildren();
  chapters.forEach((ch, i) => {
    const b = make("button", "tab" + (i === chapterShown ? " on" : "") + (ch.open ? "" : " shut"));
    b.append(img(backdropArt(ch.backdrop), "tab-art"), make("span", "", t("chapter", { n: i + 1 }) + " · " + ch.name));
    b.addEventListener("click", () => {
      chapterShown = i;
      renderCampaign();
    });
    tabs.append(b);
  });
  const ch = chapters[chapterShown];
  const box = el("c-chapter");
  box.replaceChildren();
  box.style.backgroundImage = "url(" + backdropArt(ch.backdrop) + ")";
  box.append(make("p", "chapter-blurb", ch.blurb));
  const list = make("div", "stages");
  for (const st of ch.stages) {
    const card = make("div", "stage card" + (st.open ? "" : " shut") + (st.cleared ? " cleared" : ""));
    const head = make("div", "stage-head");
    head.append(make("b", "", st.name), make("span", "stars", "★".repeat(st.stars) + "☆".repeat(st.stars_missing)));
    card.append(head);
    const foes = make("div", "mini-team small-team");
    foes.append(...st.enemies.map(miniUnit));
    card.append(foes, make("p", "muted small", t("stage_reward", st)));
    const go = make("button", "primary", st.open ? t("stage_fight") : t("stage_locked_label"));
    go.disabled = !st.open;
    go.dataset.stage = st.index;
    go.addEventListener("click", () => startCampaign(st.index));
    card.append(go);
    list.append(card);
  }
  box.append(list);
  show("campaign");
}

function startCampaign(stage) {
  const r = call(game.fight_campaign(stage, Date.now()));
  if (r.error) {
    el("c-error").textContent = t(r.error);
    return;
  }
  save();
  lastFight = () => startCampaign(stage);
  runFight(r.ok);
}

function startRandom() {
  const r = call(game.fight_random(Date.now()));
  save();
  lastFight = startRandom;
  runFight(r.ok);
}

function runFight(view) {
  show("battle");
  playFight(view, DISPLAY, () => renderResult(view));
}

// ---- results -----------------------------------------------------------------

function renderResult(view) {
  const r = view.reward;
  const rec = view.record;
  const head = rec.winner === null ? t("drawn") : r.won ? t("won") : t("lost");
  el("x-head").textContent = head;
  el("x-head").className = rec.winner === null ? "" : r.won ? "good" : "bad";
  el("x-stars").textContent = view.mode === "campaign" && r.won ? "★".repeat(r.stars) + "☆".repeat(r.stars_missing) : "";
  el("x-first").hidden = !r.first_clear;
  el("x-level").textContent = r.level_after;
  el("x-account").textContent = t("account_gain", { xp: r.account_xp });
  el("x-xpbar").style.width = r.xp_pct + "%";
  el("x-levelup").textContent = r.level_after !== r.level_before ? t("account_level_up", { after: r.level_after }) : "";
  const unlocks = el("x-unlocks");
  unlocks.replaceChildren();
  const roster = call(game.roster()).ok;
  for (const id of r.unlocked) {
    const u = roster.find((x) => x.id === id);
    const li = make("li", "unlock " + u.kind);
    li.append(img(portrait(id), "mini-art"), make("span", "", t("unlocked_now", { name: u.name })));
    unlocks.append(li);
  }
  const units = el("x-units");
  units.replaceChildren();
  for (const u of r.units) {
    const row = make("div", "x-unit");
    row.append(img(portrait(u.id), "mini-art"));
    const mid = make("div", "grow");
    mid.append(make("b", "", u.name + " "), make("span", "muted small", t("unit_gain", { xp: u.xp })));
    if (u.level_after !== u.level_before) mid.append(make("span", "levelup small", " " + t("level_up", { before: u.level_before, after: u.level_after })));
    mid.append(bar("xp thin", u.xp_pct));
    row.append(mid, make("span", "badge", t("level", { level: u.level_after })));
    units.append(row);
  }
  el("x-note").textContent = view.mode === "random" ? t("random_note") : "";
  const next = el("x-next");
  next.hidden = !(view.mode === "campaign" && view.next_stage_open);
  next.onclick = () => startCampaign(view.stage + 1);
  const again = el("x-again");
  again.hidden = view.mode === "online";
  again.textContent = view.mode === "random" ? t("again_random") : t("retry");
  again.onclick = () => lastFight && lastFight();
  const back = el("x-back");
  back.textContent = view.mode === "campaign" ? t("to_campaign") : view.mode === "online" ? t("to_online") : t("to_menu");
  back.onclick = () => (view.mode === "campaign" ? renderCampaign() : view.mode === "online" ? showOnline() : renderMenu());
  show("result");
}

// ---- online -----------------------------------------------------------------

function stopPolling() {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = null;
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(tickOnline, DISPLAY.online_poll_ms);
}

function tickOnline() {
  if (!online) return;
  online.pump();
  const fight = call(game.online_take()).ok;
  if (fight) {
    save();
    online.pump();
    runFight(fight);
    return;
  }
  if (document.body.dataset.mode === "online") renderOnline();
}

function renderOnline() {
  const v = call(game.online_view()).ok;
  const on = v.role !== "off";
  el("o-start").hidden = on;
  el("o-room").hidden = !on;
  el("o-roomcode").textContent = v.room ? t("online_room", { room: v.room }) : "";
  el("o-share").hidden = !(v.role === "host" && v.room);
  el("o-status").textContent = t(v.status);
  el("o-status").dataset.status = v.status;
  el("o-mine").replaceChildren(...call(game.home()).ok.team.map(miniUnit));
  el("o-foe").replaceChildren(...v.foe.map(miniUnit));
  el("o-fights").textContent = v.fights ? t("online_fights", { n: v.fights }) : "";
  el("o-fight").hidden = v.role !== "host";
  el("o-fight").disabled = !v.can_start;
  el("o-paste-host").hidden = v.role === "joiner";
  el("o-paste-join").hidden = v.role === "host";
  document.body.dataset.online = v.status;
}

function showOnline() {
  show("online");
  renderOnline();
  startPolling();
}

function wireOnline() {
  online = makeOnline(game, BUILD);
  el("b-online").addEventListener("click", showOnline);
  el("o-back").addEventListener("click", () => {
    renderMenu();
  });
  el("o-host").addEventListener("click", () => {
    online.host("room");
    renderOnline();
  });
  el("o-join").addEventListener("click", () => {
    el("o-error").textContent = "";
    if (!online.join(el("o-code").value)) el("o-error").textContent = t("online_bad_code");
    renderOnline();
  });
  el("o-code").addEventListener("keydown", (e) => {
    if (e.key === "Enter") el("o-join").click();
  });
  el("o-leave").addEventListener("click", () => {
    online.leave();
    el("o-invite-out").value = "";
    el("o-reply-out").value = "";
    delete document.body.dataset.invite;
    delete document.body.dataset.reply;
    renderOnline();
  });
  el("o-fight").addEventListener("click", () => {
    game.online_start(Date.now());
    tickOnline();
  });
  el("o-new-invite").addEventListener("click", async () => {
    el("o-paste-error").textContent = "";
    if (!online.active()) online.host("code");
    renderOnline();
    el("o-invite-out").value = await online.invitation();
    document.body.dataset.invite = "ready";
  });
  el("o-accept").addEventListener("click", async () => {
    el("o-paste-error").textContent = "";
    try {
      await online.accept(el("o-reply-in").value);
    } catch (e) {
      el("o-paste-error").textContent = t("online_bad_paste");
    }
  });
  el("o-make-reply").addEventListener("click", async () => {
    el("o-paste-error").textContent = "";
    try {
      el("o-reply-out").value = await online.joinByCode(el("o-invite-in").value);
      document.body.dataset.reply = "ready";
    } catch (e) {
      el("o-paste-error").textContent = t("online_bad_paste");
    }
    renderOnline();
  });
}

// ---- start -----------------------------------------------------------------

async function text(path) {
  const r = await fetch(path);
  if (!r.ok) throw new Error(path + " " + r.status);
  return r.text();
}

async function start() {
  await init();
  const [units, campaign, copy, display] = await Promise.all(["data/units.json", "data/campaign.json", "data/copy.json", "data/display.json"].map(text));
  COPY = JSON.parse(copy);
  DISPLAY = JSON.parse(display);
  for (const e of document.querySelectorAll("[data-copy]")) e.textContent = t(e.dataset.copy);
  document.title = t("title");
  el("source").href = DISPLAY.source_url;
  let saved = "";
  try {
    saved = localStorage.getItem(DISPLAY.save_key) || "";
  } catch (e) {
    saved = "";
  }
  game = new Game(units, campaign, saved);
  el("o-code").placeholder = t("online_code_placeholder");
  for (const b of document.querySelectorAll("button.back")) b.addEventListener("click", renderMenu);
  el("b-campaign").addEventListener("click", () => {
    const chapters = call(game.campaign()).ok;
    chapterShown = chapters.reduce((last, ch, i) => (ch.open ? i : last), 0);
    renderCampaign();
  });
  el("b-random").addEventListener("click", startRandom);
  el("b-roster").addEventListener("click", () => {
    rosterSlot = 0;
    rosterPick = null;
    renderRoster();
  });
  el("b-help").addEventListener("click", () => show("help"));
  el("b-reset").addEventListener("click", () => {
    if (!window.confirm(t("reset_confirm"))) return;
    game.reset();
    save();
    renderMenu();
  });
  wireOnline();
  renderMenu();
  document.body.dataset.ready = "true";
}

start().catch((e) => {
  el("loading").textContent = (COPY.load_failed || "") + e;
  console.error(e);
});
