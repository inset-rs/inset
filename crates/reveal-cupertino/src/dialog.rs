//! Flutter counterpart: `cupertino/dialog.dart`.
//!
//! The `Semantics`, `MergeSemantics` and `sendSemanticsEvent` wrappers wait with
//! accessibility.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{Curve, Curves};
use reveal_embedder::{
    Brightness, Color, ColorFilter, ImageFilter, Offset, PointerDeviceKind, Radius, Size,
    TargetPlatform, TextAlign, TextBaseline, TextDirection, lerp_double,
};
use reveal_foundation::{App, Handle, K_IS_WEB, Listener, ValueSetter};
use reveal_gestures::{
    DragData, DragDirection, DragDownDetails, DragEndDetails, DragGestureRecognizer, DragLeaf,
    DragLeafData, DragStartBehavior, GestureBinding, GestureRecognizer, GestureRecognizerData,
    HitTestResult, OneSequenceData, OneSequenceGestureRecognizer, OneSequenceLeafData,
    PointerDownEvent, PointerEvent, PointerPanZoomStartEvent, RecognizerLeaf, RecognizerLeafData,
    VelocityEstimate, VerticalDragGestureRecognizerBase,
};
use reveal_painting::{
    AnyColor, Axis, BorderRadius, BorderRadiusGeometry, BoxDecoration, BoxFit, EdgeInsets,
    EdgeInsetsGeometry, FontWeight, HSLColor, K_DEFAULT_FONT_SIZE, TextOverflow, TextStyle,
};
use reveal_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestEntry, BoxHitTestResult,
    ContainerBoxParentData, ContainerRenderObjectData, ContainerRenderObjectMixin,
    CrossAxisAlignment, FlexParentData, HitTestBehavior, ImageFilterConfig, MainAxisAlignment,
    MainAxisSize, PaintingContext, PipelineOwner, RenderBox, RenderBoxContainerDefaultsMixin,
    RenderBoxData, RenderFlexData, RenderFlexMixin, RenderHandle, RenderMetaData, RenderObject,
    RenderObjectData,
};
use reveal_services::{HapticFeedback, MouseCursor, MouseCursorRef, SystemMouseCursors};
use reveal_widgets::{
    Action, ActivateIntent, AnimatedPadding, AnyAction, AnyFocusNode, AnyScrollController,
    BackdropFilter, BuildContext, CallbackAction, Center, ClipRSuperellipse, ColoredBox, Column,
    ConstrainedBox, Container, DecoratedBox, DefaultTextStyle, Directionality, FittedBox, Flexible,
    FocusableActionDetector, GestureRecognizerFactories, GestureRecognizerFactory,
    GestureRecognizerFactoryWithHandlers, IntoWidget, KeyRef, LayoutBuilder, LimitedBox,
    MediaQuery, MetaData, MouseRegion, MultiChildRenderObjectWidget, NotificationListener,
    Orientation, Padding, Positioned, RawGestureDetector, RenderObjectWidget, SafeArea,
    ScrollBehaviorRef, ScrollConfiguration, ScrollController, ScrollControllerLeaf,
    ScrollNotification, ScrollUpdateNotification, SingleChildScrollView, SizedBox, Stack, State,
    StateData, StatefulWidget, StatelessWidget, View, WidgetRef,
};

use crate::button::CupertinoButtonSize;
use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::constants::{
    K_CUPERTINO_BUTTON_TINTED_OPACITY_DARK, K_CUPERTINO_BUTTON_TINTED_OPACITY_LIGHT,
    k_cupertino_button_size_border_radius,
};
use crate::focus_halo::CupertinoFocusHalo;
use crate::interface_level::{CupertinoUserInterfaceLevel, CupertinoUserInterfaceLevelData};
use crate::localizations::CupertinoLocalizations;
use crate::scrollbar::CupertinoScrollbar;
use crate::theme::CupertinoTheme;

// TODO(abarth): These constants probably belong somewhere more general.

// Used XD to flutter plugin(https://github.com/AdobeXD/xd-to-flutter-plugin/)
// to derive values of TextStyle(height and letterSpacing) from
// Adobe XD template for iOS 13, which can be found in
// Apple Design Resources(https://developer.apple.com/design/resources/).
// However the values are not exactly the same as native, so eyeballing is needed.
fn cupertino_dialog_title_style() -> TextStyle {
    TextStyle::new()
        .font_family("CupertinoSystemText")
        .inherit(false)
        .font_size(17.0)
        .font_weight(FontWeight::W600)
        .height(1.3)
        .letter_spacing(-0.5)
        .text_baseline(TextBaseline::Alphabetic)
}

fn cupertino_dialog_content_style() -> TextStyle {
    TextStyle::new()
        .font_family("CupertinoSystemText")
        .inherit(false)
        .font_size(13.0)
        .font_weight(FontWeight::W400)
        .height(1.35)
        .letter_spacing(-0.2)
        .text_baseline(TextBaseline::Alphabetic)
}

fn cupertino_dialog_action_style() -> TextStyle {
    TextStyle::new()
        .font_family("CupertinoSystemText")
        .inherit(false)
        .font_size(16.8)
        .font_weight(FontWeight::W400)
        .text_baseline(TextBaseline::Alphabetic)
}

// CupertinoActionSheet-specific text styles.
fn action_sheet_action_style() -> TextStyle {
    // The fontSize and fontWeight may be adjusted when the text is rendered.
    TextStyle::new()
        .font_family("CupertinoSystemDisplay")
        .inherit(false)
        .font_size(17.0)
        .font_weight(FontWeight::W400)
        .text_baseline(TextBaseline::Alphabetic)
}

// The `color` is configured by ACTION_SHEET_CONTENT_TEXT_COLOR to be dynamic on context.
fn action_sheet_content_style() -> TextStyle {
    TextStyle::new()
        .font_family("CupertinoSystemText")
        .inherit(false)
        .font_size(13.0)
        .font_weight(FontWeight::W400)
        .text_baseline(TextBaseline::Alphabetic)
}

// Generic constants shared between Dialog and ActionSheet.
const K_CORNER_RADIUS: f64 = 14.0;
const K_DIVIDER_THICKNESS: f64 = 0.3;

// Dialog specific constants.
// iOS dialogs have a normal display width and another display width that is
// used when the device is in accessibility mode. Each of these widths are
// listed below.
const K_CUPERTINO_DIALOG_WIDTH: f64 = 270.0;
const K_ACCESSIBILITY_CUPERTINO_DIALOG_WIDTH: f64 = 310.0;
const K_DIALOG_EDGE_PADDING: f64 = 20.0;
const K_DIALOG_MIN_BUTTON_HEIGHT: f64 = 45.0;
const K_DIALOG_MIN_BUTTON_FONT_SIZE: f64 = 10.0;
// The min height for a button excluding dividers. Derived by comparing on iOS
// 17 simulators.
const K_DIALOG_ACTIONS_SECTION_MIN_HEIGHT: f64 = 67.8;

// ActionSheet specific constants.
const K_ACTION_SHEET_EDGE_PADDING: f64 = 8.0;
const K_ACTION_SHEET_CANCEL_BUTTON_PADDING: f64 = 8.0;
const K_ACTION_SHEET_CONTENT_HORIZONTAL_PADDING: f64 = 16.0;
const K_ACTION_SHEET_CONTENT_VERTICAL_PADDING: f64 = 13.5;
const K_ACTION_SHEET_ACTIONS_SECTION_MIN_HEIGHT: f64 = 84.0;
const K_ACTION_SHEET_BUTTON_HORIZONTAL_PADDING: f64 = 10.0;

// According to experimenting on the simulator, the height of action sheet
// buttons is proportional to the font size down to a minimal height.
const K_ACTION_SHEET_BUTTON_MIN_HEIGHT: f64 = 57.17;
const K_ACTION_SHEET_BUTTON_VERTICAL_PADDING_FACTOR: f64 = 0.4;
const K_ACTION_SHEET_BUTTON_VERTICAL_PADDING_BASE: f64 = 1.8;

// A translucent color that is painted on top of the blurred backdrop as the
// dialog's background color
// Extracted from https://developer.apple.com/design/resources/.
const DIALOG_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xCCF2F2F2), Color::new(0xCC2D2D2D));
const K_DIALOG_COLOR: AnyColor = DIALOG_COLOR_DYNAMIC.to_any();

// Translucent light gray that is painted on top of the blurred backdrop as the
// background color of a pressed button.
// Eyeballed from iOS 13 beta simulator.
const DIALOG_PRESSED_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFE1E1E1), Color::new(0xFF404040));
const K_DIALOG_PRESSED_COLOR: AnyColor = DIALOG_PRESSED_COLOR_DYNAMIC.to_any();

// Translucent light gray that is painted on top of the blurred backdrop as the
// background color of a pressed button.
// Eyeballed from iOS 17 simulator.
const ACTION_SHEET_PRESSED_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xCAE0E0E0), Color::new(0xC1515151));
const K_ACTION_SHEET_PRESSED_COLOR: AnyColor = ACTION_SHEET_PRESSED_COLOR_DYNAMIC.to_any();

const ACTION_SHEET_CANCEL_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFFFFFFF), Color::new(0xFF2C2C2C));
const K_ACTION_SHEET_CANCEL_COLOR: AnyColor = ACTION_SHEET_CANCEL_COLOR_DYNAMIC.to_any();

const ACTION_SHEET_CANCEL_PRESSED_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFECECEC), Color::new(0xFF494949));
const K_ACTION_SHEET_CANCEL_PRESSED_COLOR: AnyColor =
    ACTION_SHEET_CANCEL_PRESSED_COLOR_DYNAMIC.to_any();

// Translucent, very light gray that is painted on top of the blurred backdrop
// as the action sheet's background color.
// TODO(LongCatIsLooong): https://github.com/flutter/flutter/issues/39272. Use
// System Materials once we have them.
// Eyeballed from iOS 17 simulator.
const ACTION_SHEET_BACKGROUND_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xC8FCFCFC), Color::new(0xBE292929));
const K_ACTION_SHEET_BACKGROUND_COLOR: AnyColor = ACTION_SHEET_BACKGROUND_COLOR_DYNAMIC.to_any();

// The gray color used for text that appears in the title area.
// Eyeballed from iOS 17 simulator.
const ACTION_SHEET_CONTENT_TEXT_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0x851D1D1D), Color::new(0x96F1F1F1));
const K_ACTION_SHEET_CONTENT_TEXT_COLOR: AnyColor =
    ACTION_SHEET_CONTENT_TEXT_COLOR_DYNAMIC.to_any();

// Translucent gray that is painted on top of the blurred backdrop in the gap
// areas between the content section and actions section, as well as between
// buttons.
// Eyeballed from iOS 17 simulator.
const ACTION_SHEET_BUTTON_DIVIDER_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xD4C9C9C9), Color::new(0xD57D7D7D));
const K_ACTION_SHEET_BUTTON_DIVIDER_COLOR: AnyColor =
    ACTION_SHEET_BUTTON_DIVIDER_COLOR_DYNAMIC.to_any();

// The alert dialog layout policy changes depending on whether the user is using
// a "regular" font size vs a "large" font size. This is a spectrum. There are
// many "regular" font sizes and many "large" font sizes. But depending on which
// policy is currently being used, a dialog is laid out differently.
//
// Empirically, the jump from one policy to the other occurs at the following text
// scale factors:
// Largest regular scale factor:  1.3529411764705883
// Smallest large scale factor:   1.6470588235294117
//
// The following constant represents a division in text scale factor beyond which
// we want to change how the dialog is laid out.
const K_MAX_REGULAR_TEXT_SCALE_FACTOR: f64 = 1.4;

// Accessibility mode on iOS is determined by the text scale factor that the
// user has selected.
fn is_in_accessibility_mode(app: &mut App, context: BuildContext) -> bool {
    const DEFAULT_FONT_SIZE: f64 = 14.0;
    let scaled_font_size = MediaQuery::maybe_text_scaler_of(app, context)
        .map(|scaler| scaler.scale(DEFAULT_FONT_SIZE));
    scaled_font_size.is_some_and(|size| size > DEFAULT_FONT_SIZE * K_MAX_REGULAR_TEXT_SCALE_FACTOR)
}

/// An iOS-style alert dialog.
///
/// An alert dialog informs the user about situations that require
/// acknowledgment. An alert dialog has an optional title, optional content,
/// and an optional list of actions. The title is displayed above the content
/// and the actions are displayed below the content.
///
/// This dialog styles its title and content (typically a message) to match the
/// standard iOS title and message dialog text style. These default styles can
/// be overridden by explicitly defining `TextStyle`s for `Text` widgets that
/// are part of the title or content.
///
/// To display action buttons that look like standard iOS dialog buttons,
/// provide [`CupertinoDialogAction`]s for the [`actions`](Self::actions) given to this dialog.
///
/// Typically passed as the child widget to `showCupertinoDialog`, which displays the
/// dialog.
///
/// See also:
///
///  * [`CupertinoPopupSurface`], which is a generic iOS-style popup surface that
///    holds arbitrary content to create custom popups.
///  * [`CupertinoDialogAction`], which is an iOS-style dialog button.
///  * <https://developer.apple.com/design/human-interface-guidelines/alerts/>
#[derive(Clone)]
pub struct CupertinoAlertDialog {
    pub key: Option<KeyRef>,
    /// The (optional) title of the dialog is displayed in a large font at the top
    /// of the dialog.
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,
    /// The (optional) content of the dialog is displayed in the center of the
    /// dialog in a lighter font.
    ///
    /// Typically a `Text` widget.
    pub content: Option<WidgetRef>,
    /// The (optional) set of actions that are displayed at the bottom of the
    /// dialog.
    ///
    /// Typically this is a list of [`CupertinoDialogAction`] widgets.
    pub actions: Vec<WidgetRef>,
    /// A scroll controller that can be used to control the scrolling of the
    /// [`content`](Self::content) in the dialog.
    ///
    /// Defaults to null, which means the [`CupertinoDialogAction`] will create a
    /// scroll controller internally.
    ///
    /// See also:
    ///
    ///  * [`action_scroll_controller`](Self::action_scroll_controller), which can be used for
    ///    controlling the actions section when there are many actions.
    pub scroll_controller: Option<AnyScrollController>,
    /// A scroll controller that can be used to control the scrolling of the
    /// actions in the dialog.
    ///
    /// Defaults to null, which means the [`CupertinoDialogAction`] will create an
    /// action scroll controller internally.
    ///
    /// See also:
    ///
    ///  * [`scroll_controller`](Self::scroll_controller), which can be used for controlling the
    ///    [`content`](Self::content) section when it is long.
    pub action_scroll_controller: Option<AnyScrollController>,
    /// The duration of the animation of the dialog's padding when the view insets change.
    pub inset_animation_duration: Duration,
    /// The curve of the animation of the dialog's padding when the view insets change.
    pub inset_animation_curve: Rc<dyn Curve>,
}

impl CupertinoAlertDialog {
    /// Creates an iOS-style alert dialog.
    pub fn new() -> CupertinoAlertDialog {
        CupertinoAlertDialog {
            key: None,
            title: None,
            content: None,
            actions: Vec::new(),
            scroll_controller: None,
            action_scroll_controller: None,
            inset_animation_duration: Duration::from_millis(100),
            inset_animation_curve: Curves::decelerate(),
        }
    }

    /// Dart `CupertinoAlertDialog(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoAlertDialog {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoAlertDialog(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> CupertinoAlertDialog {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `CupertinoAlertDialog(content:)`.
    pub fn content<K>(mut self, content: impl IntoWidget<K>) -> CupertinoAlertDialog {
        self.content = Some(content.into_widget());
        self
    }

    /// Dart `CupertinoAlertDialog(actions:)`.
    pub fn actions(mut self, actions: impl IntoIterator<Item = WidgetRef>) -> CupertinoAlertDialog {
        self.actions = actions.into_iter().collect();
        self
    }

    /// Dart `CupertinoAlertDialog(scrollController:)`.
    pub fn scroll_controller(
        mut self,
        scroll_controller: AnyScrollController,
    ) -> CupertinoAlertDialog {
        self.scroll_controller = Some(scroll_controller);
        self
    }

    /// Dart `CupertinoAlertDialog(actionScrollController:)`.
    pub fn action_scroll_controller(
        mut self,
        action_scroll_controller: AnyScrollController,
    ) -> CupertinoAlertDialog {
        self.action_scroll_controller = Some(action_scroll_controller);
        self
    }

    /// Dart `CupertinoAlertDialog(insetAnimationDuration:)`.
    pub fn inset_animation_duration(
        mut self,
        inset_animation_duration: Duration,
    ) -> CupertinoAlertDialog {
        self.inset_animation_duration = inset_animation_duration;
        self
    }

    /// Dart `CupertinoAlertDialog(insetAnimationCurve:)`.
    pub fn inset_animation_curve(
        mut self,
        inset_animation_curve: Rc<dyn Curve>,
    ) -> CupertinoAlertDialog {
        self.inset_animation_curve = inset_animation_curve;
        self
    }
}

impl Default for CupertinoAlertDialog {
    fn default() -> CupertinoAlertDialog {
        CupertinoAlertDialog::new()
    }
}

impl Debug for CupertinoAlertDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoAlertDialog")
            .field("title", &self.title)
            .field("content", &self.content)
            .field("actions", &self.actions.len())
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoAlertDialog {
    type State = CupertinoAlertDialogState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoAlertDialogState {
        CupertinoAlertDialogState {
            state: StateData::new(),
            pressed_index: None,
            backup_scroll_controller: None,
            backup_action_scroll_controller: None,
        }
    }
}

/// Dart's `_CupertinoAlertDialogState`.
pub struct CupertinoAlertDialogState {
    state: StateData<CupertinoAlertDialog>,
    /// The index of the action button that the user is holding on.
    ///
    /// `None` if the user is not holding on any buttons.
    pressed_index: Option<i32>,
    backup_scroll_controller: Option<AnyScrollController>,
    backup_action_scroll_controller: Option<AnyScrollController>,
}

impl CupertinoAlertDialogState {
    fn effective_scroll_controller(self: Handle<Self>, app: &mut App) -> AnyScrollController {
        if let Some(controller) = self.widget(app).scroll_controller {
            return controller;
        }
        if let Some(controller) = app.get(self).backup_scroll_controller {
            return controller;
        }
        let controller = ScrollController::default(app).as_controller();
        app.get_mut(self).backup_scroll_controller = Some(controller);
        controller
    }

    fn effective_action_scroll_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> AnyScrollController {
        if let Some(controller) = self.widget(app).action_scroll_controller {
            return controller;
        }
        if let Some(controller) = app.get(self).backup_action_scroll_controller {
            return controller;
        }
        let controller = ScrollController::default(app).as_controller();
        app.get_mut(self).backup_action_scroll_controller = Some(controller);
        controller
    }

    fn build_content(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> Option<WidgetRef> {
        let widget = self.widget(app).clone();
        let has_content = widget.title.is_some() || widget.content.is_some();
        if !has_content {
            return None;
        }

        const DEFAULT_FONT_SIZE: f64 = 14.0;
        let effective_text_scale_factor =
            MediaQuery::text_scaler_of(app, context).scale(DEFAULT_FONT_SIZE) / DEFAULT_FONT_SIZE;

        let label = CupertinoDynamicColor::resolve(&CupertinoColors::LABEL, app, context);
        let scroll_controller = self.effective_scroll_controller(app);
        let mut section = CupertinoAlertContentSection::new(scroll_controller)
            .title_padding(EdgeInsets::only(
                K_DIALOG_EDGE_PADDING,
                K_DIALOG_EDGE_PADDING * effective_text_scale_factor,
                K_DIALOG_EDGE_PADDING,
                if widget.content.is_none() {
                    K_DIALOG_EDGE_PADDING
                } else {
                    1.0
                },
            ))
            .message_padding(EdgeInsets::only(
                K_DIALOG_EDGE_PADDING,
                if widget.title.is_none() {
                    K_DIALOG_EDGE_PADDING
                } else {
                    1.0
                },
                K_DIALOG_EDGE_PADDING,
                K_DIALOG_EDGE_PADDING * effective_text_scale_factor,
            ))
            .title_text_style(
                cupertino_dialog_title_style()
                    .copy_with()
                    .color(label.clone()),
            )
            .message_text_style(cupertino_dialog_content_style().copy_with().color(label));
        if let Some(title) = widget.title {
            section = section.title(title);
        }
        if let Some(content) = widget.content {
            section = section.message(content);
        }

        let color = CupertinoDynamicColor::resolve(&K_DIALOG_COLOR, app, context);
        Some(ColoredBox::new(color).child(section).into_widget())
    }

    fn on_pressed_update(self: Handle<Self>, app: &mut App, action_index: i32, is_pressed: bool) {
        if is_pressed {
            self.set_state(app, |state| {
                state.pressed_index = Some(action_index);
            });
        } else if app.get(self).pressed_index == Some(action_index) {
            self.set_state(app, |state| {
                state.pressed_index = None;
            });
        }
    }

    fn build_actions(self: Handle<Self>, app: &mut App) -> Option<WidgetRef> {
        let actions = self.widget(app).actions.clone();
        if actions.is_empty() {
            return None;
        }
        let scroll_controller = self.effective_action_scroll_controller(app);
        let pressed_index = app.get(self).pressed_index;
        Some(
            CupertinoAlertActionSection::new(
                actions,
                Rc::new(move |app: &mut App, action_index, state| {
                    self.on_pressed_update(app, action_index, state);
                }),
                pressed_index,
                scroll_controller,
            )
            .into_widget(),
        )
    }

    fn build_body(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let background_color = CupertinoDynamicColor::resolve(&K_DIALOG_COLOR, app, context);
        let divider_color =
            CupertinoDynamicColor::resolve(&CupertinoColors::SEPARATOR, app, context);
        // Remove view padding here because the `Scrollbar` widget uses the view
        // padding as padding, which is unwanted.
        // https://github.com/flutter/flutter/issues/150544
        MediaQuery::remove_padding(
            app,
            context,
            true,
            true,
            true,
            true,
            LayoutBuilder::new(move |app, context, constraints| {
                let content_section = self.build_content(app, context);
                let actions_section = self.build_actions(app);
                let Some(actions_section) = actions_section else {
                    return content_section.unwrap_or_else(|| {
                        LimitedBox::new()
                            .max_width(0.0)
                            .child(SizedBox::new().width(f64::INFINITY).height(0.0))
                            .into_widget()
                    });
                };
                let scrolled_actions_section =
                    OverscrollBackground::new(background_color.clone(), actions_section);
                let Some(content_section) = content_section else {
                    return scrolled_actions_section.into_widget();
                };
                // It is observed on the simulator that the minimal height varies
                // depending on whether the device is in accessibility mode.
                let actions_min_height = if is_in_accessibility_mode(app, context) {
                    constraints.max_height / 2.0 + K_DIVIDER_THICKNESS
                } else {
                    K_DIALOG_ACTIONS_SECTION_MIN_HEIGHT + K_DIVIDER_THICKNESS
                };
                PriorityColumn::new(
                    content_section,
                    Column::new().children([
                        SizedBox::new()
                            .width(f64::INFINITY)
                            .child(Divider::new(
                                divider_color.clone(),
                                background_color.clone(),
                                false,
                            ))
                            .into_widget(),
                        Flexible::new(scrolled_actions_section).into_widget(),
                    ]),
                    actions_min_height,
                )
                .into_widget()
            })
            .into_widget(),
        )
        .into_widget()
    }
}

impl State for CupertinoAlertDialogState {
    type Widget = CupertinoAlertDialog;
    reveal_widgets::state_accessors!();

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).backup_scroll_controller {
            ScrollController::dispose(controller, app);
        }
        if let Some(controller) = app.get(self).backup_action_scroll_controller {
            ScrollController::dispose(controller, app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // `Semantics(label: localizations.alertDialogLabel)` waits with accessibility; the read
        // keeps the dependency on `Localizations` Dart registers here.
        <dyn CupertinoLocalizations>::of(app, context).alert_dialog_label();
        let is_in_accessibility_mode = is_in_accessibility_mode(app, context);
        let behavior = ScrollConfiguration::of(app, context)
            .copy_with()
            .scrollbars(false);
        CupertinoUserInterfaceLevel::new(
            CupertinoUserInterfaceLevelData::Elevated,
            MediaQuery::with_clamped_text_scaling(
                None,
                // iOS does not shrink dialog content below a 1.0 scale factor
                1.0,
                f64::INFINITY,
                ScrollConfiguration::new(
                    // A CupertinoScrollbar is built-in below.
                    ScrollBehaviorRef::new(behavior),
                    LayoutBuilder::new(move |app, context, _constraints| {
                        let padding = MediaQuery::view_insets_of(app, context)
                            + EdgeInsets::symmetric(24.0, 40.0);
                        let body = self.build_body(app, context);
                        let widget = self.widget(app).clone();
                        AnimatedPadding::new(padding.into(), widget.inset_animation_duration)
                            .curve(widget.inset_animation_curve)
                            .child(
                                MediaQuery::remove_view_insets(
                                    app,
                                    context,
                                    true,
                                    true,
                                    true,
                                    true,
                                    Center::new().child(
                                        Padding::new(EdgeInsetsGeometry::symmetric(
                                            K_DIALOG_EDGE_PADDING,
                                            0.0,
                                        ))
                                        .child(
                                            SizedBox::new()
                                                .width(if is_in_accessibility_mode {
                                                    K_ACCESSIBILITY_CUPERTINO_DIALOG_WIDTH
                                                } else {
                                                    K_CUPERTINO_DIALOG_WIDTH
                                                })
                                                .child(
                                                    ActionSheetGestureDetector::new().child(
                                                        CupertinoPopupSurface::new(body)
                                                            .is_surface_painted(false),
                                                    ),
                                                ),
                                        ),
                                    ),
                                )
                                .into_widget(),
                            )
                            .into_widget()
                    })
                    .into_widget(),
                ),
            ),
        )
        .into_widget()
    }
}

thread_local! {
    static DEBUG_IS_VIBRANCE_PAINTED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Whether or not the area beneath a [`CupertinoPopupSurface`] should be saturated with a
/// `ColorFilter`.
///
/// The appearance of the `ColorFilter` is determined by the `Brightness` value obtained from
/// the ambient [`CupertinoTheme`].
///
/// The vibrance is always painted if asserts are disabled.
///
/// Defaults to true.
pub fn debug_is_vibrance_painted() -> bool {
    DEBUG_IS_VIBRANCE_PAINTED.get()
}

/// Sets [`debug_is_vibrance_painted`].
pub fn set_debug_is_vibrance_painted(value: bool) {
    DEBUG_IS_VIBRANCE_PAINTED.set(value);
}

/// An iOS-style component for creating modal overlays like dialogs and action
/// sheets.
///
/// By default, [`CupertinoPopupSurface`] generates a rounded rectangle surface
/// that applies two effects to the background content:
///
///   1. Background filter: Saturates and then blurs content behind the surface.
///   2. Overlay color: Covers the filtered background with a transparent
///      surface color. The color adapts to the [`CupertinoTheme`]'s brightness:
///      light gray when the ambient [`CupertinoTheme`] brightness is
///      [`Brightness::Light`], and dark gray when [`Brightness::Dark`].
///
/// The blur strength can be changed by setting [`blur_sigma`](Self::blur_sigma) to a positive
/// value, or removed by setting the [`blur_sigma`](Self::blur_sigma) to 0.
///
/// The saturation effect can be removed for debugging by setting
/// [`debug_is_vibrance_painted`] to false.
///
/// The surface color can be disabled by setting
/// [`is_surface_painted`](Self::is_surface_painted) to false, which is useful for more
/// complicated layouts, such as rendering divider gaps in [`CupertinoAlertDialog`] or
/// rendering custom surface colors.
///
/// See also:
///
///  * [`CupertinoAlertDialog`], which is a dialog with a title, content, and
///    actions.
///  * <https://developer.apple.com/design/human-interface-guidelines/alerts/>
#[derive(Clone, Debug)]
pub struct CupertinoPopupSurface {
    pub key: Option<KeyRef>,
    /// The strength of the gaussian blur applied to the area beneath this
    /// surface.
    ///
    /// Defaults to [`CupertinoPopupSurface::DEFAULT_BLUR_SIGMA`]. Setting
    /// [`blur_sigma`](Self::blur_sigma) to 0 will remove the blur filter.
    pub blur_sigma: f64,
    /// Whether or not to paint a translucent white on top of this surface's
    /// blurred background. [`is_surface_painted`](Self::is_surface_painted) should be true
    /// for a typical popup that contains content without any dividers. A popup that requires
    /// dividers should set [`is_surface_painted`](Self::is_surface_painted) to false and then
    /// paint its own surface area.
    ///
    /// Some popups, like iOS's volume control popup, choose to render a blurred
    /// area without any white paint covering it. To achieve this effect,
    /// [`is_surface_painted`](Self::is_surface_painted) should be set to false.
    ///
    /// Defaults to true.
    pub is_surface_painted: bool,
    /// The widget below this widget in the tree.
    // Because [CupertinoPopupSurface] is composed of proxy boxes, which mimic
    // the size of their child, a [child] is required to ensure that this surface
    // has a size.
    pub child: WidgetRef,
}

impl CupertinoPopupSurface {
    /// The default strength of the blur applied to widgets underlying a
    /// [`CupertinoPopupSurface`].
    ///
    /// Eyeballed from the iOS 17 simulator.
    pub const DEFAULT_BLUR_SIGMA: f64 = 30.0;

    /// The default corner radius of a [`CupertinoPopupSurface`].
    fn clipper() -> BorderRadius {
        BorderRadius::circular(13.0)
    }

    // The [ColorFilter] matrix used to saturate widgets underlying a
    // [CupertinoPopupSurface] when the ambient [CupertinoThemeData::brightness] is
    // [Brightness::Light].
    //
    // To derive this matrix, the saturation matrix was taken from
    // https://docs.rainmeter.net/tips/colormatrix-guide/ and was tweaked to
    // resemble the iOS 17 simulator.
    //
    // The matrix can be derived from the following function:
    // fn light_saturation_matrix() -> [f64; 20] {
    //    const LIGHT_LUM_R: f64 = 0.26;
    //    const LIGHT_LUM_G: f64 = 0.4;
    //    const LIGHT_LUM_B: f64 = 0.17;
    //    const SATURATION: f64 = 2.0;
    //    const SR: f64 = (1.0 - SATURATION) * LIGHT_LUM_R;
    //    const SG: f64 = (1.0 - SATURATION) * LIGHT_LUM_G;
    //    const SB: f64 = (1.0 - SATURATION) * LIGHT_LUM_B;
    //    [
    //      SR + SATURATION, SG, SB, 0.0, 0.0,
    //      SR, SG + SATURATION, SB, 0.0, 0.0,
    //      SR, SG, SB + SATURATION, 0.0, 0.0,
    //      0.0, 0.0, 0.0, 1.0, 0.0,
    //    ]
    //  }
    const LIGHT_SATURATION_MATRIX: [f64; 20] = [
        1.74, -0.40, -0.17, 0.00, 0.00, //
        -0.26, 1.60, -0.17, 0.00, 0.00, //
        -0.26, -0.40, 1.83, 0.00, 0.00, //
        0.00, 0.00, 0.00, 1.00, 0.00,
    ];

    // The [ColorFilter] matrix used to saturate widgets underlying a
    // [CupertinoPopupSurface] when the ambient [CupertinoThemeData::brightness] is
    // [Brightness::Dark].
    //
    // To derive this matrix, the saturation matrix was taken from
    // https://docs.rainmeter.net/tips/colormatrix-guide/ and was tweaked to
    // resemble the iOS 17 simulator.
    //
    // The matrix can be derived from the following function:
    // fn dark_saturation_matrix() -> [f64; 20] {
    //    const ADDITIVE: f64 = 0.3;
    //    const DARK_LUM_R: f64 = 0.45;
    //    const DARK_LUM_G: f64 = 0.8;
    //    const DARK_LUM_B: f64 = 0.16;
    //    const SATURATION: f64 = 1.7;
    //    const SR: f64 = (1.0 - SATURATION) * DARK_LUM_R;
    //    const SG: f64 = (1.0 - SATURATION) * DARK_LUM_G;
    //    const SB: f64 = (1.0 - SATURATION) * DARK_LUM_B;
    //    [
    //      SR + SATURATION, SG, SB, 0.0, ADDITIVE,
    //      SR, SG + SATURATION, SB, 0.0, ADDITIVE,
    //      SR, SG, SB + SATURATION, 0.0, ADDITIVE,
    //      0.0, 0.0, 0.0, 1.0, 0.0,
    //    ]
    //  }
    const DARK_SATURATION_MATRIX: [f64; 20] = [
        1.39, -0.56, -0.11, 0.00, 0.30, //
        -0.32, 1.14, -0.11, 0.00, 0.30, //
        -0.32, -0.56, 1.59, 0.00, 0.30, //
        0.00, 0.00, 0.00, 1.00, 0.00,
    ];

    /// Creates an iOS-style rounded rectangle popup surface.
    pub fn new<K>(child: impl IntoWidget<K>) -> CupertinoPopupSurface {
        CupertinoPopupSurface {
            key: None,
            blur_sigma: CupertinoPopupSurface::DEFAULT_BLUR_SIGMA,
            is_surface_painted: true,
            child: child.into_widget(),
        }
    }

    /// Dart `CupertinoPopupSurface(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoPopupSurface {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoPopupSurface(blurSigma:)`.
    pub fn blur_sigma(mut self, blur_sigma: f64) -> CupertinoPopupSurface {
        debug_assert!(
            blur_sigma >= 0.0,
            "CupertinoPopupSurface requires a non-negative blur sigma."
        );
        self.blur_sigma = blur_sigma;
        self
    }

    /// Dart `CupertinoPopupSurface(isSurfacePainted:)`.
    pub fn is_surface_painted(mut self, is_surface_painted: bool) -> CupertinoPopupSurface {
        self.is_surface_painted = is_surface_painted;
        self
    }

    // TODO(davidhicks980): Set `bounded` to true on ImageFilterConfig::blur after
    // https://github.com/flutter/flutter/issues/182066 is resolved.
    fn build_filter(&self, brightness: Option<Brightness>) -> Option<ImageFilterConfig> {
        let mut is_vibrance_painted = true;
        if cfg!(debug_assertions) {
            is_vibrance_painted = debug_is_vibrance_painted();
        }
        let blur = || {
            ImageFilterConfig::blur()
                .sigma_x(self.blur_sigma)
                .sigma_y(self.blur_sigma)
        };
        if !is_vibrance_painted {
            if self.blur_sigma == 0.0 {
                return None;
            }
            return Some(blur());
        }

        let color_filter =
            ImageFilterConfig::new(ImageFilter::Color(ColorFilter::Matrix(match brightness {
                Some(Brightness::Dark) => CupertinoPopupSurface::DARK_SATURATION_MATRIX,
                Some(Brightness::Light) | None => CupertinoPopupSurface::LIGHT_SATURATION_MATRIX,
            })));

        if self.blur_sigma == 0.0 {
            return Some(color_filter);
        }

        Some(ImageFilterConfig::compose(blur(), color_filter))
    }
}

impl StatelessWidget for CupertinoPopupSurface {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let filter = self.build_filter(CupertinoTheme::maybe_brightness_of(app, context));
        let mut contents = self.child.clone();

        if self.is_surface_painted {
            let color = CupertinoDynamicColor::resolve(&K_DIALOG_COLOR, app, context);
            contents = ColoredBox::new(color).child(contents).into_widget();
        }

        let border_radius: BorderRadiusGeometry = CupertinoPopupSurface::clipper().into();
        match filter {
            Some(filter) => ClipRSuperellipse::new()
                .border_radius(border_radius)
                .child(BackdropFilter::new(filter).child(contents))
                .into_widget(),
            None => ClipRSuperellipse::new()
                .border_radius(border_radius)
                .child(contents)
                .into_widget(),
        }
    }
}

/// Dart's `typedef _HitTester = HitTestResult Function(Offset location)`.
type HitTester = Rc<dyn Fn(&mut App, Offset) -> HitTestResult>;

/// Dart's `DragGestureRecognizer` default `allowedButtonsFilter`, which
/// [`SlidingTapGestureRecognizer`] inherits.
fn primary_button_only(buttons: i64) -> bool {
    buttons == reveal_gestures::K_PRIMARY_BUTTON
}

// Recognizes taps with possible sliding during the tap.
//
// This recognizer only tracks one pointer at a time (called the primary
// pointer), and other pointers added while the primary pointer is alive are
// ignored and can not be used by other gestures either. After the primary
// pointer ends, the pointer added next becomes the new primary pointer (which
// starts a new gesture sequence).
//
// This recognizer only allows `K_PRIMARY_MOUSE_BUTTON`.
struct SlidingTapGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    drag: DragData,
    /// Called whenever the primary pointer moves regardless of whether drag has
    /// started.
    ///
    /// The parameter is the global position of the primary pointer.
    ///
    /// This is similar to `on_update`, but allows the caller to track the primary
    /// pointer's location before the drag starts, which is useful to enhance
    /// responsiveness.
    on_responsive_update: Option<ValueSetter<Offset>>,
    /// Called whenever the primary pointer is lifted regardless of whether drag
    /// has started.
    ///
    /// The parameter is the global position of the primary pointer.
    ///
    /// This is similar to `on_end`, but allows know the primary pointer's final
    /// location even if the drag never started, which is useful to enhance
    /// responsiveness.
    on_responsive_end: Option<ValueSetter<Offset>>,
    primary_pointer: Option<i64>,
}

impl SlidingTapGestureRecognizer {
    fn new(app: &mut App) -> Handle<SlidingTapGestureRecognizer> {
        let mut recognizer = GestureRecognizerData::new();
        recognizer.set_allowed_buttons_filter(Rc::new(primary_button_only));
        let this = app.create(SlidingTapGestureRecognizer {
            recognizer,
            one_sequence: OneSequenceData::new(),
            drag: DragData::new(),
            on_responsive_update: None,
            on_responsive_end: None,
            primary_pointer: None,
        });
        this.set_drag_start_behavior(app, DragStartBehavior::Down);
        this
    }

    fn set_on_responsive_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<ValueSetter<Offset>>,
    ) {
        app.get_mut(self).on_responsive_update = callback;
    }

    fn set_on_responsive_end(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<ValueSetter<Offset>>,
    ) {
        app.get_mut(self).on_responsive_end = callback;
    }
}

impl RecognizerLeafData for SlidingTapGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }

    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
}

impl OneSequenceLeafData for SlidingTapGestureRecognizer {
    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }

    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl DragLeafData for SlidingTapGestureRecognizer {
    fn drag(&self) -> &DragData {
        &self.drag
    }

    fn drag_mut(&mut self) -> &mut DragData {
        &mut self.drag
    }
}

impl RecognizerLeaf for SlidingTapGestureRecognizer {
    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, _event: &PointerDownEvent) {
        reveal_gestures::OneSequenceGestureRecognizer::handle_non_allowed_pointer(self, app);
    }

    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        DragGestureRecognizer::is_pointer_allowed(self, app, event)
    }

    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        app.get_mut(self)
            .primary_pointer
            .get_or_insert(event.pointer);
        DragGestureRecognizer::add_allowed_pointer(self, app, event);
    }

    fn add_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        app: &mut App,
        event: PointerPanZoomStartEvent,
    ) {
        DragGestureRecognizer::add_allowed_pointer_pan_zoom(self, app, event);
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        if Some(event.pointer()) == app.get(self).primary_pointer {
            if let PointerEvent::Move(_) = &event
                && let Some(callback) = app.get(self).on_responsive_update.clone()
            {
                callback(app, event.position());
            }
            // Sliding tap needs to handle 'up' events differently compared to typical
            // drag gestures. If there's another gesture recognizer (like scrolling)
            // competing and the pointer hasn't moved beyond the tolerance limit
            // (slop), this gesture must still be accepted.
            //
            // Simply calling `accept()` here to handle this won't work because it
            // would break backward compatibility with legacy buttons (see
            // https://github.com/flutter/flutter/issues/150980 for more details).
            // Legacy buttons recognize taps using `GestureDetector.onTap`, which
            // neither accepts nor rejects for short taps. Instead, they wait for the
            // default resolution as the last contender in the gesture arena.
            //
            // Therefore, this gesture should also follow the same strategy of not
            // immediately accepting or rejecting. This allows tap gestures to take
            // precedence for being inner, while sliding taps can take precedence over
            // scroll gestures when the latter give up.
            if let PointerEvent::Up(_) = &event {
                let primary_pointer = app.get(self).primary_pointer.expect("checked above");
                OneSequenceGestureRecognizer::stop_tracking_pointer(self, app, primary_pointer);
                if let Some(callback) = app.get(self).on_responsive_end.clone() {
                    callback(app, event.position());
                }
                app.get_mut(self).primary_pointer = None;
                // Do not call `DragGestureRecognizer::handle_event`, which gives up the
                // pointer and thus rejects the gesture.
                return;
            }
            if let PointerEvent::Cancel(_) = &event {
                app.get_mut(self).primary_pointer = None;
            }
        }
        DragGestureRecognizer::handle_event(self, app, event);
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        DragGestureRecognizer::accept_gesture(self, app, pointer);
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        if Some(pointer) == app.get(self).primary_pointer {
            app.get_mut(self).primary_pointer = None;
        }
        DragGestureRecognizer::reject_gesture(self, app, pointer);
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        DragGestureRecognizer::did_stop_tracking_last_pointer(self, app, pointer);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        DragGestureRecognizer::dispose(self, app);
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "tap slide"
    }
}

impl DragLeaf for SlidingTapGestureRecognizer {
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        VerticalDragGestureRecognizerBase::is_fling_gesture(self, app, estimate, kind)
    }

    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        VerticalDragGestureRecognizerBase::consider_fling(self, app, estimate, kind)
    }

    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool {
        VerticalDragGestureRecognizerBase::has_sufficient_global_distance_to_accept(
            self,
            app,
            pointer_device_kind,
            device_touch_slop,
        )
    }

    fn get_delta_for_details(self: Handle<Self>, app: &App, delta: Offset) -> Offset {
        VerticalDragGestureRecognizerBase::get_delta_for_details(self, app, delta)
    }

    fn get_primary_value_from_offset(self: Handle<Self>, app: &App, value: Offset) -> Option<f64> {
        VerticalDragGestureRecognizerBase::get_primary_value_from_offset(self, app, value)
    }

    fn get_primary_drag_axis(self: Handle<Self>, app: &App) -> Option<DragDirection> {
        VerticalDragGestureRecognizerBase::get_primary_drag_axis(self, app)
    }
}

// A region (typically a button) that can receive entering, exiting, and
// updating events of a "sliding tap" gesture.
//
// Some Cupertino widgets, such as action sheets or dialogs, allow the user to
// select buttons using "sliding taps", where the user can drag around after
// pressing on the screen, and whichever button the drag ends in is selected.
//
// This class is used to define the regions that sliding taps recognize. A
// `SlideTarget` must be provided to a `MetaData` widget as `meta_data` (an
// `Rc<dyn SlideTarget>` inside the `Rc<dyn Any>`, see `slide_target_meta_data`), and is
// typically implemented by a widget state class. When an eligible dragging gesture
// enters, leaves, or ends this `MetaData` widget, corresponding methods of this
// class will be called.
//
// Multiple `SlideTarget`s might be nested.
// `TargetSelectionGestureRecognizer` uses a simple algorithm that only
// compares if the inner-most slide target has changed (which suffices our use
// case). Semantically, this means that all outer targets will be treated as
// having the identical area as the inner-most one, i.e. when the pointer enters
// or leaves a slide target, the corresponding method will be called on all
// targets that nest it.
//
// A state's `Handle` implements `SlideTarget` and the state hands out one `Rc<dyn SlideTarget>`
// over that handle for its lifetime: the recognizer compares targets by identity, as Dart's
// `identical` does on the state object.
trait SlideTarget {
    // A pointer has entered this region.
    //
    // This includes:
    //
    //  * The pointer has moved into this region from outside.
    //  * The point has contacted the screen in this region. In this case, this
    //    method is called as soon as the pointer down event occurs regardless of
    //    whether the gesture wins the arena immediately.
    //
    // The `from_pointer_down` should be true if this callback is triggered by a
    // PointerDownEvent, i.e. the second case from the list above.
    //
    // The return value of this method is used as the `inner_enabled` for the next
    // target, while `inner_enabled` of the innermost target is true.
    fn did_enter(&self, app: &mut App, from_pointer_down: bool, inner_enabled: bool) -> bool;

    // A pointer has exited this region.
    //
    // This includes:
    //  * The pointer has moved out of this region.
    //  * The pointer is no longer in contact with the screen.
    //  * The pointer is canceled.
    //  * The gesture loses the arena.
    //  * The gesture ends. In this case, this method is called immediately
    //    before `did_confirm`.
    fn did_leave(&self, app: &mut App);

    // The drag gesture is completed in this region.
    //
    // This method is called immediately after a `did_leave`.
    fn did_confirm(&self, app: &mut App);
}

/// The `meta_data` of a slide target's `MetaData` widget: the state's `Rc<dyn SlideTarget>`,
/// minted in its `init_state`, boxed as `Any` for the recognizer to downcast.
fn slide_target_meta_data(target: &Option<Rc<dyn SlideTarget>>) -> Rc<dyn Any> {
    Rc::new(Rc::clone(target.as_ref().expect("init_state has run")))
}

/// Dart's `identical` on the inner-most targets of two nests.
fn identical_targets(a: Option<&Rc<dyn SlideTarget>>, b: Option<&Rc<dyn SlideTarget>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

// Recognizes sliding taps and thereupon interacts with `SlideTarget`s.
//
// TODO(dkwingsmt): It should recompute hit testing when the app is updated,
// or better, share code with `MouseTracker`.
// https://github.com/flutter/flutter/issues/155266
struct TargetSelectionGestureRecognizer {
    recognizer: GestureRecognizerData,
    /// Dart's class extends `GestureRecognizer` directly and never tracks a pointer; the bag
    /// stays empty.
    one_sequence: OneSequenceData,
    hit_test: HitTester,
    current_targets: Vec<Rc<dyn SlideTarget>>,
    sliding_tap: Handle<SlidingTapGestureRecognizer>,
}

impl TargetSelectionGestureRecognizer {
    fn new(app: &mut App, hit_test: HitTester) -> Handle<TargetSelectionGestureRecognizer> {
        let sliding_tap = SlidingTapGestureRecognizer::new(app);
        let this = app.create(TargetSelectionGestureRecognizer {
            recognizer: GestureRecognizerData::new(),
            one_sequence: OneSequenceData::new(),
            hit_test,
            current_targets: Vec::new(),
            sliding_tap,
        });
        sliding_tap.set_on_down(
            app,
            Some(Rc::new(move |app: &mut App, details: DragDownDetails| {
                this.on_down(app, details);
            })),
        );
        sliding_tap.set_on_responsive_update(
            app,
            Some(Rc::new(move |app: &mut App, position: Offset| {
                this.on_update(app, position);
            })),
        );
        sliding_tap.set_on_responsive_end(
            app,
            Some(Rc::new(move |app: &mut App, position: Offset| {
                this.on_end(app, position);
            })),
        );
        sliding_tap.set_on_cancel(app, Some(Listener::new(move |app| this.on_cancel(app))));
        this
    }

    // Collect the `SlideTarget`s that are currently hit by the pointer, check
    // whether the current target have changed, and invoke their methods if
    // necessary.
    //
    // The `from_pointer_down` should be true if this update is triggered by a
    // PointerDownEvent.
    fn update_drag(
        self: Handle<Self>,
        app: &mut App,
        pointer_position: Offset,
        from_pointer_down: bool,
    ) {
        let hit_test = Rc::clone(&app.get(self).hit_test);
        let result = hit_test(app, pointer_position);

        // A slide target might nest other targets, therefore multiple targets might
        // be found.
        let mut found_targets = Vec::new();
        for entry in result.path() {
            let Some(box_entry) = (entry.target() as &dyn Any).downcast_ref::<BoxHitTestEntry>()
            else {
                continue;
            };
            let Some(target) = box_entry
                .target()
                .as_object()
                .downcast::<RenderMetaData>(app)
            else {
                continue;
            };
            if let Some(meta_data) = target.meta_data(app)
                && let Some(slide_target) = meta_data.downcast_ref::<Rc<dyn SlideTarget>>()
            {
                found_targets.push(Rc::clone(slide_target));
            }
        }

        // Compare whether the active target has changed by simply comparing the
        // first (inner-most) avatar of the nest, ignoring the cases where
        // current_targets intersect with found_targets (see SlideTarget's
        // document for more explanation).
        if !identical_targets(app.get(self).current_targets.first(), found_targets.first()) {
            for target in app.get(self).current_targets.clone() {
                target.did_leave(app);
            }
            app.get_mut(self).current_targets = found_targets;
            let mut enabled = true;
            for target in app.get(self).current_targets.clone() {
                enabled = target.did_enter(app, from_pointer_down, enabled);
            }
        }
    }

    fn on_down(self: Handle<Self>, app: &mut App, details: DragDownDetails) {
        self.update_drag(app, details.global_position, true);
    }

    fn on_update(self: Handle<Self>, app: &mut App, global_position: Offset) {
        self.update_drag(app, global_position, false);
    }

    fn on_end(self: Handle<Self>, app: &mut App, global_position: Offset) {
        self.update_drag(app, global_position, false);
        for target in app.get(self).current_targets.clone() {
            target.did_confirm(app);
        }
        app.get_mut(self).current_targets.clear();
    }

    fn on_cancel(self: Handle<Self>, app: &mut App) {
        for target in app.get(self).current_targets.clone() {
            target.did_leave(app);
        }
        app.get_mut(self).current_targets.clear();
    }
}

impl RecognizerLeafData for TargetSelectionGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }

    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
}

impl OneSequenceLeafData for TargetSelectionGestureRecognizer {
    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }

    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl RecognizerLeaf for TargetSelectionGestureRecognizer {
    fn add_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        let sliding_tap = app.get(self).sliding_tap;
        RecognizerLeaf::add_pointer(sliding_tap, app, event);
    }

    fn add_pointer_pan_zoom(self: Handle<Self>, app: &mut App, event: PointerPanZoomStartEvent) {
        let sliding_tap = app.get(self).sliding_tap;
        RecognizerLeaf::add_pointer_pan_zoom(sliding_tap, app, event);
    }

    fn handle_event(self: Handle<Self>, _app: &mut App, _event: PointerEvent) {
        unreachable!("a target selection recognizer never tracks a pointer itself");
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        let sliding_tap = app.get(self).sliding_tap;
        RecognizerLeaf::accept_gesture(sliding_tap, app, pointer);
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        let sliding_tap = app.get(self).sliding_tap;
        RecognizerLeaf::reject_gesture(sliding_tap, app, pointer);
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, _app: &mut App, _pointer: i64) {
        unreachable!("a target selection recognizer never tracks a pointer itself");
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let sliding_tap = app.get(self).sliding_tap;
        RecognizerLeaf::dispose(sliding_tap, app);
        GestureRecognizer::dispose(self, app);
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "target selection"
    }
}

// The gesture detector used by action sheets.
//
// This gesture detector only recognizes one gesture,
// `TargetSelectionGestureRecognizer`.
//
// This widget's child might contain another VerticalDragGestureRecognizer if
// the actions section or the content section scrolls. Conveniently, Flutter's
// gesture algorithm makes the inner gesture take priority.
#[derive(Clone, Debug, Default)]
struct ActionSheetGestureDetector {
    child: Option<WidgetRef>,
}

impl ActionSheetGestureDetector {
    fn new() -> ActionSheetGestureDetector {
        ActionSheetGestureDetector::default()
    }

    fn child<K>(mut self, child: impl IntoWidget<K>) -> ActionSheetGestureDetector {
        self.child = Some(child.into_widget());
        self
    }

    fn hit_test(app: &mut App, context: BuildContext, global_position: Offset) -> HitTestResult {
        let view_id = View::of(app, context).id();
        let mut result = HitTestResult::new();
        GestureBinding::instance(app).hit_test_in_view(app, &mut result, global_position, view_id);
        result
    }
}

impl StatelessWidget for ActionSheetGestureDetector {
    fn build(&self, _app: &mut App, context: BuildContext) -> WidgetRef {
        let gestures: GestureRecognizerFactories = vec![(
            TypeId::of::<TargetSelectionGestureRecognizer>(),
            GestureRecognizerFactoryWithHandlers::<TargetSelectionGestureRecognizer>::new(
                move |app| {
                    TargetSelectionGestureRecognizer::new(
                        app,
                        Rc::new(move |app: &mut App, global_position| {
                            ActionSheetGestureDetector::hit_test(app, context, global_position)
                        }),
                    )
                },
                |_app, _instance| {},
            )
            .into_factory(),
        )];

        // `excludeFromSemantics` waits with accessibility.
        let mut detector = RawGestureDetector::new().gestures(gestures);
        if let Some(child) = &self.child {
            detector = detector.child(child.clone());
        }
        detector.into_widget()
    }
}

/// An iOS-style action sheet.
///
/// An action sheet is a specific style of alert that presents the user
/// with a set of two or more choices related to the current context.
/// An action sheet can have a title, an additional message, and a list
/// of actions. The title is displayed above the message and the actions
/// are displayed below this content.
///
/// This action sheet styles its title and message to match standard iOS action
/// sheet title and message text style.
///
/// To display action buttons that look like standard iOS action sheet buttons,
/// provide [`CupertinoActionSheetAction`]s for the [`actions`](Self::actions) given to this
/// action sheet.
///
/// To include a iOS-style cancel button separate from the other buttons,
/// provide an [`CupertinoActionSheetAction`] for the [`cancel_button`](Self::cancel_button)
/// given to this action sheet.
///
/// An action sheet is typically passed as the child widget to
/// [`show_cupertino_modal_popup`](crate::show_cupertino_modal_popup), which displays the
/// action sheet by sliding it up from the bottom of the screen.
///
/// See also:
///
///  * [`CupertinoActionSheetAction`], which is an iOS-style action sheet button.
///  * <https://developer.apple.com/design/human-interface-guidelines/ios/views/action-sheets/>
#[derive(Clone, Debug, Default)]
pub struct CupertinoActionSheet {
    pub key: Option<KeyRef>,
    /// An optional title of the action sheet. When the [`message`](Self::message) is
    /// non-null, the font of the [`title`](Self::title) is bold.
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,
    /// An optional descriptive message that provides more details about the
    /// reason for the alert.
    ///
    /// Typically a `Text` widget.
    pub message: Option<WidgetRef>,
    /// The set of actions that are displayed for the user to select.
    ///
    /// This must be a list of [`CupertinoActionSheetAction`] widgets.
    pub actions: Option<Vec<WidgetRef>>,
    /// A scroll controller that can be used to control the scrolling of the
    /// [`message`](Self::message) in the action sheet.
    ///
    /// Defaults to null, which means the [`CupertinoActionSheet`] will create a
    /// scroll controller internally.
    pub message_scroll_controller: Option<AnyScrollController>,
    /// A scroll controller that can be used to control the scrolling of the
    /// [`actions`](Self::actions) in the action sheet.
    ///
    /// Defaults to null, which means the [`CupertinoActionSheet`] will create an
    /// action scroll controller internally.
    pub action_scroll_controller: Option<AnyScrollController>,
    /// The optional cancel button that is grouped separately from the other
    /// actions.
    ///
    /// This must be a [`CupertinoActionSheetAction`] widget.
    pub cancel_button: Option<WidgetRef>,
}

impl CupertinoActionSheet {
    /// Creates an iOS-style action sheet.
    ///
    /// An action sheet must have a non-null value for at least one of the
    /// following arguments: [`actions`](Self::actions), [`title`](Self::title),
    /// [`message`](Self::message), or [`cancel_button`](Self::cancel_button).
    ///
    /// Generally, action sheets are used to give the user a choice between
    /// two or more choices for the current context.
    pub fn new() -> CupertinoActionSheet {
        CupertinoActionSheet::default()
    }

    /// Dart `CupertinoActionSheet(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoActionSheet {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoActionSheet(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> CupertinoActionSheet {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `CupertinoActionSheet(message:)`.
    pub fn message<K>(mut self, message: impl IntoWidget<K>) -> CupertinoActionSheet {
        self.message = Some(message.into_widget());
        self
    }

    /// Dart `CupertinoActionSheet(actions:)`.
    pub fn actions(mut self, actions: impl IntoIterator<Item = WidgetRef>) -> CupertinoActionSheet {
        self.actions = Some(actions.into_iter().collect());
        self
    }

    /// Dart `CupertinoActionSheet(messageScrollController:)`.
    pub fn message_scroll_controller(
        mut self,
        message_scroll_controller: AnyScrollController,
    ) -> CupertinoActionSheet {
        self.message_scroll_controller = Some(message_scroll_controller);
        self
    }

    /// Dart `CupertinoActionSheet(actionScrollController:)`.
    pub fn action_scroll_controller(
        mut self,
        action_scroll_controller: AnyScrollController,
    ) -> CupertinoActionSheet {
        self.action_scroll_controller = Some(action_scroll_controller);
        self
    }

    /// Dart `CupertinoActionSheet(cancelButton:)`.
    pub fn cancel_button<K>(mut self, cancel_button: impl IntoWidget<K>) -> CupertinoActionSheet {
        self.cancel_button = Some(cancel_button.into_widget());
        self
    }
}

impl StatefulWidget for CupertinoActionSheet {
    type State = CupertinoActionSheetState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoActionSheetState {
        debug_assert!(
            self.actions.is_some()
                || self.title.is_some()
                || self.message.is_some()
                || self.cancel_button.is_some(),
            "An action sheet must have a non-null value for at least one of the following \
             arguments: actions, title, message, or cancelButton"
        );
        CupertinoActionSheetState {
            state: StateData::new(),
            pressed_index: None,
            backup_message_scroll_controller: None,
            backup_action_scroll_controller: None,
        }
    }
}

/// Dart's `_CupertinoActionSheetState`.
pub struct CupertinoActionSheetState {
    state: StateData<CupertinoActionSheet>,
    pressed_index: Option<i32>,
    backup_message_scroll_controller: Option<AnyScrollController>,
    backup_action_scroll_controller: Option<AnyScrollController>,
}

impl CupertinoActionSheetState {
    const K_CANCEL_BUTTON_INDEX: i32 = -1;

    fn effective_message_scroll_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> AnyScrollController {
        if let Some(controller) = self.widget(app).message_scroll_controller {
            return controller;
        }
        if let Some(controller) = app.get(self).backup_message_scroll_controller {
            return controller;
        }
        let controller = ScrollController::default(app).as_controller();
        app.get_mut(self).backup_message_scroll_controller = Some(controller);
        controller
    }

    fn effective_action_scroll_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> AnyScrollController {
        if let Some(controller) = self.widget(app).action_scroll_controller {
            return controller;
        }
        if let Some(controller) = app.get(self).backup_action_scroll_controller {
            return controller;
        }
        let controller = ScrollController::default(app).as_controller();
        app.get_mut(self).backup_action_scroll_controller = Some(controller);
        controller
    }

    fn has_content(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.title.is_some() || widget.message.is_some()
    }

    fn build_content(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> Option<WidgetRef> {
        if !self.has_content(app) {
            return None;
        }
        let widget = self.widget(app).clone();
        let color =
            CupertinoDynamicColor::resolve(&K_ACTION_SHEET_CONTENT_TEXT_COLOR, app, context);
        let text_style = action_sheet_content_style().copy_with().color(color);
        let scroll_controller = self.effective_message_scroll_controller(app);
        let mut section = CupertinoAlertContentSection::new(scroll_controller)
            .title_padding(EdgeInsets::only(
                K_ACTION_SHEET_CONTENT_HORIZONTAL_PADDING,
                K_ACTION_SHEET_CONTENT_VERTICAL_PADDING,
                K_ACTION_SHEET_CONTENT_HORIZONTAL_PADDING,
                if widget.message.is_none() {
                    K_ACTION_SHEET_CONTENT_VERTICAL_PADDING
                } else {
                    0.0
                },
            ))
            .message_padding(EdgeInsets::only(
                K_ACTION_SHEET_CONTENT_HORIZONTAL_PADDING,
                if widget.title.is_none() {
                    K_ACTION_SHEET_CONTENT_VERTICAL_PADDING
                } else {
                    0.0
                },
                K_ACTION_SHEET_CONTENT_HORIZONTAL_PADDING,
                K_ACTION_SHEET_CONTENT_VERTICAL_PADDING,
            ))
            .title_text_style(if widget.message.is_none() {
                text_style.clone()
            } else {
                text_style.copy_with().font_weight(FontWeight::W600)
            })
            .message_text_style(if widget.title.is_none() {
                text_style.copy_with().font_weight(FontWeight::W600)
            } else {
                text_style
            })
            .additional_padding_between_title_and_message(EdgeInsets::only(0.0, 4.0, 0.0, 0.0));
        if let Some(title) = widget.title {
            section = section.title(title);
        }
        if let Some(message) = widget.message {
            section = section.message(message);
        }

        let background =
            CupertinoDynamicColor::resolve(&K_ACTION_SHEET_BACKGROUND_COLOR, app, context);
        Some(ColoredBox::new(background).child(section).into_widget())
    }

    fn on_pressed_update(self: Handle<Self>, app: &mut App, action_index: i32, state: bool) {
        if !state {
            if app.get(self).pressed_index == Some(action_index) {
                self.set_state(app, |this| {
                    this.pressed_index = None;
                });
            }
        } else {
            self.set_state(app, |this| {
                this.pressed_index = Some(action_index);
            });
        }
    }

    fn build_cancel_button(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.widget(app).clone();
        debug_assert!(widget.cancel_button.is_some());
        let cancel_padding =
            if widget.actions.is_some() || widget.message.is_some() || widget.title.is_some() {
                K_ACTION_SHEET_CANCEL_BUTTON_PADDING
            } else {
                0.0
            };
        let pressed =
            app.get(self).pressed_index == Some(CupertinoActionSheetState::K_CANCEL_BUTTON_INDEX);

        Padding::new(EdgeInsetsGeometry::only(0.0, cancel_padding, 0.0, 0.0))
            .child(CupertinoFocusHalo::with_rrect(
                k_cupertino_button_size_border_radius(CupertinoButtonSize::Large).into(),
                ActionSheetButtonBackground::new(
                    pressed,
                    widget.cancel_button.expect("asserted above"),
                )
                .is_cancel(true)
                .on_press_state_change(move |app: &mut App, state: bool| {
                    self.on_pressed_update(
                        app,
                        CupertinoActionSheetState::K_CANCEL_BUTTON_INDEX,
                        state,
                    );
                }),
            ))
            .into_widget()
    }

    // Given data point (x1, y1) and (x2, y2), derive the y corresponding to x
    // using linear interpolation between the two data points, and extrapolates
    // flatly beyond these points.
    //
    //              (x2, y2)
    //                _____________
    //               /
    //              /
    //    _________/
    //           (x1, y1)
    fn lerp(x: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        if x <= x1 {
            y1
        } else if x >= x2 {
            y2
        } else {
            lerp_double(Some(y1), Some(y2), (x - x1) / (x2 - x1)).expect("both ends are given")
        }
    }

    // Derive the top padding, which is the distance between the top of a
    // full-height action sheet and the top of the safe area.
    //
    // The algorithm and its values are derived from measuring on the simulator.
    fn top_padding(app: &mut App, context: BuildContext) -> f64 {
        if MediaQuery::orientation_of(app, context) == Orientation::Landscape {
            return K_ACTION_SHEET_EDGE_PADDING;
        }

        // The top padding in portrait mode is in general close to the top view
        // padding, but not always equal:
        //
        //                            | view padding | action sheet padding | ratio
        //   No notch (eg. iPhone SE) |     20.0     |        20.0          | 1.0
        //   Notch (eg. iPhone 13)    |     47.0     |        47.0          | 1.0
        //   Capsule (eg. iPhone 15)  |     59.0     |        54.0          | 0.915
        //
        // Currently, we cannot determine why the result changes on "capsules."
        // Therefore, we'll hard code this rule, given the limited types of actual
        // devices. To provide an algorithm that accepts arbitrary view padding, this
        // function calculates the ratio as a continuous curve with linear
        // interpolation.

        // The x for lerp is the top view padding, while the y is ratio of
        // action sheet padding versus top view padding.
        const VIEW_PADDING_DATA1: f64 = 47.0;
        const PADDING_RATIO_DATA1: f64 = 1.0;
        const VIEW_PADDING_DATA2: f64 = 59.0;
        const PADDING_RATIO_DATA2: f64 = 54.0 / 59.0;

        let current_view_padding = MediaQuery::view_padding_of(app, context).top;

        let current_padding_ratio = CupertinoActionSheetState::lerp(
            current_view_padding,
            VIEW_PADDING_DATA1,
            PADDING_RATIO_DATA1,
            VIEW_PADDING_DATA2,
            PADDING_RATIO_DATA2,
        );
        let padding = (current_padding_ratio * current_view_padding).round();
        // In case there is no view padding, there should still be some space
        // between the action sheet and the edge.
        padding.max(K_DIALOG_EDGE_PADDING)
    }
}

impl State for CupertinoActionSheetState {
    type Widget = CupertinoActionSheet;
    reveal_widgets::state_accessors!();

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).backup_message_scroll_controller {
            ScrollController::dispose(controller, app);
        }
        if let Some(controller) = app.get(self).backup_action_scroll_controller {
            ScrollController::dispose(controller, app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        //  ╭─────────────────╮  ↑                ↑
        //  │    The title    │ Content section   |
        //  │   The message   │  ↓                |
        //  ├─────────────────┤  ↑             Main sheet
        //  │    Action 1     │  |                |
        //  ├─────────────────┤ Actions section   |
        //  │    Action 2     │  |                |
        //  ╰─────────────────╯  ↓                ↓
        //  ╭─────────────────╮
        //  │     Cancel      │
        //  ╰─────────────────╯

        let widget = self.widget(app).clone();
        let pressed_index = app.get(self).pressed_index;
        let scroll_controller = self.effective_action_scroll_controller(app);
        let content_section = self.build_content(app, context);
        let divider_color =
            CupertinoDynamicColor::resolve(&K_ACTION_SHEET_BUTTON_DIVIDER_COLOR, app, context);
        let mut children = vec![
            Flexible::new(
                ClipRSuperellipse::new()
                    .border_radius(BorderRadius::circular(12.0).into())
                    .child(
                        BackdropFilter::filter(ImageFilter::blur(
                            CupertinoPopupSurface::DEFAULT_BLUR_SIGMA,
                            CupertinoPopupSurface::DEFAULT_BLUR_SIGMA,
                        ))
                        .child(ActionSheetMainSheet::new(
                            pressed_index,
                            Rc::new(move |app: &mut App, action_index, state| {
                                self.on_pressed_update(app, action_index, state);
                            }),
                            scroll_controller,
                            widget.actions.clone().unwrap_or_default(),
                            content_section,
                            divider_color,
                        )),
                    ),
            )
            .into_widget(),
        ];
        if widget.cancel_button.is_some() {
            children.push(self.build_cancel_button(app));
        }
        let action_sheet_width = match MediaQuery::orientation_of(app, context) {
            Orientation::Portrait => MediaQuery::width_of(app, context),
            Orientation::Landscape => MediaQuery::height_of(app, context),
        };
        let top_padding = CupertinoActionSheetState::top_padding(app, context);
        let behavior = ScrollConfiguration::of(app, context)
            .copy_with()
            .scrollbars(false);

        SafeArea::new(ScrollConfiguration::new(
            // A CupertinoScrollbar is built-in below
            ScrollBehaviorRef::new(behavior),
            CupertinoUserInterfaceLevel::new(
                CupertinoUserInterfaceLevelData::Elevated,
                Padding::new(EdgeInsetsGeometry::only(
                    K_ACTION_SHEET_EDGE_PADDING,
                    top_padding,
                    K_ACTION_SHEET_EDGE_PADDING,
                    // The bottom padding is set on SafeArea::minimum, allowing it to
                    // be consumed by bottom view padding.
                    0.0,
                ))
                .child(
                    SizedBox::new()
                        .width(action_sheet_width - K_ACTION_SHEET_EDGE_PADDING * 2.0)
                        .child(
                            ActionSheetGestureDetector::new().child(
                                Column::new()
                                    .main_axis_alignment(MainAxisAlignment::End)
                                    .main_axis_size(MainAxisSize::Min)
                                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                    .children(children),
                            ),
                        ),
                ),
            ),
        ))
        .minimum(EdgeInsets::only(0.0, 0.0, 0.0, K_ACTION_SHEET_EDGE_PADDING))
        .into_widget()
    }
}

/// The content of a typical action button in a [`CupertinoActionSheet`].
///
/// This widget draws the content of a button, i.e. the text, while the
/// background of the button is drawn by [`CupertinoActionSheet`]. When
/// [`focus_node`](Self::focus_node) has focus, this widget will draw the background of color
/// [`focus_color`](Self::focus_color).
///
/// See also:
///
///  * [`CupertinoActionSheet`], an alert that presents the user with a set of two or
///    more choices related to the current context.
#[derive(Clone)]
pub struct CupertinoActionSheetAction {
    pub key: Option<KeyRef>,
    /// The callback that is called when the button is selected.
    ///
    /// The button can be selected by either by tapping on this button or by
    /// pressing elsewhere and sliding onto this button before releasing.
    pub on_pressed: Listener,
    /// Whether this action is the default choice in the action sheet.
    ///
    /// Default buttons have bold text.
    pub is_default_action: bool,
    /// Whether this action might change or delete data.
    ///
    /// Destructive buttons have red text.
    pub is_destructive_action: bool,
    /// The cursor that will be shown when hovering over the button.
    ///
    /// If null, defaults to `SystemMouseCursors::CLICK` on web and
    /// `MouseCursor::defer()` on other platforms.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<AnyFocusNode>,
    /// The color of the background that highlights active focus.
    ///
    /// A transparency of `K_CUPERTINO_BUTTON_TINTED_OPACITY_LIGHT` (light mode) or
    /// `K_CUPERTINO_BUTTON_TINTED_OPACITY_DARK` (dark mode) is automatically applied to
    /// this color.
    ///
    /// When [`focus_color`](Self::focus_color) is null, defaults to
    /// `CupertinoColors::ACTIVE_BLUE`.
    pub focus_color: Option<AnyColor>,
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    pub child: WidgetRef,
}

impl CupertinoActionSheetAction {
    /// Creates an action for an iOS-style action sheet.
    pub fn new<K>(on_pressed: Listener, child: impl IntoWidget<K>) -> CupertinoActionSheetAction {
        CupertinoActionSheetAction {
            key: None,
            on_pressed,
            is_default_action: false,
            is_destructive_action: false,
            mouse_cursor: None,
            focus_node: None,
            focus_color: None,
            child: child.into_widget(),
        }
    }

    /// Dart `CupertinoActionSheetAction(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoActionSheetAction {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoActionSheetAction(isDefaultAction:)`.
    pub fn is_default_action(mut self, is_default_action: bool) -> CupertinoActionSheetAction {
        self.is_default_action = is_default_action;
        self
    }

    /// Dart `CupertinoActionSheetAction(isDestructiveAction:)`.
    pub fn is_destructive_action(
        mut self,
        is_destructive_action: bool,
    ) -> CupertinoActionSheetAction {
        self.is_destructive_action = is_destructive_action;
        self
    }

    /// Dart `CupertinoActionSheetAction(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> CupertinoActionSheetAction {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `CupertinoActionSheetAction(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> CupertinoActionSheetAction {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CupertinoActionSheetAction(focusColor:)`.
    pub fn focus_color(mut self, focus_color: impl Into<AnyColor>) -> CupertinoActionSheetAction {
        self.focus_color = Some(focus_color.into());
        self
    }
}

impl Debug for CupertinoActionSheetAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoActionSheetAction")
            .field("is_default_action", &self.is_default_action)
            .field("is_destructive_action", &self.is_destructive_action)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoActionSheetAction {
    type State = CupertinoActionSheetActionState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoActionSheetActionState {
        CupertinoActionSheetActionState {
            state: StateData::new(),
            show_highlight: false,
            action_map: HashMap::new(),
            slide_target: None,
        }
    }
}

/// Dart's `_CupertinoActionSheetActionState`.
pub struct CupertinoActionSheetActionState {
    state: StateData<CupertinoActionSheetAction>,
    show_highlight: bool,
    /// Dart's `late final _actionMap`, filled in `init_state` (an action is an arena object).
    action_map: HashMap<TypeId, AnyAction>,
    /// This state as the `SlideTarget` its `MetaData` carries, minted once in `init_state`.
    slide_target: Option<Rc<dyn SlideTarget>>,
}

impl CupertinoActionSheetActionState {
    fn on_show_focus_highlight(self: Handle<Self>, app: &mut App, show_highlight: bool) {
        self.set_state(app, |state| {
            state.show_highlight = show_highlight;
        });
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        let on_pressed = self.widget(app).on_pressed.clone();
        on_pressed.call(app);
        // `sendSemanticsEvent(TapSemanticEvent())` waits with accessibility.
    }

    fn effective_focus_background_color(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> AnyColor {
        let opacity = if CupertinoTheme::brightness_of(app, context) == Brightness::Light {
            K_CUPERTINO_BUTTON_TINTED_OPACITY_LIGHT
        } else {
            K_CUPERTINO_BUTTON_TINTED_OPACITY_DARK
        };
        let base = self
            .widget(app)
            .focus_color
            .clone()
            .unwrap_or(CupertinoColors::ACTIVE_BLUE);
        // Dart's deprecated `withOpacity` is `with_values(alpha:)`, as its note says.
        let color = base
            .color()
            .with_values(Some(opacity), None, None, None, None);
        AnyColor::new(HSLColor::from_color(color).to_color())
    }
}

impl SlideTarget for Handle<CupertinoActionSheetActionState> {
    fn did_enter(&self, _app: &mut App, _from_pointer_down: bool, inner_enabled: bool) -> bool {
        inner_enabled
    }

    fn did_leave(&self, _app: &mut App) {}

    fn did_confirm(&self, app: &mut App) {
        let this = *self;
        let on_pressed = this.widget(app).on_pressed.clone();
        on_pressed.call(app);
    }
}

impl State for CupertinoActionSheetActionState {
    type Widget = CupertinoActionSheetAction;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let activate = CallbackAction::<ActivateIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ActivateIntent| {
                self.handle_tap(app);
                None
            }),
        );
        app.get_mut(self)
            .action_map
            .insert(TypeId::of::<ActivateIntent>(), activate.as_action());
        app.get_mut(self).slide_target = Some(Rc::new(self));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for action in std::mem::take(&mut app.get_mut(self).action_map).into_values() {
            app.destroy(action.id());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let content = ActionSheetActionContent {
            is_destructive_action: widget.is_destructive_action,
            is_default_action: widget.is_default_action,
            child: widget.child.clone(),
        };
        let highlighted = if app.get(self).show_highlight {
            let color = self.effective_focus_background_color(app, context);
            DecoratedBox::new(BoxDecoration::new().color(color))
                .child(content)
                .into_widget()
        } else {
            content.into_widget()
        };
        let mut detector = FocusableActionDetector::new(highlighted)
            .actions(app.get(self).action_map.clone())
            .on_show_focus_highlight(move |app, show_highlight| {
                self.on_show_focus_highlight(app, show_highlight)
            });
        if let Some(focus_node) = widget.focus_node {
            detector = detector.focus_node(focus_node);
        }
        let cursor = widget.mouse_cursor.unwrap_or_else(|| {
            if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                <dyn MouseCursor>::defer()
            }
        });
        // `Semantics(button: true, onTap:)` waits with accessibility.
        MouseRegion::new()
            .cursor(cursor)
            .child(
                MetaData::new()
                    .meta_data(slide_target_meta_data(&app.get(self).slide_target))
                    .behavior(HitTestBehavior::Opaque)
                    .child(
                        ConstrainedBox::new(
                            BoxConstraints::new().min_height(K_ACTION_SHEET_BUTTON_MIN_HEIGHT),
                        )
                        .child(detector),
                    ),
            )
            .into_widget()
    }
}

#[derive(Clone, Debug)]
struct ActionSheetActionContent {
    is_destructive_action: bool,
    is_default_action: bool,
    child: WidgetRef,
}

impl ActionSheetActionContent {
    // Calculates the font size for action sheet buttons.
    //
    // The `context_body_size` is the body font size specified by context. The
    // return value is the button font size, including the effect of context font
    // scale factor. Divide by context font scale factor before using in a `Text`.
    fn button_font_size(context_body_size: f64) -> f64 {
        // It is observed that the native action sheet buttons use font sizes that
        // deviate from standard HIG specifications in a non-linear way. The following
        // table shows the regular body font size vs the button font size:
        //
        //  Text scale  | xs |  s |  m |  l | xl | xxl | xxxl | ax1 | ax2 | ax3 | ax4 | ax5
        //  Body font   | 14 | 15 | 16 | 17 | 19 |  21 |  23  |  28 |  33 |  40 |  47 |  53
        //  Button font | 21 | 21 | 21 | 21 | 23 |  24 |  24  |  28 |  33 |  40 |  47 |  53

        // For very small or very large text, simple rules can be observed.
        // For mid-sized text, piecewise linear interpolation is used.
        if context_body_size <= 17.0 {
            21.0
        } else if context_body_size <= 19.0 {
            lerp_double(
                Some(21.0),
                Some(23.0),
                (context_body_size - 17.0) / (19.0 - 17.0),
            )
            .expect("both ends are given")
        } else if context_body_size <= 21.0 {
            lerp_double(
                Some(23.0),
                Some(24.0),
                (context_body_size - 19.0) / (21.0 - 19.0),
            )
            .expect("both ends are given")
        } else if context_body_size <= 24.0 {
            24.0
        } else {
            context_body_size
        }
    }
}

impl StatelessWidget for ActionSheetActionContent {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        // The context scale factor is derived from the current body size and the
        // standard body size in "large".
        const HIG_LARGE_BODY_SIZE: f64 = 17.0;
        let context_body_size = MediaQuery::text_scaler_of(app, context).scale(HIG_LARGE_BODY_SIZE);
        let context_scale_factor = context_body_size / HIG_LARGE_BODY_SIZE;
        let font_size = ActionSheetActionContent::button_font_size(context_body_size);

        let color = if self.is_destructive_action {
            CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_RED, app, context)
        } else {
            CupertinoTheme::of(app, context).primary_color()
        };
        let mut style = action_sheet_action_style()
            .copy_with()
            // `Text` will scale the provided font size inside, so its parameter is
            // unscaled first.
            .font_size(font_size / context_scale_factor)
            .color(color);

        if self.is_default_action {
            style = style.copy_with().font_weight(FontWeight::W600);
        }
        let vertical_padding = K_ACTION_SHEET_BUTTON_VERTICAL_PADDING_BASE
            + font_size * K_ACTION_SHEET_BUTTON_VERTICAL_PADDING_FACTOR;

        Padding::new(EdgeInsetsGeometry::from_ltrb(
            K_ACTION_SHEET_BUTTON_HORIZONTAL_PADDING,
            vertical_padding,
            K_ACTION_SHEET_BUTTON_HORIZONTAL_PADDING,
            vertical_padding,
        ))
        .child(
            DefaultTextStyle::new(style, Center::new().child(self.child.clone()))
                .text_align(TextAlign::Center),
        )
        .into_widget()
    }
}

// Renders the background of a button (both the pressed background and the idle
// background) and reports its state to the parent with `on_press_state_change`.
//
// Although this class doesn't keep any states, it's still a stateful widget
// because the state is used as a persistent object across rebuilds to provide
// to `MetaData`'s `meta_data`.
#[derive(Clone)]
struct ActionSheetButtonBackground {
    is_cancel: bool,
    /// Whether the user is holding on this button.
    pressed: bool,
    /// Called when the user taps down or lifts up on the button.
    ///
    /// The boolean value is true if the user is tapping down on the button.
    on_press_state_change: Option<ValueSetter<bool>>,
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    child: WidgetRef,
}

impl ActionSheetButtonBackground {
    fn new<K>(pressed: bool, child: impl IntoWidget<K>) -> ActionSheetButtonBackground {
        ActionSheetButtonBackground {
            is_cancel: false,
            pressed,
            on_press_state_change: None,
            child: child.into_widget(),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn is_cancel(mut self, is_cancel: bool) -> ActionSheetButtonBackground {
        self.is_cancel = is_cancel;
        self
    }

    fn on_press_state_change(
        mut self,
        on_press_state_change: impl Fn(&mut App, bool) + 'static,
    ) -> ActionSheetButtonBackground {
        self.on_press_state_change = Some(Rc::new(on_press_state_change));
        self
    }
}

impl Debug for ActionSheetButtonBackground {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionSheetButtonBackground")
            .field("is_cancel", &self.is_cancel)
            .field("pressed", &self.pressed)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ActionSheetButtonBackground {
    type State = ActionSheetButtonBackgroundState;

    fn create_state(&self) -> ActionSheetButtonBackgroundState {
        ActionSheetButtonBackgroundState {
            state: StateData::new(),
            slide_target: None,
        }
    }
}

/// Dart's `_ActionSheetButtonBackgroundState`.
struct ActionSheetButtonBackgroundState {
    state: StateData<ActionSheetButtonBackground>,
    /// This state as the `SlideTarget` its `MetaData` carries, minted once in `init_state`.
    slide_target: Option<Rc<dyn SlideTarget>>,
}

impl ActionSheetButtonBackgroundState {
    fn emit_vibration(app: &mut App) {
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::Android => HapticFeedback::selection_click(app),
            TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {}
        }
    }
}

impl SlideTarget for Handle<ActionSheetButtonBackgroundState> {
    fn did_enter(&self, app: &mut App, from_pointer_down: bool, inner_enabled: bool) -> bool {
        let this = *self;
        // Action sheet doesn't support disabled buttons, therefore `inner_enabled`
        // is always true.
        debug_assert!(inner_enabled);
        if let Some(callback) = this.widget(app).on_press_state_change.clone() {
            callback(app, true);
        }
        if !from_pointer_down {
            ActionSheetButtonBackgroundState::emit_vibration(app);
        }
        inner_enabled
    }

    fn did_leave(&self, app: &mut App) {
        let this = *self;
        if let Some(callback) = this.widget(app).on_press_state_change.clone() {
            callback(app, false);
        }
    }

    fn did_confirm(&self, app: &mut App) {
        let this = *self;
        if let Some(callback) = this.widget(app).on_press_state_change.clone() {
            callback(app, false);
        }
    }
}

impl State for ActionSheetButtonBackgroundState {
    type Widget = ActionSheetButtonBackground;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).slide_target = Some(Rc::new(self));
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let child = if !widget.is_cancel {
            let unresolved = if widget.pressed {
                K_ACTION_SHEET_PRESSED_COLOR
            } else {
                K_ACTION_SHEET_BACKGROUND_COLOR
            };
            let color = CupertinoDynamicColor::resolve(&unresolved, app, context);
            ColoredBox::new(color).child(widget.child).into_widget()
        } else {
            let border_radius = BorderRadius::circular(K_CORNER_RADIUS);
            let unresolved = if widget.pressed {
                K_ACTION_SHEET_CANCEL_PRESSED_COLOR
            } else {
                K_ACTION_SHEET_CANCEL_COLOR
            };
            let color = CupertinoDynamicColor::resolve(&unresolved, app, context);
            ClipRSuperellipse::new()
                .border_radius(border_radius.into())
                .child(DecoratedBox::new(BoxDecoration::new().color(color)).child(widget.child))
                .into_widget()
        };

        MetaData::new()
            .meta_data(slide_target_meta_data(&app.get(self).slide_target))
            .child(child)
            .into_widget()
    }
}

// The divider of an action sheet or an alert dialog.
//
// The divider can function as either a horizontal divider (in a column) or a
// vertical divider (in a row) without widget-layer configuration. Instead, this
// is determined during the layout phase based on the constraints. This approach
// is necessary to allow the alert dialog to provide a list of widgets to the
// layout widget, which doesn't know its layout mode until the layout phase.
//
// The constraints provided to this widget should match the column container's
// width or the row container's height, while being unlimited in the other
// dimension. This unlimited dimension will result in the divider's thickness.
//
// If the divider is not `hidden`, then it displays the `divider_color`.
// Otherwise it displays the background color.
#[derive(Clone, Debug)]
struct Divider {
    divider_color: AnyColor,
    hidden_color: AnyColor,
    hidden: bool,
}

impl Divider {
    fn new(divider_color: AnyColor, hidden_color: AnyColor, hidden: bool) -> Divider {
        Divider {
            divider_color,
            hidden_color,
            hidden,
        }
    }
}

impl StatelessWidget for Divider {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let color = if self.hidden {
            CupertinoDynamicColor::resolve(&self.hidden_color, app, context)
        } else {
            self.divider_color.clone()
        };
        // The LimitedBox turns unconstrained dimension (typically the main axis of
        // a flex container) to the divider thickness.
        LimitedBox::new()
            .max_height(K_DIVIDER_THICKNESS)
            .max_width(K_DIVIDER_THICKNESS)
            // The constrained box prevents the divider from collapsing to nothing.
            .child(
                ConstrainedBox::new(
                    BoxConstraints::new()
                        .min_height(K_DIVIDER_THICKNESS)
                        .min_width(K_DIVIDER_THICKNESS),
                )
                .child(DecoratedBox::new(BoxDecoration::new().color(color))),
            )
            .into_widget()
    }
}

// Fills the overscroll area at the top or bottom of a scrollable widget with a
// solid color.
//
// This is necessary for action sheets and alert dialogs, because their actions
// section's background is rendered by the buttons, so that a button's
// background can be _replaced_ by a different color when the button is pressed.
#[derive(Clone, Debug)]
struct OverscrollBackground {
    // The color for the overscroll part.
    //
    // This value must be a resolved color instead of, for example, a
    // CupertinoDynamicColor.
    color: AnyColor,
    child: WidgetRef,
}

impl OverscrollBackground {
    fn new<K>(color: AnyColor, child: impl IntoWidget<K>) -> OverscrollBackground {
        OverscrollBackground {
            color,
            child: child.into_widget(),
        }
    }
}

impl StatefulWidget for OverscrollBackground {
    type State = OverscrollBackgroundState;

    fn create_state(&self) -> OverscrollBackgroundState {
        OverscrollBackgroundState {
            state: StateData::new(),
            top_overscroll: 0.0,
            bottom_overscroll: 0.0,
        }
    }
}

/// Dart's `_OverscrollBackgroundState`.
struct OverscrollBackgroundState {
    state: StateData<OverscrollBackground>,
    top_overscroll: f64,
    bottom_overscroll: f64,
}

impl OverscrollBackgroundState {
    fn on_scroll_update(
        self: Handle<Self>,
        app: &mut App,
        notification: &ScrollUpdateNotification,
    ) -> bool {
        let metrics = Rc::clone(notification.metrics());
        self.set_state(app, |state| {
            // The sizes of the overscroll should not be longer than the height of the
            // actions section.
            state.top_overscroll = (metrics.min_scroll_extent() - metrics.pixels())
                .max(0.0)
                .min(metrics.viewport_dimension());
            state.bottom_overscroll = (metrics.pixels() - metrics.max_scroll_extent())
                .max(0.0)
                .min(metrics.viewport_dimension());
        });
        false
    }
}

impl State for OverscrollBackgroundState {
    type Widget = OverscrollBackground;
    reveal_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let (top_overscroll, bottom_overscroll) = {
            let state = app.get(self);
            (state.top_overscroll, state.bottom_overscroll)
        };
        let overscroll = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children([
                DecoratedBox::new(BoxDecoration::new().color(widget.color.clone()))
                    .child(SizedBox::new().height(top_overscroll))
                    .into_widget(),
                DecoratedBox::new(BoxDecoration::new().color(widget.color))
                    .child(SizedBox::new().height(bottom_overscroll))
                    .into_widget(),
            ]);
        Stack::new()
            .children([
                Positioned::fill(overscroll).into_widget(),
                NotificationListener::<ScrollUpdateNotification>::new(widget.child)
                    .on_notification(move |app, notification| {
                        self.on_scroll_update(app, notification)
                    })
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// Dart's `typedef _PressedUpdateHandler = void Function(int actionIndex, bool state)`.
type PressedUpdateHandler = Rc<dyn Fn(&mut App, i32, bool)>;

// The list of actions in an action sheet.
//
// This excludes the divider between the action section and the content section.
#[derive(Clone)]
struct ActionSheetActionSection {
    actions: Option<Vec<WidgetRef>>,
    on_pressed_update: PressedUpdateHandler,
    pressed_index: Option<i32>,
    divider_color: AnyColor,
    background_color: AnyColor,
    scroll_controller: AnyScrollController,
}

impl ActionSheetActionSection {
    fn new(
        actions: Option<Vec<WidgetRef>>,
        pressed_index: Option<i32>,
        divider_color: AnyColor,
        background_color: AnyColor,
        on_pressed_update: PressedUpdateHandler,
        scroll_controller: AnyScrollController,
    ) -> ActionSheetActionSection {
        ActionSheetActionSection {
            actions,
            on_pressed_update,
            pressed_index,
            divider_color,
            background_color,
            scroll_controller,
        }
    }
}

impl Debug for ActionSheetActionSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionSheetActionSection")
            .field("pressed_index", &self.pressed_index)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for ActionSheetActionSection {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let _ = &self.background_color; // Dart's unused `backgroundColor` field.
        let actions = match &self.actions {
            Some(actions) if !actions.is_empty() => actions,
            _ => {
                return LimitedBox::new()
                    .max_width(0.0)
                    .child(SizedBox::new().width(f64::INFINITY).height(0.0))
                    .into_widget();
            }
        };
        let mut column = Vec::new();
        for (action_index, action) in actions.iter().enumerate() {
            let action_index = action_index as i32;
            if action_index != 0 {
                column.push(
                    Divider::new(
                        self.divider_color.clone(),
                        K_ACTION_SHEET_BACKGROUND_COLOR,
                        self.pressed_index == Some(action_index - 1)
                            || self.pressed_index == Some(action_index),
                    )
                    .into_widget(),
                );
            }
            let on_pressed_update = Rc::clone(&self.on_pressed_update);
            column.push(
                ActionSheetButtonBackground::new(
                    self.pressed_index == Some(action_index),
                    action.clone(),
                )
                .on_press_state_change(move |app, state| {
                    on_pressed_update(app, action_index, state);
                })
                .into_widget(),
            );
        }

        CupertinoScrollbar::new(
            SingleChildScrollView::new()
                .controller(self.scroll_controller)
                .child(
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .children(column),
                ),
        )
        .controller(self.scroll_controller)
        .into_widget()
    }
}

// The part of an action sheet without the cancel button.
#[derive(Clone)]
struct ActionSheetMainSheet {
    pressed_index: Option<i32>,
    on_pressed_update: PressedUpdateHandler,
    scroll_controller: AnyScrollController,
    actions: Vec<WidgetRef>,
    content_section: Option<WidgetRef>,
    divider_color: AnyColor,
}

impl ActionSheetMainSheet {
    fn new(
        pressed_index: Option<i32>,
        on_pressed_update: PressedUpdateHandler,
        scroll_controller: AnyScrollController,
        actions: Vec<WidgetRef>,
        content_section: Option<WidgetRef>,
        divider_color: AnyColor,
    ) -> ActionSheetMainSheet {
        ActionSheetMainSheet {
            pressed_index,
            on_pressed_update,
            scroll_controller,
            actions,
            content_section,
            divider_color,
        }
    }

    fn empty() -> WidgetRef {
        LimitedBox::new()
            .max_width(0.0)
            .child(SizedBox::new().width(f64::INFINITY).height(0.0))
            .into_widget()
    }

    fn scrolled_actions_section(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let background_color =
            CupertinoDynamicColor::resolve(&K_ACTION_SHEET_BACKGROUND_COLOR, app, context);
        let border_radius = k_cupertino_button_size_border_radius(CupertinoButtonSize::Large)
            .copy_with(Some(Radius::ZERO), Some(Radius::ZERO), None, None);
        OverscrollBackground::new(
            background_color.clone(),
            CupertinoFocusHalo::with_rrect(
                border_radius.into(),
                ActionSheetActionSection::new(
                    Some(self.actions.clone()),
                    self.pressed_index,
                    self.divider_color.clone(),
                    background_color,
                    Rc::clone(&self.on_pressed_update),
                    self.scroll_controller,
                ),
            ),
        )
        .into_widget()
    }

    fn divider_and_actions_section(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let background_color =
            CupertinoDynamicColor::resolve(&K_ACTION_SHEET_BACKGROUND_COLOR, app, context);
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children([
                Divider::new(self.divider_color.clone(), background_color, false).into_widget(),
                Flexible::new(self.scrolled_actions_section(app, context)).into_widget(),
            ])
            .into_widget()
    }
}

impl Debug for ActionSheetMainSheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionSheetMainSheet")
            .field("pressed_index", &self.pressed_index)
            .field("actions", &self.actions.len())
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for ActionSheetMainSheet {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if self.actions.is_empty() {
            return self
                .content_section
                .clone()
                .unwrap_or_else(ActionSheetMainSheet::empty);
        }
        let Some(content_section) = self.content_section.clone() else {
            return self.scrolled_actions_section(app, context);
        };

        PriorityColumn::new(
            content_section,
            self.divider_and_actions_section(app, context),
            K_ACTION_SHEET_ACTIONS_SECTION_MIN_HEIGHT + K_DIVIDER_THICKNESS,
        )
        .into_widget()
    }
}

// The "content section" of a CupertinoAlertDialog.
//
// If title is missing, then only content is added. If content is
// missing, then only title is added. If both are missing, then it returns
// a SingleChildScrollView with a zero-sized SizedBox.
#[derive(Clone, Debug)]
struct CupertinoAlertContentSection {
    // The (optional) title of the dialog is displayed in a large font at the top
    // of the dialog.
    //
    // Typically a Text widget.
    title: Option<WidgetRef>,
    // The (optional) message of the dialog is displayed in the center of the
    // dialog in a lighter font.
    //
    // Typically a Text widget.
    message: Option<WidgetRef>,
    // A scroll controller that can be used to control the scrolling of the
    // content in the dialog.
    scroll_controller: AnyScrollController,
    // Paddings used around title and message.
    // CupertinoAlertDialog and CupertinoActionSheet have different paddings.
    title_padding: Option<EdgeInsets>,
    message_padding: Option<EdgeInsets>,
    // Additional padding to be inserted between title and message.
    // Only used for CupertinoActionSheet.
    additional_padding_between_title_and_message: Option<EdgeInsets>,
    // Text styles used for title and message.
    // CupertinoAlertDialog and CupertinoActionSheet have different text styles.
    title_text_style: Option<TextStyle>,
    message_text_style: Option<TextStyle>,
}

impl CupertinoAlertContentSection {
    fn new(scroll_controller: AnyScrollController) -> CupertinoAlertContentSection {
        CupertinoAlertContentSection {
            title: None,
            message: None,
            scroll_controller,
            title_padding: None,
            message_padding: None,
            additional_padding_between_title_and_message: None,
            title_text_style: None,
            message_text_style: None,
        }
    }

    fn title<K>(mut self, title: impl IntoWidget<K>) -> CupertinoAlertContentSection {
        self.title = Some(title.into_widget());
        self
    }

    fn message<K>(mut self, message: impl IntoWidget<K>) -> CupertinoAlertContentSection {
        self.message = Some(message.into_widget());
        self
    }

    fn title_padding(mut self, title_padding: EdgeInsets) -> CupertinoAlertContentSection {
        self.title_padding = Some(title_padding);
        self
    }

    fn message_padding(mut self, message_padding: EdgeInsets) -> CupertinoAlertContentSection {
        self.message_padding = Some(message_padding);
        self
    }

    fn additional_padding_between_title_and_message(
        mut self,
        padding: EdgeInsets,
    ) -> CupertinoAlertContentSection {
        self.additional_padding_between_title_and_message = Some(padding);
        self
    }

    fn title_text_style(mut self, title_text_style: TextStyle) -> CupertinoAlertContentSection {
        self.title_text_style = Some(title_text_style);
        self
    }

    fn message_text_style(mut self, message_text_style: TextStyle) -> CupertinoAlertContentSection {
        self.message_text_style = Some(message_text_style);
        self
    }
}

impl StatelessWidget for CupertinoAlertContentSection {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        debug_assert!(
            self.title.is_none()
                || (self.title_padding.is_some() && self.title_text_style.is_some())
        );
        debug_assert!(
            self.message.is_none()
                || (self.message_padding.is_some() && self.message_text_style.is_some())
        );
        if self.title.is_none() && self.message.is_none() {
            return SingleChildScrollView::new()
                .controller(self.scroll_controller)
                .child(SizedBox::shrink())
                .into_widget();
        }

        let mut title_content_group = Vec::new();
        if let Some(title) = &self.title {
            title_content_group.push(
                Padding::new(self.title_padding.expect("asserted above").into())
                    .child(
                        DefaultTextStyle::new(
                            self.title_text_style.clone().expect("asserted above"),
                            title.clone(),
                        )
                        .text_align(TextAlign::Center),
                    )
                    .into_widget(),
            );
        }
        if let Some(message) = &self.message {
            title_content_group.push(
                Padding::new(self.message_padding.expect("asserted above").into())
                    .child(
                        DefaultTextStyle::new(
                            self.message_text_style.clone().expect("asserted above"),
                            message.clone(),
                        )
                        .text_align(TextAlign::Center),
                    )
                    .into_widget(),
            );
        }

        // Add padding between the widgets if necessary.
        if let Some(additional) = self.additional_padding_between_title_and_message
            && title_content_group.len() > 1
        {
            title_content_group.insert(1, Padding::new(additional.into()).into_widget());
        }

        CupertinoScrollbar::new(
            SingleChildScrollView::new()
                .controller(self.scroll_controller)
                .child(
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .children(title_content_group),
                ),
        )
        .controller(self.scroll_controller)
        .into_widget()
    }
}

// The "actions section" of a [CupertinoAlertDialog].
//
// The `actions` must not be empty.
#[derive(Clone)]
struct CupertinoAlertActionSection {
    // A list of action buttons.
    //
    // This list must not include the dividers between the buttons. If the list
    // is empty, then this widget returns an empty box.
    actions: Vec<WidgetRef>,
    on_pressed_update: PressedUpdateHandler,
    pressed_index: Option<i32>,
    // A scroll controller that can be used to control the scrolling of the
    // actions in the dialog.
    scroll_controller: AnyScrollController,
}

impl CupertinoAlertActionSection {
    fn new(
        actions: Vec<WidgetRef>,
        on_pressed_update: PressedUpdateHandler,
        pressed_index: Option<i32>,
        scroll_controller: AnyScrollController,
    ) -> CupertinoAlertActionSection {
        debug_assert!(!actions.is_empty());
        CupertinoAlertActionSection {
            actions,
            on_pressed_update,
            pressed_index,
            scroll_controller,
        }
    }
}

impl Debug for CupertinoAlertActionSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoAlertActionSection")
            .field("actions", &self.actions.len())
            .field("pressed_index", &self.pressed_index)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for CupertinoAlertActionSection {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let dialog_color = CupertinoDynamicColor::resolve(&K_DIALOG_COLOR, app, context);
        let dialog_pressed_color =
            CupertinoDynamicColor::resolve(&K_DIALOG_PRESSED_COLOR, app, context);
        let divider_color =
            CupertinoDynamicColor::resolve(&CupertinoColors::SEPARATOR, app, context);

        let mut column = Vec::new();
        for (action_index, action) in self.actions.iter().enumerate() {
            let action_index = action_index as i32;
            if action_index != 0 {
                column.push(
                    Divider::new(
                        divider_color.clone(),
                        dialog_color.clone(),
                        self.pressed_index == Some(action_index - 1)
                            || self.pressed_index == Some(action_index),
                    )
                    .into_widget(),
                );
            }
            let on_pressed_update = Rc::clone(&self.on_pressed_update);
            column.push(
                AlertDialogButtonBackground::new(
                    dialog_color.clone(),
                    dialog_pressed_color.clone(),
                    self.pressed_index == Some(action_index),
                    Some(Rc::new(move |app: &mut App, state: bool| {
                        on_pressed_update(app, action_index, state);
                    })),
                    action.clone(),
                )
                .into_widget(),
            );
        }

        CupertinoScrollbar::new(
            SingleChildScrollView::new()
                .controller(self.scroll_controller)
                .child(AlertDialogActionsLayout::new(K_DIVIDER_THICKNESS, column)),
        )
        .controller(self.scroll_controller)
        .into_widget()
    }
}

// Renders the background of a button (both the pressed background and the idle
// background) and reports its state to the parent with `on_press_state_change`.
#[derive(Clone)]
struct AlertDialogButtonBackground {
    /// Called whether the user is holding on this button.
    pressed: bool,
    /// Called when the user taps down or lifts up on the button.
    ///
    /// The boolean value is true if the user is tapping down on the button.
    on_press_state_change: Option<ValueSetter<bool>>,
    idle_color: AnyColor,
    pressed_color: AnyColor,
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    child: WidgetRef,
}

impl AlertDialogButtonBackground {
    fn new<K>(
        idle_color: AnyColor,
        pressed_color: AnyColor,
        pressed: bool,
        on_press_state_change: Option<ValueSetter<bool>>,
        child: impl IntoWidget<K>,
    ) -> AlertDialogButtonBackground {
        AlertDialogButtonBackground {
            pressed,
            on_press_state_change,
            idle_color,
            pressed_color,
            child: child.into_widget(),
        }
    }
}

impl Debug for AlertDialogButtonBackground {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlertDialogButtonBackground")
            .field("pressed", &self.pressed)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for AlertDialogButtonBackground {
    type State = AlertDialogButtonBackgroundState;

    fn create_state(&self) -> AlertDialogButtonBackgroundState {
        AlertDialogButtonBackgroundState {
            state: StateData::new(),
            slide_target: None,
        }
    }
}

/// Dart's `_AlertDialogButtonBackgroundState`.
struct AlertDialogButtonBackgroundState {
    state: StateData<AlertDialogButtonBackground>,
    /// This state as the `SlideTarget` its `MetaData` carries, minted once in `init_state`.
    slide_target: Option<Rc<dyn SlideTarget>>,
}

impl AlertDialogButtonBackgroundState {
    fn emit_vibration(app: &mut App) {
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::Android => HapticFeedback::selection_click(app),
            TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {}
        }
    }
}

impl SlideTarget for Handle<AlertDialogButtonBackgroundState> {
    fn did_enter(&self, app: &mut App, from_pointer_down: bool, inner_enabled: bool) -> bool {
        let this = *self;
        if let Some(callback) = this.widget(app).on_press_state_change.clone() {
            callback(app, inner_enabled);
        }
        if inner_enabled && !from_pointer_down {
            AlertDialogButtonBackgroundState::emit_vibration(app);
        }
        inner_enabled
    }

    fn did_leave(&self, app: &mut App) {
        let this = *self;
        if let Some(callback) = this.widget(app).on_press_state_change.clone() {
            callback(app, false);
        }
    }

    fn did_confirm(&self, app: &mut App) {
        let this = *self;
        if let Some(callback) = this.widget(app).on_press_state_change.clone() {
            callback(app, false);
        }
    }
}

impl State for AlertDialogButtonBackgroundState {
    type Widget = AlertDialogButtonBackground;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).slide_target = Some(Rc::new(self));
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let background_color = if widget.pressed {
            widget.pressed_color
        } else {
            widget.idle_color
        };
        let color = CupertinoDynamicColor::resolve(&background_color, app, context);
        // `MergeSemantics` waits with accessibility.
        MetaData::new()
            .meta_data(slide_target_meta_data(&app.get(self).slide_target))
            .child(
                Container::new()
                    .decoration(BoxDecoration::new().color(color))
                    .child(widget.child),
            )
            .into_widget()
    }
}

/// A button typically used in a [`CupertinoAlertDialog`].
///
/// See also:
///
///  * [`CupertinoAlertDialog`], a dialog that informs the user about situations
///    that require acknowledgment.
#[derive(Clone)]
pub struct CupertinoDialogAction {
    pub key: Option<KeyRef>,
    /// The callback that is called when the button is tapped or otherwise
    /// activated.
    ///
    /// If this is set to null, the button will be disabled.
    pub on_pressed: Option<Listener>,
    /// Set to true if button is the default choice in the dialog.
    ///
    /// Default buttons have bold text. Similar to
    /// [`UIAlertController.preferredAction`](https://developer.apple.com/documentation/uikit/uialertcontroller/1620102-preferredaction),
    /// but more than one action can have this attribute set to true in the same
    /// [`CupertinoAlertDialog`].
    ///
    /// This parameters defaults to false.
    pub is_default_action: bool,
    /// Whether this action destroys an object.
    ///
    /// For example, an action that deletes an email is destructive.
    ///
    /// Defaults to false.
    pub is_destructive_action: bool,
    /// `TextStyle` to apply to any text that appears in this button.
    ///
    /// Dialog actions have a built-in text resizing policy for long text. To
    /// ensure that this resizing policy always works as expected,
    /// [`text_style`](Self::text_style) must be used if a text size is desired other than
    /// that specified in Dart's `_kCupertinoDialogActionStyle`.
    pub text_style: Option<TextStyle>,
    /// The cursor that will be shown when hovering over the button.
    ///
    /// If null, defaults to `SystemMouseCursors::CLICK` on web and
    /// `MouseCursor::defer()` on other platforms.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    pub child: WidgetRef,
}

impl CupertinoDialogAction {
    /// Creates an action for an iOS-style dialog.
    pub fn new<K>(child: impl IntoWidget<K>) -> CupertinoDialogAction {
        CupertinoDialogAction {
            key: None,
            on_pressed: None,
            is_default_action: false,
            is_destructive_action: false,
            text_style: None,
            mouse_cursor: None,
            child: child.into_widget(),
        }
    }

    /// Dart `CupertinoDialogAction(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoDialogAction {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoDialogAction(onPressed:)`.
    pub fn on_pressed(mut self, on_pressed: Listener) -> CupertinoDialogAction {
        self.on_pressed = Some(on_pressed);
        self
    }

    /// Dart `CupertinoDialogAction(isDefaultAction:)`.
    pub fn is_default_action(mut self, is_default_action: bool) -> CupertinoDialogAction {
        self.is_default_action = is_default_action;
        self
    }

    /// Dart `CupertinoDialogAction(isDestructiveAction:)`.
    pub fn is_destructive_action(mut self, is_destructive_action: bool) -> CupertinoDialogAction {
        self.is_destructive_action = is_destructive_action;
        self
    }

    /// Dart `CupertinoDialogAction(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> CupertinoDialogAction {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `CupertinoDialogAction(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> CupertinoDialogAction {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }
}

impl Debug for CupertinoDialogAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoDialogAction")
            .field("enabled", &self.on_pressed.is_some())
            .field("is_default_action", &self.is_default_action)
            .field("is_destructive_action", &self.is_destructive_action)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoDialogAction {
    type State = CupertinoDialogActionState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoDialogActionState {
        CupertinoDialogActionState {
            state: StateData::new(),
            slide_target: None,
        }
    }
}

/// Dart's `_CupertinoDialogActionState`.
pub struct CupertinoDialogActionState {
    state: StateData<CupertinoDialogAction>,
    /// This state as the `SlideTarget` its `MetaData` carries, minted once in `init_state`.
    slide_target: Option<Rc<dyn SlideTarget>>,
}

impl CupertinoDialogActionState {
    // The button is enabled when it has [on_pressed].
    fn enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).on_pressed.is_some()
    }

    // Dialog action content shrinks to fit, up to a certain point, and if it still
    // cannot fit at the minimum size, the text content is ellipsized.
    //
    // This policy only applies when the device is not in accessibility mode.
    fn build_content_with_regular_sizing_policy(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        text_style: TextStyle,
        content: WidgetRef,
        padding: f64,
    ) -> WidgetRef {
        let is_in_accessibility_mode = is_in_accessibility_mode(app, context);
        let dialog_width = if is_in_accessibility_mode {
            K_ACCESSIBILITY_CUPERTINO_DIALOG_WIDTH
        } else {
            K_CUPERTINO_DIALOG_WIDTH
        };
        // The font_size_ratio is the ratio of the current text size (including any
        // iOS scale factor) vs the minimum text size that we allow in action
        // buttons. This ratio information is used to automatically scale down action
        // button text to fit the available space.
        let font_size_ratio = MediaQuery::text_scaler_of(app, context)
            .scale(text_style.font_size.expect("the style carries a font size"))
            / K_DIALOG_MIN_BUTTON_FONT_SIZE;

        // `Semantics(button: true, onTap:)` waits with accessibility.
        FittedBox::new()
            .fit(BoxFit::ScaleDown)
            .child(
                ConstrainedBox::new(
                    BoxConstraints::new()
                        .max_width(font_size_ratio * (dialog_width - (2.0 * padding))),
                )
                .child(
                    DefaultTextStyle::new(text_style, content)
                        .text_align(TextAlign::Center)
                        .overflow(TextOverflow::Ellipsis)
                        .max_lines(1),
                ),
            )
            .into_widget()
    }

    // Dialog action content is permitted to be as large as it wants when in
    // accessibility mode. If text is used as the content, the text wraps instead
    // of ellipsizing.
    fn build_content_with_accessibility_sizing_policy(
        text_style: TextStyle,
        content: WidgetRef,
    ) -> WidgetRef {
        DefaultTextStyle::new(text_style, content)
            .text_align(TextAlign::Center)
            .into_widget()
    }
}

impl SlideTarget for Handle<CupertinoDialogActionState> {
    fn did_enter(&self, app: &mut App, _from_pointer_down: bool, _inner_enabled: bool) -> bool {
        let this = *self;
        this.enabled(app)
    }

    fn did_leave(&self, _app: &mut App) {}

    fn did_confirm(&self, app: &mut App) {
        let this = *self;
        if let Some(on_pressed) = this.widget(app).on_pressed.clone() {
            on_pressed.call(app);
        }
    }
}

impl State for CupertinoDialogActionState {
    type Widget = CupertinoDialogAction;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).slide_target = Some(Rc::new(self));
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let enabled = self.enabled(app);
        let unresolved = if widget.is_destructive_action {
            CupertinoColors::SYSTEM_RED
        } else {
            CupertinoTheme::of(app, context).primary_color()
        };
        let color = CupertinoDynamicColor::resolve(&unresolved, app, context);
        let mut style = cupertino_dialog_action_style()
            .copy_with()
            .color(color)
            .merge(widget.text_style.as_ref());

        if widget.is_default_action {
            style = style.copy_with().font_weight(FontWeight::W600);
        }

        if !enabled {
            // Dart's deprecated `withOpacity` is `with_values(alpha:)`, as its note says.
            let faded = style
                .color
                .clone()
                .expect("the style carries a color")
                .color()
                .with_values(Some(0.5), None, None, None, None);
            style = style.copy_with().color(faded);
        }
        let font_size = style.font_size.unwrap_or(K_DEFAULT_FONT_SIZE);
        let font_size_to_scale = if font_size == 0.0 {
            K_DEFAULT_FONT_SIZE
        } else {
            font_size
        };
        let effective_text_scale =
            MediaQuery::text_scaler_of(app, context).scale(font_size_to_scale) / font_size_to_scale;
        let padding = 8.0 * effective_text_scale;
        // Apply a sizing policy to the action button's content based on whether or
        // not the device is in accessibility mode.
        // TODO(mattcarroll): The following logic is not entirely correct. It is also
        // the case that if content text does not contain a space, it should also
        // wrap instead of ellipsizing. We are consciously not implementing that
        // now due to complexity.
        let sized_content = if is_in_accessibility_mode(app, context) {
            CupertinoDialogActionState::build_content_with_accessibility_sizing_policy(
                style,
                widget.child.clone(),
            )
        } else {
            self.build_content_with_regular_sizing_policy(
                app,
                context,
                style,
                widget.child.clone(),
                padding,
            )
        };

        let cursor = widget.mouse_cursor.unwrap_or_else(|| {
            if enabled && K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                <dyn MouseCursor>::defer()
            }
        });
        MouseRegion::new()
            .cursor(cursor)
            .child(
                MetaData::new()
                    .meta_data(slide_target_meta_data(&app.get(self).slide_target))
                    .behavior(HitTestBehavior::Opaque)
                    .child(
                        ConstrainedBox::new(
                            BoxConstraints::new().min_height(K_DIALOG_MIN_BUTTON_HEIGHT),
                        )
                        .child(
                            Padding::new(EdgeInsetsGeometry::all(padding))
                                .child(Center::new().child(sized_content)),
                        ),
                    ),
            )
            .into_widget()
    }
}

// iOS style dialog action button layout.
//
// [AlertDialogActionsLayout] does not provide any scrolling
// behavior for its buttons. It only handles the sizing and layout of buttons.
// Scrolling behavior can be composed on top of this widget, if desired.
//
// The layout operates in two modes:
//
// 1. Horizontal Mode: If there are exactly two buttons and they fit in a single
//    row, the buttons are rendered side by side with a vertical divider between
//    them.
// 2. Vertical Mode: In all other cases, the buttons are arranged in a column,
//    separated by horizontal dividers.
//
// The `children` parameter must be a non-empty list containing button widgets
// and divider widgets in an alternating sequence. Therefore, the list must have
// an odd length.
#[derive(Clone, Debug)]
struct AlertDialogActionsLayout {
    divider_thickness: f64,
    children: Vec<WidgetRef>,
}

impl AlertDialogActionsLayout {
    fn new(divider_thickness: f64, children: Vec<WidgetRef>) -> AlertDialogActionsLayout {
        AlertDialogActionsLayout {
            divider_thickness,
            children,
        }
    }
}

impl RenderObjectWidget for AlertDialogActionsLayout {
    type RenderObject = RenderAlertDialogActionsLayout;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        RenderAlertDialogActionsLayout::new(app, self.divider_thickness, Some(text_direction))
            .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderAlertDialogActionsLayout>,
    ) {
        let text_direction = Directionality::of(app, context);
        render_object.set_divider_thickness(app, self.divider_thickness);
        RenderFlexMixin::set_text_direction(render_object, app, Some(text_direction));
    }
}

impl MultiChildRenderObjectWidget for AlertDialogActionsLayout {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_RenderAlertDialogActionsLayout extends RenderFlex`.
struct RenderAlertDialogActionsLayout {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    flex: RenderFlexData,
    divider_thickness: f64,
}

impl RenderAlertDialogActionsLayout {
    /// Creates the actions layout of a [`CupertinoAlertDialog`].
    fn new(
        app: &mut App,
        divider_thickness: f64,
        text_direction: Option<TextDirection>,
    ) -> RenderHandle<RenderAlertDialogActionsLayout> {
        let flex = RenderFlexData::new()
            .direction(Axis::Vertical)
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .text_direction(text_direction);
        RenderHandle::new_box(
            app,
            RenderAlertDialogActionsLayout {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                flex,
                divider_thickness,
            },
        )
    }

    /// The thickness of the divider between buttons.
    fn divider_thickness(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).divider_thickness
    }

    /// Sets [`divider_thickness`](Self::divider_thickness).
    fn set_divider_thickness(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if value != self.get(app).divider_thickness {
            self.get_mut(app).divider_thickness = value;
            self.mark_needs_layout(app);
        }
    }

    /// The width each of the two buttons gets in the horizontal layout.
    fn horizontal_slot_width_for(self: RenderHandle<Self>, app: &App, overall_width: f64) -> f64 {
        (overall_width - self.divider_thickness(app)) / 2.0
    }

    fn debug_has_valid_constraints(constraints: BoxConstraints) -> bool {
        if cfg!(debug_assertions) && constraints.max_width.is_infinite() {
            panic!(
                "The incoming width constraints are unbounded.\n\
                 The incoming constraints are: {constraints:?}"
            );
        }
        true
    }

    fn use_horizontal_layout(self: RenderHandle<Self>, app: &mut App, overall_width: f64) -> bool {
        // Horizontal layout only applies to cases of 3 children: 2 action buttons
        // and 1 divider.
        if self.child_count(app) != 3 {
            return false;
        }
        let slot_width = self.horizontal_slot_width_for(app, overall_width);
        let mut child = self.first_child(app).expect("three children");
        loop {
            // If both children fit into a half-row slot, use the horizontal layout.
            // Max intrinsic widths are used here, which, according to
            // `TextPainter::max_intrinsic_width`, allows text to be displayed at their
            // full font size.
            if child.get_max_intrinsic_width(app, f64::INFINITY) > slot_width {
                return false;
            }
            let Some(divider) = self.child_after(app, child) else {
                break;
            };
            child = self.child_after(app, divider).expect("an odd child count");
        }
        true
    }

    fn for_each_slot(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        debug_assert!(self.child_count(app) % 2 == 1);
        let mut slots = Vec::new();
        let mut slot = self.first_child(app).expect("a non-empty child list");
        loop {
            slots.push(slot);
            let Some(divider) = self.child_after(app, slot) else {
                break;
            };
            slot = self.child_after(app, divider).expect("an odd child count");
        }
        slots
    }
}

impl ContainerRenderObjectMixin for RenderAlertDialogActionsLayout {
    type ChildType = AnyRenderBox;
    type ParentDataType = FlexParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderAlertDialogActionsLayout {}

impl RenderFlexMixin for RenderAlertDialogActionsLayout {
    fn flex_data(self: RenderHandle<Self>, app: &App) -> &RenderFlexData {
        &self.get(app).flex
    }

    fn flex_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderFlexData {
        &mut self.get_mut(app).flex
    }
}

impl RenderObject for RenderAlertDialogActionsLayout {
    reveal_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        if self.first_child(app).is_none() {
            let smallest = self.constraints(app).smallest();
            self.set_size(app, smallest);
            return;
        }

        let constraints = self.constraints(app);
        if !RenderAlertDialogActionsLayout::debug_has_valid_constraints(constraints) {
            let smallest = constraints.smallest();
            self.set_size(app, smallest);
            return;
        }

        let overall_width = constraints.max_width;
        if !self.use_horizontal_layout(app, overall_width) {
            RenderFlexMixin::perform_layout(self, app);
            return;
        }

        let slot_width = self.horizontal_slot_width_for(app, overall_width);
        let height = self.get_min_intrinsic_height(app, overall_width);
        self.set_size(app, Size::new(overall_width, height));

        let divider_thickness = self.divider_thickness(app);
        let ltr = RenderFlexMixin::text_direction(self, app) == Some(TextDirection::Ltr);
        let mut slot = self.first_child(app).expect("checked above");
        let mut x = if ltr { 0.0 } else { overall_width - slot_width };
        loop {
            slot.layout(
                app,
                BoxConstraints::tight(Size::new(slot_width, height)),
                true,
            );
            slot.as_object()
                .parent_data_of_mut::<FlexParentData>(app)
                .set_offset(Offset::new(x, 0.0));
            let slot_size = slot.size(app);
            if ltr {
                x += slot_size.width();
            } else {
                x -= slot_size.width();
            }

            let Some(divider) = self.child_after(app, slot) else {
                break;
            };
            divider.layout(
                app,
                BoxConstraints::tight(Size::new(divider_thickness, height)),
                false,
            );
            divider
                .as_object()
                .parent_data_of_mut::<FlexParentData>(app)
                .set_offset(Offset::new(x, 0.0));
            if ltr {
                x += divider_thickness;
            } else {
                x -= divider_thickness;
            }
            slot = self.child_after(app, divider).expect("an odd child count");
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderFlexMixin::paint(self, app, context, offset)
    }
}

impl RenderBox for RenderAlertDialogActionsLayout {
    reveal_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderFlexMixin::setup_parent_data(self, app, child)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if !self.use_horizontal_layout(app, width) {
            return RenderFlexMixin::compute_min_intrinsic_height(self, app, width);
        }

        let slot_width = self.horizontal_slot_width_for(app, width);
        let mut height: f64 = 0.0;
        for slot in self.for_each_slot(app) {
            height = height.max(slot.get_min_intrinsic_height(app, slot_width));
        }
        height
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if !self.use_horizontal_layout(app, width) {
            return RenderFlexMixin::compute_max_intrinsic_height(self, app, width);
        }

        let slot_width = self.horizontal_slot_width_for(app, width);
        let mut height: f64 = 0.0;
        for slot in self.for_each_slot(app) {
            height = height.max(slot.get_max_intrinsic_height(app, slot_width));
        }
        height
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        if !RenderAlertDialogActionsLayout::debug_has_valid_constraints(constraints) {
            return Size::ZERO;
        }

        let overall_width = constraints.max_width;
        if !self.use_horizontal_layout(app, overall_width) {
            return RenderFlexMixin::compute_dry_layout(self, app, constraints);
        }

        let height = self.get_min_intrinsic_height(app, overall_width);
        Size::new(overall_width, height)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderFlexMixin::hit_test_children(self, app, result, position)
    }
}

/// Dart's `typedef _TwoChildrenHeights = ({double topChildHeight, double bottomChildHeight})`.
struct TwoChildrenHeights {
    top_child_height: f64,
    bottom_child_height: f64,
}

// A column layout with two widgets, where the top widget expands vertically as
// needed, and the bottom widget has a minimum height.
//
// Both child widgets stretch horizontally to the parent's maximum width
// constraint, with vertical space allocated in this priority:
//
//  1. The `bottom` widget receives its requested height, up to a
//     `bottom_max_height` limit and the container's constraint.
//  2. The `top` widget receives its requested height, up to the remaining space
//     in the container.
//  3. The `bottom` widget receives its requested height, up to any remaining
//     space in the container.
//
// This mirrors the behavior seen in iOS components like action sheets and
// alerts.
//
// Implementing this layout with simple compositing widgets is challenging
// because:
//
//  * The bottom widget should take more than `bottom_min_height` if the top
//    widget is short.
//  * The bottom widget should take less than `bottom_min_height` if it is
//    naturally shorter.
#[derive(Clone, Debug)]
struct PriorityColumn {
    bottom_min_height: f64,
    children: Vec<WidgetRef>,
}

impl PriorityColumn {
    fn new<T, B, KT, KB>(top: T, bottom: B, bottom_min_height: f64) -> PriorityColumn
    where
        T: IntoWidget<KT>,
        B: IntoWidget<KB>,
    {
        PriorityColumn {
            bottom_min_height,
            children: vec![top.into_widget(), bottom.into_widget()],
        }
    }
}

impl RenderObjectWidget for PriorityColumn {
    type RenderObject = RenderPriorityColumn;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderPriorityColumn::new(app, self.bottom_min_height).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderPriorityColumn>,
    ) {
        render_object.set_bottom_min_height(app, self.bottom_min_height);
    }
}

impl MultiChildRenderObjectWidget for PriorityColumn {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_RenderPriorityColumn extends RenderFlex`.
struct RenderPriorityColumn {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    flex: RenderFlexData,
    bottom_min_height: f64,
}

impl RenderPriorityColumn {
    /// Creates the two-child column of an alert dialog or action sheet.
    fn new(app: &mut App, bottom_min_height: f64) -> RenderHandle<RenderPriorityColumn> {
        let flex = RenderFlexData::new()
            .direction(Axis::Vertical)
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        RenderHandle::new_box(
            app,
            RenderPriorityColumn {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                flex,
                bottom_min_height,
            },
        )
    }

    /// The height the bottom child is given as long as it needs that much.
    fn bottom_min_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).bottom_min_height
    }

    /// Sets [`bottom_min_height`](Self::bottom_min_height).
    fn set_bottom_min_height(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if value != self.get(app).bottom_min_height {
            self.get_mut(app).bottom_min_height = value;
            self.mark_needs_layout(app);
        }
    }

    fn children_heights(
        self: RenderHandle<Self>,
        app: &mut App,
        width: f64,
        max_height: f64,
    ) -> TwoChildrenHeights {
        debug_assert!(self.child_count(app) == 2);
        let first_child = self.first_child(app).expect("two children");
        let last_child = self.last_child(app).expect("two children");
        let top_intrinsic = first_child.get_min_intrinsic_height(app, width);
        let bottom_intrinsic = last_child.get_min_intrinsic_height(app, width);
        // Try to layout both children as their intrinsic height.
        if top_intrinsic + bottom_intrinsic <= max_height {
            return TwoChildrenHeights {
                top_child_height: top_intrinsic,
                bottom_child_height: bottom_intrinsic,
            };
        }
        // bottom_min_height is only effective when bottom actually needs that much.
        let effective_bottom_min_height = self.bottom_min_height(app).min(bottom_intrinsic);
        // Try to layout top as intrinsics, as long as the bottom has at least
        // effective_bottom_min_height.
        if max_height - top_intrinsic >= effective_bottom_min_height {
            return TwoChildrenHeights {
                top_child_height: top_intrinsic,
                bottom_child_height: max_height - top_intrinsic,
            };
        }
        // Try to layout bottom as effective_bottom_min_height, as long as top has at
        // least 0.
        if max_height >= effective_bottom_min_height {
            return TwoChildrenHeights {
                top_child_height: max_height - effective_bottom_min_height,
                bottom_child_height: effective_bottom_min_height,
            };
        }
        TwoChildrenHeights {
            top_child_height: 0.0,
            bottom_child_height: max_height,
        }
    }
}

impl ContainerRenderObjectMixin for RenderPriorityColumn {
    type ChildType = AnyRenderBox;
    type ParentDataType = FlexParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderPriorityColumn {}

impl RenderFlexMixin for RenderPriorityColumn {
    fn flex_data(self: RenderHandle<Self>, app: &App) -> &RenderFlexData {
        &self.get(app).flex
    }

    fn flex_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderFlexData {
        &mut self.get_mut(app).flex
    }
}

impl RenderObject for RenderPriorityColumn {
    reveal_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let width = constraints.max_width;
        let max_height = constraints.max_height;
        let heights = self.children_heights(app, width, max_height);
        self.set_size(
            app,
            Size::new(
                width,
                heights.top_child_height + heights.bottom_child_height,
            ),
        );

        let first_child = self.first_child(app).expect("two children");
        let last_child = self.last_child(app).expect("two children");
        first_child.layout(
            app,
            BoxConstraints::tight(Size::new(width, heights.top_child_height)),
            true,
        );
        first_child
            .as_object()
            .parent_data_of_mut::<FlexParentData>(app)
            .set_offset(Offset::ZERO);

        last_child.layout(
            app,
            BoxConstraints::tight(Size::new(width, heights.bottom_child_height)),
            true,
        );
        last_child
            .as_object()
            .parent_data_of_mut::<FlexParentData>(app)
            .set_offset(Offset::new(0.0, heights.top_child_height));
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderFlexMixin::paint(self, app, context, offset)
    }
}

impl RenderBox for RenderPriorityColumn {
    reveal_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderFlexMixin::setup_parent_data(self, app, child)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        debug_assert!(self.child_count(app) == 2);
        let first_child = self.first_child(app).expect("two children");
        let last_child = self.last_child(app).expect("two children");
        first_child.get_min_intrinsic_height(app, width)
            + last_child.get_min_intrinsic_height(app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        debug_assert!(self.child_count(app) == 2);
        let first_child = self.first_child(app).expect("two children");
        let last_child = self.last_child(app).expect("two children");
        first_child.get_max_intrinsic_height(app, width)
            + last_child.get_max_intrinsic_height(app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let width = constraints.max_width;
        let max_height = constraints.max_height;
        let heights = self.children_heights(app, width, max_height);
        Size::new(
            width,
            heights.top_child_height + heights.bottom_child_height,
        )
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderFlexMixin::hit_test_children(self, app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;

    use reveal_embedder::{
        Locale, PointerChange, PointerData, PointerDataPacket, Rect, TextDirection,
    };
    use reveal_painting::TextScaler;
    use reveal_rendering::RenderDecoratedBox;
    use reveal_widgets::{
        Builder, DefaultWidgetsLocalizations, GlobalKey, Localizations, MediaQueryData,
    };

    use super::*;
    use crate::localizations::DefaultCupertinoLocalizations;
    use crate::test_support::{build, pump, test_cell};

    fn key_of(key: &GlobalKey) -> KeyRef {
        Rc::new(key.clone())
    }

    /// The first descendant render object of type `T`, from `node` down.
    fn find<T: RenderObject>(app: &App, node: AnyRenderObject) -> Option<RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find::<T>(app, child);
            }
        });
        found
    }

    fn for_each_render_object(
        app: &App,
        node: AnyRenderObject,
        visitor: &mut dyn FnMut(&App, AnyRenderObject),
    ) {
        visitor(app, node);
        node.visit_children(app, &mut |child| {
            for_each_render_object(app, child, visitor);
        });
    }

    fn root(app: &mut App, key: &GlobalKey) -> AnyRenderObject {
        key.current_context(app)
            .expect("the widget mounted")
            .find_render_object(app)
            .expect("the widget has been laid out")
    }

    fn box_of(app: &mut App, key: &GlobalKey) -> AnyRenderBox {
        root(app, key).as_box().expect("a box")
    }

    /// The keyed box's rectangle in the view's coordinate system.
    fn rect_of(app: &mut App, key: &GlobalKey) -> Rect {
        let render_box = box_of(app, key);
        let origin = render_box.local_to_global(app, Offset::ZERO, None);
        origin & render_box.size(app)
    }

    fn mount<K>(cell: &AppCell, child: impl IntoWidget<K>) {
        build(
            cell,
            Localizations::new(
                Locale::new("en"),
                vec![
                    DefaultWidgetsLocalizations::delegate(),
                    DefaultCupertinoLocalizations::delegate(),
                ],
            )
            .child(Directionality::new(TextDirection::Ltr, child))
            .into_widget(),
        );
    }

    /// Mounts under an ambient text scaler, which is what puts a dialog in accessibility mode.
    fn mount_scaled<K>(cell: &AppCell, scale: f64, child: impl IntoWidget<K>) {
        mount(
            cell,
            MediaQuery::new(
                MediaQueryData::new().text_scaler(TextScaler::linear(scale)),
                child,
            )
            .into_widget(),
        );
    }

    /// Records the text style in force where the dialog places an action's content.
    fn probe(seen: &Rc<RefCell<Vec<TextStyle>>>) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            seen.borrow_mut()
                .push(DefaultTextStyle::of(app, context).style);
            SizedBox::new().width(40.0).height(20.0).into_widget()
        })
    }

    fn slot(key: &GlobalKey, width: f64, height: f64) -> SizedBox {
        SizedBox::new().key(key_of(key)).width(width).height(height)
    }

    /// Every `BoxDecoration` colour painted under `key`'s render object.
    fn decoration_colors(app: &mut App, key: &GlobalKey) -> Vec<Color> {
        let node = root(app, key);
        let mut colors = Vec::new();
        for_each_render_object(app, node, &mut |app, object| {
            if let Some(decorated) = object.downcast::<RenderDecoratedBox>(app)
                && let Some(decoration) = decorated
                    .decoration(app)
                    .as_any()
                    .downcast_ref::<BoxDecoration>()
                && let Some(color) = &decoration.color
            {
                colors.push(color.color());
            }
        });
        colors
    }

    /// Sends one pointer packet; the test view is 2x, so logical coordinates double.
    fn send(app: &mut App, change: PointerChange, at: Offset, previous: Offset) {
        GestureBinding::instance(app).handle_pointer_data_packet(
            app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Touch,
                time_stamp: Duration::ZERO,
                pointer_identifier: 1,
                physical_x: at.dx() * 2.0,
                physical_y: at.dy() * 2.0,
                physical_delta_x: (at.dx() - previous.dx()) * 2.0,
                physical_delta_y: (at.dy() - previous.dy()) * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
    }

    #[test]
    fn an_alert_dialog_stacks_its_content_section_over_its_actions_at_the_dialog_width() {
        let cell = test_cell();
        let (dialog, title) = (GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoAlertDialog::new()
                .key(key_of(&dialog))
                .title(slot(&title, 100.0, 30.0))
                .content(SizedBox::new().width(100.0).height(20.0))
                .actions([
                    CupertinoDialogAction::new(SizedBox::new().width(40.0).height(20.0))
                        .into_widget(),
                ]),
        );
        let mut app = cell.borrow_mut();

        let node = root(&mut app, &dialog);
        let actions = find::<RenderAlertDialogActionsLayout>(&app, node)
            .expect("the actions section is laid out");
        assert_eq!(
            actions.size(&app).width(),
            K_CUPERTINO_DIALOG_WIDTH,
            "the dialog is 270 wide"
        );
        let actions_top = actions
            .as_box()
            .local_to_global(&app, Offset::ZERO, None)
            .dy();
        let title_rect = rect_of(&mut app, &title);
        assert!(
            actions_top >= title_rect.bottom,
            "the actions sit below the title, {actions_top} vs {}",
            title_rect.bottom
        );
    }

    #[test]
    fn an_alert_dialog_in_accessibility_mode_is_wider() {
        let cell = test_cell();
        let dialog = GlobalKey::new();
        mount_scaled(
            &cell,
            2.0,
            CupertinoAlertDialog::new()
                .key(key_of(&dialog))
                .title(SizedBox::new().width(100.0).height(30.0))
                .actions([
                    CupertinoDialogAction::new(SizedBox::new().width(40.0).height(20.0))
                        .into_widget(),
                ]),
        );
        let mut app = cell.borrow_mut();

        let node = root(&mut app, &dialog);
        let actions = find::<RenderAlertDialogActionsLayout>(&app, node)
            .expect("the actions section is laid out");
        assert_eq!(
            actions.size(&app).width(),
            K_ACCESSIBILITY_CUPERTINO_DIALOG_WIDTH,
            "an accessibility-mode dialog is 310 wide"
        );
    }

    #[test]
    fn two_actions_sit_side_by_side_and_three_stack_vertically() {
        let cell = test_cell();
        let (first, second) = (GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoAlertDialog::new()
                .title(SizedBox::new().width(100.0).height(30.0))
                .actions([
                    CupertinoDialogAction::new(slot(&first, 40.0, 20.0)).into_widget(),
                    CupertinoDialogAction::new(slot(&second, 40.0, 20.0)).into_widget(),
                ]),
        );
        let mut app = cell.borrow_mut();
        let (first_rect, second_rect) = (rect_of(&mut app, &first), rect_of(&mut app, &second));
        assert_eq!(first_rect.top, second_rect.top, "two actions share a row");
        assert!(
            first_rect.left < second_rect.left,
            "the first action is to the left"
        );

        let cell = crate::test_support::test_cell();

        let (first, second, third) = (GlobalKey::new(), GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoAlertDialog::new()
                .title(SizedBox::new().width(100.0).height(30.0))
                .actions([
                    CupertinoDialogAction::new(slot(&first, 40.0, 20.0)).into_widget(),
                    CupertinoDialogAction::new(slot(&second, 40.0, 20.0)).into_widget(),
                    CupertinoDialogAction::new(slot(&third, 40.0, 20.0)).into_widget(),
                ]),
        );
        let mut app = cell.borrow_mut();
        let (first_rect, second_rect, third_rect) = (
            rect_of(&mut app, &first),
            rect_of(&mut app, &second),
            rect_of(&mut app, &third),
        );
        assert!(
            first_rect.top < second_rect.top && second_rect.top < third_rect.top,
            "three actions stack"
        );
    }

    #[test]
    fn a_default_action_is_bold_and_a_destructive_action_is_red() {
        let cell = test_cell();
        let seen = Rc::new(RefCell::new(Vec::new()));
        mount(
            &cell,
            CupertinoAlertDialog::new().actions([
                CupertinoDialogAction::new(probe(&seen))
                    .on_pressed(Listener::new(|_app| {}))
                    .is_default_action(true)
                    .into_widget(),
                CupertinoDialogAction::new(probe(&seen))
                    .on_pressed(Listener::new(|_app| {}))
                    .is_destructive_action(true)
                    .into_widget(),
            ]),
        );
        let seen = seen.borrow();
        assert_eq!(seen.len(), 2, "one style per action");
        assert_eq!(seen[0].font_weight, Some(FontWeight::W600));
        assert_eq!(seen[1].font_weight, Some(FontWeight::W400));
        assert_eq!(
            seen[1].color.as_ref().map(|color| color.color()),
            Some(CupertinoColors::SYSTEM_RED.color()),
            "a destructive action is red"
        );
    }

    #[test]
    fn holding_an_alert_dialog_action_paints_the_pressed_colour() {
        let cell = test_cell();
        let (dialog, action) = (GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoAlertDialog::new()
                .key(key_of(&dialog))
                .title(SizedBox::new().width(100.0).height(30.0))
                .actions([CupertinoDialogAction::new(slot(&action, 40.0, 20.0))
                    .on_pressed(Listener::new(|_app| {}))
                    .into_widget()]),
        );
        let mut app = cell.borrow_mut();
        let pressed = K_DIALOG_PRESSED_COLOR.color();
        assert!(
            !decoration_colors(&mut app, &dialog).contains(&pressed),
            "nothing is pressed yet"
        );

        let center = rect_of(&mut app, &action).center();
        send(&mut app, PointerChange::Down, center, center);
        let state = dialog
            .current_state::<CupertinoAlertDialogState>(&mut app)
            .expect("the dialog is mounted");
        assert_eq!(app.get(state).pressed_index, Some(0));
        pump(&mut app, Duration::ZERO);
        assert!(
            decoration_colors(&mut app, &dialog).contains(&pressed),
            "the held action paints the pressed colour once the press rebuilds"
        );

        send(&mut app, PointerChange::Up, center, center);
        assert_eq!(app.get(state).pressed_index, None);
    }

    #[test]
    fn an_action_sheets_cancel_button_sits_below_the_main_sheet_with_a_gap() {
        let cell = test_cell();
        let (action, cancel) = (GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoActionSheet::new()
                .actions([CupertinoActionSheetAction::new(
                    Listener::new(|_app| {}),
                    SizedBox::new()
                        .key(key_of(&action))
                        .width(40.0)
                        .height(20.0),
                )
                .into_widget()])
                .cancel_button(CupertinoActionSheetAction::new(
                    Listener::new(|_app| {}),
                    SizedBox::new()
                        .key(key_of(&cancel))
                        .width(40.0)
                        .height(20.0),
                )),
        );
        let mut app = cell.borrow_mut();

        let action_rect = rect_of(&mut app, &action);
        let cancel_rect = rect_of(&mut app, &cancel);
        assert!(
            cancel_rect.top > action_rect.bottom,
            "the cancel button is below the sheet"
        );
        let node = root(&mut app, &action);
        let main_sheet = find::<RenderPriorityColumn>(&app, node);
        assert!(main_sheet.is_none(), "one action needs no priority column");
    }

    #[test]
    fn dragging_across_action_sheet_buttons_moves_the_pressed_index() {
        let cell = test_cell();
        let (sheet, first, second) = (GlobalKey::new(), GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoActionSheet::new().key(key_of(&sheet)).actions([
                CupertinoActionSheetAction::new(Listener::new(|_app| {}), slot(&first, 40.0, 20.0))
                    .into_widget(),
                CupertinoActionSheetAction::new(
                    Listener::new(|_app| {}),
                    slot(&second, 40.0, 20.0),
                )
                .into_widget(),
            ]),
        );
        let mut app = cell.borrow_mut();

        let state = sheet
            .current_state::<CupertinoActionSheetState>(&mut app)
            .expect("the sheet is mounted");
        let first_center = rect_of(&mut app, &first).center();
        let second_center = rect_of(&mut app, &second).center();

        send(&mut app, PointerChange::Down, first_center, first_center);
        assert_eq!(app.get(state).pressed_index, Some(0));

        send(&mut app, PointerChange::Move, second_center, first_center);
        assert_eq!(
            app.get(state).pressed_index,
            Some(1),
            "the drag moved onto the second action"
        );

        send(&mut app, PointerChange::Up, second_center, second_center);
        assert_eq!(app.get(state).pressed_index, None);
    }

    #[test]
    fn the_popup_surface_composes_saturation_with_the_blur() {
        let surface = CupertinoPopupSurface::new(SizedBox::shrink());
        let filter = surface
            .build_filter(Some(Brightness::Light))
            .expect("a filter with the default blur");
        let ImageFilterConfig::Compose { outer, inner } = &filter else {
            panic!("expected a composed filter, got {filter:?}");
        };
        assert_eq!(
            **outer,
            ImageFilterConfig::blur()
                .sigma_x(CupertinoPopupSurface::DEFAULT_BLUR_SIGMA)
                .sigma_y(CupertinoPopupSurface::DEFAULT_BLUR_SIGMA)
        );
        assert_eq!(
            **inner,
            ImageFilterConfig::new(ImageFilter::Color(ColorFilter::Matrix(
                CupertinoPopupSurface::LIGHT_SATURATION_MATRIX
            )))
        );

        let dark = CupertinoPopupSurface::new(SizedBox::shrink())
            .build_filter(Some(Brightness::Dark))
            .expect("a filter with the default blur");
        let ImageFilterConfig::Compose { inner, .. } = &dark else {
            panic!("expected a composed filter");
        };
        assert_eq!(
            **inner,
            ImageFilterConfig::new(ImageFilter::Color(ColorFilter::Matrix(
                CupertinoPopupSurface::DARK_SATURATION_MATRIX
            )))
        );
    }

    #[test]
    fn a_popup_surface_without_blur_is_a_plain_saturation_and_without_vibrance_a_plain_blur() {
        let filter = CupertinoPopupSurface::new(SizedBox::shrink())
            .blur_sigma(0.0)
            .build_filter(Some(Brightness::Light))
            .expect("a saturation filter");
        assert_eq!(
            filter,
            ImageFilterConfig::new(ImageFilter::Color(ColorFilter::Matrix(
                CupertinoPopupSurface::LIGHT_SATURATION_MATRIX
            )))
        );

        set_debug_is_vibrance_painted(false);
        let plain = CupertinoPopupSurface::new(SizedBox::shrink())
            .build_filter(Some(Brightness::Light))
            .expect("a blur filter");
        assert_eq!(
            plain,
            ImageFilterConfig::blur()
                .sigma_x(CupertinoPopupSurface::DEFAULT_BLUR_SIGMA)
                .sigma_y(CupertinoPopupSurface::DEFAULT_BLUR_SIGMA)
        );
        assert!(
            CupertinoPopupSurface::new(SizedBox::shrink())
                .blur_sigma(0.0)
                .build_filter(Some(Brightness::Light))
                .is_none(),
            "no vibrance and no blur leaves no filter"
        );
        set_debug_is_vibrance_painted(true);
    }
}
