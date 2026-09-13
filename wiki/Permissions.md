# Permissions

**English** · [Русский](ru/Permissions)

## How a check is resolved

1. Collect the user's own nodes and every parent group, recursively. Cycles are ignored.
2. Drop expired nodes and nodes whose context doesn't match the query.
3. Keep nodes that cover the queried permission:
   - exact key
   - `foo.*` covering `foo.bar` / `foo.bar.baz`
   - `*` covering everything
4. Pick the best match: more specific beats wildcard, own nodes beat inherited, higher group weight breaks ties.
5. The `value` of that node is the result. `false` is a deny.

If nothing matched:

- `ops-override: true` and the player is OP → allow
- otherwise `allow-ops: true` keeps whatever Pumpkin already decided (usually OP / vanilla level)
- else deny

## Command nodes

These are checked when a player runs `/vcp`. Console skips them.

A few of the ones we look at:

- `vcperms.user.info`
- `vcperms.user.permission.set`
- `vcperms.user.parent.add`
- `vcperms.user.meta.set`
- `vcperms.user.promote` / `vcperms.user.demote`
- `vcperms.group.*` (same idea)
- `vcperms.track.info`
- `vcperms.creategroup` / `vcperms.deletegroup`
- `vcperms.verbose`
- `vcperms.import` / `vcperms.export`

Give a staff group `vcperms.*` and be done with it.

The plugin also registers `vcperms.command` with the host so the command tree exists. That's just "can type /vcp", not the fine-grained nodes above.

## Wildcards

```
/vcp group admin permission set * true
/vcp group builder permission set worldedit.* true
/vcp group builder permission set worldedit.navigation.* false
```

The deny on `worldedit.navigation.*` wins over `worldedit.*` because it's longer.
