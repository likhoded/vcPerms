# API

[English](../API) · **Русский**

Контракт между плагинами для vcPerms **0.8.5**.

Другие плагины Pumpkin ходят в vcPerms через IPC хоста. В обе стороны — UTF-8 JSON. HTTP нет, общего Rust-крейта тоже нет: wasm-плагины друг к другу не линкуются.

Рядом: [Права](Permissions) · [Префиксы и мета](Prefixes-and-meta) · [Хранение](Storage) · [Команды](Commands)

---

## 1. Транспорт

| | |
|---|---|
| Id получателя | `vcPerms` (имя из метаданных плагина, не имя файла) |
| Вызов хоста | `pumpkin_plugin_api::ipc::send_ipc_message("vcPerms", &bytes)` |
| Тело запроса | JSON-объект, UTF-8 |
| Тело ответа | JSON-объект, UTF-8 |
| Битый JSON | WIT `Err("bad ipc json: …")` — не объект `{ok:false}` |

Вызывать после того, как сервер догрузил плагины. Если vcPerms нет, `send_ipc_message` вернёт ошибку хоста. Это и есть «плагин не установлен».

Типичная обёртка:

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

Любой успешно разобранный ответ — объект с `ok` (bool). При `ok: false` смотри `error` (string).

| `error` | Значение |
|---|---|
| `unknown user` | Нет холдера с таким ником или uuid |
| `unknown op` | Нет поля `op` или оно не из этого документа |

Кривой JSON в `{ok:false}` не превращается. Сигнал — ошибка WIT.

---

## 2. Идентичность

Поле `user` — ник **или** строка uuid.

Порядок поиска: `uuidcache.json` по нику, затем uuid, затем регистронезависимый обход сохранённых имён.

IPC холдеров **не создаёт**. Ник, который ни разу не заходил и которого не трогали через `/vcp user`, даст `unknown user`. Файл появляется после реального входа или `/vcp user <name> info`.

Запросы идут в **глобальном** контексте из `config.yml` (только статичные ключи сервера). Текущий мир и измерение игрока сюда не попадают. Живая проверка в мире — через API хоста, [§6](#6-проверки-хоста).

---

## 3. Операции

У каждого запроса есть:

```json
{ "op": "<name>", ... }
```

`op` чувствителен к регистру.

### 3.1 `check`

Одно право для сохранённого игрока. Те же правила, что у `hasPermission` на хосте: точная нода, `foo.*`, `*`, свои ноды бьют унаследованные, более длинный wildcard бьёт короткий, вес группы разводит ничью, `value: false` — запрет. См. [Права](Permissions).

**Запрос**

| Поле | Тип | |
|---|---|---|
| `op` | string | `check` |
| `user` | string | ник или uuid |
| `permission` | string | проверяемая нода |

**Ответ**

```json
{ "ok": true, "allowed": true, "node": "*", "source": "group/admin" }
```

| Поле | Тип | |
|---|---|---|
| `allowed` | bool | итог |
| `node` | string | ключ, который победил, либо запрошенный ключ, если совпадений не было |
| `source` | string | `user/<name>`, `group/<group>` или `none` |

`source: "none"` — покрывающей ноды нет. Тогда `allowed` = `false`. Обход OP (`ops-override` / `allow-ops`) на IPC **не применяется**. Он есть только в `PlayerPermissionCheckEvent`.

```json
{ "ok": true, "allowed": false, "node": "essentials.fly", "source": "none" }
{ "ok": false, "error": "unknown user" }
```

### 3.2 `prefix`

Префикс с наибольшим приоритетом среди своих нод игрока и унаследованных групп.

**Запрос:** `{ "op": "prefix", "user": "Steve" }`

**Ответ**

```json
{ "ok": true, "prefix": "&c[Admin] " }
{ "ok": true, "prefix": null }
```

`prefix` — как лежит в хранилище (legacy-цвета). `null`, если префикса нет.

### 3.3 `suffix`

То же, что `prefix`, для суффикса.

```json
{ "op": "suffix", "user": "Steve" }
```

```json
{ "ok": true, "suffix": " &7[VIP]" }
{ "ok": true, "suffix": null }
```

### 3.4 `primary`

Имя первичной группы на холдере (`primary_group`). Из родителей не выводится.

```json
{ "op": "primary", "user": "Steve" }
```

```json
{ "ok": true, "primary": "admin" }
```

### 3.5 `parents`

Имена групп из нод `group.<name>` на игроке. Порядок — как на холдере.

```json
{ "op": "parents", "user": "Steve" }
```

```json
{ "ok": true, "parents": ["admin", "member"] }
```

### 3.6 `register`

Опубликовать ноды, чтобы `/vcp … permission set <tab>` их видел. Вызывать из `on_load`, когда vcPerms уже поднят.

Те же обработчики: `announce`, `announce-permissions`.

**Запрос** — любая комбинация:

| Поле | Тип | |
|---|---|---|
| `permissions` | string[] | предпочтительно |
| `nodes` | string[] | то же, что `permissions` |
| `permission` | string | одна дополнительная нода |

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

**Ответ**

```json
{ "ok": true, "added": 4 }
```

`added` — сколько **новых** ключей попало в каталог за этот вызов. Повторы отбрасываются.

Родительский wildcard vcPerms допишет сам: `myplugin.use` даёт `myplugin.*`, если его не прислали.

Игнорируются структурные ключи (это не permission-ноды):

- `group.*`
- `prefix.*` / `suffix.*`
- `meta.*`
- `weight.*`
- `displayname.*`

Каталог пишется в `plugins/data/vcPerms/known-permissions.json` и читается вместе с остальным стором.

Если плагин стартанул раньше vcPerms, `send_ipc_message` упадёт. Это нормально. Первая живая `hasPermission` по вашей ноде всё равно попадёт в каталог — см. [§6](#6-проверки-хоста).

---

## 4. Чего в IPC нет

Нет `set` / `unset` / `addparent`.

Менять данные снаружи можно так:

1. `/vcp user|group …` (консоль или игрок с `vcperms.*`)
2. Записать JSON холдера в `plugins/data/vcPerms/` и сделать `/vcp reload`

Не пишите в файлы параллельно с запущенным плагином. Либо стоп сервера, либо сразу reload после атомарной замены.

---

## 5. Как встроиться

### 5.1 Проверки в игре

Для онлайн-игрока — API хоста:

```rust
if player.has_permission("myplugin.use")
    || player.has_permission("myplugin.*")
    || player.has_permission("*")
{
    // можно
}
```

Pumpkin кидает `PlayerPermissionCheckEvent`. Отвечает vcPerms. IPC на этом пути не нужен.

Хостовую ноду команды регистрируйте как обычно (`myplugin:command` или что выдаст хост). Это отдельно от нод, которые вы раздаёте игрокам (`myplugin.use`).

### 5.2 Анонс каталога

На load, после регистрации команд и прав:

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

Дальше в табе будет:

```
/vcp group member permission set myplugin.use
```

### 5.3 Оффлайн и консоль

`check` / `prefix` / `parents` — когда есть ник или uuid, а `Player` нет: отложенная награда, мост в Discord, если на сервере уже крутится ваш Pumpkin-плагин.

### 5.4 Полный пример

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

## 6. Проверки хоста

Каждый `Player.has_permission` и требование команды идёт через vcPerms, пока плагин загружен.

| Шаг | |
|---|---|
| 1 | Найти uuid игрока в сторе (создаётся на входе) |
| 2 | Собрать **игровой** контекст (мир, измерение, плюс статичные ключи из конфига) |
| 3 | Резолв как в [Правах](Permissions) |
| 4 | Если нода нашлась — её `value` окончательный |
| 5 | Если нет, `ops-override` и игрок OP → allow |
| 6 | Иначе при `allow-ops` оставляем решение хоста (ванильный OP) |
| 7 | Иначе deny |

Запрошенная строка пишется в `known-permissions.json`. Поэтому плагин, который не вызывал `register`, всё равно появится в табе `/vcp` после первого реального использования фичи.

---

## 7. Язык нод

| Нода | Что кроет |
|---|---|
| `myplugin.use` | только этот ключ |
| `myplugin.*` | `myplugin.use`, `myplugin.admin.kick`, … |
| `*` | всё |

Более длинное совпадение побеждает. `myplugin.admin.*` = false бьёт `myplugin.*` = true на `myplugin.admin.kick`.

Временные ноды (`expiry` — unix-секунды) после срока выкидываются. IPC `check` видит те же правила.

---

## 8. Совместимость

| | |
|---|---|
| Версия документа | 0.8.5 |
| Транспорт | IPC плагинов Pumpkin, JSON-объекты |
| Стабильные `op` | `check`, `prefix`, `suffix`, `primary`, `parents`, `register` |
| Синонимы | `announce`, `announce-permissions` → `register` |
| Аддитивные изменения | новые `op`, лишние поля в ответе |
| Ломающие | переименование `op`, смена смысла `ok` / `error` / `allowed` |

Неизвестные поля запроса игнорируются. В ответах 0.8.x могут появиться новые поля — читайте то, что нужно.

Хост: Pumpkin `0.1.0-dev+26.2-26.45` (Java protocol 776).
