use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::dungeon_grant::{Grants, Marks};
use crate::error::Res;
use crate::slice::edit::Edited;
use crate::slice::skull_edit::Drop;

pub struct State<'a> {
    pub silver: &'a BM<u64, &'static str>,
    pub silver_used: BS<&'static str>,
    pub silver_hit: BS<u64>,
    pub grants: &'a Grants,
    pub marks: Marks,
    pub skulls: &'a [Drop],
    pub skull_hit: BS<u64>,
    pub day: bool,
    pub day_woke: usize,
}

impl<'a> State<'a> {
    pub fn new(silver: &'a BM<u64, &'static str>, grants: &'a Grants, skulls: &'a [Drop]) -> State<'a> {
        State { silver, silver_used: BS::new(), silver_hit: BS::new(), grants, marks: Marks::default(), skulls, skull_hit: BS::new(), day: false, day_woke: 0 }
    }
}

pub type Pass = fn(&str, &[u8], Option<Edited>, &mut State) -> Res<Option<Edited>>;

pub const PASSES: [(&str, Pass); 4] = [("silver rupees", silver), ("dungeon item grants", grants), ("skulltula drops", skulls), ("skulltulas by day", day)];

pub type Check = fn(&State) -> Res<()>;

pub const CHECKS: [(&str, Check); 4] = [("every skulltula drop was placed", skulls_found), ("every dungeon item grant was marked", grants_marked), ("every silver rupee was placed", silver_found), ("night skulltulas were found", day_woke)];

pub fn run(file: &str, raw: &[u8], e: Option<Edited>, st: &mut State) -> Res<Option<Edited>> {
    let mut e: Option<Edited> = e;
    for (_, p) in PASSES {
        e = p(file, raw, e, st)?;
    }
    Ok(e)
}

pub fn finish(st: &State) -> Res<()> {
    for (_, c) in CHECKS {
        c(st)?;
    }
    Ok(())
}

fn silver(file: &str, raw: &[u8], e: Option<Edited>, st: &mut State) -> Res<Option<Edited>> {
    crate::slice::silver::apply(file, raw, e, st.silver, &mut st.silver_used, &mut st.silver_hit)
}

fn grants(file: &str, raw: &[u8], e: Option<Edited>, st: &mut State) -> Res<Option<Edited>> {
    crate::slice::dungeon_items::apply(file, raw, e, st.grants, &mut st.marks)
}

fn skulls(file: &str, raw: &[u8], e: Option<Edited>, st: &mut State) -> Res<Option<Edited>> {
    crate::slice::skull_edit::apply(file, raw, e, st.skulls, &mut st.skull_hit)
}

fn day(file: &str, raw: &[u8], e: Option<Edited>, st: &mut State) -> Res<Option<Edited>> {
    crate::slice::skull_day::apply(file, raw, e, st.day, &mut st.day_woke)
}

fn day_woke(st: &State) -> Res<()> {
    crate::slice::skull_day::all_woke(st.day, st.day_woke)
}

fn skulls_found(st: &State) -> Res<()> {
    crate::slice::skull_edit::all_found(st.skulls, &st.skull_hit)
}

fn grants_marked(st: &State) -> Res<()> {
    crate::dungeon_grant::all_marked(st.grants, &st.marks)
}

fn silver_found(st: &State) -> Res<()> {
    crate::slice::silver::all_found(st.silver, &st.silver_hit)
}
