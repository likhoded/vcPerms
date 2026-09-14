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
    command::Command,
    events::{
        EventData, EventHandler, EventPriority, PlayerChangeWorldEvent, PlayerChatEvent,
        PlayerJoinEvent, PlayerLeaveEvent, PlayerPermissionCheckEvent,
    },
    permission::{Permission, PermissionDefault, PermissionLevel},
    scheduler::SchedulerExt,
    Context, Plugin, PluginMetadata, Server, permissions, register_plugin,
};
use tracing::info;

use crate::cmd::RootHelp;
use crate::context::ContextSet;
use crate::holder::Holder;
use crate::state::{init, with_store, with_store_mut, with_verbose};
use crate::store::Store;
use crate::util::{is_server_op, player_uuid};
use crate::verbose::Verbose;

const PERM_CMD: &str = "vcPerms:command";

struct JoinHandler;
struct LeaveHandler;
struct WorldChangeHandler;
struct CheckHandler;
struct ChatHandler;

fn apply_nametag(player: &pumpkin_plugin_api::player::Player) {
    let apply = with_store(|s| s.config.apply_chat_meta);
    if !apply {
        return;
    }
    let uuid = player_uuid(player);
    let decorated = with_store(|store| {
        let Some(user) = store.user(&uuid) else {
            return None;
        };
        let ctx = ContextSet::for_player(player, &store.config);
        let prefix = resolve::prefix_of(store, user, &ctx)
            .map(|(_, t)| t)
            .unwrap_or_default();
        let suffix = resolve::suffix_of(store, user, &ctx)
            .map(|(_, t)| t)
            .unwrap_or_default();
        if prefix.is_empty() && suffix.is_empty() {
            return None;
        }
        Some(format!("{prefix}{}{suffix}", player.get_name()))
    });
    if let Some(name) = decorated {
        player.set_display_name(crate::util::legacy(&name));
        player.set_tab_list_name(Some(crate::util::legacy(&name)));
    }
}

impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, _server: Server, event: EventData<PlayerJoinEvent>) -> EventData<PlayerJoinEvent> {
        let name = event.player.get_name();
        let uuid = player_uuid(&event.player);
        with_store_mut(|store| {
            store.ensure_user(&name, Some(&uuid));
            let world = event.player.get_world();
            store.remember_location(&uuid, &world.get_name(), &world.get_dimension());
            if store.config.debug_logins {
                info!("{name} ({uuid}) loaded into vcPerms");
            }
            store.save_if_dirty();
        });
        apply_nametag(&event.player);
        event
    }
}

impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(&self, _server: Server, event: EventData<PlayerLeaveEvent>) -> EventData<PlayerLeaveEvent> {
        let uuid = player_uuid(&event.player);
        with_store_mut(|store| store.unload_user(&uuid));
        event
    }
}

impl EventHandler<PlayerChangeWorldEvent> for WorldChangeHandler {
    fn handle(
        &self,
        _server: Server,
        event: EventData<PlayerChangeWorldEvent>,
    ) -> EventData<PlayerChangeWorldEvent> {
        let uuid = player_uuid(&event.player);
        let world = &event.new_world;
        with_store_mut(|store| {
            store.remember_location(&uuid, &world.get_name(), &world.get_dimension());
        });
        apply_nametag(&event.player);
        event
    }
}

impl EventHandler<PlayerPermissionCheckEvent> for CheckHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: EventData<PlayerPermissionCheckEvent>,
    ) -> EventData<PlayerPermissionCheckEvent> {
        let uuid = player_uuid(&event.player);
        let perm = event.permission.clone();
        if with_store(|store| !store.knows(&perm)) {
            with_store_mut(|store| store.remember(&perm));
        }
        if with_store(|store| store.user(&uuid).is_none()) {
            with_store_mut(|store| store.load_user(&uuid));
        }
        let host_result = event.permission_result;
        let (allowed, source, node, ops_override, allow_ops, is_op) = with_store(|store| {
            let ctx = ContextSet::for_player(&event.player, &store.config);
            let user = store.user(&uuid);
            let is_op = is_server_op(event.player.get_permission_level());
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
        with_verbose(|v| {
            v.record(Verbose::hit(
                event.player.get_name(),
                perm,
                result,
                source,
                node,
            ));
        });
        event.permission_result = result;
        event
    }
}

impl EventHandler<PlayerChatEvent> for ChatHandler {
    fn handle(&self, _server: Server, event: EventData<PlayerChatEvent>) -> EventData<PlayerChatEvent> {
        apply_nametag(&event.player);
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
            store.indexed_user_count()
        );
        init(store);

        context.register_event_handler(JoinHandler, EventPriority::Normal, true)?;
        context.register_event_handler(LeaveHandler, EventPriority::Normal, true)?;
        context.register_event_handler(WorldChangeHandler, EventPriority::Normal, true)?;
        context.register_event_handler(CheckHandler, EventPriority::Highest, true)?;
        context.register_event_handler(ChatHandler, EventPriority::Low, true)?;

        context.register_permission(&Permission {
            node: PERM_CMD.to_string(),
            description: "Use /vcp".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Two),
            children: Vec::new(),
        })?;

        let root = Command::new(
            &["vcp".to_string(), "vcperms".to_string()],
            "vcPerms — manage permissions",
        )
        .execute(RootHelp);
        context.register_command(cmd::tree::attach(root), PERM_CMD);

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
        if matches!(op, "register" | "announce" | "announce-permissions") {
            let mut keys = Vec::new();
            if let Some(list) = req.get("permissions").or_else(|| req.get("nodes")).and_then(|v| v.as_array()) {
                for item in list {
                    if let Some(s) = item.as_str() {
                        keys.push(s.to_string());
                    }
                }
            }
            if let Some(s) = req.get("permission").and_then(|v| v.as_str()) {
                keys.push(s.to_string());
            }
            let n = with_store_mut(|store| {
                let before = store.known_permissions().len();
                store.remember_many(&keys);
                store.save_if_dirty();
                store.known_permissions().len().saturating_sub(before)
            });
            return Ok(serde_json::to_vec(&serde_json::json!({"ok": true, "added": n}))
                .unwrap_or_else(|_| b"{}".to_vec()));
        }

        let world = req.get("world").and_then(|v| v.as_str()).map(str::to_string);
        let dimension = req.get("dimension").and_then(|v| v.as_str()).map(str::to_string);
        let reply = with_store_mut(|store| {
            let Some(id) = store.load_named(user) else {
                return serde_json::json!({"ok": false, "error": "unknown user"});
            };
            let last_world = store.last_world(&id).map(str::to_string);
            let last_dim = store.last_dimension(&id).map(str::to_string);
            let Some(u) = store.user(&id).cloned() else {
                return serde_json::json!({"ok": false, "error": "unknown user"});
            };
            let ctx = ContextSet::global(&store.config).with_world(
                world.as_deref().or(last_world.as_deref()),
                dimension.as_deref().or(last_dim.as_deref()),
            );
            match op {
                "check" => {
                    let perm = req.get("permission").and_then(|v| v.as_str()).unwrap_or("");
                    let r = resolve::check(store, &u, perm, &ctx);
                    serde_json::json!({"ok": true, "allowed": r.allowed, "node": r.node, "source": r.source})
                }
                "prefix" => {
                    let prefix = resolve::prefix_of(store, &u, &ctx).map(|(_, t)| t);
                    serde_json::json!({"ok": true, "prefix": prefix})
                }
                "suffix" => {
                    let suffix = resolve::suffix_of(store, &u, &ctx).map(|(_, t)| t);
                    serde_json::json!({"ok": true, "suffix": suffix})
                }
                "primary" => serde_json::json!({"ok": true, "primary": u.primary_group}),
                "parents" => serde_json::json!({"ok": true, "parents": u.parents()}),
                _ => serde_json::json!({"ok": false, "error": "unknown op"}),
            }
        });
        Ok(serde_json::to_vec(&reply).unwrap_or_else(|_| b"{}".to_vec()))
    }
}

register_plugin!(VcPerms);
