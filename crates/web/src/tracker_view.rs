use std::collections::BTreeMap;

use ootrr_core::json::Json;
use ootrr_core::tracker::index::{Index, MapInfo, pin_json};
use ootrr_core::tracker::project;
use ootrr_core::tracker::state::{Mark, S};
use ootrr_core::tracker::targets::{self, How, KINDS, OFF_BY_DEFAULT, Target};

use crate::events::{Batch, Ev};
use crate::pathsafe;
use crate::tracker::{Inner, Mode, now_secs};

fn ts() -> &'static [Target] {
    targets::all().unwrap_or(&[])
}

fn kind_on(g: &Inner, t: &Target) -> bool {
    g.opts.kinds.iter().any(|k| k == &t.kind) && (g.opts.dream || !t.dream())
}

pub fn visible(g: &Inner, i: usize) -> bool {
    let Some(t) = ts().get(i) else {
        return false;
    };
    let Some(m) = g.track.marks.get(i) else {
        return false;
    };
    kind_on(g, t) && (g.opts.mode == Mode::All || m.rev)
}

fn opt(s: &Option<String>) -> Json {
    s.as_deref().map(Json::from).unwrap_or(Json::Null)
}

fn how(h: How) -> &'static str {
    match h {
        How::Flag => "flag",
        How::Key => "key",
        How::None => "none",
    }
}

pub fn mark_json(m: &Mark) -> Json {
    Json::obj()
        .with("s", m.s.name().into())
        .with("by", m.by.map(|b| Json::from(b.name())).unwrap_or(Json::Null))
        .with("at", Json::Int(m.at as i64))
}

pub fn loc_json(g: &Inner, i: usize) -> Json {
    let t: &Target = &ts()[i];
    let ix: &Index = &g.index;
    let (ages, pins): (Vec<Json>, Vec<Json>) = match ix.locs.get(i) {
        Some(l) => (
            l.ages.iter().map(|a| Json::from(a.as_str())).collect(),
            l.pins.iter().map(|p| pin_json(p, ix.floors_of(&p.map))).collect(),
        ),
        None => (Vec::new(), Vec::new()),
    };
    let maybe: Json = match (&t.ootr, t.grade.as_deref()) {
        (Some(o), Some("LIKELY")) | (Some(o), Some("GUESS")) => o.as_str().into(),
        _ => Json::Null,
    };
    Json::obj()
        .with("id", t.id.as_str().into())
        .with("label", t.label().into())
        .with("name", t.name.as_str().into())
        .with("maybe", maybe)
        .with("kind", t.kind.as_str().into())
        .with("area", opt(&t.area))
        .with("scene", opt(&t.scene))
        .with("room", t.room.map(Json::Int).unwrap_or(Json::Null))
        .with("life", t.life.name().into())
        .with("how", how(t.how).into())
        .with("weak", t.weak.into())
        .with("dream", t.dream().into())
        .with("ages", Json::Arr(ages))
        .with("pins", Json::Arr(pins))
        .with("st", mark_json(&g.track.marks[i]))
}

fn area(t: &Target) -> String {
    t.area.clone().unwrap_or_else(|| "Elsewhere".to_string())
}

fn places(g: &Inner) -> Json {
    let mut m: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for (i, t) in ts().iter().enumerate() {
        if !kind_on(g, t) {
            continue;
        }
        let e: &mut (i64, i64) = m.entry(area(t)).or_insert((0, 0));
        e.1 += 1;
        if g.track.marks[i].s == S::Found {
            e.0 += 1;
        }
    }
    let all: bool = g.opts.mode == Mode::All;
    Json::Arr(
        m.into_iter()
            .filter(|(_, v)| all || v.0 > 0)
            .map(|(k, v)| {
                Json::obj()
                    .with("id", k.as_str().into())
                    .with("found", Json::Int(v.0))
                    .with("all", if all { Json::Int(v.1) } else { Json::Null })
            })
            .collect(),
    )
}

fn kinds_json(g: &Inner) -> Json {
    Json::Arr(
        KINDS
            .iter()
            .map(|k| {
                Json::obj()
                    .with("id", (*k).into())
                    .with("on", g.opts.kinds.iter().any(|x| x == k).into())
                    .with("default", (!OFF_BY_DEFAULT.contains(k)).into())
            })
            .collect(),
    )
}

pub fn source_json(g: &Inner) -> Json {
    let s = &g.src;
    let p = |x: &Option<std::path::PathBuf>| x.as_ref().map(|p| Json::from(pathsafe::text(p))).unwrap_or(Json::Null);
    let age: Json = match s.read_at {
        Some(t) => Json::Int(now_secs().saturating_sub(t) as i64),
        None => Json::Null,
    };
    Json::obj()
        .with("kind", "save".into())
        .with("asked", p(&s.asked))
        .with("folder", p(&s.folder))
        .with("folder_ok", s.folder.is_some().into())
        .with("slot", opt(&s.slot))
        .with("pinned", opt(&s.pinned))
        .with("slots", Json::strs(s.slots.iter().map(|x| x.as_str())))
        .with("read_at", s.read_at.map(|t| Json::Int(t as i64)).unwrap_or(Json::Null))
        .with("age_s", age)
        .with("keys", Json::Int(s.keys as i64))
        .with("reads", Json::Int(s.reads as i64))
        .with("error", opt(&s.error))
        .with("game_dir", p(&g.game_dir))
        .with("game", if g.index.game.is_empty() { Json::Null } else { g.index.game.as_str().into() })
        .with("job", g.job.map(|j| Json::Int(j as i64)).unwrap_or(Json::Null))
        .with("index_error", opt(&g.index_error))
}

fn player(g: &Inner) -> Json {
    let Some(p) = &g.place else {
        return Json::Null;
    };
    let mut uv: Json = Json::Null;
    if let (Some(m), Some(pos)) = (&p.map, p.pos) {
        let base: &str = m.strip_suffix("_Alternative").unwrap_or(m);
        let mi: Option<&MapInfo> = g.index.maps.iter().find(|x| x.id == base);
        if let Some(c) = mi.and_then(|x| x.cap) {
            let (u, v): (f32, f32) = project::uv(&c, pos[0], pos[2]);
            uv = Json::Arr(vec![Json::Num(u as f64), Json::Num(v as f64)]);
        }
    }
    Json::obj()
        .with("map", opt(&p.map))
        .with("child", p.child.into())
        .with("room", p.room.map(|r| Json::Int(r as i64)).unwrap_or(Json::Null))
        .with("uv", uv)
}

fn total(g: &Inner) -> Json {
    let mut all: i64 = 0;
    let mut found: i64 = 0;
    for (i, t) in ts().iter().enumerate() {
        if kind_on(g, t) {
            all += 1;
            if g.track.marks[i].s == S::Found {
                found += 1;
            }
        }
    }
    Json::obj().with("found", Json::Int(found)).with("all", Json::Int(all))
}

pub fn index_json(g: &Inner) -> Json {
    let locs: Vec<Json> = (0..ts().len()).filter(|i| visible(g, *i)).map(|i| loc_json(g, i)).collect();
    Json::obj()
        .with("format", Json::Int(1))
        .with("game", if g.index.game.is_empty() { Json::Null } else { g.index.game.as_str().into() })
        .with("ready", (!g.index.game.is_empty()).into())
        .with("mode", g.opts.mode.name().into())
        .with("maps", g.index.maps_json())
        .with("kinds", kinds_json(g))
        .with("locations", Json::Arr(locs))
}

fn seed_json(g: &Inner) -> Json {
    match &g.seeded {
        Some(s) => Json::obj()
            .with("loaded", true.into())
            .with("seed", s.seed.as_str().into())
            .with("checksum", s.checksum.as_str().into())
            .with("rewired", Json::Int(s.rewired as i64)),
        None => Json::obj().with("loaded", false.into()),
    }
}

pub fn state_json(g: &Inner, seq: u64) -> Json {
    let mut locs: Vec<(String, Json)> = Vec::new();
    for i in 0..ts().len() {
        if visible(g, i) {
            locs.push((ts()[i].id.clone(), mark_json(&g.track.marks[i])));
        }
    }
    Json::obj()
        .with("seq", Json::Int(seq as i64))
        .with("source", source_json(g))
        .with("player", player(g))
        .with("mode", g.opts.mode.name().into())
        .with("dream", g.opts.dream.into())
        .with("kinds", kinds_json(g))
        .with("seed", seed_json(g))
        .with("total", total(g))
        .with("places", places(g))
        .with("locations", Json::Obj(locs))
}

pub fn events_json(g: &Inner, b: &Batch) -> Json {
    let mut out: Vec<Json> = Vec::new();
    for (s, e) in &b.items {
        match e {
            Ev::Loc { i, fresh } => {
                if visible(g, *i) {
                    out.push(Json::obj().with("seq", Json::Int(*s as i64)).with("type", "loc".into()).with("loc", loc_json(g, *i)).with("fresh", (*fresh).into()));
                }
            }
            Ev::Source => out.push(
                Json::obj()
                    .with("seq", Json::Int(*s as i64))
                    .with("type", "source".into())
                    .with("source", source_json(g))
                    .with("player", player(g))
                    .with("total", total(g))
                    .with("places", places(g)),
            ),
            Ev::Reset => out.push(Json::obj().with("seq", Json::Int(*s as i64)).with("type", "reset".into())),
        }
    }
    Json::obj().with("seq", Json::Int(b.seq as i64)).with("reset", b.reset.into()).with("events", Json::Arr(out))
}
