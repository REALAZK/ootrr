import { h } from "./dom.js";

export const THEMES = [
  { id: "hyrule", label: "Hyrule" },
  { id: "system", label: "System" },
  { id: "win95", label: "Windows 95" },
  { id: "winxp", label: "Windows XP" },
  { id: "vista", label: "Windows Vista" },
  { id: "pinkblack", label: "Pink and black" },
  { id: "terminal", label: "Terminal" },
  { id: "nord", label: "Nord" },
  { id: "sunset", label: "Sunset" },
  { id: "paper", label: "Paper" },
  { id: "contrast", label: "High contrast" },
];

const KEY = "ootrr.theme";
const DEFAULT = "hyrule";

function known(id) {
  return THEMES.some((t) => t.id === id);
}

function saved() {
  try {
    const v = localStorage.getItem(KEY);
    return v && known(v) ? v : DEFAULT;
  } catch {
    return DEFAULT;
  }
}

export function apply(id) {
  const root = document.documentElement;
  if (id === "system" || !known(id)) root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", id);
}

function remember(id) {
  try {
    localStorage.setItem(KEY, id);
  } catch {
    return;
  }
}

export function start() {
  const now = saved();
  apply(now);
  const box = document.querySelector(".top-in");
  if (!box) return;
  const sel = h(
    "select",
    {
      id: "theme-pick",
      "aria-label": "Theme",
      onchange: () => {
        apply(sel.value);
        remember(sel.value);
      },
    },
    THEMES.map((t) => h("option", { value: t.id, selected: t.id === now }, t.label)),
  );
  box.append(h("div", { class: "themepick" }, sel));
}
