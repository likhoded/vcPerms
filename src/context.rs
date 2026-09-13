use std::collections::BTreeMap;

use pumpkin_plugin_api::player::Player;

use crate::config::Config;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextSet {
    pub pairs: BTreeMap<String, String>,
}

impl ContextSet {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_pairs(pairs: BTreeMap<String, String>) -> Self {
        Self { pairs }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.pairs.insert(key.into().to_ascii_lowercase(), value.into());
    }

    pub fn parse_trailing(args: &[String]) -> Result<Self, String> {
        let mut set = Self::empty();
        for raw in args {
            let Some((k, v)) = raw.split_once('=') else {
                return Err(format!("expected context as key=value, got '{raw}'"));
            };
            if k.is_empty() || v.is_empty() {
                return Err(format!("empty context in '{raw}'"));
            }
            set.insert(k, v);
        }
        Ok(set)
    }

    pub fn for_player(player: &Player, cfg: &Config) -> Self {
        let mut set = Self::empty();
        set.insert("server", cfg.server.clone());
        set.insert("world", player.get_world().get_name());
        set
    }

    pub fn global(cfg: &Config) -> Self {
        let mut set = Self::empty();
        set.insert("server", cfg.server.clone());
        set
    }

    /// A stored node applies here if every stored pair is present on the query.
    pub fn matches(&self, query: &ContextSet) -> bool {
        self.pairs
            .iter()
            .all(|(k, v)| query.pairs.get(k).map(|qv| qv == v).unwrap_or(false))
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn display(&self) -> String {
        if self.pairs.is_empty() {
            return "global".into();
        }
        self.pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<BTreeMap<String, Vec<String>>> for ContextSet {
    fn from(map: BTreeMap<String, Vec<String>>) -> Self {
        let mut pairs = BTreeMap::new();
        for (k, values) in map {
            if let Some(first) = values.into_iter().next() {
                pairs.insert(k.to_ascii_lowercase(), first);
            }
        }
        Self { pairs }
    }
}

impl From<&ContextSet> for BTreeMap<String, Vec<String>> {
    fn from(set: &ContextSet) -> Self {
        set.pairs
            .iter()
            .map(|(k, v)| (k.clone(), vec![v.clone()]))
            .collect()
    }
}
