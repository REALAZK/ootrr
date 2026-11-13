import { h, fill, save_text } from "./dom.js";
import { viewer } from "./viewer.js";

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
