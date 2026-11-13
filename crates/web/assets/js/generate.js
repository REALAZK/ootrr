import { h, fill, notice, copy, save_text } from "./dom.js";
import { button as report_button } from "./report.js";
import { panel as plando_panel } from "./plando.js";
import { post } from "./api.js";
import { state, touch } from "./store.js";
import { effective, match_preset, changed } from "./rules.js";
import { panel as spoiler_panel } from "./spoiler.js";
import { go, bar } from "./nav.js";
import { live, refresh_string } from "./strings.js";

function summary(res) {
  return h(
    "dl",
    { class: "kv" },
    h("dt", null, "Seed"),
    h("dd", { class: "mono" }, res.seed),
    h("dt", null, "Settings string"),
    h("dd", { class: "mono" }, res.settings_string),
    h("dt", null, "Checksum"),
    h("dd", { class: "mono" }, res.checksum),
    h("dt", null, "Placements"),
    h("dd", null, String(res.placement_count)),
    h("dt", null, "Made by"),
    h("dd", null, res.generator || "unknown"),
  );
}

export function render() {
  const schema = state.schema;
  const out = h("div", { "aria-live": "polite" });
  const fix = h("div");
  const seed = h("input", { type: "text", id: "seed", maxlength: "100", value: state.seed, spellcheck: "false", autocomplete: "off", placeholder: "Leave empty for a random seed", oninput: () => {
    state.seed = seed.value;
    touch();
  } });
  const go_btn = h("button", { type: "button", class: "btn primary" }, "Generate seed");
  const hide = h("input", { type: "checkbox", id: "hide_spoiler", checked: state.hide_spoiler, onchange: () => {
    state.hide_spoiler = hide.checked;
    touch();
    if (state.result) show(state.result);
  } });
  const pick = h("input", { type: "file", accept: ".ootrr,.json,application/json", class: "hide" });
  const preset_id = match_preset(schema, state.vals);
  const preset = preset_id ? schema.presets.find((p) => p.id === preset_id).name : "Custom";
  const n = changed(schema, state.vals).length;

  function show(res) {
    state.result = res;
    touch();
    bar();
    fill(
      out,
      h(
        "section",
        { class: "card stack" },
        h("h2", null, "Your seed"),
        res.placements_ready ? null : notice("warn", res.notice || "This seed changes no items. Turn on a shuffle setting if you want items to move."),
        res.warnings && res.warnings.length ? notice("warn", res.warnings.join(" ")) : null,
        Array.isArray(res.notes) && res.notes.length ? notice("warn", res.notes.join("\n")) : null,
        state.plando.length && res.report && res.report.includes("Plandomizer:") ? notice("info", `Plandomizer: ${state.plando.length} hand placed ${state.plando.length === 1 ? "entry was" : "entries were"} used for this seed. Share the seed file, because the seed text and settings alone cannot recreate it.`) : null,
        summary(res),
        h(
          "div",
          { class: "row" },
          h("button", { type: "button", class: "btn primary", onclick: () => save_text(res.file_name || "seed.ootrr", res.seed_text) }, "Download seed file"),
          h("button", { type: "button", class: "btn", onclick: () => copy(res.settings_string) }, "Copy settings string"),
          report_button(res),
          h("button", { type: "button", class: "btn", onclick: () => go("result") }, "Continue to build"),
        ),
      ),
      h("section", { class: "card" }, h("h2", null, "Spoiler"), spoiler_panel(res, state.hide_spoiler)),
    );
  }

  async function run() {
    go_btn.disabled = true;
    fill(out, h("section", { class: "card" }, h("p", null, "Generating"), h("progress", { "aria-label": "Generating" })));
    try {
      const res = await post("/api/generate", { seed: seed.value, settings: effective(schema, state.vals), spoiler: true, plando: state.plando });
      seed.value = res.seed;
      state.seed = res.seed;
      state.last_error = "";
      show(res);
    } catch (e) {
      state.result = null;
      state.last_error = e.message;
      touch();
      bar();
      fill(out, notice("error", e.message));
    }
    go_btn.disabled = false;
  }

  async function open_file() {
    const f = pick.files && pick.files[0];
    if (!f) return;
    if (f.size > 8 * 1024 * 1024) return fill(out, notice("error", "That file is too large to be a seed file."));
    const text = await f.text();
    try {
      const res = await post("/api/seed/read", { seed_text: text });
      res.seed_text = text;
      res.file_name = f.name.replace(/[^A-Za-z0-9._-]/g, "-");
      res.notice = res.placements_ready ? "" : "This seed changes no items. Turn on a shuffle setting if you want items to move.";
      state.vals = res.settings;
      state.seed = res.seed;
      seed.value = res.seed;
      show(res);
    } catch (e) {
      fill(out, notice("error", e.message));
    }
    pick.value = "";
  }

  go_btn.onclick = run;
  pick.onchange = open_file;
  if (state.result) {
    try {
      show(state.result);
    } catch {
      state.result = null;
      touch();
      fill(out, notice("warn", "The saved seed from an earlier visit could not be shown, so it was cleared. Generate a new one."));
    }
  }
  refresh_string().then(() => {
    if (live.error) fill(fix, notice("error", `The settings need a fix before a seed can be made. ${live.error}`));
  });

  return h(
    "div",
    null,
    h("h1", null, "Generate"),
    h(
      "section",
      { class: "card stack" },
      h("div", null, h("label", { class: "lbl", for: "seed" }, "Seed"), seed, h("p", { class: "hint" }, "The same seed with the same settings (and no plandomizer entries) always gives the same seed file.")),
      h("p", { class: "muted" }, `Settings: ${preset}${n ? `, ${n} changed from the defaults` : ""}. `, h("a", { href: "#/settings" }, "Change settings")),
      h("div", null, h("label", { class: "switch", for: "hide_spoiler" }, hide, h("span", null, "Hide the spoiler")), h("p", { class: "hint" }, "Keeps the spoiler out of sight when the seed is made. You can still show it or download a copy. This only hides it on this page, the seed file still holds the placements.")),
      h(
        "div",
        { class: "row" },
        go_btn,
        h("button", { type: "button", class: "btn", onclick: () => pick.click() }, "Open a seed file"),
        pick,
      ),
    ),
    fix,
    plando_panel(),
    out,
  );
}
