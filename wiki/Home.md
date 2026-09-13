# vcPerms

**English** · [Русский](ru/Home)

Permission manager for Pumpkin. Users, groups, tracks, contexts. Every `hasPermission` check from the host goes through us.

## Pages

- [Installation](Installation)
- [Config](Config)
- [Commands](Commands)
- [Permissions](Permissions)
- [Groups and inheritance](Groups-and-inheritance)
- [Contexts](Contexts)
- [Tracks](Tracks)
- [Prefixes and meta](Prefixes-and-meta)
- [Storage](Storage)
- [Import / export](Import-Export)
- [Verbose](Verbose)
- [API](API)
- [FAQ](FAQ)

## Typical setup

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

Then `/vcp user <name> promote ranks` when someone pays or you hire staff.
