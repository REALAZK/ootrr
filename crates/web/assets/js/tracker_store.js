const KEY = "ootrr.tracker.v1.vanilla";
const THEME_KEY = "ootrr.tracker.theme";
const MAX_PINS = 500;
const MAX_NOTES = 2000;

export const ts = {
  locs: new Map(),
  maps: new Map(),
  kinds: [],
  mode: "hidden",
  ready: false,
  state: null,
  seq: 0,
  search: "",
  place: "",
  age: "both",
  floor: "all",
  sel: "",
  tool: "",
  my_pins: true,
  pins: [],
  notes: {},
  views: {},
  just: new Map(),
  online: true,
};

const subs = new Set();

export function on_change(fn) {
  subs.add(fn);
  return () => subs.delete(fn);
}

export function emit() {
  for (const f of subs) f();
}

export function set_index(ix) {
  ts.locs.clear();
  for (const l of ix.locations || []) ts.locs.set(l.id, l);
  ts.maps.clear();
  for (const m of ix.maps || []) ts.maps.set(m.id, m);
  ts.kinds = ix.kinds || [];
  ts.mode = ix.mode || "hidden";
  ts.ready = Boolean(ix.ready);
}

export function set_state(st) {
  ts.state = st;
  ts.seq = st.seq || 0;
  ts.mode = st.mode || ts.mode;
  if (st.kinds) ts.kinds = st.kinds;
  for (const [id, m] of Object.entries(st.locations || {})) {
    const l = ts.locs.get(id);
    if (l) l.st = m;
  }
}

export function upsert(loc) {
  ts.locs.set(loc.id, loc);
}

export function mark_just(id) {
  ts.just.set(id, Date.now());
}

export function is_just(id) {
  const t = ts.just.get(id);
  if (!t) return false;
  if (Date.now() - t > 8000) {
    ts.just.delete(id);
    return false;
  }
  return true;
}

function text(v, max) {
  return typeof v === "string" ? v.slice(0, max) : "";
}

function frac(v) {
  return typeof v === "number" && Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0.5;
}

function clean_pin(p) {
  if (!p || typeof p !== "object") return null;
  const id = text(p.id, 24);
  const map = text(p.map, 64);
  if (!id || !map) return null;
  return {
    id,
    place: text(p.place, 64),
    map,
    floor: Number.isInteger(p.floor) ? p.floor : null,
    u: frac(p.u),
    v: frac(p.v),
    kind: text(p.kind, 24) || "note",
    text: text(p.text, 80),
    done: p.done === true,
  };
}

export function remember() {
  try {
    const v = { search: ts.search, place: ts.place, age: ts.age, my_pins: ts.my_pins, pins: ts.pins.slice(0, MAX_PINS), notes: ts.notes };
    localStorage.setItem(KEY, JSON.stringify(v));
  } catch {
    return;
  }
}

export function recall() {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) || "null");
    if (!v || typeof v !== "object") return;
    ts.search = text(v.search, 100);
    ts.place = text(v.place, 64);
    if (["child", "adult", "both"].includes(v.age)) ts.age = v.age;
    ts.my_pins = v.my_pins !== false;
    if (Array.isArray(v.pins)) ts.pins = v.pins.map(clean_pin).filter(Boolean).slice(0, MAX_PINS);
    if (v.notes && typeof v.notes === "object") {
      const out = {};
      for (const [k, n] of Object.entries(v.notes).slice(0, MAX_NOTES)) {
        if (typeof n === "string" && n) out[k.slice(0, 80)] = n.slice(0, 200);
      }
      ts.notes = out;
    }
  } catch {
    ts.search = "";
  }
}

export function set_note(id, note) {
  const t = note.trim().slice(0, 200);
  if (t) ts.notes[id] = t;
  else delete ts.notes[id];
  remember();
}

export function add_pin(p) {
  const pin = clean_pin({ ...p, id: `${Date.now().toString(36)}${Math.floor(Math.random() * 1296).toString(36)}` });
  if (!pin || ts.pins.length >= MAX_PINS) return null;
  ts.pins.push(pin);
  remember();
  return pin;
}

export function drop_pin(id) {
  ts.pins = ts.pins.filter((p) => p.id !== id);
  remember();
}

export const THEMES = ["hyrule", "dark", "light"];

export function saved_theme() {
  try {
    const v = localStorage.getItem(THEME_KEY);
    return THEMES.includes(v) ? v : "hyrule";
  } catch {
    return "hyrule";
  }
}

export function save_theme(t) {
  try {
    localStorage.setItem(THEME_KEY, t);
  } catch {
    return;
  }
}

export function main_paths() {
  try {
    const v = JSON.parse(localStorage.getItem("ootrr.v1") || "null");
    const p = v && v.paths;
    return { game: p && typeof p.game === "string" ? p.game : "", host: p && typeof p.host === "string" ? p.host : "eden" };
  } catch {
    return { game: "", host: "eden" };
  }
}
