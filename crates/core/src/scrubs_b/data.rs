use std::sync::OnceLock;

use crate::error::{Error as E, Res};
use crate::json::{self, Json};

pub const JSON: &str = include_str!("../../../../data/scrubs_b.json");

#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    pub file: String,
    pub hash: u64,
    pub talk: String,
    pub attacked: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scrub {
    pub key: String,
    pub step: u32,
    pub location: String,
    pub flag: String,
    pub price: i32,
    pub why: String,
    pub spots: Vec<Spot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Book {
    pub flow: String,
    pub template: String,
    pub label: String,
    pub scrubs: Vec<Scrub>,
}

fn text(j: &Json, k: &str) -> Res<String> {
    j.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| E::Msg(format!("scrubs_b json: missing text field {k}")))
}

fn num(j: &Json, k: &str) -> Res<i64> {
    j.get(k).and_then(|v| v.as_i64()).ok_or_else(|| E::Msg(format!("scrubs_b json: missing number field {k}")))
}

fn spot(j: &Json) -> Res<Spot> {
    let h: String = text(j, "hash")?;
    let hash: u64 = h.parse::<u64>().map_err(|_| E::Msg(format!("scrubs_b json: {h} is not a hash")))?;
    let file: String = text(j, "file")?;
    if !file.starts_with("Banc/") || !file.ends_with(".bcett.byml.zs") {
        return Err(E::Msg(format!("scrubs_b json: {file} is not a Banc file")));
    }
    Ok(Spot { file, hash, talk: text(j, "talk")?, attacked: text(j, "attacked")? })
}

fn scrub(j: &Json) -> Res<Scrub> {
    let key: String = text(j, "key")?;
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(E::Msg(format!("scrubs_b json: {key} is not a plain key")));
    }
    let price: i32 = i32::try_from(num(j, "price")?).map_err(|_| E::Format("scrubs_b json: price out of range"))?;
    if !(1..=999).contains(&price) {
        return Err(E::Msg(format!("scrubs_b json: price {price} of {key} is out of range")));
    }
    let mut spots: Vec<Spot> = Vec::new();
    for s in j.get("placements").and_then(|v| v.as_arr()).ok_or(E::Format("scrubs_b json: missing placements"))? {
        spots.push(spot(s)?);
    }
    if spots.is_empty() {
        return Err(E::Msg(format!("scrubs_b json: {key} has no placement")));
    }
    Ok(Scrub {
        key,
        step: u32::try_from(num(j, "step")?).map_err(|_| E::Format("scrubs_b json: bad step"))?,
        location: text(j, "location")?,
        flag: text(j, "flag")?,
        price,
        why: text(j, "why")?,
        spots,
    })
}

impl Scrub {
    pub fn entry(&self) -> String {
        format!("OOTRR_Scrub_{}", self.key)
    }

    pub fn ready(&self) -> bool {
        !self.location.is_empty() && !self.flag.is_empty()
    }
}

impl Book {
    pub fn parse(t: &str) -> Res<Book> {
        let j: Json = json::parse(t)?;
        if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
            return Err(E::Format("scrubs_b json: unsupported format"));
        }
        let mut scrubs: Vec<Scrub> = Vec::new();
        for x in j.get("scrubs").and_then(|v| v.as_arr()).ok_or(E::Format("scrubs_b json: missing scrubs"))? {
            let s: Scrub = scrub(x)?;
            let twice: bool = scrubs.iter().any(|o| {
                o.key == s.key
                    || (!s.location.is_empty() && o.location == s.location)
                    || (!s.flag.is_empty() && o.flag == s.flag)
                    || o.spots.iter().any(|a| s.spots.iter().any(|b| a.hash == b.hash))
            });
            if twice {
                return Err(E::Msg(format!("scrubs_b json: {} shares a key, location, flag or placement with another scrub", s.key)));
            }
            scrubs.push(s);
        }
        Ok(Book { flow: text(&j, "flow")?, template: text(&j, "template")?, label: text(&j, "label")?, scrubs })
    }

    pub fn by_location(&self, n: &str) -> Option<&Scrub> {
        self.scrubs.iter().find(|s| s.ready() && s.location == n)
    }

    pub fn by_key(&self, k: &str) -> Option<&Scrub> {
        self.scrubs.iter().find(|s| s.key == k)
    }
}

pub fn book() -> Res<&'static Book> {
    static B: OnceLock<Result<Book, String>> = OnceLock::new();
    match B.get_or_init(|| Book::parse(JSON).map_err(|e| e.to_string())) {
        Ok(b) => Ok(b),
        Err(e) => Err(E::Msg(e.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_book_lists_ten_scrubs_and_twelve_placements() {
        let b: &Book = book().unwrap();
        assert_eq!(b.scrubs.len(), 10);
        assert_eq!(b.scrubs.iter().map(|s| s.spots.len()).sum::<usize>(), 12);
        let ready: Vec<&str> = b.scrubs.iter().filter(|s| s.ready()).map(|s| s.location.as_str()).collect();
        assert_eq!(ready, vec!["LW Deku Scrub Grotto Rear", "Jabu Jabus Belly Deku Scrub"]);
        assert!(b.scrubs.iter().all(|s| (10..=70).contains(&s.price)));
        assert!(b.scrubs.windows(2).all(|w| w[0].step <= w[1].step));
        assert_eq!(b.by_key("Jabu").unwrap().entry(), "OOTRR_Scrub_Jabu");
        assert!(b.by_location("").is_none());
    }

    #[test]
    fn bad_books_are_refused() {
        let one = |extra: &str| format!("{{\"format\":1,\"flow\":\"F\",\"template\":\"T\",\"label\":\"L\",\"scrubs\":[{extra}]}}");
        let s = |k: &str, h: &str| format!("{{\"key\":\"{k}\",\"step\":1,\"location\":\"\",\"flag\":\"\",\"price\":30,\"why\":\"w\",\"placements\":[{{\"file\":\"Banc/A/A.bcett.byml.zs\",\"hash\":\"{h}\",\"talk\":\"T\",\"attacked\":\"\"}}]}}");
        assert!(Book::parse(&one(&s("A", "1"))).is_ok());
        assert!(Book::parse(&one(&format!("{},{}", s("A", "1"), s("A", "2")))).is_err());
        assert!(Book::parse(&one(&format!("{},{}", s("A", "1"), s("B", "1")))).is_err());
        assert!(Book::parse(&one(&s("A b", "1"))).is_err());
        assert!(Book::parse(&one(&s("A", "x"))).is_err());
        assert!(Book::parse(&one(&s("A", "1")).replace("\"price\":30", "\"price\":0")).is_err());
        assert!(Book::parse(&one(&s("A", "1")).replace("Banc/A/A.bcett.byml.zs", "A.byml")).is_err());
        assert!(Book::parse("{\"format\":2}").is_err());
    }
}
