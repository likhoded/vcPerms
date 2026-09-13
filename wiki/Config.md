# Config

**English** · [Русский](ru/Config)

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

| key | what it does |
| --- | --- |
| `server` | Default `server=` context on every check |
| `default-group` | Given to new users. Don't delete this group |
| `allow-ops` | If vcPerms has no node for a check, keep the host/OP result |
| `ops-override` | Treat OP as `*` unless a node actually matched |
| `commands-allow-ops` | Let OP run `/vcp` so you can bootstrap |
| `apply-chat-meta` | Prepend prefix/suffix to chat. Off by default |
| `debug-logins` | Extra log line when a player is ensured |
| `temporary-add-behaviour` | `deny`, `replace` or `accumulate` when `settemp` hits an existing temp node |

`/vcp reload` rereads this file and every holder json.

Leave `ops-override` off once you have real groups. Otherwise every OP still bypasses the tree and you'll think inheritance is broken.
