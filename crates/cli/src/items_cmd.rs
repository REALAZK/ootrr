use std::fs;
use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::items::dump::{self, Dump};
use ootrr_core::items::{self, Counts, Report, Table};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn check(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let flows: &str = arg(a, 1, "missing flow_rewards_json")?;
    let t: Table = items::load()?;
    let mut d: Dump = dump::scan(P::new(root))?;
    d.flow = dump::flow_items(&fs::read_to_string(flows)?)?;
    let r: Report = items::verify(&t, &d);
    let c: Counts = t.counts();
    println!("items: {}  mapped: {}  logic only: {}  unmapped: {}", c.total, c.mapped, c.logic, c.unmapped);
    println!(
        "progression: {}  mapped: {}  logic only: {}  unmapped: {}",
        c.adv, c.adv_mapped, c.adv_logic, c.adv_unmapped
    );
    println!(
        "remake actors named: {}  in ActorInfo: {}  with a pack: {}  in PouchActorInfo: {}",
        r.actors, r.in_actor_info, r.in_packs, r.in_pouch
    );
    println!("quantities checked against BundleActorNum: {}  evidence claims checked: {}", r.qty_checked, r.claims);
    for n in &r.notes {
        println!("  note: {n}");
    }
    for i in &r.issues {
        println!("  issue: {i}");
    }
    if let Some(out) = a.get(2) {
        let u: Vec<String> = items::unclaimed(&t, &d);
        fs::write(out, items::render(&t, Some(&u)))?;
        println!("wrote {out} with {} unclaimed remake rows", u.len());
    }
    if r.issues.is_empty() {
        println!("every actor named in data/items.json exists in the dump and every claim matches");
        Ok(())
    } else {
        Err(Error::Msg(format!("{} issue(s)", r.issues.len())))
    }
}
