use crate::byml::Byml;
use crate::chest_slot::LIST;
use crate::error::Res;
use crate::flow_locations::dict_id;
use crate::pots::one_time;
use crate::romfs::Romfs;
use crate::staging::{rewrite, Out};
use crate::zs;

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>, hashes: &[u64]) -> Res<()> {
    if hashes.is_empty() {
        return Ok(());
    }
    match more.iter().position(|o| o.path == LIST) {
        Some(i) => {
            let id: Option<u32> = dict_id(&more[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&more[i].z)?;
            let mut y: Byml = Byml::parse(&raw)?;
            one_time(&mut y, hashes)?;
            let out: Vec<u8> = y.write()?;
            more[i].new = out.len();
            more[i].z = zs::compress_raw(&out, id)?;
        }
        None => {
            let o: Out = rewrite(rf, LIST, |raw| {
                let mut y: Byml = Byml::parse(raw)?;
                one_time(&mut y, hashes)?;
                y.write()
            })?;
            more.push(o);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::Node;
    use crate::pots::TABLE;

    #[test]
    fn the_row_edit_makes_each_collected_row_permanent_through_the_shared_helper() {
        let row = |h: u64, t: i32| {
            Node::Map(vec![
                ("ExtraByte".to_string(), Node::I32(3)),
                ("Hash".to_string(), Node::U64(h)),
                ("ResetTypeValue".to_string(), Node::I32(t)),
                ("SaveFileIndex".to_string(), Node::I32(0)),
            ])
        };
        let data: Node = Node::Map(vec![(TABLE.to_string(), Node::Arr(vec![row(1, 88), row(2, 280), row(3, 88)]))]);
        let mut y: Byml = Byml { root: Node::Map(vec![("Data".to_string(), data)]), version: 2, big: false };
        one_time(&mut y, &[1, 2]).unwrap();
        let t: &[Node] = y.root.get("Data").unwrap().get(TABLE).unwrap().as_arr().unwrap();
        assert_eq!(t[0].get("ResetTypeValue"), Some(&Node::I32(24)));
        assert_eq!(t[1].get("ResetTypeValue"), Some(&Node::I32(24)));
        assert!(t[0].get("ExtraByte").is_none() && t[1].get("ExtraByte").is_none());
        assert_eq!(t[2].get("ResetTypeValue"), Some(&Node::I32(88)));
        assert!(t[2].get("ExtraByte").is_some());
    }
}
