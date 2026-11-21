import { h, fill, notice, copy, debounce } from "./dom.js";
import { post } from "./api.js";
import { state, touch } from "./store.js";
import { effective } from "./rules.js";

export const live = { text: "", error: "" };

let ticket = 0;

export async function refresh_string() {
  const mine = ++ticket;
  try {
    const r = await post("/api/settings/encode", { settings: effective(state.schema, state.vals) });
    if (mine !== ticket) return;
    live.text = r.settings_string;
    live.error = "";
  } catch (e) {
    if (mine !== ticket) return;
    live.text = "";
    live.error = e.message;
  }
}

export function panel(on_loaded) {
  const out = h("input", { type: "text", readonly: true, "aria-label": "Settings string", class: "mono", spellcheck: "false" });
  const err = h("div", { "aria-live": "polite" });
  const copy_btn = h("button", { type: "button", class: "btn", onclick: () => live.text && copy(live.text) }, "Copy");
  const imp = h("textarea", { "aria-label": "Settings string to import", placeholder: "Paste a settings string here", spellcheck: "false", rows: "3" });
  const imp_out = h("div", { "aria-live": "polite" });

  async function show() {
    await refresh_string();
    out.value = live.text;
    copy_btn.disabled = !live.text;
    fill(err, live.error ? notice("error", live.error) : null);
  }

  const later = debounce(show, 250);

  async function load() {
    const t = imp.value.trim();
    if (!t) return fill(imp_out, notice("warn", "Paste a settings string first."));
    try {
      const r = await post("/api/settings/decode", { settings_string: t });
      state.vals = r.settings;
      touch();
      on_loaded();
      imp.value = "";
      fill(imp_out, notice("ok", "Settings loaded."), r.warnings.length ? notice("warn", r.warnings.join(" ")) : null);
    } catch (e) {
      fill(imp_out, notice("error", e.message));
    }
  }

  show();

  const box = h(
    "section",
    { class: "stringbar" },
    h("label", { class: "lbl" }, "Settings string"),
    h("div", { class: "stringbox" }, out, copy_btn),
    err,
    h(
      "details",
      null,
      h("summary", null, "Import a settings string"),
      h("div", { class: "stack" }, imp, h("div", { class: "row" }, h("button", { type: "button", class: "btn", onclick: load }, "Load settings")), imp_out),
    ),
  );
  return { el: box, update: later };
}
