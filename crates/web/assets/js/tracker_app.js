import { $, h, fill, notice } from "./dom.js";
import { init_token, get } from "./api.js";
import { state } from "./store.js";
import { t_index, t_state, t_events } from "./tracker_api.js";
import { ts, on_change, set_index, set_state, upsert, mark_just, recall, remember, saved_theme, save_theme, THEMES } from "./tracker_store.js";
import { render_rail } from "./tracker_rail.js";
import { render_stage, set_open_settings, map_key } from "./tracker_map.js";
import { render_list, flip } from "./tracker_list.js";
import { render_card } from "./tracker_card.js";
import { render_filters, search_box, send_mode, set_reload as filters_reload } from "./tracker_filters.js";
import { paint_live, offline, open_source, refresh_source, set_reload as source_reload } from "./tracker_source.js";
import { announce } from "./tracker_news.js";
import { init_items, load_items, items_button } from "./tracker_items.js";

function fail(msg) {
  fill($("#stage"), h("div", { class: "float title" }, h("h1", null, "Tracker"), notice("error", msg)));
}

function paint_total() {
  const t = (ts.state && ts.state.total) || { found: 0, all: 0 };
  const fillbar = h("i");
  fillbar.style.width = `${t.all ? Math.round((100 * t.found) / t.all) : 0}%`;
  fill($("#total"), h("span", null, "Found"), h("b", null, `${t.found} / ${t.all}`), h("span", { class: "bar" }, fillbar));
}

function paint_tools() {
  const all = ts.mode === "all";
  const now = document.documentElement.getAttribute("data-theme");
  const theme = THEMES.includes(now) ? now : "hyrule";
  const next = THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length];
  fill(
    $("#tools"),
    h(
      "button",
      {
        type: "button",
        class: "switch",
        "aria-pressed": String(all),
        title: "Off: only what your save or your own ticks revealed is sent to this page. On: every location is listed.",
        onclick: () => send_mode({ mode: all ? "hidden" : "all" }),
      },
      h("span", { class: "tr" }),
      "Show all",
    ),
    items_button(paint_tools),
    h("button", { type: "button", class: "btn", onclick: open_source }, "Folders"),
    h(
      "button",
      {
        type: "button",
        class: "btn",
        "aria-label": `Theme: ${theme}. Switch to ${next}`,
        onclick: () => {
          document.documentElement.setAttribute("data-theme", next);
          save_theme(next);
          paint_tools();
        },
      },
      next[0].toUpperCase() + next.slice(1),
    ),
  );
}

function busy(box) {
  const a = document.activeElement;
  return Boolean(a && box.contains(a) && (a.tagName === "INPUT" || a.tagName === "SELECT"));
}

function paint() {
  fill($("#rail"), render_rail());
  render_stage($("#stage"));
  fill($("#filters"), render_filters());
  fill($("#list"), render_list());
  const card = $("#card");
  if (!busy(card)) fill(card, render_card());
  paint_total();
  paint_tools();
  paint_live();
}

async function reload() {
  const [ix, st] = await Promise.all([t_index(), t_state()]);
  set_index(ix);
  set_state(st);
  paint();
  refresh_source();
  load_items();
}

function apply(r) {
  let reset = false;
  const fresh = [];
  for (const e of r.events || []) {
    if (e.type === "reset") reset = true;
    else if (e.type === "loc") {
      upsert(e.loc);
      if (e.fresh) {
        mark_just(e.loc.id);
        fresh.push(e.loc);
      }
    } else if (e.type === "source" && ts.state) {
      ts.state.source = e.source;
      ts.state.player = e.player;
      ts.state.total = e.total;
      ts.state.places = e.places;
      refresh_source();
      load_items();
    }
  }
  ts.seq = r.seq;
  if (fresh.length) {
    announce(fresh);
    setTimeout(paint, 8500);
  }
  return reset || r.reset;
}

async function poll() {
  for (;;) {
    try {
      const r = await t_events(ts.seq);
      offline(false);
      if (apply(r)) await reload();
      else if ((r.events || []).length) paint();
    } catch {
      offline(true);
      await new Promise((ok) => setTimeout(ok, 3000));
    }
  }
}

function typing(e) {
  const t = e.target;
  return t && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA" || t.isContentEditable);
}

function keys(e) {
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  if (e.key === "Escape") {
    if (ts.tool || ts.sel) {
      ts.tool = "";
      ts.sel = "";
      paint();
    }
    return;
  }
  if (typing(e)) return;
  const age = { c: "child", a: "adult", b: "both" }[e.key];
  if (e.key === "/") {
    e.preventDefault();
    $("#trk-search").focus();
  } else if (age) {
    ts.age = age;
    remember();
    paint();
  } else if (e.key === " " && ts.sel && !ts.sel.startsWith("u:") && e.target.tagName !== "BUTTON") {
    const l = ts.locs.get(ts.sel);
    if (l) {
      e.preventDefault();
      flip(l.id, (l.st || {}).s !== "found");
    }
  } else if (map_key(e)) e.preventDefault();
}

async function boot() {
  document.documentElement.setAttribute("data-theme", saved_theme());
  if (!init_token()) {
    return fail("This page has no session token. Open the tracker from the link at the top of the main page, or open the link that ootrr serve printed and go to the tracker from there.");
  }
  recall();
  init_items();
  try {
    state.info = await get("/api/info");
    fill($("#searchbox"), search_box());
    filters_reload(reload);
    source_reload(reload);
    set_open_settings(open_source);
    on_change(paint);
    await reload();
  } catch (e) {
    return fail(e.message);
  }
  document.addEventListener("keydown", keys);
  setInterval(paint_live, 1000);
  const s = (ts.state && ts.state.source) || {};
  if (!s.asked && !s.game_dir) open_source();
  poll();
}

boot();
