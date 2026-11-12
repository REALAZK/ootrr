import { h, fill, notice, save_text } from "./dom.js";
import { get, post } from "./api.js";
import { state } from "./store.js";
import { go } from "./nav.js";
import { name_ok } from "./setup.js";
import { button as report_button } from "./report.js";

const SYMS = { done: ["ok", "✓"], failed: ["bad", "✗"], unavailable: ["wait", "–"], running: ["wait", "…"] };

function steps_view(job) {
  return h(
    "ul",
    { class: "steps-list" },
    job.steps.map((s) => {
      const [k, c] = SYMS[s.state] || SYMS.running;
      return h("li", null, h("span", { class: `mark ${k}`, "aria-hidden": "true" }, c), h("span", null, s.label));
    }),
  );
}

function instructions() {
  const sep = state.info.sep;
  return h(
    "section",
    { class: "card" },
    h("h2", null, "Using the mod in your emulator"),
    h(
      "ol",
      null,
      h("li", null, state.paths.host === "hookshot" || state.paths.host === "nx2navi"
        ? "Build the mod. It is written as mods/<name>/romfs in the folder you chose. For Hookshot, name it randomizer-<seed> so co-op can match the seed."
        : `Build the mod. It is written to the mod folder you chose, inside a folder named ${state.info.title_id} and then your mod name.`),
      h("li", null, "Make sure that mod folder is the one your emulator loads mods from."),
      h("li", null, "In the emulator, open the game's properties and tick your mod in the add-ons list."),
      h("li", null, "Start the game from a fresh save."),
    ),
    h("p", { class: "muted small" }, `Layout on disk: mod folder${sep}${state.info.title_id}${sep}name${sep}romfs`),
  );
}

export function render() {
  const res = state.result;
  const p = state.paths;
  const out = h("div", { "aria-live": "polite" });
  const btn = h("button", { type: "button", class: "btn primary" }, "Build mod");
  const open_btn = h("button", { type: "button", class: "btn" }, "Open mod folder");
  const open_out = h("div");

  const missing = [];
  if (!state.game_check || !state.game_check.ok) missing.push("a valid game folder");
  if (!p.mod.trim()) missing.push("a mod folder");
  else if (state.mod_check && !state.mod_check.ok) missing.push("a mod folder that exists");
  if (!name_ok(p.name)) missing.push("a valid mod name");

  async function poll(id) {
    for (let i = 0; i < 600; i++) {
      const job = await get(`/api/jobs/${id}`);
      fill(out, h("section", { class: "card" }, h("h2", null, "Build"), job.finished ? null : h("progress", { max: "100", value: String(job.percent), "aria-label": "Build progress" }), steps_view(job), job.message ? notice(job.state === "failed" ? "error" : job.state === "done" ? "ok" : "info", job.message) : null));
      if (job.finished) {
        state.build_note = job.message || job.state;
        return;
      }
      await new Promise((r) => setTimeout(r, 400));
    }
  }

  async function build() {
    btn.disabled = true;
    fill(out, h("p", { class: "loading" }, "Starting"));
    try {
      const r = await post("/api/build", { seed_text: res.seed_text, game_dir: p.game.trim(), mod_dir: p.mod.trim(), mod_name: p.name, host: p.host || "eden" });
      await poll(r.job_id);
    } catch (e) {
      fill(out, notice("error", e.message));
    }
    btn.disabled = false;
  }

  async function open_folder() {
    try {
      await post("/api/open-folder", { path: p.mod.trim() });
      fill(open_out, null);
    } catch (e) {
      fill(open_out, notice("error", e.message));
    }
  }

  btn.onclick = build;
  open_btn.onclick = open_folder;
  btn.disabled = !res || missing.length > 0;
  open_btn.disabled = !p.mod.trim();

  if (!res) {
    return h(
      "div",
      null,
      h("h1", null, "Result"),
      notice("info", "There is no seed yet. Generate one or open a seed file first."),
      h("button", { type: "button", class: "btn primary", onclick: () => go("generate") }, "Go to generate"),
    );
  }

  return h(
    "div",
    null,
    h("h1", null, "Result"),
    h(
      "section",
      { class: "card stack" },
      h("h2", null, "Seed"),
      h("dl", { class: "kv" }, h("dt", null, "Seed"), h("dd", { class: "mono" }, res.seed), h("dt", null, "Checksum"), h("dd", { class: "mono" }, res.checksum), h("dt", null, "Placements"), h("dd", null, String(res.placement_count))),
      res.placements_ready ? null : notice("warn", "This seed file has no item placements yet, so a built mod would change nothing."),
      h("div", { class: "row" }, h("button", { type: "button", class: "btn", onclick: () => save_text(res.file_name || "seed.ootrr", res.seed_text) }, "Download seed file"), report_button(res)),
    ),
    h(
      "section",
      { class: "card stack" },
      h("h2", null, "Build the mod"),
      h("dl", { class: "kv" }, h("dt", null, "Game folder"), h("dd", null, p.game || "Not set"), h("dt", null, "Mod folder"), h("dd", null, p.mod || "Not set"), h("dt", null, "Mod name"), h("dd", null, p.name)),
      missing.length ? notice("warn", `Still needed: ${missing.join(", ")}. `, h("a", { href: "#/setup" }, "Go to setup")) : null,
      h("div", { class: "row" }, btn, open_btn),
      open_out,
      out,
    ),
    instructions(),
  );
}
