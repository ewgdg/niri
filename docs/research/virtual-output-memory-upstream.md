# Upstream research: virtual-output memory growth

**Checked:** 2026-08-29. Upstream Niri: `dd75865f547f0eac0e9b6c4d86d2cd00c0744252` (`upstream/main`). Smithay: the revision pinned by this fork, `4cf0b62028039661477d482ec4758b687d8f4392`.

## Bottom line

There is no confirmed upstream fix for this exact Niri anonymous-heap growth, and upstream Niri does not yet contain the fork's virtual-output implementation. Two upstream bug classes are close enough to test directly:

1. Monitor-off redraws can accumulate callbacks and client buffers without presentation backpressure. Niri [#3295](https://github.com/niri-wm/niri/issues/3295), [#1742](https://github.com/niri-wm/niri/issues/1742), discussion [#3691](https://github.com/niri-wm/niri/discussions/3691), and closed-unmerged PR [#3910](https://github.com/niri-wm/niri/pull/3910) document growth into tens of gigabytes. Most measurements there are GPU or shared-memory allocations that drain when monitors wake, unlike the observed 28 GB glibc heap. The underlying callback/backpressure failure is still relevant.
2. Smithay selection devices can accumulate when clients disconnect without explicitly destroying them, causing both memory growth and progressively slower selection operations. Niri [#2430](https://github.com/niri-wm/niri/issues/2430) contains telemetry and a source-level diagnosis. Smithay commit [`c7cd09492`](https://github.com/Smithay/smithay/commit/c7cd09492bd16ea9afbd0c89191426b48ff160a3) adds cleanup, but it is only on an upstream testing branch. It is absent from the fork's pinned Smithay `4cf0b620`, current Smithay `master`, and upstream Niri. This matches anonymous heap plus multi-day lag better than the GPU reports, but no evidence yet ties it to GoldenDict or Sunshine.

Fork-local commit [`935eb7d4`](https://github.com/ewgdg/niri/commit/935eb7d41c209147d6b045c4aa2f8d711b647e95) fixes a real pre-existing virtual-output feedback-drain bug. It computes render-element states, updates primary-scanout visibility, then drains feedbacks. Because the unhealthy session started on that commit, it cannot yet be called the fix or the cause of the observed 28 GB heap. Its new full-scene visibility pass remains a possible source of allocation churn.

## Exact and strong matches

### Virtual-output presentation feedback (fork-local fix; not upstream)

Smithay documents that compositors should drain committed presentation feedback before sending frame callbacks ([presentation module](https://github.com/Smithay/smithay/blob/4cf0b62028039661477d482ec4758b687d8f4392/src/wayland/presentation/mod.rs#L45-L72)). Its collector only removes callbacks when the surface's primary scanout output matches ([`take_presentation_feedback_surface_tree`](https://github.com/Smithay/smithay/blob/4cf0b62028039661477d482ec4758b687d8f4392/src/desktop/wayland/utils.rs#L434-L472)); callbacks are stored in `PresentationFeedbackCachedState.callbacks` ([source](https://github.com/Smithay/smithay/blob/4cf0b62028039661477d482ec4758b687d8f4392/src/wayland/presentation/mod.rs#L296-L345)).

Local [`935eb7d4`](https://github.com/ewgdg/niri/commit/935eb7d41c209147d6b045c4aa2f8d711b647e95) fixes exactly this for virtual outputs. Its parent path passed default/empty render states; the new path renders logical elements, updates primary-output state, and then drains feedback. This is the closest source match for Sunshine + virtual output + presentation callbacks, but it describes the path before `935eb7d4`. The observed session ran `935eb7d4`, so a clean A/B run is still required.

Smithay commit [`5420c222`](https://github.com/Smithay/smithay/commit/5420c2225c66ed12d3e2faa3ec5fe36075051e51), merged via [PR #1713](https://github.com/Smithay/smithay/pull/1713), intentionally preserves existing feedback when an empty subsurface merge occurs. It fixes discarded feedback for subsurface trees, not a leak, but means a collector/visibility mistake can accumulate callbacks instead of having them accidentally discarded. [Smithay #1711](https://github.com/Smithay/smithay/issues/1711) is the corresponding confirmed feedback bug.

### Monitor-off redraw loop and screencopy retention (upstream issue; adjacent but highly relevant)

[Niri #3295](https://github.com/niri-wm/niri/issues/3295) remains open. Reports show VRAM/GTT growing up to about 20 MB/s while monitors are powered off, then freeing when they wake. A later code-reading comment identifies the same missing backpressure pattern: upstream `Niri::redraw()` skips `backend.render()` when monitors are inactive but still sends frame callbacks and renders casts; TTY's estimated-vblank throttle is only scheduled inside `Tty::render()` ([upstream source at `dd75865f`](https://github.com/niri-wm/niri/blob/dd75865f547f0eac0e9b6c4d86d2cd00c0744252/src/niri.rs#L4630-L4738)). Callback-driven client commits can therefore requeue redraws without a real VBlank.

[Niri PR #3910](https://github.com/niri-wm/niri/pull/3910), **closed unmerged**, proposes the relevant fix: throttle the inactive path, skip frame callbacks/fallback callbacks while monitors are off, and clear animation state. Its branch commits ([example](https://github.com/niri-wm/niri/commit/164c9575cdb37ee8e57951eea7dac3ce957579c2)) are not in upstream `dd75865f`.

The fork already carries part of this defense. Commit [`c3a7657e`](https://github.com/ewgdg/niri/commit/c3a7657e05c07090fd17f7d77ae9a1a0b4d75909) pauses fallback callbacks while monitors are inactive, and the current redraw path returns before sending frame callbacks or casts. It does not carry PR #3910's inactive estimated-vblank scheduling or explicit animation-state clearing. This makes #3910 useful as a comparison, not a drop-in known fix.

Upstream `dd75865f` also appends every `copy_with_damage` request to the screencopy queue ([source](https://github.com/niri-wm/niri/blob/dd75865f547f0eac0e9b6c4d86d2cd00c0744252/src/protocols/screencopy.rs#L123-L140)); each queued frame owns its client buffer until it is processed or dropped ([`Drop`](https://github.com/niri-wm/niri/blob/dd75865f547f0eac0e9b6c4d86d2cd00c0744252/src/protocols/screencopy.rs#L627-L642)). Thus a client can retain an unbounded number of buffers when no damage arrives. The fork's [`4705744e`](https://github.com/ewgdg/niri/commit/4705744e68b1c2e385cf7ad3bae0ac2403ac42d9) caps virtual-output queued frames, but that is also branch-local.

## Adjacent upstream fixes

- [Niri #1869](https://github.com/niri-wm/niri/issues/1869) / [PR #3404](https://github.com/niri-wm/niri/pull/3404), merged as [`6d5c5f12`](https://github.com/niri-wm/niri/commit/6d5c5f12b2a6ed39cf750eb67aea72f50e00fa1f): dead-surface dmabuf pre-commit hooks retained buffers. Confirmed GPU-resource leak, but not virtual-output/presentation-specific.
- [Smithay PR #2080](https://github.com/Smithay/smithay/pull/2080), merged as [`d1da9512`](https://github.com/Smithay/smithay/commit/d1da95126d8231da170364fccb4ea1babf9707b1): thread-safe exported-dmabuf/GBM caches, PBO error cleanup, multi-GPU surface-cache cleanup, and orphan EGLImage cleanup. Relevant to VRAM/resource retention; not an explanation for callback-driven process heap growth.
- [Smithay PR #1976](https://github.com/Smithay/smithay/pull/1976): `OutputModeSource` now uses `WeakOutput`, preventing a damage tracker from retaining removed outputs. Relevant to output removal lifecycle, not steady per-frame growth.
- [Smithay PR #1928](https://github.com/Smithay/smithay/pull/1928), merged as [`3d3f9e35`](https://github.com/Smithay/smithay/commit/3d3f9e359352d95cffd1e53287d57df427fcbd34): removed `mem::forget` from Smithay's **ext-image-copy-capture** `Frame`; the old code leaked completed frames. This is a real leak, but Sunshine's Niri path here is wlr-screencopy/virtual-output, so it is not the direct match. [PR #1925](https://github.com/Smithay/smithay/pull/1925) was closed because #1928 was the proper fix.

### Selection-device retention and long-session lag

[Niri #2430](https://github.com/niri-wm/niri/issues/2430) reports that clipboard and primary-selection operations become hundreds of milliseconds slower after days. A contributor instrumented Smithay and found dead selection devices accumulating when clients disconnected without sending the protocol destroy request. The proposed Smithay cleanup commit [`c7cd09492`](https://github.com/Smithay/smithay/commit/c7cd09492bd16ea9afbd0c89191426b48ff160a3) removes dead wl-data-device, primary-selection, ext-data-control, and wlr-data-control devices from seat state in each resource's `destroyed()` callback.

The cleanup commit is not in Smithay `master` as of `e3d461a057ba244d213a8498ec372b0799cca103`, and it is not in Niri's pinned `4cf0b620`. It is therefore an available experimental patch, not an upstream release fix. This candidate predicts growth proportional to short-lived selection-capable Wayland clients and worsening clipboard/selection latency, independent of virtual-output frame rate.

## Virtual-output and Sunshine status

[Niri PR #3800](https://github.com/niri-wm/niri/pull/3800) and duplicate [PR #3953](https://github.com/niri-wm/niri/pull/3953) explicitly target Sunshine/Moonlight, wayvnc, and headless outputs, but both are open/unmerged at `dd75865f`. Therefore upstream has no merged virtual-output implementation against which to claim a fix.

[Niri #3816](https://github.com/niri-wm/niri/issues/3816) is a Sunshine/Moonlight OOM report, but it concerns Sunshine KMS capture plus injected virtual-pointer/cursor-device allocation and was closed as a Smithay DRM issue. It does not involve virtual outputs or presentation callbacks and is a false positive for this investigation.

## False positives / not root-cause evidence

- [Niri #2441](https://github.com/niri-wm/niri/issues/2441): suspected SHM buffer release after screencopy, but the reporter later acknowledged the alleged spec violation was an inference; [PR #4217](https://github.com/niri-wm/niri/pull/4217) is still open.
- [Niri #4257](https://github.com/niri-wm/niri/issues/4257): ambiguous multi-output wlr-screencopy queue semantics, not a memory-growth report.
- [Smithay #1562](https://github.com/Smithay/smithay/issues/1562): broad Niri/Cosmic VRAM growth after closing windows; useful context for renderer lifetime bugs, but no virtual-output/Sunshine link and still unresolved upstream.
- [Niri #1742](https://github.com/niri-wm/niri/issues/1742): 27 GB OOM peak after a long session. Later reports established Niri-owned DRM/shared-memory growth during monitor-off periods, but not a 27 GB anonymous glibc heap.

## Post-restart local reproduction

After restarting, Niri began at 220 MiB RSS with 105 MiB anonymous RSS and a 63 MiB resident heap. An active Sunshine stream at approximately 72 FPS and a 30-second 60 Hz mpv window produced no anonymous-memory or heap growth. One thousand abrupt selection-capable Wayland client disconnects also produced no growth. These results rule out the simplest continuous-frame and selection-device triggers.

A purpose-built Wayland client exposed a deterministic virtual-output pacing bug. On a virtual output configured for 144 Hz, Niri delivered 16,000 to 18,000 `wl_surface.frame` callbacks per second and used approximately 96 percent of one CPU core. The same process used about 6.5 percent during the idle control. `send_frame_callbacks_for_virtual_output()` sends callbacks unconditionally and bypasses the regular function's per-output frame-sequence check. Each callback-driven commit queues another immediate redraw.

Presentation feedback did not accumulate in the minimal client. Visible feedback was presented, hidden feedback was discarded, and repeated one-second runs stopped growing the warmed heap. The pacing bug is proven; its responsibility for the historical 28 GB heap is not. Commit `935eb7d4` makes the loop substantially more expensive because every commit with presentation feedback invokes the full-scene render-element and damage pass. A complex client or scene may therefore cause allocator churn and leave a large glibc high-water mark even though the minimal client remains bounded.

The local fix keeps a virtual output in `WaitingForEstimatedVBlankAndQueued` until its estimated VBlank timer fires. Physical outputs retain their existing immediate-redraw behavior in this state. A Wayland integration test verifies that a second frame callback remains pending during the same refresh cycle and arrives after the next refresh. All 219 library tests pass. The external protocol probe against a separate fixed headless instance received 115 callbacks in two seconds instead of approximately 35,000.

## Conclusion

Upstream provides adjacent reports, not a merged fix. The virtual-output callback storm now has a local fix and a protocol-level regression test. A clean-session soak test must still establish whether this also prevents the historical heap growth. Keep the monitor-off path, Smithay selection cleanup, and `935eb7d4` as separate comparisons rather than combining further changes before that result.
