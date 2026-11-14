import { $, h, fill, notice, toast } from "./dom.js";
import { pick_folder } from "./picker.js";
import { t_open, t_job, t_seed } from "./tracker_api.js";
import { ts, main_paths } from "./tracker_store.js";

let reload_all = async () => {};

export function set_reload(fn) {
  reload_all = fn;
}

function ago(s) {
  if (s === null || s === undefined) return "";
  if (s < 60) return `${s} s ago`;
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  return `${Math.floor(s / 3600)} h ago`;
}

export function paint_live() {
  const box = $("#live");
  if (!box || !ts.state) return;
  const s = ts.state.source || {};
  let cls = "live";
  let text = "No save folder set";
  if (!ts.online) {
    cls += " bad";
    text = "The local server does not answer";
  } else if (s.error) {
    cls += " bad";
    text = `Save not readable yet. ${s.error}`;
  } else if (s.folder_ok && s.reads > 0) {
    cls += " on";
    const age = s.read_at ? Math.max(0, Math.floor(Date.now() / 1000) - s.read_at) : null;
    text = `Live. Save written ${ago(age)}, ${(s.slot || "").replace("_", " ")}`;
  } else if (s.folder_ok) {
    cls += " wait";
    text = "Watching, waiting for a save";
  }
  box.className = cls;
  box.textContent = text;
}

async function follow(id, out) {
  for (let i = 0; i < 600; i += 1) {
    let j;
    try {
      j = await t_job(id);
    } catch (e) {
      fill(out, notice("error", e.message));
      return;
    }
    fill(out, h("p", { class: "muted small" }, (j.steps || []).map((s) => s.label).join(". ") || "Working"));
    if (j.finished) {
      fill(out, notice(j.state === "done" ? "ok" : "error", j.message || j.state));
      await reload_all();
      return;
    }
    await new Promise((r) => setTimeout(r, 500));
  }
}

function field(label, value, title, hint) {
  const input = h("input", { type: "text", value, spellcheck: "false", autocomplete: "off", "aria-label": label });
  const browse = h("button", { type: "button", class: "btn small" }, "Browse");
  browse.addEventListener("click", async () => {
    const p = await pick_folder(input.value.trim(), title);
    if (p) input.value = p;
  });
  return { input, el: h("div", null, h("label", { class: "lbl" }, label), h("div", { class: "pathrow" }, input, browse), hint ? h("p", { class: "hint" }, hint) : null) };
}

function seed_box(s) {
  const out = h("div", { "aria-live": "polite" });
  const sd = (ts.state && ts.state.seed) || {};
  const pick = h("input", { type: "file", accept: ".ootrr,application/json", "aria-label": "Seed file" });
  const apply = async (text) => {
    try {
      const r = await t_seed(text);
      fill(out, notice("ok", r.loaded ? `Seed loaded, ${r.rewired} gift rows follow it` : "Seed cleared, gift rows assume the vanilla items"));
      await reload_all();
    } catch (e) {
      fill(out, notice("error", e.message));
    }
  };
  pick.addEventListener("change", async () => {
    const f = pick.files && pick.files[0];
    if (f) await apply(await f.text());
  });
  const clear = h("button", { type: "button", class: "btn small", onclick: () => apply("") }, "Clear seed");
  return h(
    "div",
    null,
    h("label", { class: "lbl" }, "Seed file"),
    pick,
    h("p", { class: "hint" }, sd.loaded ? `Loaded seed ${sd.seed}, ${sd.rewired} gift rows follow it.` : "Optional. Gift rows such as songs and rewards assume the vanilla items until a seed is loaded."),
    sd.loaded ? clear : null,
    out,
  );
}

export function render_source() {
  const s = (ts.state && ts.state.source) || {};
  const hosts = (ts.state && ts.state.hosts) || [];
  const mp = main_paths();
  const save = field("Save folder", s.asked || "", "Choose the save folder", "Only read, never written. The tracker finds the slot folders inside it.");
  const game = field("Game folder", s.game_dir || mp.game, "Choose the game folder", "Your own extracted game, the same folder as on the Setup page. It is used to place each location on a map. Optional.");
  const slot = h(
    "select",
    { "aria-label": "Slot" },
    h("option", { value: "" }, "Follow the newest slot"),
    (s.slots || []).map((n) => h("option", { value: n, selected: s.pinned === n }, n)),
  );
  const out = h("div", { "aria-live": "polite" });
  const host_btns = hosts.map((x) =>
    h(
      "button",
      {
        type: "button",
        class: "btn small",
        title: x.hint,
        onclick: async () => {
          if (x.path) {
            save.input.value = x.path;
            return;
          }
          const p = await pick_folder("", `Choose your ${x.label} folder`);
          if (p) save.input.value = p;
        },
      },
      x.path ? `Use ${x.label}` : `Find ${x.label}`,
    ),
  );
  const go = h("button", { type: "button", class: "btn primary" }, "Start tracking");
  go.addEventListener("click", async () => {
    go.disabled = true;
    fill(out, h("p", { class: "loading" }, "Starting"));
    try {
      const body = { save_dir: save.input.value.trim(), game_dir: game.input.value.trim(), slot: slot.value };
      const r = await t_open(body);
      fill(out, r.save_folder ? notice("ok", `Watching ${r.save_folder}`) : null);
      if (r.job_id) await follow(r.job_id, out);
      else await reload_all();
    } catch (e) {
      fill(out, notice("error", e.message));
    } finally {
      go.disabled = false;
    }
  });
  const facts = h(
    "dl",
    { class: "kv" },
    h("dt", null, "Watching"),
    h("dd", { class: "mono small" }, s.folder || "nothing yet"),
    h("dt", null, "Slot"),
    h("dd", null, s.slot || "none read yet"),
    h("dt", null, "Objects in save"),
    h("dd", null, String(s.keys || 0)),
    h("dt", null, "Map data"),
    h("dd", null, s.game ? "placed from your game folder" : s.job !== null && s.job !== undefined ? "being read" : "not loaded, the list still works"),
  );
  return h(
    "section",
    { class: "src", "aria-label": "Save source" },
    h("h2", null, "Your save"),
    h("p", { class: "hint" }, "Nothing is set until you choose it here. The save folder is only read, never written."),
    h("div", { class: "hostrow" }, host_btns),
    save.el,
    h("label", { class: "lbl" }, "Slot"),
    slot,
    game.el,
    seed_box(s),
    s.index_error ? notice("error", s.index_error) : null,
    h("div", { class: "row" }, go),
    out,
    facts,
  );
}

let shown = null;

export function close_source() {
  if (shown) shown.remove();
  shown = null;
}

export function open_source() {
  close_source();
  const overlay = h(
    "div",
    { class: "overlay", onclick: (e) => e.target === overlay && close_source() },
    h(
      "div",
      { class: "dialog", role: "dialog", "aria-modal": "true", "aria-label": "Save and game folders" },
      h("div", { class: "body" }, render_source()),
      h("footer", null, h("button", { type: "button", class: "btn", onclick: close_source }, "Close")),
    ),
  );
  shown = overlay;
  document.body.append(overlay);
  const first = overlay.querySelector("input");
  if (first) first.focus();
}

export function refresh_source() {
  if (!shown) return;
  const body = shown.querySelector(".body");
  if (body && !body.contains(document.activeElement)) fill(body, render_source());
}

export function offline(on) {
  if (ts.online === !on) return;
  ts.online = !on;
  paint_live();
  if (on) toast("Lost the local server, retrying");
}
