# API

**English** · [Русский](ru/API)

Other Pumpkin plugins can talk to vcPerms over IPC. Send JSON, get JSON.

Plugin id is the registered name: `vcPerms`.

```json
{"op":"check","user":"Steve","permission":"minecraft.command.gamemode"}
{"op":"prefix","user":"Steve"}
{"op":"suffix","user":"Steve"}
{"op":"primary","user":"Steve"}
{"op":"parents","user":"Steve"}
```

`user` can be a name or a uuid.

Replies look like:

```json
{"ok":true,"allowed":true,"node":"*","source":"group/admin"}
{"ok":true,"prefix":"&c[Admin] "}
{"ok":false,"error":"unknown user"}
```

That's the whole surface for now. If you need set/unset from another plugin, open an issue or just write the json files; we reload on `/vcp reload`.
