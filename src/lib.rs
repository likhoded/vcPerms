mod cmd;
mod config;
mod context;
mod holder;
mod node;
mod resolve;
mod state;
mod store;
mod track;
mod util;
mod verbose;

use pumpkin_plugin_api::{
    command::{ArgumentType, Command, CommandNode, StringType},
    events::{
        EventData, EventHandler, EventPriority, PlayerChatEvent, PlayerJoinEvent,
        PlayerPermissionCheckEvent,
    },
    permission::{Permission, PermissionDefault, PermissionLevel},
    scheduler::SchedulerExt,
    Context, Plugin, PluginMetadata, Server, permissions, register_plugin,
};
use tracing::info;

use crate::cmd::{Dispatch, RootHelp, Suggest};
use crate::context::ContextSet;
use crate::holder::Holder;
use crate::state::{init, with_store, with_store_mut, with_verbose};
use crate::store::Store;
use crate::util::player_uuid;
use crate::verbose::Verbose;

const PERM_CMD: &str = "vcPerms:command";

struct JoinHandler;
struct CheckHandler;
struct ChatHandler;

impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, _server: Server, event: EventData<PlayerJoinEvent>) -> EventData<PlayerJoinEvent> {
        let name = event.player.get_name();
        let uuid = player_uuid(&event.player);
        with_store_mut(|store| {
            store.ensure_user(&name, Some(&uuid));
            if store.config.debug_logins {
                info!("{name} ({uuid}) loaded into vcPerms");
            }
            store.save_if_dirty();
        });
        event
    }
}

impl EventHandler<PlayerPermissionCheckEvent> for CheckHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: EventData<PlayerPermissionCheckEvent>,
    ) -> EventData<PlayerPermissionCheckEvent> {
        let name = event.player.get_name();
        let uuid = player_uuid(&event.player);
        let perm = event.permission.clone();
        let host_result = event.permission_result;
        let (allowed, source, node, ops_override, allow_ops, is_op) = with_store(|store| {
            let ctx = ContextSet::for_player(&event.player, &store.config);
            let user = store.user(&uuid);
            let is_op = !matches!(event.player.get_permission_level(), PermissionLevel::Zero);
            match user {
                Some(user) => {
                    let r = resolve::check(store, user, &perm, &ctx);
                    (
                        r.allowed,
                        r.source,
                        r.node,
                        store.config.ops_override,
                        store.config.allow_ops,
                        is_op,
                    )
                }
                None => (
                    false,
                    "none".into(),
                    perm.clone(),
                    store.config.ops_override,
                    store.config.allow_ops,
                    is_op,
                ),
            }
        });

        let result = if allowed {
            true
        } else if source != "none" {
            // explicit node, including a deny
            false
        } else if ops_override && is_op {
            true
        } else {
            allow_ops && host_result
        };
        let _ = (name, node);

        with_verbose(|v| {
            v.record(Verbose::hit(
                event.player.get_name(),
                perm,
                result,
                source,
                event.permission.clone(),
            ));
        });
        event.permission_result = result;
        event
    }
}

impl EventHandler<PlayerChatEvent> for ChatHandler {
    fn handle(&self, _server: Server, mut event: EventData<PlayerChatEvent>) -> EventData<PlayerChatEvent> {
        let apply = with_store(|s| s.config.apply_chat_meta);
        if !apply || event.cancelled {
            return event;
        }
        let uuid = player_uuid(&event.player);
        let decorated = with_store(|store| {
            let Some(user) = store.user(&uuid) else {
                return None;
            };
            let ctx = ContextSet::for_player(&event.player, &store.config);
            let prefix = resolve::prefix_of(store, user, &ctx).map(|(_, t)| t).unwrap_or_default();
            let suffix = resolve::suffix_of(store, user, &ctx).map(|(_, t)| t).unwrap_or_default();
            if prefix.is_empty() && suffix.is_empty() {
                return None;
            }
            Some(format!("{prefix}{}{suffix}", event.player.get_name()))
        });
        if let Some(name) = decorated {
            // Pumpkin chat event only exposes the raw message. Prefix the body so
            // other plugins still see the original text if they run after us.
            if !event.message.starts_with(&name) {
                event.message = format!("{name}: {}", event.message);
            }
        }
        event
    }
}

struct VcPerms;

impl Plugin for VcPerms {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "vcPerms".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["likhoded".into()],
            description: "Permission groups, tracks and contexts for Pumpkin".into(),
            dependencies: vec![],
            permissions: vec![
                permissions::FS_READ_DATA.into(),
                permissions::FS_WRITE_DATA.into(),
            ],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let dir = context.get_data_folder();
        let store = Store::load(&dir);
        info!(
            "vcPerms {} — {} groups, {} users",
            env!("CARGO_PKG_VERSION"),
            store.groups().count(),
            store.users().count()
        );
        init(store);

        context.register_event_handler(JoinHandler, EventPriority::Normal, true)?;
        context.register_event_handler(CheckHandler, EventPriority::Highest, true)?;
        context.register_event_handler(ChatHandler, EventPriority::Low, true)?;

        context.register_permission(&Permission {
            node: PERM_CMD.to_string(),
            description: "Use /vcp".to_string(),
            default: PermissionDefault::Op(PermissionLevel::One),
            children: Vec::new(),
        })?;

        let mut root = Command::new(
            &["vcp".to_string(), "vcperms".to_string()],
            "vcPerms — manage permissions",
        )
        .execute(RootHelp);

        let args = CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .suggest(Suggest)
            .execute(Dispatch);
        root = root.then(args);
        context.register_command(root, PERM_CMD);

        context.schedule_repeating_task(20, 20 * 60, |_server| {
            let n = with_store_mut(|s| {
                let n = s.sweep_expired();
                s.save_if_dirty();
                n
            });
            if n > 0 {
                info!("dropped {n} expired node(s)");
            }
        });

        Ok(())
    }

    fn on_unload(&self, _context: Context) -> pumpkin_plugin_api::Result<()> {
        with_store_mut(|s| s.save_all());
        Ok(())
    }

    fn handle_ipc_message(
        &self,
        _sender: String,
        message: Vec<u8>,
    ) -> pumpkin_plugin_api::Result<Vec<u8>, String> {
        let raw = String::from_utf8_lossy(&message);
        let req: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("bad ipc json: {e}"))?;
        let op = req.get("op").and_then(|v| v.as_str()).unwrap_or("");
        let user = req.get("user").and_then(|v| v.as_str()).unwrap_or("");
        let reply = with_store(|store| match op {
            "check" => {
                let perm = req.get("permission").and_then(|v| v.as_str()).unwrap_or("");
                let Some(u) = store.user_by_name(user).or_else(|| store.user(user)) else {
                    return serde_json::json!({"ok": false, "error": "unknown user"});
                };
                let ctx = ContextSet::global(&store.config);
                let r = resolve::check(store, u, perm, &ctx);
                serde_json::json!({"ok": true, "allowed": r.allowed, "node": r.node, "source": r.source})
            }
            "prefix" => {
                let Some(u) = store.user_by_name(user).or_else(|| store.user(user)) else {
                    return serde_json::json!({"ok": false, "error": "unknown user"});
                };
                let ctx = ContextSet::global(&store.config);
                let prefix = resolve::prefix_of(store, u, &ctx).map(|(_, t)| t);
                serde_json::json!({"ok": true, "prefix": prefix})
            }
            "suffix" => {
                let Some(u) = store.user_by_name(user).or_else(|| store.user(user)) else {
                    return serde_json::json!({"ok": false, "error": "unknown user"});
                };
                let ctx = ContextSet::global(&store.config);
                let suffix = resolve::suffix_of(store, u, &ctx).map(|(_, t)| t);
                serde_json::json!({"ok": true, "suffix": suffix})
            }
            "primary" => {
                let Some(u) = store.user_by_name(user).or_else(|| store.user(user)) else {
                    return serde_json::json!({"ok": false, "error": "unknown user"});
                };
                serde_json::json!({"ok": true, "primary": u.primary_group})
            }
            "parents" => {
                let Some(u) = store.user_by_name(user).or_else(|| store.user(user)) else {
                    return serde_json::json!({"ok": false, "error": "unknown user"});
                };
                serde_json::json!({"ok": true, "parents": u.parents()})
            }
            _ => serde_json::json!({"ok": false, "error": "unknown op"}),
        });
        Ok(serde_json::to_vec(&reply).unwrap_or_else(|_| b"{}".to_vec()))
    }
}

register_plugin!(VcPerms);
