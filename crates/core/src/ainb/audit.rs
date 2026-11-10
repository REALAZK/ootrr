use super::tab::{INSZ, OUTSZ, PSZ};
use super::Ainb;

fn gaps(v: &mut Vec<String>, tag: &str, k: usize, want: &[(usize, usize)], total: usize) {
    let mut at: usize = 0;
    for (i, (b, n)) in want.iter().enumerate() {
        if *b != at {
            v.push(format!("{tag} {k} element {i} starts at {b}, expected {at}"));
            return;
        }
        at += n;
    }
    if at != total {
        v.push(format!("{tag} {k} covers {at} of {total} entries"));
    }
}

pub fn audit(a: &Ainb) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for k in 0..6 {
        let pr: Vec<(usize, usize)> = a.params.iter().map(|p| (p.pe[k][0] as usize, p.pe[k][1] as usize)).collect();
        gaps(&mut v, "property array", k, &pr, a.props[k].len() / PSZ[k]);
        let ir: Vec<(usize, usize)> = a.params.iter().map(|p| (p.ioe[k][0] as usize, p.ioe[k][1] as usize)).collect();
        gaps(&mut v, "input array", k, &ir, a.ins[k].len() / INSZ[k]);
        let or: Vec<(usize, usize)> = a.params.iter().map(|p| (p.ioe[k][2] as usize, p.ioe[k][3] as usize)).collect();
        gaps(&mut v, "output array", k, &or, a.outs[k].len() / OUTSZ[k]);
    }
    for (i, p) in a.params.iter().enumerate() {
        let mut at: usize = 0;
        for g in &p.pl {
            if g[1] as usize != at {
                v.push(format!("element {i} plug groups are not contiguous"));
                break;
            }
            at += g[0] as usize;
        }
        for k in 0..6 {
            for g in a.input_ids(i, k) {
                if let Some(x) = a.input(k, g) {
                    if a.pool.get(x.name).is_none() {
                        v.push(format!("element {i} input {g} has a bad name offset"));
                    }
                }
            }
        }
    }
    v
}
