# vcPerms

[English](../Home) · **Русский**

Менеджер прав для Pumpkin. Игроки, группы, треки, контексты. Каждая проверка `hasPermission` от хоста идёт через нас.

## Страницы

- [Установка](Installation)
- [Конфиг](Config)
- [Команды](Commands)
- [Права](Permissions)
- [Группы и наследование](Groups-and-inheritance)
- [Контексты](Contexts)
- [Треки](Tracks)
- [Префиксы и мета](Prefixes-and-meta)
- [Хранение](Storage)
- [Импорт / экспорт](Import-Export)
- [Verbose](Verbose)
- [API](API)
- [FAQ](FAQ)

## Обычная настройка

```
/vcp creategroup member
/vcp creategroup vip
/vcp creategroup admin
/vcp group member parent add default
/vcp group vip parent add member
/vcp group admin parent add vip
/vcp group admin permission set * true
/vcp createtrack ranks
/vcp track ranks append default
/vcp track ranks append member
/vcp track ranks append vip
/vcp track ranks append admin
```

Дальше `/vcp user <name> promote ranks`, когда кто-то задонатил или вы взяли человека в стафф.
