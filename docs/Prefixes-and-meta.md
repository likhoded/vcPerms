# Prefixes, suffixes, meta

**English** · [Русский](ru/Prefixes-and-meta)

Prefix, suffix and arbitrary meta are stored as nodes.

| command | node |
| --- | --- |
| `meta addprefix 100 "&c[Admin] "` | `prefix.100.&c[Admin] ` |
| `meta addsuffix 10 "&7"` | `suffix.10.&7` |
| `meta set clan wolves` | `meta.clan.wolves` |

Highest priority prefix/suffix wins. `/vcp user Steve meta info` lists what they actually hold. `/vcp user Steve info` shows the resolved ones if we can see a context.

vcPerms does **not** rewrite the chat message unless you want a nametag. With `apply-chat-meta: true` it sets the player's display name and tab list from prefix/suffix. If another plugin already formats chat, leave it off and read the prefix over [IPC](API).
