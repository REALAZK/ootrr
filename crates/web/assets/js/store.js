import { defaults, sanitize } from "./rules.js";

const KEY = "ootrr.v1";
const RKEY = "ootrr.result";

export const state = {
  schema: null,
  info: null,
  vals: {},
  seed: "",
  paths: { game: "", mod: "", name: "My Seed" },
  game_check: null,
  mod_check: null,
  result: null,
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
  put(localStorage, KEY, { vals: state.vals, seed: state.seed, paths: state.paths });
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
  if (s && s.paths && typeof s.paths === "object") {
    for (const k of ["game", "mod", "name"]) {
      if (typeof s.paths[k] === "string") state.paths[k] = s.paths[k].slice(0, 1000);
    }
  }
  const r = take(sessionStorage, RKEY);
  if (r && typeof r.seed_text === "string") state.result = r;
  if (!state.vals) state.vals = defaults(schema);
}
