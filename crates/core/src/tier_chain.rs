#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Give(usize),
    Ask { owner: usize, yes: usize, no: usize },
}

pub fn base_first(family: &str) -> bool {
    matches!(family, "Bow" | "Slingshot" | "Hookshot")
}

pub fn steps(n: usize, base: bool) -> Vec<Step> {
    let mut out: Vec<Step> = (0..n).map(Step::Give).collect();
    if n < 2 {
        return out;
    }
    let lo: usize = if base { 1 } else { 0 };
    let mut prev: usize = lo;
    for i in lo..n {
        let yes: usize = (i + 1).min(n - 1);
        let no: usize = prev;
        let both_answers_meet: bool = yes == no;
        if both_answers_meet {
            continue;
        }
        out.push(Step::Ask { owner: i, yes, no });
        prev = out.len() - 1;
    }
    if base {
        out.push(Step::Ask { owner: 0, yes: prev, no: 0 });
    }
    out
}

pub fn of(family: &str, n: usize) -> Vec<Step> {
    steps(n, base_first(family))
}

pub fn resolve(s: &[Step], owned: &[bool]) -> Option<usize> {
    let mut at: usize = s.len().checked_sub(1)?;
    for _ in 0..s.len() {
        match s[at] {
            Step::Give(i) => return Some(i),
            Step::Ask { owner, yes, no } => {
                at = if owned.get(owner).copied().unwrap_or(false) { yes } else { no };
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(n: usize) -> Vec<Vec<bool>> {
        (0..1usize << n).map(|m| (0..n).map(|i| m >> i & 1 == 1).collect()).collect()
    }

    #[test]
    fn nothing_owned_gives_the_first_tier() {
        for n in 1..=3 {
            for base in [false, true] {
                assert_eq!(resolve(&steps(n, base), &vec![false; n]), Some(0), "{n} {base}");
            }
        }
    }

    #[test]
    fn owning_tiers_in_order_gives_the_next_one() {
        for n in 2..=3 {
            for base in [false, true] {
                let s: Vec<Step> = steps(n, base);
                for k in 1..n {
                    let owned: Vec<bool> = (0..n).map(|i| i < k).collect();
                    assert_eq!(resolve(&s, &owned), Some(k), "{n} {base} {k}");
                }
                assert_eq!(resolve(&s, &vec![true; n]), Some(n - 1));
            }
        }
    }

    #[test]
    fn a_capacity_chain_never_hands_out_a_tier_below_one_already_owned() {
        for n in 2..=3 {
            let s: Vec<Step> = steps(n, false);
            for owned in all(n) {
                let top: Option<usize> = owned.iter().rposition(|x| *x);
                let got: usize = resolve(&s, &owned).unwrap();
                if let Some(t) = top {
                    assert!(got >= t, "{owned:?} gave {got}");
                    assert_eq!(got, (t + 1).min(n - 1), "{owned:?}");
                }
            }
        }
    }

    #[test]
    fn a_base_chain_gives_the_base_item_first_and_never_lowers_the_upgrades() {
        for n in 2..=3 {
            let s: Vec<Step> = steps(n, true);
            for owned in all(n) {
                let got: usize = resolve(&s, &owned).unwrap();
                if !owned[0] {
                    assert_eq!(got, 0, "{owned:?}");
                    continue;
                }
                let top: usize = owned.iter().rposition(|x| *x).unwrap_or(0);
                assert!(got >= top.max(1).min(n - 1), "{owned:?} gave {got}");
            }
        }
    }

    #[test]
    fn every_step_points_at_an_earlier_step_and_the_root_is_last() {
        for n in 1..=3 {
            for base in [false, true] {
                let s: Vec<Step> = steps(n, base);
                for (i, x) in s.iter().enumerate() {
                    if let Step::Ask { yes, no, owner } = x {
                        assert!(*yes < i && *no < i && *owner < n, "{n} {base} {i}");
                    }
                }
            }
        }
        assert!(base_first("Bow") && base_first("Hookshot") && !base_first("BombBag"));
        assert_eq!(of("BombBag", 3).len(), 6);
        assert_eq!(of("Bow", 3).len(), 6);
        assert_eq!(of("Hookshot", 2).len(), 3);
        assert_eq!(of("Wallet", 2).len(), 4);
        assert_eq!(resolve(&[], &[]), None);
    }
}
