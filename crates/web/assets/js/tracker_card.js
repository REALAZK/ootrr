import { h } from "./dom.js";
import { ts, emit, set_note, drop_pin, remember } from "./tracker_store.js";
import { kind_name, PIN_TYPES } from "./tracker_kinds.js";
import { place_of, place_name } from "./tracker_places.js";
import { flip, status_text } from "./tracker_list.js";

function auto_text(l) {
  if (l.how === "none") return "Nothing in a save shows this one. Tick it by hand.";
  if (l.weak) return "Weak evidence. The save only shows that some source of this item was used.";
  if (l.life === "permanent") return "Read from your save. It shows after the next autosave.";
  if (l.life === "none") return "Tick it by hand.";
  return "The game forgets this kind after a while. The tracker remembers it once it was seen.";
}

function ages(l) {
  const a = l.ages || [];
  if (a.length === 1) return a[0] === "child" ? "Child only" : "Adult only";
  return a.length ? "Child and adult" : "";
}

function note_box(id, label) {
  const inp = h("input", { type: "text", class: "inp", maxlength: "200", value: ts.notes[id] || "", placeholder: "A short note for yourself", "aria-label": label });
  inp.addEventListener("change", () => {
    set_note(id, inp.value);
    emit();
  });
  return inp;
}

function loc_card(l) {
  const found = (l.st || {}).s === "found";
  const pin = (l.pins || [])[0];
  const where = [place_name(place_of(l)), pin && pin.floor_label, l.room !== null && l.room !== undefined ? `Room ${String(l.room).padStart(2, "0")}` : null].filter(Boolean).join(", ");
  return h(
    "div",
    { class: "card" },
    h("h4", null, l.label),
    h("div", { class: "meta" }, [kind_name(l.kind), ages(l)].filter(Boolean).join(". ")),
    h(
      "dl",
      null,
      h("dt", null, "Where"),
      h("dd", null, where || "Unknown"),
      l.maybe ? [h("dt", null, "Maybe"), h("dd", null, l.maybe)] : null,
      l.name !== l.label ? [h("dt", null, "Data name"), h("dd", null, l.name)] : null,
      h("dt", null, "State"),
      h("dd", null, status_text(l)),
      h("dt", null, "Auto"),
      h("dd", null, auto_text(l)),
      h("dt", null, "Note"),
      h("dd", null, note_box(l.id, `Note for ${l.label}`)),
    ),
    h(
      "div",
      { class: "acts" },
      h("button", { type: "button", class: "btn pri", onclick: () => flip(l.id, !found) }, found ? "Mark open" : "Mark collected"),
      h(
        "button",
        {
          type: "button",
          class: "btn",
          onclick: () => {
            ts.sel = "";
            emit();
          },
        },
        "Close",
      ),
    ),
  );
}

function pin_card(p) {
  const save = () => {
    remember();
    emit();
  };
  const name = h("input", { type: "text", class: "inp", maxlength: "80", value: p.text, placeholder: "What is here", "aria-label": "Pin text" });
  name.addEventListener("change", () => {
    p.text = name.value.trim().slice(0, 80);
    save();
  });
  const type = h(
    "select",
    { class: "inp", "aria-label": "Pin type" },
    PIN_TYPES.map((k) => h("option", { value: k, selected: p.kind === k }, kind_name(k))),
  );
  type.addEventListener("change", () => {
    p.kind = type.value;
    save();
  });
  return h(
    "div",
    { class: "card" },
    h("h4", null, p.text || "Your pin"),
    h("div", { class: "meta" }, `Your pin in ${place_name(p.place)}. Saved in this browser only.`),
    h("dl", null, h("dt", null, "Text"), h("dd", null, name), h("dt", null, "Type"), h("dd", null, type)),
    h(
      "div",
      { class: "acts" },
      h(
        "button",
        {
          type: "button",
          class: "btn pri",
          onclick: () => {
            p.done = !p.done;
            save();
          },
        },
        p.done ? "Mark open" : "Mark done",
      ),
      h(
        "button",
        {
          type: "button",
          class: "btn",
          onclick: () => {
            drop_pin(p.id);
            ts.sel = "";
            emit();
          },
        },
        "Delete pin",
      ),
    ),
  );
}

export function render_card() {
  if (!ts.sel) return null;
  if (ts.sel.startsWith("u:")) {
    const p = ts.pins.find((x) => `u:${x.id}` === ts.sel);
    return p ? pin_card(p) : null;
  }
  const l = ts.locs.get(ts.sel);
  return l ? loc_card(l) : null;
}
