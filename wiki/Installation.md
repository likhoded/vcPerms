# Installation

**English** · [Русский](ru/Installation)

## Requirements

- Pumpkin `0.1.0-dev+26.2-26.45` or whatever build still speaks the same plugin WIT
- `wasm32-wasip2` rustc target if you build from source

## Drop-in

Put `vcperms.wasm` in the server `plugins/` folder.

vcPerms needs its data directory. Allow it in `pumpkin.toml` or the host will load the plugin and then refuse every file write:

```toml
[plugins]
enabled = true
ask_permission_confirmation = false
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

Restart. You should see something like:

```
INFO vcPerms 0.8.4 — 1 groups, 0 users
INFO Loaded vcPerms (0.8.4)
```

`plugins/data/vcPerms/config.yml` is created on first run.

## Building

```
git clone https://github.com/likhoded/vcPerms
cd vcPerms
rustup target add wasm32-wasip2
cargo build --release
```

The artifact is `target/wasm32-wasip2/release/vcperms.wasm`.

Unsigned plugins: Pumpkin warns on every boot. That's the host, not us. Either sign it or leave `allow_unsigned = true`.
