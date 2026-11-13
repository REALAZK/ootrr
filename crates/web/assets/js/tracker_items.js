import { $, h, fill } from "./dom.js";
import { get, get_blob, q } from "./api.js";
import { SIDE_KEY, fold_parse, fold_text, clean_shut, clean_marks, next_level, owned } from "./tracker_fold.js";

const KEY = "ootrr.tracker.items.v1";
const LANES = 4;
const SECTIONS = ["tools", "spells", "bottles", "gear", "hearts", "songs", "dungeons"];

const it = {
  open: false,
  small: false,
  side: true,
  shut: {},
  data: null,
  error: "",
  art: new Map(),
  missed: new Set(),
  art_key: "",
  busy: false,
  manual: {},
  tags: {},
  timer: 0,
  again: false,
  loading: false,
};

let repaint_tools = () => {};

function remember() {
  try {
    localStorage.setItem(KEY, JSON.stringify({ open: it.open, small: it.small, shut: it.shut, manual: it.manual, tags: it.tags }));
  } catch {
    return;
  }
}

function recall() {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) || "null");
    if (!v || typeof v !== "object") return;
    it.open = v.open === true;
    it.small = v.small === true;
    it.shut = clean_shut(v.shut, SECTIONS);
    it.manual = clean_marks(v.manual, 200);
    for (const [k, t] of Object.entries(v.tags || {}).slice(0, 50)) {
      if (typeof t === "string") it.tags[k.slice(0, 40)] = t.slice(0, 10);
    }
  } catch {
    it.open = false;
  }
}

function recall_side() {
  let raw = null;
  try {
    raw = localStorage.getItem(SIDE_KEY);
  } catch {
    raw = null;
  }
  it.side = fold_parse(raw, window.innerWidth);
}

function fold_button(open, what, fn) {
  return h("button", { type: "button", class: "fold", "aria-expanded": String(open), "aria-label": `${open ? "Collapse" : "Expand"} ${what}`, title: `${open ? "Collapse" : "Expand"} ${what}`, onclick: fn }, h("i", { class: "chev", "aria-hidden": "true" }));
}

function paint_side() {
  const app = $(".app");
  const side = $(".side");
  if (!app || !side) return;
  app.classList.toggle("side-off", !it.side);
  let head = $(".sidehead", side);
  if (!head) {
    head = h("div", { class: "sidehead" });
    side.prepend(head);
  }
  fill(head, h("span", { class: "ph" }, "Locations"), fold_button(it.side, "the location list", toggle_side));
}

function toggle_side() {
  it.side = !it.side;
  try {
    localStorage.setItem(SIDE_KEY, fold_text(it.side));
  } catch {
    it.side = Boolean(it.side);
  }
  paint_side();
  window.dispatchEvent(new Event("resize"));
}

export function init_items() {
  recall();
  recall_side();
  paint_side();
  paint_items();
}

function live() {
  return Boolean(it.data && it.data.live && it.data.state);
}

function state_of(c) {
  if (live()) return it.data.state[c.id] || { lv: 0, n: null, part: null };
  return { lv: Math.min(c.max, it.manual[c.id] || 0), n: null, part: null };
}

function icon_of(c, st) {
  return c.icons[Math.min(c.icons.length, Math.max(st.lv, 1)) - 1];
}

function tag_of(c) {
  return it.tags[c.id] || c.tag;
}

function plain(c) {
  if (c.slot === 2) return "Keys";
  if (c.slot === 3) return "Boss key";
  if (c.slot === 1) return c.label.endsWith("Medallion") ? "Medal" : "Stone";
  return c.label;
}

function words(c, st) {
  const parts = [c.label];
  if (c.max > 1) parts.push(`level ${st.lv} of ${c.max}`);
  else parts.push(st.lv > 0 ? "found" : "not found");
  if (st.n !== null && st.n !== undefined) parts.push(c.id === "hearts" ? `${st.n} hearts` : `${st.n} held`);
  if (st.part) parts.push(`${st.part} of 4 heart pieces`);
  if (!live()) parts.push("click to change");
  return parts.join(", ");
}

function bump(c) {
  if (live()) return;
  const next = next_level(it.manual[c.id], c.max);
  if (next) it.manual[c.id] = next;
  else delete it.manual[c.id];
  remember();
  paint_items();
}

function next_tag(c) {
  const all = (it.data && it.data.layout.reward_tags) || [];
  if (!all.length) return;
  const t = all[(all.indexOf(tag_of(c)) + 1) % all.length];
  if (t === c.tag) delete it.tags[c.id];
  else it.tags[c.id] = t;
  remember();
  paint_items();
}

function picture(bmp) {
  const cv = h("canvas", { width: bmp.width, height: bmp.height, "aria-hidden": "true" });
  const g = cv.getContext("2d");
  if (g) g.drawImage(bmp, 0, 0);
  return cv;
}

function pips(c, st) {
  if (c.max < 2 || c.icons.length > 1) return null;
  const dots = [];
  for (let i = 1; i <= c.max; i += 1) dots.push(h("i", { class: i <= st.lv ? "lit" : null }));
  return h("span", { class: "pips", "aria-hidden": "true" }, dots);
}

function tile(c) {
  const st = state_of(c);
  const on = st.lv > 0;
  const bmp = it.art.get(icon_of(c, st));
  const count = c.count && st.n !== null && st.n !== undefined && (on || st.n > 0);
  return h(
    "button",
    {
      type: "button",
      class: `it ${on ? "on" : "off"}${bmp ? "" : " txt"}`,
      "data-id": c.id,
      "data-lv": String(st.lv),
      title: words(c, st),
      "aria-label": words(c, st),
      "aria-pressed": String(on),
      "aria-disabled": live() ? "true" : null,
      onclick: () => bump(c),
    },
    bmp ? picture(bmp) : h("span", { class: "nm" }, plain(c)),
    pips(c, st),
    count ? h("b", { class: "n" }, String(st.n)) : null,
    st.part ? h("b", { class: "pt" }, `+${st.part}`) : null,
  );
}

function dungeon_rows(cells) {
  const lay = it.data.layout;
  return h(
    "div",
    { class: "drows" },
    (lay.rows || []).map((name) => {
      const mine = cells.filter((c) => c.group === name);
      const reward = mine.find((c) => c.reward);
      const slots = [];
      for (let s = 1; s <= (lay.row_slots || 3); s += 1) {
        const c = mine.find((x) => x.slot === s);
        slots.push(c ? tile(c) : h("span", { class: "gap", "aria-hidden": "true" }));
      }
      return h(
        "div",
        { class: "drow", "data-row": name },
        h(
          "div",
          { class: "dn" },
          h("span", null, name),
          reward ? h("button", { type: "button", class: "inote", "data-for": reward.id, title: "Your note of where this reward is. Click to change it.", onclick: () => next_tag(reward) }, tag_of(reward)) : null,
        ),
        slots,
      );
    }),
  );
}

function flip(id) {
  if (it.shut[id]) delete it.shut[id];
  else it.shut[id] = true;
  remember();
  paint_items();
}

function section(s) {
  const cells = it.data.layout.cells.filter((c) => c.sec === s.id);
  const open = !it.shut[s.id];
  const head = h(
    "button",
    { type: "button", class: "sh", "aria-expanded": String(open), onclick: () => flip(s.id) },
    h("i", { class: "chev", "aria-hidden": "true" }),
    h("span", { class: "st" }, s.title),
    h("span", { class: "sc" }, `${owned(cells, state_of)} / ${cells.length}`),
  );
  let body = null;
  if (open) body = s.id === "dungeons" ? dungeon_rows(cells) : h("div", { class: "tiles" }, cells.map(tile));
  return h("section", { class: `sec s-${s.id}${open ? "" : " shut"}`, "aria-label": s.title }, head, body);
}

function status() {
  if (it.error) return it.error;
  if (!it.data) return "Loading";
  if (live()) return `From your save${it.data.slot ? `, ${it.data.slot.replace("_", " ")}` : ""}`;
  return "No save read yet. Click a tile to mark it by hand.";
}

function art_note() {
  if (!it.data) return null;
  const a = it.data.art || {};
  if (!a.ready) return "Set your game folder under Folders to see the game's own icons. They are read from your own files and are never part of this tool.";
  if (it.busy && it.art.size === 0) return "Reading the icons from your game folder";
  if (it.art.size === 0 && it.missed.size > 0) return "The icons could not be read from your game folder, so the tiles show names.";
  if (it.missed.size > 0) return `${it.missed.size} icons could not be read, those tiles show names.`;
  return null;
}

export function paint_items() {
  const box = $("#items");
  if (!box) return;
  box.hidden = !it.open;
  box.classList.toggle("mini", it.small);
  if (!it.open) return;
  const head = h("header", { class: "ih" }, h("span", { class: "ph" }, "Items"), h("span", { class: `isrc${live() ? " on" : ""}` }, status()), fold_button(!it.small, "the item panel", shrink));
  if (!it.data || it.small) {
    fill(box, head);
    return;
  }
  const note = art_note();
  fill(
    box,
    head,
    h("div", { class: "secs" }, it.data.layout.sections.map(section)),
    note ? h("p", { class: "fine note" }, note) : null,
    live() ? null : h("p", { class: "fine note" }, "Marks made by hand stay in this browser. A save that is read replaces them on screen."),
  );
}

function later() {
  clearTimeout(it.timer);
  it.timer = setTimeout(paint_items, 120);
}

async function lane(todo, key) {
  while (todo.length && it.art_key === key) {
    const id = todo.pop();
    try {
      const bmp = await createImageBitmap(await get_blob(`/api/tracker/icon?id=${q(id)}`));
      if (it.art_key !== key) return;
      it.art.set(id, bmp);
      later();
    } catch {
      if (it.art_key === key) it.missed.add(id);
    }
  }
}

async function fetch_art() {
  const a = (it.data && it.data.art) || {};
  const key = a.ready ? a.key : "";
  if (key !== it.art_key) {
    it.art_key = key;
    it.art.clear();
    it.missed.clear();
  }
  if (!key || it.busy || !it.open) return;
  const want = new Set();
  for (const c of it.data.layout.cells) for (const i of c.icons) want.add(i);
  const todo = [...want].filter((i) => !it.art.has(i) && !it.missed.has(i)).reverse();
  if (!todo.length) return;
  it.busy = true;
  const lanes = [];
  for (let i = 0; i < LANES; i += 1) lanes.push(lane(todo, key));
  await Promise.all(lanes);
  it.busy = false;
  paint_items();
}

export async function load_items() {
  if (!it.open) return;
  if (it.loading) {
    it.again = true;
    return;
  }
  it.loading = true;
  try {
    it.data = await get("/api/tracker/items");
    it.error = "";
  } catch (e) {
    it.error = e.message;
  }
  it.loading = false;
  paint_items();
  fetch_art();
  if (it.again) {
    it.again = false;
    load_items();
  }
}

function shrink() {
  it.small = !it.small;
  remember();
  paint_items();
}

function toggle() {
  it.open = !it.open;
  if (it.open) it.small = false;
  remember();
  paint_items();
  repaint_tools();
  if (it.open) load_items();
}

export function items_button(repaint) {
  repaint_tools = repaint;
  return h("button", { type: "button", class: "btn", "aria-pressed": String(it.open), title: "Show or hide the item panel", onclick: toggle }, "Items");
}
