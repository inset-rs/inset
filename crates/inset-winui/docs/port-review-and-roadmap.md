# Port review and route to tabs/navigation

## NavigationView fidelity repair — 2026-09-07

The NavigationView review found structural mismatches, not merely small native input differences. The earlier indicator flash repair kept a detached rectangle above the whole control; it could outlive the pane or flyout that owned it. This batch restores item-owned indicator visuals, source compact-pane layout, hierarchy positions and pane chrome. The previous progress notes below are historical and do not establish parity for unreviewed combinations.

| Area | Source finding and repair |
| --- | --- |
| Selection indicators | `NavigationView.cpp::AnimateSelectionChanged` and `PlayIndicatorAnimations` animate each item's actual indicator. The port now applies transforms and opacity within that item, preserving pane, flyout and scroll ancestry while sharing an animation clock. |
| Compact layout | `NavigationView.xaml::ListSizeCompact` constrains `PaneContentGrid` to `CompactPaneLength` and collapses pane title/header. The port now sizes that grid instead of leaving an open-width pane behind a clip. Item text remains mounted: the source presenter intentionally preserves cutoff text, especially for iconless items. |
| Hierarchy | `NavigationViewItem.cpp` distinguishes top-level identity, indentation and Left/TopPrimary/TopFooter/TopOverflow position. The port retains Left presentation in compact child flyouts, preserves nested expansion state on pane close and treats top footer children separately. |
| Back, Close and header | `ShouldShowBackButton`, `ShouldShowCloseButton`, `UpdatePaneToggleSize`, `UpdatePaneTitleFrameworkElementParents` and the XAML template now determine the chrome. Minimal-open uses Close, title belongs inside the clickable toggle or its source holder, and Top does not force `AlwaysShowHeader=false` back to visible. |
| Focus disposal | `SplitView_Partial.cpp::OnUnloaded` does not restore focus. Removing that extra action avoids requesting focus on a detached scope immediately before its destruction. Ordinary pane-close focus restoration remains. |
| Retained rendering | Transition regressions exposed a native layer lifetime defect: a parent display layer could still reference a destroyed child render object. Inset's retained-handle ownership now keeps that layer representation alive until the parent releases it. |

Validation: all 116 WinUI workspace tests and all 1,390 Inset workspace tests pass (one existing ignored doctest). Both workspaces pass clippy with all targets; the kit reports only the existing valo-harness manifest warning. All eight resource-generator tests and kit formatting checks pass. Compact, flyout, minimal chrome and top title/header captures were inspected. No commits or staging changes were made by the agents.

The lower-cost audit also confirmed Slider omitted destruction of its owned focus-node handle after disposal; that omission is fixed in this batch.

TabView follow-up findings:

- `TabView.cpp::UpdateTabContent` assigns one selected content object to one presenter while TabViewItem retains ownership of the old content object. Inset keeps previously selected pages mounted under Offstage, so they continue participating in layout. Flutter’s `CupertinoTabScaffold::_TabSwitchingView` uses this same Stack, Offstage, TickerMode and FocusScope mechanism with lazy first creation. The user prefers that native mechanism, so TabView retains it and records the layout difference explicitly. KeepAlive belongs to lazy sliver viewports; introducing a viewport solely for tab retention would also change sizing behavior. Arbitrary timers can continue on a detached WinUI object, so timer stoppage is not a guaranteed source behavior.
- `TabView.cpp::OnItemsChanged` resolves replacement selection during the collection update. A GPU probe reproduced the old port’s blank frame after removing the selected last tab. TabView now retains selected item identity and reconciles it before building content; regressions cover last-item removal, disabled/hidden replacements, removal before the selected item, and single removal when no item was selected. Owner notification still runs after the frame under Inset’s ancestor-build rules, and that callback timing is explicit in PORTING.md.

## Minimal gallery content and live resize follow-up

The gallery placed its selected page title inside `NavigationView.Content` and omitted `NavigationView.Header`. `NavigationView.cpp::UpdateHeaderVisibility` explicitly collapses an empty Header, so the source minimal layout does not reserve header space for that content. The gallery now supplies its title through Header. A GPU integration test switches the actual demo to LeftMinimal and verifies that the title clears both Back and Toggle; no extra padding was added to NavigationView.

The resize crash was reproduced with rapid platform-metric updates in the whole gallery and an isolated NavigationView. Inset eagerly collected the overlay entry and its portal children before layout. An adaptive entry layout could remove a portal, leaving that snapshot with a child whose parent data had changed. Flutter’s `overlay.dart` intentionally yields children lazily to permit this mutation. Inset now resumes the source traversal after each yielded child instead of visiting a stale snapshot; the fix covers layout, paint and hit testing. Native and GPU regressions exercise traversal mutation and rapid real window-size changes.

The three source-folder PORTING files now name concrete components in their entries and use complete deferred/trigger sentences.

Follow-up validation: all 120 WinUI workspace tests and all 1,390 Inset workspace tests pass (one existing ignored doctest), including rapid actual-window resize, tooltip-overlay resizing and first-frame collection selection. Both workspaces pass clippy with all targets; only the existing valo-harness manifest warning remains on the kit side. All eight generator tests and affected formatting checks pass. The corrected minimal-mode gallery capture was inspected. No commits were made.


Current implementation: TabView and NavigationView are available in the gallery, including native in-window drag/reorder and cross-strip transfer, adaptive left panes, hierarchical items, top overflow, focus/collection handling, and source indicator animation tracks. ToolTip, internal flyout/scroll adapters, signed margins and Fluent icons support those controls. Functional native/host adaptations are documented in the source-folder PORTING files. The review and roadmap below retain the historical reasoning rather than serving as the current completion checklist.

Tab/navigation completion validation: all 98 inset-winui workspace tests pass, including GPU interaction and layout regressions; all 1,389 reveal-rs workspace tests pass (one existing ignored doctest). Both workspaces pass clippy with all targets, with only the existing valo-harness manifest warning on the WinUI side. Eight resource-generator tests pass and regenerated resources match byte-for-byte. Changed Rust files pass rustfmt and both diffs pass whitespace checks. Light/dark gallery and focused control captures were inspected. Native fixes cover retained animation parents, Flutter's equal-depth build ordering, and drag infrastructure; the kit Grid now actually measures layout-time children. No commits were made.

Follow-up validation: all 105 WinUI workspace tests and all 1,389 Inset workspace tests pass (one existing ignored doctest). Valo's display-list/renderer suites pass 59 tests, alongside the two new backdrop-seeding GPU tests and three existing backdrop goldens. All three workspaces pass clippy with all targets; WinUI retains only the pre-existing valo-harness manifest warning. Navigation, narrow-window operation and acrylic comparison captures were inspected.

Gallery and materials follow-up: the gallery now uses NavigationView with fourteen feature destinations, lazy retained pages, and a persistent pane-footer theme switch. The indicator keeps its outgoing visual during post-layout destination measurement, eliminating the target flash; pane-footer measurement no longer starts permanently constrained to zero. Non-overlay back buttons reserve the source row above the pane toggle and title.

Acrylic now uses the source blur and luminosity/tint composition through Valo, including a generic background color beneath the sampled scene before blur. The remaining source noise texture requires Inset host image-upload support; Valo already supports tiled image shaders. The porting notes describe this boundary and the explicit fallback policy.

Theme changes do not imply a global background crossfade: [NavigationView.xaml](/Users/mac/code/microsoft-ui-xaml/controls/dev/NavigationView/NavigationView.xaml:212) assigns background resources without a transition, while [Panel.cpp](/Users/mac/code/microsoft-ui-xaml/dxaml/xcp/core/core/elements/panel.cpp:78) checks for an explicit BackgroundTransition. SplitView retains its explicitly declared background transition; no gallery-wide fade was added.

Reviewed 2026-09-07 against the local microsoft-ui-xaml checkout at `aa3207e6`, the same revision recorded in the porting notes.

Implementation update: the first repair batch now gives ordinary buttons release-time keyboard activation and click-to-focus, and gives ToggleSwitch and RepeatButton their own source-specific keyboard/focus handling. Dedicated regressions cover these changes. The user accepted reveal's native tap recognition, including cancellation after movement beyond its threshold and focus timing on recognized tap-down, as a small documented divergence. Exact WinUI pointer capture is not a prerequisite for further ports, and no custom recognizer or routed-input layer is planned for this difference. The findings below retain the original review evidence.

Repair validation: the workspace suite passed with 29 tests, including 10 new input regressions. After adding logical GamepadA coverage, the affected RepeatButton and ToggleSwitch suites were rerun successfully. Workspace clippy including all targets passed with only the pre-existing sibling valo-harness manifest warning. Light and dark gallery captures were inspected. No commits were made.

SplitView milestone: implemented all four display modes and both placements, the nine template states and sixteen transitions, cancelable light dismissal, lifecycle events, native focus containment/restoration, Auto pane measurement and retained pane state. Per-side border thickness, per-corner radii, `SizeObserver`, and control-directory resource generation are available for later templates. The gallery includes an interactive SplitView section. The workspace now passes 44 Rust tests, including seven SplitView GPU tests and six visual-state tests; four resource-generator tests pass. Both themes and intermediate animations were captured and inspected. Native layout, overlay, event-timing and focus-policy adaptations are recorded in `src/controls/PORTING.md`.

SplitView exposed an existing reveal OverlayPortal resize-dispatch bug. The two Flutter `performResize` overrides were on `RenderObject` rather than the `RenderBox` trait that layout dispatches through. They now run at the correct override point, with an isolated regression for overlay child layout information and show/hide. The reveal workspace passes 1,387 tests and workspace clippy with all targets.

ToolTip also needs an explicit host boundary before implementation: ordinary WinUI 3 uses windowed popups, global cursor polling, display-power notifications and OS settings. Inset's Overlay can implement the source's constrained in-window branch, with the visible host substitute documented. Native focus policy can cover small trigger differences under the porting guidance; adding a framework focus-reason mechanism is not automatically a prerequisite.

The repaired keyboard/focus behavior and accepted native gesture behavior are a basis for continuing the prerequisite ports. Keep the resource generation, template structure and Grid work. This is a targeted source and behavior review, not certification of every template state or public WinUI property.

## Findings

### 1. P1 — shared buttons fire on key-down and keyboard repeat

[CommonStatesData::init_state](../src/primitives/common_states.rs) invokes `click` directly from `ActivateIntent`. Inset's `WidgetsApp` binds Space/Enter to that intent with a `SingleActivator` that accepts down and repeat events. It neither waits for release nor establishes the keyboard pressed state.

WinUI's [ButtonBaseKeyProcess.h](/Users/mac/code/microsoft-ui-xaml/dxaml/xcp/components/controls/KeyDownUp/inc/ButtonBaseKeyProcess.h) sets `IsPressed` on key-down, ignores repeated presses and invokes `OnClick` on key-up for the default `ClickMode.Release`. Its other-key and pointer interaction rules also belong to that state machine.

A temporary headless probe mounted the actual CommonStates widget with the application's default shortcuts and focused its node. Starting after one mouse click, Space down changed the count from 1 to 2, one repeat changed it to 3, and key-up left it at 3. WinUI's release-mode behavior would leave the count at 1 through down/repeat and advance to 2 on release.

This affects Button, ToggleButton, CheckBox, RadioButton and HyperlinkButton through their shared input wrapper; ToggleSwitch also uses it, despite having separate WinUI behavior. A held key can repeatedly execute an ordinary button action. RepeatButton's separate key handler does not fix these consumers.

Repair: transcribe ButtonBase's key lifecycle into shared control input, including pressed visuals, cancellation, disabled/focus changes, and the control-specific AcceptsReturn setting. Keep semantic activation as a separate way to invoke the action; it must not replace physical key processing.

### 2. P2 — leaving and re-entering a held button loses the click

[CommonStatesData::build](../src/primitives/common_states.rs) uses GestureDetector's tap recognizer. Moving beyond its slop rejects the gesture; re-entering cannot restore it.

[ButtonBase_Partial.cpp](/Users/mac/code/microsoft-ui-xaml/dxaml/xcp/dxaml/lib/ButtonBase_Partial.cpp), `OnPointerPressed`, `OnPointerMoved` and `OnPointerReleased`, captures the pointer and updates `IsPressed` from the current valid pointer position. Moving outside and back inside while held can still click on release. Movement inside a sufficiently wide control must not cancel merely because it is far from the down position.

The headless probe pressed the center of a 200 × 100 control, moved outside, returned to the center and released: zero clicks. A plain click on the same control worked.

Disposition: accepted divergence, recorded in `src/primitives/PORTING.md`. Keep native reveal tap recognition and its gesture arbitration. Later tab headers, close buttons and scrolling still need tests for their meaningful interactions; reproducing WinUI's exact capture mechanism is not a prerequisite.

### 3. P2 — clicking a shared button does not focus it

CommonStates wraps its gesture in FocusableActionDetector but never requests focus on pointer press. Inset's detector supplies focus, hover and actions; it does not automatically focus on click. The same headless probe confirmed that a successful mouse click left the control unfocused.

WinUI's `ButtonBase::OnPointerPressed` explicitly calls `Focus(FocusState_Pointer)` before capture. Here, clicking one control and then pressing Space can leave keyboard activation targeting the previously focused control. This becomes especially disruptive for tab selection and close behavior.

Disposition: click-to-focus is repaired and tested, including RepeatButton's separately implemented pointer path. CommonStates requests focus on recognized tap-down; the timing difference from raw pointer-down is accepted with native tap recognition.

### 4. P2 — ToggleSwitch accepts Enter

[ToggleSwitchState::build](../src/controls/toggle_switch.rs) constructs CommonStates with its default `accepts_return = true`. Under the application's default shortcuts, Enter therefore toggles the switch.

[ToggleSwitch::HandlesKey](/Users/mac/code/microsoft-ui-xaml/dxaml/xcp/dxaml/lib/ToggleSwitch_Partial.cpp) accepts Space and GamepadA, not Enter. Its [key processing](/Users/mac/code/microsoft-ui-xaml/dxaml/xcp/components/controls/KeyDownUp/inc/ToggleSwitchKeyProcess.h) also tracks the handled key-down and performs the Space toggle on release, provided no drag is active. The arrow-key branches in that helper are not evidence of supported arrow keys: HandlesKey gates them out in this checkout.

Repair: give ToggleSwitch its own source-matched key policy. Setting AcceptsReturn false alone does not fix its down/repeat/release behavior.

## What is working, and what the checks establish

- `cargo test --workspace`: all 19 existing tests passed (9 Grid unit tests, 10 gallery/integration tests).
- `cargo clippy --workspace`: passed; the only warning was missing workspace-lint inheritance in the sibling valo-harness manifest.
- Regenerating all currently included resources to a temporary file produced a byte-for-byte match with `theme/generated.rs`.
- The checked Grid group ordering and star-space distribution follow `grid.cpp`; this is substantial reusable work, not just a panel tailored to a single template.
- The checked three-state cycle and ToggleSwitch midpoint decision match the C++.
- The light and dark gallery captures rendered without obvious clipping or layout breakage. This is visual inspection of reveal output, not a pixel comparison against Windows.

The existing tests are useful smoke coverage, but do not establish keyboard or mouse-focus parity. `Fixture::capture` checks image length and opaque alpha, then optionally exports a PNG; it does not compare pixels against a reference. Most interaction tests use touch taps. The temporary input probe was removed from the workspace after recording the behavior; no implementation fixes were made in this review.

The documented gaps also remain real work: RadioButton group traversal, Slider value ToolTip, right-to-left templates, and deferred transitions. Passing tests should not turn those into implicit claims of complete controls. Accessibility remains outside this review's scope.

## Next batch: finish the foundation

1. Retain the completed keyboard/focus repairs and regression tests. Use native reveal gestures and the accepted tap-cancellation divergence. As compound controls arrive, verify nested activation and scroll cancellation without requiring WinUI's exact input mechanism.
2. Implement ToolTip and its placement/service behavior using reveal's existing Overlay machinery after checking the source-required lifecycle. Finish Slider's deferred value tooltip with it. Verify hover delays, focus-triggered visibility, edge placement and disposal while open.
3. Per-side thickness/padding and per-corner radius are available after SplitView. Complete the remaining alignment and right-to-left behavior needed by later templates. TabView's filtered top corner radii can now use `corner_radius_corners`.
4. Establish WinUI icon rendering and themed button customization. TabView's add/close/scroll buttons use their own styles and non-tab-stop settings; the current fixed button styles do not express them. Reuse the source-defined input behavior under those templates instead of duplicating it.
5. Resource generation now searches CommonStyles and the control's own directory, resolves shared aliases through the common and base dictionaries, preserves submillisecond durations, and reports unresolved names. Add the actual template dependencies as each destination is ported; ToolTip's Acrylic brush and a legacy RadioButton system-color alias are currently reported but not emitted.

Exit condition: source-backed behavioral tests pass, gallery captures cover pointer and keyboard states in both themes, and functional differences are recorded. Small differences can use native reveal mechanisms under the porting guidance; missing control features or differences that affect dependent controls still need a concrete design assessment.

## Route to tabs and navigation

These are two substantial consumers of common infrastructure. Neither is a row or column of the existing buttons.

| Stage | Deliverable | Source and dependency reason | Acceptance examples |
| --- | --- | --- | --- |
| Shared scrolling | WinUI ScrollViewer/ScrollBar behavior and templates over the appropriate reveal scrolling primitives | `CommonStyles/ScrollViewer_themeresources.xaml`, ScrollBar resources, and their C++; reveal already has scrolling machinery, but Flutter scrolling is not automatically WinUI ScrollViewer parity | Wheel/trackpad, overflow, repeat-button scrolling, focus bring-into-view, resize and boundaries |
| Shared item infrastructure | Source-shaped item containers, collection changes, selection and focus traversal | TabView uses a ListView-derived TabViewListView; NavigationView uses ItemsRepeater and SelectionModel. Inspect both consumers before defining shared APIs; they are not interchangeable | Insert/remove selected item, stable identities, disabled candidates, offscreen focus and collection updates |
| Pane primitive — implemented | SplitView | `controls/dev/SplitView/SplitView_themeresources.xaml` and `dxaml/xcp/dxaml/lib/SplitView_Partial.cpp`; NavigationView's RootSplitView owns pane presentation | Verified Inline, Overlay, CompactInline, CompactOverlay; open/close lifecycle, resize, dismissal and focus |
| Tabs | TabViewItem, TabViewListView, then TabView | `controls/dev/TabView/TabView.xaml`, `TabViewItem.cpp`, `TabViewListView.cpp`, `TabView.cpp`, `TabView.idl` | Equal/SizeToContent/Compact widths, add/close request events, selected-content lifecycle, overflow scrolling, Ctrl+Tab/Ctrl+Shift+Tab/Ctrl+F4, close focus recovery, reorder/drop |
| Navigation | NavigationViewItemBase and item family/presenter, flyout support, then NavigationView and TopNavigationViewDataProvider | `controls/dev/NavigationView/NavigationView.xaml`, item templates/C++, `NavigationView.cpp` and `TopNavigationViewDataProvider.cpp`; needs repeaters, hierarchical selection, SplitView and overflow flyouts | Expanded/compact/minimal thresholds, explicit pane modes, hierarchical expansion, invocation versus selection, settings/footer/back, top overflow and keyboard traversal |

SplitView is now available as a standalone milestone. The provisional recommendation remains TabView as the first full destination after the shared foundations, then NavigationView; tabs exercise selection, focus, collection changes and scrolling before adding navigation's adaptive pane and hierarchy rules. This ordering is not a claim that ListView or TabView is a small port.

Do not silently defer tab reordering: this checkout's TabView.idl defaults CanReorderTabs and AllowDropTabs to true. Tear-out is a separate host-sensitive feature and defaults false. Inspect the host requirements and decide its boundary before promising support; neither a fake implementation nor a partially named TabView satisfies the repository's rules.

ProgressBar, ProgressRing, InfoBar and unrelated controls need not precede this route. Add a control because a destination needs it, or because it is independently wanted—not to accumulate a larger gallery before solving selection, focus and scrolling.

The shared input repair batch is complete within the accepted native gesture behavior. Continue with the prerequisite primitives above, resolving ToolTip's host boundary separately; the button capture question no longer blocks progress toward TabView or NavigationView.
