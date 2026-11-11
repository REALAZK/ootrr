import { h, fill, notice } from "/js/dom.js";
import { post } from "/js/api.js";
import { state, touch } from "/js/store.js";
import { effective } from "/js/rules.js";

const MAX = 200;

export function panel() {
  const list = h("ul", { class: "plando-list" });
  const msg = h("div");
  const loc = h("input", { type: "text", id: "plando-loc", list: "plando-locs", placeholder: "Location", autocomplete: "off", spellcheck: "false", maxlength: "120" });
  const item = h("input", { type: "text", id: "plando-item", list: "plando-items", placeholder: "Item", autocomplete: "off", spellcheck: "false", maxlength: "120" });
  const locs = h("datalist", { id: "plando-locs" });
  const items = h("datalist", { id: "plando-items" });
  const add = h("button", { type: "button", class: "btn" }, "Add");
  const summary = h("summary", null);
  const title = () => fill(summary, state.plando.length ? `Plandomizer (advanced), ${state.plando.length} ${state.plando.length === 1 ? "entry" : "entries"} will be used` : "Plandomizer (advanced)");
  let loaded = false;

  function draw() {
    title();
    fill(
      list,
      state.plando.map((p, i) =>
        h(
          "li",
          null,
          h("span", null, `${p.location}: ${p.item}`),
          h("button", { type: "button", class: "btn quiet small", "aria-label": `Remove ${p.location}`, onclick: () => {
            state.plando.splice(i, 1);
            touch();
            draw();
          } }, "Remove"),
        ),
      ),
    );
  }

  async function load() {
    if (loaded) return;
    try {
      const r = await post("/api/plando/options", { settings: effective(state.schema, state.vals) });
      fill(locs, r.locations.map((x) => h("option", { value: x })));
      fill(items, r.items.map((x) => h("option", { value: x })));
      loaded = true;
      fill(msg, null);
    } catch (e) {
      fill(msg, notice("warn", `The name lists could not be loaded. ${e.message}`));
    }
  }

  add.onclick = () => {
    const l = loc.value.trim();
    const i = item.value.trim();
    if (!l || !i) return fill(msg, notice("warn", "Fill in both a location and an item."));
    if (state.plando.length >= MAX) return fill(msg, notice("warn", `At most ${MAX} entries are allowed.`));
    if (state.plando.some((p) => p.location.toLowerCase() === l.toLowerCase())) return fill(msg, notice("warn", "That location already has an entry. Remove it first."));
    state.plando.push({ location: l, item: i });
    touch();
    loc.value = "";
    item.value = "";
    fill(msg, null);
    draw();
  };

  const clear = h("button", { type: "button", class: "btn quiet", onclick: () => {
    state.plando = [];
    touch();
    draw();
  } }, "Clear all");

  draw();
  const box = h(
    "details",
    { class: "card", open: state.plando.length > 0 },
    summary,
    h("p", { class: "hint" }, "Pick a location and the item that must be there. Names are suggested from your current settings. The seed is only made if it can still be beaten."),
    h("div", { class: "row" }, loc, item, add),
    locs,
    items,
    msg,
    list,
    h("div", { class: "row" }, clear),
  );
  box.addEventListener("toggle", () => {
    if (box.open) load();
  });
  if (box.open) load();
  return box;
}
