use std::collections::BTreeMap as Map;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum V {
    B(bool),
    I(i64),
    E(u32),
    S(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct State {
    pub v: Map<u32, V>,
    pub npc: Map<String, bool>,
    pub grants: Map<String, i64>,
}

impl State {
    pub fn new() -> State {
        State::default()
    }
}
