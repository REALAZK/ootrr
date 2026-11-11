use std::collections::HashSet as HS;

use crate::error::{Error as E, Res};
use crate::logic_json::{parse, Json};
use crate::logic_val::Settings;
use crate::slice::data::{Data, SEP};
use crate::world::{Spec, World};

const SHIMS: &str = include_str!("../../../../data/logic/shims.json");
const SETTINGS: &str = include_str!("../../../../data/logic/settings.json");
const HELPERS: &str = include_str!("../../../../data/logic/LogicHelpers.json");

const TOWER: [&str; 3] = ["Ganons Castle Tower", "Ganons Castle Tower Below Boss", "Ganondorf Boss Room"];

const WORLD: [&str; 14] = [
    include_str!("../../../../data/logic/World/Overworld.json"),
    include_str!("../../../../data/logic/World/Bosses.json"),
    include_str!("../../../../data/logic/World/Deku Tree.json"),
    include_str!("../../../../data/logic/World/Dodongos Cavern.json"),
    include_str!("../../../../data/logic/World/Jabu Jabus Belly.json"),
    include_str!("../../../../data/logic/World/Forest Temple.json"),
    include_str!("../../../../data/logic/World/Fire Temple.json"),
    include_str!("../../../../data/logic/World/Water Temple.json"),
    include_str!("../../../../data/logic/World/Shadow Temple.json"),
    include_str!("../../../../data/logic/World/Spirit Temple.json"),
    include_str!("../../../../data/logic/World/Bottom of the Well.json"),
    include_str!("../../../../data/logic/World/Ice Cavern.json"),
    include_str!("../../../../data/logic/World/Gerudo Training Ground.json"),
    include_str!("../../../../data/logic/World/Ganons Castle.json"),
];

fn unique(files: Vec<Json>) -> Vec<Json> {
    let mut seen: HS<String> = HS::new();
    let mut out: Vec<Json> = Vec::new();
    for f in files {
        let Json::Arr(regions) = f else {
            out.push(f);
            continue;
        };
        let mut rs: Vec<Json> = Vec::with_capacity(regions.len());
        for r in regions {
            let Json::Obj(fields) = r else {
                rs.push(r);
                continue;
            };
            let name: String = fields
                .iter()
                .find(|(k, _)| k == "region_name")
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut nf: Vec<(String, Json)> = Vec::with_capacity(fields.len());
            for (k, v) in fields {
                if k == "locations"
                    && let Json::Obj(locs) = v
                {
                    let mut nl: Vec<(String, Json)> = Vec::with_capacity(locs.len());
                    for (ln, lr) in locs {
                        let full: String = if seen.contains(&ln) { format!("{ln}{SEP}{name}") } else { ln.clone() };
                        seen.insert(ln);
                        nl.push((full, lr));
                    }
                    nf.push((k, Json::Obj(nl)));
                } else {
                    nf.push((k, v));
                }
            }
            rs.push(Json::Obj(nf));
        }
        out.push(Json::Arr(rs));
    }
    out
}

pub fn spec(d: &Data, over: &[(&str, &str)]) -> Res<Spec> {
    let mut s: Spec = Spec::from_bundle(SHIMS)?;
    s.settings = Settings::from_json(&parse(SETTINGS)?)?;
    for (k, v) in over {
        s.settings.set_str(k, v);
    }
    for it in &d.items {
        s.add_item(&it.name, it.adv, 0);
    }
    s.add_helpers(HELPERS)?;
    let mut files: Vec<Json> = Vec::with_capacity(WORLD.len());
    for t in WORLD {
        files.push(parse(t)?);
    }
    s.regions.extend(unique(files));
    Ok(s)
}

pub fn world(d: &Data) -> Res<World> {
    world_with(d, &[])
}

pub fn world_with(d: &Data, over: &[(&str, &str)]) -> Res<World> {
    let s: Spec = spec(d, over)?;
    let mut w: World = World::build(&s).map_err(|e| E::Msg(format!("logic world: {e}")))?;
    for r in w.regions.iter_mut() {
        if r.dungeon.is_none() && TOWER.contains(&r.name.as_str()) {
            r.dungeon = Some("Ganons Castle".to_string());
        }
    }
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vanilla_world_builds_strictly() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        assert!(w.real.len() > 1000);
        assert!(w.loc("Deku Tree Map Chest").is_some());
        assert!(w.loc("Ganon").is_some());
    }
}
