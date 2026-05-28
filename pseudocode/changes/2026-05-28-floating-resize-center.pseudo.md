---
affects:
  - src/layout/floating.rs
  - src/layout/tests/animations.rs
---

# Preserve Center For Non-Interactive Floating Resizes

## Intent

When a floating window changes size without an active interactive edge/corner resize, keep its visual center stable so centered floating windows remain centered after app-driven or command-driven resizes.

## Behavior

```pseudo
when floating space receives a committed window update:
  find the floating tile and its stored layout data

  determine whether an interactive resize is active for this window:
    first check floating-space interactive resize state
    otherwise check the window's interactive resize data

  remember previous logical position
  remember previous visual center
  remember previous size

  apply the window commit/update to refresh tile size and layout data

  if interactive resize is active:
    preserve existing edge/corner anchoring behavior from the current implementation
    if resizing from left edge:
      move window horizontally by old_width - new_width
    if resizing from top edge:
      move window vertically by old_height - new_height
    clamp resulting position through normal floating position rules
    return

  if no interactive resize is active and the floating tile size changed:
    set new top-left position to previous_center - new_size / 2
    clamp resulting position through normal floating position rules

    if a resize animation was started by the size change:
      animate the position delta using the window resize animation config

  otherwise:
    leave position unchanged
```

## Expected Outcomes

```pseudo
client-side/autonomous floating resize:
  size changes
  center remains stable

command-driven floating resize:
  size changes
  center remains stable

interactive pointer/touch floating resize:
  size changes according to selected edge/corner
  selected edge/corner anchor behavior is preserved
```
