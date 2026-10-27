//! Desktop secondary-command presentation of WinUI TextCommandBarFlyout.

use crate::*;
use inset_embedder::{FontWeight, TargetPlatform};
use inset_foundation::{App, Handle, Listener};
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_rendering::{BoxConstraints, CrossAxisAlignment, MainAxisSize};
use inset_services::{SelectionChangedCause, TextSelectionDelegate};
use inset_widgets::*;
use std::rc::Rc;

/// Builds TextCommandBarFlyout's TextBox/PasswordBox command order and availability.
pub(super) fn build(
    app: &mut App,
    _context: BuildContext,
    editor: Handle<EditableTextState>,
    undo: Handle<UndoHistoryController>,
    password: bool,
) -> WidgetRef {
    let context = editor.context(app);
    let resources = ThemeResources::of(app, context);
    let r = &resources;
    let config = editor.widget(app);
    let read_only = config.read_only;
    let value = config.controller.value(app).clone();
    let selected = !value.selection.is_collapsed() && value.selection.is_valid();
    let pasteable = editor.clipboard_status(app).value(app) == ClipboardStatus::Pasteable;
    let history = *undo.value(app);

    let mut commands: Vec<(&str, &str, Listener)> = Vec::new();
    if !password && selected {
        if !read_only {
            commands.push((
                "Cut",
                "X",
                Listener::new(move |app| editor.cut_selection(app, SelectionChangedCause::Toolbar)),
            ));
        }
        commands.push((
            "Copy",
            "C",
            Listener::new(move |app| editor.copy_selection(app, SelectionChangedCause::Toolbar)),
        ));
    }
    if !read_only && pasteable {
        commands.push((
            "Paste",
            "V",
            Listener::new(move |app| {
                // The paste finishes on its own once the clipboard answers.
                drop(editor.paste_text(app, SelectionChangedCause::Toolbar));
            }),
        ));
    }
    if !password && !read_only {
        if history.can_undo {
            commands.push(("Undo", "Z", Listener::new(move |app| undo.undo(app))));
        }

        if history.can_redo {
            commands.push(("Redo", "Y", Listener::new(move |app| undo.redo(app))));
        }
    }
    if !value.text.is_empty() {
        commands.push((
            "Select all",
            "A",
            Listener::new(move |app| editor.select_all(app, SelectionChangedCause::Toolbar)),
        ));
    }

    let mac = app.platform().target_platform() == TargetPlatform::MacOS;
    let rows: Vec<_> = commands
        .into_iter()
        .map(|(label, key, command)| {
            let accelerator = if mac {
                if key == "Y" {
                    "⇧⌘Z".to_owned()
                } else {
                    format!("⌘{key}")
                }
            } else {
                format!("Ctrl+{key}")
            };
            Button::new(
                Row::new().main_axis_size(MainAxisSize::Min).children([
                    Expanded::new(Text::new(label)).into_widget(),
                    Padding::new(EdgeInsetsGeometry::only(24.0, 0.0, 0.0, 0.0))
                        .child(Text::new(accelerator).style(control_text_style(
                            &resources,
                            resources.caption_text_block_font_size,
                            FontWeight::NORMAL,
                            r.command_bar_flyout_app_bar_button_keyboard_text_label_foreground,
                        )))
                        .into_widget(),
                ]),
                Listener::new(move |app| {
                    command.call(app);
                    editor.hide_toolbar(app, true);
                }),
            )
            .template(menu_button)
            .into_widget()
        })
        .collect();
    if rows.is_empty() {
        return SizedBox::shrink().into_widget();
    }

    // CommandBarOverflowPresenter: desktop secondary items, three-pixel ItemsPresenter margin.
    let menu = ConstrainedBox::new(
        BoxConstraints::new()
            .min_width(136.0)
            .max_width(440.0)
            .max_height(480.0),
    )
    .child(
        ControlBorder::new(
            Brush::Acrylic(resources.acrylic_in_app_fill_color_default_brush),
            Brush::Solid(r.command_bar_flyout_border_brush),
        )
        .border_thickness_ltrb(resources.command_bar_flyout_border_theme_thickness)
        .corner_radius(resources.overlay_corner_radius[0])
        .child(
            Padding::new(EdgeInsetsGeometry::all(3.0)).child(
                SingleChildScrollView::new().child(
                    IntrinsicWidth::new().child(
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .children(rows),
                    ),
                ),
            ),
        ),
    );

    let anchor = editor.context_menu_anchors(app).primary_anchor;
    ThemeScope::with_resources(
        resources,
        CustomSingleChildLayout::new(Rc::new(DesktopTextSelectionToolbarLayoutDelegate::new(
            anchor,
        )))
        .child(TextFieldTapRegion::new(menu)),
    )
    .into_widget()
}

/// CommandBarFlyoutAppBarButtonStyle's desktop Overflow state and 83 ms brush transition.
fn menu_button(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    let r = &resources;
    let (background, foreground) = match states.common {
        CommonState::PointerOver => (
            r.command_bar_flyout_app_bar_button_background_pointer_over,
            r.command_bar_flyout_app_bar_button_foreground_pointer_over,
        ),
        CommonState::Pressed => (
            r.command_bar_flyout_app_bar_button_background_pressed,
            r.command_bar_flyout_app_bar_button_foreground_pressed,
        ),
        CommonState::Disabled => (
            r.command_bar_flyout_app_bar_button_background_disabled,
            r.command_bar_flyout_app_bar_button_foreground_disabled,
        ),
        _ => (
            r.command_bar_flyout_app_bar_button_background,
            r.command_bar_flyout_app_bar_button_foreground,
        ),
    };

    let label =
        Padding::new(EdgeInsetsGeometry::from_ltrb(12.0, 6.0, 12.0, 7.0))
            .child(Align::new().alignment(Alignment::CENTER_LEFT.into()).child(
                DefaultTextStyle::new(
                    control_text_style(
                        &resources,
                        r.control_content_theme_font_size,
                        FontWeight::NORMAL,
                        foreground,
                    ),
                    content,
                ),
            ))
            .into_widget();

    // AppBarButtonInnerBorder is inset independently of ContentRoot.
    let border = r.command_bar_flyout_app_bar_button_border_brush;
    let corner_radius = resources.control_corner_radius[0];
    let background = Positioned::fill(ColorTransition::new(
        background,
        resources.control_faster_animation_duration,
        move |_, color| {
            Padding::new(EdgeInsetsGeometry::all(2.0))
                .child(
                    ControlBorder::new(Brush::Solid(color), Brush::Solid(border))
                        .border_thickness(0.0)
                        .corner_radius(corner_radius),
                )
                .into_widget()
        },
    ));

    FocusVisual::new(
        Stack::new().children([background.into_widget(), label]),
        resources.theme,
    )
    .visible(states.focused)
    .into_widget()
}
