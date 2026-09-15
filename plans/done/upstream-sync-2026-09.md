# Sync the explicit-sync PR and personal fork with upstream

## Goal and scope

Rebase `feat/linux-drm-syncobj-v1` onto upstream `02fdd8e7` and update its existing PR, then merge the same upstream revision into `main`. Preserve the fork's virtual outputs, hidden windows, environment matching, and syncobj support. Do not install or restart the compositor.

## Decisions

- The PR keeps its single feature commit; only adjacent import/module conflicts need resolution.
- Adopt upstream `on-xdg-activate` and remove the older focus/urgency settings, as requested. Preserve the fork's handling of activation received before mapping, using the upstream enum semantics. The old valid-focus/serialless-ignore combination is intentionally removed.
- Adapt headless GBM allocation to upstream's `DeviceFd` type while retaining headless screencast support.

## Work plan and validation

1. Rebase PR, check the patch range, format with nightly Rust, compile all targets, and run the transaction regression test; push with an explicit force-with-lease.
2. Resolve main's conflicts, updating activation tests/docs and headless integration.
3. Compile default and feature configurations, run focused activation/virtual-output/hidden/screencopy tests and configuration tests; run the normal workspace suite once the merge is ready.
4. Review fork changes against both parents, finish the merge, push main, and verify remote heads and PR mergeability.

## Progress

- PR rebased to `0ce289ab`; all-target check, nightly format, and transaction test passed. Branch pushed with lease against `9548e63a`.
- Main conflicts resolved. Activation changes and backend integration received independent reviews.
- Preserved virtual/headless capture support, hidden/environment window rules, MRU tablet behavior, and both sets of test-client additions.
- Normalized the existing client-environment import group to satisfy the project's nightly formatter.

## Validation results

- `cargo check --all-targets`: passed.
- `cargo test -p niri xdg_activation`: 11 passed, including mapped/pre-map policy and serial matrices.
- `cargo test --all --exclude niri-visual-tests`: 255 passed, one ignored, no failures (including doc tests).
- `cargo check --no-default-features`: passed.
- `cargo check --no-default-features --features xdp-gnome-screencast`: passed.
- `cargo clippy --all --all-targets`: passed without warnings.
- `cargo +nightly fmt --all -- --check` and `git diff --check`: passed.
- GitHub confirmed the rebased PR head and reported it mergeable.

## Outcome

The fork now integrates upstream through `02fdd8e7`. The PR retains one feature commit with unchanged syncobj behavior. Main adopts upstream activation configuration and carries forward the fork's pre-map behavior. Migration guidance is in the window-rule documentation; no legacy runtime aliases remain.

## Risks and limits

Tests do not replace a live DRM/NVIDIA, suspend/resume, or hotplug check. User configuration still using removed settings must be migrated before installing the new binary; installation is outside this task.
