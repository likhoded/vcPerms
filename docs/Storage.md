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

Writes go to `*.json.tmp` and then rename, so a crash mid-save shouldn't leave a half file.

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

Offline players created by name get a Java offline-mode UUID (`OfflinePlayer:<name>` MD5). If they later join with a different id you'll have two files — merge by hand or `/vcp user A clone B`.
