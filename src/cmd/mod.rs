mod group;
mod misc;
mod track;
mod user;

use pumpkin_plugin_api::command::{Arg, CommandError, CommandSender, CommandSuggestion, CommandSuggestions, ConsumedArgs, SuggestionRequest};
use pumpkin_plugin_api::commands::{CommandHandler, CommandSuggestionHandler};
use pumpkin_plugin_api::permission::PermissionLevel;
use pumpkin_plugin_api::{Server, player::Player};

use crate::context::ContextSet;
use crate::resolve;
use crate::state::{with_store, with_store_mut};
use crate::util::{legacy, player_uuid, tokenize};

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
        let tokens = tokenize(&request.remaining);
        let prefix = if request.remaining.ends_with(' ') {
            ""
        } else {
            tokens.last().map(String::as_str).unwrap_or("")
        };
        let options = suggestions(&tokens, request.remaining.ends_with(' '));
        let values = options
            .into_iter()
            .filter(|v| prefix.is_empty() || v.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()))
            .map(|value| CommandSuggestion {
                value,
                tooltip: None,
            })
            .collect();
        CommandSuggestions {
            start: request.start,
            length: request.remaining.len() as u32,
            values,
        }
    }
}

fn run(sender: &CommandSender, server: &Server, line: &str) -> Result<i32, CommandError> {
    let args = tokenize(line);
    if args.is_empty() {
        help(sender);
        return Ok(1);
    }
    let head = args[0].to_ascii_lowercase();
    if !can_run(sender, server, &head, &args) {
        sender.send_message(legacy("&cYou don't have permission to do that."));
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
            sender.send_message(legacy(&format!("&cUnknown subcommand '{other}'. Try /vcp help")));
        }
    }
    Ok(1)
}

fn help(sender: &CommandSender) {
    sender.send_message(legacy(
        "&3&lvcPerms &7— permission manager\n\
         &7/vcp user <user> ...\n\
         &7/vcp group <group> ...\n\
         &7/vcp track <track> ...\n\
         &7/vcp creategroup | deletegroup | listgroups\n\
         &7/vcp createtrack | deletetrack | listtracks | listusers\n\
         &7/vcp check <user> <node>  &8/  &7search <query>\n\
         &7/vcp verbose on|off|record|paste\n\
         &7/vcp tree [user|group] <name>\n\
         &7/vcp import|export|applyedits|editor|reload|info",
    ));
}

fn can_run(sender: &CommandSender, server: &Server, head: &str, args: &[String]) -> bool {
    if sender.is_console() || !sender.is_player() {
        return true;
    }
    let Some(player) = sender.as_player() else {
        return true;
    };
    let needed = command_node(head, args);
    let allowed = with_store(|store| {
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
        let op = matches!(
            player.get_permission_level(),
            PermissionLevel::One
                | PermissionLevel::Two
                | PermissionLevel::Three
                | PermissionLevel::Four
        );
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
    let rest = args.get(1).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    match head {
        "user" => match rest.as_str() {
            "permission" => "vcperms.user.permission.set".into(),
            "parent" => "vcperms.user.parent.add".into(),
            "meta" => "vcperms.user.meta.set".into(),
            "promote" => "vcperms.user.promote".into(),
            "demote" => "vcperms.user.demote".into(),
            "info" => "vcperms.user.info".into(),
            _ => "vcperms.user.info".into(),
        },
        "group" => match rest.as_str() {
            "permission" => "vcperms.group.permission.set".into(),
            "parent" => "vcperms.group.parent.add".into(),
            "meta" => "vcperms.group.meta.set".into(),
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
    sender.send_message(legacy(text));
}

pub fn need<'a>(args: &'a [String], idx: usize, what: &str) -> Result<&'a str, CommandError> {
    args.get(idx)
        .map(String::as_str)
        .ok_or_else(|| CommandError::CommandFailed(legacy(&format!("&cMissing {what}"))))
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
        "user" if n == 3 && tok(tokens, 2) == "meta" => vec_s(&[
            "info", "set", "unset", "settemp", "unsettemp", "addprefix", "removeprefix",
            "addsuffix", "removesuffix", "addtempprefix", "addtempsuffix",
        ]),
        "user" if n == 3 && matches!(tok(tokens, 2).as_str(), "promote" | "demote" | "parent") => {
            with_store(|s| s.track_names())
        }
        "group" if n == 1 => with_store(|s| s.group_names()),
        "group" if n == 2 => vec_s(&[
            "info", "permission", "parent", "meta", "editor", "listmembers", "setweight",
            "setdisplayname", "showtracks", "clear", "rename", "clone",
        ]),
        "track" if n == 1 => with_store(|s| s.track_names()),
        "track" if n == 2 => vec_s(&["info", "append", "insert", "remove", "clear", "rename", "clone", "editor"]),
        "verbose" if n == 1 => vec_s(&["on", "off", "record", "paste"]),
        "tree" if n == 1 => vec_s(&["user", "group"]),
        _ => Vec::new(),
    }
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
            CommandError::CommandFailed(legacy(&format!("&cjson: {e}")))
        })?;
        std::fs::write(&path, raw).map_err(|e| {
            CommandError::CommandFailed(legacy(&format!("&cwrite failed: {e}")))
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
            .map_err(|_| CommandError::CommandFailed(legacy(&format!("&cfile not found: {name}"))))?;
        Ok(raw)
    })
}

pub fn save() {
    with_store_mut(|s| s.save_all());
}
