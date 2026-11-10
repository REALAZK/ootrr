import { h, fill, notice } from "./dom.js";
import { get, q } from "./api.js";
import { state } from "./store.js";

export function pick_folder(start, title) {
  return new Promise((resolve) => {
    const opener = document.activeElement;
    const body = h("div", { class: "body" });
    const input = h("input", { type: "text", "aria-label": "Folder path", spellcheck: "false", autocomplete: "off" });
    const use = h("button", { type: "button", class: "btn primary" }, "Use this folder");
    const up = h("button", { type: "button", class: "btn" }, "Up");
    let here = "";
    let parent = "";

    function close(v) {
      document.removeEventListener("keydown", key, true);
      overlay.remove();
      if (opener && opener.focus) opener.focus();
      resolve(v);
    }

    function key(e) {
      if (e.key === "Escape") {
        e.preventDefault();
        close(null);
      }
    }

    async function load(p) {
      fill(body, h("p", { class: "loading" }, "Loading"));
      try {
        const r = await get(`/api/fs?path=${q(p)}`);
        here = r.path;
        parent = r.parent || "";
        input.value = here;
        use.disabled = here === "";
        up.disabled = here === "";
        const list = h("ul", { class: "ents" });
        for (const x of r.roots) {
          list.append(h("li", null, h("button", { type: "button", onclick: () => load(x.path) }, h("span", null, "▣"), x.label)));
        }
        for (const e of r.entries) {
          const dir = e.kind === "dir";
          const full = here + (here.endsWith(state.info.sep) ? "" : state.info.sep) + e.name;
          list.append(
            h("li", null, h("button", { type: "button", disabled: !dir, onclick: () => load(full) }, h("span", { "aria-hidden": "true" }, dir ? "▸" : "·"), e.name)),
          );
        }
        fill(
          body,
          r.truncated ? notice("warn", "This folder is large, only the first entries are shown. Type the path above to go deeper.") : null,
          r.roots.length === 0 && r.entries.length === 0 ? h("p", { class: "muted" }, "This folder is empty.") : list,
        );
      } catch (e) {
        fill(body, notice("error", e.message));
        use.disabled = true;
      }
    }

    up.onclick = () => load(parent);
    use.onclick = () => close(here);
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") load(input.value.trim());
    });

    const overlay = h(
      "div",
      { class: "overlay", onclick: (e) => e.target === overlay && close(null) },
      h(
        "div",
        { class: "dialog", role: "dialog", "aria-modal": "true", "aria-label": title },
        h("header", null, h("h2", null, title), h("div", { class: "pathrow" }, up, input, h("button", { type: "button", class: "btn", onclick: () => load(input.value.trim()) }, "Go"))),
        body,
        h("footer", null, h("button", { type: "button", class: "btn", onclick: () => close(null) }, "Cancel"), use),
      ),
    );
    document.addEventListener("keydown", key, true);
    document.body.append(overlay);
    input.focus();
    load(start || "");
  });
}
