use pumpkin_plugin_api::command::{CommandError, CommandSender};
use pumpkin_plugin_api::Server;

use crate::cmd::{default_ctx, msg, need, page_of, parse_page, read_data_file, save, write_export};
use crate::context::ContextSet;
use crate::resolve;
use crate::state::{with_store, with_store_mut, with_verbose};
use crate::util::now_secs;

pub fn info(sender: &CommandSender) {
    let text = with_store(|s| {
        format!(
            "&3vcPerms &f{} \n&7server: &f{} \n&7users: &f{} \n&7groups: &f{} \n&7tracks: &f{} \n&7storage: &fflatfile/json",
            env!("CARGO_PKG_VERSION"),
            s.config.server,
            s.users().count(),
            s.groups().count(),
            s.tracks().count()
        )
    });
    msg(sender, &text);
}

pub fn reload(sender: &CommandSender) {
    with_store_mut(|s| s.reload());
    msg(sender, "&aReloaded config and all holders from disk");
}

pub fn sync(sender: &CommandSender) {
    with_store_mut(|s| s.save_all());
    msg(sender, "&aFlushed permission data to disk");
}

pub fn check(sender: &CommandSender, server: &Server, args: &[String]) -> Result<(), CommandError> {
    let name = need(args, 0, "user")?;
    let node = need(args, 1, "permission")?;
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
            "&7check &f{} &7/ &f{node}\n&7result: {} \n&7matched: &f{} \n&7from: &f{}",
            user.name,
            if r.allowed { "&atrue" } else { "&cfalse" },
            r.node,
            r.source
        )
    });
    msg(sender, &text);
    Ok(())
}

pub fn verbose(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        let on = with_verbose(|v| v.enabled);
        msg(sender, &format!("&7verbose is {}", if on { "&aon" } else { "&coff" }));
        return Ok(());
    }
    match args[0].to_ascii_lowercase().as_str() {
        "on" => {
            with_verbose(|v| {
                v.enabled = true;
                v.filter = None;
            });
            msg(sender, "&aVerbose logging enabled");
        }
        "off" => {
            with_verbose(|v| v.enabled = false);
            msg(sender, "&cVerbose logging disabled");
        }
        "record" => {
            let filter = args.get(1).cloned();
            with_verbose(|v| {
                v.enabled = true;
                v.filter = filter.clone();
                v.clear();
            });
            msg(
                sender,
                &format!(
                    "&aRecording permission checks{}",
                    filter.map(|f| format!(" matching '{f}'")).unwrap_or_default()
                ),
            );
        }
        "paste" | "upload" => {
            let hits = with_verbose(|v| {
                v.enabled = false;
                v.snapshot()
            });
            let dump = serde_json::json!({
                "generated": now_secs(),
                "count": hits.len(),
                "hits": hits,
            });
            let path = write_export(&format!("verbose-{}.json", now_secs()), &dump)?;
            msg(sender, &format!("&aWrote {} check(s) to {path}", hits.len()));
        }
        other => msg(sender, &format!("&cUnknown verbose action '{other}'")),
    }
    Ok(())
}

pub fn tree(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        let lines = with_store(|s| {
            s.group_names()
                .into_iter()
                .flat_map(|g| resolve::tree_lines(s, &g, false))
                .collect::<Vec<_>>()
        });
        msg(sender, "&aInheritance tree");
        for line in lines {
            msg(sender, &line);
        }
        return Ok(());
    }
    let (is_user, name) = if args[0].eq_ignore_ascii_case("user") {
        (true, need(args, 1, "user")?)
    } else if args[0].eq_ignore_ascii_case("group") {
        (false, need(args, 1, "group")?)
    } else {
        (false, args[0].as_str())
    };
    let lines = with_store(|s| resolve::tree_lines(s, name, is_user));
    if lines.is_empty() {
        msg(sender, "&cNothing to show. Check the name.");
    } else {
        for line in lines {
            msg(sender, &line);
        }
    }
    Ok(())
}

pub fn search(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let query = need(args, 0, "query")?;
    let page = parse_page(args, 1);
    let hits = with_store(|s| s.search(query));
    let (page, pages, slice) = page_of(&hits, page, 10);
    msg(sender, &format!("&aSearch '{query}' &7(page {page}/{pages}, {} hits)", hits.len()));
    for (holder, node, value) in slice {
        msg(sender, &format!("&7- &f{holder} &8{node} &7{value}"));
    }
    Ok(())
}

pub fn editor(sender: &CommandSender) -> Result<(), CommandError> {
    let dump = with_store(|s| s.export_dump());
    let path = write_export("editor.json", &dump)?;
    msg(sender, &format!("&aFull editor dump written to {path}"));
    msg(sender, "&7There's no hosted web editor on Pumpkin. Edit the json and /vcp applyedits it.");
    Ok(())
}

pub fn export(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let name = args.first().map(String::as_str).unwrap_or("export.json");
    let dump = with_store(|s| s.export_dump());
    let path = write_export(name, &dump)?;
    msg(sender, &format!("&aExported to {path}"));
    Ok(())
}

pub fn import(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let name = need(args, 0, "file")?;
    let raw = read_data_file(name)?;
    let value: serde_json::Value = serde_json::from_str(&raw).map_err(|e| fail(format!("invalid json: {e}")))?;
    let (u, g, t) = with_store_mut(|s| s.import_dump(value)).map_err(fail)?;
    save();
    msg(sender, &format!("&aImported {u} users, {g} groups, {t} tracks"));
    Ok(())
}

pub fn applyedits(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    import(sender, args)
}

fn fail(e: impl ToString) -> CommandError {
    CommandError::CommandFailed(crate::util::legacy(&format!("&c{}", e.to_string())))
}
