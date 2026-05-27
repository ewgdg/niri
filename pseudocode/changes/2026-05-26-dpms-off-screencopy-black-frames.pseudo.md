---
status: verified
affects:
  - src/backend/headless.rs
  - src/backend/tty.rs
  - src/handlers/mod.rs
  - src/niri.rs
  - src/protocols/screencopy.rs
  - src/render_helpers/mod.rs
---

# DPMS-Off Screencopy Black Frames

## Intent

Powering off monitors should stop normal rendering. Screencopy clients like Sunshine should still receive valid black frames instead of real desktop frames or failed frames, so streams stay alive and buffers are released.

## Behavior

### DPMS-off suppresses normal virtual output work

Applies to: `src/backend/headless.rs`, `src/backend/tty.rs`, `src/niri.rs`

```pseudo
before:
  on power_off_monitors:
    monitors_active = false
    backend.set_monitors_active(false)

  after render is skipped because monitors_active == false:
    still send surface frame callbacks
    still process queued screencopy captures

  on virtual-output frame timer:
    if no unfinished animations:
      still send virtual-output frame callbacks

after:
  on power_off_monitors:
    monitors_active = false
    backend.set_monitors_active(false)

  while monitors_active == false:
    suppress normal output rendering
    suppress normal surface frame callbacks
    do not process normal queued screencopy captures
    do not keep virtual-output frame timers alive for desktop content
```

### DPMS-off screencopy becomes black-frame capture

Applies to: `src/handlers/mod.rs`, `src/niri.rs`, `src/protocols/screencopy.rs`

```pseudo
before:
  on screencopy_request(screencopy):
    if output no longer exists:
      keep existing missing-output behavior
      return

    if screencopy uses copy_with_damage:
      queue until redraw damage
    else:
      render normal screencopy now

after:
  on screencopy_request(screencopy):
    if output no longer exists:
      keep existing missing-output behavior
      return

    if monitors_active == false:
      if screencopy uses copy_with_damage:
        mark cast active so timeout does not expire while DPMS is off

      submit_black_screencopy(screencopy)

      if black frame was sent for copy_with_damage:
        reset screencopy damage tracker so wake-up sends full real-frame damage

      return

    if screencopy uses copy_with_damage:
      queue until redraw damage
    else:
      render normal screencopy now
```

### Monitor power-off drains queued screencopy

Applies to: `src/niri.rs`, `src/protocols/screencopy.rs`

```pseudo
before:
  on power_off_monitors:
    leave queued screencopies in screencopy queues

after:
  on power_off_monitors:
    queued = drain queued screencopies for all outputs
    for each screencopy in queued:
      submit_black_screencopy(screencopy)

    reset screencopy damage trackers so wake-up sends full real-frame damage
```

### Black-frame submission

Applies to: `src/niri.rs`, `src/render_helpers/mod.rs`, `src/protocols/screencopy.rs`

```pseudo
submit_black_screencopy(screencopy):
  if screencopy uses copy_with_damage:
    send damage covering whole buffer

  if target buffer is dmabuf:
    clear dmabuf to opaque black
    send ready after GPU sync

  if target buffer is shm:
    validate XRGB8888 size/stride
    fill pixels with opaque black
    send ready now
```

### Queue draining and cast tracking

Applies to: `src/protocols/screencopy.rs`

```pseudo
drain queued screencopies for output:
  remove matching queued screencopies
  return removed screencopies
  keep other outputs queued
  update cast tracking from remaining queue or refresh deadline

touch cast tracking while DPMS-off:
  if cast exists:
    refresh deadline
  else:
    create cast tracking for output

reset damage tracker:
  replace tracker with fresh zero-size tracker
  next active copy_with_damage reports full damage
```

## Notes

- `before:` describes repository `HEAD` before this patch, not intermediate failed attempts.
- Black screencopy never exposes desktop content.
- Invalid buffer behavior stays unchanged.
- Verified with `cargo check` and `git diff --check`.
