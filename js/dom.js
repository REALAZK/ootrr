const PROPS = new Set(["value", "checked", "disabled", "selected", "indeterminate", "hidden"]);

function add(el, kids) {
  for (const k of kids.flat(Infinity)) {
    if (k === null || k === undefined || k === false) continue;
    el.append(k instanceof Node ? k : document.createTextNode(String(k)));
  }
}

export function h(tag, attrs, ...kids) {
  const el = document.createElement(tag);
  if (attrs) {
    for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined) continue;
      if (k === "class") el.className = v;
      else if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v);
      else if (PROPS.has(k)) el[k] = v;
      else if (v === false) continue;
      else el.setAttribute(k, v === true ? "" : String(v));
    }
  }
  add(el, kids);
  return el;
}

export function fill(el, ...kids) {
  el.replaceChildren();
  add(el, kids);
  return el;
}

export function $(sel, root = document) {
  return root.querySelector(sel);
}

export function notice(kind, ...kids) {
  return h("div", { class: `notice ${kind}`, role: kind === "error" ? "alert" : null }, kids);
}

let toast_timer = 0;

export function toast(msg) {
  const box = $("#toast");
  if (!box) return;
  box.replaceChildren(h("div", { class: "t" }, msg));
  clearTimeout(toast_timer);
  toast_timer = setTimeout(() => box.replaceChildren(), 2600);
}

export async function copy(text) {
  try {
    await navigator.clipboard.writeText(text);
    toast("Copied");
    return true;
  } catch {
    const t = h("textarea", { value: text, readonly: true });
    document.body.append(t);
    t.select();
    let ok = false;
    try {
      ok = document.execCommand("copy");
    } catch {
      ok = false;
    }
    t.remove();
    toast(ok ? "Copied" : "Copy failed, select the text and copy it by hand");
    return ok;
  }
}

export function save_text(name, text, type = "application/json") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = h("a", { href: url, download: name });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 4000);
}

export function debounce(fn, ms) {
  let t = 0;
  return (...a) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...a), ms);
  };
}
