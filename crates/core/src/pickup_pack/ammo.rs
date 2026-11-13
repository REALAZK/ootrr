use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::pickup_pack::rows::{find, need, POUCH_INFO};
use crate::pregrant::pregrant;

pub const GATE: &str = "ExpansionMaxNumActor";
pub const MAX: &str = "AfterExpansionMaxNum";
pub const KIND: &str = "MaxNumGameData";
pub const COLS: [&str; 5] = ["ActorFullOnlyIfIsGetWhenGetBag", MAX, GATE, "IsItemStockMaxIfGet", KIND];

pub fn bag(identity: &str) -> Res<&'static str> {
    match pregrant(identity).first() {
        Some(b) if *b != identity => Ok(b),
        Some(_) => Err(E::Msg(format!("{identity} is its own starting bag, it needs no ammo layer"))),
        None => Err(E::Msg(format!("a chest gives no starting bag with {identity}, so there is nothing for the ammo layer to add"))),
    }
}

fn is_bag(r: &Node) -> bool {
    r.get(GATE).and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty()) && r.get(MAX).is_some() && r.get(KIND).and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty())
}

pub fn grown(was: &Node, now: &Node, from: &Node) -> bool {
    let mut want: Node = was.clone();
    for k in COLS {
        match from.get(k) {
            Some(v) => {
                want.set(k, v.clone());
            }
            None => {
                want.remove(k);
            }
        }
    }
    &want == now
}

fn fill(y: &mut Byml, who: &str, from: &Node) -> Res<()> {
    let rows: &mut Vec<Node> = y.root.as_arr_mut().ok_or(E::Format("actor table: the root is not a row list"))?;
    let r: &mut Node = rows
        .iter_mut()
        .find(|r| r.get("__RowId").and_then(|v| v.as_str()) == Some(who))
        .ok_or_else(|| E::Msg(format!("{who} has no row in {POUCH_INFO}")))?;
    for k in COLS {
        match (from.get(k), r.get(k)) {
            (Some(v), Some(old)) if old != v => return Err(E::Msg(format!("{who} already has another {k}, the ammo layer does not fit it"))),
            (None, Some(_)) => return Err(E::Msg(format!("{who} already has a {k}, the ammo layer does not fit it"))),
            (Some(v), _) => {
                r.set(k, v.clone());
            }
            (None, None) => {}
        }
    }
    Ok(())
}

pub fn apply(y: &mut Byml, name: &str, identity: &str) -> Res<&'static str> {
    let b: &'static str = bag(identity)?;
    let from: Node = need(y, b, POUCH_INFO)?.clone();
    if !is_bag(&from) {
        return Err(E::Msg(format!("{b} has no capacity columns in {POUCH_INFO}")));
    }
    fill(y, identity, &from)?;
    fill(y, name, &from)?;
    Ok(b)
}

pub fn audit(was: &Byml, now: &Byml, name: &str, identity: &str) -> Res<String> {
    let b: &'static str = bag(identity)?;
    let from: &Node = need(now, b, POUCH_INFO)?;
    if find(was, b) != Some(from) || !is_bag(from) {
        return Err(E::Msg(format!("audit: the bag row {b} changed")));
    }
    for who in [name, identity] {
        let r: &Node = need(now, who, POUCH_INFO)?;
        if COLS.iter().any(|k| r.get(k) != from.get(k)) {
            return Err(E::Msg(format!("audit: {who} does not carry the capacity columns of {b}")));
        }
    }
    let old: &Node = need(was, identity, POUCH_INFO)?;
    if !grown(old, need(now, identity, POUCH_INFO)?, from) {
        return Err(E::Msg(format!("audit: the row of {identity} changed in more than the capacity columns")));
    }
    Ok(format!(
        "ammo layer: one shipped row changed, {name} and {identity} carry the capacity columns of {b} (kind {}, maximum {:?}), no other column of {identity} changed",
        from.get(KIND).and_then(|v| v.as_str()).unwrap_or(""),
        from.get(MAX)
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf as PB;

    use super::*;
    use crate::pickup_pack::rows::tests::{row, table};
    use crate::pickup_pack::rows::{insert, pouch_row};
    use crate::romfs::Romfs;

    fn s(x: &str) -> Node {
        Node::Str(x.to_string())
    }

    fn sample() -> Byml {
        let mut y: Byml = table(vec![
            row("Item_Quiver_Small", &[(MAX, Node::I32(30)), (GATE, s("Work/Actor/Item_Arrow_Normal.engine__actor__ActorParam.gyml")), ("IsItemStockMaxIfGet", Node::Bool(true)), (KIND, s("Arrow")), ("PouchCategory", s("KeyItem"))]),
            row("Item_Range_Bow", &[("PouchCategory", s("Bow"))]),
            row("Weapon_Range_Bow", &[("AssignableItem", s("Bow")), ("PouchCategory", s("Bow"))]),
        ]);
        let new: Node = pouch_row(find(&y, "Item_Range_Bow").unwrap(), "Obj_Range_Bow");
        insert(&mut y, new).unwrap();
        y
    }

    #[test]
    fn the_bag_comes_from_the_chest_table() {
        assert_eq!(bag("Weapon_Range_Bow").unwrap(), "Item_Quiver_Small");
        assert_eq!(bag("Weapon_Range_SlingShot").unwrap(), "Item_BulletBag_Small");
        assert!(bag("Item_BombBag_Small").unwrap_err().to_string().contains("its own starting bag"));
        assert!(bag("Item_Hookshot").unwrap_err().to_string().contains("no starting bag"));
    }

    #[test]
    fn the_pickup_row_and_the_identity_row_get_the_bag_columns_and_nothing_else() {
        let was: Byml = sample();
        let mut now: Byml = was.clone();
        assert_eq!(apply(&mut now, "Obj_Range_Bow", "Weapon_Range_Bow").unwrap(), "Item_Quiver_Small");
        for who in ["Obj_Range_Bow", "Weapon_Range_Bow"] {
            let r: &Node = find(&now, who).unwrap();
            assert_eq!(r.get(MAX), Some(&Node::I32(30)), "{who}");
            assert_eq!(r.get(KIND).and_then(|v| v.as_str()), Some("Arrow"), "{who}");
            assert_eq!(r.get("IsItemStockMaxIfGet"), Some(&Node::Bool(true)), "{who}");
            assert_eq!(r.get("ActorFullOnlyIfIsGetWhenGetBag"), None, "{who}");
            assert_eq!(r.get("PouchCategory").and_then(|v| v.as_str()), Some("Bow"), "{who}");
        }
        assert_eq!(find(&now, "Weapon_Range_Bow").unwrap().get("AssignableItem").and_then(|v| v.as_str()), Some("Bow"));
        assert_eq!(find(&now, "Item_Range_Bow"), find(&was, "Item_Range_Bow"));
        assert_eq!(find(&now, "Item_Quiver_Small"), find(&was, "Item_Quiver_Small"));
        assert!(audit(&was, &now, "Obj_Range_Bow", "Weapon_Range_Bow").unwrap().contains("kind Arrow"));
        let twice: Byml = {
            let mut y: Byml = now.clone();
            apply(&mut y, "Obj_Range_Bow", "Weapon_Range_Bow").unwrap();
            y
        };
        assert_eq!(twice, now);
        assert!(audit(&now, &twice, "Obj_Range_Bow", "Weapon_Range_Bow").is_ok());
    }

    #[test]
    fn a_row_that_is_already_a_bag_and_a_changed_identity_row_are_refused() {
        let mut y: Byml = sample();
        y.root.as_arr_mut().unwrap().iter_mut().find(|r| r.get("__RowId").and_then(|v| v.as_str()) == Some("Weapon_Range_Bow")).unwrap().set(MAX, Node::I32(50));
        assert!(apply(&mut y, "Obj_Range_Bow", "Weapon_Range_Bow").unwrap_err().to_string().contains("already has another"));
        let was: Byml = sample();
        let mut now: Byml = was.clone();
        apply(&mut now, "Obj_Range_Bow", "Weapon_Range_Bow").unwrap();
        now.root.as_arr_mut().unwrap().iter_mut().find(|r| r.get("__RowId").and_then(|v| v.as_str()) == Some("Weapon_Range_Bow")).unwrap().set("AssignableItem", s("Hammer"));
        assert!(audit(&was, &now, "Obj_Range_Bow", "Weapon_Range_Bow").unwrap_err().to_string().contains("more than the capacity columns"));
        assert!(audit(&was, &was, "Obj_Range_Bow", "Weapon_Range_Bow").is_err());
        let mut bare: Byml = sample();
        bare.root.as_arr_mut().unwrap().retain(|r| r.get("__RowId").and_then(|v| v.as_str()) != Some("Item_Quiver_Small"));
        assert!(apply(&mut bare, "Obj_Range_Bow", "Weapon_Range_Bow").is_err());
    }

    #[test]
    fn the_real_bag_rows_have_the_columns_the_layer_copies() {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let y: Byml = rf.read_byml(POUCH_INFO).unwrap();
        for (identity, item, b, kind, max) in [("Weapon_Range_Bow", "Item_Range_Bow", "Item_Quiver_Small", "Arrow", 30), ("Weapon_Range_SlingShot", "Item_Range_SlingShot", "Item_BulletBag_Small", "DekuSeed", 30)] {
            assert_eq!(bag(identity).unwrap(), b);
            let from: &Node = find(&y, b).unwrap();
            assert!(is_bag(from), "{b}");
            assert_eq!(from.get(KIND).and_then(|v| v.as_str()), Some(kind));
            assert_eq!(from.get(MAX), Some(&Node::I32(max)));
            assert_eq!(from.get("IsItemStockMaxIfGet"), Some(&Node::Bool(true)));
            for who in [identity, item] {
                let r: &Node = find(&y, who).unwrap();
                assert!(COLS.iter().all(|k| r.get(k).is_none()), "{who}");
            }
            let mut now: Byml = y.clone();
            let new: Node = pouch_row(find(&y, item).unwrap(), "Obj_Test_Ammo");
            insert(&mut now, new).unwrap();
            let mid: Byml = now.clone();
            apply(&mut now, "Obj_Test_Ammo", identity).unwrap();
            assert!(audit(&mid, &now, "Obj_Test_Ammo", identity).is_ok());
            let a: &[Node] = mid.root.as_arr().unwrap();
            let c: &[Node] = now.root.as_arr().unwrap();
            let diff: Vec<&str> = (0..a.len()).filter(|i| a[*i] != c[*i]).filter_map(|i| a[i].get("__RowId").and_then(|v| v.as_str())).collect();
            assert_eq!(diff, vec!["Obj_Test_Ammo", identity]);
        }
        let cols: usize = y.root.as_arr().unwrap().iter().filter(|r| r.get(GATE).is_some()).count();
        assert_eq!(cols, 36);
    }
}
