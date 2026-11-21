import { h, fill, save_text } from "./dom.js";
import { viewer } from "./viewer.js";
import { color_request } from "./store.js";
import { effective, enabled, match_preset, value_label } from "./rules.js";

const RULE = "=".repeat(60);

export function full_text(res, ctx) {
  const schema = ctx.schema;
  const vals = effective(schema, res.settings || ctx.vals);
  const by_id = new Map(schema.settings.map((s) => [s.id, s]));
  const preset = match_preset(schema, vals);
  const sp = res.spoiler || {};
  const locs = sp.locations && sp.locations.length ? sp.locations : res.placements || [];
  const steps = sp.playthrough || [];
  const out = [
    "OOTRR spoiler log",
    RULE,
    `OOTRR version:   ${ctx.version || "unknown"}`,
    `Seed:            ${res.seed}`,
    `Settings string: ${res.settings_string || ""}`,
    `Checksum:        ${res.checksum || ""}`,
    `Preset:          ${preset ? schema.presets.find((p) => p.id === preset).name : "Custom"}`,
    `Made by:         ${res.generator || "unknown"}`,
    "",
    "Settings",
    RULE,
  ];
  for (const p of ctx.pages) {
    const rows = [];
    for (const sec of p.sections) {
      for (const id of sec.ids) {
        const s = by_id.get(id);
        if (!s || !enabled(s, vals)) continue;
        rows.push(`  ${s.label}: ${value_label(s, vals[id])}`);
      }
    }
    if (rows.length) out.push(`[${p.heading} › ${p.label}]`, ...rows, "");
  }
  const cos = ctx.cosmetic || {};
  const col = color_request(cos);
  out.push("[Cosmetics]", `  Music shuffle: ${cos.music || "off"}`, `  Sound effect shuffle: ${cos.sfx || "off"}`, `  Colour mode: ${col.colors}`);
  for (const [k, v] of Object.entries(col.color_parts)) out.push(`  Colour of ${k}: ${v}`);
  if (cos.seed) out.push(`  Cosmetic seed: ${cos.seed}`);
  out.push("");
  if (ctx.plando && ctx.plando.length) {
    out.push(`Plandomizer (${ctx.plando.length})`, RULE, ...ctx.plando.map((p) => `  ${p.location}: ${p.item}`), "");
  }
  if (steps.length) {
    out.push(`Playthrough (${steps.length} steps)`, RULE);
    steps.forEach((step, i) => {
      out.push(`Step ${i + 1}`);
      for (const [a, b] of step) out.push(`  ${a}: ${b}`);
    });
    out.push("");
  }
  out.push(`Locations (${locs.length})`, RULE);
  for (const [a, b] of locs) out.push(`  ${a}: ${b}`);
  return out.join("\n") + "\n";
}

export function text(res) {
  const sp = res.spoiler || {};
  const locs = sp.locations && sp.locations.length ? sp.locations : res.placements || [];
  const lines = ["OOTRR spoiler", `Seed: ${res.seed}`, `Checksum: ${res.checksum}`, "", "Placements"];
  for (const [a, b] of locs) lines.push(`  ${a}: ${b}`);
  (sp.playthrough || []).forEach((step, i) => {
    lines.push("", `Step ${i + 1}`);
    for (const [a, b] of step) lines.push(`  ${a}: ${b}`);
  });
  return lines.join("\n") + "\n";
}

function name(res) {
  const safe = String(res.seed || "seed").replace(/[^A-Za-z0-9._-]/g, "-").slice(0, 60);
  return `${safe}-spoiler.txt`;
}

export function panel(res, hidden) {
  if (!hidden) return viewer(res);
  const body = h("div", { class: "stack" });
  function closed() {
    fill(
      body,
      h("p", { class: "muted" }, "The spoiler is hidden. It is still saved with this seed, so you can open it later or keep a copy."),
      h(
        "div",
        { class: "row" },
        h("button", { type: "button", class: "btn", onclick: open }, "Show the spoiler"),
        h("button", { type: "button", class: "btn", onclick: () => save_text(name(res), text(res), "text/plain") }, "Download a copy of the spoiler"),
      ),
    );
  }
  function open() {
    fill(body, h("div", { class: "row" }, h("button", { type: "button", class: "btn", onclick: closed }, "Hide the spoiler again")), viewer(res));
  }
  closed();
  return body;
}
