# vcPerms

[English](README.md) · **Русский**

**vcPerms** — менеджер прав для Pumpkin. Он задаёт, какие узлы есть у игрока: через группы, наследование, треки, контексты, временные права, префиксы и суффиксы. Каждая проверка прав на сервере проходит через этот плагин.

Собран под Pumpkin `0.1.0-dev+26.2-26.45` (Java protocol 776).

## Установка

Соберите плагин командой `cargo build --release` и положите `target/wasm32-wasip2/release/vcperms.wasm` в `plugins/`. В `pumpkin.toml` разрешите доступ к данным:

```toml
[plugins]
ask_permission_confirmation = false
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

После перезапуска сервер создаст `plugins/data/vcPerms/config.yml` и группу `default`.

## Первые команды

```
/vcp creategroup admin
/vcp group admin permission set * true
/vcp group admin meta addprefix 100 "&c[Admin] "
/vcp user Steve parent add admin
/vcp check Steve minecraft.command.gamemode
```

Управление идёт через `/vcp` и `/vcperms`. С консоли они доступны всегда. В игре нужен узел `vcperms.*` либо статус оператора, пока в конфиге включён `commands-allow-ops`.

Лестница рангов собирается так: группы наследуют родителей, трек ставит их по порядку, игрока двигают `promote` и `demote`.

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

## Другие плагины

Чужой wasm пишет в `vcPerms` по IPC хоста и может проверить узел, прочитать префикс или список групп. Контракт описан в [API](wiki/ru/API.md). vcEdit и vcGuard уже отдают свои ноды при загрузке.

## Хранение

Данные лежат в JSON, по одному файлу на игрока, группу или трек:

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

Команда `/vcp import` читает наши дампы. JSON-экспорт LuckPerms обычно тоже открывается, если вы переезжаете со старого сервера.

## Документация

https://likhoded.github.io/vcPerms/

Исходники вики: [English](wiki/Home.md) · [Русский](wiki/ru/Home.md)

## Сборка

```
rustup target add wasm32-wasip2
cargo build --release
```

Нужен свежий stable rustc. Edition 2024.

## Лицензия

MIT. См. [LICENSE](LICENSE).
