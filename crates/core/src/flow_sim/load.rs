use std::collections::HashMap as Map;
use std::fs;
use std::path::PathBuf as PB;

use crate::bfevfl::{Bf, Flow};
use crate::door_fix;
use crate::error::Res;
use crate::event_chests::resolve;
use crate::romfs::Romfs;

pub trait Src {
    fn flow(&mut self, name: &str) -> Res<Option<Flow>>;
    fn event(&mut self, name: &str) -> Res<Option<(String, String)>>;
    fn cuts(&mut self, name: &str) -> Vec<String>;
}

pub struct Disk {
    rf: Romfs,
    over: Option<PB>,
    patches: Vec<String>,
    table: Option<Map<String, resolve::Row>>,
    cuts: Map<String, Vec<String>>,
}

impl Disk {
    pub fn new(rf: Romfs, over: Option<PB>, patches: Vec<String>) -> Disk {
        Disk { rf, over, patches, table: None, cuts: Map::new() }
    }

    pub fn romfs(&mut self) -> &mut Romfs {
        &mut self.rf
    }

    fn bytes(&mut self, name: &str) -> Res<Option<Vec<u8>>> {
        let rel: String = format!("Event/EventFlow/{name}.bfevfl.zs");
        if let Some(o) = &self.over {
            let p: PB = o.join(&rel);
            if p.exists() {
                let z: Vec<u8> = fs::read(&p)?;
                return Ok(Some(self.rf.zs.decompress(&z)?));
            }
        }
        if !self.rf.root.join(&rel).exists() {
            return Ok(None);
        }
        Ok(Some(self.rf.read_zs(&rel)?))
    }
}

impl Src for Disk {
    fn flow(&mut self, name: &str) -> Res<Option<Flow>> {
        let Some(raw) = self.bytes(name)? else { return Ok(None) };
        let mut b: Bf = Bf::parse(&raw)?;
        for p in self.patches.clone() {
            if p == "doorfix" && b.flows.iter().any(|f| f.name == door_fix::FLOW) {
                door_fix::patch_flow(&mut b)?;
            }
        }
        let mut cs: Vec<(f32, String)> = Vec::new();
        for t in &b.tls {
            cs.extend(t.cuts.iter().map(|c| (c.start, c.name.clone())));
        }
        cs.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.cuts.insert(name.to_string(), cs.into_iter().map(|c| c.1).collect());
        let i: usize = b.flows.iter().position(|f| f.name == name).unwrap_or(0);
        if i >= b.flows.len() {
            return Ok(None);
        }
        Ok(Some(b.flows.swap_remove(i)))
    }

    fn event(&mut self, name: &str) -> Res<Option<(String, String)>> {
        if self.table.is_none() {
            self.table = Some(resolve::table(&mut self.rf)?);
        }
        Ok(self.table.as_ref().and_then(|t| t.get(name)).map(|r| (r.stem.clone(), r.entry.clone())))
    }

    fn cuts(&mut self, name: &str) -> Vec<String> {
        self.cuts.get(name).cloned().unwrap_or_default()
    }
}

#[derive(Default)]
pub struct Mem {
    pub flows: Map<String, Flow>,
    pub events: Map<String, (String, String)>,
    pub cut_names: Map<String, Vec<String>>,
}

impl Src for Mem {
    fn flow(&mut self, name: &str) -> Res<Option<Flow>> {
        Ok(self.flows.get(name).cloned())
    }

    fn event(&mut self, name: &str) -> Res<Option<(String, String)>> {
        Ok(self.events.get(name).cloned())
    }

    fn cuts(&mut self, name: &str) -> Vec<String> {
        self.cut_names.get(name).cloned().unwrap_or_default()
    }
}
