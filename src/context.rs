use std::collections::BTreeMap;

use pumpkin_plugin_api::player::Player;
use pumpkin_plugin_api::world::World;

use crate::config::Config;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextSet {
    /// Same key, several values = OR. Different keys = AND.
    pub pairs: BTreeMap<String, Vec<String>>,
}

impl ContextSet {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_pairs(pairs: BTreeMap<String, String>) -> Self {
        let mut set = Self::empty();
        for (k, v) in pairs {
            set.insert(k, v);
        }
        set
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into().to_ascii_lowercase();
        let value = value.into();
        let vals = self.pairs.entry(key).or_default();
        if !vals.iter().any(|v| v == &value) {
            vals.push(value);
            vals.sort();
        }
    }

    /// Player query: one value per key (current world, not a history).
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.pairs
            .insert(key.into().to_ascii_lowercase(), vec![value.into()]);
    }

    pub fn merge(&mut self, other: ContextSet) {
        for (k, values) in other.pairs {
            for v in values {
                self.insert(k.clone(), v);
            }
        }
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
        set.set("server", cfg.server.clone());
        let world = player.get_world();
        set.set("world", world.get_name());
        set.set("dimension", dimension_of(&world));
        set
    }

    pub fn global(cfg: &Config) -> Self {
        let mut set = Self::empty();
        set.set("server", cfg.server.clone());
        set
    }

    pub fn with_world(mut self, world: Option<&str>, dimension: Option<&str>) -> Self {
        if let Some(w) = world.filter(|s| !s.is_empty()) {
            self.set("world", w);
        }
        if let Some(d) = dimension.filter(|s| !s.is_empty()) {
            self.set("dimension", d);
        }
        self
    }

    /// A stored node applies here if every stored key is present on the query
    /// and at least one stored value for that key matches.
    pub fn matches(&self, query: &ContextSet) -> bool {
        self.pairs.iter().all(|(k, required)| {
            let Some(have) = query.pairs.get(k) else {
                return false;
            };
            required.iter().any(|r| have.iter().any(|h| h == r))
        })
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty() || self.pairs.values().all(|v| v.is_empty())
    }

    pub fn display(&self) -> String {
        if self.is_empty() {
            return "global".into();
        }
        let mut parts = Vec::new();
        for (k, values) in &self.pairs {
            for v in values {
                parts.push(format!("{k}={v}"));
            }
        }
        parts.join(" ")
    }
}

fn dimension_of(world: &World) -> String {
    world.get_dimension()
}

impl From<BTreeMap<String, Vec<String>>> for ContextSet {
    fn from(map: BTreeMap<String, Vec<String>>) -> Self {
        let mut pairs = BTreeMap::new();
        for (k, mut values) in map {
            values.retain(|v| !v.is_empty());
            values.sort();
            values.dedup();
            if !values.is_empty() {
                pairs.insert(k.to_ascii_lowercase(), values);
            }
        }
        Self { pairs }
    }
}

impl From<&ContextSet> for BTreeMap<String, Vec<String>> {
    fn from(set: &ContextSet) -> Self {
        set.pairs.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_value_is_or() {
        let mut node = ContextSet::empty();
        node.insert("world", "world");
        node.insert("world", "world_nether");
        let mut overworld = ContextSet::empty();
        overworld.set("world", "world");
        let mut nether = ContextSet::empty();
        nether.set("world", "world_nether");
        let mut end = ContextSet::empty();
        end.set("world", "world_the_end");
        assert!(node.matches(&overworld));
        assert!(node.matches(&nether));
        assert!(!node.matches(&end));
        assert!(!node.matches(&ContextSet::empty()));
    }

    #[test]
    fn empty_node_matches_anything() {
        let node = ContextSet::empty();
        let mut q = ContextSet::empty();
        q.set("world", "world");
        assert!(node.matches(&q));
    }
}
