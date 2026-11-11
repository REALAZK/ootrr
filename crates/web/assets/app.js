import { $, h, fill, notice } from "/js/dom.js";
import { init_token, get } from "/js/api.js";
import { state, restore } from "/js/store.js";
import { register, start, show, here } from "/js/nav.js";
import { render as setup, probe } from "/js/setup.js";
import { render as settings } from "/js/settings.js";
import { render as generate } from "/js/generate.js";
import { render as result } from "/js/result.js";
import { start as start_theme } from "/js/theme.js";
import { start as start_update } from "/js/update.js";

function fail(msg) {
  fill($("#main"), h("h1", null, "OOTRR"), notice("error", msg));
}

async function boot() {
  start_theme();
  if (!init_token()) {
    return fail("This page has no session token. Open the link that ootrr serve printed in your terminal.");
  }
  try {
    const [info, schema] = await Promise.all([get("/api/info"), get("/api/schema")]);
    state.info = info;
    restore(schema);
  } catch (e) {
    return fail(e.message);
  }
  register("setup", setup);
  register("settings", settings);
  register("generate", generate);
  register("result", result);
  start();
  start_update();
  if (here() !== "setup") {
    await probe();
    show();
  }
}

boot();
