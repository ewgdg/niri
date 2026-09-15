# Window Activation

This context defines how applications may request attention or compositor focus for their windows.

## Language

**Direct focus**:
Focus assigned through compositor interaction, such as clicking a window, keyboard navigation, or an explicit compositor action.
_Avoid_: Activation

**XDG activation**:
A Wayland client request for the compositor to focus a target window. It is separate from direct focus and may be accepted, downgraded to urgency, or ignored.
_Avoid_: Focus stealing

**Valid activation**:
XDG activation supported by a token tied to valid user input.

**Invalid activation**:
XDG activation whose token is not supported by valid user input.

**Urgency**:
A non-focusing indication that a window requests the user's attention.
_Avoid_: Activation

**XDG activation policy**:
A per-window `on-xdg-activate` rule that focuses, marks urgent, or ignores accepted requests. When unset, valid-serial requests focus and serialless requests mark urgent. Explicit `"focus"` also focuses serialless requests. Invalid-serial requests require the global debug override, and expired tokens are ignored. The rule applies before and after mapping; `open-focused` independently controls ordinary initial focus.
_Avoid_: Focusability
