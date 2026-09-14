# Verbose

**English** · [Русский](ru/Verbose)

```
/vcp verbose on
/vcp verbose record Steve
# ... let them click things ...
/vcp verbose paste
```

`on` logs every permission check into a ring buffer (2000 entries). Each hit stores the **winning** node, not only the queried key. `record` clears the buffer and optionally filters by player name or node substring. `paste` / `upload` writes `exports/verbose-<unix>.json` and turns verbose off.

Use this when a command "doesn't work" and you have no idea which node the host asked for. Pumpkin's command tree is not Bukkit; look at the file.
