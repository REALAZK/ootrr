import { $, h, fill, notice } from "./js/dom.js";
import { init_token, get } from "./js/api.js";
import { state, restore } from "./js/store.js";
import { start } from "./js/nav.js";
import { start_menu } from "./js/menu.js";
import { start as start_theme, mount as mount_theme } from "./js/theme.js";
import { start as start_tips } from "./js/tooltip.js";
import { start as start_sfx } from "./js/sfx.js";
import { start as start_update } from "./js/update.js";

function fail(msg) {
  fill($("#main"), h("h1", null, "OOTRR"), notice("error", msg));
}

async function boot() {
  start_theme();
  start_tips();
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
  try {
    start_menu();
    mount_theme();
    start_sfx();
    start();
  } catch (e) {
    return fail(`The page could not start. ${e.message}`);
  }
  start_update();
}

boot();
