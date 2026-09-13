# Import / export

**English** · [Русский](ru/Import-Export)

```
/vcp export backup.json
/vcp import backup.json
/vcp editor
/vcp applyedits editor.json
```

Files are resolved relative to the plugin data folder, then `exports/`.

The dump looks like:

```json
{
  "vcperms": "0.8.4",
  "users": [ ... ],
  "groups": [ ... ],
  "tracks": [ ... ]
}
```

That's the native format. If you are migrating an old Paper world, a LuckPerms JSON export with the same node objects (`key`, `value`, `expiry`, `context`) usually imports cleanly. SQL dumps, messaging-service state and web-editor sessions are ignored.

There is no hosted web editor. Edit the file, `applyedits` it.
