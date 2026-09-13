# API

[English](../API) · **Русский**

Другие плагины Pumpkin могут ходить в vcPerms по IPC. Шлёте JSON, получаете JSON.

Id плагина — зарегистрированное имя: `vcPerms`.

```json
{"op":"check","user":"Steve","permission":"minecraft.command.gamemode"}
{"op":"prefix","user":"Steve"}
{"op":"suffix","user":"Steve"}
{"op":"primary","user":"Steve"}
{"op":"parents","user":"Steve"}
```

`user` — ник или uuid.

Ответы такие:

```json
{"ok":true,"allowed":true,"node":"*","source":"group/admin"}
{"ok":true,"prefix":"&c[Admin] "}
{"ok":false,"error":"unknown user"}
```

Пока весь API такой. Если нужно set/unset из другого плагина — issue или правьте json руками; перечитываем на `/vcp reload`.
