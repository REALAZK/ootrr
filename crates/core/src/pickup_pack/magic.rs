use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::pickup_pack::closure::{local, norm, work, Chain, Pool, COMPONENTS, PARENT};

pub const TABLE_REF: &str = "GameParameterTableRef";
pub const ABILITY: &str = "ExpansionAbilityActor";

pub fn table_file(name: &str) -> String {
    format!("GameParameter/GameParameterTable/{name}.engine__actor__GameParameterTable.bgyml")
}

fn table_of(p: &Pool, c: &Chain, who: &str) -> Res<String> {
    let r: &str = c.comps.get(TABLE_REF).ok_or_else(|| E::Msg(format!("{who} has no {TABLE_REF}")))?;
    let f: String = norm(r).ok_or_else(|| E::Msg(format!("{who} has a {TABLE_REF} that is not a file reference")))?;
    if p.get(&f).is_none() {
        return Err(E::Msg(format!("{who} points its {TABLE_REF} at {f}, which is not in its pack")));
    }
    Ok(f)
}

pub fn ability(p: &Pool, item: &Chain, who: &str) -> Res<String> {
    let f: String = table_of(p, item, who)?;
    let y: Byml = p.byml(&f)?;
    let r: &str = y
        .root
        .get(COMPONENTS)
        .and_then(|c| c.get(ABILITY))
        .and_then(|v| v.as_str())
        .ok_or_else(|| E::Msg(format!("{who} has no {ABILITY} in its game parameter table, the magic layer is only for the magic meter, the spells and the magic arrows")))?;
    let held: String = norm(r).ok_or_else(|| E::Msg(format!("{who} has an {ABILITY} that is not a file reference")))?;
    if p.get(&held).is_none() {
        return Err(E::Msg(format!("{who} keeps its {ABILITY} file outside its pack")));
    }
    Ok(r.to_string())
}

pub fn make(p: &Pool, item: &Chain, base: &Chain, who: &str, name: &str) -> Res<(String, Vec<u8>, String)> {
    let up: String = table_of(p, base, "the pickup base")?;
    let r: String = ability(p, item, who)?;
    let version: u16 = p.byml(&up)?.version;
    let mut comps: Node = Node::Map(Vec::new());
    comps.set(ABILITY, Node::Str(r));
    let mut root: Node = Node::Map(Vec::new());
    root.set(PARENT, Node::Str(work(&up)));
    root.set(COMPONENTS, comps);
    let f: String = table_file(name);
    Ok((f.clone(), Byml { root, version, big: false }.write()?, local(&f)))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap as BM;

    use super::*;

    fn file(pairs: &[(&str, Node)]) -> Vec<u8> {
        let mut root: Node = Node::Map(Vec::new());
        for (k, v) in pairs {
            root.set(k, v.clone());
        }
        Byml { root, version: 7, big: false }.write().unwrap()
    }

    fn comps(pairs: &[(&str, &str)]) -> Node {
        let mut m: Node = Node::Map(Vec::new());
        for (k, v) in pairs {
            m.set(k, Node::Str(v.to_string()));
        }
        m
    }

    fn chain_of(table: &str) -> Chain {
        let mut c: BM<String, String> = BM::new();
        c.insert(TABLE_REF.to_string(), format!("?{}", table_file(table)));
        Chain { files: Vec::new(), comps: c }
    }

    fn pool() -> Pool {
        let mut p: Pool = Pool::default();
        p.add(&table_file("Obj_Item_Base"), &file(&[("Category", Node::Str("GameParameterTableDefault".to_string())), (COMPONENTS, comps(&[("DropBoundParam", "?GameParameter/DropBoundParam/Obj_Item_Base.game__drop__DropBoundParam.bgyml")]))])).unwrap();
        p.add(&table_file("Item_Meter"), &file(&[(COMPONENTS, comps(&[(ABILITY, "?GameParameter/ExpansionAbilityActor/Item_Meter.game__actor__ExpansionAbilityActor.bgyml")]))])).unwrap();
        p.add("GameParameter/ExpansionAbilityActor/Item_Meter.game__actor__ExpansionAbilityActor.bgyml", &file(&[("MPMax", Node::I32(24))])).unwrap();
        p.add(&table_file("Item_Plain"), &file(&[(COMPONENTS, comps(&[("MenuActorParam", "?GameParameter/MenuActorParam/A.game__ui__MenuActorParam.bgyml")]))])).unwrap();
        p
    }

    #[test]
    fn the_child_table_inherits_the_pickup_base_and_adds_only_the_magic_raise() {
        let p: Pool = pool();
        let (f, d, r): (String, Vec<u8>, String) = make(&p, &chain_of("Item_Meter"), &chain_of("Obj_Item_Base"), "Item_Meter", "Obj_Meter").unwrap();
        assert_eq!(f, "GameParameter/GameParameterTable/Obj_Meter.engine__actor__GameParameterTable.bgyml");
        assert_eq!(r, format!("?{f}"));
        let y: Byml = Byml::parse(&d).unwrap();
        assert_eq!(y.version, 7);
        assert_eq!(y.root.get(PARENT).and_then(|v| v.as_str()), Some("Work/GameParameter/GameParameterTable/Obj_Item_Base.engine__actor__GameParameterTable.gyml"));
        let Some(Node::Map(m)) = y.root.get(COMPONENTS) else { panic!() };
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].0, ABILITY);
        assert_eq!(m[0].1.as_str(), Some("?GameParameter/ExpansionAbilityActor/Item_Meter.game__actor__ExpansionAbilityActor.bgyml"));
        assert_eq!(y.root.get("Category"), None);
    }

    #[test]
    fn an_item_without_a_magic_raise_is_refused() {
        let p: Pool = pool();
        let e: String = make(&p, &chain_of("Item_Plain"), &chain_of("Obj_Item_Base"), "Item_Plain", "Obj_Plain").unwrap_err().to_string();
        assert!(e.contains("only for the magic meter"), "{e}");
        assert!(make(&p, &chain_of("Item_Gone"), &chain_of("Obj_Item_Base"), "Item_Gone", "Obj_Gone").is_err());
        assert!(make(&p, &Chain { files: Vec::new(), comps: BM::new() }, &chain_of("Obj_Item_Base"), "Item_None", "Obj_None").is_err());
        let mut lost: Pool = Pool::default();
        lost.add(&table_file("Obj_Item_Base"), &file(&[(COMPONENTS, comps(&[]))])).unwrap();
        lost.add(&table_file("Item_Meter"), &file(&[(COMPONENTS, comps(&[(ABILITY, "?GameParameter/ExpansionAbilityActor/Item_Meter.game__actor__ExpansionAbilityActor.bgyml")]))])).unwrap();
        assert!(make(&lost, &chain_of("Item_Meter"), &chain_of("Obj_Item_Base"), "Item_Meter", "Obj_Meter").unwrap_err().to_string().contains("outside its pack"));
    }
}
