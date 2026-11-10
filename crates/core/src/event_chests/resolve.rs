use std::collections::HashMap as HM;

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::romfs::Romfs;

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub stem: String,
    pub entry: String,
    pub comp: String,
}

pub fn row_of(r: &Node) -> (String, Row) {
    let g = |k: &str| r.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let id: String = g("__RowId");
    let f: String = g("FlowchartFileName");
    let mut stem: String = f.rsplit('/').next().unwrap_or("").trim_end_matches(".evfc").to_string();
    if stem.is_empty() {
        stem = id.clone();
    }
    let c: String = g("EventSettingComponentList");
    let comp: String = c.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("").to_string();
    (id, Row { stem, entry: g("EntryPointName"), comp })
}

pub fn table(rf: &mut Romfs) -> Res<HM<String, Row>> {
    let y: Byml = rf.read_byml("RSDB/EventSetting.Product.111.rstbl.byml.zs")?;
    Ok(y.root.as_arr().unwrap_or(&[]).iter().map(row_of).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Node {
        Node::Str(v.to_string())
    }

    #[test]
    fn flow_rows_resolve_to_stem_entry_and_component() {
        let n: Node = Node::Map(vec![
            ("__RowId".to_string(), s("Ev_One")),
            ("FlowchartFileName".to_string(), s("Work/Event/EventFlow/NaviChat.evfc")),
            ("EntryPointName".to_string(), s("Go")),
            ("EventSettingComponentList".to_string(), s("Work/x/FlowchartEvent.engine__event__EventSettingComponentList.gyml")),
        ]);
        let (id, r): (String, Row) = row_of(&n);
        assert_eq!(id, "Ev_One");
        assert_eq!((r.stem.as_str(), r.entry.as_str(), r.comp.as_str()), ("NaviChat", "Go", "FlowchartEvent"));
    }

    #[test]
    fn timeline_rows_fall_back_to_the_row_id() {
        let n: Node = Node::Map(vec![("__RowId".to_string(), s("Dm_Tl")), ("FlowchartFileName".to_string(), s(""))]);
        let (_, r): (String, Row) = row_of(&n);
        assert_eq!((r.stem.as_str(), r.entry.as_str()), ("Dm_Tl", ""));
    }
}
