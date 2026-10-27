# inset-winui/src/controls
Audience: readers familiar with Flutter and Rust; the headings identify the corresponding WinUI code.

WinUI home: controls/dev/CommonStyles (templates), dxaml/xcp/dxaml/lib (`<Control>_Partial.cpp`), dxaml/xcp/core/core/elements
Ported against: aa3207e6

## mod.rs → shared control conventions

- Change: `Button`, `CheckBox`, `RadioButton`, `ToggleButton`, `HyperlinkButton`, `RepeatButton`, `ToggleSwitch`, `Slider`, and navigation and tab item click handlers do not request keyboard focus on pointer activation.
  Reason: framework — Flutter controls separate pointer activation from focus acquisition, and Inset's native focus policy is used for this port.
  Affect: These controls acquire focus and its highlight through keyboard traversal; a pointer click leaves any existing keyboard focus highlight in place.

- Change: `ToggleSwitch`, `Slider`, `Expander` and other controls with application-owned values report requested changes through callbacks.
  Reason: framework — WinUI changes values on persistent control objects, while Inset follows Flutter by rebuilding widgets from application state.
  Affect: A value-change callback that the caller ignores leaves the corresponding control unchanged.

- Change: A control takes the size its parent forces and, where the parent leaves room, its own size, instead of applying its style's `HorizontalAlignment` and `VerticalAlignment`; `TextBox`, `NumberBox`, `Slider`, `ColorPickerSlider`, `ProgressBar` and `TabView` take the whole of a bounded width (a vertical slider, of a bounded height), and `NavigationView` and `SplitView` the whole bounded space, as Flutter's text fields, sliders and scaffolds do.
  Reason: framework — a XAML element aligns itself within the slot its panel gives it, while a Flutter parent decides a child's size through tight or loose constraints.
  Affect: Under `Center` or in a start-aligned `Column`, a `CheckBox`, `Expander` or `InfoBar` is as wide as its content, and a stretched `Column` or a `Grid` cell makes it fill the width as WinUI's default `Stretch` does.

## button.rs → `ControlTemplate`

- Change: `Button` accepts a drawing function to customize its appearance while retaining its input behavior.
  Reason: language — Rust callbacks receive the current button state and content in place of a XAML template resource.
  Affect: A `Button` caller can supply `.template(...)` without replacing the button’s input behavior.

## text_control.rs → TextBoxBase

- Change: `TextBox` and `PasswordBox` use Flutter’s text editor, selection gestures, controllers and input formatters.
  Reason: framework — WinUI delegates editing to Windows RichEdit, while Inset supplies Flutter's editor.
  Affect: Text selection, keyboard shortcuts, IME composition, length enforcement and undo follow the native Flutter behavior.

- Change: `TextBox` and `PasswordBox` report text and selection changes after the edit has been applied.
  Reason: framework — WinUI can let application code cancel a pending edit or selection change, while Flutter validates edits with input formatters before updating the controller.
  Affect: Applications use input formatters to reject user edits and controller listeners to observe changes, rather than canceling a before-change event.

- Change: `TextControl` adjusts its clear and password-reveal buttons after the editor is measured.
  Reason: framework — the button widgets are built before the editor’s size is known, so they need another build to apply size-dependent changes.
  Affect: A clear or reveal button can settle its visibility or width on the frame following a size change.

## text_box.rs → TextBox

- Change: `TextBox` wraps long lines in multiline mode even when wrapping is disabled.
  Reason: framework — Flutter's multiline layout does not provide independent wrapping and two-axis editor scrolling.
  Affect: A field configured to accept newlines preserves typed line breaks but also wraps lines that exceed its width.

- Change: `TextBox` receives a TextEditingController owned by the application instead of storing editable text on the widget itself.
  Reason: framework — Inset keeps mutable editing state in a persistent controller while rebuilding the surrounding widget descriptions.
  Affect: Callers read or assign text and selection through the controller and dispose it after the editor unmounts.

## password_box.rs → PasswordBox

- Change: `PasswordBox` keeps its password in a native TextEditingController.
  Reason: os — WinUI's additional CryptProtectMemory storage is a Windows facility outside the native Flutter editing engine.
  Affect: The field conceals text visually and disables copying and cutting, but the controller contains an ordinary text value.

## text_edit_menu.rs → TextCommandBarFlyout desktop commands

- Change: `TextBox` and `PasswordBox` show editing commands in Flutter’s text-selection menu overlay.
  Reason: framework — Flutter’s editor opens, places and closes the menu, whereas WinUI uses a separate popup window for the editing commands.
  Affect: Editing menus stay inside the application window and follow Flutter’s placement and keyboard shortcuts, while preserving WinUI’s command order and rules for enabling commands.

- Change: `TextEditMenu` blurs content inside the application window and appears without an opening animation.
  Reason: os — the Windows system backdrop and popup theme animation are not available through the native overlay host.
  Affect: The menu samples the application scene instead of the desktop behind the window and appears immediately.

## grid/render_grid.rs → `Grid`

- Change: `Grid` gives each child its whole grid cell, with alignment expressed by child wrappers.
  Reason: framework — WinUI combines alignment and sizing on each element, while Inset uses separate alignment wrappers and requires each render box to obey its parent’s size constraints.
  Affect: `Grid` constrains a child larger than its cell to that cell rather than letting it overflow.

## grid/layout.rs → `Grid` row and column layout

- Change: `Grid` row and column sizes are not rounded to physical pixels.
  Reason: framework — XAML rounds track sizes using the display scale, whereas Inset’s layout keeps fractional logical-pixel measurements.
  Affect: `Grid` proportionally sized rows and columns can have edges between physical pixels.

## toggle_switch.rs → `ToggleSwitch`

- Change: `ToggleSwitch` moves its knob over 167 ms with fast-out-slow-in easing.
  Reason: os — the template delegates knob movement to Windows through `RepositionThemeAnimation`, leaving the duration and acceleration outside the checkout.
  Affect: `ToggleSwitch` slide duration and curve can differ from Windows.

- Change: `ToggleSwitch` waits for enough pointer movement to distinguish a drag from a tap before moving its knob.
  Reason: framework — Flutter recognizers first decide whether the press is a tap or a drag, while WinUI’s draggable knob starts tracking on press.
  Affect: `ToggleSwitch` keeps its knob still for the first 18 px of touch movement, and shorter movement remains a tap.

## radio_button.rs → `RadioButton`

- Change: `RadioButton` asks the caller to update the selected option instead of finding and unchecking other buttons in a named group.
  Reason: framework — Inset’s controlled widgets receive their checked values from owner state, while WinUI can mutate other persistent RadioButton objects in the group.
  Affect: A `RadioButton` caller checks the chosen button and unchecks the others.

- Change: `RadioButton` keeps its selected dot’s outline when the pointer moves over it.
  Reason: framework — the port chooses one appearance for the current state rather than letting several WinUI animations overwrite the same color in sequence.
  Affect: Hovering a selected radio button does not briefly change its one-pixel dot outline.

## repeat_button.rs → `RepeatButton`

- Change: `RepeatButton` uses the App clock with a 500 ms delay and 33 ms interval.
  Reason: os — the port schedules exact deadlines on Inset’s application clock, whereas Windows rounds timer delivery to its system timer resolution.
  Affect: A held `RepeatButton` can repeat somewhat faster than on Windows.

## hyperlink_button.rs → `HyperlinkButton`

- Change: `HyperlinkButton` does not open a URL automatically.
  Reason: os — Windows opens the URL for the original control, but this host does not provide an equivalent URL-opening service.
  Affect: A `HyperlinkButton` caller opens the link from the click handler.

## slider.rs → `Slider`

- Change: `Slider` sizes its filled portion as a fraction of the available width.
  Reason: framework — the fill’s widget description is built before layout knows its width, so a proportional grid track expresses the size without waiting for another build.
  Affect: `Slider`’s fill can end on a fractional pixel where Windows rounds it.

- Change: `Slider` shows its value label using the kit’s tooltip.
  Reason: framework — Inset shows popups in an `Overlay`, where WinUI's ToolTip opens a popup of its own.
  Affect: The slider needs an Overlay ancestor for its value label, whose placement and fade follow the tooltip behavior documented below.

- Change: `Slider` accepts a function returning text to format its value label.
  Reason: language — a Rust formatting callback replaces the object that converts bound values to text in XAML.
  Affect: `Slider` callers provide `thumb_tool_tip_value_converter` for custom text.

- Change: `Slider` vertical tooltip placement assumes a right-handed user.
  Reason: os — this host does not expose Windows handedness settings.
  Affect: `Slider` vertical tooltips prefer the left side of the thumb.

- Change: `Slider` accepts a `SliderTemplate`, a function that builds its template around the `SliderContainer` it is given, and a `key_down` function that sees each key before the slider does.
  Reason: language — Rust callbacks receive the slider's states and template parts in place of a XAML `ControlTemplate`, and a function in place of a derived class's `OnKeyDown` override.
  Affect: A control built on `Slider`, as `ColorPickerSlider` is, supplies `.template(...)` and `.key_down(...)` and keeps the slider's input and value behavior.

## split_view.rs → `SplitView`

- Change: `SplitView` dismisses an overlay pane when Inset recognizes a tap outside it.
  Reason: framework — the pane lives in the application’s overlay, where tap recognition competes with other gestures instead of receiving pointer events from a separate XAML popup.
  Affect: `SplitView` requires an Overlay ancestor, and outside dismissal waits for a recognized tap.

- Change: `SplitView` updates automatic pane width after the pane is measured.
  Reason: framework — Inset cannot read the child’s measured width during widget build.
  Affect: `SplitView` initial layout and content-width changes can take an extra frame to settle.

- Change: `SplitView` reports opening and closing requested by application code after the new layout is ready.
  Reason: framework — Inset forbids updating ancestors or showing overlays during child build.
  Affect: `SplitView` opening and explicit closing notifications arrive after layout, while tapping outside the pane still raises a cancelable event before closing.

## tool_tip/mod.rs → `ToolTip` and `ToolTipService`

- Change: `ToolTipService` draws help text in the application’s overlay rather than a separate popup window.
  Reason: framework — Inset composes overlays and pointer listeners as widgets rather than attaching a popup service to any existing control.
  Affect: `ToolTipService` callers wrap the owner beneath an Overlay, and the tooltip stays inside that window.

- Change: `ToolTipService` uses a 150 ms fade and fixed delays before showing a tooltip.
  Reason: os — this host does not report Windows theme timing or user hover settings.
  Affect: Hovering first shows a tooltip after 800 ms, or after 400 ms if one was recently shown, regardless of the user’s Windows hover settings.

## navigation_scroll_viewport.rs → NavigationView navigation scroll presentation

- Change: `NavigationScrollViewport` uses WinUI’s scrollbar appearance with Flutter’s scrollbar input handling.
  Reason: framework — the port reuses Flutter’s existing behavior for dragging the scroll handle and clicking the track around it.
  Affect: `NavigationScrollViewport` scrolls once by 80% of the viewport on a track press instead of repeatedly paging while the pointer is held down.

- Change: `NavigationScrollViewport` changes its scrollbar thickness instantly after the hover delay.
  Reason: framework — Flutter’s scrollbar accepts a thickness value but does not animate that value itself.
  Affect: The scroll handle jumps to its new width instead of widening or narrowing over 167 ms.

## tab_view.rs → `TabView`

- Change: `TabView` receives its tab list and selection from the application, with a unique identity for each tab that stays the same across rebuilds.
  Reason: framework — Inset rebuilds item descriptions from owner state instead of mutating a collection on the control.
  Affect: Callers apply close, reorder and transfer requests and update the selected index.

- Change: `TabView` builds its headers in a lazy ListView, and each tab keeps its key and focus node across rebuilds.
  Reason: framework — Flutter’s lazy list creates widgets as they come into view instead of using WinUI’s system for creating item containers.
  Affect: Offscreen selection first scrolls to an estimated position and then adjusts to measured headers.

- Change: `TabView` keeps inactive pages’ State alive, but those pages do not paint or receive animation ticks.
  Reason: framework — removing a Flutter page would dispose its State, whereas WinUI can keep the page object alive separately from its displayed content.
  Affect: Inactive pages can still be laid out; Offstage hides them and TickerMode mutes their animation ticks, while timers and other application updates still run.

- Change: `TabView` chooses a valid selected tab immediately when its tab list changes and reports that choice after the frame.
  Reason: framework — a Flutter child cannot trigger an ancestor rebuild while the ancestor’s current build is updating that child.
  Affect: The replacement `TabView` page appears in the current frame, while the owner’s selection callback runs after layout.

## tab_view/drag.rs → TabView tab dragging and reordering

- Change: `TabView` uses native drag recognition, an in-window preview and scrolling near the strip edges.
  Reason: framework — Inset’s drag source and target communicate through the application overlay rather than Windows drag-and-drop services.
  Affect: Drag thresholds and edge-scroll speed follow Inset, and transfers stay within one overlay.

- Change: `TabView` moves neighboring headers immediately after the 200 ms reorder delay.
  Reason: os — Windows supplies the source reposition animation.
  Affect: The insertion gap opens without the Windows motion animation.

- Change: `TabView` does not reserve space in a destination strip while a tab from another strip is hovering over it.
  Reason: framework — Inset reports drag-target entry and movement but does not insert a temporary tab into the destination layout.
  Affect: Cross-strip hover does not widen the strip or open an insertion gap.

## tab_view_item.rs → `TabViewItem`

- Change: `TabViewItem` includes its text and icon in the default drag preview but omits other custom header widgets.
  Reason: framework — Inset cannot capture the displayed header as an image, so it builds a separate widget for the drag preview.
  Affect: Applications supply a custom drag-preview builder for other header content, using keys and focus nodes separate from the original header.

- Change: `TabViewItem` uses Flutter gesture recognition and focus nodes, together with the kit’s tooltips.
  Reason: framework — WinUI can direct all events from a captured pointer to a control, whereas Flutter decides which gesture recognizer handles the press.
  Affect: A drag can cancel a tab click, and clicking a tab leaves keyboard focus unchanged, as described in the shared control conventions.

## navigation_indicator_transition.rs → NavigationView selection-indicator animations

- Change: `NavigationIndicatorTransition` starts moving the line marking selection after the newly selected navigation item is laid out.
  Reason: framework — the animation needs the new item’s position and size, which are unavailable until Flutter layout finishes.
  Affect: The line stays at the previously selected item for up to one extra frame before moving.

- Change: `NavigationIndicatorTransition` speeds up and slows down using an explicit ease-in-out curve when selection moves between a parent item and a nested child.
  Reason: os — the source leaves that acceleration to Windows, so the port substitutes Inset’s cubic curve with control points (0.42, 0) and (0.58, 1).
  Affect: `NavigationIndicatorTransition` can speed up and slow down differently from Windows when changing hierarchy levels.

## navigation_view.rs → `NavigationView`

- Change: `NavigationView` receives its items and selection from the application, with a unique identity for each item that stays the same across rebuilds.
  Reason: framework — Inset rebuilds item descriptions from owner state instead of mutating a XAML item collection on the control.
  Affect: Applications update `selected_item` in `selection_changed` and use `item_invoked` for activation, including activating an already selected item; item ids must remain unique and reserve `NavigationView::SETTINGS_ITEM_ID` for the built-in Settings item.

- Change: `NavigationView` uses Flutter gestures and focus handling, and reports pane changes after layout.
  Reason: framework — WinUI routes input events through its control objects, while this port composes the gesture, focus and pane widgets described above.
  Affect: Starting a scroll can cancel an item click, and application code receives pane-change callbacks after the frame.

- Change: `NavigationView` accepts any widget for its search field and item badges.
  Reason: framework — Inset composes ordinary child widgets, while WinUI requires `AutoSuggestBox` for search and `InfoBadge` for badges.
  Affect: Applications can use the kit's `InfoBadge` or a custom badge, and supply their own search editor until `AutoSuggestBox` is ported.

- Change: `NavigationView` decides which top-level items fit and which go into the overflow menu after measuring them.
  Reason: framework — the widget tree is built before item widths are measured, so moving items between the main row and overflow requires a subsequent build.
  Affect: After item text changes, moving items between the top row and its overflow menu can take an extra frame.

## navigation_view_item.rs → NavigationView item containers

- Change: `NavigationViewItem` keeps its key and stays mounted when hidden or moved between the side pane and top bar.
  Reason: framework — Flutter State belongs to a mounted element, so keeping the element preserves custom item state across those moves.
  Affect: `NavigationViewItem` preserves custom item state across presentation changes, and its collapsed children remain mounted but cannot paint or take focus.

- Change: `NavigationViewItem` uses a replacement font symbol for its arrow and rotates it over 250 ms.
  Reason: os — Windows supplies the original animated arrow, while the replacement font contains only a static arrow.
  Affect: Navigation arrows can differ from Windows in shape and in how they move while opening or closing a child list.

## progress_bar.rs → `ProgressBar`

- Change: `ProgressBar` jumps to the new filled length when its value changes and omits the fade when switching from moving segments to a known progress value.
  Reason: os — the WinUI template asks Windows to perform those movement and fade effects through `RepositionThemeAnimation` and `FadeInThemeAnimation`, but their durations and acceleration are not defined in the checkout.
  Affect: `ProgressBar` still animates moving segments and pause/error colors using the timings written in the template; only the Windows-supplied effects above are omitted.

## expander.rs → `Expander`

- Change: `Expander` asks the application to change whether its content is shown.
  Reason: framework — like a controlled Flutter widget, it receives a boolean and a callback rather than changing the supplied boolean itself.
  Affect: The application must handle the callback and rebuild with the new value before the section opens or closes.

- Change: `Expander` reports that its content is opening or closing through callbacks that run after layout.
  Reason: framework — changing the supplied boolean rebuilds the widget, and a callback that updates application state cannot run safely during that build.
  Affect: Application code receives the notification at the end of the frame rather than immediately when it changes the boolean.

- Change: `Expander` rotates its expand/collapse arrow over 250 ms.
  Reason: os — Windows supplies the original animated icon, while the replacement font supplies only a static arrow that Inset rotates.
  Affect: The arrow points in the correct direction, but its shape and movement differ from Windows.

- Change: `Expander` keeps hidden content’s State alive, but ticker-driven animations stop receiving ticks.
  Reason: framework — removing a Flutter widget would dispose its State, so hiding it preserves the child state for the next expansion.
  Affect: Child widget values survive closing and reopening, and TickerMode mutes their animation ticks while hidden; timers and other application updates still run.

## info_bar.rs → `InfoBar`

- Change: `InfoBar` asks the application to update whether it is open.
  Reason: framework — like a controlled Flutter widget, it receives a boolean and a callback rather than changing the supplied boolean itself.
  Affect: Handle `is_open_changed` by storing its requested boolean and rebuilding, including a request for `true` when the closing callback cancels a close started by application code.

- Change: `InfoBar` reports opening and closing requested by application code after layout.
  Reason: framework — these events may update application state, so they wait until the current build has finished.
  Affect: The opening notification arrives at the end of the frame, and a closing notification can then cancel the close before the banner is hidden.

- Change: `InfoBar` reports completion of a close-button request after the application rebuilds it as closed.
  Reason: framework — the button can request a new value, but only the application can supply that value to the next build.
  Affect: The click and cancelable closing callbacks run immediately; an accepted close reports completion only after the application rebuilds with the open value set to false, while a canceled close immediately reports that the banner is open again.

- Change: `InfoBar` keeps child State alive while closed, but ticker-driven animations stop receiving ticks.
  Reason: framework — removing a Flutter widget would dispose its State, so hiding the banner preserves the child state for the next opening.
  Affect: Child widget values survive closing and reopening, and TickerMode mutes their animation ticks while hidden; timers and other application updates still run.

- Change: `InfoBar` draws its status icons with the bundled replacement font over a colored circle.
  Reason: os — the original matching foreground and background symbols belong to a font supplied with Windows.
  Affect: The replacement icon shapes differ from Windows, while the warning uses an exclamation mark inside a circle that matches its circular background.

- Change: `InfoBar` accepts a widget-building callback to customize its close button.
  Reason: language — a Rust callback receives the button’s current state and content in place of a XAML style declaration.
  Affect: Applications can change the button’s appearance while keeping its existing click and close behavior.

- Change: `InfoBar` uses a callback rather than a command object for close-button actions.
  Reason: framework — WinUI command objects also report whether an action is available, while the kit’s button callbacks only perform the action.
  Affect: Applications capture action arguments in their callback, but the button does not automatically disable itself when that action becomes unavailable.

## progress_ring.rs → `ProgressRing`

- Change: `ProgressRing` supports the built-in loading and percentage animations, but does not accept a custom animation.
  Reason: framework — WinUI plays replaceable animation objects through Windows Composition, while this port draws the two built-in animations directly with Inset's canvas and animation clock.
  Affect: Applications can change the range, value, activity and colors, but cannot replace the animation through WinUI's `DeterminateSource` or `IndeterminateSource` properties.

## info_badge.rs → `InfoBadge`

- Change: `InfoBadgeStyle` selects the badge's background color but does not supply an icon or icon-specific padding.
  Reason: os — WinUI's named icon styles depend on symbols from the Windows icon font, which this kit replaces with the bundled Fluent System Icons font.
  Affect: Applications supply `icon_source` themselves and use 4 logical pixels of top padding, 2 at the bottom and none on either side when reproducing the attention or informational icon styles.

## flyout/mod.rs → `Flyout` and `FlyoutBase`

- Change: `Flyout` keeps its content mounted while hidden and displays it inside the application’s root Overlay.
  Reason: framework — Flutter preserves child State through mounted widgets and draws overlays inside the existing window, while WinUI retains detached content and can create separate popup windows.
  Affect: Reopening the same Flyout from another button preserves its child State, but the flyout cannot extend beyond the application window.

- Change: `Flyout` uses Flutter’s ModalBarrier to handle outside input in `FlyoutShowMode::Standard`, except when a `MenuBar` groups its headers with the popup.
  Reason: framework — WinUI's host distinguishes pointer buttons during popup hit testing, while Flutter's native barrier blocks input behind it and recognizes a completed tap from any button.
  Affect: Standalone flyouts dismiss on release rather than press, and consume an outside right-click rather than passing it to the control behind the flyout.

- Change: `Flyout` opens and closes without a visual transition.
  Reason: os — WinUI obtains popup motion and timing from Windows rather than declaring them in the template.
  Affect: The flyout’s visible container appears and disappears immediately.

- Change: `Flyout` accepts a widget builder to customize the visible container around its content.
  Reason: language — Rust supplies widget construction through callbacks rather than runtime XAML template resources.
  Affect: Applications provide `flyout_presenter_style` when replacing the default border, scrolling or content layout.

## flyout/shadow.rs → `Flyout`, `MenuFlyout`, `ContentDialog`, `NumberBox` and `ComboBox` popup shadows

- Change: `Flyout`, `MenuFlyout`, `ContentDialog`, the popup of `NumberBox` and the drop-down of `ComboBox` paint their shadows using Inset's native BoxShadow blur with WinUI's default shadow colors, offsets and elevations: a flyout's rises with each nested submenu, a dialog's is the highest the recipe allows, the number box popup's is lower than a flyout's, and the combo box drop-down's matches a flyout's.
  Reason: os — Windows Composition performs WinUI's blur, and its blur-radius conversion is not specified, so the port interprets the radii from WinUI's `DropShadowRecipe.h` using Flutter's native conversion.
  Affect: Popups have a soft shadow that becomes stronger in dark theme and at deeper submenu levels, but its softness can differ from Windows; the shadow is clipped at the application window's edges.

## content_dialog.rs → `ContentDialog`

- Change: `ContentDialog::show_async` pushes the dialog onto the root navigator as a dialog route whose modal barrier is the smoke layer, instead of opening one popup for the smoke layer and another for the dialog.
  Reason: framework — Inset's navigator already has modal routes that block input to the page beneath and keep keyboard focus inside, while WinUI builds modality from two popups and its own focus cycling.
  Affect: A dialog needs a `Navigator` above the context that shows it, as `WidgetsApp` provides, and returns keyboard focus to the page the way any popped route does.

- Change: `ContentDialog` always shows over its whole window: there is no `ContentDialogPlacement` choosing `InPlace` or `UnconstrainedPopup`.
  Reason: framework — a XAML control is at once the object `ShowAsync` is called on and an element that can sit in a page, while an Inset widget is a description its parent rebuilds, so the persistent dialog object has no element of its own to place in a page; and Inset draws overlays only inside the window.
  Affect: A dialog cannot be confined to part of a page or extend beyond the application window.

- Change: `ContentDialogButtonClickEventArgs` and `ContentDialogClosingEventArgs` reach their handlers as an `Rc`, with `cancel` and `set_cancel` in place of a `Cancel` property.
  Reason: language — WinRT event arguments are shared objects a handler may keep after it returns, which in Rust is a reference-counted value whose fields change through `Cell`.
  Affect: A handler that decides later clones the `Rc`, takes `get_deferral`, and calls `set_cancel` before it completes the deferral.

- Change: The future `ContentDialog::show_async` returns cannot be canceled, and dropping it leaves the dialog open.
  Reason: language — `ShowAsync` returns an `IAsyncOperation` whose `Cancel` closes the dialog without Closing, while an Inset future has no cancellation and runs to completion.
  Affect: An application closes a shown dialog with `hide`, which raises Closing, or with `dispose`.

- Change: `ContentDialog` keeps the theme and text direction of the context that showed it for as long as it is shown.
  Reason: framework — the dialog's route lives in the root navigator's overlay, usually above the page's `ThemeScope`, so it inherits no theme and holds what the showing context had.
  Affect: A theme change while a dialog is open reaches the dialog the next time it is shown.

- Change: `ContentDialog` has click handlers for its buttons but no `PrimaryButtonCommand`, `CommandParameter` or button style properties.
  Reason: language — XAML binds an `ICommand` object with its parameter and a `Style` resource per button, while a Rust closure captures both and the kit's buttons choose between its default and accent styles.
  Affect: Applications put a command's work in `primary_button_click` and its siblings, and the command buttons always look like the kit's default buttons, the default button accented.

- Change: `ContentDialog::dispose` releases the dialog object, closing a shown dialog first without its Closing and Closed events.
  Reason: language — WinUI's dialog is released when nothing refers to it, while the kit's persistent objects, like `Flyout`, are released by their owner.
  Affect: The owner of a dialog disposes it when the owner is disposed; a shown dialog then disappears and its `show_async` result is `None`.

## number_box.rs → `NumberBox`

- Change: `NumberBox` reports through `value_changed` only the values it asks for itself, from typing, keys, the wheel and its spin buttons, and shows its value as the application passes it back.
  Reason: framework — WinUI's `Value` is a property of a persistent control, which the control sets itself and every change of which raises `ValueChanged`, while an Inset widget is rebuilt from the application's state.
  Affect: An application stores `new_value` and rebuilds; a value it sets raises no `ValueChanged`, and a value it does not pass back is undone in the box at the box's next build.

- Change: `NumberBox` raises the `ValueChanged` of a change it makes to what the application passes, coercing a `value` out of range or reading a new `text` or `validation_mode`, at the end of the frame.
  Reason: framework — these changes are found while the box is rebuilt, and Inset forbids a callback that updates the application's state during a build.
  Affect: The application hears of such a change one frame late, and a new `text` shows as given for that frame before the box shows its value.

- Change: `NumberBox::text` is read as committed input whenever the application passes a new one, but the box does not report the text it shows.
  Reason: framework — XAML can bind `Text` both ways, so the control's own updates reach the application, while an Inset widget only receives what the application passes.
  Affect: An application that needs the shown text formats `value` itself, and passing the same `text` again is not read again.

- Change: `NumberBox::number_formatter` takes any `NumberFormatter` with `PartialEq`, a trait of `format_double` and `parse_double`, and formats the text again only when a formatter of another type or value is passed.
  Reason: language — WinUI takes a Windows.Globalization object that must implement two interfaces, checked when it is set, while a Rust trait states both methods; and a widget is described again on every build, where XAML sets the property once, so the box compares formatters by value.
  Affect: Applications implement `NumberFormatter` and `PartialEq` for their own formatting and can build the formatter inline; an equal one leaves text being typed alone.

- Change: `NumberBox` opens its compact spin buttons in the root Overlay of the application window.
  Reason: framework — the template's `UpDownPopup` sets `ShouldConstrainToRootBounds` to false to open in a window of its own, while Inset draws popups only inside the application window.
  Affect: Next to a window edge the popup is cut off by the edge instead of extending past it.

## number_box/formatter.rs → `DecimalFormatter` and NumberBox's display rounding

- Change: `DecimalFormatter` and `NumberBox`'s rounding stand in for the Windows `DecimalFormatter` and `SignificantDigitsNumberRounder` NumberBox creates: a value is rounded to 10 significant digits, a halfway value toward positive infinity, and written in its shortest decimal form with `.` and `-` and no grouping, and text is read as an optional `-`, digits and one `.`.
  Reason: os — Windows.Globalization formats, rounds and parses numbers following the user's regional settings, which the repository does not contain and this host does not provide.
  Affect: Numbers always follow the en-US conventions: text with grouping separators, exponents or another locale's decimal separator is not a number, and formatting can differ from Windows beyond those conventions.

## combo_box.rs → `ComboBox` and `ComboBoxItem`

- Change: `ComboBox` reports through `selection_changed` only the selections it asks for itself, from the pointer, keys, the wheel, typing and an editable box's text, and shows the selection the application passes back in `selected_index` and `custom_value`.
  Reason: framework — WinUI's `SelectedIndex` and `SelectedItem` are properties of a persistent control, which the control sets itself and every change of which raises `SelectionChanged`, while an Inset widget is rebuilt from the application's state.
  Affect: An application stores the selection the event asks for and rebuilds; a selection it sets raises no `SelectionChanged`, and one it does not pass back is undone in the box at the box's next build.

- Change: `ComboBoxSelectionChangedEventArgs` carries `removed_item` and `added_item`, each an `Option<ComboBoxSelectedItem>` holding an item's index and id or a custom value's text, in place of the `RemovedItems` and `AddedItems` lists of objects.
  Reason: language — XAML's lists hold whatever objects the items are, while a box with one selection removes and adds at most one, which Rust states as an `Option` of an enum.
  Affect: A handler reads `selected_index()` or `custom_value()` of the arguments, or matches `added_item`, instead of casting the first element of `AddedItems`.

- Change: `ComboBoxItem` is a unique `id`, a `content` widget, an optional `plain_text` and `is_enabled`, in place of the objects of `Items` or `ItemsSource` that the box shows through `ItemTemplate` or `DisplayMemberPath`.
  Reason: language — the source turns an item object into the string it searches with `DisplayMemberPath` or the object's `ToString`, while a widget has no string form, so the text is given beside the content.
  Affect: Type-ahead and an editable box find only items with `plain_text`, which `ComboBoxItem::text` sets; an item without it ends a search, as an item without a string does in WinUI.

- Change: `ComboBox` opens and closes its drop-down itself unless the application passes `is_drop_down_open`, in which case it asks through the callback and shows the state the application passes.
  Reason: framework — WinUI's `IsDropDownOpen` is a property that both the control and the application set, while an Inset widget is rebuilt from the application's state.
  Affect: An application that owns the drop-down stores the state it is asked for; a state it declines is undone at the box's next build without asking again, and a state it passes takes effect, with `DropDownOpened` or `DropDownClosed`, at the end of that frame.

- Change: `ComboBox` opens its drop-down in the root Overlay of the application window and places it within the window.
  Reason: framework — the source sets the popup's `ShouldConstrainToRootBounds` to false, so that the drop-down opens in a window of its own placed within the monitor's work area, while Inset draws popups only inside the application window.
  Affect: The drop-down never extends past the application window, so in a short window it shows fewer items and scrolls sooner than WinUI's.

- Change: `ComboBox` places its drop-down using the item heights laid out in earlier frames, and one line of text for items not laid out yet, and makes it as wide as the widest item laid out since it opened.
  Reason: framework — the source measures any item's container on demand while it arranges the popup, while Inset lays a widget out only in the frame's layout pass, and the lazy list lays out the items in its view and a margin beyond it.
  Affect: Items taller than one line can move the drop-down a frame after it opens, and an item laid out just beyond the drop-down's view widens it before it is scrolled into view.

- Change: `ComboBox` opens and closes its drop-down without the template's split animation, and raises `DropDownClosed` as soon as the drop-down closes.
  Reason: os — the template's `SplitOpenThemeAnimation` and `SplitCloseThemeAnimation` take their motion and timing from Windows, and the source raises `DropDownClosed` when the close animation completes.
  Affect: The drop-down appears and disappears at once, and `drop_down_closed` runs in the frame the drop-down closes.

- Change: `ComboBox` uses a static Fluent chevron for the template's animated `DropDownGlyph`.
  Reason: os — the Windows animated-chevron asset has no matching animation in the bundled Fluent font.
  Affect: The chevron changes color with the box's state but does not animate its shape.

- Change: A non-editable `ComboBox` takes the characters for type-ahead from the key presses that reach it.
  Reason: framework — XAML raises `CharacterReceived` for every character the window receives, including those an input method commits, while Inset delivers an input method's text only to a focused text field and otherwise reports the character a key press produces.
  Affect: Text composed with an input method, such as kana converted to kanji, does not select items in a non-editable box.

- Change: An editable `ComboBox` searches its items for submitted text as soon as `text_submitted` returns unhandled, before the application can pass an item it added in the handler.
  Reason: framework — a WinUI handler can add the text to an observable `ItemsSource`, which the box sees at once and then selects, while an Inset widget sees new items only when the application rebuilds it.
  Affect: An application that adds submitted text as an item without setting `handled` receives the text as a custom value in `selection_changed`, and selects the new item by passing its index in `selected_index`.

- Change: An editable `ComboBox` opens its drop-down directly below the box rather than below an input method's candidate window.
  Reason: os — the source adds the height of the candidate window that Windows reports through the text box's `CandidateWindowBoundsChanged`, which this host does not report.
  Affect: While text is being composed with an input method, its candidate window can cover the top of the drop-down.

- Change: `ComboBox` takes pointer input only within its own bounds, not in the 4-pixel focus border (`HighlightBackground`) that the template draws around the box.
  Reason: framework — XAML hit-tests an element where it is drawn, even outside its parent's layout bounds and at zero opacity, while Inset tests a child only where its parent's bounds contain the point.
  Affect: A press or hover up to 4 pixels outside the box's edges reaches what lies beneath instead of the box.

## combo_box/search.rs → ComboBox text search

- Change: `ComboBox` matches typed text to an item's text ignoring case, the width of full-width Latin letters, digits and punctuation, and the difference between hiragana and katakana, but not diacritics, as a substitute for the Windows comparison.
  Reason: os — the source compares with `FindNLSStringEx` under the user's locale, with flags that also ignore diacritics and apply the locale's casing, which Windows implements and this host does not provide.
  Affect: Typing `e` does not find an item that starts with `é`, and case follows Unicode's default rules rather than the user's language, as Turkish dotted and dotless i show.

## drop_down_button.rs → `DropDownButton`

- Change: `DropDownButton` uses a static Fluent chevron for the source animated icon.
  Reason: os — the Windows animated-chevron asset has no matching animation in the bundled Fluent font.
  Affect: The chevron changes color on hover and press but does not animate its shape.

## toggle_split_button.rs → `ToggleSplitButton`

- Change: `ToggleSplitButton` receives its checked value from the application and reports the next value through a callback.
  Reason: framework — Inset rebuilds widgets from application state, while WinUI changes the control's own checked property before raising its change event.
  Affect: Applications store the requested value in `is_checked_changed` for the next rebuild, then `click` runs with that updated application state; assigning a checked value directly does not invoke the change callback.

## split_button.rs → `SplitButton` and `ToggleSplitButton`

- Change: `SplitButton` and `ToggleSplitButton` use an action callback instead of a command object.
  Reason: framework — WinUI commands can report whether they are available, while the kit's callbacks only perform the action.
  Affect: Applications capture action arguments in the callback and set `is_enabled` themselves when the action becomes unavailable.

- Change: `SplitButton` and `ToggleSplitButton` use a static Fluent chevron for the menu button.
  Reason: os — the Windows animated-chevron asset is unavailable in the bundled icon font.
  Affect: The arrow changes color with the button's state but does not animate its shape.

## menu_flyout/mod.rs → `MenuFlyout`

- Change: `MenuFlyout` receives toggle and radio-item checked values from the application.
  Reason: framework — Inset builds menu descriptions from application state, while WinUI stores checked values on its item objects and coordinates radio items by group name.
  Affect: Applications use the callbacks passed to `MenuFlyoutItem::toggle` and `MenuFlyoutItem::radio` to replace the menu's items with updated checked values and keep radio choices mutually exclusive.

## menu_bar.rs → `MenuBar` and `MenuBarItem`

- Change: `MenuBar` receives header descriptions with application-owned ids instead of live child controls.
  Reason: framework — WinUI retains control objects in its collection, while Inset rebuilds widgets from data and uses keys to preserve their State.
  Affect: Applications keep header ids unique and stable when changing titles, commands or display order; replacing an id creates a new header and menu.

- Change: `MenuBar` groups its headers and menus using Flutter's TapRegion mechanism.
  Reason: framework — WinUI selects which elements receive input through its popup layer, while Flutter groups widgets and consumes gestures outside that group.
  Affect: Outside taps dismiss the menu without activating ordinary tap callbacks behind it, but raw pointer listeners behind the menu can still observe those pointer events.

## flyout/placement.rs → `MenuFlyout` point placement

- Change: `MenuFlyout` starts pen-triggered menus at the requested point, aligning the left edge in left-to-right layouts and the right edge in right-to-left layouts.
  Reason: os — WinUI adjusts pen menus using the Windows handedness preference, which Inset's host does not expose.
  Affect: Pen-triggered menus adjust to fit the window but do not switch sides to avoid the user's hand.

## breadcrumb_bar.rs → `BreadcrumbBar` and `BreadcrumbBarItem`

- Change: `BreadcrumbBar` receives item descriptions with stable ids and widget content instead of a XAML item collection and data template.
  Reason: framework — Inset rebuilds widgets from application data, while WinUI retains item objects and asks its element factory to create their controls.
  Affect: Applications preserve item ids across collection changes; inline and overflow content have separate widget State, and `item_clicked` reports the item and its index in the current collection for inline clicks or the copied collection from when the overflow menu opened for dropdown clicks, without changing the path.

- Change: `BreadcrumbBarItem` recognizes dropdown clicks with Inset’s native tap recognizer.
  Reason: framework — WinUI tracks a pointer id and invokes the dropdown item on release, while Inset resolves taps alongside competing drag and scroll gestures.
  Affect: Dragging the overflow list can cancel a pending item click using the same native behavior as the kit’s buttons.

## color_spectrum.rs → `ColorSpectrum`

- Change: `ColorSpectrum` reports through `color_changed` only the colours the user picks, with the `HsvColor` it asks for, and shows the `hsv_color` the application passes back.
  Reason: framework — WinUI's `Color` and `HsvColor` are properties of a persistent control, which the control sets itself and every change of which raises `ColorChanged`, while an Inset widget is rebuilt from the application's state.
  Affect: An application stores `args.hsv_color` and rebuilds; an `hsv_color` it sets raises no `ColorChanged`, and one it does not pass back is undone at the spectrum's next build.

- Change: `ColorSpectrum` takes its colour only as `hsv_color`, whose `Color` is `HsvColor::to_color`, and an RGB colour becomes an `HsvColor` through `HsvColor::from_color`.
  Reason: framework — WinUI keeps `Color` and `HsvColor` as two properties that set each other, while a widget the application rebuilds needs one value to compare with what it shows.
  Affect: Where XAML sets `Color`, a caller writes `ColorSpectrum::new(HsvColor::from_color(color), …)`.

- Change: `HsvColor` names its components `hue`, `saturation`, `value` and `alpha` where XAML's `Vector4` has `X`, `Y`, `Z` and `W`.
  Reason: language — a Rust struct gives the four numbers their meanings as field names instead of reusing a general vector type.
  Affect: Code reading `HsvColor.X` reads `hsv_color.hue`.

- Change: `ColorSpectrum` computes its spectrum images on the UI thread, in the update that changes its size, `Shape`, `Components` or a component's range.
  Reason: framework — WinUI computes the pixels on a thread-pool work item and hands them back through the dispatcher, which the kit's controls do not use; the largest spectrum ColorPicker shows, 336 square, takes 2 to 5 ms in an optimized build.
  Affect: The spectrum appears in the frame after its size is known, without the empty frames WinUI can show while its work item runs, and that frame takes the time of the computation.

## color_picker_slider.rs → `ColorPickerSlider`

- Change: `ColorPickerSlider` is a widget of its own that builds a `Slider` with the template of `ColorPickerSliderStyle` and an `OnKeyDown` override, and it has the Slider properties that template shows.
  Reason: language — the XAML control derives from `Slider` and inherits all of its properties, while a Rust widget composes another.
  Affect: `ColorPickerSlider` has no tick, `SnapsTo`, `SmallChange` or thumb tooltip converter properties; an application needing them uses `Slider` with its own template.

- Change: `ColorPickerSlider` opens its tooltip of the value and the colour's name when it takes the focus from the keyboard, not when it is pressed.
  Reason: framework — the source opens the tooltip from `OnGotFocus`, and the kit's `Slider` does not take the focus on a press (see the shared conventions).
  Affect: Pressing or dragging the thumb does not open the tooltip; once the value has changed, it opens as other tooltips do when the pointer rests on the thumb.

## color_picker.rs → `ColorPicker`

- Change: `ColorPicker` reports through `color_changed` only the colours it asks for itself, from its spectrum, sliders and text boxes, and shows the `color` the application passes back.
  Reason: framework — WinUI's `Color` is a property of a persistent control, which the control sets itself and every change of which raises `ColorChanged`, while an Inset widget is rebuilt from the application's state.
  Affect: An application stores `new_color` and rebuilds; a colour it sets raises no `ColorChanged`, a colour it does not pass back is undone in the picker at its next build, and the hue and saturation of a grey or black are kept only while the application passes back the colours the picker asks for.

- Change: `ColorPicker` raises the `ColorChanged` of a change it makes to the colour the application passes, bringing it into a narrowed `MinHue` to `MaxValue` range, at the end of the frame.
  Reason: framework — these changes are found while the picker is rebuilt, and Inset forbids a callback that updates the application's state during a build.
  Affect: The application hears of such a change one frame late.

## tree_view.rs → `TreeView`, `TreeViewItem` and `TreeViewList`

- Change: `TreeView` receives its nodes, each node's `is_expanded` and the selected nodes from the application, and reports what it asks to change through `expanding`, `collapsed`, `selection_changed` and `move_requested`.
  Reason: framework — WinUI's `TreeViewNode` objects and `SelectedNodes` belong to the control, which changes them itself, while an Inset widget is rebuilt from the application's state.
  Affect: An application stores what each callback asks for and rebuilds; a change it does not store is undone at the tree's next build, and a change it makes itself raises no event.

- Change: `TreeView` reports the moves a drop or Shift+Alt with an arrow makes as `TreeViewNodeMove`s, each a node with its new parent and its index counted after the node leaves its parent, and reports a parent the moves leave without children through `collapsed`.
  Reason: framework — WinUI's drop and keyboard reorder handlers remove and insert `TreeViewNode`s in the control's own collections, which an application owning its tree has to do itself.
  Affect: An application applies the moves in order and stores the collapse; `DragItemsCompleted`'s `new_parent_item` is the parent the moves give the first item.

- Change: `TreeView` draws `selected_nodes` as the application passes them, without the rules `SelectedNodes` applies when a node is added to it.
  Reason: framework — WinUI's `SelectedNodes` is a collection on the control whose `Append` also selects the node's descendants and completes its ancestors, while an Inset widget only reads what the application passes.
  Affect: In Multiple mode, an application selecting a node itself also passes its descendants, and a parent whose children are all passed is drawn partly selected until it is passed too; the check boxes and Space apply WinUI's rules and report the full selection in `selected_nodes`.

- Change: In Single mode, `TreeView` draws every node of `selected_nodes` selected, not only the list's selection, which is the first.
  Reason: framework — WinUI's Single-mode list holds one selected item of its own, while an Inset widget draws the selection its application passes, which an application such as a layers panel extends with clicks it reads itself.
  Affect: An application can show several nodes selected in Single mode, deciding them from `item_invoked` and the keys held; clicks, the arrow keys and Ctrl+click still ask for one node, or none, through `selection_changed`.

- Change: `TreeView` has no `Expand`, `Collapse`, `SelectAll`, `SelectedItem`, `NodeFromContainer` or `ContainerFromNode`, and no content mode with `ItemsSource`, `ItemTemplate`, `ItemTemplateSelector`, `ItemContainerStyle` or `ItemContainerStyleSelector`.
  Reason: language — the methods act on the control's own nodes and containers, and the content mode turns application objects into rows through XAML templates, while in Rust the application passes `TreeViewNode`s with the widget each row shows.
  Affect: An application sets `is_expanded` and `selected_nodes` itself, chooses each node's `content` widget where it would choose a template, and cannot set a property, such as `AllowDrop`, on a single row.

- Change: `TreeView`'s event arguments name nodes by their `id`, `TreeViewSelectionChangedEventArgs` adds the new `selected_nodes`, and `TreeViewItemInvokedEventArgs` has no `Handled`.
  Reason: language — WinUI passes the node objects, which a handler reads further from the control, while a Rust handler receives values; nothing in TreeView reads `Handled`.
  Affect: Handlers look nodes up by id in the application's own tree.

- Change: `TreeView` builds its rows in a lazy `ListView`, and Left and Right move the focus to a parent or first child whose row is not built yet, scrolling to it.
  Reason: framework — Flutter's lazy list creates rows only near the view, and estimates the place of rows not laid out yet, where WinUI's ItemsStackPanel keeps a cache of containers and `HandleExpandCollapse` only focuses a container it has.
  Affect: Paging and scrolling to rows far from the view can land a few pixels off and correct themselves once the rows are laid out, and Left from a deep row reaches a parent above the cached range, where WinUI's key does nothing.

- Change: `TreeView` raises no `SelectionChanged` when a selected node the application passes while it is hidden becomes a row, and raises the one for a selected node the application's change hides at the end of the frame.
  Reason: framework — WinUI's list selects the container it creates for a node the view model has selected and unselects one it removes, each with an event, while the tree draws the selection from `selected_nodes` and finds such changes while it is rebuilt, when Inset forbids a callback that updates the application's state.
  Affect: Expanding the parent of a selected node raises no event, and the application hears of a selection its own collapse cleared one frame late.

- Change: `TreeViewItem`'s chevron uses the bundled Fluent chevrons for the template's `\uE76C` and `\uE70D` glyphs of `SymbolThemeFontFamily`.
  Reason: os — the Segoe Fluent Icons font ships with Windows and is not bundled with the kit.
  Affect: The chevrons can differ slightly in shape and weight from Windows'.

- Change: `TreeView` adds, removes and moves rows at once, and fades a dragged row out at once, without the template's `ContentThemeTransition`, `ReorderThemeTransition`, `EntranceThemeTransition` and `FadeOutThemeAnimation`.
  Reason: os — these theme transitions and animations take their motion and timing from Windows.
  Affect: Rows appear when a node expands, and the gap of a drag opens, without movement.

## tree_view/drag.rs → TreeView drag and drop

- Change: `TreeView` starts a drag with Inset's native drag recognition and shows the dragged row as a separate widget under the pointer, opaque and without the system's drop glyph or caption.
  Reason: framework — Inset cannot capture the displayed row as an image for the system drag visual, and its drags stay in the window.
  Affect: The drag starts after Inset's drag threshold, and the preview is the row in its pressed state, with the count of nodes for a drag of several, built from the node's `content` a second time, so content holding a `GlobalKey` cannot be dragged.

- Change: `TreeView`'s drags carry no `DataPackage` and reach only the tree they started in: `TreeViewDragItemsStartingEventArgs` has no `Data`, and drags from outside are not taken.
  Reason: framework — Inset's drag and drop passes widgets' data within a window, with no transfer of formats to other controls or applications.
  Affect: Nodes cannot be dragged to another tree, control or application, or dropped in from them.

- Change: `TreeView` goes over the pointer's place again when a gap opens, a hovered row expands, the list edge-scrolls and a drop happens.
  Reason: framework — Windows keeps reporting a drag's position while the pointer is still, and Inset reports it only when the pointer moves.
  Affect: What lies under a still pointer decides the drop as on Windows, though the row a still pointer points at shows only after one of these.

- Change: `TreeView` moves nothing for a drag when `can_drag_items` is false and `can_reorder_items` true.
  Reason: framework — without `DragItemsStarting` TreeViewList knows no dragged node, and WinUI's ListView then reorders the flattened rows of its view model, which in Inset is no collection a list owns.
  Affect: The rows open a gap while dragged and the drop leaves the tree as it was, where WinUI moves a row without changing the tree.

## selector_bar.rs → `SelectorBar` and `SelectorBarItem`

- Change: `SelectorBar` lays its items out in a row inside the kit's scroll viewport, and does itself what the template's `ItemsView` does for it: single selection on a click (Control with a click deselects the selected item), Left, Right, Home and End moving the focus with selection following it, and one Tab stop at the selected item.
  Reason: framework — WinUI's `ItemsView` makes item containers from a collection through `ItemsRepeater`, while Inset builds the items as widgets from the application's list, as the kit's `TabView` and `TreeView` do with their lists.
  Affect: `ItemsView` is not a kit control; only the parts `SelectorBar`'s spec relies on are reproduced.

- Change: `SelectorBar` receives its selection from the application as an index, and `selection_changed` asks the application to change it.
  Reason: framework — Inset rebuilds a control's description from its owner's state instead of the control holding a `SelectedItem` property of its own.
  Affect: Callers set `selected` from `selection_changed`, and selecting an item from code raises no event, where setting WinUI's `SelectedItem` raises `SelectionChanged`.

## Deferred

- `MenuBar` activation by pressing Alt followed by a letter, and the temporary labels showing those letters, remain deferred. Trigger: The kit provides shared access-key mode and label display.

- `ProgressRing` starts its arc at twelve o'clock, but that starting point remains unverified against Windows. Trigger: A Windows capture establishes where the original ellipse starts drawing its arc.

- Repeat buttons always activate on press rather than release. Trigger: A consumer needs release-time activation in the shared button handler.
- Arrow keys do not yet move between radio buttons in a group. Trigger: The group provides a list of buttons that can receive focus.
- Keeping the slider’s header widget mounted while its header is absent remains deferred. Trigger: A consumer needs that hidden header state to survive.
- ToggleSwitch state-color fades remain deferred. Trigger: The duration for each supported state change is identified from the source or measured on Windows.
- ToggleSwitch tap bounds still include the header. Trigger: A source review of hit areas using native gestures is available.
- ToggleSwitch knob animation timing remains unverified. Trigger: A recording from Windows is available.
- Collapsed Grid children are omitted rather than retained at zero size. Trigger: A consumer needs retained collapsed children.
- RepeatButton timer-rate verification remains deferred. Trigger: A Windows click-rate measurement is available.
- HyperlinkButton does not expose NavigateUri. Trigger: A host URL launcher is available.
- Slider default-value verification for StepFrequency, TickFrequency and SnapsTo remains deferred. Trigger: The Windows type table or a runtime measurement is available.
- Slider gamepad operation, including switching between navigating controls and adjusting the slider’s value, remains deferred. Trigger: A host supplies gamepad or remote-control input.
- The slider’s larger value step for accessibility actions remains deferred. Trigger: Accessibility support uses that step.
- SplitView system-back handling, element sounds and Xbox focus/dimming behavior remain deferred. Trigger: A host exposes those facilities.
- Template and Slider-key right-to-left mirroring remain deferred. Trigger: A consumer needs right-to-left layout, prompting a source comparison of each affected template and keyboard direction.
- CheckBox indeterminate sweep animation remains deferred. Trigger: A recording from Windows is available.
- `RatingControl` remains deferred because the bundled icon font supplies outlined stars but no filled stars for selected ratings. Trigger: A matching filled-star glyph is bundled.
- Additional TextBlock styles remain deferred. Trigger: The next control port is selected.
- `ContentDialog` placement inside part of a page (`InPlace`) remains deferred. Trigger: An application needs a dialog confined to part of a page.
- `NumberBox`'s accessibility, the name it gives its text box from its header and range and the automation event of a value change, remains deferred. Trigger: The kit gains accessibility support.
- `ComboBox`'s touch presentation remains deferred: the larger item padding of `TouchInputMode`, the drop-down's touch minimum width, the scrolling layout a touch-opened long list uses, and an editable box opening its drop-down when a touch focuses it and selecting its text at the next tap. Trigger: The kit distinguishes touch input in its controls.
- `ComboBox`'s gamepad and remote-control input, its `GameControllerInputMode` item padding and the `LightDismissOverlayMode` overlay remain deferred. Trigger: A host supplies gamepad or remote-control input.
- `ComboBox`'s accessibility, its automation peer and the automation events of opening, closing and selection, remains deferred. Trigger: The kit gains accessibility support.
- `ComboBox`'s element sounds and its closing on system back remain deferred. Trigger: A host exposes those facilities.
- `ComboBox::text`, an editable box's text as a property apart from its selection, remains deferred. Trigger: An application needs to set or read an editable box's text before it is committed.
- Mica wallpaper backdrops remain deferred. Trigger: A host material path supplies the wallpaper processing absent from the source.
- Text controls’ spell-check suggestion menus, touch command-bar presentation, input-scope-specific soft keyboards and OS credential/autofill integration remain deferred. Trigger: The corresponding native services or additional control ports are available.
- TabView tear-out, caption regions, window movement and cross-window drag transport remain deferred. Trigger: A host provides window management and native drag transport.
- Tab gamepad navigation and interaction, high-contrast colors and accessibility support remain deferred. Trigger: The corresponding input, theme and accessibility support is available.
- TabView insertion, removal and reorder animations remain deferred. Trigger: Windows timing is verified or a host animation service is available.
- Navigation integration with title-bar space, window borders, system sounds and page-change animations remains deferred. Trigger: The host exposes the corresponding window or animation support.
- Navigation by gamepad shoulder buttons, directional focus movement, accessibility and high-contrast colors remain deferred. Trigger: The corresponding input, accessibility and theme support is available.
- Standalone ScrollViewer and ScrollBar APIs remain deferred. Trigger: A consumer needs those public controls beyond NavigationView’s internal adapters.
- `ColorSpectrum`, `ColorPickerSlider` and `ColorPicker` accessibility remains deferred: their automation peers, the names of their parts, and the value strings of the spectrum and the sliders. Trigger: The kit gains accessibility support.
- `ColorSpectrum`'s mirrored spectrum and the reversed Left and Right keys of `ColorSpectrum` and `ColorPickerSlider` in right-to-left layout remain deferred. Trigger: A consumer needs right-to-left layout.
- `ColorPicker`'s labels, tooltip texts and colour names in languages other than en-US remain deferred. Trigger: The kit gains localized resources.
- Whether WinUI shows the tooltip of a vertical `ColorPickerSlider`, whose `ToolTip` belongs to the collapsed horizontal thumb, remains unverified; the kit shows none. Trigger: A Windows capture of a `Horizontal` ColorPicker's slider taking the keyboard focus.
- `TreeView`'s touch input, dragging a row by pressing and holding it and the press feedback of a touch, remains deferred. Trigger: The kit distinguishes touch input in its controls.
- `TreeView`'s gamepad input, Gamepad A cycling a node of Multiple mode between expanding and selecting, and directional focus movement, remains deferred. Trigger: A host supplies gamepad or remote-control input.
- `TreeView`'s accessibility, its automation peers, the automation events of expanding, collapsing, selection and drops, and the drop-effect descriptions TreeViewList builds, remains deferred. Trigger: The kit gains accessibility support.
- Cancelling a `TreeView` drag with Escape remains deferred. Trigger: Inset's drag and drop can be cancelled by a key.
- `SelectorBar`'s accessibility, its automation peers and their selection pattern, remains deferred. Trigger: The kit gains accessibility support.
- `SelectorBarItem`'s tooltip for a truncated text, which the spec leaves open, remains deferred. Trigger: WinUI's control shows one.
