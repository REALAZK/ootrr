import { h, fill } from "./dom.js";
import { state, touch } from "./store.js";
import { defaults, effective, enabled, from_preset, match_preset, search_text, with_value, clone } from "./rules.js";
import { make_row } from "./controls.js";
import { panel } from "./strings.js";
import { go } from "./nav.js";

let ui = { tab: "", advanced: false, query: "" };

export function render() {
  const schema = state.schema;
  const ctx = {
    set(id, v) {
      state.vals = with_value(schema, state.vals, id, v);
      touch();
      sync();
    },
  };
  const rows = schema.settings.map((s) => make_row(schema, s, ctx));
  if (!ui.tab || !schema.groups.some((g) => g.id === ui.tab)) ui.tab = schema.groups[0].id;

  const strings = panel(() => sync());
  const preset_box = h("div", { class: "preset-grid", role: "group", "aria-label": "Presets" });
  const tabs = h("div", { class: "tabs", role: "tablist", "aria-label": "Setting groups" });
  const group_note = h("p", { class: "groupnote" });
  const list = h("div", { class: "settings" }, rows.map((r) => r.el));
  const empty = h("p", { class: "muted hide" }, "No settings match that search.");
  const count = h("span", { class: "muted small" });
  const search = h("input", { type: "search", "aria-label": "Search settings", placeholder: "Search settings", value: ui.query, oninput: () => {
    ui.query = search.value;
    filter();
  } });
  const adv = h("input", { type: "checkbox", id: "adv", checked: ui.advanced, onchange: () => {
    ui.advanced = adv.checked;
    filter();
  } });

  function presets() {
    const now = match_preset(schema, state.vals);
    fill(
      preset_box,
      schema.presets.map((p) => {
        const ok = p.supported !== false;
        const gaps = p.unsupported || [];
        return h(
          "button",
          {
            type: "button",
            class: ok ? "preset" : "preset unsupported",
            title: ok ? p.tip : `Not available yet. This build cannot apply:\n${gaps.join("\n")}`,
            disabled: !ok,
            "aria-pressed": now === p.id ? "true" : "false",
            onclick: () => {
              state.vals = from_preset(schema, p);
              touch();
              sync();
            },
          },
          h("b", null, p.name),
          h("span", null, ok ? p.tip.split(". ")[0] : "Not available yet in this build."),
        );
      }),
    );
  }

  function tab_bar() {
    const searching = ui.query.trim() !== "";
    fill(
      tabs,
      schema.groups.map((g) =>
        h(
          "button",
          {
            type: "button",
            class: "tab",
            role: "tab",
            "aria-selected": !searching && ui.tab === g.id ? "true" : "false",
            onclick: () => {
              ui.tab = g.id;
              search.value = "";
              ui.query = "";
              filter();
            },
          },
          g.label,
        ),
      ),
    );
  }

  function filter() {
    const qy = ui.query.trim().toLowerCase();
    let shown = 0;
    for (const r of rows) {
      let show = qy ? search_text(r.s).includes(qy) : r.s.group === ui.tab;
      if (show && !ui.advanced && r.s.tag === "niche" && !qy) show = false;
      r.el.hidden = !show;
      if (show) shown++;
    }
    const g = schema.groups.find((x) => x.id === ui.tab);
    group_note.textContent = qy ? `${shown} matching settings` : g.tip;
    empty.classList.toggle("hide", shown > 0);
    tab_bar();
  }

  function sync() {
    const eff = effective(schema, state.vals);
    for (const r of rows) r.sync(eff, enabled(r.s, eff));
    presets();
    strings.update();
    const base = effective(schema, defaults(schema));
    const n = schema.settings.filter((s) => JSON.stringify(eff[s.id]) !== JSON.stringify(base[s.id])).length;
    count.textContent = n === 0 ? "All settings are at their defaults" : `${n} setting${n === 1 ? "" : "s"} changed from the defaults`;
  }

  sync();
  filter();

  return h(
    "div",
    null,
    h("h1", null, "Settings"),
    h("p", { class: "muted" }, "Pick a preset or change settings one by one. The settings string below is a short code for everything you chose, and you can share it."),
    preset_box,
    strings.el,
    h(
      "div",
      { class: "toolbar" },
      search,
      h(
        "div",
        { class: "row between" },
        h("label", { class: "switch", for: "adv" }, adv, h("span", null, "Show rarely used settings")),
        h(
          "button",
          { type: "button", class: "btn small", onclick: () => {
            state.vals = clone(defaults(schema));
            touch();
            sync();
          } },
          "Reset all",
        ),
      ),
    ),
    count,
    tabs,
    group_note,
    list,
    empty,
    h("div", { class: "row", }, h("button", { type: "button", class: "btn primary", onclick: () => go("generate") }, "Continue to generate")),
  );
}
