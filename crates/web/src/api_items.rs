use std::path::PathBuf as PB;

use ootrr_core::json::Json;
use ootrr_core::tracker::grid;
use ootrr_core::tracker::icon_map;

use crate::api_tracker;
use crate::api_validate;
use crate::error::{Fail, Hr};
use crate::icon_store;
use crate::req::Req;
use crate::resp::Resp;
use crate::state::Shared;

pub const PNG: &str = "image/png";
const MAX_ID: usize = 64;

pub fn items(sh: &Shared) -> Json {
    api_tracker::ensure(sh);
    let g = sh.tracker.lock();
    let live: bool = g.src.reads > 0 && g.items.is_some();
    let state: Json = match (&g.items, live) {
        (Some(st), true) => grid::state_json(st),
        _ => Json::Null,
    };
    let key: String = format!("{}:{}", g.index.game, icon_map::VERSION);
    Json::obj()
        .with("format", Json::Int(1))
        .with("layout", grid::layout_json())
        .with("live", live.into())
        .with("slot", g.src.slot.as_deref().filter(|_| live).map(Json::from).unwrap_or(Json::Null))
        .with("state", state)
        .with("art", Json::obj().with("ready", g.game_dir.is_some().into()).with("cache", sh.tracker.icons.mode().into()).with("key", key.as_str().into()))
}

pub fn icon(sh: &Shared, r: &Req) -> Hr<Resp> {
    api_tracker::ensure(sh);
    let id: &str = r.param("id").unwrap_or("");
    if id.is_empty() || id.len() > MAX_ID || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Fail::bad("id must be an icon name"));
    }
    if icon_map::find(id).is_none() {
        return Err(Fail::new(404, "no such icon"));
    }
    let (game, fp): (PB, String) = {
        let g = sh.tracker.lock();
        (g.game_dir.clone().ok_or_else(|| Fail::new(404, "no game folder is set"))?, g.index.game.clone())
    };
    let root: PB = api_validate::inspect(&game).romfs.unwrap_or(game);
    let cache: Option<PB> = sh.tracker.data.as_ref().filter(|_| !fp.is_empty()).map(|d| icon_store::cache_dir(d, &fp));
    match sh.tracker.icons.get(&root, cache.as_deref(), id) {
        Ok(b) => Ok(Resp::new(200, PNG, b.to_vec())),
        Err(e) => Err(Fail::new(404, &e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::Guard;
    use crate::icon_store::tests::fake_game;
    use crate::tracker::Tracker;
    use crate::watch::tests::save_bytes;
    use ootrr_core::murmur::murmur3_32;
    use ootrr_core::tex::png;
    use ootrr_core::tracker::index::Index;
    use ootrr_core::tracker::peek::{Lim, Peek, peek_with};
    use std::sync::Arc;

    fn sh() -> Shared {
        Shared::with_tracker(Guard { port: 5000, token: "tok".into(), lan: false }, false, Arc::new(Tracker::new(None)))
    }

    fn rq(id: &str) -> Req {
        Req { method: "GET".into(), path: "/api/tracker/icon".into(), query: vec![("id".into(), id.into())], heads: Vec::new(), body: Vec::new() }
    }

    fn h(n: &str) -> u32 {
        murmur3_32(n.as_bytes(), 0)
    }

    #[test]
    fn without_a_save_the_layout_is_sent_and_nothing_is_live() {
        let s: Shared = sh();
        let j: Json = items(&s);
        assert_eq!(j.get("live").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(j.get("state"), Some(&Json::Null));
        assert_eq!(j.get("layout").and_then(|l| l.get("cells")).and_then(|x| x.as_arr()).map(|a| a.len()), Some(grid::CELLS.len()));
        let art: &Json = j.get("art").unwrap();
        assert_eq!(art.get("ready").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(icon(&s, &rq("Item_Bomb")).unwrap_err().status, 404);
    }

    #[test]
    fn a_published_save_fills_the_state() {
        let s: Shared = sh();
        let b: Vec<u8> = save_bytes(&[(h("IsGet.Item_Hookshot"), 1), (h("PouchData.Item_DekuStick.StockNum"), 10), (h("PlayerStatus.MaxLife"), 12)], &[]);
        let p: Peek = peek_with(&b, &Lim::ANY).unwrap();
        s.tracker.publish("slot_00", &p, None, vec!["slot_00".into()]);
        let t: String = items(&s).to_string();
        assert!(t.contains("\"live\":true") && t.contains("\"slot\":\"slot_00\""));
        assert!(t.contains("\"hookshot\":{\"lv\":1,\"n\":null,\"part\":null}"));
        assert!(t.contains("\"sticks\":{\"lv\":1,\"n\":10,\"part\":null}"));
        assert!(t.contains("\"hearts\":{\"lv\":1,\"n\":3,\"part\":0}"));
        assert!(!t.contains("\"longshot\""));
    }

    #[test]
    fn icons_come_from_the_game_folder_and_bad_ids_are_refused() {
        let d: PB = fake_game("api", &["Item_Bomb"]);
        let s: Shared = sh();
        s.tracker.set_index(d.clone(), Index { game: "oot-remake:test".into(), table: 0, maps: Vec::new(), locs: Vec::new(), shapes: Vec::new() });
        let r: Resp = icon(&s, &rq("Item_Bomb")).unwrap();
        assert_eq!((r.status, r.ctype), (200, PNG));
        assert!(png::looks_ok(&r.body));
        assert_eq!(icon(&s, &rq("Item_Hookshot")).unwrap_err().status, 404);
        assert_eq!(icon(&s, &rq("Item_Nope")).unwrap_err().status, 404);
        for bad in ["", "../x", "a/b", "Item Bomb", "Item_Bomb.png"] {
            assert_eq!(icon(&s, &rq(bad)).unwrap_err().status, 400, "{bad}");
        }
        assert!(items(&s).to_string().contains("\"ready\":true"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
