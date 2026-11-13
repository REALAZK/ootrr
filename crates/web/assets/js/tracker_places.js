import { ts } from "./tracker_store.js";

const NAMES = {
  HountedWasteland: "Haunted Wasteland",
  LostWood: "Lost Woods",
  ZoraDomain: "Zora's Domain",
  ZoraRiver: "Zora's River",
  ZoraFountain: "Zora's Fountain",
  InsideTheDekuTree: "Inside the Deku Tree",
  DodongoCavern: "Dodongo's Cavern",
  InsideJabuJabuBelly: "Inside Jabu-Jabu's Belly",
  BottomOfTheWell: "Bottom of the Well",
  InsideGanonCastle: "Inside Ganon's Castle",
  GanonCastle: "Ganon's Castle",
  GerudoFortress: "Gerudo's Fortress",
};

export function place_name(id) {
  if (NAMES[id]) return NAMES[id];
  return id.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/_/g, " ");
}

export function place_of(l) {
  for (const p of l.pins || []) {
    const m = ts.maps.get(p.map);
    if (m && m.place) return m.place;
  }
  return l.scene || "Elsewhere";
}

function base_map(id) {
  return id ? id.replace(/_Alternative$/, "") : "";
}

export function player_map() {
  const p = ts.state && ts.state.player;
  return p && p.map ? base_map(p.map) : "";
}

export function places() {
  const by = new Map();
  const add = (id) => {
    if (!by.has(id)) by.set(id, { id, name: place_name(id), dungeon: false, maps: [], found: 0, all: 0, here: false });
    return by.get(id);
  };
  const here = player_map();
  for (const m of ts.maps.values()) {
    const p = add(m.place || m.id);
    p.maps.push(m);
    if (m.id === here) p.here = true;
  }
  for (const p of by.values()) p.dungeon = p.maps.length > 0 && p.maps.every((m) => !m.age);
  for (const l of ts.locs.values()) {
    const p = add(place_of(l));
    p.all += 1;
    if ((l.st || {}).s === "found") p.found += 1;
  }
  return [...by.values()].filter((p) => p.maps.length > 0 || p.all > 0);
}

export function shown_map(pl) {
  if (!pl || !pl.maps.length) return null;
  if (pl.maps.length === 1) return pl.maps[0];
  const aged = pl.maps.find((m) => m.age === ts.age);
  if (aged) return aged;
  const here = player_map();
  return pl.maps.find((m) => m.id === here) || pl.maps[0];
}

export function age_ok(l) {
  return ts.age === "both" || !l.ages || !l.ages.length || l.ages.includes(ts.age);
}

export function pin_on(l, pl, map) {
  if (!map) return null;
  const ok = (p) => p.u !== null && p.v !== null && !p.tray;
  const own = (l.pins || []).find((p) => p.map === map.id && ok(p));
  if (own) return own;
  if (ts.age !== "both") return null;
  return (l.pins || []).find((p) => ok(p) && pl.maps.some((m) => m.id === p.map)) || null;
}

export function place_locs(pl) {
  const out = [];
  for (const l of ts.locs.values()) {
    if (place_of(l) === pl.id && age_ok(l)) out.push(l);
  }
  return out;
}
