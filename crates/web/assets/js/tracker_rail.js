import { h } from "./dom.js";
import { ts } from "./tracker_store.js";
import { places } from "./tracker_places.js";
import { current_place, go_place } from "./tracker_map.js";

function count(p) {
  if (ts.mode === "hidden") return p.found > 0 ? String(p.found) : "··";
  if (p.all > 0 && p.found === p.all) return "✓";
  return `${p.found} / ${p.all}`;
}

function entry(p, cur) {
  const done = ts.mode !== "hidden" && p.all > 0 && p.found === p.all;
  return h(
    "button",
    {
      type: "button",
      class: `place${p.dungeon ? " dgn" : ""}${done ? " done" : ""}`,
      "aria-current": cur && cur.id === p.id ? "true" : null,
      title: p.name,
      onclick: () => go_place(p.id),
    },
    h("i", { class: "glyph" }),
    h("span", { class: "n" }, p.name),
    p.here ? h("i", { class: "here", title: "Your last save is here" }) : null,
    h("span", { class: "c" }, count(p)),
  );
}

export function render_rail() {
  const all = places();
  const cur = current_place();
  const zones = all.filter((p) => p.maps.length > 0 && !p.dungeon);
  const dungeons = all.filter((p) => p.maps.length > 0 && p.dungeon);
  const other = all.filter((p) => p.maps.length === 0);
  const part = (name, list) => (list.length ? [h("h3", null, name), list.map((p) => entry(p, cur))] : null);
  if (!all.length) return h("p", { class: "fine pad" }, "Places appear here once the tracker has something to show.");
  return [part("Zones", zones), part("Dungeons", dungeons), part(zones.length ? "Elsewhere" : "Places", other)];
}
