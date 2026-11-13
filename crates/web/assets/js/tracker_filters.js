import { h, toast, debounce } from "./dom.js";
import { t_mode } from "./tracker_api.js";
import { ts, emit, remember } from "./tracker_store.js";
import { kind_name, kind_class, icon } from "./tracker_kinds.js";

let reload_all = async () => {};

export function set_reload(fn) {
  reload_all = fn;
}

export async function send_mode(body) {
  try {
    await t_mode(body);
    await reload_all();
  } catch (e) {
    toast(e.message);
  }
}

function chip(k) {
  return h(
    "button",
    {
      type: "button",
      class: `f ${kind_class(k.id)}`,
      "aria-pressed": String(k.on),
      onclick: () => {
        const now = ts.kinds.filter((x) => (x.id === k.id ? !k.on : x.on)).map((x) => x.id);
        send_mode({ kinds: now });
      },
    },
    icon(k.id, 14),
    kind_name(k.id),
  );
}

export function render_filters() {
  const dream = Boolean(ts.state && ts.state.dream);
  return [
    (ts.kinds || []).map(chip),
    h(
      "button",
      {
        type: "button",
        class: "f k-user",
        "aria-pressed": String(ts.my_pins),
        onclick: () => {
          ts.my_pins = !ts.my_pins;
          remember();
          emit();
        },
      },
      icon("note", 14),
      "My pins",
    ),
    h("button", { type: "button", class: "f k-dream", "aria-pressed": String(dream), onclick: () => send_mode({ dream: !dream }) }, "Dream World"),
  ];
}

export function search_box() {
  const run = debounce(() => {
    remember();
    emit();
  }, 120);
  const inp = h("input", { type: "search", id: "trk-search", value: ts.search, placeholder: "Search every place", "aria-label": "Search locations" });
  inp.addEventListener("input", () => {
    ts.search = inp.value;
    run();
  });
  return h("label", { class: "search" }, inp, h("kbd", null, "/"));
}
