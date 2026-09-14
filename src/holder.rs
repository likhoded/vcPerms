use serde::{Deserialize, Serialize};

use crate::context::ContextSet;
use crate::node::Node;
use crate::util::now_secs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(alias = "uuid")]
    pub unique_id: String,
    pub name: String,
    #[serde(default = "default_group_name", alias = "primaryGroup")]
    pub primary_group: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
}

fn default_group_name() -> String {
    "default".into()
}

impl User {
    pub fn new(uuid: String, name: String, default_group: &str) -> Self {
        let mut user = Self {
            unique_id: uuid,
            name,
            primary_group: default_group.to_ascii_lowercase(),
            nodes: Vec::new(),
        };
        user.nodes
            .push(Node::group(default_group, &ContextSet::empty(), None));
        user
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    #[serde(default, alias = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
    #[serde(default)]
    pub nodes: Vec<Node>,
}

impl Group {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_ascii_lowercase(),
            display_name: None,
            weight: None,
            nodes: Vec::new(),
        }
    }

    pub fn resolved_weight(&self) -> i32 {
        if let Some(w) = self.weight {
            return w;
        }
        self.nodes
            .iter()
            .filter_map(|n| n.weight_value())
            .max()
            .unwrap_or(0)
    }

    pub fn resolved_display(&self) -> String {
        if let Some(d) = &self.display_name {
            return d.clone();
        }
        self.nodes
            .iter()
            .rev()
            .find_map(|n| n.displayname_value())
            .unwrap_or_else(|| self.name.clone())
    }
}

pub trait Holder {
    fn name(&self) -> &str;
    fn nodes(&self) -> &[Node];
    fn nodes_mut(&mut self) -> &mut Vec<Node>;

    fn add_node(&mut self, node: Node) {
        let nodes = self.nodes_mut();
        if let Some(existing) = nodes.iter_mut().find(|n| n.same_identity(&node)) {
            *existing = node;
        } else {
            nodes.push(node);
        }
    }

    fn remove_node(&mut self, key: &str, ctx: &ContextSet) -> usize {
        let key_l = key.to_ascii_lowercase();
        let nodes = self.nodes_mut();
        let before = nodes.len();
        nodes.retain(|n| {
            if !n.key.eq_ignore_ascii_case(&key_l) {
                return true;
            }
            if ctx.is_empty() {
                // LP: unset without context removes the global node only.
                !n.ctx().is_empty()
            } else {
                n.ctx() != *ctx
            }
        });
        before - nodes.len()
    }

    fn remove_temp(&mut self, key: &str, ctx: &ContextSet) -> usize {
        let key_l = key.to_ascii_lowercase();
        let nodes = self.nodes_mut();
        let before = nodes.len();
        nodes.retain(|n| {
            if n.expiry.is_none() || !n.key.eq_ignore_ascii_case(&key_l) {
                return true;
            }
            if ctx.is_empty() {
                !n.ctx().is_empty()
            } else {
                n.ctx() != *ctx
            }
        });
        before - nodes.len()
    }

    fn clear_matching(&mut self, pred: impl Fn(&Node) -> bool) -> usize {
        let nodes = self.nodes_mut();
        let before = nodes.len();
        nodes.retain(|n| !pred(n));
        before - nodes.len()
    }

    fn parents(&self) -> Vec<String> {
        let now = now_secs();
        let mut names = Vec::new();
        for node in self.nodes() {
            if node.expired(now) {
                continue;
            }
            if let Some(g) = node.group_name() {
                if !node.value {
                    continue;
                }
                if !names.contains(&g) {
                    names.push(g);
                }
            }
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextSet;
    use crate::node::Node;

    #[test]
    fn unset_without_context_keeps_world_nodes() {
        let mut u = User::new("id".into(), "Steve".into(), "default");
        let mut world = ContextSet::empty();
        world.set("world", "nether");
        u.add_node(Node::perm("fly", true, &ContextSet::empty(), None));
        u.add_node(Node::perm("fly", true, &world, None));
        assert_eq!(u.remove_node("fly", &ContextSet::empty()), 1);
        assert_eq!(u.nodes.iter().filter(|n| n.key == "fly").count(), 1);
    }
}

impl Holder for User {
    fn name(&self) -> &str {
        &self.name
    }
    fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    fn nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
    }
}

impl Holder for Group {
    fn name(&self) -> &str {
        &self.name
    }
    fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    fn nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
    }
}
