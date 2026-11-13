use std::collections::HashMap as HM;

use crate::error::{Error as E, Res};
use crate::pickup_pack::rows::GAME_ACTOR_INFO;
use crate::pickup_pack::write::{actor_of, File};
use crate::pickup_persist::{respawn_flags, verdict_with, Extra, Verdict};
use crate::romfs::Romfs;

pub fn extra(files: &[File], carried: &[File]) -> Res<Extra> {
    let mut out: Extra = Extra::default();
    for f in files.iter().chain(carried.iter()) {
        if f.path == GAME_ACTOR_INFO {
            out.add_table(&f.raw)?;
        } else if f.path.starts_with("Pack/Actor/") {
            out.add_pack(actor_of(&f.path), &f.raw)?;
        }
    }
    Ok(out)
}

pub fn check(rf: &mut Romfs, name: &str, files: &[File], carried: &[File]) -> Res<String> {
    let x: Extra = extra(files, carried)?;
    if !x.has(name) {
        return Err(E::Msg(format!("guard: the pack of {name} was not written")));
    }
    let flags: HM<String, bool> = respawn_flags(rf)?;
    let v: Verdict = verdict_with(rf, &flags, &x, name)?;
    if !v.stays() {
        return Err(E::Msg(format!("{name} would not stay collected once taken ({}), so the pack is refused", v.why())));
    }
    Ok(format!("{name} stays collected once taken: IsRespawn row, game data setting {}, death table {}", v.setting, v.table))
}

#[cfg(test)]
mod tests {
    use std::path::{Path as P, PathBuf as PB};

    use super::*;
    use crate::byml::{Byml, Node};
    use crate::modpack::Host;
    use crate::pickup_pack::closure::actor_file;
    use crate::pickup_pack::write::{run, Done, Opts};
    use crate::pickup_pack::{pack_of, Spec, Style};
    use crate::pickup_persist::ALWAYS;
    use crate::sarc::{self, Entry};

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn built(root: &P) -> Done {
        let s: Spec = Spec { item: "Item_Hookshot".to_string(), name: "Obj_Hookshot".to_string(), style: Style::Touch, heap: None };
        let o: Opts = Opts { host: Host::Hookshot, base: None, rst_from: None, places: Vec::new(), dry: true };
        run(root, P::new("unused"), "unused", &s, &o).unwrap()
    }

    fn with_setting(f: &File, setting: &str) -> File {
        let mut es: Vec<Entry> = sarc::parse(&f.raw).unwrap();
        let name: String = actor_file("Obj_Hookshot");
        let mut y: Byml = Byml::parse(&sarc::get(&es, &name).unwrap().data).unwrap();
        let r: String = format!("?Component/ActorGameDataSetting/{setting}.game__component__ActorGameDataSetting.bgyml");
        y.root.get_mut("Components").unwrap().set("ActorGameDataSetting", Node::Str(r));
        assert!(sarc::set(&mut es, &name, y.write().unwrap()));
        File { raw: sarc::write(&es).unwrap(), ..f.clone() }
    }

    #[test]
    fn the_real_hookshot_pack_inherits_the_pickup_rules_and_passes() {
        let Some(root) = real() else {
            return;
        };
        let d: Done = built(&root);
        assert!(d.report.contains("Obj_Hookshot stays collected once taken: IsRespawn row, game data setting NoRespawn, death table DropItem"));
        let x: Extra = extra(&d.files, &d.carried).unwrap();
        assert!(x.has("Obj_Hookshot"));
        assert_eq!(x.packs.len(), 1);
        assert_eq!(x.flags.get("Obj_Hookshot"), Some(&true));
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        assert!(check(&mut rf, "Obj_Hookshot", &d.files, &d.carried).is_ok());
        assert!(check(&mut rf, "Obj_Boomerang", &d.files, &d.carried).is_err());
    }

    #[test]
    fn a_pack_that_always_respawns_or_has_no_respawn_row_is_refused() {
        let Some(root) = real() else {
            return;
        };
        let d: Done = built(&root);
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let at: usize = d.files.iter().position(|f| f.path == pack_of("Obj_Hookshot")).unwrap();
        let mut bad: Vec<File> = d.files.clone();
        bad[at] = with_setting(&d.files[at], ALWAYS);
        let e: String = check(&mut rf, "Obj_Hookshot", &bad, &[]).unwrap_err().to_string();
        assert!(e.contains(ALWAYS), "{e}");
        let mut same: Vec<File> = d.files.clone();
        same[at] = with_setting(&d.files[at], "NoRespawn");
        assert!(check(&mut rf, "Obj_Hookshot", &same, &[]).is_ok());
        let ti: usize = d.files.iter().position(|f| f.path == GAME_ACTOR_INFO).unwrap();
        let mut y: Byml = Byml::parse(&d.files[ti].raw).unwrap();
        let row: &mut Node = y.root.as_arr_mut().unwrap().iter_mut().find(|r| crate::pickup_pack::rows::id(r) == Some("Obj_Hookshot")).unwrap();
        row.set(crate::pickup_pack::rows::RESPAWN, Node::Bool(false));
        let mut off: Vec<File> = d.files.clone();
        off[ti].raw = y.write().unwrap();
        let e: String = check(&mut rf, "Obj_Hookshot", &off, &[]).unwrap_err().to_string();
        assert!(e.contains("IsRespawn"), "{e}");
        let none: Vec<File> = d.files.iter().filter(|f| f.path != pack_of("Obj_Hookshot")).cloned().collect();
        assert!(check(&mut rf, "Obj_Hookshot", &none, &[]).is_err());
    }
}
