import { h, fill, notice, copy, save_text } from "./dom.js";
import { button as report_button } from "./report.js";
import { post } from "./api.js";
import { state, touch } from "./store.js";
import { effective, match_preset, changed } from "./rules.js";
import { viewer } from "./viewer.js";
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
        res.placements_ready ? null : notice("warn", res.notice || "This seed file has no item placements yet. Placement arrives with the chest shuffle slice."),
        res.warnings && res.warnings.length ? notice("warn", res.warnings.join(" ")) : null,
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
      h("section", { class: "card" }, h("h2", null, "Spoiler"), viewer(res)),
    );
  }

  async function run() {
    go_btn.disabled = true;
    fill(out, h("section", { class: "card" }, h("p", null, "Generating"), h("progress", { "aria-label": "Generating" })));
    try {
      const res = await post("/api/generate", { seed: seed.value, settings: effective(schema, state.vals), spoiler: true });
      seed.value = res.seed;
      state.seed = res.seed;
      show(res);
    } catch (e) {
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
      res.notice = res.placements_ready ? "" : "This seed file has no item placements yet. Placement arrives with the chest shuffle slice.";
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
  if (state.result) show(state.result);
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
      h("div", null, h("label", { class: "lbl", for: "seed" }, "Seed"), seed, h("p", { class: "hint" }, "The same seed with the same settings always gives the same seed file.")),
      h("p", { class: "muted" }, `Settings: ${preset}${n ? `, ${n} changed from the defaults` : ""}. `, h("a", { href: "#/settings" }, "Change settings")),
      h(
        "div",
        { class: "row" },
        go_btn,
        h("button", { type: "button", class: "btn", onclick: () => pick.click() }, "Open a seed file"),
        pick,
      ),
    ),
    fix,
    out,
  );
}
