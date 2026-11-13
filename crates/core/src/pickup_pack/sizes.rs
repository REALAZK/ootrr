use crate::error::{Error as E, Res};
use crate::rsizetable::{align, Rst};

pub const PACK_EXTRA: u64 = 312;
pub const FILE_MARGIN: u64 = 0x400;

pub fn pack_size(sarc_len: usize) -> Res<u32> {
    u32::try_from(align(sarc_len as u64) + PACK_EXTRA).map_err(|_| E::Format("pack size does not fit in 32 bits"))
}

pub fn kind(path: &str) -> &str {
    let file: &str = path.rsplit('/').next().unwrap_or(path);
    file.split_once('.').map(|p| p.1).unwrap_or(file)
}

pub fn overhead(rst: &Rst, peers: &[(String, usize)], path: &str) -> Option<u64> {
    let k: &str = kind(path);
    peers
        .iter()
        .filter(|p| kind(&p.0) == k)
        .filter_map(|p| rst.get(&p.0).map(|sz| (sz as u64).saturating_sub(p.1 as u64)))
        .max()
}

pub fn file_size(rst: &Rst, peers: &[(String, usize)], path: &str, len: usize) -> Res<u32> {
    let over: u64 = overhead(rst, peers, path).ok_or_else(|| E::Msg(format!("no shipped file of the kind of {path} has a size table entry to learn from")))?;
    u32::try_from(align(len as u64 + over) + FILE_MARGIN).map_err(|_| E::Format("file size does not fit in 32 bits"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Rst {
        let mut r: Rst = Rst { version: 1, name_len: 160, crc: Vec::new(), names: Vec::new() };
        r.set_size("Actor/A.engine__actor__ActorParam.bgyml", 6488).unwrap();
        r.set_size("Actor/B.engine__actor__ActorParam.bgyml", 9288).unwrap();
        r.set_size("Component/PouchContentParam/A.game__component__PouchContentParam.bgyml", 5568).unwrap();
        r
    }

    fn peers() -> Vec<(String, usize)> {
        vec![
            ("Actor/A.engine__actor__ActorParam.bgyml".to_string(), 936),
            ("Actor/B.engine__actor__ActorParam.bgyml".to_string(), 2920),
            ("Actor/C.engine__actor__ActorParam.bgyml".to_string(), 10),
            ("Component/PouchContentParam/A.game__component__PouchContentParam.bgyml".to_string(), 344),
        ]
    }

    #[test]
    fn a_pack_entry_is_the_aligned_archive_plus_the_fixed_extra() {
        assert_eq!(pack_size(37656).unwrap(), 37976);
        assert_eq!(pack_size(7076).unwrap(), 7416);
        assert_eq!(pack_size(169084).unwrap(), 169400);
    }

    #[test]
    fn the_kind_is_everything_after_the_first_dot_of_the_file_name() {
        assert_eq!(kind("Actor/Obj_X.engine__actor__ActorParam.bgyml"), "engine__actor__ActorParam.bgyml");
        assert_eq!(kind("A/B.C/D.game__x.bgyml"), "game__x.bgyml");
    }

    #[test]
    fn a_new_file_takes_the_largest_overhead_of_its_kind_plus_a_margin() {
        let r: Rst = table();
        assert_eq!(overhead(&r, &peers(), "Actor/New.engine__actor__ActorParam.bgyml"), Some(6368));
        assert_eq!(file_size(&r, &peers(), "Actor/New.engine__actor__ActorParam.bgyml", 500).unwrap(), 6880 + 1024);
        assert_eq!(file_size(&r, &peers(), "Component/PouchContentParam/N.game__component__PouchContentParam.bgyml", 200).unwrap(), 5440 + 1024);
        assert!(file_size(&r, &peers(), "Phive/New.phive__ShapeParam.bgyml", 10).is_err());
    }
}
