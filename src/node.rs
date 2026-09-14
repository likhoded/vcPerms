use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::context::ContextSet;
use crate::util::now_secs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub key: String,
    #[serde(default = "default_true")]
    pub value: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, Vec<String>>,
}

fn default_true() -> bool {
    true
}

impl Node {
    pub fn perm(key: impl Into<String>, value: bool, ctx: &ContextSet, expiry: Option<u64>) -> Self {
        Self {
            key: key.into(),
            value,
            expiry,
            context: ctx.into(),
        }
    }

    pub fn group(name: &str, ctx: &ContextSet, expiry: Option<u64>) -> Self {
        Self::perm(format!("group.{}", name.to_ascii_lowercase()), true, ctx, expiry)
    }

    pub fn prefix(priority: i32, text: &str, ctx: &ContextSet, expiry: Option<u64>) -> Self {
        Self::perm(format!("prefix.{priority}.{text}"), true, ctx, expiry)
    }

    pub fn suffix(priority: i32, text: &str, ctx: &ContextSet, expiry: Option<u64>) -> Self {
        Self::perm(format!("suffix.{priority}.{text}"), true, ctx, expiry)
    }

    pub fn meta(key: &str, value: &str, ctx: &ContextSet, expiry: Option<u64>) -> Self {
        Self::perm(format!("meta.{key}.{value}"), true, ctx, expiry)
    }

    pub fn weight(weight: i32) -> Self {
        Self::perm(format!("weight.{weight}"), true, &ContextSet::empty(), None)
    }

    pub fn displayname(name: &str) -> Self {
        Self::perm(format!("displayname.{name}"), true, &ContextSet::empty(), None)
    }

    pub fn ctx(&self) -> ContextSet {
        ContextSet::from(self.context.clone())
    }

    pub fn expired(&self, now: u64) -> bool {
        self.expiry.map(|e| e <= now).unwrap_or(false)
    }

    pub fn same_identity(&self, other: &Node) -> bool {
        self.key.eq_ignore_ascii_case(&other.key)
            && self.ctx() == other.ctx()
            && self.expiry.is_some() == other.expiry.is_some()
    }

    pub fn is_group(&self) -> bool {
        self.key.to_ascii_lowercase().starts_with("group.")
    }

    pub fn group_name(&self) -> Option<String> {
        let key = self.key.to_ascii_lowercase();
        key.strip_prefix("group.").map(|s| s.to_string())
    }

    pub fn prefix_parts(&self) -> Option<(i32, String)> {
        parse_weighted("prefix.", &self.key)
    }

    pub fn suffix_parts(&self) -> Option<(i32, String)> {
        parse_weighted("suffix.", &self.key)
    }

    pub fn meta_parts(&self) -> Option<(String, String)> {
        let key = &self.key;
        let rest = key.strip_prefix("meta.")?;
        let (k, v) = rest.split_once('.')?;
        Some((k.to_string(), v.to_string()))
    }

    pub fn weight_value(&self) -> Option<i32> {
        self.key
            .strip_prefix("weight.")
            .and_then(|s| s.parse().ok())
    }

    pub fn displayname_value(&self) -> Option<String> {
        self.key.strip_prefix("displayname.").map(|s| s.to_string())
    }

    pub fn applies(&self, query: &ContextSet, now: u64) -> bool {
        !self.expired(now) && self.ctx().matches(query)
    }
}

fn parse_weighted(prefix: &str, key: &str) -> Option<(i32, String)> {
    let rest = key.strip_prefix(prefix)?;
    let (prio, text) = rest.split_once('.')?;
    Some((prio.parse().ok()?, text.to_string()))
}

pub fn purge_expired(nodes: &mut Vec<Node>) -> usize {
    let now = now_secs();
    let before = nodes.len();
    nodes.retain(|n| !n.expired(now));
    before - nodes.len()
}

/// How well `node` covers `query`. Higher wins.
pub fn specificity(node_key: &str, query: &str) -> Option<i32> {
    let node = node_key.to_ascii_lowercase();
    let q = query.to_ascii_lowercase();
    if node == "*" {
        return Some(1);
    }
    if node == q {
        return Some(1000 + node.len() as i32);
    }
    if let Some(prefix) = node.strip_suffix(".*") {
        if q == prefix || q.starts_with(&format!("{prefix}.")) || q.starts_with(&format!("{prefix}:"))
        {
            return Some(100 + prefix.len() as i32);
        }
    }
    if let Some(prefix) = node.strip_suffix(":*") {
        if q == prefix || q.starts_with(&format!("{prefix}:")) {
            return Some(100 + prefix.len() as i32);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_covers_colon_host_nodes() {
        assert_eq!(specificity("vcedit.*", "vcedit:command").unwrap() > 1, true);
        assert!(specificity("vcperms:*", "vcperms:command").is_some());
        assert!(specificity("vcedit.*", "vcedit.wand").is_some());
        assert!(specificity("foo.*", "bar.baz").is_none());
        assert!(specificity("build", "build").is_some());
        assert!(specificity("*", "anything").is_some());
    }

    #[test]
    fn temp_and_permanent_are_distinct() {
        let a = Node::perm("x.y", true, &crate::context::ContextSet::empty(), None);
        let b = Node::perm("x.y", true, &crate::context::ContextSet::empty(), Some(9));
        assert!(!a.same_identity(&b));
        assert!(a.same_identity(&Node::perm("x.y", false, &crate::context::ContextSet::empty(), None)));
    }
}
