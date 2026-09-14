# Права

[English](../Permissions) · **Русский**

## Как считается проверка

1. Собираем свои ноды игрока и все родительские группы, рекурсивно. Циклы игнорируются.
2. Выкидываем просроченные ноды и те, чей контекст не совпал с запросом.
3. Оставляем ноды, которые покрывают проверяемое право:
   - точный ключ
   - `foo.*` покрывает `foo.bar` / `foo.bar.baz` и хостовые ноды вроде `foo:command`
   - `foo:*` покрывает `foo:command`
   - `*` покрывает всё
4. Берём лучшее совпадение: более конкретное бьёт вайлдкард, свои ноды бьют унаследованные, временная нода бьёт постоянную с тем же spec, больший вес группы разводит ничью.
5. `value` этой ноды — результат. `false` это запрет. Нода `group.X` с `value: false` группу **не** наследует.

Если ничего не совпало:

- `ops-override: true` и игрок OP (уровень 2+) → разрешить
- иначе `allow-ops: true` оставляет то, что уже решил Pumpkin (обычно OP / ванильный уровень)
- иначе запрет

## Ноды команд

Их смотрим, когда игрок пишет `/vcp`. Консоль не проверяем.

Несколько из тех, что реально смотрим:

- `vcperms.user.info`
- `vcperms.user.permission.set` / `.unset` / `.check`
- `vcperms.user.parent.add` / `.remove`
- `vcperms.user.meta.set` / `.unset`
- `vcperms.user.promote` / `vcperms.user.demote`
- `vcperms.group.*` (та же схема)
- `vcperms.track.info`
- `vcperms.creategroup` / `vcperms.deletegroup`
- `vcperms.verbose`
- `vcperms.import` / `vcperms.export`

Выдайте стаффу `vcperms.*` и забудьте.

Плагин ещё регистрирует у хоста `vcPerms:command`, чтобы дерево команд вообще существовало. Это просто «можно набрать /vcp», не мелкие ноды выше. `vcperms.*` эту хостовую ноду покрывает.

## Вайлдкарды

```
/vcp group admin permission set * true
/vcp group builder permission set worldedit.* true
/vcp group builder permission set worldedit.navigation.* false
```

Запрет на `worldedit.navigation.*` побеждает `worldedit.*`, потому что он длиннее.
