# Конфиг

[English](../Config) · **Русский**

`plugins/data/vcPerms/config.yml`

```yaml
server: pumpkin
default-group: default
allow-ops: true
ops-override: false
commands-allow-ops: true
apply-chat-meta: false
debug-logins: false
temporary-add-behaviour: deny
```

| ключ | что делает |
| --- | --- |
| `server` | Контекст `server=` на каждой проверке |
| `default-group` | Выдаётся новым игрокам. Эту группу не удаляйте |
| `allow-ops` | Если у vcPerms нет ноды на проверку, оставляем решение хоста/OP |
| `ops-override` | OP считается как `*`, пока не сработала явная нода |
| `commands-allow-ops` | OP может писать `/vcp`, чтобы вообще зайти в систему |
| `apply-chat-meta` | Display name и имя в табе из префикса/суффикса. По умолчанию выкл |
| `debug-logins` | Лишняя строка в лог, когда игрока создали в сторе |
| `temporary-add-behaviour` | `deny`, `replace`, `accumulate` или `shadow`, если `settemp` бьёт в уже существующую временную ноду |

`/vcp reload` перечитывает этот файл и все json холдеров.

`ops-override` лучше выключить, как только появились нормальные группы. Иначе каждый OP обходит дерево, и кажется, что наследование сломано.
