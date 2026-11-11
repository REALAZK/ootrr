use crate::flags::{hash_name, parse_hash, FlagList, KEYED};

pub const RECOVERED: &str = include_str!("../../../data/flag_names.tsv");

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Applied {
    pub named: usize,
    pub replaced: Vec<(String, String)>,
    pub rejected: usize,
}

pub fn apply(l: &mut FlagList) -> Applied {
    let mut out: Applied = Applied::default();
    for line in RECOVERED.lines().skip(1) {
        let mut c = line.split('\t');
        let (Some(h), Some(_), Some(n), Some(how)) = (c.next(), c.next(), c.next(), c.next()) else {
            continue;
        };
        let Some(hash) = parse_hash(h) else {
            out.rejected += 1;
            continue;
        };
        let Some(i) = l.index_of(hash) else {
            out.rejected += 1;
            continue;
        };
        let f = &mut l.flags[i];
        if f.ty == KEYED || hash_name(n) as u64 != hash {
            out.rejected += 1;
            continue;
        }
        match f.name.as_deref() {
            Some(old) if old == n => {}
            Some(old) => {
                out.replaced.push((old.to_string(), n.to_string()));
                f.name = Some(n.to_string());
                f.how = Some(format!("recovered:{how}"));
            }
            None => {
                f.name = Some(n.to_string());
                f.how = Some(format!("recovered:{how}"));
                out.named += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_recovered_name_hashes_to_its_key() {
        let mut n: usize = 0;
        for line in RECOVERED.lines().skip(1) {
            let c: Vec<&str> = line.split('\t').collect();
            assert_eq!(c.len(), 4, "{line}");
            assert_eq!(hash_name(c[2]) as u64, parse_hash(c[0]).unwrap(), "{line}");
            n += 1;
        }
        assert!(n > 4000);
    }
}
