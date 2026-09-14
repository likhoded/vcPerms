# Команды

[English](../Commands) · **Русский**

Корневая команда — `/vcp`. `/vcperms` — то же дерево.

Кавычки можно: `/vcp group admin meta addprefix 100 "&c[Admin] "`

## Корень

| команда | |
| --- | --- |
| `/vcp` / `/vcp help` | короткая справка |
| `/vcp info` | версия, счётчики, хранилище |
| `/vcp reload` | перечитать диск |
| `/vcp sync` | сбросить память на диск |
| `/vcp check <user> <node> [ctx...]` | проверить одно право |
| `/vcp search <query> [page]` | найти холдеров с нодой |
| `/vcp tree [user\|group] <name>` | дерево наследования |
| `/vcp verbose on\|off\|record [filter]\|paste` | см. [Verbose](Verbose) |
| `/vcp editor` | полный дамп в `exports/editor.json` |
| `/vcp export [file]` | то же, своё имя файла |
| `/vcp import <file>` / `/vcp applyedits <file>` | загрузить дамп |
| `/vcp creategroup <name>` | |
| `/vcp deletegroup <name>` | `default` удалить нельзя |
| `/vcp createtrack <name>` | |
| `/vcp deletetrack <name>` | |
| `/vcp listgroups` | по весу |
| `/vcp listusers [page]` | |
| `/vcp listtracks` | |

## Игрок

`/vcp user <name|uuid> ...`

`info`, `permission`, `parent`, `meta`, `editor`, `promote <track>` (первая группа, если игрока на треке нет), `demote <track>`, `showtracks`, `clear [ctx...]`, `clone <other>`

### permission

- `info [page]`
- `set <node> [true\|false] [ctx...]`
- `unset <node> [ctx...]`
- `settemp <node> <duration> [true\|false] [ctx...]`
- `unsettemp <node> [ctx...]`
- `check <node> [ctx...]`
- `clear [ctx...]`

### parent

- `info`
- `add <group> [ctx...]`
- `remove <group> [ctx...]`
- `set <group>` — сносит остальные группы
- `addtemp <group> <duration> [ctx...]`
- `removetemp <group> [ctx...]`
- `clear [ctx...]`
- `cleartrack <track>`
- `switchprimarygroup <group>`
- `settrack <track> <group>`

### meta

- `info`
- `set <key> <value> [ctx...]`
- `unset <key>`
- `settemp <key> <value> <duration> [ctx...]`
- `unsettemp <key>`
- `addprefix <priority> <text> [ctx...]`
- `removeprefix <priority>`
- `addsuffix` / `removesuffix`
- `addtempprefix <priority> <duration> <text>` / `addtempsuffix` (порядок как в LuckPerms; текст можно в кавычках)

## Группа

`/vcp group <name> ...`

Те же `permission` / `parent` / `meta`, плюс:

- `listmembers [page]`
- `setweight <n>`
- `setdisplayname <name>`
- `showtracks`
- `rename <new>`
- `clone <new>`
- `clear [ctx...]`

## Трек

`/vcp track <name> info|append|insert|remove|clear|rename|clone`

`insert` считает позицию с единицы.

## Длительности

`30s`, `15m`, `2h`, `1d`, `1w`, `1mo`, `1y`, или склейка: `1d12h`.
