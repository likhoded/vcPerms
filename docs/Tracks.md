# Tracks

**English** · [Русский](ru/Tracks)

A track is an ordered list of groups. Promote / demote moves a user one step.

```
/vcp createtrack staff
/vcp track staff append helper
/vcp track staff append mod
/vcp track staff append admin
/vcp user Sam parent add helper
/vcp user Sam promote staff
```

`insert <group> <pos>` is 1-based.

`parent settrack <track> <group>` jumps to a specific rung and strips the other groups that belong to that track.

`parent cleartrack <track>` removes every group on the track from the user.

If the user isn't on the track, promote starts from the first group.
