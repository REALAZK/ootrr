const HTML: &str = "text/html; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

#[cfg(test)]
pub const JS_FILES: [&str; 16] = [
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
    "/js/result.js",
    "/js/theme.js",
    "/js/update.js",
];

pub fn get(p: &str) -> Option<(&'static str, &'static str)> {
    let r: (&'static str, &'static str) = match p {
        "/" | "/index.html" => (HTML, include_str!("../assets/index.html")),
        "/style.css" => (CSS, include_str!("../assets/style.css")),
        "/themes.css" => (CSS, include_str!("../assets/themes.css")),
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
        "/js/result.js" => (JS, include_str!("../assets/js/result.js")),
        "/js/theme.js" => (JS, include_str!("../assets/js/theme.js")),
        "/js/update.js" => (JS, include_str!("../assets/js/update.js")),
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
        let all: [&str; 4] = [get("/").unwrap().1, get("/style.css").unwrap().1, get("/themes.css").unwrap().1, get("/app.js").unwrap().1];
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
    fn no_asset_uses_inline_html_injection() {
        for p in JS_FILES {
            let t: &str = get(p).unwrap().1;
            assert!(!t.contains("innerHTML") && !t.contains("outerHTML") && !t.contains("insertAdjacentHTML"), "{p}");
            assert!(!t.contains("eval(") && !t.contains("new Function") && !t.contains("document.write"), "{p}");
        }
    }
}
