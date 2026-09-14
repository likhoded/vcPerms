use pumpkin_plugin_api::command::{CommandError, CommandSender};
use pumpkin_plugin_api::Server;

use crate::cmd::{default_ctx, msg, need, page_of, parse_page, save, usage_holder, usage_meta, usage_parent, usage_permission, usage_user_root};
use crate::config::TempAdd;
use crate::context::ContextSet;
use crate::holder::Holder;
use crate::node::Node;
use crate::resolve;
use crate::state::{with_store, with_store_mut};
use crate::util::{fmt_remaining, now_secs, parse_bool, parse_duration};

pub fn handle(sender: &CommandSender, server: &Server, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_user_root(sender);
        return Ok(());
    }
    let name = need(args, 0, "user")?;
    if args.len() == 1 {
        return info(sender, server, name);
    }
    match args[1].to_ascii_lowercase().as_str() {
        "info" => info(sender, server, name),
        "permission" => permission(sender, server, name, &args[2..]),
        "parent" => parent(sender, server, name, &args[2..]),
        "meta" => meta(sender, server, name, &args[2..]),
        "editor" => editor(sender, name),
        "promote" => shift(sender, name, &args[2..], true),
        "demote" => shift(sender, name, &args[2..], false),
        "showtracks" => showtracks(sender, name),
        "clear" => clear(sender, name, &args[2..]),
        "clone" => clone(sender, name, &args[2..]),
        other => {
            msg(sender, &format!("&cUnknown user subcommand '{other}'"));
            usage_holder(sender, "user", name);
            Ok(())
        }
    }
}

pub fn list(sender: &CommandSender, args: &[String]) {
    let names = with_store(|s| s.user_names());
    let page = parse_page(args, 0);
    let (page, pages, slice) = page_of(&names, page, 12);
    msg(sender, &format!("&aUsers &7(page {page}/{pages}, {} total)", names.len()));
    for name in slice {
        msg(sender, &format!("&7- &f{name}"));
    }
}

fn info(sender: &CommandSender, server: &Server, name: &str) -> Result<(), CommandError> {
    let ctx = default_ctx(server, name);
    let text = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let Some(user) = store.user(&id) else {
            return "&cuser missing after create".to_string();
        };
        let mut out = format!(
            "&b{} &7({}) \n&7primary: &f{} \n&7parents: &f{}",
            user.name,
            user.unique_id,
            user.primary_group,
            user.parents().join(", ")
        );
        if let Some((p, t)) = resolve::prefix_of(store, user, &ctx) {
            out.push_str(&format!("\n&7prefix: &f{t} &8({p})"));
        }
        if let Some((p, t)) = resolve::suffix_of(store, user, &ctx) {
            out.push_str(&format!("\n&7suffix: &f{t} &8({p})"));
        }
        out.push_str(&format!("\n&7own nodes: &f{}", user.nodes.len()));
        out
    });
    save();
    msg(sender, &text);
    Ok(())
}

fn permission(sender: &CommandSender, server: &Server, name: &str, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_permission(sender, "user", name);
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
            set_node(sender, name, node, value, None, ctx)
        }
        "unset" => {
            let node = need(args, 1, "node")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, node, ctx, false)
        }
        "settemp" => {
            let node = need(args, 1, "node")?;
            let dur = parse_duration(need(args, 2, "duration")?).map_err(fail)?;
            let (value, rest) = split_bool_skip_mod(&args[3..]);
            let ctx = ContextSet::parse_trailing(rest).map_err(fail)?;
            set_node(sender, name, node, value, Some(now_secs() + dur), ctx)
        }
        "unsettemp" => {
            let node = need(args, 1, "node")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, node, ctx, true)
        }
        "check" => {
            let node = need(args, 1, "node")?;
            let extra = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            let mut ctx = default_ctx(server, name);
            for (k, v) in extra.pairs {
                ctx.insert(k, v);
            }
            let text = with_store_mut(|store| {
                let id = store.ensure_user(name, None);
                let user = store.user(&id).unwrap();
                let r = resolve::check(store, user, node, &ctx);
                format!(
                    "&7{} has &f{node} &7= {} &8({} via {})",
                    user.name,
                    if r.allowed { "&atrue" } else { "&cfalse" },
                    r.node,
                    r.source
                )
            });
            msg(sender, &text);
            Ok(())
        }
        "clear" => {
            let ctx = ContextSet::parse_trailing(&args[1..]).map_err(fail)?;
            let n = with_store_mut(|store| {
                let id = store.ensure_user(name, None);
                let user = store.user_mut(&id).unwrap();
                let removed = user.clear_matching(|node| {
                    !node.is_group()
                        && node.prefix_parts().is_none()
                        && node.suffix_parts().is_none()
                        && node.meta_parts().is_none()
                        && (ctx.is_empty() || node.ctx() == ctx)
                });
                store.mark_dirty();
                removed
            });
            save();
            msg(sender, &format!("&aCleared {n} permission node(s) from {name}"));
            Ok(())
        }
        other => {
            msg(sender, &format!("&cUnknown permission action '{other}'"));
            usage_permission(sender, "user", name);
            Ok(())
        }
    }
}

fn perm_info(sender: &CommandSender, name: &str, page: usize) -> Result<(), CommandError> {
    let now = now_secs();
    let (lines, total) = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let user = store.user(&id).unwrap();
        let lines: Vec<String> = user
            .nodes
            .iter()
            .filter(|n| !n.is_group() && n.prefix_parts().is_none() && n.suffix_parts().is_none() && n.meta_parts().is_none())
            .map(|n| format_node(n, now))
            .collect();
        (lines, user.nodes.len())
    });
    let (page, pages, slice) = page_of(&lines, page, 8);
    msg(sender, &format!("&a{name} permissions &7(page {page}/{pages}, {total} total nodes)"));
    if slice.is_empty() {
        msg(sender, "&7(none)");
    }
    for line in slice {
        msg(sender, line);
    }
    Ok(())
}

fn parent(sender: &CommandSender, _server: &Server, name: &str, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_parent(sender, "user", name);
        return Ok(());
    }
    if args[0].eq_ignore_ascii_case("info") {
        let text = with_store_mut(|store| {
            let id = store.ensure_user(name, None);
            let user = store.user(&id).unwrap();
            format!("&a{} parents: &f{}", user.name, user.parents().join(", "))
        });
        msg(sender, &text);
        return Ok(());
    }
    match args[0].to_ascii_lowercase().as_str() {
        "add" => {
            let group = need(args, 1, "group")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            add_parent(sender, name, group, None, ctx)
        }
        "remove" => {
            let group = need(args, 1, "group")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, &format!("group.{group}"), ctx, false)
        }
        "set" => {
            let group = need(args, 1, "group")?;
            with_store_mut(|store| {
                if store.group(group).is_none() {
                    return Err(fail(format!("group '{group}' does not exist")));
                }
                let id = store.ensure_user(name, None);
                let user = store.user_mut(&id).unwrap();
                user.nodes.retain(|n| !n.is_group());
                user.add_node(Node::group(group, &ContextSet::empty(), None));
                user.primary_group = group.to_ascii_lowercase();
                store.mark_dirty();
                Ok(())
            })?;
            save();
            msg(sender, &format!("&aSet {name}'s parents to {group}"));
            Ok(())
        }
        "addtemp" => {
            let group = need(args, 1, "group")?;
            let dur = parse_duration(need(args, 2, "duration")?).map_err(fail)?;
            let rest = skip_mod(&args[3..]);
            let ctx = ContextSet::parse_trailing(rest).map_err(fail)?;
            add_parent(sender, name, group, Some(now_secs() + dur), ctx)
        }
        "removetemp" => {
            let group = need(args, 1, "group")?;
            let ctx = ContextSet::parse_trailing(&args[2..]).map_err(fail)?;
            unset_node(sender, name, &format!("group.{group}"), ctx, true)
        }
        "clear" => {
            let ctx = ContextSet::parse_trailing(&args[1..]).map_err(fail)?;
            let n = with_store_mut(|store| {
                let id = store.ensure_user(name, None);
                let def = store.config.default_group.clone();
                let user = store.user_mut(&id).unwrap();
                let removed = user.clear_matching(|n| n.is_group() && (ctx.is_empty() || n.ctx() == ctx));
                if user.parents().is_empty() {
                    user.add_node(Node::group(&def, &ContextSet::empty(), None));
                    user.primary_group = def;
                }
                store.mark_dirty();
                removed
            });
            save();
            msg(sender, &format!("&aCleared {n} parent(s) from {name}"));
            Ok(())
        }
        "cleartrack" => {
            let track_name = need(args, 1, "track")?;
            let n = with_store_mut(|store| {
                let groups = store.track(track_name).map(|t| t.groups.clone()).unwrap_or_default();
                let id = store.ensure_user(name, None);
                let user = store.user_mut(&id).unwrap();
                let removed = user.clear_matching(|n| n.group_name().map(|g| groups.contains(&g)).unwrap_or(false));
                store.mark_dirty();
                removed
            });
            save();
            msg(sender, &format!("&aRemoved {n} track parent(s) from {name}"));
            Ok(())
        }
        "switchprimarygroup" => {
            let group = need(args, 1, "group")?;
            with_store_mut(|store| {
                let id = store.ensure_user(name, None);
                if let Some(user) = store.user_mut(&id) {
                    user.primary_group = group.to_ascii_lowercase();
                    store.mark_dirty();
                }
            });
            save();
            msg(sender, &format!("&aPrimary group for {name} is now {group}"));
            Ok(())
        }
        "settrack" => {
            let track_name = need(args, 1, "track")?;
            let group = need(args, 2, "group")?;
            with_store_mut(|store| {
                let groups = store
                    .track(track_name)
                    .map(|t| t.groups.clone())
                    .ok_or_else(|| fail(format!("track '{track_name}' does not exist")))?;
                if !groups.iter().any(|g| g == &group.to_ascii_lowercase()) {
                    return Err(fail(format!("{group} is not on track {track_name}")));
                }
                let id = store.ensure_user(name, None);
                let user = store.user_mut(&id).unwrap();
                user.nodes.retain(|n| n.group_name().map(|g| !groups.contains(&g)).unwrap_or(true));
                user.add_node(Node::group(group, &ContextSet::empty(), None));
                user.primary_group = group.to_ascii_lowercase();
                store.mark_dirty();
                Ok(())
            })?;
            save();
            msg(sender, &format!("&aMoved {name} to {group} on {track_name}"));
            Ok(())
        }
        other => {
            msg(sender, &format!("&cUnknown parent action '{other}'"));
            usage_parent(sender, "user", name);
            Ok(())
        }
    }
}

fn meta(sender: &CommandSender, _server: &Server, name: &str, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        usage_meta(sender, "user", name);
        return Ok(());
    }
    if args[0].eq_ignore_ascii_case("info") {
        return meta_info(sender, name);
    }
    match args[0].to_ascii_lowercase().as_str() {
        "set" => {
            let key = need(args, 1, "key")?;
            let value = need(args, 2, "value")?;
            let ctx = ContextSet::parse_trailing(&args[3..]).map_err(fail)?;
            set_node(sender, name, &format!("meta.{key}.{value}"), true, None, ctx)
        }
        "unset" => {
            let key = need(args, 1, "key")?;
            unset_prefix(sender, name, &format!("meta.{key}."), false)
        }
        "settemp" => {
            let key = need(args, 1, "key")?;
            let value = need(args, 2, "value")?;
            let dur = parse_duration(need(args, 3, "duration")?).map_err(fail)?;
            let ctx = ContextSet::parse_trailing(&args[4..]).map_err(fail)?;
            set_node(sender, name, &format!("meta.{key}.{value}"), true, Some(now_secs() + dur), ctx)
        }
        "unsettemp" => {
            let key = need(args, 1, "key")?;
            unset_prefix(sender, name, &format!("meta.{key}."), true)
        }
        "addprefix" => add_weighted(sender, name, true, &args[1..], None),
        "addsuffix" => add_weighted(sender, name, false, &args[1..], None),
        "removeprefix" => remove_weighted(sender, name, true, &args[1..]),
        "removesuffix" => remove_weighted(sender, name, false, &args[1..]),
        "addtempprefix" => {
            let prio = need(args, 1, "priority")?;
            let text = need(args, 2, "prefix")?;
            let dur = parse_duration(need(args, 3, "duration")?).map_err(fail)?;
            let mut rest = vec![prio.to_string(), text.to_string()];
            rest.extend(args.iter().skip(4).cloned());
            add_weighted(sender, name, true, &rest, Some(now_secs() + dur))
        }
        "addtempsuffix" => {
            let prio = need(args, 1, "priority")?;
            let text = need(args, 2, "suffix")?;
            let dur = parse_duration(need(args, 3, "duration")?).map_err(fail)?;
            let mut rest = vec![prio.to_string(), text.to_string()];
            rest.extend(args.iter().skip(4).cloned());
            add_weighted(sender, name, false, &rest, Some(now_secs() + dur))
        }
        other => {
            msg(sender, &format!("&cUnknown meta action '{other}'"));
            usage_meta(sender, "user", name);
            Ok(())
        }
    }
}

fn meta_info(sender: &CommandSender, name: &str) -> Result<(), CommandError> {
    let now = now_secs();
    let lines = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let user = store.user(&id).unwrap();
        user.nodes
            .iter()
            .filter(|n| n.prefix_parts().is_some() || n.suffix_parts().is_some() || n.meta_parts().is_some())
            .map(|n| format_node(n, now))
            .collect::<Vec<_>>()
    });
    msg(sender, &format!("&a{name} meta"));
    if lines.is_empty() {
        msg(sender, "&7(none)");
    }
    for line in lines {
        msg(sender, &line);
    }
    Ok(())
}

fn add_weighted(
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
    with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        store.user_mut(&id).unwrap().add_node(node);
        store.mark_dirty();
    });
    save();
    msg(sender, &format!("&aUpdated {} for {name}", if prefix { "prefix" } else { "suffix" }));
    Ok(())
}

fn remove_weighted(sender: &CommandSender, name: &str, prefix: bool, args: &[String]) -> Result<(), CommandError> {
    let prio: i32 = need(args, 0, "priority")?.parse().map_err(|_| fail("priority must be a number"))?;
    let kind = if prefix { "prefix" } else { "suffix" };
    let n = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let user = store.user_mut(&id).unwrap();
        let removed = user.clear_matching(|node| {
            let parts = if prefix { node.prefix_parts() } else { node.suffix_parts() };
            parts.map(|(p, _)| p == prio).unwrap_or(false)
        });
        store.mark_dirty();
        removed
    });
    save();
    msg(sender, &format!("&aRemoved {n} {kind} node(s) at {prio} from {name}"));
    Ok(())
}

fn editor(sender: &CommandSender, name: &str) -> Result<(), CommandError> {
    let dump = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        serde_json::to_value(store.user(&id)).unwrap_or(serde_json::Value::Null)
    });
    let path = super::write_export(&format!("user-{name}.json"), &dump)?;
    msg(sender, &format!("&aWrote editor snapshot to {path}"));
    msg(sender, "&7Apply it back with /vcp applyedits <file>");
    Ok(())
}

fn shift(sender: &CommandSender, name: &str, args: &[String], up: bool) -> Result<(), CommandError> {
    let track_name = need(args, 0, "track")?;
    let result = with_store_mut(|store| {
        let track = store
            .track(track_name)
            .cloned()
            .ok_or_else(|| fail(format!("track '{track_name}' does not exist")))?;
        let id = store.ensure_user(name, None);
        let current = {
            let user = store.user(&id).unwrap();
            user.parents()
                .into_iter()
                .find(|g| track.contains(g))
                .or_else(|| track.groups.first().cloned())
        };
        let Some(current) = current else {
            return Err(fail("track is empty"));
        };
        let next = if up { track.next(&current) } else { track.prev(&current) };
        let Some(next) = next else {
            return Err(fail(if up { "already at the top of the track" } else { "already at the bottom of the track" }));
        };
        let user = store.user_mut(&id).unwrap();
        user.nodes.retain(|n| n.group_name().as_deref() != Some(current.as_str()));
        user.add_node(Node::group(&next, &ContextSet::empty(), None));
        user.primary_group = next.clone();
        store.mark_dirty();
        Ok((current, next))
    })?;
    save();
    msg(sender, &format!("&a{} {name}: {} -> {}", if up { "Promoted" } else { "Demoted" }, result.0, result.1));
    Ok(())
}

fn showtracks(sender: &CommandSender, name: &str) -> Result<(), CommandError> {
    let lines = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let parents = store.user(&id).unwrap().parents();
        store
            .tracks()
            .filter(|t| parents.iter().any(|p| t.contains(p)))
            .map(|t| format!("&7- &f{} &8{}", t.name, t.display()))
            .collect::<Vec<_>>()
    });
    msg(sender, &format!("&aTracks for {name}"));
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
    with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let def = store.config.default_group.clone();
        let user = store.user_mut(&id).unwrap();
        if ctx.is_empty() {
            user.nodes.clear();
        } else {
            user.nodes.retain(|n| n.ctx() != ctx);
        }
        if user.parents().is_empty() {
            user.add_node(Node::group(&def, &ContextSet::empty(), None));
            user.primary_group = def;
        }
        store.mark_dirty();
    });
    save();
    msg(sender, &format!("&aCleared nodes for {name}"));
    Ok(())
}

fn clone(sender: &CommandSender, name: &str, args: &[String]) -> Result<(), CommandError> {
    let dest = need(args, 0, "target user")?;
    with_store_mut(|store| store.clone_user(name, dest)).map_err(fail)?;
    save();
    msg(sender, &format!("&aCloned {name} -> {dest}"));
    Ok(())
}

fn set_node(
    sender: &CommandSender,
    name: &str,
    key: &str,
    value: bool,
    expiry: Option<u64>,
    ctx: ContextSet,
) -> Result<(), CommandError> {
    with_store_mut(|store| {
        let temp = store.config.temp_add;
        let id = store.ensure_user(name, None);
        let user = store.user_mut(&id).unwrap();
        let node = Node::perm(key, value, &ctx, expiry);
        if expiry.is_some() {
            if let Some(existing) = user.nodes.iter().find(|n| n.same_identity(&node) && n.expiry.is_some()) {
                match temp {
                    TempAdd::Deny => {
                        return Err(fail("temporary node already exists (temporary-add-behaviour: deny)"));
                    }
                    TempAdd::Accumulate => {
                        let extra = expiry.unwrap_or(0).saturating_sub(now_secs());
                        let merged = existing.expiry.unwrap_or(now_secs()).saturating_add(extra);
                        let mut node = node;
                        node.expiry = Some(merged);
                        user.add_node(node);
                        store.mark_dirty();
                        return Ok(());
                    }
                    TempAdd::Replace => {}
                }
            }
        }
        user.add_node(node);
        store.remember(key);
        store.mark_dirty();
        Ok(())
    })?;
    save();
    msg(sender, &format!("&aSet &f{key}&7={} &aon {name}", if value { "true" } else { "false" }));
    Ok(())
}

fn add_parent(
    sender: &CommandSender,
    name: &str,
    group: &str,
    expiry: Option<u64>,
    ctx: ContextSet,
) -> Result<(), CommandError> {
    with_store_mut(|store| {
        if store.group(group).is_none() {
            return Err(fail(format!("group '{group}' does not exist")));
        }
        let id = store.ensure_user(name, None);
        store.user_mut(&id).unwrap().add_node(Node::group(group, &ctx, expiry));
        store.mark_dirty();
        Ok(())
    })?;
    save();
    msg(sender, &format!("&aAdded {name} to {group}"));
    Ok(())
}

fn unset_node(
    sender: &CommandSender,
    name: &str,
    key: &str,
    ctx: ContextSet,
    temp_only: bool,
) -> Result<(), CommandError> {
    let n = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let user = store.user_mut(&id).unwrap();
        let n = if temp_only {
            user.remove_temp(key, &ctx)
        } else {
            user.remove_node(key, &ctx)
        };
        store.mark_dirty();
        n
    });
    save();
    msg(sender, &format!("&aRemoved {n} matching node(s) from {name}"));
    Ok(())
}

fn unset_prefix(sender: &CommandSender, name: &str, prefix: &str, temp_only: bool) -> Result<(), CommandError> {
    let n = with_store_mut(|store| {
        let id = store.ensure_user(name, None);
        let user = store.user_mut(&id).unwrap();
        let removed = user.clear_matching(|n| {
            n.key.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()) && (!temp_only || n.expiry.is_some())
        });
        store.mark_dirty();
        removed
    });
    save();
    msg(sender, &format!("&aRemoved {n} node(s) from {name}"));
    Ok(())
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

fn split_bool_skip_mod(args: &[String]) -> (bool, &[String]) {
    let args = skip_mod(args);
    split_bool(args)
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
