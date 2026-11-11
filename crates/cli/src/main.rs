mod boss_heart_cmd;
mod calibrate_cmd;
mod check_ainb;
mod check_bfevfl;
mod check_catalog;
mod check_msbt;
mod check_rstbl;
mod check_sarc;
mod entrance_cmd;
mod entrance_json;
mod event_chests_cmd;
mod flags_cmd;
mod flow_cmd;
mod game_cmd;
mod hints_cmd;
mod items_cmd;
mod logic_cmd;
mod pot_cmd;
mod progressive_cmd;
mod save_cmd;
mod serve_cmd;
mod settings_cmd;
mod slice_cmd;

use std::env;
use std::fs;
use std::path::Path as P;
use std::process::ExitCode;
use std::time::Instant;

use ootrr_core::TITLE_ID;
use ootrr_core::byml::Byml;
use ootrr_core::error::{Error, Res};
use ootrr_core::ips::Ips;
use ootrr_core::modpack::Pack;
use ootrr_core::placement;
use ootrr_core::rng::Rng;
use ootrr_core::romfs::Romfs;
use ootrr_core::zs;

const USAGE: &str = "usage:
  ootrr shuffle <seed> <count>
  ootrr init <load_dir> <mod_name>
  ootrr ips <out_file> <hex_offset> <hex_bytes>
  ootrr check-byml <romfs_root> <subfolder> <suffix>
  ootrr check-sarc <romfs_root> <subfolder> <suffix>
  ootrr check-rstbl <romfs_root> [sample_per_group]
  ootrr check-ainb <romfs_root> <subfolder>
  ootrr ainb-dump <romfs_root> <relative_path>
  ootrr ainb-start <romfs_root> <load_dir_or_-> <mod_name> [--hearts N] [--item NAME[:COUNT]]...
  ootrr check-bfevfl <romfs_root> <subfolder>
  ootrr bfevfl-dump <romfs_root> <relative_path>
  ootrr bfevfl-rewrite <romfs_root> <relative_path> <out_file>
  ootrr bfevfl-replace <romfs_root> <relative_path> <old_item> <new_item> [out_file]
  ootrr flow-durations <romfs_root> [name_prefix] [top_n]
  ootrr flow-scale <romfs_root> <load_dir> <mod_name> <factor> <min_frames> <relative_path>...
  ootrr flow-force <romfs_root> <load_dir> <mod_name> <relative_path> <event_name> <case_value> [<event_name> <case_value>]...
  ootrr flow-rewards <romfs_root> <out_json>
  ootrr flow-locations-report [--tests] [--all]
  ootrr flow-locations-check <romfs_root> [flags_json]
  ootrr flow-apply <romfs_root> <load_dir> <mod_name> [--untested] <location=Item_Actor>...
  ootrr flow-verify <romfs_root> <mod_folder> [--untested] [location=Item_Actor]...
  ootrr check-catalog <romfs_root> <catalog_json>
  ootrr calibrate <romfs_root> <load_dir> <mod_name> <area> [--catalog <json>] [--dry]
  ootrr check-msbt <romfs_root> [language ... | all]
  ootrr settings-dump [--compact] [--presets <dir>] [--decode <settings_string>]
  ootrr serve [--port N] [--lan] [--no-open]
  ootrr generate <settings_string_or_preset_or_file> <seed> <out.ootrr> [--spoiler <file>]
  ootrr build <seed.ootrr> <romfs_root> <load_dir> <mod_name>
  ootrr slice-scan <romfs_root> <out_json>
  ootrr flags-build <romfs_root> <out_json>
  ootrr flags-lookup <flags_json> <name_or_hash> ...
  ootrr flags-check <romfs_root> <flags_json>
  ootrr flags-report <flags_json>
  ootrr items-check <romfs_root> <flow_rewards_json> [out_md]
  ootrr logic-demo [seed]
  ootrr logic-bench [locations] [seed] [chain]
  ootrr logic-load <ootr_data_dir> <World_or_Glitched> [vanilla_or_mq]
  ootrr event-chests <romfs_root> <locations_json> <slice_json> <out_json>
  ootrr event-chest-show <romfs_root> <event_name> ...
  ootrr event-chests-testmod <romfs_root> <event_chests_json> <load_dir> <mod_name>
  ootrr find-drop <romfs_root> <item_name_or_star>
  ootrr set-drop <romfs_root> <load_dir> <mod_name> <actor_hash> <item_name> [no-event]
  ootrr hints-show <seed.ootrr> <romfs_root>
  ootrr check-game <romfs_root>
  ootrr pot-set <romfs_root> <load_dir> <mod_name> [--rst-from <rsizetable.zs>] [--banc-from <mod_romfs_dir>] <ActorHash>=<Item_Actor>...
  ootrr boss-heart <romfs_root> <load_dir> <mod_name> [--rst-from <rsizetable.zs>] [--banc-from <mod_romfs_dir>] [--chest <actor>] [--slot] <Location>=<Item_Actor>...
  ootrr progressive <romfs_root> <load_dir> <mod_name> [--scale <factor> <min_frames>] [--rst-from <rsizetable.zs>] [--banc-from <mod_romfs_dir>] [--grants] <Family>=<hash>[,<hash>...]...
  ootrr zs-extract <romfs_root> <relative_path> <out_file>
  ootrr zs-pack <in_file> <out_file> <dict_id_or_0>
  ootrr byml-rewrite <romfs_root> <relative_path> <out_file>
  ootrr entrance-catalog <romfs_root> <out_json>
  ootrr entrance-list <romfs_root> [filter]
  ootrr swap-entrance <romfs_root> <load_dir> <mod_name> <entrance_a> <entrance_b> [force]
  ootrr save-info <progress.sav> [--flags <flags_json>]
  ootrr save-get <progress.sav> <name_or_hash>... [--flags <flags_json>]
  ootrr save-set <in> <out> <name=value>... [--flags <flags_json>]
  ootrr save-unlock <in> <out> <preset[,preset]> [name=value]... [--start Map/StartPos --romfs <root> | --map M --xyz x,y,z --rad-y R] [--keys clear] [--key-on <hash,hash>] [--key-off <hash,hash>] [--flags <flags_json>]
  ootrr save-check <progress.sav>... [--flags <flags_json>]
  ootrr save-diff <a> <b> [--flags <flags_json>]
  ootrr save-presets [--in <progress.sav>] [--flags <flags_json>]
  ootrr save-starts <romfs_root> [filter]";

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

fn hex(s: &str) -> Res<Vec<u8>> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !t.len().is_multiple_of(2) {
        return Err(Error::Format("odd hex length"));
    }
    let b: &[u8] = t.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len() / 2);
    for i in (0..b.len()).step_by(2) {
        let h: &str = std::str::from_utf8(&b[i..i + 2]).map_err(|_| Error::Format("bad hex"))?;
        out.push(u8::from_str_radix(h, 16).map_err(|_| Error::Format("bad hex"))?);
    }
    Ok(out)
}

fn shuffle(a: &[String]) -> Res<()> {
    let seed: &str = arg(a, 0, "missing seed")?;
    let n: usize = arg(a, 1, "missing count")?
        .parse()
        .map_err(|_| Error::Format("bad count"))?;
    if n > 1_000_000 {
        return Err(Error::Format("count too large"));
    }
    let mut v: Vec<usize> = (0..n).collect();
    Rng::from_text(seed).shuffle(&mut v);
    let s: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    println!("{}", s.join(" "));
    Ok(())
}

fn init(a: &[String]) -> Res<()> {
    let load: &str = arg(a, 0, "missing load_dir")?;
    let name: &str = arg(a, 1, "missing mod_name")?;
    let pk: Pack = Pack::create(P::new(load), TITLE_ID, name)?;
    println!("{}", pk.root.display());
    Ok(())
}

fn ips(a: &[String]) -> Res<()> {
    let out: &str = arg(a, 0, "missing out_file")?;
    let off_s: &str = arg(a, 1, "missing hex_offset")?;
    let off: u32 = u32::from_str_radix(off_s.trim_start_matches("0x"), 16)
        .map_err(|_| Error::Format("bad offset"))?;
    let data: Vec<u8> = hex(arg(a, 2, "missing hex_bytes")?)?;
    let mut p: Ips = Ips::new();
    p.add(off, &data)?;
    fs::write(out, p.to_bytes())?;
    println!("{} record(s) written", p.recs.len());
    Ok(())
}

fn check_one(rf: &mut Romfs, f: &str) -> Res<(bool, bool)> {
    let raw: Vec<u8> = if f.ends_with(".zs") { rf.read_zs(f)? } else { rf.read(f)? };
    let a: Byml = Byml::parse(&raw)?;
    let out: Vec<u8> = a.write()?;
    let b: Byml = Byml::parse(&out)?;
    let packed: Vec<u8> = zs::compress_raw(&out, Some(3))?;
    let back: Vec<u8> = rf.zs.decompress(&packed)?;
    Ok((out == raw, a == b && back == out))
}

fn check_byml(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let sub: &str = arg(a, 1, "missing subfolder")?;
    let suf: &str = arg(a, 2, "missing file suffix")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf
        .list(sub)?
        .into_iter()
        .filter(|p| p.ends_with(suf))
        .collect();
    let t0: Instant = Instant::now();
    let mut same: usize = 0;
    let mut eq: usize = 0;
    let mut bad: Vec<String> = Vec::new();
    for f in &files {
        match check_one(&mut rf, f) {
            Ok((s, e)) => {
                if s {
                    same += 1;
                } else {
                    println!("  differs from the original bytes: {f}");
                }
                if e {
                    eq += 1;
                } else {
                    bad.push(format!("{f}: structure or zstd mismatch"));
                }
            }
            Err(e) => bad.push(format!("{f}: {e}")),
        }
    }
    println!(
        "files: {}  byte-identical rewrites: {}  structurally equal: {}  failures: {}",
        files.len(),
        same,
        eq,
        bad.len()
    );
    for m in bad.iter().take(8) {
        println!("  {m}");
    }
    println!("elapsed: {:.1}s", t0.elapsed().as_secs_f32());
    Ok(())
}

fn find_drop(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let pat: &str = arg(a, 1, "missing item name or *")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let hits: Vec<placement::Hit> = placement::drops(&mut rf)?
        .into_iter()
        .filter(|h| pat == "*" || h.item == pat)
        .collect();
    for h in hits.iter().take(40) {
        println!(
            "{}  {}  name={}  hash={}  item={}  pos={:?}",
            h.file, h.gyaml, h.name, h.hash, h.item, h.pos
        );
    }
    println!("{} matching actor(s)", hits.len());
    Ok(())
}

fn set_drop(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let hash: u64 = arg(a, 3, "missing actor hash")?
        .parse()
        .map_err(|_| Error::Format("bad actor hash"))?;
    let item: &str = arg(a, 4, "missing item name")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let clear: bool = a.get(5).is_some_and(|s| s == "no-event");
    let edits: Vec<placement::Edit> = placement::set_drop(&mut rf, hash, item, clear)?;
    if edits.is_empty() {
        return Err(Error::Format("no actor with that hash and a Drop__DropActor field"));
    }
    let pk: Pack = Pack::create(P::new(load), TITLE_ID, name)?;
    for e in &edits {
        let z: Vec<u8> = zs::compress_raw(&e.bytes, Some(3))?;
        let at: std::path::PathBuf = pk.write_romfs(&e.file, &z)?;
        println!("{}: {} -> {} bytes decompressed", e.file, e.old_len, e.bytes.len());
        if e.bytes.len() > e.old_len {
            println!("  warning: grew by {} bytes, the resource size table may need an update", e.bytes.len() - e.old_len);
        }
        println!("  wrote {}", at.display());
    }
    Ok(())
}

fn zs_extract(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let r: &str = arg(a, 1, "missing relative path")?;
    let out: &str = arg(a, 2, "missing out_file")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    fs::write(out, rf.read_zs(r)?)?;
    Ok(())
}

fn zs_pack(a: &[String]) -> Res<()> {
    let inp: &str = arg(a, 0, "missing in_file")?;
    let out: &str = arg(a, 1, "missing out_file")?;
    let id: u32 = arg(a, 2, "missing dict_id")?
        .parse()
        .map_err(|_| Error::Format("bad dict_id"))?;
    let d: Option<u32> = if id == 0 { None } else { Some(id) };
    fs::write(out, zs::compress_raw(&fs::read(inp)?, d)?)?;
    Ok(())
}

fn byml_rewrite(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let r: &str = arg(a, 1, "missing relative path")?;
    let out: &str = arg(a, 2, "missing out_file")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let y: Byml = rf.read_byml(r)?;
    fs::write(out, y.write()?)?;
    Ok(())
}

fn main() -> ExitCode {
    let a: Vec<String> = env::args().skip(1).collect();
    let r: Res<()> = match a.first().map(|s| s.as_str()) {
        Some("shuffle") => shuffle(&a[1..]),
        Some("init") => init(&a[1..]),
        Some("ips") => ips(&a[1..]),
        Some("check-byml") => check_byml(&a[1..]),
        Some("check-sarc") => check_sarc::run(&a[1..]),
        Some("check-rstbl") => check_rstbl::run(&a[1..]),
        Some("check-ainb") => check_ainb::run(&a[1..]),
        Some("ainb-dump") => check_ainb::dump(&a[1..]),
        Some("ainb-start") => check_ainb::start_cmd(&a[1..]),
        Some("check-bfevfl") => check_bfevfl::run(&a[1..]),
        Some("bfevfl-dump") => check_bfevfl::dump(&a[1..]),
        Some("bfevfl-rewrite") => check_bfevfl::rewrite(&a[1..]),
        Some("bfevfl-replace") => check_bfevfl::replace(&a[1..]),
        Some("flow-force") => check_bfevfl::force(&a[1..]),
        Some("flow-durations") => check_bfevfl::durations(&a[1..]),
        Some("flow-scale") => check_bfevfl::scale(&a[1..]),
        Some("progressive") => progressive_cmd::run(&a[1..]),
        Some("boss-heart") => boss_heart_cmd::run(&a[1..]),
        Some("hints-show") => hints_cmd::show(&a[1..]),
        Some("check-game") => game_cmd::check(&a[1..]),
        Some("pot-set") => pot_cmd::run(&a[1..]),
        Some("flow-rewards") => check_bfevfl::catalog(&a[1..]),
        Some("flow-locations-report") => flow_cmd::report(&a[1..]),
        Some("flow-locations-check") => flow_cmd::check(&a[1..]),
        Some("flow-apply") => flow_cmd::run_apply(&a[1..]),
        Some("flow-verify") => flow_cmd::run_verify(&a[1..]),
        Some("check-catalog") => check_catalog::run(&a[1..]),
        Some("calibrate") => calibrate_cmd::run(&a[1..]),
        Some("check-msbt") => check_msbt::run(&a[1..]),
        Some("settings-dump") => settings_cmd::run(&a[1..]),
        Some("serve") => serve_cmd::run(&a[1..]),
        Some("generate") => slice_cmd::generate(&a[1..]),
        Some("build") => slice_cmd::build(&a[1..]),
        Some("slice-scan") => slice_cmd::scan_cmd(&a[1..]),
        Some("flags-build") => flags_cmd::build(&a[1..]),
        Some("flags-lookup") => flags_cmd::lookup(&a[1..]),
        Some("flags-check") => flags_cmd::check(&a[1..]),
        Some("flags-report") => flags_cmd::report(&a[1..]),
        Some("items-check") => items_cmd::check(&a[1..]),
        Some("logic-demo") => logic_cmd::demo(&a[1..]),
        Some("logic-bench") => logic_cmd::bench(&a[1..]),
        Some("logic-load") => logic_cmd::load(&a[1..]),
        Some("event-chests") => event_chests_cmd::run(&a[1..]),
        Some("event-chest-show") => event_chests_cmd::show(&a[1..]),
        Some("event-chests-testmod") => event_chests_cmd::testmod(&a[1..]),
        Some("find-drop") => find_drop(&a[1..]),
        Some("set-drop") => set_drop(&a[1..]),
        Some("zs-extract") => zs_extract(&a[1..]),
        Some("zs-pack") => zs_pack(&a[1..]),
        Some("byml-rewrite") => byml_rewrite(&a[1..]),
        Some("entrance-catalog") => entrance_cmd::catalog(&a[1..]),
        Some("entrance-list") => entrance_cmd::list(&a[1..]),
        Some("swap-entrance") => entrance_cmd::swap(&a[1..]),
        Some("save-info") => save_cmd::info(&a[1..]),
        Some("save-get") => save_cmd::get(&a[1..]),
        Some("save-set") => save_cmd::set(&a[1..]),
        Some("save-unlock") => save_cmd::unlock(&a[1..]),
        Some("save-check") => save_cmd::check(&a[1..]),
        Some("save-diff") => save_cmd::compare(&a[1..]),
        Some("save-presets") => save_cmd::list_presets(&a[1..]),
        Some("save-starts") => save_cmd::starts(&a[1..]),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
