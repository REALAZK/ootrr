use std::collections::BTreeSet as BS;
use std::path::Path as P;

use crate::ainb::start;
use crate::bomb_lift;
use crate::fast;
use crate::flags_build::LIST_PATH;
use crate::flow_locations::path_of;
use crate::hints;
use crate::progressive::{ANALYZED_TABLE, BIG_FLOW, SETTING_TABLE, SMALL_FLOW};
use crate::rsizetable::TABLE_PATH;
use crate::slice::data::Data;
use crate::song_models;
use crate::songs;
use crate::special::SPOTS;

pub const LANGUAGE: &str = "Mals/USen.Product.110.sarc.zs";

pub fn list(d: &Data) -> Vec<String> {
    let mut out: BS<String> = BS::new();
    out.extend(d.scan.files.iter().cloned());
    for l in &d.flow.locs {
        for e in &l.edits {
            out.insert(path_of(&e.file));
            out.extend(e.also.iter().map(|a| path_of(a)));
        }
    }
    for s in [BIG_FLOW, SMALL_FLOW, SETTING_TABLE, ANALYZED_TABLE, TABLE_PATH, LIST_PATH, start::MODULE, songs::FLOW, hints::FLOW, LANGUAGE] {
        out.insert(s.to_string());
    }
    out.extend(fast::FILES.iter().map(|s| s.to_string()));
    out.extend(SPOTS.iter().map(|s| s.flow.to_string()));
    out.extend(song_models::SONGS.iter().map(|s| song_models::pack_of(s)));
    out.extend(bomb_lift::PACKS.iter().map(|s| bomb_lift::pack_of(s)));
    out.insert("Pack/ZsDic.pack.zs".to_string());
    out.into_iter().collect()
}

pub fn missing(root: &P, d: &Data) -> Vec<String> {
    list(d)
        .into_iter()
        .filter(|r| !r.split('/').fold(root.to_path_buf(), |a, s| a.join(s)).is_file())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_names_the_files_a_default_build_reads() {
        let d: Data = Data::load().unwrap();
        let l: Vec<String> = list(&d);
        assert!(l.iter().any(|x| x == TABLE_PATH));
        assert!(l.iter().any(|x| x.starts_with("Banc/")));
        assert!(l.iter().any(|x| x.contains("Enemy_Scrub_Business_Common")));
        assert!(l.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn an_empty_folder_misses_everything() {
        let d: Data = Data::load().unwrap();
        let dir = std::env::temp_dir().join(format!("ootrr_required_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(missing(&dir, &d).len(), list(&d).len());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
