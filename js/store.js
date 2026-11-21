import { defaults, sanitize } from "./rules.js";

const KEY = "ootrr.v1";
const RKEY = "ootrr.result";

export const state = {
  schema: null,
  info: null,
  vals: {},
  seed: "",
  plando: [],
  paths: { game: "", update: "", mod: "/zip/mods", name: "My Seed", host: "eden" },
  include_update: false,
  cosmetic: { music: "off", sfx: "off", seed: "", title: true, parts: {}, locks: {}, reroll: false, style: "random" },
  game_check: null,
  mod_check: null,
  result: null,
  hide_spoiler: false,
  spoiler_log: true,
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
  put(localStorage, KEY, { vals: state.vals, seed: state.seed, plando: state.plando, paths: state.paths, cosmetic: state.cosmetic, hide_spoiler: state.hide_spoiler, spoiler_log: state.spoiler_log, include_update: state.include_update });
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
  if (s && typeof s.spoiler_log === "boolean") state.spoiler_log = s.spoiler_log;
  if (s && typeof s.include_update === "boolean") state.include_update = s.include_update;
  if (s && Array.isArray(s.plando)) {
    state.plando = s.plando
      .filter((p) => p && typeof p.location === "string" && typeof p.item === "string")
      .slice(0, 200)
      .map((p) => ({ location: p.location.slice(0, 120), item: p.item.slice(0, 120) }));
  }
  if (s && s.paths && typeof s.paths === "object") {
    for (const k of ["game", "update", "mod", "name", "host"]) {
      if (typeof s.paths[k] === "string") state.paths[k] = s.paths[k].slice(0, 1000);
    }
    if (!["eden", "hookshot", "longshot", "nx2navi"].includes(state.paths.host)) state.paths.host = "eden";
    state.paths.mod = "/zip/mods";
  }
  if (s && s.cosmetic && typeof s.cosmetic === "object") {
    if (["off", "grouped", "anywhere"].includes(s.cosmetic.music)) state.cosmetic.music = s.cosmetic.music;
    if (["off", "grouped", "anywhere"].includes(s.cosmetic.sfx)) state.cosmetic.sfx = s.cosmetic.sfx;
    if (typeof s.cosmetic.seed === "string") state.cosmetic.seed = s.cosmetic.seed.slice(0, 64);
    if (typeof s.cosmetic.title === "boolean") state.cosmetic.title = s.cosmetic.title;
    restore_colors(s.cosmetic);
  }
  const r = take(sessionStorage, RKEY);
  if (r && typeof r.seed_text === "string") state.result = r;
  if (!state.vals) state.vals = defaults(schema);
}

export function color_groups() {
  const groups = new Map();
  for (const p of (state.info && state.info.color_parts) || []) {
    if (!groups.has(p.group)) groups.set(p.group, []);
    groups.get(p.group).push(p);
  }
  return groups;
}

const PICK = /^(random|#[0-9a-f]{6})$/;

function old_picks(o) {
  const out = {};
  const own = o.parts && typeof o.parts === "object" ? o.parts : {};
  const gs = o.groups && typeof o.groups === "object" ? o.groups : {};
  for (const [g, ps] of color_groups()) {
    const mode = g === "Other" ? "custom" : gs[g] || "original";
    for (const p of ps) {
      if (mode === "random") out[p.id] = "random";
      else if (mode === "custom" && typeof own[p.id] === "string" && PICK.test(own[p.id])) out[p.id] = own[p.id];
    }
  }
  return out;
}

function restore_colors(o) {
  const cos = state.cosmetic;
  const known = new Set(((state.info && state.info.color_parts) || []).map((p) => p.id));
  if (typeof o.reroll !== "boolean") {
    if (o.colors === "random" || o.colors === "shared") {
      cos.reroll = true;
      cos.style = o.colors;
    } else if (o.colors === "custom") cos.parts = old_picks(o);
    return;
  }
  cos.reroll = o.reroll;
  cos.style = o.style === "shared" ? "shared" : "random";
  if (o.parts && typeof o.parts === "object") {
    for (const [k, v] of Object.entries(o.parts).slice(0, 64)) if (known.has(k) && typeof v === "string" && PICK.test(v)) cos.parts[k] = v;
  }
  if (o.locks && typeof o.locks === "object") {
    for (const [k, v] of Object.entries(o.locks).slice(0, 64)) if (known.has(k) && v === true) cos.locks[k] = true;
  }
}

export function color_request(cos) {
  const parts = cos.parts || {};
  const out = {};
  if (cos.reroll) {
    for (const [k, v] of Object.entries(cos.locks || {})) if (v) out[k] = parts[k] || "original";
    return { colors: cos.style === "shared" ? "shared" : "random", color_parts: out };
  }
  for (const [k, v] of Object.entries(parts)) if (PICK.test(v)) out[k] = v;
  return { colors: Object.keys(out).length ? "custom" : "off", color_parts: out };
}
