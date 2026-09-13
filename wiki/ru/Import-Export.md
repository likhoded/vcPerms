# Импорт / экспорт

[English](../Import-Export) · **Русский**

```
/vcp export backup.json
/vcp import backup.json
/vcp editor
/vcp applyedits editor.json
```

Файлы ищем относительно data-папки плагина, потом в `exports/`.

Дамп выглядит так:

```json
{
  "vcperms": "0.8.4",
  "users": [ ... ],
  "groups": [ ... ],
  "tracks": [ ... ]
}
```

Это наш формат. Если переезжаете со старого Paper, JSON-экспорт LuckPerms с теми же объектами нод (`key`, `value`, `expiry`, `context`) обычно импортируется нормально. SQL-дампы, messaging service и сессии веб-редактора игнорируются.

Хостед веб-редактора нет. Правите файл и `applyedits`.
