import { h, fill, notice, debounce } from "./dom.js";
import { get, q, MODS, in_place } from "./api.js";
import { state, touch } from "./store.js";
import { pick_folder } from "./picker.js";
import { go, bar } from "./nav.js";

const NAME_RE = /^[A-Za-z0-9 _\-!.]{1,64}$/;
const AGAIN = "Choose the game folder again. The browser lets this page read it only after you pick it on this visit.";

export function name_ok(s) {
  return NAME_RE.test(s) && !s.includes("..") && s.trim() === s;
}

function mark(kind, text) {
  const sym = kind === "ok" ? "✓" : kind === "bad" ? "✗" : "…";
  return h("li", null, h("span", { class: `mark ${kind}`, "aria-hidden": "true" }, sym), h("span", null, text));
}

function web_paths() {
  state.paths.mod = MODS;
  state.mod_check = { ok: true };
}

export function render() {
  web_paths();
  const game_out = h("div", { "aria-live": "polite" });
  const name_out = h("div", { "aria-live": "polite", class: "hint" });

  const game_in = h("input", { type: "text", id: "game", value: state.paths.game, readonly: "", spellcheck: "false", autocomplete: "off", placeholder: "Choose the folder with the extracted game files" });
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
      const gone = !r.ok && r.problems.some((x) => /does not exist or cannot be read/.test(x));
      fill(
        game_out,
        gone ? null : h("ul", { class: "checks" }, r.checks.map((c) => mark(c.ok ? "ok" : "bad", c.label))),
        r.ok ? h("p", { class: "hint" }, `Game folder found at ${r.romfs}. ${r.version_note}`) : notice("error", gone ? AGAIN : r.problems.join(" ")),
      );
    } catch (e) {
      fill(game_out, notice("error", e.message));
    }
    bar();
  }

  function show_name() {
    const n = state.paths.name;
    if (!name_ok(n)) {
      return fill(name_out, h("span", { class: "mark bad" }, "Use 1 to 64 letters, digits, spaces and _ - ! . characters, with no spaces at the ends."));
    }
    const flat = ["hookshot", "longshot", "nx2navi"].includes(state.paths.host);
    fill(name_out, `The zip will hold ${flat ? "" : `${state.info.title_id}/`}${n}/romfs`);
  }

  const later_game = debounce(check_game, 350);

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

  async function browse() {
    const p = await pick_folder(game_in.value.trim(), "Pick the game folder", (n) => {
      fill(game_out, h("ul", { class: "checks" }, mark("wait", `Reading the list of ${n.toLocaleString()} files`)));
    });
    if (p) {
      game_in.value = p;
      state.paths.game = p;
      touch();
      later_game();
    }
  }

  check_game();
  show_name();

  const how = in_place()
    ? "Your browser asks to let this page view the folder. The files are read where they are, only the ones the build needs."
    : "Your browser lists every file in the folder first, which can take a minute for the whole game. The files are read where they are and are not uploaded anywhere.";

  return h(
    "div",
    null,
    h("h1", null, "Setup"),
    h("p", { class: "muted" }, "Everything runs on this computer. Generating a seed needs no game files. Building the mod reads your extracted game folder and gives you the mod as a zip. Nothing in the game folder is changed."),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Game folder"),
      h("label", { class: "lbl", for: "game" }, "Extracted game folder"),
      h("div", { class: "pathrow" }, game_in, h("button", { type: "button", class: "btn primary", onclick: browse }, "Choose folder")),
      h("p", { class: "hint" }, `Pick the romfs folder of the game, extracted with the 1.1.1 update. ${how}`),
      game_out,
    ),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Where you play"),
      h("label", { class: "lbl", for: "host" }, "Game"),
      host_in,
      h("p", { class: "hint" }, "Lichii and Eden load mods from their load folder. Hookshot, Longshot and NX2navi load them from the mods folder inside the game folder. The zip is laid out for the one you pick."),
    ),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Mod"),
      h("label", { class: "lbl", for: "name" }, "Mod name"),
      name_in,
      name_out,
      h("p", { class: "hint" }, "The mod downloads as a zip. Unzip it into the folder your emulator loads mods from."),
    ),
    h(
      "section",
      { class: "card" },
      h("h2", null, "Music"),
      h("p", { class: "hint" }, "Music changes only the mod you build, never the seed, so the same seed can be built with different music. Cutscene music keeps its own tracks. Themes built from stacked layers trade whole layer sets. Area themes that change with the time of day play one track all day. Jingles only trade with other jingles."),
      h("label", { class: "lbl", for: "music" }, "Music shuffle"),
      music_in,
      h("label", { class: "lbl", for: "cosseed" }, "Cosmetic seed"),
      cos_seed,
    ),
    h("div", { class: "row" }, h("button", { type: "button", class: "btn primary", onclick: () => go("settings") }, "Continue to settings")),
  );
}

export async function probe() {
  web_paths();
  const g = state.paths.game.trim();
  state.game_check = null;
  if (g) {
    try {
      state.game_check = await get(`/api/validate?path=${q(g)}`);
    } catch {
      state.game_check = { ok: false };
    }
  }
}
