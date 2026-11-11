use std::fs;
use std::path::Path as P;

use crate::bfevfl::pace::scale_waits;
use crate::bfevfl::Bf;
use crate::error::Res;
use crate::flow_locations::dict_id;
use crate::modpack::Pack;
use crate::romfs::Romfs;
use crate::zs;

pub const FACTOR: f32 = 0.4;
pub const MIN: i32 = 10;
pub const DICT: u32 = 1;
pub const FILES: [&str; 3] = [
    "Event/EventFlow/Dm_SY_Get.bfevfl.zs",
    "Event/EventFlow/Dm_SY_Treasurebox.bfevfl.zs",
    "Event/EventFlow/Dm_SY_TreasureboxSmall.bfevfl.zs",
];

pub fn pace(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    scale_waits(&mut b, FACTOR, MIN);
    b.write()
}

pub fn apply(rf: &mut Romfs, pk: &Pack) -> Res<usize> {
    let mut n: usize = 0;
    for f in FILES {
        let mine = pk.work.join("romfs").join(f);
        let z: Vec<u8> = if P::new(&mine).is_file() { fs::read(&mine)? } else { rf.read(f)? };
        let id: u32 = dict_id(&z).unwrap_or(DICT);
        let raw: Vec<u8> = rf.zs.decompress(&z)?;
        let out: Vec<u8> = pace(&raw)?;
        pk.write_romfs(f, &zs::compress_raw(&out, Some(id))?)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Cont, Entry, Event, Flow, Kind, Val};

    fn wait(n: &str, frames: i32) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Action {
                next: 0xFFFF,
                actor: 0,
                act: 0,
                params: Some(Cont { items: vec![("Frames".to_string(), Val::Int(frames))] }),
            },
        }
    }

    #[test]
    fn long_waits_shrink_and_short_ones_stay() {
        let b: Bf = Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: "T".to_string(),
            flows: vec![Flow {
                name: "T".to_string(),
                actors: vec![Actor {
                    name: "A".to_string(),
                    sub: String::new(),
                    arg: String::new(),
                    actions: vec!["EventWait".to_string()],
                    queries: Vec::new(),
                    params: None,
                    ep: 0xFFFF,
                    cut: 1,
                }],
                events: vec![wait("E0", 100), wait("E1", 5)],
                entries: vec![Entry { name: "Start".to_string(), subs: vec![0], vars: Vec::new(), main: 0, pad: 0 }],
            }],
            tls: Vec::new(),
        };
        let out: Vec<u8> = pace(&b.write().unwrap()).unwrap();
        let back: Bf = Bf::parse(&out).unwrap();
        let get = |i: usize| back.flows[0].events[i].kind.params().and_then(|c| c.get("Frames").cloned());
        assert_eq!(get(0), Some(Val::Int(40)));
        assert_eq!(get(1), Some(Val::Int(5)));
    }
}
