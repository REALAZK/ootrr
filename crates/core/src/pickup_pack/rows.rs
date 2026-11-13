use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};

pub const ACTOR_INFO: &str = "RSDB/ActorInfo.Product.111.rstbl.byml.zs";
pub const GAME_ACTOR_INFO: &str = "RSDB/GameActorInfo.Product.111.rstbl.byml.zs";
pub const POUCH_INFO: &str = "RSDB/PouchActorInfo.Product.111.rstbl.byml.zs";
pub const ROW_ID: &str = "__RowId";
pub const HEAP: &str = "InstanceHeapSize";
pub const RESPAWN: &str = "IsRespawn";
pub const LAYER: &str = "MainRigidBodyLayer";
pub const VARIATION: &str = "ModelVariationAnims";
pub const HEAP_PEERS: [&str; 5] = ["Obj_Armor_Goron", "Obj_Arrow_Fire", "Obj_Bombchu", "Obj_Heart_Piece", "Obj_Shield_Deku"];
pub const HEAP_MARGIN: i32 = 0x8000;
const DROPPED: [&str; 3] = ["CalcPriority", "ELinkUserName", "SLinkUserName"];

pub fn id(n: &Node) -> Option<&str> {
    n.get(ROW_ID).and_then(|v| v.as_str())
}

fn list(y: &Byml) -> Res<&[Node]> {
    y.root.as_arr().ok_or(E::Format("actor table: the root is not a row list"))
}

pub fn find<'a>(y: &'a Byml, name: &str) -> Option<&'a Node> {
    y.root.as_arr()?.iter().find(|r| id(r) == Some(name))
}

pub fn need<'a>(y: &'a Byml, name: &str, table: &str) -> Res<&'a Node> {
    find(y, name).ok_or_else(|| E::Msg(format!("{name} has no row in {table}")))
}

pub fn sorted(y: &Byml) -> Res<bool> {
    Ok(list(y)?.windows(2).all(|w| id(&w[0]).unwrap_or("").as_bytes() < id(&w[1]).unwrap_or("").as_bytes()))
}

pub fn insert(y: &mut Byml, row: Node) -> Res<usize> {
    let name: String = id(&row).ok_or(E::Format("actor table: the new row has no id"))?.to_string();
    if !sorted(y)? {
        return Err(E::Format("actor table: the rows are not sorted by id, the game files differ from the ones this was made for"));
    }
    let rows: &mut Vec<Node> = y.root.as_arr_mut().ok_or(E::Format("actor table: the root is not a row list"))?;
    match rows.binary_search_by(|r| id(r).unwrap_or("").as_bytes().cmp(name.as_bytes())) {
        Ok(i) => {
            rows[i] = row;
            Ok(i)
        }
        Err(i) => {
            rows.insert(i, row);
            Ok(i)
        }
    }
}

fn int(n: &Node, k: &str) -> Option<i32> {
    match n.get(k) {
        Some(Node::I32(v)) => Some(*v),
        Some(Node::U32(v)) => i32::try_from(*v).ok(),
        _ => None,
    }
}

pub fn own_heap(table: &Byml, item: &str) -> Res<i32> {
    Ok(int(need(table, item, ACTOR_INFO)?, HEAP).unwrap_or(0))
}

pub fn heap(table: &Byml, item: &str) -> Res<i32> {
    let own: i32 = own_heap(table, item)?;
    let mut top: i32 = own;
    for p in HEAP_PEERS {
        top = top.max(int(need(table, p, ACTOR_INFO)?, HEAP).unwrap_or(0));
    }
    let n: i32 = top.checked_add(HEAP_MARGIN).ok_or(E::Format("actor table: the heap size does not fit"))?;
    Ok((n + 7) / 8 * 8)
}

pub fn actor_row(item_row: &Node, name: &str, heap: i32) -> Node {
    let mut r: Node = item_row.clone();
    for k in DROPPED {
        r.remove(k);
    }
    let v: Node = match item_row.get(HEAP) {
        Some(Node::U32(_)) => Node::U32(heap.max(0) as u32),
        _ => Node::I32(heap),
    };
    r.set(HEAP, v);
    r.set(ROW_ID, Node::Str(name.to_string()));
    r
}

pub fn game_row(base_row: &Node, item_row: &Node, name: &str) -> Node {
    let mut r: Node = base_row.clone();
    if let Some(v) = item_row.get(VARIATION) {
        r.set(VARIATION, v.clone());
    }
    r.set(ROW_ID, Node::Str(name.to_string()));
    r
}

pub fn pouch_row(item_row: &Node, name: &str) -> Node {
    let mut r: Node = item_row.clone();
    r.set(ROW_ID, Node::Str(name.to_string()));
    r
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn row(name: &str, pairs: &[(&str, Node)]) -> Node {
        let mut r: Node = Node::Map(Vec::new());
        for (k, v) in pairs {
            r.set(k, v.clone());
        }
        r.set(ROW_ID, Node::Str(name.to_string()));
        r
    }

    pub(crate) fn table(rows: Vec<Node>) -> Byml {
        Byml { root: Node::Arr(rows), version: 7, big: false }
    }

    fn s(x: &str) -> Node {
        Node::Str(x.to_string())
    }

    #[test]
    fn a_new_row_lands_in_sorted_order_and_a_second_run_replaces_it() {
        let mut y: Byml = table(vec![row("Item_A", &[]), row("Obj_A", &[]), row("Obj_Z", &[])]);
        assert_eq!(insert(&mut y, row("Obj_Hookshot", &[("X", Node::I32(1))])).unwrap(), 2);
        assert_eq!(insert(&mut y, row("Obj_Hookshot", &[("X", Node::I32(2))])).unwrap(), 2);
        let ids: Vec<&str> = y.root.as_arr().unwrap().iter().filter_map(id).collect();
        assert_eq!(ids, vec!["Item_A", "Obj_A", "Obj_Hookshot", "Obj_Z"]);
        assert_eq!(find(&y, "Obj_Hookshot").unwrap().get("X"), Some(&Node::I32(2)));
        assert!(sorted(&y).unwrap());
    }

    #[test]
    fn an_unsorted_table_is_refused() {
        let mut y: Byml = table(vec![row("B", &[]), row("A", &[])]);
        assert!(insert(&mut y, row("C", &[])).is_err());
    }

    #[test]
    fn the_actor_row_keeps_the_item_identity_and_loses_the_link_names() {
        let item: Node = row(
            "Item_Test",
            &[("ActorName", s("Item_Test")), ("CalcPriority", s("After")), ("ELinkUserName", s("Item_Test")), ("FmdbName", s("Body")), (HEAP, Node::I32(1000)), ("SLinkUserName", s("Item_Test"))],
        );
        let r: Node = actor_row(&item, "Obj_Test", 5000);
        assert_eq!(id(&r), Some("Obj_Test"));
        assert_eq!(r.get("ActorName").and_then(|v| v.as_str()), Some("Item_Test"));
        assert_eq!(r.get("FmdbName").and_then(|v| v.as_str()), Some("Body"));
        assert_eq!(r.get(HEAP), Some(&Node::I32(5000)));
        assert!(DROPPED.iter().all(|k| r.get(k).is_none()));
    }

    #[test]
    fn the_heap_is_the_largest_peer_plus_a_margin() {
        let mut rows: Vec<Node> = vec![row("Item_Test", &[(HEAP, Node::I32(130808))])];
        for (i, p) in HEAP_PEERS.iter().enumerate() {
            rows.push(row(p, &[(HEAP, Node::I32(140000 + i as i32 * 1001))]));
        }
        let y: Byml = table(rows);
        assert_eq!(heap(&y, "Item_Test").unwrap(), (144004 + HEAP_MARGIN + 7) / 8 * 8);
        assert!(heap(&y, "Item_None").is_err());
    }

    #[test]
    fn an_item_heavier_than_every_peer_sets_the_heap_itself() {
        let mut rows: Vec<Node> = vec![row("Item_Heavy", &[(HEAP, Node::I32(170648))]), row("Item_Light", &[(HEAP, Node::I32(90000))])];
        for p in HEAP_PEERS {
            rows.push(row(p, &[(HEAP, Node::I32(167160))]));
        }
        let y: Byml = table(rows);
        assert_eq!(own_heap(&y, "Item_Heavy").unwrap(), 170648);
        assert_eq!(heap(&y, "Item_Heavy").unwrap(), 203416);
        assert_eq!(heap(&y, "Item_Light").unwrap(), 199928);
        assert!(own_heap(&y, "Item_None").is_err());
    }

    #[test]
    fn the_game_row_is_the_pickup_base_row_with_the_item_pose() {
        let base: Node = row("Obj_Item_Base", &[(RESPAWN, Node::Bool(true)), (LAYER, s("SmallObject"))]);
        let item: Node = row("Item_Test", &[(VARIATION, Node::Arr(vec![s("pose")])), ("HoldAnimationType", s("Bottle"))]);
        let r: Node = game_row(&base, &item, "Obj_Test");
        assert_eq!(r.get(RESPAWN), Some(&Node::Bool(true)));
        assert_eq!(r.get(LAYER).and_then(|v| v.as_str()), Some("SmallObject"));
        assert_eq!(r.get(VARIATION), item.get(VARIATION));
        assert_eq!(r.get("HoldAnimationType"), None);
        assert_eq!(id(&pouch_row(&item, "Obj_Test")), Some("Obj_Test"));
    }
}
