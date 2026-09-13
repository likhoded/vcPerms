# Группы и наследование

[English](../Groups-and-inheritance) · **Русский**

Игрокам обычно не вешают десятки нод. Они сидят в группе, группа сидит на другой группе.

```
/vcp creategroup member
/vcp group member parent add default
/vcp group member permission set pumpkin.whatever true
/vcp user Alex parent add member
```

`parent add` добавляет. `parent set` сначала сносит все остальные `group.*`.

Вес нужен только когда две унаследованные ноды спорят:

```
/vcp group admin setweight 100
/vcp group mod setweight 50
```

Отображаемое имя косметика (`/vcp group admin setdisplayname Admin`). Ещё пишется нодой `displayname.*`, чтобы экспорт был самодостаточным.

`default` создаётся при первом запуске. `/vcp deletegroup default` отвергается. Если хотите другой дефолт — смените `default-group` в конфиге и создайте группу до того, как кто-то зайдёт.
