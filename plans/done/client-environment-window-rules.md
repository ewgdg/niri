# Client environment window rules

## Goal

Let window rules match environment entries inherited by the process that opened a Wayland client connection. This lets the personal fork keep agent-launched windows from taking focus while preserving normal activation policy for other windows.

## Intention

Add a generic `client-env` matcher. Pi is only a config-level use case through `PI_CODING_AGENT=true`; niri must not know about specific agents.

## Scope and constraints

- Match regular expressions against individual `KEY=VALUE` entries.
- Capture the environment once when niri accepts the Wayland connection, before any toplevel exists or initial window rules run.
- Share the captured environment across every toplevel on that connection.
- Keep environment contents private. Do not expose them through IPC or debug output.
- Unknown credentials or unreadable process environments do not match.
- Native Wayland clients are supported. Existing singleton processes, D-Bus and portal activation, and Xwayland Satellite remain outside the reliable detection scope.
- Do not add parent-process, parent-window, or title correlation.

## Work plan

1. Add parsing coverage for `client-env` and behavioral tests for matching and non-matching clients.
2. Introduce a small client-environment module that reads `/proc/<peer-pid>/environ`, hides stored values behind a matching interface, and redacts debug output.
3. Capture this metadata in Wayland `ClientState` and consult it from the window-rule matcher for mapped, unmapped, and hidden windows.
4. Document the matcher, the agent-focus example, and its limits.
5. Run targeted tests and formatting, then the broader relevant test suite if targeted validation passes.

## Validation

- Config parsing accepts `match client-env="^PI_CODING_AGENT=true$"`.
- A matching test client obeys `open-focused false` during initial mapping.
- A non-matching expression leaves normal focus behavior unchanged.
- Existing XDG activation tests still pass.
- `cargo fmt --check` passes.

## Progress

- [x] Feasibility investigated against Wayland client credentials and window-rule timing.
- [x] Fork issue created as `ewgdg/niri#1`.
- [x] Tests added and observed failing because `client-env` is not yet a recognized matcher.
- [x] Implementation complete: connection environments are captured once and matched without exposing their contents.
- [x] Documentation complete, including the Pi example and known limits.
- [x] Validation complete: formatting, config/wiki parsing, clippy, targeted activation tests, peer-credential capture, unknown-credential behavior, and the full non-visual test suite pass.

## Decisions

Use connection environment rather than process ancestry. A Wayland connection has a clear compositor-side seam, while ancestor window titles are ambiguous when a terminal owns several toplevels.

Keep the matcher generic and regex-based to match existing `title` and `app-id` matcher behavior.

## Surprises and discoveries

The headless integration fixture uses real Unix socket peer credentials, so window behavior can be exercised end to end by matching the test process's inherited `PATH`. A separate helper-process test confirms that `SO_PEERCRED` selects the peer process rather than niri and that the captured environment remains usable after the peer exits.

Linux environment entries are byte strings while the existing window-rule regex type operates on UTF-8. The matcher deliberately ignores non-UTF-8 entries and documents that limit rather than adding a second regex implementation.

## Outcomes and retrospective

`client-env` now works during initial configure, after config reload, and for XDG activation policy. Missing or explicitly unknown client credentials fail closed as no match. Environment contents remain behind a matching interface with redacted debug output and are not added to IPC.

The implementation stays at the Wayland client seam. This avoids adding process metadata to mapped, unmapped, and hidden window representations separately, and naturally gives every toplevel on one connection the same captured environment.
