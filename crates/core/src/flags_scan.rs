use std::fs;

use crate::error::Res;
use crate::romfs::Romfs;

const SKIP_DIRS: [&str; 12] = [
    "Bake",
    "Model",
    "Phive",
    "PreComputedCulling",
    "Sound",
    "TerrainArc",
    "Tex",
    "Voice",
    "Foliage",
    "Font2",
    "Shader",
    "Movie",
];

const SKIP_EXT: [&str; 12] = [
    ".webm",
    ".bwav",
    ".extm",
    ".hght",
    ".mate",
    ".bphcl",
    ".tscb",
    ".mc",
    ".bntx.zs",
    ".bntx",
    ".genvb.zs",
    ".txt",
];

const MAX_BYTES: usize = 512 << 20;

#[derive(Debug, Default, Clone, Copy)]
pub struct Walked {
    pub files: usize,
    pub bytes: usize,
    pub failed: usize,
}

pub fn class_of(p: &str) -> &'static str {
    let top: &str = p.split('/').next().unwrap_or("");
    match top {
        "Event" => {
            if p.contains(".bfevfl") {
                "flow"
            } else {
                "eventparam"
            }
        }
        "Banc" => "banc",
        "Pack" => "pack",
        "RSDB" => "rsdb",
        "Logic" | "Sequence" | "AI" => "ainb",
        "Mals" => "text",
        "GameData" => "gamedata",
        "AS" => "asb",
        "AnimationEvent" => "baev",
        "AoC1" => "aoc",
        "Effect" => "effect",
        "UI" => "ui",
        "Scene" | "WorldMgr" | "GameParameter" | "Game" | "Component" | "System" | "AISchedule" => {
            "param"
        }
        _ => "other",
    }
}

fn skipped(p: &str) -> bool {
    let top: &str = p.split('/').next().unwrap_or("");
    SKIP_DIRS.contains(&top) || SKIP_EXT.iter().any(|e| p.ends_with(e))
}

pub fn walk(rf: &mut Romfs, f: &mut dyn FnMut(&str, &[u8])) -> Res<Walked> {
    let mut all: Vec<String> = Vec::new();
    for e in fs::read_dir(&rf.root)? {
        let e: fs::DirEntry = e?;
        let n: String = e.file_name().to_string_lossy().to_string();
        if e.file_type()?.is_dir() && !SKIP_DIRS.contains(&n.as_str()) {
            all.extend(rf.list(&n)?);
        }
    }
    all.sort();
    let mut out: Walked = Walked::default();
    for p in all.iter().filter(|p| !skipped(p)) {
        let raw: Vec<u8> = match rf.read(p) {
            Ok(b) => b,
            Err(_) => {
                out.failed += 1;
                continue;
            }
        };
        let data: Vec<u8> = if p.ends_with(".zs") {
            match rf.zs.decompress(&raw) {
                Ok(d) => d,
                Err(_) => {
                    out.failed += 1;
                    continue;
                }
            }
        } else {
            raw
        };
        if data.len() > MAX_BYTES {
            out.failed += 1;
            continue;
        }
        out.files += 1;
        out.bytes += data.len();
        f(p, &data);
    }
    Ok(out)
}

pub fn runs(b: &[u8], min: usize, f: &mut dyn FnMut(&str)) {
    let mut s: usize = 0;
    let mut i: usize = 0;
    while i <= b.len() {
        let on: bool = i < b.len() && (0x21..=0x7e).contains(&b[i]);
        if !on {
            if i - s >= min
                && let Ok(t) = std::str::from_utf8(&b[s..i])
            {
                f(t);
            }
            s = i + 1;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_runs() {
        let b: &[u8] = b"\x00abc\x00de\x00IsGet.Item_X\x01\x02ok";
        let mut got: Vec<String> = Vec::new();
        runs(b, 3, &mut |s| got.push(s.to_string()));
        assert_eq!(got, vec!["abc", "IsGet.Item_X"]);
    }

    #[test]
    fn classes() {
        assert_eq!(class_of("Event/EventFlow/A.bfevfl.zs"), "flow");
        assert_eq!(class_of("Banc/X/Y.bcett.byml.zs"), "banc");
        assert_eq!(class_of("Zzz/a"), "other");
    }

    #[test]
    fn skips_heavy_dirs() {
        assert!(skipped("Tex/a.bntx.zs"));
        assert!(skipped("Event/x.webm"));
        assert!(!skipped("Event/EventFlow/a.bfevfl.zs"));
    }
}
