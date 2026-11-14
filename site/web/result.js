import { h, fill, notice, save_text } from "./dom.js";
import { get, post, last_zip } from "./api.js";
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

function zip_name() {
  return `${state.paths.name}.zip`;
}

function save_zip() {
  const z = last_zip();
  if (!z) return false;
  const a = h("a", { href: URL.createObjectURL(z), download: zip_name() });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 60000);
  return true;
}

function instructions() {
  const flat = ["hookshot", "longshot", "nx2navi"].includes(state.paths.host);
  const inside = flat ? `${state.paths.name}/romfs` : `${state.info.title_id}/${state.paths.name}/romfs`;
  return h(
    "section",
    { class: "card" },
    h("h2", null, state.paths.host === "longshot" ? "Using the mod in Longshot" : "Using the mod in your emulator"),
    state.paths.host === "longshot"
      ? h(
          "ol",
          null,
          h("li", null, "Build the mod and download the zip."),
          h("li", null, "Unzip it into the mods folder in your Longshot game folder, beside bin\\longshot.exe."),
          h("li", null, "Turn other mods off by putting _ or - in front of their folder names. Of the mods that ship a size table, the game uses the one whose name sorts last."),
          h("li", null, "Start Longshot and begin a fresh save. longshot_log.txt lists each mod it loaded."),
        )
      : h(
          "ol",
          null,
          h("li", null, flat
            ? "Build the mod and download the zip. Unzip it into the mods folder inside the game folder. For Hookshot, name it randomizer-<seed> so co-op can match the seed."
            : "Build the mod and download the zip. Unzip it into the load folder of your emulator."),
          h("li", null, "In the emulator, open the game's properties and tick your mod in the add-ons list."),
          h("li", null, "Start the game from a fresh save."),
        ),
    h("p", { class: "muted small" }, `Inside the zip: ${inside}`),
  );
}

export function render() {
  const res = state.result;
  const p = state.paths;
  const out = h("div", { "aria-live": "polite" });
  const btn = h("button", { type: "button", class: "btn primary" }, "Build mod");
  const dl = h("button", { type: "button", class: "btn", disabled: "" }, "Download mod zip");

  const missing = [];
  if (!state.game_check || !state.game_check.ok) missing.push("a valid game folder");
  if (!name_ok(p.name)) missing.push("a valid mod name");

  async function poll(id) {
    for (let i = 0; i < 600; i++) {
      const job = await get(`/api/jobs/${id}`);
      const ready = job.finished && job.state === "done" && last_zip();
      fill(
        out,
        h(
          "section",
          { class: "card" },
          h("h2", null, "Build"),
          job.finished ? null : h("progress", { max: "100", value: String(job.percent), "aria-label": "Build progress" }),
          steps_view(job),
          job.message ? notice(job.state === "failed" ? "error" : job.state === "done" ? "ok" : "info", job.message) : null,
          ready ? notice("ok", `${zip_name()} is ready, ${(last_zip().size / 1e6).toFixed(1)} MB. Your download should start. If it does not, press Download mod zip.`) : null,
        ),
      );
      if (job.finished) {
        state.build_note = job.message || job.state;
        if (ready) {
          dl.disabled = false;
          save_zip();
        }
        return;
      }
      await new Promise((r) => setTimeout(r, 400));
    }
  }

  async function build() {
    btn.disabled = true;
    dl.disabled = true;
    fill(out, h("p", { class: "loading" }, "Building"));
    try {
      const r = await post("/api/build", { seed_text: res.seed_text, game_dir: p.game.trim(), mod_dir: p.mod.trim(), mod_name: p.name, host: p.host || "eden", music: state.cosmetic.music, cosmetic_seed: state.cosmetic.seed });
      await poll(r.job_id);
    } catch (e) {
      fill(out, notice("error", e.message));
    }
    btn.disabled = false;
  }

  btn.onclick = build;
  dl.onclick = save_zip;
  btn.disabled = !res || missing.length > 0;
  dl.disabled = !last_zip();

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
      h("dl", { class: "kv" }, h("dt", null, "Game folder"), h("dd", null, p.game || "Not set"), h("dt", null, "Output"), h("dd", null, zip_name()), h("dt", null, "Mod name"), h("dd", null, p.name)),
      missing.length ? notice("warn", `Still needed: ${missing.join(", ")}. `, h("a", { href: "#/setup" }, "Go to setup")) : null,
      h("div", { class: "row" }, btn, dl),
      out,
    ),
    instructions(),
  );
}
