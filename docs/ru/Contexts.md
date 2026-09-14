# Контексты

[English](../Contexts) · **Русский**

Ноду можно ограничить парой ключ/значение. Проверка должна предъявить ту же пару, иначе нода не применяется.

```
/vcp group vip permission set some.fly true world=world
/vcp user Steve permission set some.fly false server=pumpkin world=world_nether
```

Что ставим сами онлайн-игроку:

- `server` — из `config.yml`
- `world` — текущий мир
- `dimension` — текущее измерение (`overworld`, `the_nether`, `the_end`, …)

Несколько значений на **одном** сохранённом ключе — это OR. Разные ключи — AND.

Остальное пишете сами в конце команды как `key=value`.

Нода без контекста глобальная. Нода с контекстом работает только если в запросе есть эти ключи. `/vcp user … permission unset <node>` без контекста снимает только глобальную ноду, не world-копии.

Проверки с консоли идут с `server=<config>` и без мира.
