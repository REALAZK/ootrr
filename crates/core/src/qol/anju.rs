use crate::ainb::Ainb;
use crate::error::{Error, Res};
use crate::qol::{edit_plain, Have};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const FILE: &str = "Logic/MiniGame_CatchCucco_c69b.logic.module.ainb";
pub const LOGIC_KEY: &str = "chicken_count";
pub const NODE: &str = "OperateCompareInt";
pub const COUNTER: &str = "OperateBoolCount";
pub const INPUT: &str = "Int_A";
pub const OLD: i32 = 7;
pub const NEW: i32 = 1;

pub const NOTE: &str = "Anju needs one cucco in the pen instead of seven. The remaining six still run loose and the texts still say seven. Experimental and untested in the game.";

fn hits(a: &Ainb) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for i in a.find_els(NODE) {
        let Some(g) = a.find_in(i, 0, INPUT) else {
            continue;
        };
        let Some(x) = a.input(0, g) else {
            continue;
        };
        if x.src == -1 && a.in_s32(i, INPUT) == Some(OLD) {
            out.push((i, g));
        }
    }
    out
}

fn counts_a_bool_array(a: &Ainb, i: usize) -> bool {
    let Some(g) = a.find_in(i, 0, "Int_B") else {
        return false;
    };
    let Some(x) = a.input(0, g) else {
        return false;
    };
    x.src >= 0 && a.el_name(x.src as usize) == COUNTER
}

pub fn patch(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let h: Vec<(usize, usize)> = hits(&a);
    let [(i, g)] = h.as_slice() else {
        return Err(Error::Msg(format!("anju one cucco: expected one compare node with {INPUT} {OLD}, found {}", h.len())));
    };
    if !counts_a_bool_array(&a, *i) {
        return Err(Error::Format("anju one cucco: the compare node does not read the bool counter"));
    }
    a.set_in_val(0, *g, &NEW.to_le_bytes())?;
    let out: Vec<u8> = a.write()?;
    let b: Ainb = Ainb::parse(&out)?;
    if !hits(&b).is_empty() || b.in_s32(*i, INPUT) != Some(NEW) {
        return Err(Error::Format("anju one cucco: the written module does not read back"));
    }
    Ok(out)
}

pub fn stage(rf: &Romfs, have: Have) -> Res<Out> {
    edit_plain(rf, have, FILE, patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_dump_module_round_trips_and_has_exactly_one_hit_that_reads_the_counter() {
        let Some(root) = dump() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(FILE).unwrap();
        let a: Ainb = Ainb::parse(&raw).unwrap();
        assert_eq!(a.write().unwrap(), raw);
        let h: Vec<(usize, usize)> = hits(&a);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].0, 17);
        assert!(counts_a_bool_array(&a, 17));
        assert_eq!(a.el_name(9), COUNTER);
    }

    #[test]
    fn the_patch_changes_only_the_constant_and_refuses_a_second_run() {
        let Some(root) = dump() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(FILE).unwrap();
        let out: Vec<u8> = patch(&raw).unwrap();
        assert_eq!(out.len(), raw.len());
        let diff: Vec<usize> = (0..raw.len()).filter(|&i| raw[i] != out[i]).collect();
        assert_eq!(diff.len(), 1, "{diff:?}");
        assert_eq!(raw[diff[0]], OLD as u8);
        assert_eq!(out[diff[0]], NEW as u8);
        let a: Ainb = Ainb::parse(&out).unwrap();
        assert_eq!(a.in_s32(17, INPUT), Some(NEW));
        assert_eq!(a.in_s32(17, "Int_B"), Some(0));
        assert!(patch(&out).is_err());
    }

    #[test]
    fn no_other_cucco_module_has_the_constant() {
        let Some(root) = dump() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        for h in ["0ea7", "9eec", "a37a"] {
            let raw: Vec<u8> = rf.read(&format!("Logic/MiniGame_CatchCucco_{h}.logic.module.ainb")).unwrap();
            assert!(hits(&Ainb::parse(&raw).unwrap()).is_empty(), "{h}");
        }
    }

    #[test]
    fn staging_reads_the_vanilla_file_or_an_earlier_output() {
        let Some(root) = dump() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let o: Out = stage(&rf, &none).unwrap();
        assert_eq!(o.path, FILE);
        assert_eq!(o.old, o.new);
        let first: Vec<u8> = o.z.clone();
        let again = |_p: &str| -> Option<Vec<u8>> { Some(first.clone()) };
        assert!(stage(&rf, &again).is_err());
    }

    #[test]
    fn a_module_without_the_node_is_refused() {
        let Some(root) = dump() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read("Logic/MiniGame_CatchCucco_9eec.logic.module.ainb").unwrap();
        assert!(patch(&raw).is_err());
    }
}
