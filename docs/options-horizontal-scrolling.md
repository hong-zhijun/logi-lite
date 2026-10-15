# Options+ horizontal scrolling and Logi Lite behavior

Checked 2026-10-15 against installed Options+ resources. The bounded excerpts and
source digests are retained in [options-horizontal-toggle.json](evidence/options-horizontal-toggle.json).

## Observed from shipped resources

- Options+ stores a `virtualThumbwheel.keys` array. Each entry has its own
  `controlId`, `enabled`, and `stateChange`. Its `updateAssignmentCard` handler
  changes only the key whose `controlId` matches the currently selected button.
  This establishes a per-side-button switch rather than one action assignment or
  one global switch. The UI uses a checked toggle named `horizontal-scroll`.
- The same component renders `CONFIGURATION_COPY_HORIZONTAL_SCROLLING`.
  The installed English and Chinese YAML resources, line 1947, say respectively
  “Hold the button and use the scroll wheel to scroll horizontally.” and
  “按住按钮并使用滚轮水平滚动。”
- Lines 1949–1957 describe conflict handling. Options+ disables horizontal scrolling
  for incompatible assignments and warns that enabling it can reset the button to
  its default action. This evidence does **not** establish that every custom action
  can coexist with horizontal scrolling in Options+. The bounded conflict
  predicate is `card.continuous` for an eligible side slot, or a keystroke whose
  `code == 0` and `modifiers` exist (including the cached shortcut). The latter
  represents modifier-only/held shortcuts. Logi Lite exposes one-shot actions and
  chords with a real nonzero key, so these two incompatible action forms are not
  currently selectable.

The JavaScript excerpt comes from the existing inert string-table decoding of the
installed bundle; no JavaScript was executed. Alias discovery is syntactic and
its bounds do not prove runtime reachability. The YAML wording establishes the
intended interaction, rather than an observed input trace. A native REA query did
not complete, so there is no native function Evidence ID or confirmed Options+
click-suppression timing for this pass.

## Logi Lite implementation choices

`horizontalScroll: {back: false, forward: false}` is stored separately from the
three button actions. Both switches default off. There is no middle-button option.
Normal clicks retain the configured action; holding the enabled side button and
moving the wheel changes the wheel axes and consumes that click. To avoid
triggering a shortcut or app before knowing whether scrolling will happen, its
normal click waits until release. Default mouse clicks are replayed as a balanced
marked down/up pair, while custom actions run once on release. This exact
mechanism and deferred-click timing are Logi Lite design choices, **not** claims
about Options+ internals. Releasing the button restores vertical scrolling; pausing or
reconfiguring cancels pending actions while swallowing the matching release.

Old persisted `水平滚动` action values migrate to the appropriate side switch plus
`默认`. A legacy middle-button value becomes `默认` and never enables horizontal
scrolling. Migration preserves unrelated mappings and does not rewrite the source
file until the next explicit save. Windows configuration shape remains unchanged.

REA close reported `cleanup_incomplete`: the Hopper document shutdown
acknowledgement was not observed and its private `/tmp/rea-Vr0Um0` runtime was
retained. No user application was terminated and no additional native session was
opened after this boundary.

Offline tests cover independent side states, middle exclusion, click versus
scroll decisions, cancellation, balanced button suppression, preserved wheel
phases/deltas, and legacy configuration migration. Physical mouse/application
behavior remains a manual test.
