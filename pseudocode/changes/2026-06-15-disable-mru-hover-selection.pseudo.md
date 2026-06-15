---
affects:
  - src/input/mod.rs
---

# Disable Recent Window Switcher Hover Selection

## Intent

Keep the recent windows switcher selection controlled by keyboard navigation or explicit pointer activation, not by incidental pointer hover while the switcher is open.

## Behavior

```pseudo
when pointer-like motion occurs while recent windows switcher is open:
    do not move the switcher selection based on the pointer position
    continue normal pointer motion processing for other UI and surfaces

when a left pointer click, tablet click, or touch press occurs while recent windows switcher is open:
    if press is on the switcher output and over a window thumbnail:
        select that thumbnail
        confirm the switcher
    else:
        cancel the switcher
```
