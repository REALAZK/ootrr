import { defaults, sanitize } from "./rules.js";

const KEY = "ootrr.v1";
const RKEY = "ootrr.result";

export const state = {
  schema: null,
  info: null,
  vals: {},
  seed: "",
  plando: [],
  paths: { game: "", mod: "", name: "My Seed", host: "eden" },
  game_check: null,
  mod_check: null,
  result: null,
  hide_spoiler: false,
};

const subs = new Set();

export function on_change(fn) {
  subs.add(fn);
  return () => subs.delete(fn);
}

function put(store, key, v) {
  try {
    store.setItem(key, JSON.stringify(v));
  } catch {
    return;
  }
}

function take(store, key) {
  try {
    return JSON.parse(store.getItem(key) || "null");
  } catch {
    return null;
  }
}

export function save() {
  put(localStorage, KEY, { vals: state.vals, seed: state.seed, plando: state.plando, paths: state.paths, hide_spoiler: state.hide_spoiler });
  put(sessionStorage, RKEY, state.result);
}

export function touch() {
  for (const f of subs) f();
  save();
}

export function restore(schema) {
  state.schema = schema;
  const s = take(localStorage, KEY);
  state.vals = sanitize(schema, s && s.vals);
  if (s && typeof s.seed === "string") state.seed = s.seed.slice(0, 100);
  if (s && typeof s.hide_spoiler === "boolean") state.hide_spoiler = s.hide_spoiler;
  if (s && Array.isArray(s.plando)) {
    state.plando = s.plando
      .filter((p) => p && typeof p.location === "string" && typeof p.item === "string")
      .slice(0, 200)
      .map((p) => ({ location: p.location.slice(0, 120), item: p.item.slice(0, 120) }));
  }
  if (s && s.paths && typeof s.paths === "object") {
    for (const k of ["game", "mod", "name", "host"]) {
      if (typeof s.paths[k] === "string") state.paths[k] = s.paths[k].slice(0, 1000);
    }
    if (!["eden", "hookshot", "longshot", "nx2navi"].includes(state.paths.host)) state.paths.host = "eden";
  }
  const r = take(sessionStorage, RKEY);
  if (r && typeof r.seed_text === "string") state.result = r;
  if (!state.vals) state.vals = defaults(schema);
}
