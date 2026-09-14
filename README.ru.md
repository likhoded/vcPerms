# vcPerms

[English](README.md) · **Русский**

Плагин прав для [Pumpkin](https://pumpkinmc.org). Группы, наследование, треки, контексты, временные ноды, префиксы.

Собран под Pumpkin `0.1.0-dev+26.2-26.45` (Java protocol 776).

## Установка

1. `cargo build --release`
2. Скопировать `target/wasm32-wasip2/release/vcperms.wasm` в `plugins/`
3. В `pumpkin.toml`:

```toml
[plugins]
ask_permission_confirmation = false
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

4. Перезапустить сервер.

При первом запуске появятся `plugins/data/vcPerms/config.yml` и группа `default`.

## Быстрый старт

```
/vcp creategroup admin
/vcp group admin permission set * true
/vcp group admin meta addprefix 100 "&c[Admin] "
/vcp user Steve parent add admin
/vcp check Steve minecraft.command.gamemode
```

Команды: `/vcp` и `/vcperms`.

С консоли можно всегда. В игре нужен `vcperms.*`, либо OP, пока включён `commands-allow-ops`.

## Что хранит

JSON, один файл на холдера:

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

`/vcp import` читает наши дампы. JSON-экспорт LuckPerms обычно тоже встаёт, если переезжаете со старого сервера.

## Документация

Вики: https://likhoded.github.io/vcPerms/

Исходники: [English](wiki/Home.md) · [Русский](wiki/ru/Home.md)

## Сборка

```
rustup target add wasm32-wasip2
cargo build --release
```

Нужен свежий stable rustc. Edition 2024.

## Лицензия

MIT. См. [LICENSE](LICENSE).
