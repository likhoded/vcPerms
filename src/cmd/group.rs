use pumpkin_plugin_api::command::{CommandError, CommandSender};
use pumpkin_plugin_api::Server;

use crate::cmd::{msg, need, page_of, parse_page, parse_temp_mod, save, split_text_and_ctx, usage_group_root, usage_holder, usage_meta, usage_parent, usage_permission};
use crate::config::TempAdd;
use crate::context::ContextSet;
use crate::holder::Holder;
use crate::node::Node;
use crate::state::{with_store, with_store_mut};
use crate::util::{fmt_remaining, now_secs, parse_bool, parse_duration};

pub fn handle(sender: &CommandSender, _server: &Server, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_group_root(sender);
        return Ok(());
    }
    let name = need(args, 0, "group")?;
    if args.len() == 1 {
        return info(sender, name);
    }
    match args[1].to_ascii_lowercase().as_str() {
        "info" => info(sender, name),
        "permission" => permission(sender, name, &args[2..]),
        "parent" => parent(sender, name, &args[2..]),
        "meta" => meta(sender, name, &args[2..]),
        "editor" => editor(sender, name),
        "listmembers" => listmembers(sender, name, parse_page(&args[2..], 0)),
        "setweight" => setweight(sender, name, &args[2..]),
        "setdisplayname" => setdisplayname(sender, name, &args[2..]),
        "showtracks" => showtracks(sender, name),
        "clear" => clear(sender, name, &args[2..]),
        "rename" => {
            let to = need(args, 2, "new name")?;
            with_store_mut(|s| s.rename_group(name, to)).map_err(fail)?;
            save();
            msg(sender, &format!("&aRenamed group {name} -> {to}"));
            Ok(())
        }
        "clone" => {
            let to = need(args, 2, "new name")?;
            with_store_mut(|s| s.clone_group(name, to)).map_err(fail)?;
            save();
            msg(sender, &format!("&aCloned group {name} -> {to}"));
            Ok(())
        }
        other => {
            msg(sender, &format!("&cUnknown group subcommand '{other}'"));
            usage_holder(sender, "group", name);
            Ok(())
        }
    }
}

pub fn create(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let name = need(args, 0, "group")?;
    let created = with_store_mut(|s| s.ensure_group(name));
    save();
    if created {
        msg(sender, &format!("&aCreated group {name}"));
    } else {
        msg(sender, &format!("&eGroup {name} already exists"));
    }
    Ok(())
}

pub fn delete(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let name = need(args, 0, "group")?;
    with_store_mut(|s| s.delete_group(name)).map_err(fail)?;
    save();
    msg(sender, &format!("&aDeleted group {name}"));
    Ok(())
}

pub fn list(sender: &CommandSender) {
    let lines = with_store(|s| {
        let mut groups: Vec<_> = s.groups().collect();
        groups.sort_by_key(|g| std::cmp::Reverse(g.resolved_weight()));
        groups
            .into_iter()
            .map(|g| format!("&7- &f{} &8w={} &7{}", g.name, g.resolved_weight(), g.resolved_display()))
            .collect::<Vec<_>>()
    });
    msg(sender, &format!("&aGroups &7({})", lines.len()));
    for line in lines {
        msg(sender, &line);
    }
}

fn info(sender: &CommandSender, name: &str) -> Result<(), CommandError> {
    let text = with_store(|store| {
        let Some(g) = store.group(name) else {
            return format!("&cgroup '{name}' does not exist");
        };
        format!(
            "&b{} &7({}) \n&7weight: &f{} \n&7parents: &f{} \n&7nodes: &f{}",
            g.resolved_display(),
            g.name,
            g.resolved_weight(),
            g.parents().join(", "),
            g.nodes.len()
        )
    });
    msg(sender, &text);
    Ok(())
}

fn permission(sender: &CommandSender, name: &str, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_permission(sender, "group", name);
        return Ok(());
    }
    if args[0].eq_ignore_ascii_case("info") {
        return perm_info(sender, name, parse_page(args, 1));
    }
    match args[0].to_ascii_lowercase().as_str() {
        "set" => {
            let node = need(args, 1, "node")?;
            let (value, rest) = split_bool(&args[2..]);
            let ctx = ContextSet::parse_trailing(rest).map_err(fail)?;
            set_node(sender, name, node, value, None, ctx, None)
        }
        "unset" => {
            let node = need(args, 1, "node")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, node, ctx, false)
        }
        "settemp" => {
            let node = need(args, 1, "node")?;
            let dur = parse_duration(need(args, 2, "duration")?).map_err(fail)?;
            let (temp_mod, rest) = parse_temp_mod(&args[3..]);
            let (value, rest) = split_bool(rest);
            let ctx = ContextSet::parse_trailing(rest).map_err(fail)?;
            set_node(sender, name, node, value, Some(now_secs() + dur), ctx, temp_mod)
        }
        "unsettemp" => {
            let node = need(args, 1, "node")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, node, ctx, true)
        }
        "clear" => {
            let ctx = ContextSet::parse_trailing(&args[1..]).map_err(fail)?;
            let n = mutate(name, |g| {
                g.clear_matching(|node| {
                    !node.is_group()
                        && node.prefix_parts().is_none()
                        && node.suffix_parts().is_none()
                        && node.meta_parts().is_none()
                        && (ctx.is_empty() || node.ctx() == ctx)
                })
            })?;
            save();
            msg(sender, &format!("&aCleared {n} permission node(s) from {name}"));
            Ok(())
        }
        other => {
            msg(sender, &format!("&cUnknown permission action '{other}'"));
            usage_permission(sender, "group", name);
            Ok(())
        }
    }
}

fn perm_info(sender: &CommandSender, name: &str, page: usize) -> Result<(), CommandError> {
    let now = now_secs();
    let lines = with_store(|store| {
        let Some(g) = store.group(name) else {
            return None;
        };
        Some(
            g.nodes
                .iter()
                .filter(|n| !n.is_group() && n.prefix_parts().is_none() && n.suffix_parts().is_none() && n.meta_parts().is_none())
                .map(|n| format_node(n, now))
                .collect::<Vec<_>>(),
        )
    })
    .ok_or_else(|| fail(format!("group '{name}' does not exist")))?;
    let (page, pages, slice) = page_of(&lines, page, 8);
    msg(sender, &format!("&a{name} permissions &7(page {page}/{pages})"));
    if slice.is_empty() {
        msg(sender, "&7(none)");
    }
    for line in slice {
        msg(sender, line);
    }
    Ok(())
}

fn parent(sender: &CommandSender, name: &str, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_parent(sender, "group", name);
        return Ok(());
    }
    if args[0].eq_ignore_ascii_case("info") {
        let text = with_store(|store| {
            store
                .group(name)
                .map(|g| format!("&a{} parents: &f{}", g.name, g.parents().join(", ")))
                .unwrap_or_else(|| format!("&cgroup '{name}' does not exist"))
        });
        msg(sender, &text);
        return Ok(());
    }
    match args[0].to_ascii_lowercase().as_str() {
        "add" => {
            let parent = need(args, 1, "group")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            if parent.eq_ignore_ascii_case(name) {
                return Err(fail("a group cannot inherit itself"));
            }
            with_store_mut(|store| {
                if store.group(parent).is_none() {
                    return Err(fail(format!("group '{parent}' does not exist")));
                }
                let g = store.group_mut(name).ok_or_else(|| fail(format!("group '{name}' does not exist")))?;
                g.add_node(Node::group(parent, &ctx, None));
                store.touch_group(name);
                Ok(())
            })?;
            save();
            msg(sender, &format!("&a{name} now inherits {parent}"));
            Ok(())
        }
        "remove" => {
            let parent = need(args, 1, "group")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, &format!("group.{parent}"), ctx, false)
        }
        "set" => {
            let parent = need(args, 1, "group")?;
            with_store_mut(|store| {
                if store.group(parent).is_none() {
                    return Err(fail(format!("group '{parent}' does not exist")));
                }
                let g = store.group_mut(name).ok_or_else(|| fail(format!("group '{name}' does not exist")))?;
                g.nodes.retain(|n| !n.is_group());
                g.add_node(Node::group(parent, &ContextSet::empty(), None));
                store.touch_group(name);
                Ok(())
            })?;
            save();
            msg(sender, &format!("&aSet {name}'s parent to {parent}"));
            Ok(())
        }
        "addtemp" => {
            let parent = need(args, 1, "group")?;
            let dur = parse_duration(need(args, 2, "duration")?).map_err(fail)?;
            let ctx = ContextSet::parse_trailing(skip_mod(&args[3..])).map_err(fail)?;
            with_store_mut(|store| {
                if store.group(parent).is_none() {
                    return Err(fail(format!("group '{parent}' does not exist")));
                }
                let g = store.group_mut(name).ok_or_else(|| fail(format!("group '{name}' does not exist")))?;
                g.add_node(Node::group(parent, &ctx, Some(now_secs() + dur)));
                store.touch_group(name);
                Ok(())
            })?;
            save();
            msg(sender, &format!("&a{name} temporarily inherits {parent}"));
            Ok(())
        }
        "removetemp" => {
            let parent = need(args, 1, "group")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, &format!("group.{parent}"), ctx, true)
        }
        "clear" => {
            let ctx = ContextSet::parse_trailing(&args[1..]).map_err(fail)?;
            let n = mutate(name, |g| g.clear_matching(|n| n.is_group() && (ctx.is_empty() || n.ctx() == ctx)))?;
            save();
            msg(sender, &format!("&aCleared {n} parent(s) from {name}"));
            Ok(())
        }
        other => {
            msg(sender, &format!("&cUnknown parent action '{other}'"));
            usage_parent(sender, "group", name);
            Ok(())
        }
    }
}

fn meta(sender: &CommandSender, name: &str, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_meta(sender, "group", name);
        return Ok(());
    }
    if args[0].eq_ignore_ascii_case("info") {
        let now = now_secs();
        let lines = with_store(|store| {
            store.group(name).map(|g| {
                g.nodes
                    .iter()
                    .filter(|n| n.prefix_parts().is_some() || n.suffix_parts().is_some() || n.meta_parts().is_some())
                    .map(|n| format_node(n, now))
                    .collect::<Vec<_>>()
            })
        })
        .ok_or_else(|| fail(format!("group '{name}' does not exist")))?;
        msg(sender, &format!("&a{name} meta"));
        if lines.is_empty() {
            msg(sender, "&7(none)");
        }
        for line in lines {
            msg(sender, &line);
        }
        return Ok(());
    }
    match args[0].to_ascii_lowercase().as_str() {
        "set" => {
            let key = need(args, 1, "key")?;
            let value = need(args, 2, "value")?;
            let ctx = ContextSet::parse_trailing(&args[3..]).map_err(fail)?;
            set_node(sender, name, &format!("meta.{key}.{value}"), true, None, ctx, None)
        }
        "unset" => {
            let key = need(args, 1, "key")?;
            let n = mutate(name, |g| {
                g.clear_matching(|n| n.key.to_ascii_lowercase().starts_with(&format!("meta.{}.", key.to_ascii_lowercase())))
            })?;
            save();
            msg(sender, &format!("&aRemoved {n} meta node(s) from {name}"));
            Ok(())
        }
        "settemp" => {
            let key = need(args, 1, "key")?;
            let value = need(args, 2, "value")?;
            let dur = parse_duration(need(args, 3, "duration")?).map_err(fail)?;
            let ctx = ContextSet::parse_trailing(&args[4..]).map_err(fail)?;
            set_node(sender, name, &format!("meta.{key}.{value}"), true, Some(now_secs() + dur), ctx, None)
        }
        "addprefix" => weighted(sender, name, true, &args[1..], None),
        "addsuffix" => weighted(sender, name, false, &args[1..], None),
        "removeprefix" => remove_weighted(sender, name, true, &args[1..]),
        "removesuffix" => remove_weighted(sender, name, false, &args[1..]),
        "addtempprefix" => {
            let prio = need(args, 1, "priority")?;
            let dur = parse_duration(need(args, 2, "duration")?).map_err(fail)?;
            let (text, rest) = split_text_and_ctx(&args[3..])?;
            let mut rebuilt = vec![prio.to_string(), text];
            rebuilt.extend(rest.iter().cloned());
            weighted(sender, name, true, &rebuilt, Some(now_secs() + dur))
        }
        "addtempsuffix" => {
            let prio = need(args, 1, "priority")?;
            let dur = parse_duration(need(args, 2, "duration")?).map_err(fail)?;
            let (text, rest) = split_text_and_ctx(&args[3..])?;
            let mut rebuilt = vec![prio.to_string(), text];
            rebuilt.extend(rest.iter().cloned());
            weighted(sender, name, false, &rebuilt, Some(now_secs() + dur))
        }
        other => {
            msg(sender, &format!("&cUnknown meta action '{other}'"));
            usage_meta(sender, "group", name);
            Ok(())
        }
    }
}

fn weighted(
    sender: &CommandSender,
    name: &str,
    prefix: bool,
    args: &[String],
    expiry: Option<u64>,
) -> Result<(), CommandError> {
    let prio: i32 = need(args, 0, "priority")?.parse().map_err(|_| fail("priority must be a number"))?;
    let text = need(args, 1, if prefix { "prefix" } else { "suffix" })?;
    let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
    let node = if prefix {
        Node::prefix(prio, text, &ctx, expiry)
    } else {
        Node::suffix(prio, text, &ctx, expiry)
    };
    mutate(name, |g| {
        g.add_node(node);
        1
    })?;
    save();
    msg(sender, &format!("&aUpdated {} for group {name}", if prefix { "prefix" } else { "suffix" }));
    Ok(())
}

fn remove_weighted(sender: &CommandSender, name: &str, prefix: bool, args: &[String]) -> Result<(), CommandError> {
    let prio: i32 = need(args, 0, "priority")?.parse().map_err(|_| fail("priority must be a number"))?;
    let n = mutate(name, |g| {
        g.clear_matching(|node| {
            let parts = if prefix { node.prefix_parts() } else { node.suffix_parts() };
            parts.map(|(p, _)| p == prio).unwrap_or(false)
        })
    })?;
    save();
    msg(sender, &format!("&aRemoved {n} node(s) from {name}"));
    Ok(())
}

fn editor(sender: &CommandSender, name: &str) -> Result<(), CommandError> {
    let dump = with_store(|store| serde_json::to_value(store.group(name)).unwrap_or(serde_json::Value::Null));
    let path = super::write_export(&format!("group-{name}.json"), &dump)?;
    msg(sender, &format!("&aWrote editor snapshot to {path}"));
    Ok(())
}

fn listmembers(sender: &CommandSender, name: &str, page: usize) -> Result<(), CommandError> {
    let members = with_store_mut(|s| s.members_of(name));
    let (page, pages, slice) = page_of(&members, page, 12);
    msg(sender, &format!("&aMembers of {name} &7(page {page}/{pages}, {} total)", members.len()));
    for m in slice {
        msg(sender, &format!("&7- &f{m}"));
    }
    Ok(())
}

fn setweight(sender: &CommandSender, name: &str, args: &[String]) -> Result<(), CommandError> {
    let w: i32 = need(args, 0, "weight")?.parse().map_err(|_| fail("weight must be a number"))?;
    mutate(name, |g| {
        g.weight = Some(w);
        g.nodes.retain(|n| n.weight_value().is_none());
        g.add_node(Node::weight(w));
        1
    })?;
    save();
    msg(sender, &format!("&aSet weight of {name} to {w}"));
    Ok(())
}

fn setdisplayname(sender: &CommandSender, name: &str, args: &[String]) -> Result<(), CommandError> {
    let display = need(args, 0, "display name")?;
    mutate(name, |g| {
        g.display_name = Some(display.to_string());
        g.nodes.retain(|n| n.displayname_value().is_none());
        g.add_node(Node::displayname(display));
        1
    })?;
    save();
    msg(sender, &format!("&aDisplay name for {name} is now {display}"));
    Ok(())
}

fn showtracks(sender: &CommandSender, name: &str) -> Result<(), CommandError> {
    let lines = with_store(|store| {
        store
            .tracks()
            .filter(|t| t.contains(name))
            .map(|t| format!("&7- &f{} &8{}", t.name, t.display()))
            .collect::<Vec<_>>()
    });
    msg(sender, &format!("&aTracks containing {name}"));
    if lines.is_empty() {
        msg(sender, "&7(none)");
    }
    for line in lines {
        msg(sender, &line);
    }
    Ok(())
}

fn clear(sender: &CommandSender, name: &str, args: &[String]) -> Result<(), CommandError> {
    let ctx = ContextSet::parse_trailing(args).map_err(fail)?;
    mutate(name, |g| {
        if ctx.is_empty() {
            let n = g.nodes.len();
            g.nodes.clear();
            n
        } else {
            g.clear_matching(|n| n.ctx() == ctx)
        }
    })?;
    save();
    msg(sender, &format!("&aCleared nodes on {name}"));
    Ok(())
}

fn set_node(
    sender: &CommandSender,
    name: &str,
    key: &str,
    value: bool,
    expiry: Option<u64>,
    ctx: ContextSet,
    temp_mod: Option<TempAdd>,
) -> Result<(), CommandError> {
    with_store_mut(|store| {
        let temp = temp_mod.unwrap_or(store.config.temp_add);
        let g = store
            .group_mut(name)
            .ok_or_else(|| fail(format!("group '{name}' does not exist")))?;
        let node = Node::perm(key, value, &ctx, expiry);
        if expiry.is_some() {
            if let Some(existing) = g.nodes.iter().find(|n| n.same_identity(&node) && n.expiry.is_some()) {
                match temp {
                    TempAdd::Deny => return Err(fail("temporary node already exists")),
                    TempAdd::Accumulate => {
                        let extra = expiry.unwrap_or(0).saturating_sub(now_secs());
                        let merged = existing.expiry.unwrap_or(now_secs()).saturating_add(extra);
                        let mut node = node;
                        node.expiry = Some(merged);
                        g.add_node(node);
                        store.touch_group(name);
                        return Ok(());
                    }
                    TempAdd::Replace | TempAdd::Shadow => {}
                }
            }
        }
        g.add_node(node);
        store.remember(key);
        store.touch_group(name);
        Ok(())
    })?;
    save();
    msg(sender, &format!("&aSet &f{key}&7={} &aon {name}", if value { "true" } else { "false" }));
    Ok(())
}

fn unset_node(
    sender: &CommandSender,
    name: &str,
    key: &str,
    ctx: ContextSet,
    temp_only: bool,
) -> Result<(), CommandError> {
    let n = mutate(name, |g| {
        if temp_only {
            g.remove_temp(key, &ctx)
        } else {
            g.remove_node(key, &ctx)
        }
    })?;
    save();
    msg(sender, &format!("&aRemoved {n} matching node(s) from {name}"));
    Ok(())
}

fn mutate<F>(name: &str, f: F) -> Result<usize, CommandError>
where
    F: FnOnce(&mut crate::holder::Group) -> usize,
{
    with_store_mut(|store| {
        let g = store
            .group_mut(name)
            .ok_or_else(|| fail(format!("group '{name}' does not exist")))?;
        let n = f(g);
        store.touch_group(name);
        Ok(n)
    })
}

fn format_node(node: &Node, now: u64) -> String {
    let val = if node.value { "&atrue" } else { "&cfalse" };
    let mut line = format!("&7- &f{} {val}", node.key);
    if !node.ctx().is_empty() {
        line.push_str(&format!(" &8[{}]", node.ctx().display()));
    }
    if let Some(exp) = node.expiry {
        line.push_str(&format!(" &e{}", fmt_remaining(exp, now)));
    }
    line
}

fn split_bool(args: &[String]) -> (bool, &[String]) {
    if let Some(v) = args.first().and_then(|s| parse_bool(s)) {
        (v, &args[1..])
    } else {
        (true, args)
    }
}

fn skip_mod(args: &[String]) -> &[String] {
    match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("accumulate") | Some("replace") | Some("shadow") | Some("deny") => &args[1..],
        _ => args,
    }
}

fn fail(e: impl ToString) -> CommandError {
    CommandError::CommandFailed(crate::util::chat(&format!("&c{}", e.to_string())))
}
