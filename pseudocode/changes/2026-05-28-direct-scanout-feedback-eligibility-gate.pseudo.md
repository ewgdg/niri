---
affects:
  - src/niri.rs
---

# Direct Scanout Feedback Eligibility Gate

## Intent

Avoid broad scanout DMA-BUF feedback for composited surfaces while preserving Smithay's scanout feedback selection for the active real fullscreen window.

## Behavior

Applies to: `src/niri.rs`

```pseudo
when sending DMA-BUF feedback for an output:
    if debug disables direct scanout:
        send render feedback to every surface
        return

    candidate = active window on this output

    allow Smithay scanout-feedback selection for candidate surfaces only if:
        session is not locked
        output layout is currently showing the active fullscreen window above top-layer surfaces
        candidate exists
        candidate is real fullscreen
        candidate is not windowed/fake fullscreen

    for surfaces belonging to the eligible candidate window:
        use Smithay render-element state selection:
            ZeroCopy -> scanout feedback
            Rendering because FormatUnsupported -> scanout feedback
            Rendering because ScanoutFailed -> scanout feedback
            otherwise -> render feedback

    for all other windows, layer-shell surfaces, lock surfaces, drag-and-drop icons, and cursor surfaces:
        send render feedback
```
