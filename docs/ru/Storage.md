# Хранение

[English](../Storage) · **Русский**

Обычные файлы. Один json на игрока / группу / трек.

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

`known-permissions.json` — каталог для таба. Плагины пишут туда через IPC `register` и через живые `hasPermission`. См. [API](API).

Пишем в `*.json.tmp` и потом rename, чтобы краш посреди сейва не оставил обрубок.

MySQL/Postgres/Mongo тут нет. Плагин на WASI. Нормальную базу надо тащить хост-хелпером, а не ещё одним крейтом здесь.

## Файл игрока

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

`uniqueId` / `primaryGroup` принимаются при импорте, чтобы старые дампы тоже вставали.

Оффлайн-игроки, созданные по нику, получают Java offline-mode UUID (`OfflinePlayer:<name>` MD5). Если потом зайдут с другим id — будет два файла. Склеивайте руками или `/vcp user A clone B`.
