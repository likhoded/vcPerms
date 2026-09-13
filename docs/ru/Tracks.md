# Треки

[English](../Tracks) · **Русский**

Трек — упорядоченный список групп. Promote / demote двигает игрока на одну ступень.

```
/vcp createtrack staff
/vcp track staff append helper
/vcp track staff append mod
/vcp track staff append admin
/vcp user Sam parent add helper
/vcp user Sam promote staff
```

`insert <group> <pos>` считает с единицы.

`parent settrack <track> <group>` прыгает на конкретную ступень и снимает остальные группы этого трека.

`parent cleartrack <track>` снимает с игрока все группы трека.

Если игрока на треке нет, promote начинается с первой группы.
