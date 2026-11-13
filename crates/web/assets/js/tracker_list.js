import { h, toast } from "./dom.js";
import { t_mark } from "./tracker_api.js";
import { ts, emit, is_just, remember } from "./tracker_store.js";
import { kind_name, kind_class, icon } from "./tracker_kinds.js";
import { place_of, place_name, age_ok, place_locs } from "./tracker_places.js";
import { current_place } from "./tracker_map.js";

const MAX_ROWS = 300;

function matches(l, q) {
  if (!q) return true;
  const t = `${l.label} ${l.name} ${l.maybe || ""} ${kind_name(l.kind)} ${l.area || ""} ${ts.notes[l.id] || ""}`.toLowerCase();
  return q.split(/\s+/).every((w) => t.includes(w));
}

export function status_text(l) {
  const st = l.st || {};
  if (st.s === "lost") return "Lost on reload";
  if (st.s === "weak") return "Maybe, weak evidence";
  if (st.s === "found") {
    if (is_just(l.id)) return "Seen in your save just now";
    if (st.by === "user") return "Ticked by you";
    if (st.by === "sticky") return "Remembered by the tracker";
    return "Seen in your save";
  }
  if (l.how === "none") return "Manual, no trace in a save";
  return "Open";
}

export async function flip(id, on) {
  try {
    await t_mark(id, on);
  } catch (e) {
    toast(e.message);
  }
}

function tags(l, wide) {
  const out = [];
  const pin = (l.pins || [])[0];
  if (wide) out.push(h("span", { class: "tag" }, place_name(place_of(l))));
  if (l.ages && l.ages.length === 1) out.push(h("span", { class: "tag" }, l.ages[0] === "child" ? "Child" : "Adult"));
  if (pin && pin.floor_label) out.push(h("span", { class: "tag" }, pin.floor_label));
  if (pin && pin.tray) out.push(h("span", { class: "tag" }, "Indoors"));
  if (l.dream) out.push(h("span", { class: "tag" }, "Dream"));
  return out;
}

function row(l, wide) {
  const st = l.st || {};
  const found = st.s === "found";
  const cls = ["row", kind_class(l.kind)];
  if (found) cls.push("done");
  if (ts.sel === l.id) cls.push("sel");
  if (is_just(l.id)) cls.push("just");
  const note = ts.notes[l.id];
  const pick = h(
    "button",
    {
      type: "button",
      class: "pick",
      "aria-current": ts.sel === l.id ? "true" : null,
      onclick: () => {
        ts.sel = l.id;
        if (wide) {
          ts.place = place_of(l);
          remember();
        }
        emit();
      },
    },
    h("span", { class: "ic" }, icon(l.kind)),
    h(
      "span",
      { class: "tx" },
      h("span", { class: "nm" }, l.label),
      h("span", { class: "sub" }, tags(l, wide), status_text(l)),
      note ? h("span", { class: "sub note" }, note) : null,
    ),
  );
  const tick = h(
    "button",
    { type: "button", class: "tickbtn", "aria-pressed": String(found), "aria-label": `${l.label}, ${found ? "collected, mark open" : "mark collected"}`, onclick: () => flip(l.id, !found) },
    found ? "✓" : "",
  );
  return h("li", { class: cls.join(" "), "data-id": l.id }, pick, tick);
}

function pin_row(p) {
  const id = `u:${p.id}`;
  const cls = ["row", "k-user"];
  if (p.done) cls.push("done");
  if (ts.sel === id) cls.push("sel");
  return h(
    "li",
    { class: cls.join(" ") },
    h(
      "button",
      {
        type: "button",
        class: "pick",
        onclick: () => {
          ts.sel = id;
          emit();
        },
      },
      h("span", { class: "ic" }, icon("note")),
      h("span", { class: "tx" }, h("span", { class: "nm" }, p.text || kind_name(p.kind)), h("span", { class: "sub" }, "Your pin, saved in this browser")),
    ),
    h("span", { class: "st" }, "pin"),
  );
}

function group(name, count, rows) {
  if (!rows.length) return null;
  return [h("div", { class: "grp" }, h("span", null, name), h("span", null, String(count))), h("ul", { class: "rows" }, rows)];
}

function by_name(a, b) {
  return a.label.localeCompare(b.label, undefined, { numeric: true });
}

export function render_list() {
  const q = ts.search.trim().toLowerCase();
  const pl = current_place();
  let locs = [];
  if (q) {
    for (const l of ts.locs.values()) {
      if (age_ok(l) && matches(l, q)) locs.push(l);
    }
  } else if (pl) locs = place_locs(pl);
  locs.sort(by_name);
  const open = locs.filter((l) => (l.st || {}).s !== "found");
  const done = locs.filter((l) => (l.st || {}).s === "found");
  const mine = ts.my_pins && pl && !q ? ts.pins.filter((p) => p.place === pl.id) : [];
  const out = [];
  const wide = Boolean(q);
  const cut = (a) => a.slice(0, MAX_ROWS).map((l) => row(l, wide));
  out.push(group(ts.mode === "hidden" ? "Revealed" : "Open", open.length, cut(open)));
  out.push(group("Your pins", mine.length, mine.map(pin_row)));
  out.push(group("Collected", done.length, cut(done)));
  if (open.length > MAX_ROWS || done.length > MAX_ROWS) out.push(h("p", { class: "fine pad" }, `Only the first ${MAX_ROWS} of each group are listed. Search to narrow it down.`));
  if (!locs.length && !mine.length) {
    const msg = q
      ? "No location matches."
      : ts.mode === "hidden"
        ? "Nothing found here yet. Locations appear once your save shows them collected. Turn on Show all to see every one."
        : "No location of the chosen kinds here.";
    out.push(h("p", { class: "fine pad" }, msg));
  }
  return out;
}
