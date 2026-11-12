use crate::slice::data::Data;
use crate::slice::edit::is_sale;
use crate::slice::flow_rules::linked;

pub fn pool_actor(d: &Data, loc: &str, flow_actor: &str) -> String {
    match d.vanilla_of(loc).and_then(|v| d.item(v)) {
        Some(i) if !linked(i, flow_actor) => i.actor.clone().unwrap_or_else(|| flow_actor.to_string()),
        _ => flow_actor.to_string(),
    }
}

pub fn dest_form(d: &Data, item: &str, loc: &str, actor: String) -> String {
    let Some(i) = d.item(item) else {
        return actor;
    };
    if i.world.is_empty() {
        return actor;
    }
    if !is_sale(d, loc) {
        return if i.world.contains(&actor) { i.actor.clone().unwrap_or(actor) } else { actor };
    }
    if d.flow.usable(&actor) {
        return actor;
    }
    match i.world.iter().find(|w| d.flow.usable(w)) {
        Some(w) => w.clone(),
        None => actor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rupee_forms_follow_the_destination() {
        let d: Data = Data::load().unwrap();
        assert_eq!(dest_form(&d, "Rupees (200)", "Kak 100 Gold Skulltula Reward", "TBox_Rupee_Gold".to_string()), "Obj_Rupee_Gold");
        assert_eq!(dest_form(&d, "Rupees (200)", "Some Chest", "Obj_Rupee_Gold".to_string()), "TBox_Rupee_Gold");
        assert_eq!(dest_form(&d, "Rupees (200)", "Some Chest", "TBox_Rupee_Gold".to_string()), "TBox_Rupee_Gold");
        assert_eq!(dest_form(&d, "Piece of Heart", "Kak 50 Gold Skulltula Reward", "Obj_Heart_Piece".to_string()), "Obj_Heart_Piece");
    }

    #[test]
    fn a_tiered_flow_with_a_junk_vanilla_item_pools_the_junk_actor() {
        let d: Data = Data::load().unwrap();
        assert_eq!(pool_actor(&d, "LW Target in Woods", "Item_BulletBag_Medium"), "TBox_Rupee_Blue");
        assert_eq!(pool_actor(&d, "Kak 100 Gold Skulltula Reward", "Obj_Rupee_Gold"), "Obj_Rupee_Gold");
        assert_eq!(pool_actor(&d, "Deku Theater Skull Mask", "Item_StickBag_Medium"), "Item_StickBag_Medium");
    }
}
