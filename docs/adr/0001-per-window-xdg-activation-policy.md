# Add a per-window XDG activation policy

Niri will provide a dynamic, target-based `xdg-activation` window-rule property with `"default"`, `"valid-only"`, `"valid-or-urgent"`, `"urgent"`, and `"never"` policies. The policy controls only client-requested XDG activation; direct focus through pointer input, keyboard navigation, or compositor actions remains available. `open-focused` remains independent and controls ordinary focus when a window maps.

The policy applies immediately to mapped and unmapped matching windows. It classifies valid, invalid-serial, and serial-less tokens before resolving the target policy, allowing explicit policies to downgrade otherwise-invalid activation to urgency without permitting focus. `"default"` preserves the global activation policy, expired tokens remain ignored, and later matching window rules may restore `"default"`.
