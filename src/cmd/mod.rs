mod group;
mod misc;
mod track;
pub mod tree;
mod user;

use pumpkin_plugin_api::command::{Arg, CommandError, CommandSender, CommandSuggestion, CommandSuggestions, ConsumedArgs, SuggestionRequest};
use pumpkin_plugin_api::commands::{CommandHandler, CommandSuggestionHandler};
use pumpkin_plugin_api::{Server, player::Player};

use crate::context::ContextSet;
use crate::resolve;
use crate::state::{with_store, with_store_mut};
use crate::util::{chat, player_uuid, tokenize};

pub struct RootHelp;
pub struct Dispatch;
pub struct Suggest;

impl CommandHandler for RootHelp {
    fn handle(&self, sender: CommandSender, _server: Server, _args: ConsumedArgs) -> Result<i32, CommandError> {
        help(&sender);
        Ok(1)
    }
}

impl CommandHandler for Dispatch {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let line = match args.get_value("args") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => String::new(),
        };
        run(&sender, &server, &line)
    }
}

impl CommandSuggestionHandler for Suggest {
    fn suggest(
        &self,
        _sender: CommandSender,
        _server: Server,
        request: SuggestionRequest,
    ) -> CommandSuggestions {
        let trailing = request.remaining.ends_with(' ');
        let tokens = tokenize(&request.remaining);
        let prefix = if trailing {
            String::new()
        } else {
            tokens.last().cloned().unwrap_or_default()
        };
        let options = suggestions(&tokens, trailing);
        let values = options
            .into_iter()
            .filter(|v| {
                prefix.is_empty() || v.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase())
            })
            .map(|value| CommandSuggestion {
                value,
                tooltip: None,
            })
            .collect();
        let token_off = if trailing {
            request.remaining.len()
        } else {
            request
                .remaining
                .rfind(' ')
                .map(|i| i + 1)
                .unwrap_or(0)
        };
        CommandSuggestions {
            start: request.start + token_off as u32,
            length: prefix.len() as u32,
            values,
        }
    }
}

pub(crate) fn run(sender: &CommandSender, server: &Server, line: &str) -> Result<i32, CommandError> {
    let args = tokenize(line);
    if args.is_empty() {
        help(sender);
        return Ok(1);
    }
    let head = args[0].to_ascii_lowercase();
    if !can_run(sender, server, &head, &args) {
        sender.send_message(chat("&cYou don't have permission to do that."));
        return Ok(0);
    }
    match head.as_str() {
        "help" => help(sender),
        "info" => misc::info(sender),
        "reload" => misc::reload(sender),
        "sync" | "networksync" => misc::sync(sender),
        "check" => misc::check(sender, server, &args[1..])?,
        "verbose" => misc::verbose(sender, &args[1..])?,
        "tree" => misc::tree(sender, &args[1..])?,
        "search" => misc::search(sender, &args[1..])?,
        "editor" => misc::editor(sender)?,
        "import" => misc::import(sender, &args[1..])?,
        "export" => misc::export(sender, &args[1..])?,
        "applyedits" => misc::applyedits(sender, &args[1..])?,
        "creategroup" => group::create(sender, &args[1..])?,
        "deletegroup" => group::delete(sender, &args[1..])?,
        "createtrack" => track::create(sender, &args[1..])?,
        "deletetrack" => track::delete(sender, &args[1..])?,
        "listgroups" => group::list(sender),
        "listusers" => user::list(sender, &args[1..]),
        "listtracks" => track::list(sender),
        "user" => user::handle(sender, server, &args[1..])?,
        "group" => group::handle(sender, server, &args[1..])?,
        "track" => track::handle(sender, &args[1..])?,
        other => {
            sender.send_message(chat(&format!("&cUnknown subcommand '{other}'. Try /vcp help")));
        }
    }
    Ok(1)
}

fn help(sender: &CommandSender) {
    sender.send_message(chat(
        "&7Permission manager\n\
         &7/vcp user <user> permission set <node> [true|false]\n\
         &7/vcp group <group> permission set <node> [true|false]\n\
         &7/vcp user <user> parent add <group>\n\
         &7/vcp group <group> parent add <group>\n\
         &7/vcp user <user> meta addprefix <prio> <text>\n\
         &7/vcp track <track> append <group>\n\
         &7/vcp creategroup | deletegroup | listgroups\n\
         &7/vcp check <user> <node>  &8/  &7search <query>\n\
         &7/vcp verbose on|off|record|paste  &8/  &7reload|info",
    ));
}

pub fn usage_holder(sender: &CommandSender, kind: &str, name: &str) {
    if kind == "user" {
        msg(
            sender,
            &format!(
                "&fUser &e{name}\n\
                 &7/vcp user {name} info\n\
                 &7/vcp user {name} permission set|unset|settemp|unsettemp|check|clear|info\n\
                 &7/vcp user {name} parent add|remove|set|addtemp|removetemp|clear|...\n\
                 &7/vcp user {name} meta set|unset|addprefix|addsuffix|...\n\
                 &7/vcp user {name} promote|demote <track>  clone <user>  clear  editor"
            ),
        );
    } else {
        msg(
            sender,
            &format!(
                "&fGroup &e{name}\n\
                 &7/vcp group {name} info\n\
                 &7/vcp group {name} permission set|unset|settemp|unsettemp|clear|info\n\
                 &7/vcp group {name} parent add|remove|set|addtemp|removetemp|clear\n\
                 &7/vcp group {name} meta set|unset|addprefix|addsuffix|...\n\
                 &7/vcp group {name} listmembers|setweight|setdisplayname|rename|clone|clear|editor"
            ),
        );
    }
}

pub fn usage_permission(sender: &CommandSender, kind: &str, name: &str) {
    let check = if kind == "user" {
        format!("&7/vcp {kind} {name} permission check <node> [ctx...]\n")
    } else {
        String::new()
    };
    msg(
        sender,
        &format!(
            "&f{kind} &e{name} &7permission\n\
             &7/vcp {kind} {name} permission info [page]\n\
             &7/vcp {kind} {name} permission set <node> [true|false] [ctx...]\n\
             &7/vcp {kind} {name} permission unset <node> [ctx...]\n\
             &7/vcp {kind} {name} permission settemp <node> <duration> [true|false] [ctx...]\n\
             &7/vcp {kind} {name} permission unsettemp <node> [ctx...]\n\
             {check}&7/vcp {kind} {name} permission clear [ctx...]"
        ),
    );
}

pub fn usage_parent(sender: &CommandSender, kind: &str, name: &str) {
    let extra = if kind == "user" {
        format!(
            "&7/vcp {kind} {name} parent cleartrack <track>\n\
             &7/vcp {kind} {name} parent switchprimarygroup <group>\n\
             &7/vcp {kind} {name} parent settrack <track> <group>\n"
        )
    } else {
        String::new()
    };
    msg(
        sender,
        &format!(
            "&f{kind} &e{name} &7parent\n\
             &7/vcp {kind} {name} parent info\n\
             &7/vcp {kind} {name} parent add <group> [ctx...]\n\
             &7/vcp {kind} {name} parent remove <group> [ctx...]\n\
             &7/vcp {kind} {name} parent set <group>\n\
             &7/vcp {kind} {name} parent addtemp <group> <duration> [ctx...]\n\
             &7/vcp {kind} {name} parent removetemp <group> [ctx...]\n\
             &7/vcp {kind} {name} parent clear [ctx...]\n\
             {extra}"
        ),
    );
}

pub fn usage_meta(sender: &CommandSender, kind: &str, name: &str) {
    msg(
        sender,
        &format!(
            "&f{kind} &e{name} &7meta\n\
             &7/vcp {kind} {name} meta info\n\
             &7/vcp {kind} {name} meta set <key> <value> [ctx...]\n\
             &7/vcp {kind} {name} meta unset <key>\n\
             &7/vcp {kind} {name} meta settemp <key> <value> <duration> [ctx...]\n\
             &7/vcp {kind} {name} meta addprefix <priority> <text>\n\
             &7/vcp {kind} {name} meta addsuffix <priority> <text>\n\
             &7/vcp {kind} {name} meta addtempprefix <priority> <duration> <text>\n\
             &7/vcp {kind} {name} meta removeprefix|removesuffix <priority>"
        ),
    );
}

pub fn usage_group_root(sender: &CommandSender) {
    msg(
        sender,
        "&fGroup\n\
         &7/vcp group <group> info\n\
         &7/vcp group <group> permission set <node> [true|false]\n\
         &7/vcp group <group> parent add <group>\n\
         &7/vcp group <group> meta addprefix <priority> <text>\n\
         &7/vcp group <group> listmembers|setweight|setdisplayname|rename|clone",
    );
}

pub fn usage_user_root(sender: &CommandSender) {
    msg(
        sender,
        "&fUser\n\
         &7/vcp user <user> info\n\
         &7/vcp user <user> permission set <node> [true|false]\n\
         &7/vcp user <user> parent add <group>\n\
         &7/vcp user <user> meta addprefix <priority> <text>\n\
         &7/vcp user <user> promote|demote <track>",
    );
}

fn can_run(sender: &CommandSender, server: &Server, head: &str, args: &[String]) -> bool {
    if sender.is_console() || !sender.is_player() {
        return true;
    }
    let Some(player) = sender.as_player() else {
        return true;
    };
    let needed = command_node(head, args);
    let allowed = with_store_mut(|store| {
        let uuid = player_uuid(&player);
        let user = store.user(&uuid).cloned();
        let ctx = ContextSet::for_player(&player, &store.config);
        if let Some(user) = user.as_ref() {
            let r = resolve::check(store, user, &needed, &ctx);
            if r.allowed {
                return true;
            }
            if resolve::check(store, user, "vcperms.*", &ctx).allowed
                || resolve::check(store, user, "vcperms.admin", &ctx).allowed
                || resolve::check(store, user, "*", &ctx).allowed
            {
                return true;
            }
        }
        let op = crate::util::is_server_op(player.get_permission_level());
        if store.config.commands_allow_ops && op {
            return true;
        }
        if !store.anyone_has_admin() && op {
            return true;
        }
        let _ = server;
        false
    });
    allowed
}

fn command_node(head: &str, args: &[String]) -> String {
    let action = args.get(2).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let verb = args.get(3).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    match head {
        "user" => match action.as_str() {
            "permission" => match verb.as_str() {
                "unset" | "unsettemp" | "clear" => "vcperms.user.permission.unset".into(),
                "check" | "info" => "vcperms.user.permission.check".into(),
                _ => "vcperms.user.permission.set".into(),
            },
            "parent" => match verb.as_str() {
                "remove" | "removetemp" | "clear" | "cleartrack" => {
                    "vcperms.user.parent.remove".into()
                }
                _ => "vcperms.user.parent.add".into(),
            },
            "meta" => match verb.as_str() {
                "unset" | "unsettemp" | "removeprefix" | "removesuffix" => {
                    "vcperms.user.meta.unset".into()
                }
                _ => "vcperms.user.meta.set".into(),
            },
            "promote" => "vcperms.user.promote".into(),
            "demote" => "vcperms.user.demote".into(),
            "info" => "vcperms.user.info".into(),
            _ => "vcperms.user.info".into(),
        },
        "group" => match action.as_str() {
            "permission" => match verb.as_str() {
                "unset" | "unsettemp" | "clear" => "vcperms.group.permission.unset".into(),
                "info" => "vcperms.group.info".into(),
                _ => "vcperms.group.permission.set".into(),
            },
            "parent" => match verb.as_str() {
                "remove" | "removetemp" | "clear" => "vcperms.group.parent.remove".into(),
                _ => "vcperms.group.parent.add".into(),
            },
            "meta" => match verb.as_str() {
                "unset" | "unsettemp" | "removeprefix" | "removesuffix" => {
                    "vcperms.group.meta.unset".into()
                }
                _ => "vcperms.group.meta.set".into(),
            },
            _ => "vcperms.group.info".into(),
        },
        "track" => "vcperms.track.info".into(),
        "creategroup" => "vcperms.creategroup".into(),
        "deletegroup" => "vcperms.deletegroup".into(),
        "createtrack" => "vcperms.createtrack".into(),
        "deletetrack" => "vcperms.deletetrack".into(),
        "verbose" => "vcperms.verbose".into(),
        "import" | "applyedits" => "vcperms.import".into(),
        "export" | "editor" => "vcperms.export".into(),
        other => format!("vcperms.{other}"),
    }
}

pub fn msg(sender: &CommandSender, text: &str) {
    sender.send_message(chat(text));
}

pub fn need<'a>(args: &'a [String], idx: usize, what: &str) -> Result<&'a str, CommandError> {
    args.get(idx)
        .map(String::as_str)
        .ok_or_else(|| CommandError::CommandFailed(chat(&format!("&cMissing {what}"))))
}

pub fn page_of<T>(items: &[T], page: usize, per: usize) -> (usize, usize, &[T]) {
    let pages = items.len().div_ceil(per).max(1);
    let page = page.clamp(1, pages);
    let start = (page - 1) * per;
    let end = (start + per).min(items.len());
    (page, pages, &items[start..end])
}

pub fn parse_page(args: &[String], idx: usize) -> usize {
    args.get(idx)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1)
}

pub fn online_ctx(server: &Server, name: &str) -> Option<(Player, ContextSet)> {
    let player = server.get_player_by_name(name)?;
    let cfg = with_store(|s| s.config.clone());
    let ctx = ContextSet::for_player(&player, &cfg);
    Some((player, ctx))
}

pub fn default_ctx(server: &Server, name: &str) -> ContextSet {
    if let Some((_, ctx)) = online_ctx(server, name) {
        ctx
    } else {
        with_store(|s| ContextSet::global(&s.config))
    }
}

fn suggestions(tokens: &[String], complete: bool) -> Vec<String> {
    let n = if complete { tokens.len() } else { tokens.len().saturating_sub(1) };
    if n == 0 {
        return vec![
            "user", "group", "track", "help", "info", "reload", "sync", "check", "verbose",
            "tree", "search", "editor", "import", "export", "applyedits", "creategroup",
            "deletegroup", "createtrack", "deletetrack", "listgroups", "listusers", "listtracks",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
    }
    match tokens.first().map(|s| s.to_ascii_lowercase()).unwrap_or_default().as_str() {
        "user" if n == 1 => with_store(|s| s.user_names()),
        "user" if n == 2 => vec_s(&[
            "info", "permission", "parent", "meta", "editor", "promote", "demote",
            "showtracks", "clear", "clone",
        ]),
        "user" if n == 3 && tok(tokens, 2) == "permission" => {
            vec_s(&["info", "set", "unset", "settemp", "unsettemp", "check", "clear"])
        }
        "user" if n == 3 && tok(tokens, 2) == "parent" => vec_s(&[
            "info", "add", "remove", "set", "addtemp", "removetemp", "clear", "cleartrack",
            "switchprimarygroup", "settrack",
        ]),
        "user" if n == 3 && tok(tokens, 2) == "meta" => meta_actions(),
        "user" if n == 3 && matches!(tok(tokens, 2).as_str(), "promote" | "demote") => {
            with_store(|s| s.track_names())
        }
        "user" if n == 3 && tok(tokens, 2) == "clone" => with_store(|s| s.user_names()),
        "user" if n == 4 && tok(tokens, 2) == "permission" && matches!(tok(tokens, 3).as_str(), "set" | "unset" | "settemp" | "unsettemp" | "check") => {
            holder_nodes("user", tokens.get(1).map(String::as_str).unwrap_or(""))
        }
        "user" if n == 5 && tok(tokens, 2) == "permission" && tok(tokens, 3) == "set" => {
            vec_s(&["true", "false"])
        }
        "user" if n == 5 && tok(tokens, 2) == "permission" && tok(tokens, 3) == "settemp" => {
            vec_s(&["30s", "15m", "1h", "1d", "7d", "30d"])
        }
        "user" if n == 6 && tok(tokens, 2) == "permission" && tok(tokens, 3) == "settemp" => {
            vec_s(&["true", "false"])
        }
        "user" if n == 4 && tok(tokens, 2) == "parent" && matches!(tok(tokens, 3).as_str(), "add" | "remove" | "set" | "addtemp" | "removetemp" | "switchprimarygroup") => {
            with_store(|s| s.group_names())
        }
        "user" if n == 4 && tok(tokens, 2) == "parent" && matches!(tok(tokens, 3).as_str(), "cleartrack" | "settrack") => {
            with_store(|s| s.track_names())
        }
        "user" if n == 5 && tok(tokens, 2) == "parent" && tok(tokens, 3) == "addtemp" => {
            vec_s(&["30s", "15m", "1h", "1d", "7d", "30d"])
        }
        "user" if n == 5 && tok(tokens, 2) == "parent" && tok(tokens, 3) == "settrack" => {
            with_store(|s| s.group_names())
        }
        "group" if n == 1 => with_store(|s| s.group_names()),
        "group" if n == 2 => vec_s(&[
            "info", "permission", "parent", "meta", "editor", "listmembers", "setweight",
            "setdisplayname", "showtracks", "clear", "rename", "clone",
        ]),
        "group" if n == 3 && tok(tokens, 2) == "permission" => {
            vec_s(&["info", "set", "unset", "settemp", "unsettemp", "clear"])
        }
        "group" if n == 3 && tok(tokens, 2) == "parent" => vec_s(&[
            "info", "add", "remove", "set", "addtemp", "removetemp", "clear",
        ]),
        "group" if n == 3 && tok(tokens, 2) == "meta" => meta_actions(),
        "group" if n == 3 && matches!(tok(tokens, 2).as_str(), "rename" | "clone") => with_store(|s| s.group_names()),
        "group" if n == 4 && tok(tokens, 2) == "permission" && matches!(tok(tokens, 3).as_str(), "set" | "unset" | "settemp" | "unsettemp") => {
            holder_nodes("group", tokens.get(1).map(String::as_str).unwrap_or(""))
        }
        "group" if n == 5 && tok(tokens, 2) == "permission" && tok(tokens, 3) == "set" => {
            vec_s(&["true", "false"])
        }
        "group" if n == 5 && tok(tokens, 2) == "permission" && tok(tokens, 3) == "settemp" => {
            vec_s(&["30s", "15m", "1h", "1d", "7d", "30d"])
        }
        "group" if n == 6 && tok(tokens, 2) == "permission" && tok(tokens, 3) == "settemp" => {
            vec_s(&["true", "false"])
        }
        "group" if n == 4 && tok(tokens, 2) == "parent" && matches!(tok(tokens, 3).as_str(), "add" | "remove" | "set" | "addtemp" | "removetemp") => {
            with_store(|s| s.group_names())
        }
        "group" if n == 5 && tok(tokens, 2) == "parent" && tok(tokens, 3) == "addtemp" => {
            vec_s(&["30s", "15m", "1h", "1d", "7d", "30d"])
        }
        "track" if n == 1 => with_store(|s| s.track_names()),
        "track" if n == 2 => vec_s(&["info", "append", "insert", "remove", "clear", "rename", "clone", "editor"]),
        "track" if n == 3 && matches!(tok(tokens, 2).as_str(), "append" | "insert" | "remove") => {
            with_store(|s| s.group_names())
        }
        "verbose" if n == 1 => vec_s(&["on", "off", "record", "paste"]),
        "tree" if n == 1 => vec_s(&["user", "group"]),
        "tree" if n == 2 && tok(tokens, 1) == "user" => with_store(|s| s.user_names()),
        "tree" if n == 2 && tok(tokens, 1) == "group" => with_store(|s| s.group_names()),
        "check" if n == 1 => with_store(|s| s.user_names()),
        _ => Vec::new(),
    }
}

fn meta_actions() -> Vec<String> {
    vec_s(&[
        "info", "set", "unset", "settemp", "unsettemp", "addprefix", "removeprefix",
        "addsuffix", "removesuffix", "addtempprefix", "addtempsuffix",
    ])
}

fn holder_nodes(_kind: &str, _name: &str) -> Vec<String> {
    with_store(|s| s.known_permissions())
}

fn tok(tokens: &[String], i: usize) -> String {
    tokens.get(i).map(|s| s.to_ascii_lowercase()).unwrap_or_default()
}

fn vec_s(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

pub fn write_export(name: &str, value: &serde_json::Value) -> Result<String, CommandError> {
    with_store(|store| {
        let path = store.data_dir().join("exports").join(name);
        let raw = serde_json::to_string_pretty(value).map_err(|e| {
            CommandError::CommandFailed(chat(&format!("&cjson: {e}")))
        })?;
        std::fs::write(&path, raw).map_err(|e| {
            CommandError::CommandFailed(chat(&format!("&cwrite failed: {e}")))
        })?;
        Ok(path.display().to_string())
    })
}

pub fn read_data_file(name: &str) -> Result<String, CommandError> {
    with_store(|store| {
        let path = if name.contains('/') || name.contains('\\') {
            std::path::PathBuf::from(name)
        } else {
            store.data_dir().join(name)
        };
        let alt = store.data_dir().join("exports").join(name);
        let raw = std::fs::read_to_string(&path)
            .or_else(|_| std::fs::read_to_string(&alt))
            .map_err(|_| CommandError::CommandFailed(chat(&format!("&cfile not found: {name}"))))?;
        Ok(raw)
    })
}

pub fn save() {
    with_store_mut(|s| s.save_if_dirty());
}

pub fn parse_temp_mod(args: &[String]) -> (Option<crate::config::TempAdd>, &[String]) {
    match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("accumulate") => (Some(crate::config::TempAdd::Accumulate), &args[1..]),
        Some("replace") => (Some(crate::config::TempAdd::Replace), &args[1..]),
        Some("shadow") => (Some(crate::config::TempAdd::Shadow), &args[1..]),
        Some("deny") => (Some(crate::config::TempAdd::Deny), &args[1..]),
        _ => (None, args),
    }
}

pub fn split_text_and_ctx(args: &[String]) -> Result<(String, &[String]), CommandError> {
    if args.is_empty() {
        return Err(CommandError::CommandFailed(chat("&cMissing prefix/suffix text")));
    }
    let mut i = 0;
    while i < args.len() && !args[i].contains('=') {
        i += 1;
    }
    if i == 0 {
        return Err(CommandError::CommandFailed(chat("&cMissing prefix/suffix text")));
    }
    Ok((args[..i].join(" "), &args[i..]))
}
