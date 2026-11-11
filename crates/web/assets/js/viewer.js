import { h, fill, notice } from "./dom.js";
import { state } from "./store.js";
import { defaults, effective, same, value_label } from "./rules.js";

const PAGE = 300;

function table(head, rows) {
  return h(
    "div",
    { class: "table-wrap" },
    h("table", null, h("thead", null, h("tr", null, head.map((x) => h("th", { scope: "col" }, x)))), h("tbody", null, rows.map((r) => h("tr", null, r.map((c) => h("td", null, c)))))),
  );
}

function placements(res) {
  const all = res.placements || [];
  if (all.length === 0) {
    return notice("info", res.notice || "This seed changes no items. Turn on a shuffle setting if you want items to move.");
  }
  const out = h("div");
  const info = h("p", { class: "muted small" });
  let shown = PAGE;
  let qy = "";
  const search = h("input", { type: "search", "aria-label": "Search placements", placeholder: "Search locations and items", oninput: () => {
    qy = search.value.trim().toLowerCase();
    shown = PAGE;
    draw();
  } });
  const more = h("button", { type: "button", class: "btn", onclick: () => {
    shown += PAGE;
    draw();
  } }, "Show more");
  function draw() {
    const hits = qy ? all.filter(([a, b]) => `${a} ${b}`.toLowerCase().includes(qy)) : all;
    info.textContent = `${hits.length} of ${all.length} placements`;
    more.classList.toggle("hide", hits.length <= shown);
    fill(out, table(["Location", "Item"], hits.slice(0, shown)));
  }
  draw();
  return h("div", { class: "stack" }, search, info, out, more);
}

function playthrough(res) {
  const sp = res.spoiler;
  if (!sp || !sp.available) return notice("info", (sp && sp.reason) || "The playthrough is not available for this seed file.");
  const spheres = sp.playthrough || [];
  return h(
    "div",
    { class: "stack" },
    spheres.map((s, i) => h("section", { class: "card" }, h("h3", null, `Step ${i + 1}`), table(["Location", "Item"], s))),
  );
}

function used(res) {
  const schema = state.schema;
  const base = effective(schema, defaults(schema));
  const now = res.settings || {};
  const rows = schema.settings.filter((s) => s.id in now && !same(now[s.id], base[s.id])).map((s) => [s.label, value_label(s, now[s.id])]);
  if (rows.length === 0) return h("p", { class: "muted" }, "Every setting is at its default.");
  return table(["Setting", "Value"], rows);
}

export function viewer(res) {
  const views = [
    ["Placements", () => placements(res)],
    ["Playthrough", () => playthrough(res)],
    ["Settings used", () => used(res)],
  ];
  const body = h("div");
  const tabs = h("div", { class: "tabs", role: "tablist" });
  let at = 0;
  function draw() {
    fill(
      tabs,
      views.map(([name], i) =>
        h("button", { type: "button", class: "tab", role: "tab", "aria-selected": i === at ? "true" : "false", onclick: () => {
          at = i;
          draw();
        } }, name),
      ),
    );
    fill(body, views[at][1]());
  }
  draw();
  return h("div", null, tabs, body);
}
