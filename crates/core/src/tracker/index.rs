use std::collections::{BTreeMap, HashMap as Map, HashSet as Set};
use std::fs;
use std::path::Path as P;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::json::{self, Json};
use crate::placement::hash_of;
use crate::romfs::Romfs;
use crate::settings::digest::sha256_hex;
use crate::tracker::project::{self, Cap, Floors};
use crate::tracker::targets::{self, Target};

pub const FORMAT: i64 = 2;
pub const GRID_W: usize = 100;
pub const GRID_H: usize = 72;
pub const CAPTURE: &str = "Game/MapCaptureInfo/MapCaptureInfo.game__MapCaptureInfo.bgyml";
pub const MENU: &str = "Game/MapMenuInfo/MapMenuInfo.game__ui__MapMenuInfo.bgyml";
pub const MAP_PARAM: &str = "Banc/BancBasedResource/MapParam.byml.zs";
const SUB_AREAS: [&str; 2] = ["LonLonRanch", "LakeHylia"];

#[derive(Debug, Clone, PartialEq)]
pub struct MapInfo {
    pub id: String,
    pub place: String,
    pub age: Option<String>,
    pub cap: Option<Cap>,
    pub floors: Option<Floors>,
    pub art: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    pub map: String,
    pub floor: Option<usize>,
    pub u: Option<f32>,
    pub v: Option<f32>,
    pub tray: bool,
    pub pos: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loc {
    pub i: usize,
    pub ages: Vec<String>,
    pub pins: Vec<Pin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub map: String,
    pub floor: Option<usize>,
    pub rows: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub game: String,
    pub table: u32,
    pub maps: Vec<MapInfo>,
    pub locs: Vec<Loc>,
    pub shapes: Vec<Shape>,
}

struct Menu {
    place: String,
    bancs: Vec<String>,
    arts: Vec<String>,
    cap: Option<Cap>,
}

type Spots = Vec<(String, [f32; 3])>;

fn num(n: &Node) -> Option<f32> {
    match n {
        Node::F32(v) => Some(*v),
        Node::F64(v) => Some(*v as f32),
        Node::I32(v) => Some(*v as f32),
        Node::U32(v) => Some(*v as f32),
        _ => None,
    }
}

fn arr<'a>(n: &'a Node, k: &str) -> &'a [Node] {
    n.get(k).and_then(|x| x.as_arr()).unwrap_or(&[])
}

fn vec3(n: Option<&Node>) -> Option<[f32; 3]> {
    let a: &[Node] = n?.as_arr()?;
    if a.len() != 3 {
        return None;
    }
    Some([num(&a[0])?, num(&a[1])?, num(&a[2])?])
}

fn strs(n: &Node, k: &str) -> Vec<String> {
    arr(n, k).iter().map(|x| x.as_str().unwrap_or("").to_string()).collect()
}

fn menus(rf: &mut Romfs) -> Res<Vec<Menu>> {
    let caps: Byml = rf.read_byml(CAPTURE)?;
    let menu: Byml = rf.read_byml(MENU)?;
    let cl: &[Node] = arr(&caps.root, "MapCaptureParamArray");
    let ml: &[Node] = arr(&menu.root, "MapMenuParamArray");
    if ml.is_empty() || cl.len() < ml.len() {
        return Err(Error::Msg("the map menu and capture tables of this game folder do not match".to_string()));
    }
    let mut out: Vec<Menu> = Vec::new();
    for (i, m) in ml.iter().enumerate() {
        let c: &Node = &cl[i];
        let cam: Option<&Node> = c.get("CameraPos");
        let x: Option<f32> = cam.and_then(|p| p.get("X")).and_then(num);
        let z: Option<f32> = cam.and_then(|p| p.get("Z")).and_then(num);
        let h: Option<f32> = c.get("Height").and_then(num).filter(|h| *h > 0.0);
        let cap: Option<Cap> = match (x, z, h) {
            (Some(x), Some(z), Some(h)) => Some(Cap { x, z, h }),
            _ => None,
        };
        out.push(Menu {
            place: m.get("LocationName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            bancs: strs(m, "BancName"),
            arts: strs(m, "TextureName"),
            cap,
        });
    }
    Ok(out)
}

struct Param {
    floors: Option<Floors>,
    rooms: Vec<Vec<i32>>,
}

fn params(rf: &mut Romfs) -> Res<Map<String, Param>> {
    let y: Byml = rf.read_byml(MAP_PARAM)?;
    let mut out: Map<String, Param> = Map::new();
    let Node::Map(m) = &y.root else {
        return Ok(out);
    };
    for (k, v) in m {
        let ys: Vec<f32> = arr(v, "Floor").iter().filter_map(|f| f.get("TransY").and_then(num)).collect();
        let min: i32 = match v.get("MinFloor") {
            Some(Node::I32(x)) => *x,
            _ => 0,
        };
        let rooms: Vec<Vec<i32>> = arr(v, "Room")
            .iter()
            .map(|r| arr(r, "Floor").iter().filter_map(|f| if let Node::I32(x) = f { Some(*x) } else { None }).collect())
            .collect();
        let floors: Option<Floors> = if ys.is_empty() { None } else { Some(Floors { min, ys }) };
        out.insert(k.clone(), Param { floors, rooms });
    }
    Ok(out)
}

fn map_of_file(f: &str) -> Option<String> {
    let n: &str = f.rsplit('/').next()?;
    let s: &str = n.strip_suffix(".bcett.byml.zs")?;
    let m: &str = s.strip_suffix("_Dynamic").or_else(|| s.strip_suffix("_Static"))?;
    Some(m.to_string())
}

type Found = (Map<(String, u64), Spots>, Map<(String, String), Spots>, Spots);

fn spots(rf: &mut Romfs, ts: &[Target]) -> Res<Found> {
    let mut want_h: Map<String, Set<u64>> = Map::new();
    let mut want_g: Map<String, Set<String>> = Map::new();
    for t in ts {
        let Some(sc) = t.scene.clone() else {
            continue;
        };
        if let Some(h) = t.hash() {
            want_h.entry(sc.clone()).or_default().insert(h);
        }
        if let Some(g) = t.keeper() {
            want_g.entry(sc).or_default().insert(g.to_string());
        }
    }
    let mut scenes: Vec<String> = want_h.keys().chain(want_g.keys()).cloned().collect();
    scenes.sort();
    scenes.dedup();
    let mut by_h: Map<(String, u64), Spots> = Map::new();
    let mut by_g: Map<(String, String), Spots> = Map::new();
    let mut every: Spots = Vec::new();
    let none_h: Set<u64> = Set::new();
    let none_g: Set<String> = Set::new();
    for sc in scenes {
        let files: Vec<String> = match rf.list(&format!("Banc/{sc}")) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let wh: &Set<u64> = want_h.get(&sc).unwrap_or(&none_h);
        let wg: &Set<String> = want_g.get(&sc).unwrap_or(&none_g);
        for f in files {
            let Some(map) = map_of_file(&f) else {
                continue;
            };
            let y: Byml = rf.read_byml(&f)?;
            for a in arr(&y.root, "Actors") {
                let Some(pos) = vec3(a.get("Translate")) else {
                    continue;
                };
                every.push((map.clone(), pos));
                if let Some(h) = hash_of(a)
                    && wh.contains(&h)
                {
                    let e: &mut Spots = by_h.entry((sc.clone(), h)).or_default();
                    if !e.iter().any(|x| x.0 == map) {
                        e.push((map.clone(), pos));
                    }
                }
                if let Some(g) = a.get("Gyaml").and_then(|v| v.as_str())
                    && wg.contains(g)
                {
                    let e: &mut Spots = by_g.entry((sc.clone(), g.to_string())).or_default();
                    if !e.iter().any(|x| x.0 == map) {
                        e.push((map.clone(), pos));
                    }
                }
            }
        }
    }
    Ok((by_h, by_g, every))
}

fn hex_rows(cells: &[bool]) -> Vec<String> {
    cells
        .chunks(GRID_W)
        .map(|r| {
            r.chunks(4)
                .map(|q| {
                    let n: u32 = q.iter().enumerate().map(|(i, b)| if *b { 8 >> i } else { 0 }).sum();
                    char::from_digit(n, 16).unwrap_or('0')
                })
                .collect()
        })
        .collect()
}

fn shapes_of(cx: &Ctx, every: &Spots) -> Vec<Shape> {
    let mut grids: BTreeMap<(String, Option<usize>), Vec<bool>> = BTreeMap::new();
    for (m, pos) in every {
        let p: Pin = pin(cx, m, *pos, None);
        let (Some(u), Some(v)) = (p.u, p.v) else {
            continue;
        };
        if p.tray {
            continue;
        }
        let x: usize = ((u * GRID_W as f32) as usize).min(GRID_W - 1);
        let y: usize = ((v * GRID_H as f32) as usize).min(GRID_H - 1);
        let g: &mut Vec<bool> = grids.entry((p.map, p.floor)).or_insert_with(|| vec![false; GRID_W * GRID_H]);
        g[y * GRID_W + x] = true;
    }
    grids.into_iter().map(|((map, floor), g)| Shape { map, floor, rows: hex_rows(&g) }).collect()
}

fn age_of(map: &str) -> Option<&'static str> {
    if map.ends_with("_Child") {
        Some("child")
    } else if map.ends_with("_Adult") || map.ends_with("_Adult_Alternative") {
        Some("adult")
    } else {
        None
    }
}

fn base(map: &str) -> &str {
    map.strip_suffix("_Alternative").unwrap_or(map)
}

struct Ctx {
    caps: Map<String, Cap>,
    params: Map<String, Param>,
}

fn pin(cx: &Ctx, map: &str, pos: [f32; 3], room: Option<i64>) -> Pin {
    let b: &str = base(map);
    let mut id: String = b.to_string();
    let mut uv: Option<(f32, f32)> = None;
    if let Some(rest) = b.strip_prefix("HyruleField") {
        for s in SUB_AREAS {
            let sub: String = format!("{s}{rest}");
            if let Some(c) = cx.caps.get(&sub) {
                let (u, v): (f32, f32) = project::uv(c, pos[0], pos[2]);
                if project::inside(u, v) {
                    id = sub;
                    uv = Some((u, v));
                    break;
                }
            }
        }
    }
    if uv.is_none()
        && let Some(c) = cx.caps.get(b)
    {
        uv = Some(project::uv(c, pos[0], pos[2]));
    }
    let tray: bool = uv.map(|(u, v)| !project::inside(u, v)).unwrap_or(false);
    let floor: Option<usize> = cx.params.get(b).and_then(|p| {
        let fl: &Floors = p.floors.as_ref()?;
        let rf: &[i32] = room
            .and_then(|r| usize::try_from(r).ok())
            .and_then(|r| p.rooms.get(r))
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        project::floor_of(fl, pos[1], rf)
    });
    Pin { map: id, floor, u: uv.map(|x| x.0), v: uv.map(|x| x.1), tray, pos }
}

pub fn fingerprint(root: &P) -> Res<String> {
    let rf: Romfs = Romfs::open(root)?;
    let mut buf: Vec<u8> = Vec::new();
    for f in rf.list("Banc")? {
        let n: u64 = fs::metadata(root.join(&f))?.len();
        buf.extend_from_slice(f.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&n.to_le_bytes());
    }
    for f in [CAPTURE, MENU, MAP_PARAM] {
        buf.extend_from_slice(&rf.read(f)?);
    }
    let h: String = sha256_hex(&buf);
    Ok(format!("oot-remake:{}", &h[..16]))
}

pub fn build(root: &P) -> Res<Index> {
    let ts: &[Target] = targets::all()?;
    let game: String = fingerprint(root)?;
    let mut rf: Romfs = Romfs::open(root)?;
    let ms: Vec<Menu> = menus(&mut rf)?;
    let ps: Map<String, Param> = params(&mut rf)?;
    let mut maps: Vec<MapInfo> = Vec::new();
    let mut caps: Map<String, Cap> = Map::new();
    for m in &ms {
        for (i, b) in m.bancs.iter().enumerate() {
            if b.is_empty() || caps.contains_key(b) {
                continue;
            }
            if let Some(c) = m.cap {
                caps.insert(b.clone(), c);
            }
            let age: Option<String> = if m.bancs.iter().all(|x| x == b) {
                None
            } else {
                Some((if i == 0 { "child" } else { "adult" }).to_string())
            };
            maps.push(MapInfo {
                id: b.clone(),
                place: m.place.clone(),
                age,
                cap: m.cap,
                floors: ps.get(b).and_then(|p| p.floors.clone()),
                art: m.arts.get(i).filter(|a| !a.is_empty()).cloned(),
            });
        }
    }
    let (by_h, by_g, every): Found = spots(&mut rf, ts)?;
    let cx: Ctx = Ctx { caps, params: ps };
    let mut locs: Vec<Loc> = Vec::with_capacity(ts.len());
    let mut seen: Set<String> = maps.iter().map(|m| m.id.clone()).collect();
    for (i, t) in ts.iter().enumerate() {
        let sc: String = t.scene.clone().unwrap_or_default();
        let found: Option<&Spots> = match (t.hash(), t.keeper()) {
            (Some(h), _) => by_h.get(&(sc.clone(), h)),
            (None, Some(g)) => by_g.get(&(sc.clone(), g.to_string())),
            _ => None,
        };
        let mut sp: Spots = found.cloned().unwrap_or_default();
        if let Some(pm) = &t.map {
            sp.sort_by_key(|x| x.0 != *pm);
        }
        let mut ages: Vec<String> = Vec::new();
        let mut pins: Vec<Pin> = Vec::new();
        for (m, pos) in &sp {
            match age_of(m) {
                Some(a) => ages.push(a.to_string()),
                None => {
                    ages.push("child".to_string());
                    ages.push("adult".to_string());
                }
            }
            let p: Pin = pin(&cx, m, *pos, t.room);
            if seen.insert(p.map.clone()) {
                maps.push(MapInfo {
                    id: p.map.clone(),
                    place: sc.clone(),
                    age: age_of(&p.map).map(|a| a.to_string()),
                    cap: None,
                    floors: cx.params.get(&p.map).and_then(|x| x.floors.clone()),
                    art: None,
                });
            }
            pins.push(p);
        }
        ages.sort();
        ages.dedup();
        locs.push(Loc { i, ages, pins });
    }
    let shapes: Vec<Shape> = shapes_of(&cx, &every);
    Ok(Index { game, table: targets::revision(), maps, locs, shapes })
}

fn f(x: f32) -> Json {
    Json::Num(((x as f64) * 10000.0).round() / 10000.0)
}

fn fo(x: Option<f32>) -> Json {
    x.map(f).unwrap_or(Json::Null)
}

fn shape_json(s: &Shape) -> Json {
    Json::obj()
        .with("floor", s.floor.map(|x| Json::Int(x as i64)).unwrap_or(Json::Null))
        .with("rows", Json::strs(s.rows.iter().map(|r| r.as_str())))
}

fn floors_json(fl: &Option<Floors>) -> Json {
    match fl {
        None => Json::Arr(Vec::new()),
        Some(fl) => Json::Arr(
            fl.ys
                .iter()
                .enumerate()
                .map(|(i, y)| Json::obj().with("label", project::label(fl.min, i).into()).with("y", f(*y)))
                .collect(),
        ),
    }
}

pub fn pin_json(p: &Pin, fl: Option<&Floors>) -> Json {
    let lab: Json = match (p.floor, fl) {
        (Some(i), Some(fl)) => project::label(fl.min, i).into(),
        _ => Json::Null,
    };
    Json::obj()
        .with("map", p.map.as_str().into())
        .with("floor", p.floor.map(|x| Json::Int(x as i64)).unwrap_or(Json::Null))
        .with("floor_label", lab)
        .with("u", fo(p.u))
        .with("v", fo(p.v))
        .with("tray", p.tray.into())
}

impl Index {
    pub fn floors_of(&self, map: &str) -> Option<&Floors> {
        self.maps.iter().find(|m| m.id == map).and_then(|m| m.floors.as_ref())
    }

    pub fn maps_json(&self) -> Json {
        Json::Arr(
            self.maps
                .iter()
                .map(|m| {
                    Json::obj()
                        .with("id", m.id.as_str().into())
                        .with("place", m.place.as_str().into())
                        .with("age", m.age.as_deref().map(Json::from).unwrap_or(Json::Null))
                        .with("aspect", f(project::ASPECT))
                        .with("cam", m.cap.map(|c| Json::Arr(vec![f(c.x), f(c.z)])).unwrap_or(Json::Null))
                        .with("width", m.cap.map(|c| f(c.h)).unwrap_or(Json::Null))
                        .with("art", m.art.as_deref().map(Json::from).unwrap_or(Json::Null))
                        .with("floors", floors_json(&m.floors))
                        .with("shapes", Json::Arr(self.shapes.iter().filter(|s| s.map == m.id).map(shape_json).collect()))
                })
                .collect(),
        )
    }

    pub fn to_json(&self) -> Json {
        let ts: &[Target] = targets::all().unwrap_or(&[]);
        let locs: Vec<Json> = self
            .locs
            .iter()
            .filter_map(|l| {
                let t: &Target = ts.get(l.i)?;
                let pins: Vec<Json> = l
                    .pins
                    .iter()
                    .map(|p| {
                        pin_json(p, self.floors_of(&p.map))
                            .with("pos", Json::Arr(p.pos.iter().map(|x| f(*x)).collect()))
                    })
                    .collect();
                Some(
                    Json::obj()
                        .with("id", t.id.as_str().into())
                        .with("ages", Json::strs(l.ages.iter().map(|s| s.as_str())))
                        .with("pins", Json::Arr(pins)),
                )
            })
            .collect();
        let maps: Vec<Json> = self
            .maps
            .iter()
            .map(|m| {
                let fl: Json = match &m.floors {
                    Some(x) => Json::obj().with("min", Json::Int(x.min as i64)).with("ys", Json::Arr(x.ys.iter().map(|y| f(*y)).collect())),
                    None => Json::Null,
                };
                Json::obj()
                    .with("id", m.id.as_str().into())
                    .with("place", m.place.as_str().into())
                    .with("age", m.age.as_deref().map(Json::from).unwrap_or(Json::Null))
                    .with("cap", m.cap.map(|c| Json::Arr(vec![f(c.x), f(c.z), f(c.h)])).unwrap_or(Json::Null))
                    .with("floors", fl)
                    .with("art", m.art.as_deref().map(Json::from).unwrap_or(Json::Null))
            })
            .collect();
        Json::obj()
            .with("format", Json::Int(FORMAT))
            .with("game", self.game.as_str().into())
            .with("table", Json::Int(self.table as i64))
            .with("maps", Json::Arr(maps))
            .with("shapes", Json::Arr(self.shapes.iter().map(|s| shape_json(s).with("map", s.map.as_str().into())).collect()))
            .with("locations", Json::Arr(locs))
    }
}

fn bad(m: &str) -> Error {
    Error::Msg(format!("tracker index cache: {m}"))
}

fn jf(j: &Json) -> Option<f32> {
    j.as_f64().map(|x| x as f32)
}

fn read_pin(j: &Json) -> Res<Pin> {
    let pos: Vec<f32> = j.get("pos").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().filter_map(jf).collect();
    if pos.len() != 3 {
        return Err(bad("a pin has no position"));
    }
    Ok(Pin {
        map: j.get("map").and_then(|x| x.as_str()).ok_or_else(|| bad("a pin has no map"))?.to_string(),
        floor: j.get("floor").and_then(|x| x.as_i64()).and_then(|x| usize::try_from(x).ok()),
        u: j.get("u").and_then(jf),
        v: j.get("v").and_then(jf),
        tray: j.get("tray").and_then(|x| x.as_bool()).unwrap_or(false),
        pos: [pos[0], pos[1], pos[2]],
    })
}

fn read_map(j: &Json) -> Res<MapInfo> {
    let s = |k: &str| j.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
    let cap: Option<Cap> = match j.get("cap").and_then(|x| x.as_arr()) {
        Some(a) if a.len() == 3 => Some(Cap { x: jf(&a[0]).unwrap_or(0.0), z: jf(&a[1]).unwrap_or(0.0), h: jf(&a[2]).unwrap_or(1.0) }),
        _ => None,
    };
    let floors: Option<Floors> = j.get("floors").and_then(|x| {
        let min: i32 = x.get("min")?.as_i64()? as i32;
        let ys: Vec<f32> = x.get("ys")?.as_arr()?.iter().filter_map(jf).collect();
        Some(Floors { min, ys })
    });
    Ok(MapInfo {
        id: s("id").ok_or_else(|| bad("a map has no id"))?,
        place: s("place").unwrap_or_default(),
        age: s("age"),
        cap,
        floors,
        art: s("art"),
    })
}

fn read_shape(j: &Json) -> Res<Shape> {
    let rows: Vec<String> = j.get("rows").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
    if rows.len() != GRID_H || rows.iter().any(|r| r.len() != GRID_W / 4 || !r.bytes().all(|b| b.is_ascii_hexdigit())) {
        return Err(bad("a map shape has the wrong size"));
    }
    Ok(Shape {
        map: j.get("map").and_then(|x| x.as_str()).ok_or_else(|| bad("a map shape has no map"))?.to_string(),
        floor: j.get("floor").and_then(|x| x.as_i64()).and_then(|x| usize::try_from(x).ok()),
        rows,
    })
}

pub fn from_json(text: &str) -> Res<Index> {
    let j: Json = json::parse(text)?;
    if j.get("format").and_then(|x| x.as_i64()) != Some(FORMAT) {
        return Err(bad("unknown format"));
    }
    if j.get("table").and_then(|x| x.as_i64()) != Some(targets::revision() as i64) {
        return Err(bad("made for another location table"));
    }
    let ts: &[Target] = targets::all()?;
    let pos: Map<&str, usize> = ts.iter().enumerate().map(|(i, t)| (t.id.as_str(), i)).collect();
    let mut locs: Vec<Loc> = Vec::new();
    for l in j.get("locations").and_then(|x| x.as_arr()).unwrap_or(&[]) {
        let id: &str = l.get("id").and_then(|x| x.as_str()).ok_or_else(|| bad("a location has no id"))?;
        let i: usize = *pos.get(id).ok_or_else(|| bad("a location is not in the table"))?;
        let ages: Vec<String> = l.get("ages").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
        let pins: Vec<Pin> = l.get("pins").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().map(read_pin).collect::<Res<Vec<Pin>>>()?;
        locs.push(Loc { i, ages, pins });
    }
    if locs.len() != ts.len() {
        return Err(bad("the location count does not match the table"));
    }
    let maps: Vec<MapInfo> = j.get("maps").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().map(read_map).collect::<Res<Vec<MapInfo>>>()?;
    let shapes: Vec<Shape> = j.get("shapes").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().map(read_shape).collect::<Res<Vec<Shape>>>()?;
    Ok(Index {
        game: j.get("game").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        table: targets::revision(),
        maps,
        locs,
        shapes,
    })
}

pub fn empty() -> Res<Index> {
    let ts: &[Target] = targets::all()?;
    Ok(Index {
        game: String::new(),
        table: targets::revision(),
        maps: Vec::new(),
        locs: (0..ts.len()).map(|i| Loc { i, ages: Vec::new(), pins: Vec::new() }).collect(),
        shapes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    fn ctx() -> Ctx {
        let mut caps: Map<String, Cap> = Map::new();
        caps.insert("HyruleField_Child".to_string(), Cap { x: 0.0, z: 0.0, h: 1000.0 });
        caps.insert("LakeHylia_Child".to_string(), Cap { x: 300.0, z: 300.0, h: 100.0 });
        caps.insert("KokiriForest_Child".to_string(), Cap { x: 0.0, z: 0.0, h: 100.0 });
        let mut params: Map<String, Param> = Map::new();
        params.insert("FireTemple".to_string(), Param { floors: Some(Floors { min: -1, ys: vec![0.0, 10.0] }), rooms: vec![vec![-1], vec![0]] });
        Ctx { caps, params }
    }

    #[test]
    fn file_names_give_map_names() {
        assert_eq!(map_of_file("Banc/KokiriForest/KokiriForest_Child_Dynamic.bcett.byml.zs").as_deref(), Some("KokiriForest_Child"));
        assert_eq!(map_of_file("Banc/FireTemple/FireTemple_Static.bcett.byml.zs").as_deref(), Some("FireTemple"));
        assert_eq!(map_of_file("Banc/X/readme.txt"), None);
        assert_eq!(age_of("KakarikoVillage_Adult_Alternative"), Some("adult"));
        assert_eq!(age_of("FireTemple"), None);
    }

    #[test]
    fn hyrule_field_points_inside_a_sub_area_move_there() {
        let c: Ctx = ctx();
        let lake: Pin = pin(&c, "HyruleField_Child", [300.0, 0.0, 300.0], None);
        assert_eq!(lake.map, "LakeHylia_Child");
        assert!((lake.u.unwrap() - 0.5).abs() < 1e-6);
        let field: Pin = pin(&c, "HyruleField_Child", [0.0, 0.0, 0.0], None);
        assert_eq!(field.map, "HyruleField_Child");
        assert!(!field.tray);
        let far: Pin = pin(&c, "KokiriForest_Child", [500.0, 0.0, 0.0], None);
        assert!(far.tray);
        let none: Pin = pin(&c, "DreamRoom", [1.0, 2.0, 3.0], None);
        assert_eq!((none.u, none.tray, none.floor), (None, false, None));
    }

    #[test]
    fn dungeon_pins_take_the_room_floor() {
        let c: Ctx = ctx();
        assert_eq!(pin(&c, "FireTemple", [0.0, 12.0, 0.0], None).floor, Some(1));
        assert_eq!(pin(&c, "FireTemple", [0.0, 12.0, 0.0], Some(0)).floor, Some(0));
        assert_eq!(pin(&c, "FireTemple", [0.0, 2.0, 0.0], Some(1)).floor, Some(1));
    }

    #[test]
    fn actor_positions_fill_the_shape_grid_of_their_floor() {
        let c: Ctx = ctx();
        let every: Spots = vec![
            ("KokiriForest_Child".to_string(), [0.0, 0.0, 0.0]),
            ("KokiriForest_Child".to_string(), [900.0, 0.0, 0.0]),
            ("DreamRoom".to_string(), [0.0, 0.0, 0.0]),
        ];
        let s: Vec<Shape> = shapes_of(&c, &every);
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].map.as_str(), s[0].floor), ("KokiriForest_Child", None));
        assert_eq!(s[0].rows.len(), GRID_H);
        let on: usize = s[0].rows.iter().map(|r| r.chars().filter(|c| *c != '0').count()).sum();
        assert_eq!(on, 1);
        assert_eq!(s[0].rows[GRID_H / 2].chars().nth(GRID_W / 8), Some('2'));
    }

    #[test]
    fn the_cache_round_trips() {
        let mut ix: Index = empty().unwrap();
        ix.game = "oot-remake:0123456789abcdef".to_string();
        ix.maps.push(MapInfo { id: "FireTemple".into(), place: "FireTemple".into(), age: None, cap: Some(Cap { x: 1.0, z: 2.0, h: 3.0 }), floors: Some(Floors { min: -1, ys: vec![0.5, 9.25] }), art: None });
        ix.locs[0].ages = vec!["child".into()];
        ix.locs[0].pins.push(Pin { map: "FireTemple".into(), floor: Some(1), u: Some(0.25), v: Some(0.75), tray: false, pos: [1.5, 2.5, 3.5] });
        let mut cells: Vec<bool> = vec![false; GRID_W * GRID_H];
        cells[0] = true;
        cells[GRID_W + 7] = true;
        ix.shapes.push(Shape { map: "FireTemple".into(), floor: Some(1), rows: hex_rows(&cells) });
        assert_eq!(&ix.shapes[0].rows[0][..2], "80");
        assert_eq!(&ix.shapes[0].rows[1][..2], "01");
        assert!(ix.maps_json().to_string().contains("\"shapes\":[{\"floor\":1,"));
        let back: Index = from_json(&ix.to_json().to_string()).unwrap();
        assert_eq!(back, ix);
        let wrong: String = ix.to_json().with("table", Json::Int(1)).to_string();
        assert!(from_json(&wrong).is_err());
    }

    #[test]
    fn builds_from_a_real_dump_when_one_is_given() {
        let Ok(r) = std::env::var("OOTRR_TEST_ROMFS") else {
            return;
        };
        let ix: Index = build(&PB::from(r)).unwrap();
        let with: usize = ix.locs.iter().filter(|l| !l.pins.is_empty()).count();
        assert!(with >= 1200, "{with}");
        assert!(ix.game.starts_with("oot-remake:"));
    }
}
