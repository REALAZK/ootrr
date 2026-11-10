use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::path::Path as P;

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::json::{self, Json};
use crate::placement;
use crate::romfs::Romfs;

pub const ACTOR_INFO: &str = "RSDB/ActorInfo.Product.111.rstbl.byml.zs";
pub const POUCH_INFO: &str = "RSDB/PouchActorInfo.Product.111.rstbl.byml.zs";

#[derive(Debug, Clone, Default)]
pub struct Dump {
    pub actors: BS<String>,
    pub packs: BS<String>,
    pub pouch: BS<String>,
    pub bundle: BM<String, i64>,
    pub chest: BS<String>,
    pub dropped: BS<String>,
    pub placed: BS<String>,
    pub flow: BS<String>,
}

pub fn row_ids(y: &Byml) -> BS<String> {
    let mut out: BS<String> = BS::new();
    for r in y.root.as_arr().unwrap_or(&[]) {
        if let Some(s) = r.get("__RowId").and_then(|v| v.as_str()) {
            out.insert(s.to_string());
        }
    }
    out
}

pub fn bundle_counts(y: &Byml) -> BM<String, i64> {
    let mut out: BM<String, i64> = BM::new();
    for r in y.root.as_arr().unwrap_or(&[]) {
        let Some(id) = r.get("__RowId").and_then(|v| v.as_str()) else {
            continue;
        };
        let n: Option<i64> = match r.get("BundleActorNum") {
            Some(Node::I32(x)) => Some(*x as i64),
            Some(Node::U32(x)) => Some(*x as i64),
            Some(Node::I64(x)) => Some(*x),
            _ => None,
        };
        if let Some(n) = n {
            out.insert(id.to_string(), n);
        }
    }
    out
}

pub fn pack_names(files: &[String]) -> BS<String> {
    let mut out: BS<String> = BS::new();
    for f in files {
        if let Some(n) = f.strip_prefix("Pack/Actor/").and_then(|s| s.strip_suffix(".pack.zs")) {
            out.insert(n.to_string());
        }
    }
    out
}

pub fn banc(y: &Byml, d: &mut Dump) {
    for a in y.root.get("Actors").and_then(|v| v.as_arr()).unwrap_or(&[]) {
        let g: &str = a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("");
        d.placed.insert(g.to_string());
        let Some(v) = a.get("Dynamic").and_then(|x| x.get("Drop__DropActor")).and_then(|x| x.as_str()) else {
            continue;
        };
        if g.contains("TBox") {
            d.chest.insert(v.to_string());
        } else {
            d.dropped.insert(v.to_string());
        }
    }
}

pub fn flow_items(t: &str) -> Res<BS<String>> {
    let j: Json = json::parse(t)?;
    let mut out: BS<String> = BS::new();
    for f in j.get("flows").and_then(|v| v.as_arr()).unwrap_or(&[]) {
        for e in f.get("events").and_then(|v| v.as_arr()).unwrap_or(&[]) {
            let role: &str = e.get("role").and_then(|v| v.as_str()).unwrap_or("");
            if role != "give" && role != "sell" {
                continue;
            }
            if let Some(i) = e.get("item").and_then(|v| v.as_str()) {
                out.insert(i.to_string());
            }
        }
    }
    Ok(out)
}

pub fn scan(root: &P) -> Res<Dump> {
    let mut rf: Romfs = Romfs::open(root)?;
    let mut d: Dump = Dump::default();
    d.actors = row_ids(&rf.read_byml(ACTOR_INFO)?);
    let pouch: Byml = rf.read_byml(POUCH_INFO)?;
    d.pouch = row_ids(&pouch);
    d.bundle = bundle_counts(&pouch);
    d.packs = pack_names(&rf.list("Pack/Actor")?);
    for f in placement::banc_files(&rf)? {
        let y: Byml = rf.read_byml(&f)?;
        banc(&y, &mut d);
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(kv: Vec<(&str, Node)>) -> Node {
        Node::Map(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn file(root: Node) -> Byml {
        Byml { root, version: 7, big: false }
    }

    #[test]
    fn row_ids_and_bundles_read_the_tables() {
        let y: Byml = file(Node::Arr(vec![
            map(vec![("BundleActorNum", Node::I32(5)), ("__RowId", Node::Str("A".to_string()))]),
            map(vec![("__RowId", Node::Str("B".to_string()))]),
            map(vec![("BundleActorNum", Node::U32(3))]),
        ]));
        assert_eq!(row_ids(&y).into_iter().collect::<Vec<String>>(), vec!["A", "B"]);
        let b = bundle_counts(&y);
        assert_eq!(b.get("A"), Some(&5));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn pack_names_keep_only_actor_packs() {
        let f: Vec<String> = vec![
            "Pack/Actor/Item_A.pack.zs".to_string(),
            "Pack/Actor/Item_B.txt".to_string(),
            "Pack/Other/Item_C.pack.zs".to_string(),
        ];
        assert_eq!(pack_names(&f).into_iter().collect::<Vec<String>>(), vec!["Item_A"]);
    }

    #[test]
    fn banc_separates_chest_drops_from_other_drops() {
        let chest: Node = map(vec![
            ("Dynamic", map(vec![("Drop__DropActor", Node::Str("Item_Hammer".to_string()))])),
            ("Gyaml", Node::Str("Obj_TBox_A_02".to_string())),
        ]);
        let pot: Node = map(vec![
            ("Dynamic", map(vec![("Drop__DropActor", Node::Str("Obj_Rupee_Blue".to_string()))])),
            ("Gyaml", Node::Str("Obj_Pot_A_01".to_string())),
        ]);
        let free: Node = map(vec![("Gyaml", Node::Str("Obj_Heart".to_string()))]);
        let mut d: Dump = Dump::default();
        banc(&file(map(vec![("Actors", Node::Arr(vec![chest, pot, free]))])), &mut d);
        assert!(d.chest.contains("Item_Hammer") && !d.chest.contains("Obj_Rupee_Blue"));
        assert!(d.dropped.contains("Obj_Rupee_Blue"));
        assert!(d.placed.contains("Obj_Heart") && d.placed.contains("Obj_TBox_A_02"));
    }

    #[test]
    fn flows_count_only_gives_and_sales() {
        let t: &str = "{\"flows\":[{\"events\":[\
            {\"role\":\"give\",\"item\":\"A\"},{\"role\":\"sell\",\"item\":\"B\"},\
            {\"role\":\"take\",\"item\":\"C\"},{\"role\":\"flag\",\"item\":\"D\"},{\"role\":\"give\"}]}]}";
        assert_eq!(flow_items(t).unwrap().into_iter().collect::<Vec<String>>(), vec!["A", "B"]);
        assert!(flow_items("{}").unwrap().is_empty());
    }
}
