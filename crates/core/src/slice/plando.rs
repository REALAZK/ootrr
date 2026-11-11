use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::world::{LocId, World};

pub const MAX_ENTRIES: usize = 200;
pub const SUGGEST: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub location: String,
    pub item: String,
}

pub fn parse(s: &str) -> Res<Entry> {
    let (l, i): (&str, &str) = s
        .split_once('=')
        .ok_or_else(|| E::Msg(format!("plando entry {s:?} must look like Location=Item")))?;
    let (l, i): (&str, &str) = (l.trim(), i.trim());
    if l.is_empty() || i.is_empty() {
        return Err(E::Msg(format!("plando entry {s:?} needs both a location and an item")));
    }
    Ok(Entry { location: l.to_string(), item: i.to_string() })
}

fn close(names: &mut dyn Iterator<Item = &str>, want: &str) -> Vec<String> {
    let w: String = want.to_lowercase();
    names.filter(|n| n.to_lowercase().contains(&w) || w.contains(&n.to_lowercase())).take(SUGGEST).map(|n| n.to_string()).collect()
}

fn missing(kind: &str, want: &str, hits: Vec<String>) -> E {
    if hits.is_empty() {
        E::Msg(format!("plando: no {kind} is named {want:?}"))
    } else {
        E::Msg(format!("plando: no {kind} is named {want:?}, did you mean {}?", hits.join(" or ")))
    }
}

pub fn loc_of(w: &World, name: &str) -> Res<LocId> {
    if let Some(l) = w.loc(name) {
        return Ok(l);
    }
    let low: String = name.to_lowercase();
    if let Some(l) = w.real.iter().copied().find(|l| w.locs[*l].name.to_lowercase() == low) {
        return Ok(l);
    }
    let hits: Vec<String> = close(&mut w.real.iter().map(|l| w.locs[*l].name.as_str()), name);
    Err(missing("location", name, hits))
}

pub fn item_of(w: &World, name: &str) -> Res<ItemId> {
    if let Some(i) = w.item(name) {
        return Ok(i);
    }
    let low: String = name.to_lowercase();
    if let Some(i) = (0..w.items.names.len()).find(|i| w.items.names[*i].to_lowercase() == low) {
        return Ok(i);
    }
    let hits: Vec<String> = close(&mut w.items.names.iter().map(|n| n.as_str()), name);
    Err(missing("item", name, hits))
}

pub fn resolve(w: &World, entries: &[Entry]) -> Res<Vec<(LocId, ItemId)>> {
    if entries.len() > MAX_ENTRIES {
        return Err(E::Msg(format!("plando: at most {MAX_ENTRIES} entries are allowed")));
    }
    let mut out: Vec<(LocId, ItemId)> = Vec::with_capacity(entries.len());
    for e in entries {
        let l: LocId = loc_of(w, &e.location)?;
        let i: ItemId = item_of(w, &e.item)?;
        if out.iter().any(|(x, _)| *x == l) {
            return Err(E::Msg(format!("plando: {} is given more than once", w.locs[l].name)));
        }
        out.push((l, i));
    }
    out.sort_by_key(|(l, _)| *l);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_split_on_the_equals_sign_and_trim() {
        let e: Entry = parse(" KF Midos Top Left Chest = Slingshot ").unwrap();
        assert_eq!(e, Entry { location: "KF Midos Top Left Chest".to_string(), item: "Slingshot".to_string() });
        for bad in ["nothing", "=Slingshot", "Place=", " = "] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
}
