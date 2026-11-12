use std::collections::BTreeMap as BM;

use crate::bfevfl::{Bf, Flow};
use crate::byml::Byml;
use crate::error::{Error as E, Res};
use crate::flow_locations::{dict_id, path_of, StagedFile};
use crate::progressive::{add_rows, ANALYZED_TABLE, SETTING_TABLE};
use crate::romfs::Romfs;
use crate::scrubs_b::banc::point;
use crate::scrubs_b::data::{book, Book, Scrub};
use crate::scrubs_b::flow::{edit, Sale};
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::edit::Edited;
use crate::slice::flow_pool::dest_form;
use crate::staging::{rewrite, Out};
use crate::zs;

#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub scrub: Scrub,
    pub item: String,
    pub sale: Sale,
}

pub fn jobs(d: &Data, seed: &SeedFile) -> Res<Vec<Job>> {
    let b: &Book = book()?;
    let mut out: Vec<Job> = Vec::new();
    for (loc, item) in &seed.placements {
        let Some(l) = d.flow.by_name(loc) else {
            continue;
        };
        if !crate::scrubs_b::is_b(l) {
            continue;
        }
        let s: &Scrub = b.by_location(loc).ok_or_else(|| E::Msg(format!("{loc} has no scrub entry in data/scrubs_b.json")))?;
        if d.vanilla_of(loc) == Some(item.as_str()) {
            continue;
        }
        let i = d.item(item).ok_or_else(|| E::Msg(format!("unknown item {item}")))?;
        if !crate::scrubs_b::fits(d, loc, i) || d.family(item).is_some() {
            return Err(E::Msg(format!("{loc} cannot sell {item}")));
        }
        let a: String = i.actor.clone().ok_or_else(|| E::Msg(format!("{item} has no remake item actor and cannot be placed")))?;
        let actor: String = dest_form(d, item, loc, a);
        out.push(Job { scrub: s.clone(), item: item.clone(), sale: Sale { entry: s.entry(), flag: s.flag.clone(), actor, price: s.price } });
    }
    out.sort_by(|x, y| x.scrub.key.cmp(&y.scrub.key));
    Ok(out)
}

pub fn banc(rf: &mut Romfs, jobs: &[Job], edits: &mut Vec<Edited>) -> Res<usize> {
    let mut by: BM<&str, Vec<(&crate::scrubs_b::Spot, String)>> = BM::new();
    for j in jobs {
        for sp in &j.scrub.spots {
            by.entry(sp.file.as_str()).or_default().push((sp, j.sale.entry.clone()));
        }
    }
    let mut n: usize = 0;
    for (f, list) in by {
        let at: Option<usize> = edits.iter().position(|e| e.file == f);
        let (raw, old): (Vec<u8>, usize) = match at {
            Some(i) => (edits[i].bytes.clone(), edits[i].old_len),
            None => {
                let b: Vec<u8> = rf.read_zs(f)?;
                let l: usize = b.len();
                (b, l)
            }
        };
        let mut y: Byml = Byml::parse(&raw)?;
        for (sp, entry) in list {
            if !point(&mut y, sp, &entry)? {
                return Err(E::Msg(format!("scrub {} was not found in {f}, the game files differ", sp.hash)));
            }
            n += 1;
        }
        let bytes: Vec<u8> = y.write()?;
        match at {
            Some(i) => edits[i].bytes = bytes,
            None => edits.push(Edited {
                file: f.to_string(),
                old_len: old,
                bytes,
                hits: Vec::new(),
                used: Vec::new(),
                hearts: Vec::new(),
                wraps: Vec::new(),
                touched: Vec::new(),
            }),
        }
    }
    Ok(n)
}

fn edit_flow(raw: &[u8], jobs: &[Job], label: &str) -> Res<Vec<u8>> {
    let mut bf: Bf = Bf::parse(raw)?;
    let f: &mut Flow = bf.flows.first_mut().ok_or(E::Format("scrubs_b: the scrub flow file holds no flow"))?;
    let sales: Vec<Sale> = jobs.iter().map(|j| j.sale.clone()).collect();
    edit(f, &sales, label)?;
    bf.write()
}

pub fn flow(rf: &mut Romfs, jobs: &[Job], files: &mut Vec<StagedFile>) -> Res<()> {
    if jobs.is_empty() {
        return Ok(());
    }
    let b: &Book = book()?;
    let path: String = path_of(&b.flow);
    match files.iter().position(|x| x.path == path) {
        Some(i) => {
            let id: Option<u32> = dict_id(&files[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&files[i].z)?;
            let out: Vec<u8> = edit_flow(&raw, jobs, &b.label)?;
            files[i].new = out.len();
            files[i].z = zs::compress_raw(&out, id)?;
        }
        None => {
            let o: Out = rewrite(rf, &path, |raw: &[u8]| edit_flow(raw, jobs, &b.label))?;
            files.push(StagedFile { path: o.path, z: o.z, old: o.old, new: o.new });
        }
    }
    Ok(())
}

fn add(raw: &[u8], template: &str, rows: &[(String, Option<String>)]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    add_rows(&mut y, template, rows)?;
    y.write()
}

pub fn rows(rf: &mut Romfs, jobs: &[Job], outs: &mut Vec<Out>, missing: bool) -> Res<usize> {
    if jobs.is_empty() {
        return Ok(0);
    }
    let b: &Book = book()?;
    let mut n: usize = 0;
    for table in [SETTING_TABLE, ANALYZED_TABLE] {
        let entry = |j: &Job| if table == SETTING_TABLE { Some(j.sale.entry.clone()) } else { None };
        let rs: Vec<(String, Option<String>)> = jobs.iter().map(|j| (j.sale.entry.clone(), entry(j))).collect();
        match outs.iter().position(|o| o.path == table) {
            Some(i) => {
                let id: Option<u32> = dict_id(&outs[i].z);
                let raw: Vec<u8> = rf.zs.decompress(&outs[i].z)?;
                let out: Vec<u8> = add(&raw, &b.template, &rs)?;
                outs[i].new = out.len();
                outs[i].z = zs::compress_raw(&out, id)?;
                n += 1;
            }
            None if missing => {
                outs.push(rewrite(rf, table, |raw: &[u8]| add(raw, &b.template, &rs))?);
                n += 1;
            }
            None => {}
        }
    }
    Ok(n)
}

pub fn notes(jobs: &[Job]) -> Vec<String> {
    if jobs.is_empty() {
        return Vec::new();
    }
    let mut v: Vec<String> = vec![format!("{} Business Scrubs became one time sales with a new event row each, none of these edits is tested in the game yet:", jobs.len())];
    for j in jobs {
        v.push(format!("  {} | {} | {} for {} rupees | sold flag {}", j.scrub.location, j.sale.entry, j.sale.actor, j.sale.price, j.sale.flag));
    }
    v
}
