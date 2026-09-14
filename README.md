# vcPerms

**English** · [Русский](README.ru.md)

**vcPerms** is a permission manager for Pumpkin. It decides which nodes a player has: through groups, inheritance, tracks, contexts, temporary permissions, prefixes and suffixes. Every permission check on the server goes through this plugin.

Built against Pumpkin `0.1.0-dev+26.2-26.45` (Java protocol 776).

## Install

Build with `cargo build --release` and put `target/wasm32-wasip2/release/vcperms.wasm` in `plugins/`. Allow data access in `pumpkin.toml`:

```toml
[plugins]
ask_permission_confirmation = false
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

On first boot the server writes `plugins/data/vcPerms/config.yml` and a `default` group.

## First commands

```
/vcp creategroup admin
/vcp group admin permission set * true
/vcp group admin meta addprefix 100 "&c[Admin] "
/vcp user Steve parent add admin
/vcp check Steve minecraft.command.gamemode
```

You manage it with `/vcp` and `/vcperms`. The console can always run them. In-game you need the `vcperms.*` node or operator status while `commands-allow-ops` is still enabled in the config.

A rank ladder is groups inheriting parents, a track listing them in order, and `promote` / `demote` moving the player.

```
/vcp creategroup member
/vcp creategroup vip
/vcp group member parent add default
/vcp group vip parent add member
/vcp group admin parent add vip
/vcp createtrack ranks
/vcp track ranks append default
/vcp track ranks append member
/vcp track ranks append vip
/vcp track ranks append admin
/vcp user Steve promote ranks
```

## Other plugins

Another wasm talks to `vcPerms` over host IPC and can check a node, read a prefix or list groups. The contract is in [API](wiki/API.md). vcEdit and vcGuard already send their nodes on load.

## Storage

Data is JSON, one file per player, group or track:

```
plugins/data/vcPerms/
  config.yml
  uuidcache.json
  known-permissions.json
  users/<uuid>.json
  groups/<name>.json
  tracks/<name>.json
  exports/
```

`/vcp import` reads our dumps. A LuckPerms JSON export usually opens as well if you are moving an old server over.

## Docs

https://likhoded.github.io/vcPerms/

Wiki sources: [English](wiki/Home.md) · [Русский](wiki/ru/Home.md)

## Build

```
rustup target add wasm32-wasip2
cargo build --release
```

Needs a current stable rustc. Edition 2024.

## License

MIT. See [LICENSE](LICENSE).
