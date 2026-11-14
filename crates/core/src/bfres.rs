use std::collections::BTreeMap as BM;

use crate::error::{Error, Res};

pub const VEC4: u16 = 15;
const MODEL: usize = 0x78;
const MAT: usize = 0xb0;
const SLOT: usize = 0x18;
const MAX: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub key: u64,
    pub kind: u16,
    pub at: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mat {
    pub name: String,
    pub params: Vec<Param>,
}

fn bad(m: &'static str) -> Error {
    Error::Format(m)
}

fn u16_at(b: &[u8], o: usize) -> Res<usize> {
    let s: &[u8] = b.get(o..o.saturating_add(2)).ok_or(bad("bfres: the file ends inside a field"))?;
    Ok(u16::from_le_bytes([s[0], s[1]]) as usize)
}

fn u32_at(b: &[u8], o: usize) -> Res<usize> {
    let s: &[u8] = b.get(o..o.saturating_add(4)).ok_or(bad("bfres: the file ends inside a field"))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
}

fn u64_at(b: &[u8], o: usize) -> Res<u64> {
    let s: &[u8] = b.get(o..o.saturating_add(8)).ok_or(bad("bfres: the file ends inside a field"))?;
    Ok(u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
}

fn ptr_at(b: &[u8], o: usize) -> Res<usize> {
    usize::try_from(u64_at(b, o)?).ok().filter(|p| *p != 0 && *p < b.len()).ok_or(bad("bfres: a pointer leaves the file"))
}

fn text(b: &[u8], o: usize) -> Res<String> {
    let n: usize = u16_at(b, o)?;
    let s: &[u8] = b.get(o + 2..o + 2 + n).ok_or(bad("bfres: a name leaves the file"))?;
    Ok(String::from_utf8_lossy(s).into_owned())
}

fn head(b: &[u8]) -> Res<()> {
    if b.len() < 0xe0 || &b[..4] != b"FRES" {
        return Err(bad("bfres: bad magic"));
    }
    Ok(())
}

fn mat(b: &[u8], o: usize) -> Res<Mat> {
    if b.get(o..o + 4) != Some(b"FMAT".as_slice()) {
        return Err(bad("bfres: a material has no FMAT tag"));
    }
    let name: String = text(b, ptr_at(b, o + 8)?)?;
    let assign: usize = ptr_at(b, ptr_at(b, o + 0x10)?)?;
    let n: usize = u16_at(b, assign + 0x4a)?;
    let mut params: Vec<Param> = Vec::with_capacity(n);
    if n > 0 {
        let arr: usize = ptr_at(b, assign + 0x20)?;
        let data: usize = ptr_at(b, o + 0x58)?;
        for i in 0..n {
            let e: usize = arr + i * SLOT;
            let at: usize = data + u16_at(b, e + 0x10)?;
            params.push(Param { key: u64_at(b, e + 8)?, kind: u16_at(b, e + 0x12)? as u16, at });
        }
    }
    Ok(Mat { name, params })
}

pub fn mats(b: &[u8]) -> Res<Vec<Mat>> {
    head(b)?;
    let models: usize = u16_at(b, 0xdc)?;
    let mut out: Vec<Mat> = Vec::new();
    if models == 0 {
        return Ok(out);
    }
    let arr: usize = ptr_at(b, 0x28)?;
    for i in 0..models.min(MAX) {
        let m: usize = arr + i * MODEL;
        if b.get(m..m + 4) != Some(b"FMDL".as_slice()) {
            return Err(bad("bfres: a model has no FMDL tag"));
        }
        if u64_at(b, m + 0x38)? == 0 {
            continue;
        }
        let list: usize = ptr_at(b, m + 0x38)?;
        let n: usize = u32_at(b, ptr_at(b, m + 0x40)? + 4)?;
        for j in 0..n.min(MAX) {
            out.push(mat(b, list + j * MAT)?);
        }
    }
    Ok(out)
}

pub fn keys(b: &[u8]) -> Res<BM<String, u64>> {
    head(b)?;
    let hashes: usize = ptr_at(b, 0xb8)?;
    let dict: usize = ptr_at(b, 0xc0)?;
    let n: usize = u32_at(b, dict + 4)?;
    let mut out: BM<String, u64> = BM::new();
    for i in 0..n.min(MAX * 16) {
        let name: String = text(b, ptr_at(b, dict + 0x20 + 16 * i)?)?;
        out.insert(name, u64_at(b, hashes + 8 * i)?);
    }
    Ok(out)
}

pub fn read4(b: &[u8], at: usize) -> Res<[f32; 4]> {
    let s: &[u8] = b.get(at..at.saturating_add(16)).ok_or(bad("bfres: a value leaves the file"))?;
    let f = |i: usize| f32::from_le_bytes([s[4 * i], s[4 * i + 1], s[4 * i + 2], s[4 * i + 3]]);
    Ok([f(0), f(1), f(2), f(3)])
}

pub fn write4(b: &mut [u8], at: usize, v: [f32; 4]) -> Res<()> {
    let s: &mut [u8] = b.get_mut(at..at.saturating_add(16)).ok_or(bad("bfres: a value leaves the file"))?;
    for (i, x) in v.iter().enumerate() {
        s[4 * i..4 * i + 4].copy_from_slice(&x.to_le_bytes());
    }
    Ok(())
}

pub fn find<'a>(ms: &'a [Mat], name: &str, key: u64) -> Option<&'a Param> {
    ms.iter().find(|m| m.name == name)?.params.iter().find(|p| p.key == key && p.kind == VEC4)
}

#[cfg(test)]
pub mod fixture {
    fn le64(out: &mut [u8], o: usize, v: u64) {
        out[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn name(out: &mut Vec<u8>, s: &str) -> u64 {
        let o: usize = out.len();
        out.extend_from_slice(&(s.len() as u16).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        while out.len() % 8 != 0 {
            out.push(0);
        }
        o as u64
    }

    pub fn model(ms: &[(&str, &[(u64, [f32; 4])])]) -> Vec<u8> {
        let mut out: Vec<u8> = vec![0; 0xf0 + 0x78];
        out[..4].copy_from_slice(b"FRES");
        out[0xdc] = 1;
        le64(&mut out, 0x28, 0xf0);
        out[0xf0..0xf4].copy_from_slice(b"FMDL");
        let list: usize = out.len();
        out.resize(list + ms.len() * 0xb0, 0);
        le64(&mut out, 0xf0 + 0x38, list as u64);
        let dict: usize = out.len();
        out.resize(dict + 8, 0);
        out[dict + 4..dict + 8].copy_from_slice(&(ms.len() as u32).to_le_bytes());
        le64(&mut out, 0xf0 + 0x40, dict as u64);
        for (i, (n, ps)) in ms.iter().enumerate() {
            let o: usize = list + i * 0xb0;
            out[o..o + 4].copy_from_slice(b"FMAT");
            let at: u64 = name(&mut out, n);
            le64(&mut out, o + 8, at);
            let info: usize = out.len();
            out.resize(info + 8, 0);
            le64(&mut out, o + 0x10, info as u64);
            let assign: usize = out.len();
            out.resize(assign + 0x50, 0);
            le64(&mut out, info, assign as u64);
            out[assign + 0x4a..assign + 0x4c].copy_from_slice(&(ps.len() as u16).to_le_bytes());
            let arr: usize = out.len();
            out.resize(arr + ps.len() * 0x18, 0);
            le64(&mut out, assign + 0x20, arr as u64);
            let data: usize = out.len();
            le64(&mut out, o + 0x58, data as u64);
            for (j, (k, v)) in ps.iter().enumerate() {
                let e: usize = arr + j * 0x18;
                le64(&mut out, e + 8, *k);
                out[e + 0x10..e + 0x12].copy_from_slice(&((16 * j) as u16).to_le_bytes());
                out[e + 0x12..e + 0x14].copy_from_slice(&15u16.to_le_bytes());
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
        }
        out.resize(out.len() + 16, 0);
        out
    }

    pub fn names(ks: &[(&str, u64)]) -> Vec<u8> {
        let mut out: Vec<u8> = vec![0; 0xf0];
        out[..4].copy_from_slice(b"FRES");
        le64(&mut out, 0xb8, 0xf0);
        for (_, k) in ks {
            out.extend_from_slice(&k.to_le_bytes());
        }
        let dict: usize = out.len();
        le64(&mut out, 0xc0, dict as u64);
        out.resize(dict + 0x18 + 16 * ks.len(), 0);
        out[dict + 4..dict + 8].copy_from_slice(&(ks.len() as u32).to_le_bytes());
        for (i, (n, _)) in ks.iter().enumerate() {
            let at: u64 = name(&mut out, n);
            le64(&mut out, dict + 0x20 + 16 * i, at);
        }
        out.resize(out.len() + 16, 0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf as PB;

    use crate::romfs::Romfs;

    fn fake() -> Vec<u8> {
        fixture::model(&[("Mt_A", &[(7, [0.1, 0.2, 0.3, 1.0]), (9, [1.0, 1.0, 1.0, 1.0])]), ("Mt_B", &[(7, [0.5, 0.6, 0.7, 0.8])])])
    }

    #[test]
    fn a_fake_model_lists_its_materials_and_values() {
        let b: Vec<u8> = fake();
        let ms: Vec<Mat> = mats(&b).unwrap();
        assert_eq!(ms.iter().map(|m| m.name.as_str()).collect::<Vec<&str>>(), vec!["Mt_A", "Mt_B"]);
        assert_eq!(ms[0].params.len(), 2);
        assert_eq!(read4(&b, find(&ms, "Mt_B", 7).unwrap().at).unwrap(), [0.5, 0.6, 0.7, 0.8]);
        assert_eq!(read4(&b, find(&ms, "Mt_A", 9).unwrap().at).unwrap(), [1.0; 4]);
        assert!(find(&ms, "Mt_B", 9).is_none());
    }

    #[test]
    fn writing_the_values_back_reproduces_the_file_and_a_change_stays_local() {
        let old: Vec<u8> = fake();
        let mut b: Vec<u8> = old.clone();
        let ms: Vec<Mat> = mats(&b).unwrap();
        for p in ms.iter().flat_map(|m| m.params.iter()) {
            let v: [f32; 4] = read4(&b, p.at).unwrap();
            write4(&mut b, p.at, v).unwrap();
        }
        assert_eq!(b, old);
        let at: usize = find(&ms, "Mt_A", 7).unwrap().at;
        write4(&mut b, at, [9.0, 8.0, 7.0, 6.0]).unwrap();
        assert_eq!(b.len(), old.len());
        assert_eq!(b.iter().zip(&old).filter(|(x, y)| x != y).count().div_ceil(16), 1);
        assert_eq!(read4(&b, find(&ms, "Mt_B", 7).unwrap().at).unwrap(), [0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn the_name_table_pairs_each_name_with_its_key() {
        let b: Vec<u8> = fixture::names(&[("alpha", 11), ("beta_colour0", 22)]);
        let k: BM<String, u64> = keys(&b).unwrap();
        assert_eq!((k["alpha"], k["beta_colour0"], k.len()), (11, 22, 2));
    }

    #[test]
    fn damaged_files_are_refused() {
        let b: Vec<u8> = fake();
        assert!(mats(b"nope").is_err());
        assert!(mats(&b[..0x120]).is_err());
        let mut c: Vec<u8> = b.clone();
        c[0xf0] = b'X';
        assert!(mats(&c).is_err());
        assert!(read4(&b, b.len() - 8).is_err());
        assert!(keys(&b[..0x80]).is_err());
    }

    #[test]
    fn the_real_models_list_colour_values_and_rewrite_to_the_same_bytes() {
        let Some(root) = std::env::var("OOTRR_ROMFS").map(PB::from).ok().filter(|p| p.join("Pack").join("ZsDic.pack.zs").is_file()) else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let k: BM<String, u64> = keys(&rf.read_zs("Shader/ExternalBinaryString.bfres.zs").unwrap()).unwrap();
        let key: u64 = k["ns_param_color0"];
        let mut seen: usize = 0;
        for f in ["Player_Adult.Player_Adult", "Player_Child.Player_Child", "Weapon_Sword_Master.Weapon_Sword_Master", "Item_Boomerang.Item_Boomerang", "Npc_Navi.Npc_Navi"] {
            let old: Vec<u8> = rf.zs.decompress(&fs::read(root.join("Model").join(format!("{f}.bfres.zs"))).unwrap()).unwrap();
            let mut b: Vec<u8> = old.clone();
            let ms: Vec<Mat> = mats(&b).unwrap();
            assert!(!ms.is_empty(), "{f}");
            for p in ms.iter().flat_map(|m| m.params.iter()).filter(|p| p.kind == VEC4) {
                let v: [f32; 4] = read4(&b, p.at).unwrap();
                assert!(v.iter().all(|x| x.is_finite()), "{f}");
                write4(&mut b, p.at, v).unwrap();
                seen += (p.key == key) as usize;
            }
            assert_eq!(b, old, "{f}");
        }
        assert!(seen >= 30, "{seen}");
    }
}
