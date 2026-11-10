use crate::flags::{Body, Flag, FlagList, hash_name};
use crate::json::Json;

pub const NAMES: [(&str, &str); 11] = [
    ("IsGet.Test_A", "Bool"),
    ("IsGet.Test_B", "Bool"),
    ("PouchData.Test_A.StockNum", "Int"),
    ("Step_Test", "Enum"),
    ("Test_Arr", "IntArray"),
    ("Test_Bits", "BoolArray"),
    ("Sequence_CurrentBanc", "String64"),
    ("PlayerStatus.SavePos", "Vector3"),
    ("PlayerStatus.SavePosRadY", "Float"),
    ("Test_Bin", "Binary"),
    ("Test_Big", "UInt64"),
];

pub fn flag_list() -> FlagList {
    let mut v: Vec<Flag> = Vec::new();
    for (n, t) in NAMES {
        let body: Body = if t == "Enum" {
            Body::Values(vec![
                "NotReady".to_string(),
                "Ready".to_string(),
                "Complete".to_string(),
            ])
        } else {
            Body::Plain
        };
        v.push(Flag {
            hash: hash_name(n) as u64,
            ty: t.to_string(),
            def: Json::Null,
            reset: 8,
            save: 0,
            extra: None,
            size: None,
            body,
            name: Some(n.to_string()),
            how: None,
        });
    }
    FlagList::from_flags(v)
}

pub fn h(n: &str) -> u32 {
    hash_name(n)
}

pub enum Cell {
    Inline(u32),
    Seg(Vec<u8>),
}

fn le(x: u32) -> Vec<u8> {
    x.to_le_bytes().to_vec()
}

pub fn raw(cells: &[(u32, Cell)], keys: &[u64]) -> Vec<u8> {
    let start: usize = 0x28 + 8 * cells.len() + 8;
    let mut b: Vec<u8> = vec![0; 0x28];
    b[0..4].copy_from_slice(&[4, 3, 2, 1]);
    b[4..8].copy_from_slice(&le(0x2d80eb));
    b[8..12].copy_from_slice(&le(start as u32));
    let mut pos: usize = start;
    let mut data: Vec<u8> = Vec::new();
    for (hash, c) in cells {
        b.extend(le(*hash));
        match c {
            Cell::Inline(v) => b.extend(le(*v)),
            Cell::Seg(s) => {
                b.extend(le(pos as u32));
                pos += s.len();
                data.extend(s);
            }
        }
    }
    b.extend(le(0xa3db_7114));
    b.extend(le(pos as u32));
    b.extend(data);
    for k in keys {
        b.extend(k.to_le_bytes());
    }
    b.extend(0u64.to_le_bytes());
    b
}

pub fn text(s: &str, n: usize) -> Vec<u8> {
    let mut v: Vec<u8> = vec![0; n];
    v[..s.len()].copy_from_slice(s.as_bytes());
    v
}

pub fn sample() -> Vec<u8> {
    let mut arr: Vec<u8> = le(3);
    for x in [10u32, 20, 30] {
        arr.extend(le(x));
    }
    let mut pos: Vec<u8> = Vec::new();
    for f in [1.0f32, 2.0, 3.0] {
        pos.extend(f.to_le_bytes());
    }
    let mut bin: Vec<u8> = le(8);
    bin.extend([9u8; 8]);
    let cells: Vec<(u32, Cell)> = vec![
        (h("IsGet.Test_A"), Cell::Inline(1)),
        (h("IsGet.Test_B"), Cell::Inline(0)),
        (0, Cell::Inline(1)),
        (h("PouchData.Test_A.StockNum"), Cell::Inline(7)),
        (h("Step_Test"), Cell::Inline(h("Ready"))),
        (0, Cell::Inline(2)),
        (h("Test_Arr"), Cell::Seg(arr)),
        (h("Test_Bits"), Cell::Seg([le(5), le(0b10100)].concat())),
        (
            h("Sequence_CurrentBanc"),
            Cell::Seg(text("KokiriForest_Child", 64)),
        ),
        (h("PlayerStatus.SavePos"), Cell::Seg(pos)),
        (
            h("PlayerStatus.SavePosRadY"),
            Cell::Inline(0.5f32.to_bits()),
        ),
        (h("Test_Bin"), Cell::Seg(bin)),
        (h("Test_Big"), Cell::Seg(5u64.to_le_bytes().to_vec())),
    ];
    raw(&cells, &[3, 9, 27])
}
