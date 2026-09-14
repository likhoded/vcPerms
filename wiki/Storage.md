# Storage

**English** · [Русский](ru/Storage)

Flat files. One json per user / group / track.

```
plugins/data/vcPerms/
  config.yml
  uuidcache.json
  known-permissions.json
  users/24392406-6f57-3484-97a6-59cc90bc7cbd.json
  groups/admin.json
  tracks/staff.json
  exports/
  verbose/
```

`known-permissions.json` is the tab-complete catalog. Plugins fill it via IPC `register` and via live `hasPermission` checks. See [API](API).

Writes go to `*.json.tmp`, then the destination is replaced (remove + rename, with a direct-write fallback on Windows). A crash mid-save shouldn't leave a half file. Broken json is logged and skipped.

User files load on join and unload on leave. Search, export, delete/rename group, and member lists load everyone first.

If `/vcp user Steve` created an offline-UUID ghost before the real login, join merges that file into the real UUID and deletes the ghost.

There is no MySQL/Postgres/Mongo here. The plugin is WASI. A real database would need a host-side helper, not more rust in this crate.

## User file

```json
{
  "unique_id": "24392406-6f57-3484-97a6-59cc90bc7cbd",
  "name": "Steve",
  "primary_group": "admin",
  "nodes": [
    { "key": "group.admin", "value": true },
    { "key": "essentials.fly", "value": true, "expiry": 1735689600 }
  ]
}
```

`uniqueId` / `primaryGroup` are accepted on import so older dumps still load.

Offline players created by name get a Java offline-mode UUID (`OfflinePlayer:<name>` MD5). On a later real join, that ghost file is merged into the true UUID.
