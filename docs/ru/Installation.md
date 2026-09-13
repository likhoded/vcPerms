# Установка

[English](../Installation) · **Русский**

## Требования

- Pumpkin `0.1.0-dev+26.2-26.45` или любая сборка с тем же plugin WIT
- таргет `wasm32-wasip2`, если собираете из исходников

## Положить файл

Киньте `vcperms.wasm` в папку `plugins/` сервера.

vcPerms нужна своя data-папка. Пропишите права в `pumpkin.toml`, иначе хост загрузит плагин и потом запретит любую запись:

```toml
[plugins]
enabled = true
ask_permission_confirmation = false
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

Рестарт. В логе что-то вроде:

```
INFO vcPerms 0.8.4 — 1 groups, 0 users
INFO Loaded vcPerms (0.8.4)
```

`plugins/data/vcPerms/config.yml` создаётся при первом запуске.

## Сборка

```
git clone https://github.com/likhoded/vcPerms
cd vcPerms
rustup target add wasm32-wasip2
cargo build --release
```

Артефакт: `target/wasm32-wasip2/release/vcperms.wasm`.

Неподписанные плагины: Pumpkin орёт варнинг на каждый бут. Это хост, не мы. Либо подпишите, либо оставьте `allow_unsigned = true`.
