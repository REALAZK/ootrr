use ootrr_core::error::{Error, Res};
use ootrr_core::logic_demo::{run_bench, run_demo, run_load};

fn num<T: std::str::FromStr>(a: &[String], i: usize, d: T, what: &'static str) -> Res<T> {
    match a.get(i) {
        Some(s) => s.parse().map_err(|_| Error::Format(what)),
        None => Ok(d),
    }
}

pub fn demo(a: &[String]) -> Res<()> {
    let seed: &str = a.first().map(|s| s.as_str()).unwrap_or("ootrr");
    println!("{}", run_demo(seed)?);
    Ok(())
}

pub fn bench(a: &[String]) -> Res<()> {
    let n: usize = num(a, 0, 1000, "bad location count")?;
    if n > 1_000_000 {
        return Err(Error::Format("location count too large"));
    }
    let seed: u64 = num(a, 1, 1, "bad seed")?;
    let chain: bool = a.get(2).is_some_and(|s| s == "chain");
    println!("{}", run_bench(n, seed, chain)?);
    Ok(())
}

pub fn load(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing data dir"))?;
    let dir: &str = a.get(1).ok_or(Error::Format("missing world folder"))?;
    let mode: &str = a.get(2).map(|s| s.as_str()).unwrap_or("vanilla");
    println!("{}", run_load(root, dir, mode)?);
    Ok(())
}
