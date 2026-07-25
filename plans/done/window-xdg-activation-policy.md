# Per-window XDG activation policy

## Goal

Add a dynamic, target-based `xdg-activation` window-rule property that controls whether XDG activation focuses a window, becomes urgency, or is ignored, while leaving direct compositor focus unaffected.

## Intention

Allow automation windows to remain visible and manually focusable without letting client activation interrupt the active workspace. Preserve Niri's global activation behavior unless a matching window rule explicitly restricts or downgrades it.

## Scope and constraints

- Public values: `default`, `valid-only`, `valid-or-urgent`, `urgent`, and `never`.
- The policy matches the activation target window.
- `open-focused` remains independent and controls ordinary focus on mapping.
- The property recomputes dynamically for mapped and unmapped windows.
- Valid, invalid-serial, and serial-less tokens are classified before target-policy resolution.
- Expired tokens remain ignored.
- Direct focus through pointer input, keyboard navigation, and compositor actions is unchanged.
- Keep the work in the `ewgdg/niri` fork; do not create an upstream request.
- Add observable-behavior tests before each implementation slice.

## Proposed test seams

These require user confirmation before tests are written.

1. Config seam: parse KDL through Niri's public configuration decoder and observe each `xdg-activation` value and invalid-value diagnostics.
2. Rule-resolution seam: resolve matching window rules and observe last-match override, including restoring `default` on config reload/recomputation.
3. Compositor protocol seam: issue XDG activation against mapped and pre-map target windows and observe focus and urgency through Niri's existing integration test harness.
4. Direct-focus seam: focus a protected window through a compositor action and observe that it remains manually focusable.

## Work plan

1. Confirm the observable test seams.
2. Add config parsing coverage, then minimally add the public enum and window-rule field.
3. Add resolved-rule behavior coverage, then minimally merge the dynamic property.
4. Add mapped-target protocol behavior one policy slice at a time.
5. Add pre-map behavior, dynamic reload, direct-focus, and timeout coverage one slice at a time.
6. Document the implemented property in the window-rule reference.
7. Run focused tests, formatting, Clippy or repository lint checks, and the relevant full test suites.
8. Review the diff for unintended behavior and commit semantically separated implementation/docs changes.

## Validation

- KDL accepts all five values and rejects unknown values.
- The five-mode behavior matrix holds for valid, invalid-serial, and serial-less tokens.
- `default` preserves behavior with the global invalid-serial debug option both enabled and disabled.
- `valid-or-urgent` and `urgent` can downgrade invalid activation even when the global option is disabled.
- Expired tokens have no focus or urgency effect.
- Matching is based on the target window.
- Existing mapped windows use reloaded policies immediately.
- Stored pre-map activation cannot bypass the policy.
- `open-focused false` plus `xdg-activation "urgent"` maps automation windows without focus and retains urgency.
- Explicit compositor focus still works.

## Progress

- Confirmed the public policy and edge-case matrix.
- Added and committed the domain glossary and ADR.
- Created the feature branch and this active ExecPlan.
- Added public KDL parsing and rejection coverage for all policy values.
- Added XDG activation support to the integration test client, including valid, invalid-serial, and serial-less tokens.
- Implemented mapped and pre-map policy resolution, target-rule precedence, live reload, and direct-focus preservation.
- Added the window-rule reference documentation.
- Focused activation tests, the complete non-visual workspace test suite, and Clippy pass.

## Decisions

- Use a policy enum rather than overlapping booleans.
- Keep initial mapping focus and XDG activation as independent concepts.
- Convert invalid activation to urgency only in modes that request it; `never` is silent.
- Resolve the policy against the target using the latest window rules.

## Outcomes and retrospective

Implemented the five-mode policy across public config parsing, dynamic rule resolution, mapped activation, and activation received before mapping. The integration test client now exercises XDG activation with real protocol objects and keyboard-enter serials.

Invalid tokens are retained only when the global debug option or at least one urgency-capable window rule may need them. This preserves the normal early-rejection path for configurations that do not use the new behavior while allowing target-based urgency downgrades.

Validation:

- `cargo test --all --exclude niri-visual-tests` passed.
- `cargo clippy --all --all-targets` passed.
- `cargo test -p niri-config` passed, including wiki parsing.
- `git diff --check` passed.
- `cargo fmt --all -- --check` reports only the pre-existing formatting difference in `src/protocols/foreign_toplevel.rs`; this branch leaves that file unchanged.
