---
affects:
  - src/input/mod.rs
  - src/niri.rs
  - src/ui/mru.rs
---

# Disable Recent Window Switcher Hover Selection

## Intent

Keep the recent windows switcher selection controlled by keyboard navigation or explicit pointer activation, not by incidental pointer hover while the switcher is open.

## Behavior

```pseudo
when pointer-like motion occurs while recent windows switcher is open:
    do not move the switcher selection based on the pointer position
    continue normal pointer motion processing for other UI and surfaces

when a left pointer press, tablet tip down, or touch down occurs while recent windows switcher is open:
    if press is on the switcher output and over a window thumbnail:
        select that thumbnail
        redraw the switcher so the selected-thumbnail highlight updates
        keep the switcher open
        suppress this input sequence from normal clients
    else:
        cancel the switcher
        suppress the matching release from normal clients

when the matching left pointer release, tablet tip up, or touch up occurs after a thumbnail press selection:
    if release is still over the pressed thumbnail:
        confirm the switcher selection from the press
    else:
        cancel the switcher without switching
    do not send the release to normal clients
```
