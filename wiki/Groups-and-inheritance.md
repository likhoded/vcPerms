# Groups and inheritance

**English** · [Русский](ru/Groups-and-inheritance)

Users don't usually hold dozens of nodes. They sit in a group, the group sits on another group.

```
/vcp creategroup member
/vcp group member parent add default
/vcp group member permission set pumpkin.whatever true
/vcp user Alex parent add member
```

`parent add` is additive. `parent set` wipes every other `group.*` node first. A `group.X` node with `value: false` does not inherit that group.

Weight is only used when two inherited nodes fight:

```
/vcp group admin setweight 100
/vcp group mod setweight 50
```

Display name is cosmetic (`/vcp group admin setdisplayname Admin`). Also stored as a `displayname.*` node so exports stay self-contained.

`default` is created on first boot. `/vcp deletegroup default` is rejected. If you really want another default, change `default-group` in config and create that group before anyone joins.
