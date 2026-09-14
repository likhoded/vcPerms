# Changelog

## 0.8.5

- yellow `[vcPerms]` prefix on every player-facing chat line
- IPC `register` / `announce` for plugin permission catalogs
- known-permissions.json + tab-complete from the live catalog
- Brigadier tree for `/vcp user|group|track`
- documented plugin API (wiki / docs)

## 0.8.4

- atomic json writes (`*.tmp` + rename)
- verbose paste writes a file instead of pretending it can hit a paste site
- track insert is 1-based again

## 0.8.3

- `temporary-add-behaviour: accumulate` actually extends expiry
- uuid cache updated on join so `/vcp user` works with current names

## 0.8.0

- first public Pumpkin build (26.2 / protocol 776)
