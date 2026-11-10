use std::fmt::Write;

use super::tab::{NAMES, PSZ};
use super::{Ainb, Input};

fn val(a: &Ainb, k: usize, v: &[u8]) -> String {
    match k {
        0 => format!("{}", i32::from_le_bytes([v[0], v[1], v[2], v[3]])),
        1 => format!("{}", u32::from_le_bytes([v[0], v[1], v[2], v[3]]) != 0),
        2 => format!("{}", f32::from_le_bytes([v[0], v[1], v[2], v[3]])),
        3 => format!("{:?}", a.s(u32::from_le_bytes([v[0], v[1], v[2], v[3]]))),
        4 => {
            let f = |i: usize| f32::from_le_bytes([v[i], v[i + 1], v[i + 2], v[i + 3]]);
            format!("({}, {}, {})", f(0), f(4), f(8))
        }
        _ => String::new(),
    }
}

pub fn show(a: &Ainb) -> String {
    let mut out: String = String::new();
    let _ = writeln!(out, "ainb elements {} commands {} pool {} bytes", a.els.len(), a.cmds.len() / 0x18, a.pool.b.len());
    for i in 0..a.els.len() {
        let _ = writeln!(out, "element {} type {} {}", i, a.el_type(i), a.el_name(i));
        let p = &a.params[i];
        for k in 0..6 {
            let (b, n): (usize, usize) = (p.pe[k][0] as usize, p.pe[k][1] as usize);
            for g in b..b + n {
                let e: &[u8] = &a.props[k][g * PSZ[k]..(g + 1) * PSZ[k]];
                let _ = writeln!(out, "  prop {} {} = {}", NAMES[k], a.prop_name(k, g).unwrap_or("?"), val(a, k, &e[8..]));
            }
        }
        for k in 0..6 {
            for g in a.input_ids(i, k) {
                if let Some(x) = a.input(k, g) {
                    let from: String = if x.src == -1 { String::new() } else { format!(" from {}.{}", x.src, x.plug) };
                    let Input { name, flags, val: v, .. } = x;
                    let _ = writeln!(out, "  in {} {} = {}{} flags {:#x}", NAMES[k], a.s(name), val(a, k, &v), from, flags);
                }
            }
        }
        for (j, g) in p.plugs.iter().enumerate() {
            let _ = writeln!(out, "  plug {} -> element {} {:?} extra {}", j, g.a, a.s(g.b), g.extra.len());
        }
        let gs: Vec<String> = p.pl.iter().map(|g| format!("{}@{}", g[0], g[1])).collect();
        let _ = writeln!(out, "  plug groups {}", gs.join(" "));
    }
    out
}
