# vcPerms

**English** · [Русский](README.ru.md)

Permission plugin for [Pumpkin](https://pumpkinmc.org). Groups, inheritance, tracks, contexts, temporary nodes, prefixes.

Built against Pumpkin `0.1.0-dev+26.2-26.45` (Java protocol 776).

## Install

1. `cargo build --release`
2. Copy `target/wasm32-wasip2/release/vcperms.wasm` to `plugins/`
3. In `pumpkin.toml`:

```toml
[plugins]
ask_permission_confirmation = false
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

4. Restart the server.

First boot writes `plugins/data/vcPerms/config.yml` and a `default` group.

## Quick start

```
/vcp creategroup admin
/vcp group admin permission set * true
/vcp group admin meta addprefix 100 "&c[Admin] "
/vcp user Steve parent add admin
/vcp check Steve minecraft.command.gamemode
```

Commands: `/vcp` and `/vcperms`.

Console can always run them. In-game you need `vcperms.*`, or OP while `commands-allow-ops` is still on.

## What it stores

JSON, one file per holder:

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

`/vcp import` reads our own dumps. A LuckPerms JSON export usually loads as well if you are moving an old server over.

## Docs

Wiki: https://likhoded.github.io/vcPerms/

Source: [English](wiki/Home.md) · [Русский](wiki/ru/Home.md)

## Build

```
rustup target add wasm32-wasip2
cargo build --release
```

Needs a current stable rustc. Edition 2024.

## License

MIT. See [LICENSE](LICENSE).
