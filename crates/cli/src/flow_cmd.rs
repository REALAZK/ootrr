use std::collections::HashSet as HS;
use std::fs;
use std::path::Path as P;

use ootrr_core::bfevfl::{Bf, Cont, Event, Val};
use ootrr_core::error::{Error, Res};
use ootrr_core::flow_locations::{
    apply, check_list, coverage, markdown, path_of, plan, verify, Applied, Change, Checked, Choice, Data, Diff, Edit, Loc,
    Opts,
};
use ootrr_core::json::{self, Json};
use ootrr_core::romfs::Romfs;

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

fn choice(s: &str) -> Res<Choice> {
    let (l, i): (&str, &str) = s
        .rsplit_once('=')
        .ok_or_else(|| Error::Msg(format!("{s} is not location=Item_Actor")))?;
    Ok(Choice { location: l.trim().to_string(), item: i.trim().to_string() })
}

fn parse(a: &[String]) -> Res<(Vec<Choice>, Opts)> {
    let mut cs: Vec<Choice> = Vec::new();
    let mut o: Opts = Opts::default();
    for s in a {
        if s == "--untested" {
            o.untested = true;
        } else {
            cs.push(choice(s)?);
        }
    }
    Ok((cs, o))
}

pub fn report(a: &[String]) -> Res<()> {
    let d: Data = Data::load()?;
    print!("{}", coverage(&d));
    if let Some(i) = a.iter().position(|s| s == "--md") {
        let p: &str = arg(a, i + 1, "--md needs an output file")?;
        fs::write(p, markdown(&d))?;
        println!("wrote {p}");
    }
    if a.iter().any(|s| s == "--tests") {
        println!("In game test list, one supported location per category:");
        for l in check_list(&d) {
            list_line(l);
        }
    }
    if a.iter().any(|s| s == "--all") {
        for l in &d.locs {
            let tag: &str = if l.supported { "supported" } else { "unsupported" };
            println!("{} | {} | {} | {} | {} | {}", l.name, l.cat, l.conf, tag, l.vanilla, l.why);
        }
        for g in &d.gaps {
            println!("{} | unmatched | {}", g.name, g.reason);
        }
    }
    Ok(())
}

fn list_line(l: &Loc) {
    let e: &Edit = &l.edits[0];
    println!(
        "  {} ({}) | {} event {} {} | {} | {} edit(s)",
        l.name,
        l.cat,
        e.file,
        e.event,
        e.index,
        e.item,
        l.edits.len()
    );
}

fn flag_names(p: &P) -> Res<HS<String>> {
    let j: Json = json::parse(&fs::read_to_string(p)?)?;
    let mut out: HS<String> = HS::new();
    for f in j.get("flags").and_then(|v| v.as_arr()).unwrap_or(&[]) {
        if let Some(n) = f.get("name").and_then(|v| v.as_str()) {
            out.insert(n.to_string());
        }
    }
    Ok(out)
}

fn current(b: &Bf, index: usize, param: &str) -> Option<(String, String)> {
    let e: &Event = b.flows.first()?.events.get(index)?;
    let c: &Cont = e.kind.params()?;
    match c.get(param)? {
        Val::Str(s) | Val::Actor(s, _) => Some((e.name.clone(), s.clone())),
        _ => None,
    }
}

pub fn check(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romvroot")?;
    let d: Data = Data::load()?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let mut n: usize = 0;
    let mut bad: Vec<String> = Vec::new();
    let mut files: HS<String> = HS::new();
    for l in &d.locs {
        for e in &l.edits {
            let mut v: Vec<&str> = vec![e.file.as_str()];
            v.extend(e.also.iter().map(|s| s.as_str()));
            for f in v {
                n += 1;
                files.insert(f.to_string());
                let raw: Vec<u8> = match rf.read_zs(&path_of(f)) {
                    Ok(r) => r,
                    Err(x) => {
                        bad.push(format!("{} {f}: {x}", l.name));
                        continue;
                    }
                };
                let b: Bf = match Bf::parse(&raw) {
                    Ok(b) => b,
                    Err(x) => {
                        bad.push(format!("{} {f}: {x}", l.name));
                        continue;
                    }
                };
                match current(&b, e.index, &e.param) {
                    Some((en, it)) if en == e.event && it == e.item => {}
                    other => bad.push(format!(
                        "{} {f} event {}: expected {} {} {}, found {:?}",
                        l.name, e.index, e.event, e.param, e.item, other
                    )),
                }
            }
        }
    }
    println!("{} locations, {} edit sites in {} files, {} do not match the dump", d.locs.len(), n, files.len(), bad.len());
    for x in bad.iter().take(20) {
        println!("  {x}");
    }
    if let Some(p) = a.get(1) {
        let names: HS<String> = flag_names(P::new(p))?;
        let mut known: usize = 0;
        let mut missing: Vec<String> = Vec::new();
        for l in &d.locs {
            for f in l.grant_flags.iter().chain(&l.bound_flags) {
                if names.contains(f) {
                    known += 1;
                } else {
                    missing.push(format!("{}: {f}", l.name));
                }
            }
        }
        println!("flag names recorded: {} found in the dictionary, {} missing", known, missing.len());
        for x in missing.iter().take(20) {
            println!("  {x}");
        }
    }
    if bad.is_empty() { Ok(()) } else { Err(Error::Msg("some edit sites do not match the dump".to_string())) }
}

pub fn run_apply(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romvroot")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let (cs, o): (Vec<Choice>, Opts) = parse(&a[3..])?;
    if cs.is_empty() {
        return Err(Error::Format("give at least one location=Item_Actor"));
    }
    let r: Applied = apply(P::new(root), P::new(load), name, &cs, &o)?;
    print!("{}", r.report);
    Ok(())
}

fn print_checked(c: &Checked) {
    println!("{} flow files decoded from the mod folder, {} differences from the originals", c.files, c.diffs.len());
    for x in &c.diffs {
        println!("  {} event {} {} {}: {} -> {}", x.file, x.index, x.event, x.param, x.old, x.new);
    }
    for s in &c.sizes {
        println!("  size {} {} -> {} (table entry {:?})", s.file, s.old, s.new, s.entry);
    }
    for p in &c.problems {
        println!("  PROBLEM {p}");
    }
}

pub fn run_verify(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romvroot")?;
    let modf: &str = arg(a, 1, "missing mod folder")?;
    let (cs, o): (Vec<Choice>, Opts) = parse(&a[2..])?;
    let want: Option<Vec<Diff>> = if cs.is_empty() {
        None
    } else {
        let d: Data = Data::load()?;
        let (ch, _, _): (Vec<Change>, usize, Vec<String>) = plan(&d, &cs, &o)?;
        Some(ch.iter().map(Diff::of).collect())
    };
    let c: Checked = verify(P::new(root), P::new(modf), want.as_deref())?;
    print_checked(&c);
    if c.problems.is_empty() { Ok(()) } else { Err(Error::Msg(format!("{} problems", c.problems.len()))) }
}
