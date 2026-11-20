use std::env;
use std::fs;
use std::path::PathBuf;

const ICON: &str = "assets/ootrr.ico";
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const GROUP_ID: u16 = 1;

struct Image {
    width: u8,
    height: u8,
    colors: u8,
    planes: u16,
    bits: u16,
    data: Vec<u8>,
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn images(ico: &[u8]) -> Vec<Image> {
    assert!(ico.len() >= 6 && u16_at(ico, 0) == 0 && u16_at(ico, 2) == 1, "{ICON} is not an icon file");
    (0..u16_at(ico, 4) as usize)
        .map(|i| {
            let e: usize = 6 + 16 * i;
            let (size, at) = (u32_at(ico, e + 8) as usize, u32_at(ico, e + 12) as usize);
            Image { width: ico[e], height: ico[e + 1], colors: ico[e + 2], planes: u16_at(ico, e + 4), bits: u16_at(ico, e + 6), data: ico[at..at + size].to_vec() }
        })
        .collect()
}

fn resource(out: &mut Vec<u8>, kind: u16, id: u16, flags: u16, data: &[u8]) {
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(32u32.to_le_bytes());
    out.extend([0xFFFFu16, kind, 0xFFFF, id].iter().flat_map(|v| v.to_le_bytes()));
    out.extend(0u32.to_le_bytes());
    out.extend(flags.to_le_bytes());
    out.extend(0x0409u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn res_file(imgs: &[Image]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    resource(&mut out, 0, 0, 0, &[]);
    for (i, img) in imgs.iter().enumerate() {
        resource(&mut out, RT_ICON, i as u16 + 1, 0x1010, &img.data);
    }
    let mut group: Vec<u8> = Vec::new();
    group.extend([0u16, 1, imgs.len() as u16].iter().flat_map(|v| v.to_le_bytes()));
    for (i, img) in imgs.iter().enumerate() {
        group.extend([img.width, img.height, img.colors, 0]);
        group.extend(img.planes.to_le_bytes());
        group.extend(img.bits.to_le_bytes());
        group.extend((img.data.len() as u32).to_le_bytes());
        group.extend((i as u16 + 1).to_le_bytes());
    }
    resource(&mut out, RT_GROUP_ICON, GROUP_ID, 0x1030, &group);
    out
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON}");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let ico: Vec<u8> = fs::read(ICON).unwrap_or_else(|e| panic!("could not read {ICON}: {e}"));
    let res: PathBuf = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("ootrr_icon.res");
    fs::write(&res, res_file(&images(&ico))).unwrap_or_else(|e| panic!("could not write {}: {e}", res.display()));
    println!("cargo:rustc-link-arg-bins={}", res.display());
}
