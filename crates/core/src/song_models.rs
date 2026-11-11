use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::{rewrite, Out};

pub const MODEL: &str = "Item_Ocarina_Time";
pub const SONGS: [&str; 12] = [
    "ZeldaLullaby",
    "EponaSong",
    "SariaSong",
    "SunSong",
    "TimeSong",
    "StormSong",
    "ForestMinuet",
    "FireBolero",
    "WaterSerenade",
    "SpiritRequiem",
    "ShadowNocturne",
    "LightPrelude",
];

pub fn pack_of(song: &str) -> String {
    format!("Pack/Actor/Item_Melody_{song}.pack.zs")
}

pub fn info_of(song: &str) -> String {
    format!("Component/ModelInfo/Item_Melody_{song}.engine__component__ModelInfo.bgyml")
}

pub fn repoint(raw: &[u8], file: &str, model: &str) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let e: &sarc::Entry = sarc::get(&es, file).ok_or(Error::Format("song models: the pack has no model info"))?;
    let mut y: Byml = Byml::parse(&e.data)?;
    for k in ["FmdbName", "ModelProjectName"] {
        if !y.root.set(k, Node::Str(model.to_string())) {
            return Err(Error::Format("song models: the model info lacks a model name"));
        }
    }
    sarc::set(&mut es, file, y.write()?);
    sarc::write(&es)
}

pub fn stage(rf: &mut Romfs) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for s in SONGS {
        let file: String = info_of(s);
        outs.push(rewrite(rf, &pack_of(s), |raw| repoint(raw, &file, MODEL))?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(m: &str) -> Vec<u8> {
        let y: Byml = Byml {
            root: Node::Map(vec![
                ("FmdbName".to_string(), Node::Str(m.to_string())),
                ("ModelProjectName".to_string(), Node::Str(m.to_string())),
            ]),
            version: 2,
            big: false,
        };
        y.write().unwrap()
    }

    #[test]
    fn the_model_names_change_and_the_rest_of_the_pack_stays() {
        let f: String = info_of("SariaSong");
        let es: Vec<sarc::Entry> = vec![
            sarc::Entry { name: f.clone(), data: info("Obj_Rupee") },
            sarc::Entry { name: "Other/File.bin".to_string(), data: vec![1, 2, 3] },
        ];
        let raw: Vec<u8> = sarc::write(&es).unwrap();
        let out: Vec<u8> = repoint(&raw, &f, MODEL).unwrap();
        let back: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
        let y: Byml = Byml::parse(&sarc::get(&back, &f).unwrap().data).unwrap();
        assert_eq!(y.root.get("FmdbName").and_then(|v| v.as_str()), Some(MODEL));
        assert_eq!(y.root.get("ModelProjectName").and_then(|v| v.as_str()), Some(MODEL));
        assert_eq!(sarc::get(&back, "Other/File.bin").unwrap().data, vec![1, 2, 3]);
        assert!(repoint(&raw, "missing", MODEL).is_err());
    }
}
