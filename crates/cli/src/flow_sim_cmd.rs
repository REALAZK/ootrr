use std::fs;
use std::path::{Path as P, PathBuf as PB};

use ootrr_core::error::{Error, Res};
use ootrr_core::flow_sim::db::Db;
use ootrr_core::flow_sim::load::Disk;
use ootrr_core::flow_sim::report::{self, Group};
use ootrr_core::flow_sim::run::{End, Env, Fin, Mach, Opts, Sim};
use ootrr_core::flow_sim::state::{State, V};
use ootrr_core::flow_sim::vocab::Census;
use ootrr_core::bfevfl::{Bf, Val};
use ootrr_core::json;
use ootrr_core::romfs::Romfs;

enum Op {
    Set(String, String),
    Own(String, i64),
    Arg(String, String),
    When(String, String),
    Run(String, String),
}

struct Cfg {
    root: String,
    ops: Vec<Op>,
    opts: Opts,
    over: Option<PB>,
    patches: Vec<String>,
    nodep: Vec<String>,
    never: Vec<String>,
    expect: Vec<String>,
    flags: Option<String>,
    compare: bool,
    shown: usize,
}

fn need<'a>(a: &'a [String], i: usize, w: &str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or_else(|| Error::Msg(format!("{w} needs a value")))
}

fn split(s: &str, sep: char, w: &str) -> Res<(String, String)> {
    let (k, v): (&str, &str) = s.split_once(sep).ok_or_else(|| Error::Msg(format!("{s} is not {w}")))?;
    Ok((k.trim().to_string(), v.trim().to_string()))
}

fn parse(a: &[String]) -> Res<Cfg> {
    let mut c: Cfg = Cfg {
        root: need(a, 0, "romfs_root")?.to_string(),
        ops: Vec::new(),
        opts: Opts::default(),
        over: None,
        patches: Vec::new(),
        nodep: Vec::new(),
        never: Vec::new(),
        expect: Vec::new(),
        flags: None,
        compare: false,
        shown: 3,
    };
    let mut pos: Vec<String> = Vec::new();
    let mut i: usize = 1;
    while i < a.len() {
        let t: &str = a[i].as_str();
        match t {
            "--set" => {
                let (k, v) = split(need(a, i + 1, t)?, '=', "name=value")?;
                c.ops.push(Op::Set(k, v));
                i += 1;
            }
            "--own" => {
                let s: &str = need(a, i + 1, t)?;
                let (k, n): (String, i64) = match s.split_once(':') {
                    Some((k, n)) => (k.to_string(), n.parse().map_err(|_| Error::Msg(format!("{s} has a bad count")))?),
                    None => (s.to_string(), 1),
                };
                c.ops.push(Op::Own(k, n));
                i += 1;
            }
            "--arg" => {
                let (k, v) = split(need(a, i + 1, t)?, '=', "name=value")?;
                c.ops.push(Op::Arg(k, v));
                i += 1;
            }
            "--when" => {
                let (k, v) = split(need(a, i + 1, t)?, '=', "name=value")?;
                c.ops.push(Op::When(k, v));
                i += 1;
            }
            "--run" => {
                let (f, e) = split(need(a, i + 1, t)?, ':', "Flow:Entry")?;
                c.ops.push(Op::Run(f, e));
                i += 1;
            }
            "--choose" => {
                c.opts.choose.push(need(a, i + 1, t)?.parse().map_err(|_| Error::Msg("bad choice".to_string()))?);
                i += 1;
            }
            "--answer" => {
                let (k, v) = split(need(a, i + 1, t)?, '=', "Query=case")?;
                c.opts.answers.insert(k, v.parse().map_err(|_| Error::Msg("bad case".to_string()))?);
                i += 1;
            }
            "--hour" => {
                c.opts.hour = Some(need(a, i + 1, t)?.parse().map_err(|_| Error::Msg("bad hour".to_string()))?);
                i += 1;
            }
            "--max-paths" => {
                c.opts.max_paths = need(a, i + 1, t)?.parse().map_err(|_| Error::Msg("bad number".to_string()))?;
                i += 1;
            }
            "--mod" => {
                c.over = Some(PB::from(need(a, i + 1, t)?));
                i += 1;
            }
            "--patch" => {
                c.patches.push(need(a, i + 1, t)?.to_string());
                i += 1;
            }
            "--nodep" => {
                c.nodep.push(need(a, i + 1, t)?.to_string());
                i += 1;
            }
            "--never" => {
                c.never.push(need(a, i + 1, t)?.to_string());
                i += 1;
            }
            "--expect" => {
                c.expect.push(need(a, i + 1, t)?.to_string());
                i += 1;
            }
            "--flags" => {
                c.flags = Some(need(a, i + 1, t)?.to_string());
                i += 1;
            }
            "--shown" => {
                c.shown = need(a, i + 1, t)?.parse().map_err(|_| Error::Msg("bad number".to_string()))?;
                i += 1;
            }
            "--verbose" => c.opts.verbose = true,
            "--follow" => c.opts.follow = true,
            "--compare" => c.compare = true,
            _ => pos.push(t.to_string()),
        }
        i += 1;
    }
    if pos.len() == 2 {
        c.ops.push(Op::Run(pos[0].clone(), pos[1].clone()));
    } else if !pos.is_empty() {
        return Err(Error::Msg(format!("unexpected arguments {pos:?}. give <flow> <entry> or --run Flow:Entry")));
    }
    if !c.ops.iter().any(|o| matches!(o, Op::Run(..))) {
        return Err(Error::Format("no flow to run"));
    }
    Ok(c)
}

fn build(c: &Cfg, patched: bool) -> Res<Sim> {
    let mut rf: Romfs = Romfs::open(P::new(&c.root))?;
    let mut db: Db = Db::load(&mut rf)?;
    if let Some(p) = &c.flags {
        db.add_names(&json::parse(&fs::read_to_string(p)?)?);
    }
    for n in &c.nodep {
        for ch in db.chals.iter_mut().filter(|x| &x.name == n) {
            ch.dep = None;
        }
    }
    let over: Option<PB> = if patched { c.over.clone() } else { None };
    let pat: Vec<String> = if patched { c.patches.clone() } else { Vec::new() };
    let mut s: Sim = Sim::new(db, Box::new(Disk::new(rf, over, pat)), c.opts.clone());
    for n in &c.never {
        let mut lits: Vec<(bool, u32)> = Vec::new();
        for t in n.split(',') {
            let (neg, name): (bool, &str) = match t.trim().strip_prefix('!') {
                Some(x) => (true, x),
                None => (false, t.trim()),
            };
            lits.push((!neg, s.db.reg(name)));
        }
        s.inv.push((n.clone(), lits));
    }
    Ok(s)
}

fn env_of(args: &[(String, String)]) -> Env {
    let mut e: Env = Env::new();
    for (k, v) in args {
        let val: Val = match v.parse::<i32>() {
            Ok(i) => Val::Int(i),
            Err(_) => Val::Str(v.clone()),
        };
        e.insert(k.clone(), val);
    }
    e
}

fn execute(c: &Cfg, patched: bool) -> Res<(Sim, Vec<Fin>, Vec<String>)> {
    let mut s: Sim = build(c, patched)?;
    let mut st: State = State::new();
    let mut live: Vec<State> = Vec::new();
    let mut args: Vec<(String, String)> = Vec::new();
    let mut last: Vec<Fin> = Vec::new();
    let mut first: bool = true;
    let mut guards: Vec<(String, String)> = Vec::new();
    let mut blocked: Vec<String> = Vec::new();
    for o in &c.ops {
        match o {
            Op::Set(k, v) => {
                let h: u32 = s.db.reg(k);
                let val: V = s.db.parse(h, v)?;
                if first {
                    st.v.insert(h, val);
                } else {
                    for x in live.iter_mut() {
                        x.v.insert(h, val.clone());
                    }
                }
            }
            Op::Own(k, n) => {
                if first {
                    s.own(&mut st, k, *n);
                } else {
                    for x in live.iter_mut() {
                        s.own(x, k, *n);
                    }
                }
            }
            Op::Arg(k, v) => args.push((k.clone(), v.clone())),
            Op::When(k, v) => guards.push((k.clone(), v.clone())),
            Op::Run(f, e) => {
                if first {
                    s.settle(&mut st);
                    live.push(st.clone());
                    first = false;
                }
                let mut next: Vec<State> = Vec::new();
                last = Vec::new();
                let mut go: Vec<State> = Vec::new();
                for x in &live {
                    let mut why: Option<String> = None;
                    for (k, v) in &guards {
                        let h: u32 = s.db.reg(k);
                        let want: V = s.db.parse(h, v)?;
                        let got: Option<V> = match ootrr_core::flow_sim::expr::truth(&s.db, x, h, 0) {
                            Some(b) if s.db.defs.get(&h).map(|d| d.ty == ootrr_core::flow_sim::db::Ty::Exp).unwrap_or(false) => Some(V::B(b)),
                            _ => s.db.get(x, h),
                        };
                        if got.as_ref() != Some(&want) {
                            let g: String = got.map(|g| s.db.show(h, &g)).unwrap_or_default();
                            why = Some(format!("{f}:{e} cannot start, {k} is {g} and the event needs {v}"));
                        }
                    }
                    match why {
                        Some(w) => {
                            if !blocked.contains(&w) {
                                blocked.push(w);
                            }
                        }
                        None => go.push(x.clone()),
                    }
                }
                guards.clear();
                for x in &go {
                    let m: Mach = s.start(f, e, x.clone(), env_of(&args))?;
                    for fin in s.explore(m) {
                        if fin.end == End::Done && !next.contains(&fin.m.st) {
                            next.push(fin.m.st.clone());
                        }
                        last.push(fin);
                    }
                }
                args.clear();
                live = next;
                if live.len() > 64 {
                    live.truncate(64);
                }
            }
        }
    }
    Ok((s, last, blocked))
}

pub fn run(a: &[String]) -> Res<()> {
    let c: Cfg = parse(a)?;
    let (s, fins, blocked): (Sim, Vec<Fin>, Vec<String>) = execute(&c, true)?;
    print!("{}", report::render(&s, &fins, c.shown, c.opts.verbose));
    for b in &blocked {
        println!("BLOCKED {b}");
    }
    for (k, v) in &s.assumed {
        println!("note: {k} x{v}");
    }
    for u in &s.unknown {
        println!("warning: flag {u} is not in the GameData list");
    }
    let mut bad: Vec<String> = report::expect(&s, &fins, &c.expect);
    for b in &blocked {
        bad.push(format!("script blocked, so nothing downstream was checked: {b}"));
    }
    if c.compare {
        let (s2, base, blk): (Sim, Vec<Fin>, Vec<String>) = execute(&c, false)?;
        for b in &blk {
            println!("BLOCKED (vanilla) {b}");
        }
        println!("=== vanilla (no --mod, no --patch) against the edited run, outcome by outcome");
        println!("{}", report::render(&s2, &base, c.shown, c.opts.verbose));
        let ga: Vec<Group> = report::groups(&base);
        let gb: Vec<Group> = report::groups(&fins);
        print!("=== diff, A is vanilla and B is edited\n{}", report::diff(&ga, &gb));
    }
    if !c.expect.is_empty() {
        println!("expectations: {} checked, {} violated", c.expect.len(), bad.len());
        for b in bad.drain(..) {
            println!("  VIOLATED {b}");
        }
    }
    Ok(())
}

pub fn vocab(a: &[String]) -> Res<()> {
    let root: &str = need(a, 0, "romfs_root")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf.list("Event/EventFlow")?.into_iter().filter(|p| p.ends_with(".bfevfl.zs")).collect();
    let mut c: Census = Census::default();
    for f in &files {
        let raw: Vec<u8> = rf.read_zs(f)?;
        let b: Bf = Bf::parse(&raw)?;
        c.add(&b);
    }
    print!("{}", c.report());
    let db: Db = Db::load(&mut rf)?;
    let st: State = State::new();
    let mut exps: usize = 0;
    let mut ok: usize = 0;
    let mut ops: std::collections::BTreeMap<i32, usize> = std::collections::BTreeMap::new();
    let mut bad: Vec<String> = Vec::new();
    for (h, d) in &db.defs {
        if d.ty != ootrr_core::flow_sim::db::Ty::Exp {
            continue;
        }
        exps += 1;
        for t in &d.exp {
            *ops.entry(t.op).or_insert(0) += 1;
        }
        match ootrr_core::flow_sim::expr::eval(&db, &st, &d.exp, 0) {
            Some(_) => ok += 1,
            None => bad.push(format!("{h:#010x}")),
        }
    }
    println!("boolexp {exps} evaluate {ok} unevaluable {} ops {ops:?}", bad.len());
    println!("flags in the list {} challenges {} with a depend flag {}", db.defs.len(), db.chals.len(), db.chals.iter().filter(|c| c.dep.is_some()).count());
    Ok(())
}

pub fn scan(a: &[String]) -> Res<()> {
    let root: &str = need(a, 0, "romfs_root")?;
    let paths: usize = match a.get(1) {
        Some(s) => s.parse().map_err(|_| Error::Msg("bad path limit".to_string()))?,
        None => 200,
    };
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf.list("Event/EventFlow")?.into_iter().filter(|p| p.ends_with(".bfevfl.zs")).collect();
    let db: Db = Db::load(&mut rf)?;
    let mut opts: Opts = Opts::default();
    opts.max_paths = paths;
    opts.max_steps = 20000;
    let mut s: Sim = Sim::new(db, Box::new(Disk::new(Romfs::open(P::new(root))?, None, Vec::new())), opts);
    let mut st: State = State::new();
    s.settle(&mut st);
    println!("flow\tentry\tpaths\tdone\tmenu\tloop\tlimit\tfail\tgrants\tsets\tfirst_problem");
    for f in &files {
        let name: String = f.rsplit('/').next().unwrap_or("").trim_end_matches(".bfevfl.zs").to_string();
        let raw: Vec<u8> = rf.read_zs(f)?;
        let b: Bf = Bf::parse(&raw)?;
        let Some(fl) = b.flows.first() else { continue };
        for e in &fl.entries {
            let m: Mach = match s.start(&name, &e.name, st.clone(), Env::new()) {
                Ok(m) => m,
                Err(x) => {
                    println!("{name}\t{}\t0\t0\t0\t0\t0\t1\t\t\tstart: {x}", e.name);
                    continue;
                }
            };
            let fins: Vec<Fin> = s.explore(m);
            let mut c: [usize; 5] = [0; 5];
            let mut gr: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            let mut sets: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            let mut prob: String = String::new();
            for x in &fins {
                match &x.end {
                    End::Done => c[0] += 1,
                    End::Menu(_) => c[1] += 1,
                    End::Loop(w) => {
                        c[2] += 1;
                        if prob.is_empty() {
                            prob = format!("loop at {w}");
                        }
                    }
                    End::Limit => c[3] += 1,
                    End::NoCase(w) | End::Fail(w) => {
                        c[4] += 1;
                        if prob.is_empty() {
                            prob = w.clone();
                        }
                    }
                }
                gr.extend(x.m.gr.keys().cloned());
                sets.extend(x.m.writes.keys().filter(|k| !k.starts_with("UI_")).cloned());
            }
            let sv: Vec<String> = sets.into_iter().take(6).collect();
            println!(
                "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                e.name,
                fins.len(),
                c[0],
                c[1],
                c[2],
                c[3],
                c[4],
                gr.into_iter().collect::<Vec<String>>().join(","),
                sv.join(","),
                prob
            );
        }
    }
    println!("# unknown flags used by the flows: {}", s.unknown.len());
    for u in &s.unknown {
        println!("# unknown\t{u}");
    }
    Ok(())
}
