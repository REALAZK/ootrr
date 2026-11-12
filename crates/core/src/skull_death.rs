use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const PATH: &str = "System/CombinationDataTableData/Enemy_Skulltula_Gold_game__death__DeathReasonProcedures.pp__CombinationDataTableData.bgyml";
pub const CELLS: &str = "CellList";
pub const FLAG: &str = "IsSetGmdRespawnFlagToFalse";

pub fn persist(raw: &[u8]) -> Res<(Vec<u8>, usize)> {
    let mut y: Byml = Byml::parse(raw)?;
    let cells: &mut Node = y.root.get_mut(CELLS).ok_or(Error::Format("skulltula death table has no cell list"))?;
    let Node::Map(list) = cells else {
        return Err(Error::Format("skulltula death table cell list is not a map"));
    };
    let mut n: usize = 0;
    for (_, c) in list.iter_mut() {
        if c.get(FLAG) == Some(&Node::Bool(false)) {
            c.set(FLAG, Node::Bool(true));
            n += 1;
        }
    }
    Ok((y.write()?, n))
}

pub fn stage(rf: &mut Romfs) -> Res<Out> {
    let raw: Vec<u8> = rf.read(PATH)?;
    let (out, n): (Vec<u8>, usize) = persist(&raw)?;
    if n == 0 {
        return Err(Error::Msg(
            "skulltula death table: no cell keeps the skulltula alive on reload, the game files may have changed".to_string(),
        ));
    }
    Ok(Out { path: PATH.to_string(), old: raw.len(), new: out.len(), z: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(pairs: Vec<(&str, Node)>) -> Node {
        let mut m: Vec<(String, Node)> = vec![("$type".to_string(), Node::Str("game__death__DeathReasonProceduresCell".to_string()))];
        m.extend(pairs.into_iter().map(|(k, v)| (k.to_string(), v)));
        Node::Map(m)
    }

    fn table() -> Vec<u8> {
        let cells: Node = Node::Map(vec![
            ("ChemicalCriticalHitIce___Data".to_string(), cell(vec![(FLAG, Node::Bool(false))])),
            ("Drowned___Data".to_string(), cell(vec![(FLAG, Node::Bool(false))])),
            ("FallIntoAbyss___Data".to_string(), cell(vec![("IsSetRespawnFlagToFalse", Node::Bool(false))])),
            (
                "LifeDepleted___Data".to_string(),
                cell(vec![
                    ("DeathProcedureType", Node::U32(3594736239)),
                    ("IsPlayAnimImmediate", Node::Bool(false)),
                    (FLAG, Node::Bool(false)),
                ]),
            ),
        ]);
        let root: Node = Node::Map(vec![
            ("$parent".to_string(), Node::Str("Work/System/CombinationDataTableData/EnemyCommon.gyml".to_string())),
            (CELLS.to_string(), cells),
            ("TableType".to_string(), Node::Str("Work/System/CombinationDataTable/x.gyml".to_string())),
        ]);
        Byml { root, version: 7, big: false }.write().unwrap()
    }

    #[test]
    fn the_three_cells_persist_and_the_rest_round_trips_byte_for_byte() {
        let raw: Vec<u8> = table();
        let (out, n): (Vec<u8>, usize) = persist(&raw).unwrap();
        assert_eq!(n, 3);
        assert_eq!(out.len(), raw.len());
        let back: Byml = Byml::parse(&out).unwrap();
        let c: &Node = back.root.get(CELLS).unwrap();
        for k in ["ChemicalCriticalHitIce___Data", "Drowned___Data", "LifeDepleted___Data"] {
            assert_eq!(c.get(k).and_then(|x| x.get(FLAG)), Some(&Node::Bool(true)), "{k}");
        }
        assert_eq!(c.get("FallIntoAbyss___Data").and_then(|x| x.get("IsSetRespawnFlagToFalse")), Some(&Node::Bool(false)));
        assert!(c.get("FallIntoAbyss___Data").and_then(|x| x.get(FLAG)).is_none());
        let life: &Node = c.get("LifeDepleted___Data").unwrap();
        assert_eq!(life.get("DeathProcedureType"), Some(&Node::U32(3594736239)));
        assert_eq!(life.get("IsPlayAnimImmediate"), Some(&Node::Bool(false)));
        let (again, m): (Vec<u8>, usize) = persist(&out).unwrap();
        assert_eq!(m, 0);
        assert_eq!(again, out);
        let plain: Vec<u8> = Byml::parse(&raw).unwrap().write().unwrap();
        assert_eq!(plain, raw);
    }

    #[test]
    fn a_table_without_cells_is_refused() {
        let y: Byml = Byml { root: Node::Map(vec![("TableType".to_string(), Node::Str("x".to_string()))]), version: 7, big: false };
        assert!(persist(&y.write().unwrap()).is_err());
    }
}
