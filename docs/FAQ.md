# FAQ

**English** · [Русский](ru/FAQ)

**`/vcp` says unknown command.**  
The client command tree on Pumpkin behind Velocity is still flaky. Use RCON or type it anyway — some clients still send it. If the plugin log shows the command, it's a client sync issue, not missing OP.

**Why is OP still bypassing groups?**  
`allow-ops` or `ops-override`. Turn both off after you give yourself `vcperms.*`.

**Can I import an old LuckPerms dump?**  
Yes, if it's JSON with `users` / `groups` / `tracks` and the usual node objects. See [Import / export](Import-Export).

**SQL?**  
Not in this build. See [Storage](Storage).

**Does it sync across a proxy?**  
No messaging service. Each Pumpkin instance has its own data folder. Copy files or import/export if you really need the same groups on two worlds.

**I deleted default and now nobody has groups.**  
Don't. If you already did, `/vcp creategroup default` and `/vcp reload`. New joins will pick it up. Existing users need `parent add default` if they have zero groups.
