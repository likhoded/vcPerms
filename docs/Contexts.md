# Contexts

**English** · [Русский](ru/Contexts)

A node can be limited to a key/value pair. The check has to present the same pair or the node is ignored.

```
/vcp group vip permission set some.fly true world=world
/vcp user Steve permission set some.fly false server=pumpkin world=world_nether
```

Keys we set automatically on online players:

- `server` — from `config.yml`
- `world` — current world name
- `dimension` — current dimension (`overworld`, `the_nether`, `the_end`, …)

Several values on the **same** stored key are OR. Different keys are AND.

Everything else you type yourself as `key=value` at the end of a command.

A stored node with no context is global. A stored node with context only applies when the query has those keys. `/vcp user … permission unset <node>` without context removes the global node only, not world-scoped copies.

Console checks use `server=<config>` and no world.
