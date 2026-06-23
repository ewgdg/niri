---
status: verified
affects:
  - src/protocols/screencopy.rs
  - src/handlers/mod.rs
  - src/niri.rs
  - src/backend/tty.rs
  - src/backend/headless.rs
---

# Screencopy Queue Cap and Black Drain

## Intent

Screencopy clients must not build unbounded pending GPU buffers when an output has no new damage or goes to sleep. Newer capture requests are fresher than older queued requests, and powered-off output capture should complete with black frames so client buffers are released.

## Behavior

### Cap queued copy-with-damage requests

Applies to: `src/protocols/screencopy.rs`

```pseudo
on enqueue_copy_with_damage(manager, screencopy):
  reject if screencopy is not copy_with_damage

  for each already_queued screencopy in this manager queue:
    if already_queued targets the same output as screencopy:
      remove already_queued from the queue
      fail already_queued frame by normal unsubmitted Screencopy drop

  mark or refresh cast tracking for screencopy output
  append screencopy to queue
  refresh cast tracking from the actual queue front

invariant:
  each screencopy manager queue has at most one queued copy_with_damage per output
  equivalently: one queued copy_with_damage per (ZwlrScreencopyManagerV1, Output)
```

### Preserve newest request semantics

Applies to: `src/protocols/screencopy.rs`, `src/niri.rs`

```pseudo
when a newer copy_with_damage request replaces an older queued request:
  older request receives failed event by normal unsubmitted-frame cleanup
  older request releases its client buffer reference
  newer request waits for the next relevant damage/redraw

when rendering queued copy_with_damage for an output:
  process the current front queued request as before
  if damage exists:
    render into that request buffer and submit ready
  if no damage exists:
    keep that request queued until future damage/redraw or replacement
```

### Drain pending screencopy as black frames

Applies to: `src/niri.rs`, `src/protocols/screencopy.rs`, `src/backend/tty.rs`, `src/backend/headless.rs`

```pseudo
submit_powered_off_screencopies_for_outputs(renderer, outputs):
  queued = []
  for each output in outputs:
    queued += drain queued screencopies for output

  for each screencopy in queued:
    submit_black_screencopy(screencopy)

  if any screencopy was submitted successfully:
    reset screencopy damage tracking so wake-up reports full real-frame damage

on output_sleep_or_power_off(output) where a current renderer is available:
  submit_powered_off_screencopies_for_outputs(renderer, [output])

on managed_virtual_output_config_off:
  before removing the output from the compositor space:
    ask backend for primary renderer
    if renderer exists:
      submit_powered_off_screencopies_for_outputs(renderer, [output])
    if renderer is absent:
      accept the removed-output fallback for this config-removal path
      remove output normally, which drains queued screencopies without rendering
      clients observe the existing failed-frame behavior for removed outputs

on all_monitors_power_off:
  set monitors inactive
  ask backend for primary renderer
  if renderer exists:
    submit_powered_off_screencopies_for_outputs(renderer, every connected output)
  if renderer is absent:
    keep queued screencopies rather than draining and failing them

on session_pause_or_sleep:
  before suspending input and pausing DRM devices:
    ask backend for primary renderer
    if renderer exists:
      submit_powered_off_screencopies_for_outputs(renderer, every connected output)
    if renderer is absent:
      keep queued screencopies rather than draining and failing them
```

### Screencopy received while powered off

Applies to: `src/handlers/mod.rs`, `src/niri.rs`

```pseudo
on screencopy_request(screencopy):
  if output no longer exists:
    keep existing missing-output behavior
    fail by dropping unsubmitted screencopy
    do not black-submit
    return

  if output is powered off or all monitors are inactive:
    if screencopy uses copy_with_damage:
      mark cast active so timeout does not expire while asleep

    submit_black_screencopy(screencopy)

    if black frame was sent for copy_with_damage:
      reset screencopy damage tracker so wake-up sends full real-frame damage

    return

  if screencopy uses copy_with_damage:
    enqueue_copy_with_damage(manager, screencopy)
  else:
    render normal screencopy immediately
```

### Removed output behavior

Applies to: `src/protocols/screencopy.rs`, `src/niri.rs`, `src/handlers/mod.rs`

```pseudo
on output_removed(output):
  keep existing behavior
  drain queued screencopies for output without rendering
  fail drained frames by normal unsubmitted Screencopy drop

on request_for_missing_output:
  keep existing behavior
  drop unsubmitted screencopy so client sees failed
```

## Notes

- This change does not coalesce frame results across different client buffers.
- It intentionally fails older queued requests rather than newer requests because the newest request carries fresher client state.
- Black-frame submission follows existing DPMS-off behavior: damage whole buffer for copy-with-damage, clear DMABUF/SHM to opaque black, and send ready after any GPU sync.
- This proposal does not add a new virtual-output sleep API. Managed virtual-output config-off is an output-removal path; when no renderer is available there, it intentionally uses existing removed-output failure behavior rather than preserving queued frames.
- Verified with `cargo test screencopy --no-default-features`, `cargo check --no-default-features`, and `git diff --check`; screencopy tests cover queue replacement, DPMS black drain, managed virtual-output `off` black drain, and removed-output failure.
