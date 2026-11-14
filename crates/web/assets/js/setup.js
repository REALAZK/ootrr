import { h, fill, notice, debounce } from "./dom.js";
import { get, q } from "./api.js";
import { state, touch } from "./store.js";
import { pick_folder } from "./picker.js";
import { go, bar } from "./nav.js";

const NAME_RE = /^[A-Za-z0-9 _\-!.]{1,64}$/;

export function name_ok(s) {
  return NAME_RE.test(s) && !s.includes("..") && s.trim() === s;
}

function mark(kind, text) {
  const sym = kind === "ok" ? "✓" : kind === "bad" ? "✗" : "…";
  return h("li", null, h("span", { class: `mark ${kind}`, "aria-hidden": "true" }, sym), h("span", null, text));
}

export function render() {
  const game_out = h("div", { "aria-live": "polite" });
  const mod_out = h("div", { "aria-live": "polite" });
  const name_out = h("div", { "aria-live": "polite", class: "hint" });

  const game_in = h("input", { type: "text", id: "game", value: state.paths.game, spellcheck: "false", autocomplete: "off", placeholder: "Folder with the extracted game files" });
  const mod_in = h("input", { type: "text", id: "mod", value: state.paths.mod, spellcheck: "false", autocomplete: "off", placeholder: "Folder the game loads mods from" });
  const host_in = h(
    "select",
    { id: "host" },
    h("option", { value: "eden" }, "Lichii / Eden"),
    h("option", { value: "hookshot" }, "Hookshot"),
    h("option", { value: "longshot" }, "Longshot"),
    h("option", { value: "nx2navi" }, "NX2navi"),
  );
  host_in.value = state.paths.host || "eden";
  const music_in = h(
    "select",
    { id: "music" },
    h("option", { value: "off" }, "Off, keep the original music"),
    h("option", { value: "grouped" }, "Shuffle within each kind of track"),
    h("option", { value: "anywhere" }, "Shuffle across all tracks"),
  );
  music_in.value = state.cosmetic.music;
  const sfx_in = h(
    "select",
    { id: "sfx" },
    h("option", { value: "off" }, "Off, keep the original sounds"),
    h("option", { value: "grouped" }, "Shuffle within each kind of sound"),
    h("option", { value: "anywhere" }, "Shuffle across all sounds"),
  );
  sfx_in.value = state.cosmetic.sfx;
  sfx_in.addEventListener("change", () => {
    state.cosmetic.sfx = sfx_in.value;
    touch();
  });
  const colors_in = h(
    "select",
    { id: "colors" },
    h("option", { value: "off" }, "Off, keep the original colours"),
    h("option", { value: "random" }, "A random colour for each item"),
    h("option", { value: "shared" }, "One random hue turn for everything"),
  );
  colors_in.value = state.cosmetic.colors;
  colors_in.addEventListener("change", () => {
    state.cosmetic.colors = colors_in.value;
    touch();
  });
  const cos_seed = h("input", { type: "text", id: "cosseed", value: state.cosmetic.seed, spellcheck: "false", autocomplete: "off", maxlength: "64", placeholder: "Leave empty to use the seed text" });
  music_in.addEventListener("change", () => {
    state.cosmetic.music = music_in.value;
    touch();
  });
  cos_seed.addEventListener("input", () => {
    state.cosmetic.seed = cos_seed.value;
    touch();
  });
  const name_in = h("input", { type: "text", id: "name", value: state.paths.name, spellcheck: "false", autocomplete: "off", maxlength: "64" });

  async function check_game() {
    const p = state.paths.game.trim();
    state.game_check = null;
    if (!p) return fill(game_out, h("p", { class: "hint" }, "Optional for generating a seed. Needed to build the mod."));
    fill(game_out, h("ul", { class: "checks" }, mark("wait", "Checking")));
    try {
      const r = await get(`/api/validate?path=${q(p)}`);
      if (p !== state.paths.game.trim()) return;
      state.game_check = r;
      fill(
        game_out,
        h("ul", { class: "checks" }, r.checks.map((c) => mark(c.ok ? "ok" : "bad", c.label))),
        r.ok ? h("p", { class: "hint" }, `Game folder found at ${r.romfs}. ${r.version_note}`) : notice("error", r.problems.join(" ")),
      );
    } catch (e) {
      fill(game_out, notice("error", e.message));
    }
    bar();
  }

  async function check_mod() {
    const p = state.paths.mod.trim();
    state.mod_check = null;
    if (!p) return fill(mod_out, h("p", { class: "hint" }, "Needed to build the mod."));
    fill(mod_out, h("ul", { class: "checks" }, mark("wait", "Checking")));
    try {
      await get(`/api/fs?path=${q(p)}`);
      if (p !== state.paths.mod.trim()) return;
      state.mod_check = { ok: true };
      fill(mod_out, h("ul", { class: "checks" }, mark("ok", "Folder found")));
    } catch (e) {
      state.mod_check = { ok: false };
      fill(mod_out, notice("error", e.message));
    }
    bar();
  }

  function show_name() {
    const n = state.paths.name;
    if (!name_ok(n)) {
      return fill(name_out, h("span", { class: "mark bad" }, "Use 1 to 64 letters, digits, spaces and _ - ! . characters, with no spaces at the ends."));
    }
    const base = state.paths.mod.trim();
    const sep = state.info.sep;
    const flat = ["hookshot", "longshot", "nx2navi"].includes(state.paths.host);
    const mid = flat ? "" : `${state.info.title_id}${sep}`;
    fill(name_out, base ? `The mod will go to ${base}${base.endsWith(sep) ? "" : sep}${mid}${n}` : "");
  }

  const later_game = debounce(check_game, 350);
  const later_mod = debounce(check_mod, 350);

  game_in.addEventListener("input", () => {
    state.paths.game = game_in.value;
    touch();
    later_game();
  });
  mod_in.addEventListener("input", () => {
    state.paths.mod = mod_in.value;
    touch();
    show_name();
    later_mod();
  });
  name_in.addEventListener("input", () => {
    state.paths.name = name_in.value;
    touch();
    show_name();
  });
  host_in.addEventListener("change", () => {
    state.paths.host = host_in.value;
    touch();
    show_name();
  });

  async function browse(input, title, field, after) {
    const p = await pick_folder(input.value.trim(), title);
    if (p) {
      input.value = p;
      state.paths[field] = p;
      touch();
      show_name();
      after();
    }
  }

  check_game();
  check_mod();
  show_name();

  return h(
    "div",
    null,
    h("h1", null, "Setup"),
    h("p", { class: "muted" }, "Generating a seed needs no game files. Building the mod reads your extracted game folder and writes a mod. Nothing already in the game folder is changed."),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Where you play"),
      h("label", { class: "lbl", for: "host" }, "Game"),
      host_in,
      h("p", { class: "hint" }, "Lichii and Eden use their load folder. Hookshot, Longshot and NX2navi use the mods folder inside the game folder."),
    ),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Game folder"),
      h("label", { class: "lbl", for: "game" }, "Extracted game folder"),
      h("div", { class: "pathrow" }, game_in, h("button", { type: "button", class: "btn", onclick: () => browse(game_in, "Pick the game folder", "game", check_game) }, "Browse")),
      game_out,
    ),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Mod folder"),
      h("label", { class: "lbl", for: "mod" }, "Mod folder"),
      h("div", { class: "pathrow" }, mod_in, h("button", { type: "button", class: "btn", onclick: () => browse(mod_in, "Pick the mod folder", "mod", check_mod) }, "Browse")),
      mod_out,
      h("label", { class: "lbl", for: "name" }, "Mod name"),
      name_in,
      name_out,
    ),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Music"),
      h("p", { class: "hint" }, "Music changes only the mod you build, never the seed, so the same seed can be built with different music. Cutscene music keeps its own tracks. Themes built from stacked layers trade whole layer sets. Area themes that change with the time of day play one track all day. Jingles only trade with other jingles."),
      h("label", { class: "lbl", for: "music" }, "Music shuffle"),
      music_in,
      h("label", { class: "lbl", for: "sfx" }, "Sound effect shuffle"),
      sfx_in,
      h("label", { class: "lbl", for: "colors" }, "Colour shuffle"),
      colors_in,
      h("p", { class: "hint" }, "Colours cover the three tunics, the swords, the shields, Navi, hair, the boomerang gem, the gauntlet jewels and recovery hearts. Particle glows, trails and the menu keep their own colours."),
      h("label", { class: "lbl", for: "cosseed" }, "Cosmetic seed"),
      cos_seed,
    ),
    h("div", { class: "row" }, h("button", { type: "button", class: "btn primary", onclick: () => go("settings") }, "Continue to settings")),
  );
}

export async function probe() {
  const g = state.paths.game.trim();
  const m = state.paths.mod.trim();
  state.game_check = null;
  state.mod_check = null;
  if (g) {
    try {
      state.game_check = await get(`/api/validate?path=${q(g)}`);
    } catch {
      state.game_check = { ok: false };
    }
  }
  if (m) {
    try {
      await get(`/api/fs?path=${q(m)}`);
      state.mod_check = { ok: true };
    } catch {
      state.mod_check = { ok: false };
    }
  }
}
