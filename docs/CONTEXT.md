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
A per-window policy deciding whether XDG activation follows the compositor default, accepts only valid activation, downgrades invalid or all activation to urgency, or ignores all activation. Tokens without a serial remain urgency-only except under the policy that ignores all activation.
_Avoid_: Focusability
