use super::kind::Kind;
use super::model::Save;
use super::presets::{BOSS_NAMES, DUNGEON_NAMES, MEDAL_NAMES, SONG_NAMES, STONE_NAMES};
use super::text::{hash_of, show};
use super::val::Val;
use crate::flags::{Flag, FlagList};

fn flag_bool(s: &Save, n: &str) -> Option<bool> {
    match s.get(hash_of(n))? {
        Val::Bool(x) => Some(x),
        _ => None,
    }
}

fn flag_int(s: &Save, n: &str) -> Option<i32> {
    match s.get(hash_of(n))? {
        Val::Int(x) => Some(x),
        _ => None,
    }
}

fn flag_txt(s: &Save, l: &FlagList, n: &str) -> String {
    let Some(v) = s.get(hash_of(n)) else {
        return "?".to_string();
    };
    match l.find(n) {
        Some(f) => show(f, &v),
        None => format!("{v:?}"),
    }
}

fn yes(b: Option<bool>) -> &'static str {
    match b {
        Some(true) => "yes",
        Some(false) => "no",
        None => "?",
    }
}

fn quarters(q: Option<i32>) -> String {
    match q {
        Some(x) => format!("{} hearts ({x} quarters)", x as f32 / 4.0),
        None => "?".to_string(),
    }
}

fn lines_for(title: &str, names: &[&str], f: impl Fn(&str) -> String) -> String {
    let v: Vec<String> = names.iter().map(|n| format!("{n}={}", f(n))).collect();
    format!("{title}: {}", v.join(" "))
}

fn held(s: &Save, l: &FlagList) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for fl in &l.flags {
        let Some(n) = fl.name.as_deref() else {
            continue;
        };
        let Some(a) = n
            .strip_prefix("PouchData.")
            .and_then(|r| r.strip_suffix(".StockNum"))
        else {
            continue;
        };
        if let Some(Val::Int(c)) = s.get(fl.hash as u32) {
            if c != 0 {
                out.push(format!("{a} x{c}"));
            }
        }
    }
    out
}

fn counts(s: &Save) -> (usize, usize, usize) {
    let mut t: usize = 0;
    let mut n: usize = 0;
    let mut z: usize = 0;
    for r in &s.rows {
        match (r.kind, r.get()) {
            (Kind::Bool, Val::Bool(true)) => t += 1,
            (Kind::Int, Val::Int(x)) if x != 0 => n += 1,
            (Kind::Sep, _) => z += 1,
            _ => {}
        }
    }
    (t, n, z)
}

pub fn report(s: &Save, l: &FlagList, size: usize) -> Vec<String> {
    let mut o: Vec<String> = Vec::new();
    let (t, n, z): (usize, usize, usize) = counts(s);
    o.push(format!(
        "file: {size} bytes, version 0x{:08x}, {} table rows ({z} separators), {} collected keys",
        s.version(),
        s.rows.len(),
        s.keys.len()
    ));
    o.push(format!("bool flags set: {t}, int flags non-zero: {n}"));
    o.push(format!(
        "place: banc {} pos {} child {}",
        flag_txt(s, l, "Sequence_CurrentBanc"),
        flag_txt(s, l, "PlayerStatus.SavePos"),
        yes(flag_bool(s, "Sequence_IsChildhood"))
    ));
    o.push(format!(
        "hearts: max {} now {}, magic max {}, rupees {}",
        quarters(flag_int(s, "PlayerStatus.MaxLife")),
        quarters(flag_int(s, "PlayerStatus.Life")),
        flag_int(s, "PlayerStatus.MPMax").unwrap_or(-1),
        flag_int(s, "PlayerStatus.CurrentRupee").unwrap_or(-1)
    ));
    let g = |p: &str, n: &str| yes(flag_bool(s, &format!("{p}{n}"))).to_string();
    let known: usize = SONG_NAMES
        .iter()
        .filter(|n| flag_bool(s, &format!("IsGet.Item_Melody_{n}")) == Some(true))
        .count();
    o.push(format!(
        "ocarina of time: {}, songs known: {known} of {}",
        g("IsGet.", "Item_Ocarina_Time"),
        SONG_NAMES.len()
    ));
    o.push(lines_for("songs", &SONG_NAMES, |n| {
        g("IsGet.Item_Melody_", n)
    }));
    o.push(lines_for("stones", &STONE_NAMES, |n| {
        g("IsGet.Item_Stone_", n)
    }));
    o.push(lines_for("medallions", &MEDAL_NAMES, |n| {
        g("IsGet.Item_Medal_", n)
    }));
    o.push(lines_for("sage steps", &MEDAL_NAMES, |n| {
        flag_txt(s, l, &format!("Step_SageOf{n}"))
    }));
    o.push(lines_for("stone steps", &STONE_NAMES, |n| {
        flag_txt(s, l, &format!("Step_Get_StoneOf{n}"))
    }));
    o.push(lines_for("bosses defeated", &BOSS_NAMES, |n| {
        let c: i32 = flag_int(s, &format!("DefeatedEnemyNum.Enemy_DungeonBoss_{n}")).unwrap_or(0);
        c.to_string()
    }));
    o.push("dungeons (small keys, boss key, map, compass):".to_string());
    for d in DUNGEON_NAMES {
        let k: i32 = flag_int(s, &format!("DungeonInfo.{d}.SmallKeyNum")).unwrap_or(-1);
        o.push(format!(
            "  {d}: {k} {} {} {}",
            yes(flag_bool(s, &format!("DungeonInfo.{d}.IsGetBossKey"))),
            yes(flag_bool(s, &format!("DungeonInfo.{d}.IsGetMap"))),
            yes(flag_bool(s, &format!("DungeonInfo.{d}.IsGetCompass")))
        ));
    }
    let h: Vec<String> = held(s, l);
    o.push(format!("pouch ({} entries): {}", h.len(), h.join(", ")));
    o.push(format!(
        "equipment strings: sword {} shield {} tunic {} boots {}",
        flag_txt(s, l, "PlayerStatus.EquipWeapon"),
        flag_txt(s, l, "PlayerStatus.EquipShield"),
        flag_txt(s, l, "PlayerStatus.EquipArmor"),
        flag_txt(s, l, "PlayerStatus.EquipBoot")
    ));
    o
}

pub fn flag_label(f: &Flag) -> String {
    match &f.name {
        Some(n) => n.clone(),
        None => format!("0x{:08x}", f.hash),
    }
}
