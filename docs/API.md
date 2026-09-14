# API

**English** · [Русский](ru/API)

Plugin-to-plugin contract for vcPerms **0.8.5**.

Other Pumpkin plugins talk to vcPerms over host IPC. The payload is UTF-8 JSON both ways. There is no HTTP API and no in-process Rust crate — wasm plugins cannot link each other.

Related pages: [Permissions](Permissions) · [Prefixes and meta](Prefixes-and-meta) · [Storage](Storage) · [Commands](Commands)

---

## 1. Transport

| | |
|---|---|
| Recipient id | `vcPerms` (the name from plugin metadata, not the file name) |
| Host call | `pumpkin_plugin_api::ipc::send_ipc_message("vcPerms", &bytes)` |
| Request body | JSON object, UTF-8 |
| Response body | JSON object, UTF-8 |
| Encoding errors | WIT `Err("bad ipc json: …")` — not a JSON object |

Call it after the server has finished loading plugins. If vcPerms is missing, the host returns an error on `send_ipc_message`. Treat that as “vcPerms is not installed”.

A typical wrapper:

```rust
use pumpkin_plugin_api::ipc;

fn vcp(req: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    let reply = ipc::send_ipc_message("vcPerms", &raw)
        .map_err(|_| "vcPerms is not loaded".to_string())?
        .map_err(|e| format!("vcPerms: {e}"))?;
    serde_json::from_slice(&reply).map_err(|e| format!("bad vcPerms reply: {e}"))
}
```

Every successful decode is a JSON object with `ok` (boolean). On `ok: false` read `error` (string).

| `error` | Meaning |
|---|---|
| `unknown user` | No holder for that name or uuid |
| `unknown op` | `op` missing or not in this document |

Malformed JSON never becomes `{ok:false}`. The host error string is the signal.

---

## 2. Identity

Field `user` is a player name **or** a uuid string.

Resolution order: name via `uuidcache.json`, then uuid lookup, then a case-insensitive scan of stored names.

IPC **does not create** holders. A name that never joined and was never touched by `/vcp user` is `unknown user`. `/vcp user <name> info` and a real join both create the file.

Queries use the global context from `config.yml`, plus optional `world` / `dimension` on the request. If those fields are missing, the last world the player was in (join / world change) is used.

---

## 3. Operations

Every request has:

```json
{ "op": "<name>", ... }
```

`op` is case-sensitive.

### 3.1 `check`

Resolve one permission for a stored user. Same matching rules as a host `hasPermission` check: exact node, `foo.*`, `*`, own nodes beat inherited, longer wildcard beats shorter, group weight breaks ties, `value: false` is deny. See [Permissions](Permissions).

**Request**

| Field | Type | |
|---|---|---|
| `op` | string | `check` |
| `user` | string | name or uuid |
| `permission` | string | node to test |
| `world` | string | optional world name |
| `dimension` | string | optional dimension |

**Response**

```json
{ "ok": true, "allowed": true, "node": "*", "source": "group/admin" }
```

| Field | Type | |
|---|---|---|
| `allowed` | bool | final result |
| `node` | string | key that won, or the queried key if nothing matched |
| `source` | string | `user/<name>`, `group/<group>`, or `none` |

`source: "none"` means no covering node. `allowed` is then `false`. OP bypass (`ops-override` / `allow-ops`) is **not** applied on IPC. That path exists only on `PlayerPermissionCheckEvent`.

```json
{ "ok": true, "allowed": false, "node": "essentials.fly", "source": "none" }
{ "ok": false, "error": "unknown user" }
```

### 3.2 `prefix`

Highest-priority prefix among the user’s own nodes and inherited groups.

**Request:** `{ "op": "prefix", "user": "Steve" }`

**Response**

```json
{ "ok": true, "prefix": "&c[Admin] " }
{ "ok": true, "prefix": null }
```

`prefix` is the raw stored text (legacy color codes). `null` if none.

### 3.3 `suffix`

Same as `prefix`, for suffix nodes.

```json
{ "op": "suffix", "user": "Steve" }
```

```json
{ "ok": true, "suffix": " &7[VIP]" }
{ "ok": true, "suffix": null }
```

### 3.4 `primary`

Primary group name stored on the user (`primary_group`). Not inferred from parents.

```json
{ "op": "primary", "user": "Steve" }
```

```json
{ "ok": true, "primary": "admin" }
```

### 3.5 `parents`

Group names from the user’s `group.<name>` nodes (current, non-structural). Order is the order on the holder.

```json
{ "op": "parents", "user": "Steve" }
```

```json
{ "ok": true, "parents": ["admin", "member"] }
```

### 3.6 `register`

Publish permission nodes so `/vcp … permission set <tab>` can complete them. Call this from `on_load` once vcPerms is up.

Aliases (same handler): `announce`, `announce-permissions`.

**Request** — any combination of:

| Field | Type | |
|---|---|---|
| `permissions` | string[] | preferred |
| `nodes` | string[] | same as `permissions` |
| `permission` | string | single extra node |

```json
{
  "op": "register",
  "permissions": [
    "myplugin.*",
    "myplugin.use",
    "myplugin.admin",
    "myplugin:command"
  ]
}
```

**Response**

```json
{ "ok": true, "added": 4 }
```

`added` is how many **new** keys landed in the catalog this call. Duplicates are ignored.

vcPerms also stores a parent wildcard: `myplugin.use` implies `myplugin.*` if you did not send it.

Ignored keys (structural, not permission nodes):

- `group.*`
- `prefix.*` / `suffix.*`
- `meta.*`
- `weight.*`
- `displayname.*`

The catalog is written to `plugins/data/vcPerms/known-permissions.json` and reloaded with the rest of the store.

If your plugin loads before vcPerms, `send_ipc_message` fails. That is fine. The first live `hasPermission` on your node still teaches the catalog — see [§6](#6-host-permission-checks).

---

## 4. What IPC does not do

There is no `set` / `unset` / `addparent` over IPC.

To change data from outside:

1. `/vcp user|group …` (console or a player with `vcperms.*`)
2. Write the holder JSON under `plugins/data/vcPerms/` and `/vcp reload`

Do not share a write lock with the running plugin. Edit files only while the server is stopped, or reload immediately after an atomic replace.

---

## 5. Integrating a plugin

### 5.1 In-game checks

For a player that is online, call the host:

```rust
if player.has_permission("myplugin.use")
    || player.has_permission("myplugin.*")
    || player.has_permission("*")
{
    // allowed
}
```

Pumpkin raises `PlayerPermissionCheckEvent`. vcPerms answers it. You do not need IPC on this path.

Register the host command node as usual (`myplugin:command` or whatever the host assigned). That is separate from the nodes you give players (`myplugin.use`).

### 5.2 Announce the catalog

On load, after commands and permissions are registered:

```rust
fn announce_vcperms() {
    let req = serde_json::json!({
        "op": "register",
        "permissions": [
            "myplugin.*",
            "myplugin.use",
            "myplugin.admin",
            "myplugin:command",
        ],
    });
    let Ok(raw) = serde_json::to_vec(&req) else { return };
    let _ = pumpkin_plugin_api::ipc::send_ipc_message("vcPerms", &raw);
}
```

Staff can then tab-complete:

```
/vcp group member permission set myplugin.use
```

### 5.3 Offline / console lookups

Use `check` / `prefix` / `parents` when you have a name or uuid and no `Player` handle — for example a queued reward or a Discord bot bridge that already has a Pumpkin plugin on the server.

### 5.4 Full example

```rust
fn can_fly(name: &str) -> Result<bool, String> {
    let reply = vcp(serde_json::json!({
        "op": "check",
        "user": name,
        "permission": "essentials.fly"
    }))?;

    if reply.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let err = reply.get("error").and_then(|v| v.as_str()).unwrap_or("failed");
        return Err(err.into());
    }
    Ok(reply.get("allowed").and_then(|v| v.as_bool()).unwrap_or(false))
}
```

---

## 6. Host permission checks

Every `Player.has_permission` / command requirement goes through vcPerms while the plugin is loaded.

| Step | |
|---|---|
| 1 | Look up the player uuid in the store (created on join) |
| 2 | Build a **player** context (world, dimension, plus config static keys) |
| 3 | Resolve as in [Permissions](Permissions) |
| 4 | If a node matched, that `value` is final |
| 5 | If nothing matched and `ops-override` and the player is OP → allow |
| 6 | Else if `allow-ops` → keep the host’s own result (vanilla OP) |
| 7 | Else deny |

The queried string is recorded in `known-permissions.json`. That is why a plugin that never called `register` still shows up in `/vcp` tab after someone actually used a feature.

---

## 7. Node language

| Node | Covers |
|---|---|
| `myplugin.use` | only that key |
| `myplugin.*` | `myplugin.use`, `myplugin.admin.kick`, … |
| `*` | everything |

A longer match wins. `myplugin.admin.*` = false beats `myplugin.*` = true on `myplugin.admin.kick`.

Temporary nodes (`expiry` unix seconds) are dropped when expired. IPC `check` sees the same expiry rules.

---

## 8. Compatibility

| | |
|---|---|
| Documented version | 0.8.5 |
| Transport | Pumpkin plugin IPC, JSON objects |
| Stable `op` names | `check`, `prefix`, `suffix`, `primary`, `parents`, `register` |
| Aliases | `announce`, `announce-permissions` → `register` |
| Additive changes | new `op` values, extra response fields |
| Breaking changes | renaming `op`, changing `ok` / `error`, changing `allowed` meaning |

Unknown fields on the request are ignored. Extra fields on the response may appear in later 0.8.x builds — read what you need.

Host target: Pumpkin `0.1.0-dev+26.2-26.45` (Java protocol 776).
