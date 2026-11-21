import { h, fill, debounce } from "./dom.js";
import { get, q } from "./api.js";
import { state, touch } from "./store.js";
import { colour_editor } from "./cosmetics.js";
import { pick_folder, pick_package } from "./picker.js";
import { drop_zone } from "./drop.js";
import { tip_button } from "./tooltip.js";
import { marked } from "./controls.js";

const NAME_RE = /^[A-Za-z0-9 _\-!.]{1,64}$/;

export function name_ok(s) {
  return NAME_RE.test(s) && !s.includes("..") && s.trim() === s;
}

export function gen_missing() {
  const miss = [];
  if (!state.paths.game.trim()) miss.push("the extracted game folder");
  else if (update_needed() && !state.paths.update.trim()) miss.push("the v1.1.1 update folder");
  if (!state.paths.mod.trim()) miss.push("the mod folder");
  return miss.join(" and ");
}

export function update_needed() {
  const r = state.game_check;
  return !!(r && r.needs_update && !r.update_auto);
}

export function update_used() {
  const r = state.game_check;
  return !!(r && r.ok && r.update);
}

export function mod_target() {
  const base = state.paths.mod.trim();
  if (!base || !state.info) return "";
  const sep = state.info.sep;
  const flat = ["hookshot", "longshot", "nx2navi"].includes(state.paths.host);
  const mid = flat ? "" : `${state.info.title_id}${sep}`;
  return `${mid}${state.paths.name}, in the zip`;
}

const HOSTS = [
  { id: "eden", label: "Lichii / Eden" },
  { id: "hookshot", label: "Hookshot" },
  { id: "longshot", label: "Longshot" },
  { id: "nx2navi", label: "NX2navi" },
];
const SHUFFLE = (what) => [
  { id: "off", label: `Off, keep the original ${what}` },
  { id: "grouped", label: `Shuffle within each kind of ${what === "music" ? "track" : "sound"}` },
  { id: "anywhere", label: `Shuffle across all ${what === "music" ? "tracks" : "sounds"}` },
];

const CONFIG = { id: "config", label: "Configuration", heading: "Configuration" };
const COSMETICS = { id: "cosmetics", label: "Cosmetics", heading: "Configuration" };

export const FIELDS = [
  { id: "host", page: CONFIG, section: "Where you play", label: "Game", choices: HOSTS,
    tip: "Lichii and Eden use their load folder. Hookshot, Longshot and NX2navi use the mods folder inside the game folder." },
  { id: "game", page: CONFIG, section: "Folders", label: "Extracted game folder (v1.1.1)",
    tip: "The folder with your extracted game files, version 1.1.1 (the game with its 1.1.1 update installed), or the game packed as a .nup or .nx (Package…). Version 1.0.0 is not enough on its own: if a 1.0.0 game is picked, a second field for the 1.1.1 update appears below it. A package holding the base and its update is read as both. Games are only read, never changed. From a package, only the files the build reads (about 370 MB, or about 2.3 GB with music or sound shuffle) are unpacked for each build into the app's data folder and deleted afterwards." },
  { id: "mod", page: CONFIG, section: "Folders", label: "Mod download", tip: "The browser builds the mod and offers it as a zip. Nothing is uploaded." },
  { id: "name", page: CONFIG, section: "Folders", label: "Mod name", tip: "The name of the mod's own folder. Use 1 to 64 letters, digits, spaces and _ - ! . characters, with no spaces at the ends." },
  { id: "seed", page: CONFIG, section: "Seed", label: "Seed",
    tip: "Type a seed to make the same game again. Left empty, Generate picks a random seed. This box is never filled in for you. The seed that was used is in the spoiler log and on the Seed page." },
  { id: "spoiler_log", page: CONFIG, section: "Seed", label: "Include Spoiler Log",
    tip: "Writes Spoiler.txt into the mod folder next to the mod. It lists the seed, the settings string, every setting used, the expected playthrough step by step, and the item at every location. Off, no spoiler is made and an old Spoiler.txt is removed." },
  { id: "music", page: COSMETICS, section: "Music and sound", label: "Music shuffle", choices: SHUFFLE("music"),
    tip: "Music changes only the mod you build, never the seed, so the same seed can be built with different music. Cutscene music keeps its own tracks. Themes built from stacked layers trade whole layer sets. Area themes that change with the time of day play one track all day. Jingles only trade with other jingles." },
  { id: "sfx", page: COSMETICS, section: "Music and sound", label: "Sound effect shuffle", choices: SHUFFLE("sounds"), tip: "Shuffles sound effects in the mod you build. The seed is not changed." },
  { id: "title_logo", page: COSMETICS, section: "Title screen", label: "Randomizer line under the logo",
    tip: "Draws a Randomizer line under the title screen logo. It changes the title screen files, so turn it off if another mod (a title screen, menu or UI mod) changes them too. The build stops and names the other mod when they clash. The seed is not changed." },
  { id: "colors", page: COSMETICS, section: "Colours", label: "Part colours",
    tip: "Colours cover the three tunics, the swords, the shields, Navi, hair, the boomerang gem, the gauntlet jewels and recovery hearts. Pick a colour with a part's swatch, or roll one with its die. Randomize unlocked gives every unlocked part its own random colour, vivid, deep, pastel, soft, dark, muted or a grey. Shades of one colour picks one colour and gives every unlocked part its own lighter, darker or softer shade of it. Lock a part or a whole group to keep its colour through these buttons. With Re-roll unlocked parts on every build, the unlocked parts get new colours at each build and the locked ones keep theirs. Only Link's own tunic is recoloured: Mido and the Fairy Ocarina holder keep theirs. Particle glows, trails and the menu keep their own colours. The swatch of a part left original is only a rough guide to the game's colour." },
  { id: "cosseed", page: COSMETICS, section: "Colours", label: "Cosmetic seed", tip: "Leave empty for new random music, sounds and re-rolled colours on every build. The build notes name the cosmetic seed that was used: type it here to build the same ones again. Colours picked or rolled on this page do not depend on it." },
].map((f) => ({ ...f, page_label: f.page.label, heading: f.page.heading, page: f.page.id }));

function set_status(el, kind, title) {
  el.className = `stat ${kind}`;
  el.textContent = kind === "ok" ? "✓" : kind === "bad" ? "✗" : "";
  el.title = title;
  el.setAttribute("aria-label", title);
  el.hidden = kind === "none";
}

function row(f, terms, control, extra, badge) {
  const tip = tip_button(f.label, () => f.tip);
  const head = f.id === "spoiler_log"
    ? h("div", { class: "wl" }, h("label", { class: "chk", for: `f_${f.id}` }, control, h("span", null, marked(f.label, terms))), tip)
    : h("div", { class: "wl" }, h("label", { for: `f_${f.id}` }, marked(f.label, terms)), tip, badge || null);
  return h("div", { class: "w", "data-id": f.id }, head, f.id === "spoiler_log" ? null : control, extra);
}

function select(id, choices, value, onchange) {
  const sel = h("select", { id: `f_${id}` }, choices.map((c) => h("option", { value: c.id }, c.label)));
  sel.value = value;
  sel.addEventListener("change", () => onchange(sel.value));
  return sel;
}

const PACKAGE = /\.(nup|nx)$/i;

// Where a picker starts: the folder itself, or the folder holding a picked package file.
function start_of(v) {
  const t = v.trim();
  if (!PACKAGE.test(t)) return t;
  const i = Math.max(t.lastIndexOf("/"), t.lastIndexOf("\\"));
  return i > 0 ? t.slice(0, i) : "";
}

function pick_button(label, aria, pick, start, title, done) {
  const b = h("button", { type: "button", class: "btn", "aria-label": aria, onclick: async () => {
    b.disabled = true;
    let v = null;
    try {
      v = await pick(start_of(start()), title);
    } finally {
      b.disabled = false;
    }
    if (v) done(v);
  } }, label);
  return b;
}

// The v1.1.1 update folder and the "add it to the mod" box. They belong to the game folder
// field and are only shown when they matter: the folder only for a v1.0.0 game, the box
// only while a separate update is in use.
function update_box(recheck) {
  const p = state.paths;
  const out = h("div", { class: "why err", "aria-live": "polite" });
  const stat = h("span", { class: "stat", role: "img", hidden: true });
  const inp = h("input", { type: "text", id: "f_update", value: p.update, spellcheck: "false", autocomplete: "off", placeholder: "Folder, .nup or .nx with the v1.1.1 update" });
  const later = debounce(recheck, 350);
  inp.addEventListener("input", () => {
    p.update = inp.value;
    touch();
    later();
  });
  const set = (v) => {
    inp.value = v;
    p.update = v;
    touch();
    recheck();
  };
  const from = () => inp.value.trim() || p.game.trim();
  const browse = pick_button("Browse", "Browse for the v1.1.1 update folder", pick_folder, from, "Pick the v1.1.1 update folder", set);
  const pack = pick_button("Package…", "Pick the v1.1.1 update as a .nup or .nx file", pick_package, from, "Pick the v1.1.1 update .nup or .nx", set);
  // One click to use an update found next to the game folder; shown only while the field is empty.
  const use = h("button", { type: "button", class: "btn small", onclick: () => use.dataset.path && set(use.dataset.path) });
  const hint = h("div", { class: "pathrow", hidden: true }, use);
  const tip = tip_button("v1.1.1 update folder", () => "Needed only because the game above is version 1.0.0. Pick the folder with the 1.1.1 update's files, the one holding its romfs folder (only the files the update changes are needed), or the update packed as a .nup or .nx (Package…). A folder is laid over the 1.0.0 files using links in the app's data folder, so no game files are copied; from a package, only the files the build reads are unpacked for each build and deleted afterwards. It is only read, never changed.");
  const folder = h("div", { class: "w sub", hidden: true },
    h("div", { class: "wl" }, h("label", { for: "f_update" }, "v1.1.1 update folder or package (required)"), tip, stat),
    h("div", { class: "pathrow" }, inp, browse, pack), hint, out);
  const box = h("input", { type: "checkbox", id: "f_include_update", checked: state.include_update });
  box.addEventListener("change", () => {
    state.include_update = box.checked;
    touch();
  });
  const btip = tip_button("Include the v1.1.1 update in the mod", () => "Adds the 1.1.1 update's own romfs and exefs files to the mod folder, under the randomizer's files, so the mod also works where only version 1.0.0 is installed. Leave it off if your emulator already has the 1.1.1 update installed. The files are copied, never linked; on APFS, Btrfs and ReFS drives the copies take no extra space. A mod built this way holds game files: never share it.");
  const include = h("div", { class: "w sub", hidden: true },
    h("div", { class: "wl" }, h("label", { class: "chk", for: "f_include_update" }, box, h("span", null, "Include the v1.1.1 update in the mod")), btip));
  const show = (r) => {
    const need = !!(r && r.needs_update);
    folder.hidden = !need || !!(r && r.update_auto);
    include.hidden = !(r && r.ok && r.update);
    out.textContent = "";
    const sug = !folder.hidden && !p.update.trim() && r && r.update_hint;
    hint.hidden = !sug;
    if (sug) {
      const parts = r.update_hint.split(/[\\/]/).filter(Boolean);
      use.textContent = `Use ${parts.slice(-2).join("/")}`;
      use.title = r.update_hint;
      use.setAttribute("aria-label", `Use the v1.1.1 update found at ${r.update_hint}`);
      use.dataset.path = r.update_hint;
    }
    if (!need || folder.hidden || !p.update.trim()) return set_status(stat, "none", "");
    if (r.update_problems && r.update_problems.length) {
      set_status(stat, "bad", "This is not a usable v1.1.1 update folder");
      out.textContent = r.update_problems.join(" ");
    } else if (r.update) set_status(stat, "ok", `v1.1.1 update found at ${r.update}`);
    else set_status(stat, "none", "");
  };
  return { el: h("div", null, folder, include), show };
}

function found_text(r) {
  const what = r.pack ? `${r.pack} package` : "Game folder";
  if (!r.update) return `${what} found at ${r.romfs}`;
  if (r.update === r.romfs) return `${what} found at ${r.romfs}, with its v1.1.1 update inside`;
  return `v1.0.0 game at ${r.romfs}, with the v1.1.1 update at ${r.update}`;
}

export function field_el(f, terms = []) {
  const p = state.paths;
  const cos = state.cosmetic;
  if (f.id === "host") {
    return row(f, terms, select("host", HOSTS, p.host || "eden", (v) => {
      p.host = v;
      touch();
    }));
  }
  if (f.id === "mod") return row(f, terms, h("p", { class: "muted" }, "In the browser the mod is built in memory on your computer and offered as a zip download when it is ready. Unzip it into the folder your emulator loads mods from."));
  if (f.id === "game" || f.id === "mod") {
    const out = h("div", { class: "why err", "aria-live": "polite" });
    const upd = f.id === "game" ? update_box(() => check()) : null;
    const stat = h("span", { class: "stat", role: "img", hidden: true });
    const inp = h("input", { type: "text", id: `f_${f.id}`, value: p[f.id], spellcheck: "false", autocomplete: "off",
      placeholder: f.id === "game" ? "Folder with the extracted game files, or a .nup or .nx" : "Folder the game loads mods from" });
    const check = async () => {
      const v = p[f.id].trim();
      out.textContent = "";
      if (!v) {
        if (upd) {
          state.game_check = null;
          upd.show(null);
        }
        return set_status(stat, "none", "");
      }
      set_status(stat, "wait", "Checking the folder");
      try {
        if (f.id === "game") {
          const u = p.update.trim();
          const r = await get(`/api/validate?path=${q(v)}${u ? `&update=${q(u)}` : ""}`);
          if (v !== p.game.trim() || u !== p.update.trim()) return;
          state.game_check = r;
          if (r.ok) set_status(stat, "ok", found_text(r));
          else {
            set_status(stat, "bad", "This is not a usable game folder");
            out.textContent = r.problems.join(" ");
          }
          upd.show(r);
          touch();
        } else {
          await get(`/api/fs?path=${q(v)}`);
          if (v !== p.mod.trim()) return;
          state.mod_check = { ok: true };
          set_status(stat, "ok", "Folder found");
        }
      } catch (e) {
        if (f.id === "mod") state.mod_check = { ok: false };
        if (upd) upd.show(null);
        set_status(stat, "bad", e.message);
        out.textContent = e.message;
      }
    };
    const later = debounce(check, 350);
    inp.addEventListener("input", () => {
      p[f.id] = inp.value;
      touch();
      later();
    });
    const set = (v) => {
      inp.value = v;
      p[f.id] = v;
      touch();
      check();
    };
    const from = () => inp.value.trim();
    const browse = pick_button("Browse", f.id === "game" ? "Browse for the game folder" : "Browse for the mod folder", pick_folder, from, f.id === "game" ? "Pick the game folder" : "Pick the mod folder", set);
    const pack = f.id === "game" ? pick_button("Package…", "Pick the game as a .nup or .nx file", pick_package, from, "Pick the game .nup or .nx", set) : null;
    check();
    return row(f, terms, h("div", { class: "pathrow" }, inp, browse, pack), [out, f.id === "game" ? drop_zone(set) : null, upd && upd.el], stat);
  }
  if (f.id === "name") {
    const out = h("div", { class: "hint", "aria-live": "polite" });
    const inp = h("input", { type: "text", id: "f_name", value: p.name, spellcheck: "false", autocomplete: "off", maxlength: "64" });
    const show = () => {
      if (!name_ok(p.name)) return fill(out, h("span", { class: "mark bad" }, "Use 1 to 64 letters, digits, spaces and _ - ! . characters, with no spaces at the ends."));
      const t = mod_target();
      fill(out, t ? `The mod will go to ${t}` : "");
    };
    inp.addEventListener("input", () => {
      p.name = inp.value;
      touch();
      show();
    });
    show();
    return row(f, terms, inp, out);
  }
  if (f.id === "seed") {
    const inp = h("input", { type: "text", id: "f_seed", value: state.seed, maxlength: "100", spellcheck: "false", autocomplete: "off", placeholder: "Leave empty for a random seed" });
    inp.addEventListener("input", () => {
      state.seed = inp.value;
      touch();
    });
    return row(f, terms, inp);
  }
  if (f.id === "spoiler_log") {
    const box = h("input", { type: "checkbox", id: "f_spoiler_log", checked: state.spoiler_log });
    box.addEventListener("change", () => {
      state.spoiler_log = box.checked;
      touch();
    });
    return row(f, terms, box);
  }
  if (f.id === "title_logo") {
    const box = h("input", { type: "checkbox", id: "f_title_logo", checked: cos.title !== false });
    box.addEventListener("change", () => {
      cos.title = box.checked;
      touch();
    });
    return row(f, terms, box);
  }
  if (f.id === "colors") return row(f, terms, colour_editor());
  if (f.id === "music" || f.id === "sfx") {
    return row(f, terms, select(f.id, f.choices, cos[f.id], (v) => {
      cos[f.id] = v;
      touch();
    }));
  }
  const inp = h("input", { type: "text", id: "f_cosseed", value: cos.seed, spellcheck: "false", autocomplete: "off", maxlength: "64", placeholder: "Leave empty for new music and colours each build" });
  inp.addEventListener("input", () => {
    cos.seed = inp.value;
    touch();
  });
  return row(f, terms, inp);
}

export function gen_line() {
  const line = h("p", { class: "genline", "aria-live": "polite" });
  const draw = () => {
    const miss = gen_missing();
    line.classList.toggle("ready", !miss);
    line.textContent = miss
      ? `Fill in ${miss}, then use Generate at the top right.`
      : `Generate, at the top right, writes the mod to ${mod_target()}${state.spoiler_log ? ", with Spoiler.txt beside it" : ""}.`;
  };
  draw();
  return { el: line, draw };
}

export function page_fields(page) {
  const secs = new Map();
  for (const f of FIELDS.filter((x) => x.page === page)) {
    if (!secs.has(f.section)) secs.set(f.section, []);
    secs.get(f.section).push(f);
  }
  return secs;
}
