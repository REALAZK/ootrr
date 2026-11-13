import { s } from "./tracker_kinds.js";

export const W = 1000;
export const H = 720;

function runs(rows) {
  const n = rows.length;
  if (!n) return "";
  const cw = W / (rows[0].length * 4);
  const ch = H / n;
  let d = "";
  rows.forEach((row, y) => {
    let start = -1;
    const bits = row.length * 4;
    for (let x = 0; x <= bits; x += 1) {
      const on = x < bits && (parseInt(row[x >> 2], 16) & (8 >> (x & 3))) !== 0;
      if (on && start < 0) start = x;
      if (!on && start >= 0) {
        const x0 = (start + 0.5) * cw;
        const len = (x - 1 - start) * cw;
        d += `M${x0.toFixed(1)} ${((y + 0.5) * ch).toFixed(1)}h${len.toFixed(1)}`;
        start = -1;
      }
    }
  });
  return d;
}

function schematic(map, floor) {
  const sh = (map.shapes || []).find((x) => x.floor === floor) || (map.shapes || []).find((x) => x.floor === null);
  if (!sh) return null;
  const d = runs(sh.rows);
  if (!d) return null;
  return s("g", { class: "art schematic" }, s("path", { class: "land-edge", d }), s("path", { class: "land", d }), s("path", { class: "land2", d }));
}

let source = schematic;

export function set_art_source(fn) {
  source = typeof fn === "function" ? fn : schematic;
}

export function art_layer(map, floor) {
  let g = null;
  try {
    g = source(map, floor);
  } catch {
    g = null;
  }
  if (!g && source !== schematic) g = schematic(map, floor);
  return g;
}
