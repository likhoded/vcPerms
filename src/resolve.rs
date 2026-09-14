use crate::context::ContextSet;
use crate::holder::{Group, Holder, User};
use crate::node::{specificity, Node};
use crate::store::Store;
use crate::util::now_secs;

#[derive(Clone, Debug)]
pub struct CheckResult {
    pub allowed: bool,
    pub source: String,
    pub node: String,
    pub negated: bool,
}

#[derive(Clone, Debug)]
struct Candidate {
    node: Node,
    source: String,
    weight: i32,
    inherited: bool,
}

pub fn check(store: &Store, user: &User, permission: &str, ctx: &ContextSet) -> CheckResult {
    let now = now_secs();
    let mut seen = Vec::new();
    let mut cands = Vec::new();
    collect_user(store, user, ctx, now, &mut seen, &mut cands);

    let mut best: Option<(i32, i32, i32, i32, Candidate)> = None;
    for cand in cands {
        let Some(spec) = specificity(&cand.node.key, permission) else {
            continue;
        };
        let own = if cand.inherited { 0 } else { 1 };
        let temp = if cand.node.expiry.is_some() { 1 } else { 0 };
        let rank = (spec, own, temp, cand.weight);
        let take = match &best {
            None => true,
            Some((s, o, t, w, _)) => rank > (*s, *o, *t, *w),
        };
        if take {
            best = Some((rank.0, rank.1, rank.2, rank.3, cand));
        }
    }

    match best {
        Some((_, _, _, _, cand)) => CheckResult {
            allowed: cand.node.value,
            source: cand.source,
            node: cand.node.key,
            negated: !cand.node.value,
        },
        None => CheckResult {
            allowed: false,
            source: "none".into(),
            node: permission.to_string(),
            negated: false,
        },
    }
}

fn collect_user(
    store: &Store,
    user: &User,
    ctx: &ContextSet,
    now: u64,
    seen: &mut Vec<String>,
    out: &mut Vec<Candidate>,
) {
    for node in user.nodes() {
        if !node.applies(ctx, now) {
            continue;
        }
        if let Some(g) = node.group_name() {
            out.push(Candidate {
                node: node.clone(),
                source: format!("user/{}", user.name),
                weight: 0,
                inherited: false,
            });
            if node.value {
                collect_group(store, &g, ctx, now, seen, out);
            }
        } else {
            out.push(Candidate {
                node: node.clone(),
                source: format!("user/{}", user.name),
                weight: 0,
                inherited: false,
            });
        }
    }
}

fn collect_group(
    store: &Store,
    name: &str,
    ctx: &ContextSet,
    now: u64,
    seen: &mut Vec<String>,
    out: &mut Vec<Candidate>,
) {
    let key = name.to_ascii_lowercase();
    if seen.iter().any(|s| s == &key) {
        return;
    }
    seen.push(key.clone());
    let Some(group) = store.group(&key) else {
        return;
    };
    let weight = group.resolved_weight();
    for node in group.nodes() {
        if !node.applies(ctx, now) {
            continue;
        }
        if let Some(parent) = node.group_name() {
            out.push(Candidate {
                node: node.clone(),
                source: format!("group/{}", group.name),
                weight,
                inherited: true,
            });
            if node.value {
                collect_group(store, &parent, ctx, now, seen, out);
            }
        } else {
            out.push(Candidate {
                node: node.clone(),
                source: format!("group/{}", group.name),
                weight,
                inherited: true,
            });
        }
    }
}

pub fn inherited_nodes(store: &Store, user: &User, ctx: &ContextSet) -> Vec<(String, Node)> {
    let now = now_secs();
    let mut seen = Vec::new();
    let mut cands = Vec::new();
    collect_user(store, user, ctx, now, &mut seen, &mut cands);
    cands
        .into_iter()
        .map(|c| (c.source, c.node))
        .collect()
}

pub fn prefix_of(store: &Store, user: &User, ctx: &ContextSet) -> Option<(i32, String)> {
    best_weighted(store, user, ctx, true)
}

pub fn suffix_of(store: &Store, user: &User, ctx: &ContextSet) -> Option<(i32, String)> {
    best_weighted(store, user, ctx, false)
}

fn best_weighted(store: &Store, user: &User, ctx: &ContextSet, prefix: bool) -> Option<(i32, String)> {
    let mut best: Option<(i32, String)> = None;
    for (_, node) in inherited_nodes(store, user, ctx) {
        let parts = if prefix {
            node.prefix_parts()
        } else {
            node.suffix_parts()
        };
        if let Some((prio, text)) = parts {
            if best.as_ref().map(|(p, _)| prio > *p).unwrap_or(true) {
                best = Some((prio, text));
            }
        }
    }
    best
}

pub fn meta_of(store: &Store, user: &User, key: &str, ctx: &ContextSet) -> Option<String> {
    let key_l = key.to_ascii_lowercase();
    for (_, node) in inherited_nodes(store, user, ctx) {
        if let Some((k, v)) = node.meta_parts() {
            if k.eq_ignore_ascii_case(&key_l) {
                return Some(v);
            }
        }
    }
    None
}

/// Highest-weight parent, or the configured default group.
pub fn primary_from_parents(store: &Store, parents: &[String]) -> String {
    let mut best: Option<(i32, String)> = None;
    for name in parents {
        let Some(group) = store.group(name) else {
            continue;
        };
        let w = group.resolved_weight();
        let take = match &best {
            None => true,
            Some((bw, _)) => w > *bw,
        };
        if take {
            best = Some((w, group.name.clone()));
        }
    }
    best.map(|(_, n)| n)
        .unwrap_or_else(|| store.config.default_group.clone())
}

pub fn tree_lines(store: &Store, start: &str, is_user: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut seen = Vec::new();
    if is_user {
        if let Some(user) = store.user_by_name(start).cloned() {
            lines.push(format!("user {}", user.name));
            walk_parents(store, &user.parents(), 1, &mut seen, &mut lines);
        }
    } else if let Some(group) = store.group(start) {
        walk_group(store, group, 0, &mut seen, &mut lines);
    }
    lines
}

fn walk_parents(
    store: &Store,
    parents: &[String],
    depth: usize,
    seen: &mut Vec<String>,
    lines: &mut Vec<String>,
) {
    for name in parents {
        if let Some(g) = store.group(name) {
            walk_group(store, g, depth, seen, lines);
        } else {
            lines.push(format!("{}- {name} &8(missing)", "  ".repeat(depth)));
        }
    }
}

fn walk_group(
    store: &Store,
    group: &Group,
    depth: usize,
    seen: &mut Vec<String>,
    lines: &mut Vec<String>,
) {
    let pad = "  ".repeat(depth);
    if seen.iter().any(|s| s == &group.name) {
        lines.push(format!("{pad}- {} &8(cycle)", group.name));
        return;
    }
    seen.push(group.name.clone());
    lines.push(format!(
        "{pad}- {} &8w={} &7{}",
        group.name,
        group.resolved_weight(),
        group.resolved_display()
    ));
    walk_parents(store, &group.parents(), depth + 1, seen, lines);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holder::{Group, Holder, User};
    use crate::node::Node;

    fn store_with(user: User, groups: Vec<Group>) -> Store {
        let mut store = Store::test_empty();
        for g in groups {
            store.test_insert_group(g);
        }
        store.test_insert_user(user);
        store
    }

    #[test]
    fn negated_group_does_not_inherit() {
        let mut admin = Group::new("admin");
        admin.add_node(Node::perm("fly", true, &ContextSet::empty(), None));
        let mut user = User::new("u".into(), "Steve".into(), "default");
        user.nodes.clear();
        let mut deny = Node::group("admin", &ContextSet::empty(), None);
        deny.value = false;
        user.add_node(deny);
        let store = store_with(user.clone(), vec![admin]);
        let ctx = ContextSet::empty();
        assert!(!check(&store, &user, "fly", &ctx).allowed);
        let group_check = check(&store, &user, "group.admin", &ctx);
        assert!(!group_check.allowed);
        assert_eq!(group_check.node, "group.admin");
    }

    #[test]
    fn temp_beats_permanent_same_spec() {
        let mut user = User::new("u".into(), "Steve".into(), "default");
        user.nodes.clear();
        user.add_node(Node::perm("fly", false, &ContextSet::empty(), None));
        user.add_node(Node::perm(
            "fly",
            true,
            &ContextSet::empty(),
            Some(now_secs() + 60),
        ));
        let store = store_with(user.clone(), vec![]);
        let r = check(&store, &user, "fly", &ContextSet::empty());
        assert!(r.allowed);
        assert!(r.source.starts_with("user/"));
    }

    #[test]
    fn colon_wildcard_from_dot_star() {
        let mut user = User::new("u".into(), "Steve".into(), "default");
        user.nodes.clear();
        user.add_node(Node::perm("vcedit.*", true, &ContextSet::empty(), None));
        let store = store_with(user.clone(), vec![]);
        assert!(check(&store, &user, "vcedit:command", &ContextSet::empty()).allowed);
    }
}
