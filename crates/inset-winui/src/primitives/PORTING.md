# inset-winui/src/primitives
Audience: readers familiar with Flutter and Rust; the headings identify the corresponding WinUI code.

WinUI home: `ContentPresenter`, `Border`, `BrushTransition`, `VisualStateManager` and the system focus visual, in dxaml/xcp
Ported against: aa3207e6

## common_states.rs → `ButtonBase`

- Change: `CommonStates` consumes activation keys even when they do not produce a click.
  Reason: framework — Inset routes keyboard events through nested shortcut handlers, so a rejected key can otherwise activate an enclosing control.
  Affect: Holding an activation key does not repeatedly trigger an enclosing shortcut, and a button configured to reject Enter still consumes that key.

- Change: `CommonStates` handles button presses with Flutter tap recognition instead of keeping all pointer events until release.
  Reason: framework — Inset lets tap, drag and scroll recognizers compete for the same press, whereas WinUI can capture the pointer directly for the control.
  Affect: Moving too far cancels the click, even if the pointer returns inside the button before release.

- Change: `CommonStates` leaves keyboard focus on the previously focused widget when a button is clicked.
  Reason: framework — the kit follows Flutter buttons, which leave focus unchanged on pointer activation, where WinUI's `ButtonBase` takes the focus on press.
  Affect: Tab moves focus and shows a focus ring; a mouse click does not move that ring to the clicked button.

## color_transition.rs → `BrushTransition`

- Change: `ColorTransition` fades background colors at a constant rate.
  Reason: os — Windows chooses how the original fade speeds up and slows down, and the template states only its length, `ControlFasterAnimationDuration`.
  Affect: A control’s background reaches its new color after the same 83 ms as on Windows, but passes through different colors on the way.

## focus_visual.rs → system focus visual

- Change: `FocusVisual` uses fixed black and white colours.
  Reason: os — XAML obtains these colours from the system palette.
  Affect: `FocusVisual` rings ignore high-contrast palette changes.

## icon.rs → `IconElement`, `FontIcon` and `PathIcon`

- Change: `FontIcon` uses the bundled Fluent System Icons font by default.
  Reason: os — WinUI's default icon fonts ship with Windows and are not bundled with this kit.
  Affect: Icons can differ in shape, spacing and alignment from Windows, and applications can select another font with `font_family`.

- Change: `FluentSymbol` provides names for the bundled icons rather than accepting WinUI's `Symbol` enumeration.
  Reason: os — WinUI's enumeration identifies characters in Windows icon fonts, whose character mappings differ from the bundled font.
  Affect: Applications choose a `FluentSymbol` or supply a character to `FontIcon`; they cannot pass WinUI `Symbol` values directly.

- Change: `PathIcon` accepts a path built in Rust rather than a XAML path string.
  Reason: language — the XAML parser converts text into geometry, while this API receives geometry constructed with `PathBuilder`.
  Affect: Applications pass the path to `PathIcon::new` and use `fill_rule` to choose how overlapping contours are filled.

- Change: `FontIcon` and `PathIcon` can be passed directly to controls that accept icons.
  Reason: framework — WinUI uses separate `IconSource` objects to create an icon element for each parent, while Inset widgets are reusable descriptions of their rendered content.
  Affect: Applications reuse the icon widget instead of constructing a separate `FontIconSource` or `PathIconSource` object.

## scroll_viewport.rs → `ScrollViewer`

- Change: `ScrollViewport` uses Flutter's scrolling gestures and motion.
  Reason: framework — WinUI delegates touch scrolling and inertia to the Windows DirectManipulation service, while Inset provides Flutter's scrolling implementation.
  Affect: Scrolling momentum and behavior at the ends follow Flutter, and programmatic scroll commands animate over 100 ms.

## anchored_flyout.rs → attached `Flyout` and `FlyoutPresenter`

- Change: `AnchoredFlyout` opens navigation menus in the application’s Overlay and uses a FocusScope.
  Reason: framework — WinUI can create a separate popup window, while a Flutter Overlay draws inside the existing window.
  Affect: A menu cannot extend outside the window, and closing it returns focus to its opening button if that button still exists.

- Change: `AnchoredFlyout` checks its preferred side of the opening button and the opposite side, rather than all four sides.
  Reason: framework — the native custom-layout callback permits one child measurement per pass, while WinUI’s popup-placement code can measure again for several candidate positions.
  Affect: Near a window edge, a menu can appear on a different side or at a different size than on Windows.

- Change: `AnchoredFlyout` opens and closes immediately instead of animating its appearance.
  Reason: os — the source asks Windows to choose the popup animation through `PopupThemeTransition`, without specifying its duration and motion in the template.
  Affect: A navigation menu appears at full size in a single frame, without the slide and fade Windows gives a popup.

## margin.rs → `FrameworkElement` layout

- Change: `Margin` preserves fractional spacing instead of snapping its edges to physical pixels.
  Reason: framework — XAML rounds layout measurements using the display scale, while Inset passes logical-pixel measurements through without that rounding step.
  Affect: At fractional display scales, the spacing can end between physical pixels instead of on a pixel boundary.

## acrylic.rs → `AcrylicBrush`

- Change: `Brush::Acrylic` omits the faint repeating noise image drawn over the blurred background.
  Reason: framework — Inset’s host interface cannot yet upload the noise image to its renderer, Valo, although that renderer can repeat an uploaded image.
  Affect: `Brush::Acrylic` surfaces lack the source’s fine grain.

- Change: `Brush::Acrylic` does not automatically replace its blurred background with a solid color when the operating system disables visual effects.
  Reason: os — this host does not report Windows battery and advanced-effects policy changes.
  Affect: Applications must select the solid replacement color themselves.

- Change: `Brush::Acrylic` changes its tint immediately when the application supplies a new color.
  Reason: framework — the port stores material settings as values and has no persistent brush object to animate between replacements.
  Affect: `Brush::Acrylic` tint changes jump to the new color instead of fading over 500 ms.

## brush.rs → `Brush`

- Change: `Brush` paints the shape itself instead of returning a Paint for the caller to draw with.
  Reason: framework — acrylic must sample the scene already drawn behind the shape, which Inset’s renderer, Valo, expresses through a compositing layer rather than an ordinary fill or stroke.
  Affect: Callers ask the helper to paint a rounded rectangle, a border or an oval instead of passing a Paint to Canvas.

## info_bar_panel.rs → `InfoBarPanel`

- Change: `InfoBarPanel` receives the intended height of a single row as an explicit input.
  Reason: framework — WinUI reads this setting from the parent, but Flutter layout constraints describe the space available to a child, not the settings used to size its parent.
  Affect: When using the panel directly, pass the container’s minimum height minus the panel’s top and bottom margins as `parent_min_height`; the full notification control does this automatically.

## retained_flyout.rs → `FlyoutBase` content lifetime

- Change: `RetainedFlyoutHost` keeps closed flyout content mounted in a root OverlayEntry, independently of its opening button.
  Reason: framework — WinUI keeps its content object alive after removing its visuals, while Flutter requires the child to stay mounted to preserve its State.
  Affect: Child State survives closing and reopening from another opening button, but hidden children can still be laid out; Offstage hides them and TickerMode mutes their animation ticks.

- Change: `FlyoutTarget` updates the flyout’s theme or requests closing after the opening widget is removed, waiting until the frame ends.
  Reason: framework — a Flutter widget reports inherited changes and disposal through its lifecycle, and updating the separate overlay must wait until the current build finishes.
  Affect: Applications wrap opening widgets in FlyoutTarget; removing that widget can take another frame to close the flyout and does not dispose the retained child State.

## input_device.rs → `CInputManager` last input device and `CFocusManager`'s focus state

- Change: `FocusState::coerce_programmatic` counts every key event as hardware keyboard input and has no gamepad category.
  Reason: os — the desktop host delivers keyboard and pointer events only, while Windows also records `GamepadOrRemote` and asks `LastInputWasNonFocusNavigationKeyFromSIP` whether a key came from the touch keyboard.
  Affect: A gamepad reaches the kit only through keys the host maps for it, which count as keyboard input, and a key typed on an on-screen keyboard counts as keyboard input and draws a ring.

## breadcrumb_layout.rs → `BreadcrumbLayout`

- Change: `BreadcrumbLayout` retains hidden item State and measures hidden content with the same available-width constraints as visible items, following Inset’s offstage behavior.
  Reason: framework — WinUI measures every item and then arranges hidden items into zero-sized rectangles, while Inset uses one constraint-driven layout operation.
  Affect: Hidden breadcrumbs do not paint or receive pointer input, but their content is still laid out at its measured size and retains its State when the bar narrows or widens.

## color_display_name.rs → `ColorHelper::ToDisplayName`

- Change: `color_display_name` names a colour by the regions `ColorIntegrationTests::CanRetrieveColorNames` lays out in HSL: black and white by luminosity, greys by low saturation, and every other colour by a saturation band, a hue range in it and a dark, mid or light luminosity, with the 42 names of `Microsoft.UI.Xaml.Common.rc`.
  Reason: os — WinUI classifies the colour with `ColorHelper_ToDisplayNameId`, which is part of Windows and not in the repository, and the test states the regions only at points inside them.
  Affect: A colour well inside a region gets Windows' name, and ColorPicker's keys reach the colours `ColorPickerTests` expects, but a colour near a region's edge may get the neighbouring name, which also moves where Ctrl and an arrow key stop.

## Deferred

- `BitmapIcon` and `ImageIcon` remain deferred. Trigger: Image loading is integrated with the kit's supported hosts, including foreground-color tinting for monochrome `BitmapIcon` images.
- `AnimatedIcon` remains deferred. Trigger: The kit can reproduce the icon animations that WinUI plays through Windows Composition.
- Navigation flyout opening and closing animations remain deferred. Trigger: A Windows timing measurement or equivalent host transition is available.
- The blurred background’s repeating noise image remains deferred. Trigger: Inset can upload that image to the renderer.
- Automatic acrylic fallback policy remains deferred. Trigger: A host reports battery and advanced-effects policy changes.
- Animated tint changes remain deferred. Trigger: The background implementation stores the old and new colors and an animation between them.
- The exact edges between `color_display_name`'s regions remain deferred. Trigger: A name table measured on Windows.
