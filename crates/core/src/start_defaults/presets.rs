use crate::error::{Error, Res};
use crate::json::{parse, Json};
use crate::start_defaults::Edit;

pub const JSON: &str = include_str!("../../../../data/start_presets.json");

#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub setting: String,
    pub value: String,
    pub edits: Vec<Edit>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Book {
    pub presets: Vec<Preset>,
    pub control: Vec<Edit>,
}

fn text(j: &Json, k: &str) -> Res<String> {
    j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| Error::Msg(format!("start presets: missing text field {k}")))
}

fn int(v: &Json, name: &str) -> Res<i32> {
    v.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or_else(|| Error::Msg(format!("start presets: {name} needs a 32-bit integer")))
}

fn edit(j: &Json) -> Res<Edit> {
    let kind: String = text(j, "kind")?;
    let name: String = text(j, "name")?;
    let v: &Json = j.get("value").ok_or_else(|| Error::Msg(format!("start presets: {name} has no value")))?;
    match kind.as_str() {
        "bool" => v.as_bool().map(|b| Edit::Bool(name.clone(), b)).ok_or_else(|| Error::Msg(format!("start presets: {name} needs true or false"))),
        "enum" => v.as_str().map(|s| Edit::Enum(name.clone(), s.to_string())).ok_or_else(|| Error::Msg(format!("start presets: {name} needs a value name"))),
        "int" => Ok(Edit::Int(name.clone(), int(v, &name)?)),
        "grant" => Ok(Edit::Grant(name.clone(), int(v, &name)?)),
        "exp" => {
            let mut terms: Vec<(i32, String)> = Vec::new();
            for t in v.as_arr().unwrap_or(&[]) {
                match t.as_arr() {
                    Some([op, f]) => terms.push((int(op, &name)?, f.as_str().ok_or_else(|| Error::Msg(format!("start presets: {name} names a flag that is not text")))?.to_string())),
                    _ => return Err(Error::Msg(format!("start presets: {name} needs a list of [op, flag] pairs"))),
                }
            }
            if terms.is_empty() {
                return Err(Error::Msg(format!("start presets: {name} has an empty formula")));
            }
            Ok(Edit::Exp(name.clone(), terms))
        }
        k => Err(Error::Msg(format!("start presets: {name} has an unknown kind {k}"))),
    }
}

fn edits(j: Option<&Json>) -> Res<Vec<Edit>> {
    j.and_then(|x| x.as_arr()).unwrap_or(&[]).iter().map(edit).collect()
}

pub fn read(t: &str) -> Res<Book> {
    let j: Json = parse(t)?;
    if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
        return Err(Error::Format("start presets: unknown format"));
    }
    let mut presets: Vec<Preset> = Vec::new();
    for p in j.get("presets").and_then(|x| x.as_arr()).unwrap_or(&[]) {
        let setting: String = text(p, "setting")?;
        let value: String = text(p, "value")?;
        if presets.iter().any(|x| x.setting == setting && x.value == value) {
            return Err(Error::Msg(format!("start presets: {setting}={value} is listed twice")));
        }
        let e: Vec<Edit> = edits(p.get("edits"))?;
        if e.is_empty() {
            return Err(Error::Msg(format!("start presets: {setting}={value} has no edits")));
        }
        presets.push(Preset { setting, value, edits: e, note: text(p, "note")? });
    }
    Ok(Book { presets, control: edits(j.get("control"))? })
}

pub fn book() -> Res<Book> {
    read(JSON)
}

pub fn find<'a>(b: &'a Book, setting: &str, value: &str) -> Res<&'a Preset> {
    b.presets
        .iter()
        .find(|p| p.setting == setting && p.value == value)
        .ok_or_else(|| Error::Msg(format!("start presets: nothing is listed for {setting}={value}")))
}

pub fn chosen(picks: &[(&str, &str)]) -> Res<Vec<Edit>> {
    if picks.is_empty() {
        return Ok(Vec::new());
    }
    let b: Book = book()?;
    let mut out: Vec<Edit> = Vec::new();
    for (s, v) in picks {
        out.extend(find(&b, s, v)?.edits.iter().cloned());
    }
    Ok(out)
}

pub fn note(setting: &str, value: &str) -> Res<String> {
    let b: Book = book()?;
    Ok(find(&b, setting, value)?.note.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::FlagList;

    #[test]
    fn the_shipped_book_parses_and_names_known_settings() {
        let b: Book = book().unwrap();
        assert_eq!(b.presets.len(), 7);
        assert_eq!(find(&b, "skip_epona_race", "true").unwrap().edits, vec![Edit::Exp("Get_Epona_IsCompleted_Exp".to_string(), vec![(0, "IsGet.Item_Melody_EponaSong".to_string())])]);
        assert_eq!(find(&b, "skip_mask_quest", "true").unwrap().edits, vec![Edit::Enum("Step_SubEvent_MaskSalesman".to_string(), "Complete".to_string())]);
        let beans: &Preset = find(&b, "plant_beans", "true").unwrap();
        assert_eq!(beans.edits.iter().filter(|e| matches!(e, Edit::Bool(n, true) if n.starts_with("IsPlantedMagicBean_SmallFarm."))).count(), 10);
        assert!(!beans.edits.contains(&Edit::Bool("IsPlantedMagicBean_SmallFarm.9976156629905609191".to_string(), true)));
        assert!(beans.edits.contains(&Edit::Int("Get_MagicBeans".to_string(), 9)));
        let z: &Preset = find(&b, "zora_fountain", "open").unwrap();
        assert_eq!(z.edits, vec![Edit::Enum("Step_Get_StoneOfWater".to_string(), "Step07".to_string())]);
        assert_eq!(find(&b, "jabu_open", "true").unwrap().edits.len(), 2);
        assert_eq!(find(&b, "time_always_passes", "true").unwrap().edits, vec![Edit::Bool("HyruleField_ArriveEntrance".to_string(), true), Edit::Bool("HavePlayedEvent.Dm_HrlField_0020".to_string(), true)]);
        let p: &Preset = find(&b, "open_kakariko", "open").unwrap();
        assert_eq!(p.edits, vec![Edit::Bool("Npc_Hylia_Soldier_003_GateOpen".to_string(), true)]);
        assert!(p.note.contains("Mask Shop"));
        assert_eq!(b.control, vec![Edit::Int("PlayerStatus.CurrentRupee".to_string(), 42)]);
        assert!(chosen(&[]).unwrap().is_empty());
        assert!(chosen(&[("open_kakariko", "closed")]).is_err());
        let s: crate::settings::Settings = crate::settings::Settings::default();
        for p in &b.presets {
            assert!(s.get(&p.setting).is_some(), "{}", p.setting);
        }
    }

    #[test]
    fn bad_books_are_refused() {
        assert!(read("{\"format\":2}").is_err());
        assert!(read("{\"format\":1,\"presets\":[{\"setting\":\"a\",\"value\":\"b\",\"edits\":[],\"note\":\"n\"}]}").is_err());
        let one: &str = "{\"setting\":\"a\",\"value\":\"b\",\"edits\":[{\"kind\":\"bool\",\"name\":\"X\",\"value\":true}],\"note\":\"n\"}";
        assert!(read(&format!("{{\"format\":1,\"presets\":[{one},{one}]}}")).is_err());
        assert!(read("{\"format\":1,\"presets\":[{\"setting\":\"a\",\"value\":\"b\",\"edits\":[{\"kind\":\"float\",\"name\":\"X\",\"value\":1}],\"note\":\"n\"}]}").is_err());
        assert!(read("{\"format\":1,\"control\":[{\"kind\":\"int\",\"name\":\"X\",\"value\":99999999999}]}").is_err());
        assert_eq!(read(&format!("{{\"format\":1,\"presets\":[{one}]}}")).unwrap().presets[0].edits, vec![Edit::Bool("X".to_string(), true)]);
    }

    #[test]
    fn every_book_flag_is_named_in_the_flag_dictionary_with_the_right_type_and_a_new_game_reset() {
        let l: FlagList = crate::flags::from_json(&crate::json::parse(include_str!("../../../../data/flags.json")).unwrap()).unwrap();
        let b: Book = book().unwrap();
        let mut all: Vec<Edit> = b.presets.iter().flat_map(|p| p.edits.clone()).collect();
        all.extend(b.control.clone());
        for e in &all {
            let pairs: Vec<(String, &str)> = match e {
                Edit::Bool(n, _) => vec![(n.clone(), "Bool")],
                Edit::Int(n, _) => vec![(n.clone(), "Int")],
                Edit::Enum(n, _) => vec![(n.clone(), "Enum")],
                Edit::Grant(a, _) => vec![(crate::start_defaults::got(a), "Bool"), (crate::start_defaults::stock(a), "Int")],
                Edit::Exp(n, t) => {
                    assert_eq!(l.find(n).unwrap_or_else(|| panic!("{n} is not in flags.json")).ty, "BoolExp", "{n}");
                    for (_, f) in t {
                        assert!(l.find(f).is_some(), "{f} is not in flags.json");
                    }
                    continue;
                }
            };
            for (n, ty) in pairs {
                let f: &crate::flags::Flag = l.find(&n).unwrap_or_else(|| panic!("{n} is not in flags.json"));
                assert_eq!(f.ty, ty, "{n}");
                assert!(crate::start_defaults::allowed(f.reset as i32, f.save as i32), "{n}");
            }
        }
    }
}
