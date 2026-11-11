import { h, copy } from "/js/dom.js";
import { state } from "/js/store.js";
import { match_preset, changed } from "/js/rules.js";

export function text(res) {
  const schema = state.schema;
  const id = schema ? match_preset(schema, state.vals) : null;
  const preset = id ? schema.presets.find((p) => p.id === id).name : "Custom";
  const n = schema ? changed(schema, state.vals).length : 0;
  const info = state.info || {};
  const game = state.game_check ? (state.game_check.ok ? "found" : "not valid") : "not checked";
  const lines = [
    "OOTRR bug report",
    `Version: ${info.version || "unknown"} (${info.os || "unknown"})`,
    `Seed: ${res.seed}`,
    `Checksum: ${res.checksum}`,
    `Settings string: ${res.settings_string || "not available"}`,
    `Preset: ${preset}${n ? `, ${n} changed` : ""}`,
    `Game folder: ${game}`,
  ];
  if (state.plando && state.plando.length) lines.push(`Plandomizer: ${state.plando.map((p) => `${p.location}=${p.item}`).join(", ")}`);
  if (state.last_error) lines.push(`Last error: ${state.last_error}`);
  if (state.build_note) lines.push(`Last build: ${state.build_note}`);
  lines.push("", "What I did:", "", "What I expected:", "", "What happened:", "");
  return lines.join("\n");
}

export function button(res) {
  return h("button", { type: "button", class: "btn", onclick: () => copy(text(res)) }, "Copy bug report");
}
