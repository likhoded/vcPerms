# Commands

**English** · [Русский](ru/Commands)

Root command is `/vcp`. `/vcperms` is the same tree.

Quoted strings are allowed: `/vcp group admin meta addprefix 100 "&c[Admin] "`

## Root

| command | |
| --- | --- |
| `/vcp` / `/vcp help` | short usage |
| `/vcp info` | version, counts, storage |
| `/vcp reload` | reread disk |
| `/vcp sync` | flush memory to disk |
| `/vcp check <user> <node> [ctx...]` | resolve one permission |
| `/vcp search <query> [page]` | find holders that have a node |
| `/vcp tree [user\|group] <name>` | inheritance dump |
| `/vcp verbose on\|off\|record [filter]\|paste` | see [Verbose](Verbose) |
| `/vcp editor` | write a full dump to `exports/editor.json` |
| `/vcp export [file]` | same, custom name |
| `/vcp import <file>` / `/vcp applyedits <file>` | load a dump |
| `/vcp creategroup <name>` | |
| `/vcp deletegroup <name>` | refuses to delete `default` |
| `/vcp createtrack <name>` | |
| `/vcp deletetrack <name>` | |
| `/vcp listgroups` | weight-sorted |
| `/vcp listusers [page]` | |
| `/vcp listtracks` | |

## User

`/vcp user <name|uuid> ...`

`info`, `permission`, `parent`, `meta`, `editor`, `promote <track>`, `demote <track>`, `showtracks`, `clear [ctx...]`, `clone <other>`

### permission

- `info [page]`
- `set <node> [true\|false] [ctx...]`
- `unset <node> [ctx...]`
- `settemp <node> <duration> [true\|false] [ctx...]`
- `unsettemp <node> [ctx...]`
- `check <node> [ctx...]`
- `clear [ctx...]`

### parent

- `info`
- `add <group> [ctx...]`
- `remove <group> [ctx...]`
- `set <group>` — wipes other groups
- `addtemp <group> <duration> [ctx...]`
- `removetemp <group> [ctx...]`
- `clear [ctx...]`
- `cleartrack <track>`
- `switchprimarygroup <group>`
- `settrack <track> <group>`

### meta

- `info`
- `set <key> <value> [ctx...]`
- `unset <key>`
- `settemp <key> <value> <duration> [ctx...]`
- `unsettemp <key>`
- `addprefix <priority> <text> [ctx...]`
- `removeprefix <priority>`
- `addsuffix` / `removesuffix`
- `addtempprefix` / `addtempsuffix`

## Group

`/vcp group <name> ...`

Same `permission` / `parent` / `meta` surface as users, plus:

- `listmembers [page]`
- `setweight <n>`
- `setdisplayname <name>`
- `showtracks`
- `rename <new>`
- `clone <new>`
- `clear [ctx...]`

## Track

`/vcp track <name> info|append|insert|remove|clear|rename|clone`

`insert` takes a 1-based position.

## Durations

`30s`, `15m`, `2h`, `1d`, `1w`, `1mo`, `1y`, or glued: `1d12h`.
