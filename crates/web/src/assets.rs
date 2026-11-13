const HTML: &str = "text/html; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const FONT: &str = "font/woff2";

#[cfg(test)]
pub const JS_FILES: [&str; 35] = [
    "/app.js",
    "/js/dom.js",
    "/js/api.js",
    "/js/store.js",
    "/js/rules.js",
    "/js/nav.js",
    "/js/picker.js",
    "/js/setup.js",
    "/js/controls.js",
    "/js/settings.js",
    "/js/strings.js",
    "/js/generate.js",
    "/js/viewer.js",
    "/js/spoiler.js",
    "/js/result.js",
    "/js/theme.js",
    "/js/update.js",
    "/js/report.js",
    "/js/plando.js",
    "/js/sfx.js",
    "/js/tracker_api.js",
    "/js/tracker_store.js",
    "/js/tracker_news.js",
    "/js/tracker_list.js",
    "/js/tracker_filters.js",
    "/js/tracker_source.js",
    "/js/tracker_kinds.js",
    "/js/tracker_places.js",
    "/js/tracker_art.js",
    "/js/tracker_map.js",
    "/js/tracker_card.js",
    "/js/tracker_rail.js",
    "/js/tracker_fold.js",
    "/js/tracker_items.js",
    "/js/tracker_app.js",
];

pub fn get_bin(p: &str) -> Option<(&'static str, &'static [u8])> {
    let r: (&'static str, &'static [u8]) = match p {
        "/fonts/cinzel.woff2" => (FONT, include_bytes!("../assets/fonts/cinzel-latin-wght-normal.woff2")),
        "/fonts/garamond.woff2" => (FONT, include_bytes!("../assets/fonts/eb-garamond-latin-wght-normal.woff2")),
        "/fonts/garamond-italic.woff2" => (FONT, include_bytes!("../assets/fonts/eb-garamond-latin-wght-italic.woff2")),
        _ => return None,
    };
    Some(r)
}

pub fn get(p: &str) -> Option<(&'static str, &'static str)> {
    let r: (&'static str, &'static str) = match p {
        "/" | "/index.html" => (HTML, include_str!("../assets/index.html")),
        "/style.css" => (CSS, include_str!("../assets/style.css")),
        "/themes.css" => (CSS, include_str!("../assets/themes.css")),
        "/hyrule.css" => (CSS, include_str!("../assets/hyrule.css")),
        "/tracker" => (HTML, include_str!("../assets/tracker.html")),
        "/tracker.css" => (CSS, include_str!("../assets/tracker.css")),
        "/tracker_items.css" => (CSS, include_str!("../assets/tracker_items.css")),
        "/app.js" => (JS, include_str!("../assets/app.js")),
        "/js/dom.js" => (JS, include_str!("../assets/js/dom.js")),
        "/js/api.js" => (JS, include_str!("../assets/js/api.js")),
        "/js/store.js" => (JS, include_str!("../assets/js/store.js")),
        "/js/rules.js" => (JS, include_str!("../assets/js/rules.js")),
        "/js/nav.js" => (JS, include_str!("../assets/js/nav.js")),
        "/js/picker.js" => (JS, include_str!("../assets/js/picker.js")),
        "/js/setup.js" => (JS, include_str!("../assets/js/setup.js")),
        "/js/controls.js" => (JS, include_str!("../assets/js/controls.js")),
        "/js/settings.js" => (JS, include_str!("../assets/js/settings.js")),
        "/js/strings.js" => (JS, include_str!("../assets/js/strings.js")),
        "/js/generate.js" => (JS, include_str!("../assets/js/generate.js")),
        "/js/viewer.js" => (JS, include_str!("../assets/js/viewer.js")),
        "/js/spoiler.js" => (JS, include_str!("../assets/js/spoiler.js")),
        "/js/result.js" => (JS, include_str!("../assets/js/result.js")),
        "/js/theme.js" => (JS, include_str!("../assets/js/theme.js")),
        "/js/update.js" => (JS, include_str!("../assets/js/update.js")),
        "/js/report.js" => (JS, include_str!("../assets/js/report.js")),
        "/js/plando.js" => (JS, include_str!("../assets/js/plando.js")),
        "/js/sfx.js" => (JS, include_str!("../assets/js/sfx.js")),
        "/js/tracker_api.js" => (JS, include_str!("../assets/js/tracker_api.js")),
        "/js/tracker_store.js" => (JS, include_str!("../assets/js/tracker_store.js")),
        "/js/tracker_news.js" => (JS, include_str!("../assets/js/tracker_news.js")),
        "/js/tracker_list.js" => (JS, include_str!("../assets/js/tracker_list.js")),
        "/js/tracker_filters.js" => (JS, include_str!("../assets/js/tracker_filters.js")),
        "/js/tracker_source.js" => (JS, include_str!("../assets/js/tracker_source.js")),
        "/js/tracker_kinds.js" => (JS, include_str!("../assets/js/tracker_kinds.js")),
        "/js/tracker_places.js" => (JS, include_str!("../assets/js/tracker_places.js")),
        "/js/tracker_art.js" => (JS, include_str!("../assets/js/tracker_art.js")),
        "/js/tracker_map.js" => (JS, include_str!("../assets/js/tracker_map.js")),
        "/js/tracker_card.js" => (JS, include_str!("../assets/js/tracker_card.js")),
        "/js/tracker_rail.js" => (JS, include_str!("../assets/js/tracker_rail.js")),
        "/js/tracker_fold.js" => (JS, include_str!("../assets/js/tracker_fold.js")),
        "/js/tracker_items.js" => (JS, include_str!("../assets/js/tracker_items.js")),
        "/js/tracker_app.js" => (JS, include_str!("../assets/js/tracker_app.js")),
        _ => return None,
    };
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_assets_resolve_and_unknown_do_not() {
        for p in ["/", "/index.html", "/style.css", "/app.js", "/js/dom.js"] {
            let (t, b): (&str, &str) = get(p).unwrap();
            assert!(!t.is_empty() && !b.is_empty(), "{p}");
        }
        for p in ["/js/../Cargo.toml", "/etc/passwd", "/js/", "/app.js/", "/APP.JS", "/api/schema", ""] {
            assert!(get(p).is_none(), "{p}");
        }
    }

    #[test]
    fn the_page_loads_nothing_from_outside() {
        let all: [&str; 5] = [get("/").unwrap().1, get("/style.css").unwrap().1, get("/themes.css").unwrap().1, get("/hyrule.css").unwrap().1, get("/app.js").unwrap().1];
        for t in all {
            assert!(!t.contains("http://") && !t.contains("https://"), "external url");
            assert!(!t.contains("@import"));
        }
        let h: &str = get("/").unwrap().1;
        assert!(!h.contains("style=") && !h.contains("onclick") && !h.contains("<style"));
        assert_eq!(h.matches("<script").count(), 1);
        assert!(h.contains("<script type=\"module\" src=\"/app.js\">"));
    }

    #[test]
    fn the_tracker_page_is_self_contained() {
        let h: &str = get("/tracker").unwrap().1;
        assert!(!h.contains("style=") && !h.contains("onclick") && !h.contains("<style"));
        assert_eq!(h.matches("<script").count(), 1);
        assert!(h.contains("<script type=\"module\" src=\"/js/tracker_app.js\">"));
        let mut n: usize = 0;
        for p in JS_FILES.iter().filter(|p| p.starts_with("/js/tracker_")) {
            let t: String = get(p).unwrap().1.replace("http://www.w3.org/2000/svg", "");
            assert!(!t.contains("http://") && !t.contains("https://"), "{p}");
            assert!(!t.contains("\"style\"") && !t.contains("sfx.js") && !t.contains("//"), "{p}");
            for part in t.split(" from \"./").skip(1) {
                let name: &str = part.split('"').next().unwrap();
                assert!(get(&format!("/js/{name}")).is_some(), "{p} imports {name}");
            }
            n += 1;
        }
        assert_eq!(n, 15);
        let items: &str = get("/tracker_items.css").unwrap().1;
        assert!(!items.contains("@import") && !items.contains("url(") && !items.contains("http"));
        assert!(h.contains("<link rel=\"stylesheet\" href=\"/tracker_items.css\">"));
        let css: &str = get("/tracker.css").unwrap().1;
        assert!(!css.contains("@import") && !css.contains("http"));
        let mut fonts: usize = 0;
        for part in css.split("url(").skip(1) {
            let p: &str = part.split(')').next().unwrap().trim_matches('"');
            assert!(get_bin(p).is_some(), "{p}");
            fonts += 1;
        }
        assert_eq!(fonts, 3);
    }

    #[test]
    fn fonts_are_woff2_files_and_unknown_fonts_do_not_resolve() {
        for p in ["/fonts/cinzel.woff2", "/fonts/garamond.woff2", "/fonts/garamond-italic.woff2"] {
            let (t, b): (&str, &[u8]) = get_bin(p).unwrap();
            assert_eq!(t, "font/woff2", "{p}");
            assert_eq!(&b[..4], b"wOF2", "{p}");
            assert!(b.len() > 10_000, "{p}");
        }
        for p in ["/fonts/", "/fonts/../app.js", "/fonts/cinzel.ttf", "/fonts/CINZEL.woff2", "/fonts/cinzel.woff2/", ""] {
            assert!(get_bin(p).is_none(), "{p}");
        }
        assert!(get("/fonts/cinzel.woff2").is_none());
    }

    #[test]
    fn the_stylesheet_names_only_served_fonts() {
        let css: &str = get("/hyrule.css").unwrap().1;
        let mut n: usize = 0;
        for part in css.split("url(\"").skip(1) {
            let p: &str = part.split('"').next().unwrap();
            assert!(get_bin(p).is_some(), "{p}");
            n += 1;
        }
        assert_eq!(n, 3);
    }

    #[test]
    fn every_script_parses_as_a_module_when_node_is_installed() {
        let probe = std::process::Command::new("node").arg("--version").output();
        if probe.map(|o| !o.status.success()).unwrap_or(true) {
            return;
        }
        let dir = std::env::temp_dir().join(format!("ootrr_js_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut all: Vec<&str> = JS_FILES.to_vec();
        all.push("/app.js");
        for (i, p) in all.iter().enumerate() {
            let f = dir.join(format!("m{i}.mjs"));
            std::fs::write(&f, get(p).unwrap().1).unwrap();
            let o = std::process::Command::new("node").arg("--check").arg(&f).output().unwrap();
            assert!(o.status.success(), "{p}: {}", String::from_utf8_lossy(&o.stderr));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_fold_rules_hold_when_node_is_installed() {
        let probe = std::process::Command::new("node").arg("--version").output();
        if probe.map(|o| !o.status.success()).unwrap_or(true) {
            return;
        }
        let dir = std::env::temp_dir().join(format!("ootrr_fold_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fold.mjs"), get("/js/tracker_fold.js").unwrap().1).unwrap();
        let script: &str = r#"
import assert from "node:assert";
import { SIDE_KEY, fold_default, fold_parse, fold_text, clean_shut, clean_marks, next_level, owned } from "./fold.mjs";
assert.equal(SIDE_KEY, "ootrr.tracker.side");
assert.equal(fold_default(1100), true);
assert.equal(fold_default(761), true);
assert.equal(fold_default(760), false);
assert.equal(fold_default(375), false);
assert.equal(fold_default(undefined), true);
assert.equal(fold_parse("open", 375), true);
assert.equal(fold_parse("closed", 1500), false);
assert.equal(fold_parse(null, 375), false);
assert.equal(fold_parse("junk", 1500), true);
assert.equal(fold_parse(fold_text(false), 1500), false);
assert.equal(fold_parse(fold_text(true), 375), true);
assert.deepEqual(clean_shut({ tools: true, gear: 1, nope: true }, ["tools", "gear"]), { tools: true });
assert.deepEqual(clean_shut("x", ["tools"]), {});
assert.deepEqual(clean_marks({ bow: 2, lens: 0, bad: "3", big: 12 }, 10), { bow: 2 });
assert.deepEqual(clean_marks(null, 10), {});
assert.equal(next_level(undefined, 1), 1);
assert.equal(next_level(1, 1), 0);
assert.equal(next_level(1, 2), 2);
assert.equal(next_level(2, 2), 0);
assert.equal(owned([{ id: "a" }, { id: "b" }, { id: "c" }], (c) => ({ lv: c.id === "b" ? 0 : 1 })), 2);
"#;
        std::fs::write(dir.join("run.mjs"), script).unwrap();
        let o = std::process::Command::new("node").arg(dir.join("run.mjs")).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_asset_uses_inline_html_injection() {
        for p in JS_FILES {
            let t: &str = get(p).unwrap().1;
            assert!(!t.contains("innerHTML") && !t.contains("outerHTML") && !t.contains("insertAdjacentHTML"), "{p}");
            assert!(!t.contains("eval(") && !t.contains("new Function") && !t.contains("document.write"), "{p}");
        }
    }
}
