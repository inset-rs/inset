//! Flutter counterpart: material_ui's `dialog.dart`, with its parts
//! `generated/dialog_defaults_m3.g.dart` and `generated/dialog_fullscreen_defaults_m3.g.dart`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationStyle, AnimationWithParent, AnyAnimation, Curve, CurvedAnimation, Curves,
};
use waft_cupertino::{CupertinoAlertDialog, show_cupertino_dialog};
use waft_embedder::{Brightness, Clip, Color, SemanticsHitTestBehavior, SemanticsRole};
use waft_embedder::{TargetPlatform, TextAlign, lerp_double};
use waft_foundation::{App, CompleterFuture, Handle};
use waft_painting::{
    AlignmentGeometry, EdgeInsets, EdgeInsetsGeometry, K_DEFAULT_FONT_SIZE, RoundedRectangleBorder,
    ShapeBorderRef, TextStyle, VerticalDirection,
};
use waft_rendering::{BoxConstraints, CrossAxisAlignment, MainAxisAlignment, MainAxisSize};
use waft_widgets::{
    Align, AnimatedPadding, AnyRoute, AnyScrollController, BuildContext, Builder, CapturedThemes,
    Column, ConstrainedBox, DefaultTextStyle, Directionality, FadeTransition, Flexible,
    HeroControllerScope, IconTheme, IconThemeData, InheritedTheme, IntoWidget, IntrinsicWidth,
    KeyRef, ListBody, LocalHistoryRoute, LocalHistoryRouteData, MediaQuery, ModalRoute,
    ModalRouteData, Navigator, OverflowBar, OverflowBarAlignment, OverlayRoute, OverlayRouteData,
    Padding, Page, PageRef, PageRouteBuilder, PopScope, PopupRoute, PredictiveBackRoute,
    RawDialogRoute, RawDialogRouteBuilder, RawDialogRouteData, RawDialogRouteLeaf, Route,
    RouteData, RoutePageBuilder, RouteResult, RouteSettingsRef, SafeArea, Semantics,
    SingleChildScrollView, State, StatelessWidget, TransitionRoute, TransitionRouteData,
    TraversalEdgeBehavior, WidgetBuilder, WidgetRef, show_raw_dialog,
};

use crate::colors::Colors;
use crate::debug::debug_check_has_material_localizations;
use crate::dialog_theme::DialogThemeDataGetters;
use crate::dialog_theme::{DialogTheme, DialogThemeData};
use crate::generated::dialog_defaults_m3::DialogDefaultsM3;
use crate::generated::dialog_fullscreen_defaults_m3::DialogFullscreenDefaultsM3;
use crate::ink_well::InkWell;
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;

const DEFAULT_INSET_PADDING: EdgeInsets = EdgeInsets::symmetric(24.0, 40.0);

/// A Material Design dialog.
///
/// This dialog widget does not have any opinion about the contents of the
/// dialog. Rather than using this widget directly, consider using [`AlertDialog`]
/// or [`SimpleDialog`], which implement specific kinds of Material Design
/// dialogs.
///
/// ## Constraints
/// The Material 3 guideline recommends that a dialog should have a maximal width of 560dp.
/// For historical reasons, Flutter's [`Dialog`] widget does not come with this constraint by
/// default. For applications targeting large screens such as desktop or Web, it is recommended
/// to set the [`constraints`](Self::constraints) property.
///
/// One set of setters serves both of Dart's constructors, [`Dialog::new`] and
/// [`Dialog::fullscreen`].
///
/// See also:
///
///  * [`AlertDialog`], for dialogs that have a message and some buttons.
///  * [`SimpleDialog`], for dialogs that offer a variety of options.
///  * [`show_dialog`], which actually displays the dialog and returns its result.
///  * <https://material.io/design/components/dialogs.html>
#[derive(Debug)]
pub struct Dialog {
    pub key: Option<KeyRef>,

    /// The background color of the surface of this [`Dialog`].
    ///
    /// This sets the `Material::color` on this [`Dialog`]'s `Material`.
    ///
    /// If null, then the [`DialogThemeData::background_color`] is used. If that is
    /// also null, defaults to `ColorScheme::surface_container_high`. If
    /// `ThemeData::use_material3` is false, defaults to `Colors::GREY` with a shade
    /// of 800 in dark theme and `Colors::WHITE` in light theme.
    ///
    /// If [`Dialog::fullscreen`] is used, defaults to `ColorScheme::surface`.
    pub background_color: Option<Color>,

    /// The z-coordinate of this [`Dialog`].
    ///
    /// Controls how far above the parent the dialog will appear. Elevation is
    /// represented by a drop shadow if [`shadow_color`](Self::shadow_color) is non null,
    /// and a surface tint overlay on the background color if
    /// [`surface_tint_color`](Self::surface_tint_color) is non null.
    ///
    /// If null then [`DialogThemeData::elevation`] is used, and if that is null then
    /// the elevation will match the Material Design specification for Dialogs.
    pub elevation: Option<f64>,

    /// The color used to paint a drop shadow under the dialog's `Material`,
    /// which reflects the dialog's [`elevation`](Self::elevation).
    ///
    /// If null and `ThemeData::use_material3` is true then no drop shadow will
    /// be rendered.
    ///
    /// If null and `ThemeData::use_material3` is false then it will default to
    /// `ThemeData::shadow_color`.
    pub shadow_color: Option<Color>,

    /// The color used as a surface tint overlay on the dialog's background color,
    /// which reflects the dialog's [`elevation`](Self::elevation).
    ///
    /// If `ThemeData::use_material3` is false property has no effect.
    ///
    /// defaults to `Colors::TRANSPARENT`.
    ///
    /// To disable this feature, set [`surface_tint_color`](Self::surface_tint_color) to
    /// `Colors::TRANSPARENT`.
    pub surface_tint_color: Option<Color>,

    /// The duration of the animation to show when the system keyboard intrudes
    /// into the space that the dialog is placed in.
    ///
    /// Defaults to 100 milliseconds.
    pub inset_animation_duration: Duration,

    /// The curve to use for the animation shown when the system keyboard intrudes
    /// into the space that the dialog is placed in.
    ///
    /// Defaults to `Curves::decelerate`.
    pub inset_animation_curve: Rc<dyn Curve>,

    /// The amount of padding added to `MediaQueryData::view_insets` on the outside
    /// of the dialog. This defines the minimum space between the screen's edges
    /// and the dialog.
    ///
    /// Defaults to `EdgeInsets.symmetric(horizontal: 40.0, vertical: 24.0)`.
    pub inset_padding: Option<EdgeInsets>,

    /// Controls how the contents of the dialog are clipped (or not) to the given
    /// [`shape`](Self::shape).
    ///
    /// See the enum [`Clip`] for details of all possible options and their common
    /// use cases.
    ///
    /// If null, then [`DialogThemeData::clip_behavior`] is used. If that is also null,
    /// defaults to [`Clip::None`].
    pub clip_behavior: Option<Clip>,

    /// The shape of this dialog's border.
    ///
    /// Defines the dialog's `Material::shape`.
    ///
    /// The default shape is a [`RoundedRectangleBorder`] with a radius of 4.0
    pub shape: Option<ShapeBorderRef>,

    /// How to align the [`Dialog`].
    ///
    /// If null, then [`DialogThemeData::alignment`] is used. If that is also null, the
    /// default is `Alignment::CENTER`.
    pub alignment: Option<AlignmentGeometry>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// This value is used to determine if this is a fullscreen dialog.
    fullscreen: bool,

    /// The role this dialog represent in assist technologies.
    ///
    /// Defaults to [`SemanticsRole::Dialog`].
    pub semantics_role: SemanticsRole,

    /// Constrains the size of the dialog.
    ///
    /// If null, then [`DialogThemeData::constraints`] is used. If that is also null, the
    /// default is `const BoxConstraints(minWidth: 280.0)`.
    pub constraints: Option<BoxConstraints>,
}

impl Dialog {
    /// Creates a dialog.
    ///
    /// Typically used in conjunction with [`show_dialog`].
    pub fn new() -> Dialog {
        Dialog {
            key: None,
            background_color: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            inset_animation_duration: Duration::from_millis(100),
            inset_animation_curve: Curves::decelerate(),
            inset_padding: None,
            clip_behavior: None,
            shape: None,
            alignment: None,
            child: None,
            fullscreen: false,
            semantics_role: SemanticsRole::Dialog,
            constraints: None,
        }
    }

    /// Creates a fullscreen dialog.
    ///
    /// Typically used in conjunction with [`show_dialog`].
    ///
    /// The dialog's `Material` covers the whole screen, including the area
    /// behind the software keyboard, while [`child`](Self::child) is kept above the keyboard,
    /// so that a translucent keyboard shows the dialog's
    /// [`background_color`](Self::background_color) rather than what is behind the dialog.
    pub fn fullscreen() -> Dialog {
        Dialog {
            key: None,
            background_color: None,
            elevation: Some(0.0),
            shadow_color: None,
            surface_tint_color: None,
            inset_animation_duration: Duration::ZERO,
            inset_animation_curve: Curves::decelerate(),
            inset_padding: Some(EdgeInsets::ZERO),
            clip_behavior: Some(Clip::None),
            shape: None,
            alignment: None,
            child: None,
            fullscreen: true,
            semantics_role: SemanticsRole::Dialog,
            constraints: None,
        }
    }

    /// Dart `Dialog(key:)`.
    pub fn key(mut self, key: KeyRef) -> Dialog {
        self.key = Some(key);
        self
    }

    /// Dart `Dialog(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> Dialog {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `Dialog(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> Dialog {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `Dialog(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> Dialog {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `Dialog(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> Dialog {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `Dialog(insetAnimationDuration:)`.
    pub fn inset_animation_duration(mut self, inset_animation_duration: Duration) -> Dialog {
        self.inset_animation_duration = inset_animation_duration;
        self
    }

    /// Dart `Dialog(insetAnimationCurve:)`.
    pub fn inset_animation_curve(mut self, inset_animation_curve: Rc<dyn Curve>) -> Dialog {
        self.inset_animation_curve = inset_animation_curve;
        self
    }

    /// Dart `Dialog(insetPadding:)`.
    pub fn inset_padding(mut self, inset_padding: EdgeInsets) -> Dialog {
        self.inset_padding = Some(inset_padding);
        self
    }

    /// Dart `Dialog(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Dialog {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `Dialog(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> Dialog {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `Dialog(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> Dialog {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `Dialog(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Dialog {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `Dialog(semanticsRole:)`.
    pub fn semantics_role(mut self, semantics_role: SemanticsRole) -> Dialog {
        self.semantics_role = semantics_role;
        self
    }

    /// Dart `Dialog(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> Dialog {
        self.constraints = Some(constraints);
        self
    }

    // Keeps [child] out of the view insets, such as the software keyboard, which
    // the child then no longer needs to account for. [padding] is the view
    // insets plus the dialog's inset padding.
    fn avoid_view_insets(
        &self,
        app: &mut App,
        context: BuildContext,
        padding: EdgeInsets,
        child: WidgetRef,
    ) -> WidgetRef {
        AnimatedPadding::new(padding.into(), self.inset_animation_duration)
            .curve(Rc::clone(&self.inset_animation_curve))
            .child(
                MediaQuery::remove_view_insets(app, context, true, true, true, true, child)
                    .into_widget(),
            )
            .into_widget()
    }
}

impl Default for Dialog {
    fn default() -> Dialog {
        Dialog::new()
    }
}

impl StatelessWidget for Dialog {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let dialog_theme = DialogTheme::of(app, context);
        let effective_padding = MediaQuery::view_insets_of(app, context)
            + self
                .inset_padding
                .or(dialog_theme.inset_padding)
                .unwrap_or(DEFAULT_INSET_PADDING);
        let defaults: Box<dyn DialogThemeDataGetters> = if theme.use_material3 {
            if self.fullscreen {
                Box::new(DialogFullscreenDefaultsM3::new(context))
            } else {
                Box::new(DialogDefaultsM3::new(context))
            }
        } else {
            Box::new(DialogDefaultsM2::new(context))
        };

        let box_constraints = self
            .constraints
            .or(dialog_theme.constraints)
            .unwrap_or(BoxConstraints::new().min_width(280.0));

        let dialog_child = if self.fullscreen {
            // The Material keeps covering the whole screen, including the area
            // behind the keyboard, so that a translucent keyboard shows the dialog's
            // surface rather than the modal barrier behind it. Only the content is
            // kept above the keyboard.
            let mut material = Material::new();
            material.color = self
                .background_color
                .or(dialog_theme.background_color)
                .or_else(|| defaults.background_color(app));
            material.child = self
                .child
                .clone()
                .map(|child| self.avoid_view_insets(app, context, effective_padding, child));
            material.into_widget()
        } else {
            let mut material = Material::new()
                .elevation(
                    self.elevation
                        .or(dialog_theme.elevation)
                        .or_else(|| defaults.elevation(app))
                        .expect("the defaults give an elevation"),
                )
                .shape(
                    self.shape
                        .clone()
                        .or(dialog_theme.shape.clone())
                        .or_else(|| defaults.shape(app))
                        .expect("the defaults give a shape"),
                )
                .r#type(MaterialType::Card)
                .clip_behavior(
                    self.clip_behavior
                        .or(dialog_theme.clip_behavior)
                        .or_else(|| defaults.clip_behavior(app))
                        .expect("the defaults give a clip behavior"),
                );
            material.color = self
                .background_color
                .or(dialog_theme.background_color)
                .or_else(|| defaults.background_color(app));
            material.shadow_color = self
                .shadow_color
                .or(dialog_theme.shadow_color)
                .or_else(|| defaults.shadow_color(app));
            material.surface_tint_color = self
                .surface_tint_color
                .or(dialog_theme.surface_tint_color)
                .or_else(|| defaults.surface_tint_color(app));
            material.child = self.child.clone();
            let aligned = Align::new()
                .alignment(
                    self.alignment
                        .or(dialog_theme.alignment)
                        .or_else(|| defaults.alignment(app))
                        .expect("the defaults give an alignment"),
                )
                .child(ConstrainedBox::new(box_constraints).child(material))
                .into_widget();
            self.avoid_view_insets(app, context, effective_padding, aligned)
        };

        Semantics::new()
            .role(self.semantics_role)
            .child(dialog_child)
            .into_widget()
    }
}

/// The adaptive half of an [`AlertDialog`]: Dart's private `_AdaptiveAlertDialog`'s own
/// fields.
#[derive(Debug)]
struct AdaptiveAlertDialog {
    scroll_controller: Option<AnyScrollController>,
    action_scroll_controller: Option<AnyScrollController>,
    inset_animation_duration: Duration,
    inset_animation_curve: Rc<dyn Curve>,
}

/// A Material Design alert dialog.
///
/// An alert dialog (also known as a basic dialog) informs the user about
/// situations that require acknowledgment. An alert dialog has an optional
/// title and an optional list of actions. The title is displayed above the
/// content and the actions are displayed below the content.
///
/// If the content is too large to fit on the screen vertically, the dialog will
/// display the title and the actions and let the content overflow, which is
/// rarely desired. Consider using a scrolling widget for
/// [`content`](Self::content), such as `SingleChildScrollView`, to avoid
/// overflow. (However, be aware that since [`AlertDialog`] tries to size itself
/// using the intrinsic dimensions of its children, widgets such as `ListView`,
/// `GridView`, and `CustomScrollView`, which use lazy viewports, will not work.
/// If this is a problem, consider using [`Dialog`] directly.)
///
/// For dialogs that offer the user a choice between several options, consider
/// using a [`SimpleDialog`].
///
/// Typically passed as the child widget to [`show_dialog`], which displays the
/// dialog.
///
/// One set of setters serves both of Dart's constructors, [`AlertDialog::new`] and
/// [`AlertDialog::adaptive`].
///
/// See also:
///
///  * [`SimpleDialog`], which handles the scrolling of the contents but has no [`actions`](Self::actions).
///  * [`Dialog`], on which [`AlertDialog`] and [`SimpleDialog`] are based.
///  * `CupertinoAlertDialog`, an iOS-styled alert dialog.
///  * [`show_dialog`], which actually displays the dialog and returns its result.
///  * <https://material.io/design/components/dialogs.html#alert-dialog>
#[derive(Debug)]
pub struct AlertDialog {
    pub key: Option<KeyRef>,

    /// An optional icon to display at the top of the dialog.
    ///
    /// Typically, an `Icon` widget. Providing an icon centers the
    /// [`title`](Self::title)'s text.
    pub icon: Option<WidgetRef>,

    /// Color for the `Icon` in the [`icon`](Self::icon) of this [`AlertDialog`].
    ///
    /// If null, [`DialogThemeData::icon_color`] is used. If that is null, defaults to
    /// color scheme's `ColorScheme::secondary` if `ThemeData::use_material3` is
    /// true, black otherwise.
    pub icon_color: Option<Color>,

    /// Padding around the [`icon`](Self::icon).
    ///
    /// If there is no [`icon`](Self::icon), no padding will be provided. Otherwise,
    /// this padding is used.
    ///
    /// This property defaults to providing 24 pixels on the top, left, and right
    /// of the [`icon`](Self::icon). If [`title`](Self::title) is _not_ null, 16 pixels of
    /// bottom padding is added to separate the [`icon`](Self::icon) from the
    /// [`title`](Self::title). If the [`title`](Self::title) is null but
    /// [`content`](Self::content) is _not_ null, then no bottom padding is added. If both
    /// the title and content are null, 24 pixels of padding are added to the bottom.
    pub icon_padding: Option<EdgeInsetsGeometry>,

    /// The (optional) title of the dialog is displayed in a large font at the top
    /// of the dialog, below the (optional) [`icon`](Self::icon).
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,

    /// Padding around the title.
    ///
    /// If there is no title, no padding will be provided. Otherwise, this padding
    /// is used.
    ///
    /// This property defaults to providing 24 pixels on the top, left, and right
    /// of the title. If the [`content`](Self::content) is not null, then no bottom padding
    /// is provided (but see [`content_padding`](Self::content_padding)). If it _is_ null,
    /// then an extra 20 pixels of bottom padding is added to separate the
    /// [`title`](Self::title) from the [`actions`](Self::actions).
    pub title_padding: Option<EdgeInsetsGeometry>,

    /// Style for the text in the [`title`](Self::title) of this [`AlertDialog`].
    ///
    /// If null, [`DialogThemeData::title_text_style`] is used. If that's null,
    /// defaults to `TextTheme::headline_small` of `ThemeData::text_theme` if
    /// `ThemeData::use_material3` is true, `TextTheme::title_large` otherwise.
    pub title_text_style: Option<TextStyle>,

    /// The (optional) content of the dialog is displayed in the center of the
    /// dialog in a lighter font.
    ///
    /// Typically this is a `SingleChildScrollView` that contains the dialog's
    /// message. As noted in the [`AlertDialog`] documentation, it's important
    /// to use a `SingleChildScrollView` if there's any risk that the content
    /// will not fit, as the contents will otherwise overflow the dialog.
    ///
    /// The horizontal dimension of this widget is defined by the
    /// [`IntrinsicWidth`] widget. As a result, the content cannot use
    /// lazy-loading widgets such as `ListView`, `GridView`, or
    /// `CustomScrollView` without being given a fixed width.
    pub content: Option<WidgetRef>,

    /// Padding around the content.
    ///
    /// If there is no content, no padding will be provided. Otherwise, padding of
    /// 20 pixels is provided above the content to separate the content from the
    /// title, and padding of 24 pixels is provided on the left, right, and bottom
    /// to separate the content from the other edges of the dialog.
    ///
    /// If `ThemeData::use_material3` is true, the top padding separating the
    /// content from the title defaults to 16 pixels instead of 20 pixels.
    pub content_padding: Option<EdgeInsetsGeometry>,

    /// Style for the text in the [`content`](Self::content) of this [`AlertDialog`].
    ///
    /// If null, [`DialogThemeData::content_text_style`] is used. If that's null,
    /// defaults to `TextTheme::body_medium` of `ThemeData::text_theme` if
    /// `ThemeData::use_material3` is true, `TextTheme::title_medium` otherwise.
    pub content_text_style: Option<TextStyle>,

    /// The (optional) set of actions that are displayed at the bottom of the
    /// dialog with an [`OverflowBar`].
    ///
    /// Typically this is a list of `TextButton` widgets. It is recommended to
    /// set the `Text::text_align` to `TextAlign::End` for the `Text` within the
    /// `TextButton`, so that buttons whose labels wrap to an extra line align
    /// with the overall [`OverflowBar`]'s alignment within the dialog.
    ///
    /// If the [`title`](Self::title) is not null but the [`content`](Self::content) _is_
    /// null, then an extra 20 pixels of padding is added above the [`OverflowBar`] to
    /// separate the [`title`](Self::title) from the [`actions`](Self::actions).
    pub actions: Option<Vec<WidgetRef>>,

    /// Padding around the set of [`actions`](Self::actions) at the bottom of the dialog.
    ///
    /// Typically used to provide padding to the button bar between the button bar
    /// and the edges of the dialog.
    ///
    /// The [`button_padding`](Self::button_padding) may contribute to the padding on the
    /// edges of [`actions`](Self::actions) as well.
    ///
    /// If there are no [`actions`](Self::actions), then no padding will be included.
    pub actions_padding: Option<EdgeInsetsGeometry>,

    /// Defines the horizontal layout of the [`actions`](Self::actions) according to the
    /// same rules as for `Row::main_axis_alignment`.
    ///
    /// This parameter is passed along to the dialog's [`OverflowBar`].
    ///
    /// If this parameter is null (the default) then [`MainAxisAlignment::End`]
    /// is used.
    pub actions_alignment: Option<MainAxisAlignment>,

    /// The horizontal alignment of [`actions`](Self::actions) within the vertical
    /// "overflow" layout.
    ///
    /// If the dialog's [`actions`](Self::actions) do not fit into a single row, then they
    /// are arranged in a column. This parameter controls the horizontal
    /// alignment of widgets in the case of an overflow.
    ///
    /// If this parameter is null (the default) then [`OverflowBarAlignment::End`]
    /// is used.
    pub actions_overflow_alignment: Option<OverflowBarAlignment>,

    /// The vertical direction of [`actions`](Self::actions) if the children overflow
    /// horizontally.
    ///
    /// If the dialog's [`actions`](Self::actions) do not fit into a single row, then they
    /// are arranged in a column. The first action is at the top of the
    /// column if this property is set to [`VerticalDirection::Down`], since it
    /// "starts" at the top and "ends" at the bottom. On the other hand,
    /// the first action will be at the bottom of the column if this
    /// property is set to [`VerticalDirection::Up`], since it "starts" at the
    /// bottom and "ends" at the top.
    pub actions_overflow_direction: Option<VerticalDirection>,

    /// The spacing between [`actions`](Self::actions) when the [`OverflowBar`] switches to
    /// a column layout because the actions don't fit horizontally.
    ///
    /// If the widgets in [`actions`](Self::actions) do not fit into a single row, they are
    /// arranged into a column. This parameter provides additional vertical space
    /// between buttons when it does overflow.
    pub actions_overflow_button_spacing: Option<f64>,

    /// The padding that surrounds each button in [`actions`](Self::actions).
    ///
    /// This is different from [`actions_padding`](Self::actions_padding), which defines
    /// the padding between the entire button bar and the edges of the dialog.
    ///
    /// If this property is null, then it will default to
    /// 8.0 logical pixels on the left and right.
    pub button_padding: Option<EdgeInsetsGeometry>,

    /// See [`Dialog::background_color`].
    pub background_color: Option<Color>,

    /// See [`Dialog::elevation`].
    pub elevation: Option<f64>,

    /// See [`Dialog::shadow_color`].
    pub shadow_color: Option<Color>,

    /// See [`Dialog::surface_tint_color`].
    pub surface_tint_color: Option<Color>,

    /// The semantic label of the dialog used by accessibility frameworks to
    /// announce screen transitions when the dialog is opened and closed.
    ///
    /// In iOS, if this label is not provided, a semantic label will be inferred
    /// from the [`title`](Self::title) if it is not null.
    ///
    /// In Android, if this label is not provided, the dialog will use the
    /// `MaterialLocalizations::alert_dialog_label` as its label.
    pub semantic_label: Option<String>,

    /// See [`Dialog::inset_padding`].
    pub inset_padding: Option<EdgeInsets>,

    /// See [`Dialog::clip_behavior`].
    pub clip_behavior: Option<Clip>,

    /// See [`Dialog::shape`].
    pub shape: Option<ShapeBorderRef>,

    /// See [`Dialog::alignment`].
    pub alignment: Option<AlignmentGeometry>,

    /// See [`Dialog::constraints`].
    pub constraints: Option<BoxConstraints>,

    /// Determines whether the [`title`](Self::title) and [`content`](Self::content) widgets
    /// are wrapped in a scrollable.
    ///
    /// This configuration is used when the [`title`](Self::title) and
    /// [`content`](Self::content) are expected to overflow. Both [`title`](Self::title) and
    /// [`content`](Self::content) are wrapped in a scroll view, allowing all overflowed
    /// content to be visible while still showing the button bar.
    pub scrollable: bool,

    /// `Some` for Dart's `AlertDialog.adaptive`, holding the fields of its
    /// `_AdaptiveAlertDialog`.
    adaptive: Option<AdaptiveAlertDialog>,
}

impl AlertDialog {
    /// Creates an alert dialog.
    ///
    /// Typically used in conjunction with [`show_dialog`].
    ///
    /// The [`title_padding`](Self::title_padding) and
    /// [`content_padding`](Self::content_padding) default to null, which implies a default
    /// that depends on the values of the other properties. See the documentation of
    /// [`title_padding`](Self::title_padding) and [`content_padding`](Self::content_padding)
    /// on how the padding is implemented.
    pub fn new() -> AlertDialog {
        AlertDialog {
            key: None,
            icon: None,
            icon_color: None,
            icon_padding: None,
            title: None,
            title_padding: None,
            title_text_style: None,
            content: None,
            content_padding: None,
            content_text_style: None,
            actions: None,
            actions_padding: None,
            actions_alignment: None,
            actions_overflow_alignment: None,
            actions_overflow_direction: None,
            actions_overflow_button_spacing: None,
            button_padding: None,
            background_color: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            semantic_label: None,
            inset_padding: None,
            clip_behavior: None,
            shape: None,
            alignment: None,
            constraints: None,
            scrollable: false,
            adaptive: None,
        }
    }

    /// Creates an adaptive [`AlertDialog`] based on whether the target platform is
    /// iOS or macOS, following Material design's
    /// [Cross-platform guidelines](https://material.io/design/platform-guidance/cross-platform-adaptation.html).
    ///
    /// On iOS and macOS, this constructor creates a `CupertinoAlertDialog`. On other
    /// platforms, this creates a Material design [`AlertDialog`].
    ///
    /// Typically passed as a child of [`show_adaptive_dialog`], which will display
    /// the alert differently based on platform.
    ///
    /// If a `CupertinoAlertDialog` is created only these parameters are used:
    /// [`title`](Self::title), [`content`](Self::content), [`actions`](Self::actions),
    /// `scroll_controller`, `action_scroll_controller`, `inset_animation_duration`, and
    /// `inset_animation_curve`. If a material [`AlertDialog`] is created,
    /// `scroll_controller`, `action_scroll_controller`, `inset_animation_duration`, and
    /// `inset_animation_curve` are ignored.
    ///
    /// The target platform is based on the current `Theme`: `ThemeData::platform`.
    pub fn adaptive() -> AlertDialog {
        let mut dialog = AlertDialog::new();
        dialog.adaptive = Some(AdaptiveAlertDialog {
            scroll_controller: None,
            action_scroll_controller: None,
            inset_animation_duration: Duration::from_millis(100),
            inset_animation_curve: Curves::decelerate(),
        });
        dialog
    }

    /// Dart `AlertDialog(key:)`.
    pub fn key(mut self, key: KeyRef) -> AlertDialog {
        self.key = Some(key);
        self
    }

    /// Dart `AlertDialog(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> AlertDialog {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `AlertDialog(iconPadding:)`.
    pub fn icon_padding(mut self, icon_padding: EdgeInsetsGeometry) -> AlertDialog {
        self.icon_padding = Some(icon_padding);
        self
    }

    /// Dart `AlertDialog(iconColor:)`.
    pub fn icon_color(mut self, icon_color: Color) -> AlertDialog {
        self.icon_color = Some(icon_color);
        self
    }

    /// Dart `AlertDialog(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> AlertDialog {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `AlertDialog(titlePadding:)`.
    pub fn title_padding(mut self, title_padding: EdgeInsetsGeometry) -> AlertDialog {
        self.title_padding = Some(title_padding);
        self
    }

    /// Dart `AlertDialog(titleTextStyle:)`.
    pub fn title_text_style(mut self, title_text_style: TextStyle) -> AlertDialog {
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `AlertDialog(content:)`.
    pub fn content<K>(mut self, content: impl IntoWidget<K>) -> AlertDialog {
        self.content = Some(content.into_widget());
        self
    }

    /// Dart `AlertDialog(contentPadding:)`.
    pub fn content_padding(mut self, content_padding: EdgeInsetsGeometry) -> AlertDialog {
        self.content_padding = Some(content_padding);
        self
    }

    /// Dart `AlertDialog(contentTextStyle:)`.
    pub fn content_text_style(mut self, content_text_style: TextStyle) -> AlertDialog {
        self.content_text_style = Some(content_text_style);
        self
    }

    /// Dart `AlertDialog(actions:)`.
    pub fn actions(mut self, actions: impl IntoIterator<Item = WidgetRef>) -> AlertDialog {
        self.actions = Some(actions.into_iter().collect());
        self
    }

    /// Dart `AlertDialog(actionsPadding:)`.
    pub fn actions_padding(mut self, actions_padding: EdgeInsetsGeometry) -> AlertDialog {
        self.actions_padding = Some(actions_padding);
        self
    }

    /// Dart `AlertDialog(actionsAlignment:)`.
    pub fn actions_alignment(mut self, actions_alignment: MainAxisAlignment) -> AlertDialog {
        self.actions_alignment = Some(actions_alignment);
        self
    }

    /// Dart `AlertDialog(actionsOverflowAlignment:)`.
    pub fn actions_overflow_alignment(
        mut self,
        actions_overflow_alignment: OverflowBarAlignment,
    ) -> AlertDialog {
        self.actions_overflow_alignment = Some(actions_overflow_alignment);
        self
    }

    /// Dart `AlertDialog(actionsOverflowDirection:)`.
    pub fn actions_overflow_direction(
        mut self,
        actions_overflow_direction: VerticalDirection,
    ) -> AlertDialog {
        self.actions_overflow_direction = Some(actions_overflow_direction);
        self
    }

    /// Dart `AlertDialog(actionsOverflowButtonSpacing:)`.
    pub fn actions_overflow_button_spacing(
        mut self,
        actions_overflow_button_spacing: f64,
    ) -> AlertDialog {
        self.actions_overflow_button_spacing = Some(actions_overflow_button_spacing);
        self
    }

    /// Dart `AlertDialog(buttonPadding:)`.
    pub fn button_padding(mut self, button_padding: EdgeInsetsGeometry) -> AlertDialog {
        self.button_padding = Some(button_padding);
        self
    }

    /// Dart `AlertDialog(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> AlertDialog {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `AlertDialog(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> AlertDialog {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `AlertDialog(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> AlertDialog {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `AlertDialog(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> AlertDialog {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `AlertDialog(semanticLabel:)`.
    pub fn semantic_label(mut self, semantic_label: impl Into<String>) -> AlertDialog {
        self.semantic_label = Some(semantic_label.into());
        self
    }

    /// Dart `AlertDialog(insetPadding:)`.
    pub fn inset_padding(mut self, inset_padding: EdgeInsets) -> AlertDialog {
        self.inset_padding = Some(inset_padding);
        self
    }

    /// Dart `AlertDialog(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> AlertDialog {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `AlertDialog(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> AlertDialog {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `AlertDialog(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> AlertDialog {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `AlertDialog(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> AlertDialog {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `AlertDialog(scrollable:)`.
    pub fn scrollable(mut self, scrollable: bool) -> AlertDialog {
        self.scrollable = scrollable;
        self
    }

    /// Dart `AlertDialog.adaptive(scrollController:)`; a dialog made by
    /// [`new`](Self::new) ignores it.
    pub fn scroll_controller(mut self, scroll_controller: AnyScrollController) -> AlertDialog {
        if let Some(adaptive) = &mut self.adaptive {
            adaptive.scroll_controller = Some(scroll_controller);
        }
        self
    }

    /// Dart `AlertDialog.adaptive(actionScrollController:)`; a dialog made by
    /// [`new`](Self::new) ignores it.
    pub fn action_scroll_controller(
        mut self,
        action_scroll_controller: AnyScrollController,
    ) -> AlertDialog {
        if let Some(adaptive) = &mut self.adaptive {
            adaptive.action_scroll_controller = Some(action_scroll_controller);
        }
        self
    }

    /// Dart `AlertDialog.adaptive(insetAnimationDuration:)`; a dialog made by
    /// [`new`](Self::new) ignores it.
    pub fn inset_animation_duration(mut self, inset_animation_duration: Duration) -> AlertDialog {
        if let Some(adaptive) = &mut self.adaptive {
            adaptive.inset_animation_duration = inset_animation_duration;
        }
        self
    }

    /// Dart `AlertDialog.adaptive(insetAnimationCurve:)`; a dialog made by
    /// [`new`](Self::new) ignores it.
    pub fn inset_animation_curve(mut self, inset_animation_curve: Rc<dyn Curve>) -> AlertDialog {
        if let Some(adaptive) = &mut self.adaptive {
            adaptive.inset_animation_curve = inset_animation_curve;
        }
        self
    }

    /// Dart's `AlertDialog.build`, which `_AdaptiveAlertDialog.build` reaches through
    /// `super.build` on the platforms that keep the Material dialog.
    fn build_material(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        let theme = Theme::of(app, context);

        let dialog_theme = DialogTheme::of(app, context);
        let defaults: Box<dyn DialogThemeDataGetters> = if theme.use_material3 {
            Box::new(DialogDefaultsM3::new(context))
        } else {
            Box::new(DialogDefaultsM2::new(context))
        };

        let target_platform = app.platform().target_platform();
        let label = match target_platform {
            TargetPlatform::IOS | TargetPlatform::MacOS => self.semantic_label.clone(),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => Some(self.semantic_label.clone().unwrap_or_else(|| {
                <dyn MaterialLocalizations>::of(app, context).alert_dialog_label()
            })),
        };

        // The paddingScaleFactor is used to adjust the padding of Dialog's
        // children.
        let font_size_to_scale = 14.0;
        let effective_text_scale =
            MediaQuery::text_scaler_of(app, context).scale(font_size_to_scale) / font_size_to_scale;
        let padding_scale_factor = scale_padding(effective_text_scale);
        let text_direction = Directionality::maybe_of(app, context);

        let mut icon_widget = None;
        let mut title_widget = None;
        let mut content_widget = None;
        let mut actions_widget = None;

        if let Some(icon) = &self.icon {
            let below_is_title = self.title.is_some();
            let below_is_content = !below_is_title && self.content.is_some();
            let default_icon_padding = EdgeInsets::only(
                24.0,
                24.0,
                24.0,
                if below_is_title {
                    16.0
                } else if below_is_content {
                    0.0
                } else {
                    24.0
                },
            );
            let effective_icon_padding = self
                .icon_padding
                .map(|padding| padding.resolve(text_direction))
                .unwrap_or(default_icon_padding);
            let mut icon_theme = IconThemeData::new();
            if let Some(color) = self
                .icon_color
                .or(dialog_theme.icon_color)
                .or_else(|| defaults.icon_color(app))
            {
                icon_theme = icon_theme.color(color);
            }
            icon_widget = Some(
                Padding::new(
                    EdgeInsets::only(
                        effective_icon_padding.left * padding_scale_factor,
                        effective_icon_padding.top * padding_scale_factor,
                        effective_icon_padding.right * padding_scale_factor,
                        effective_icon_padding.bottom,
                    )
                    .into(),
                )
                .child(IconTheme::new(icon_theme, icon.clone()))
                .into_widget(),
            );
        }

        if let Some(title) = &self.title {
            let default_title_padding = EdgeInsets::only(
                24.0,
                if self.icon.is_none() { 24.0 } else { 0.0 },
                24.0,
                if self.content.is_none() { 20.0 } else { 0.0 },
            );
            let effective_title_padding = self
                .title_padding
                .map(|padding| padding.resolve(text_direction))
                .unwrap_or(default_title_padding);
            title_widget = Some(
                Padding::new(
                    EdgeInsets::only(
                        effective_title_padding.left * padding_scale_factor,
                        if self.icon.is_none() {
                            effective_title_padding.top * padding_scale_factor
                        } else {
                            effective_title_padding.top
                        },
                        effective_title_padding.right * padding_scale_factor,
                        effective_title_padding.bottom,
                    )
                    .into(),
                )
                .child(
                    DefaultTextStyle::new(
                        self.title_text_style
                            .clone()
                            .or(dialog_theme.title_text_style.clone())
                            .or_else(|| defaults.title_text_style(app))
                            .expect("the defaults give a title text style"),
                        Semantics::new()
                            // For iOS platform, the focus always lands on the title.
                            // Set nameRoute to false to avoid title being announce twice.
                            .names_route(label.is_none() && target_platform != TargetPlatform::IOS)
                            .container(true)
                            .child(title.clone()),
                    )
                    .text_align(if self.icon.is_none() {
                        TextAlign::Start
                    } else {
                        TextAlign::Center
                    }),
                )
                .into_widget(),
            );
        }

        if let Some(content) = &self.content {
            let default_content_padding = EdgeInsets::only(
                24.0,
                if theme.use_material3 { 16.0 } else { 20.0 },
                24.0,
                24.0,
            );
            let effective_content_padding = self
                .content_padding
                .map(|padding| padding.resolve(text_direction))
                .unwrap_or(default_content_padding);
            content_widget = Some(
                Padding::new(
                    EdgeInsets::only(
                        effective_content_padding.left * padding_scale_factor,
                        if self.title.is_none() && self.icon.is_none() {
                            effective_content_padding.top * padding_scale_factor
                        } else {
                            effective_content_padding.top
                        },
                        effective_content_padding.right * padding_scale_factor,
                        effective_content_padding.bottom,
                    )
                    .into(),
                )
                .child(DefaultTextStyle::new(
                    self.content_text_style
                        .clone()
                        .or(dialog_theme.content_text_style.clone())
                        .or_else(|| defaults.content_text_style(app))
                        .expect("the defaults give a content text style"),
                    Semantics::new()
                        .container(true)
                        .explicit_child_nodes(true)
                        .child(content.clone()),
                ))
                .into_widget(),
            );
        }

        if let Some(actions) = &self.actions {
            let spacing = self
                .button_padding
                .map_or(16.0, |button_padding| button_padding.horizontal())
                / 2.0;
            let actions_padding = self
                .actions_padding
                .or(dialog_theme.actions_padding)
                .unwrap_or_else(|| {
                    let default_padding = defaults
                        .actions_padding(app)
                        .expect("the defaults give an actions padding");
                    if theme.use_material3 {
                        default_padding
                    } else {
                        default_padding.add(EdgeInsetsGeometry::all(spacing))
                    }
                });
            actions_widget = Some(
                Padding::new(actions_padding)
                    .child(
                        OverflowBar::new()
                            .alignment(self.actions_alignment.unwrap_or(MainAxisAlignment::End))
                            .spacing(spacing)
                            .overflow_alignment(
                                self.actions_overflow_alignment
                                    .unwrap_or(OverflowBarAlignment::End),
                            )
                            .overflow_direction(
                                self.actions_overflow_direction
                                    .unwrap_or(VerticalDirection::Down),
                            )
                            .overflow_spacing(self.actions_overflow_button_spacing.unwrap_or(0.0))
                            .children(actions.clone()),
                    )
                    .into_widget(),
            );
        }

        let column_children: Vec<WidgetRef> = if self.scrollable {
            let mut children = Vec::new();
            if self.title.is_some() || self.content.is_some() {
                children.push(
                    Flexible::new(
                        SingleChildScrollView::new().child(
                            Column::new()
                                .main_axis_size(MainAxisSize::Min)
                                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                .children(
                                    [icon_widget, title_widget, content_widget]
                                        .into_iter()
                                        .flatten(),
                                ),
                        ),
                    )
                    .into_widget(),
                );
            }
            children.extend(actions_widget);
            children
        } else {
            [
                icon_widget,
                title_widget,
                content_widget.map(|content_widget| Flexible::new(content_widget).into_widget()),
                actions_widget,
            ]
            .into_iter()
            .flatten()
            .collect()
        };

        let mut dialog_child = IntrinsicWidth::new()
            .child(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children(column_children),
            )
            .into_widget();

        if let Some(label) = label {
            dialog_child = Semantics::new()
                .scopes_route(true)
                .explicit_child_nodes(true)
                .names_route(true)
                .label(label)
                .child(dialog_child)
                .into_widget();
        }

        let mut dialog = Dialog::new().semantics_role(SemanticsRole::AlertDialog);
        dialog.background_color = self.background_color;
        dialog.elevation = self.elevation;
        dialog.shadow_color = self.shadow_color;
        dialog.surface_tint_color = self.surface_tint_color;
        dialog.inset_padding = self.inset_padding;
        dialog.clip_behavior = self.clip_behavior;
        dialog.shape = self.shape.clone();
        dialog.alignment = self.alignment;
        dialog.constraints = self.constraints;
        dialog.child = Some(dialog_child);
        dialog.into_widget()
    }

    /// Dart's `_AdaptiveAlertDialog.build`.
    fn build_adaptive(
        &self,
        app: &mut App,
        context: BuildContext,
        adaptive: &AdaptiveAlertDialog,
    ) -> WidgetRef {
        let theme = Theme::of(app, context);
        match theme.platform {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {}
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                let mut dialog = CupertinoAlertDialog::new()
                    .actions(self.actions.clone().unwrap_or_default())
                    .inset_animation_duration(adaptive.inset_animation_duration)
                    .inset_animation_curve(Rc::clone(&adaptive.inset_animation_curve));
                dialog.title = self.title.clone();
                dialog.content = self.content.clone();
                dialog.scroll_controller = adaptive.scroll_controller;
                dialog.action_scroll_controller = adaptive.action_scroll_controller;
                return dialog.into_widget();
            }
        }
        self.build_material(app, context)
    }
}

impl Default for AlertDialog {
    fn default() -> AlertDialog {
        AlertDialog::new()
    }
}

impl StatelessWidget for AlertDialog {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        match &self.adaptive {
            Some(adaptive) => self.build_adaptive(app, context, adaptive),
            None => self.build_material(app, context),
        }
    }
}

/// An option used in a [`SimpleDialog`].
///
/// A simple dialog offers the user a choice between several options. This
/// widget is commonly used to represent each of the options. If the user
/// selects this option, the widget will call the [`on_pressed`](Self::on_pressed)
/// callback, which typically uses [`Navigator::pop`] to close the dialog.
///
/// The padding on a [`SimpleDialogOption`] is configured to combine with the
/// default [`SimpleDialog::content_padding`] so that each option ends up 8 pixels
/// from the other vertically, with 20 pixels of spacing between the dialog's
/// title and the first option, and 24 pixels of spacing between the last option
/// and the bottom of the dialog.
///
/// See also:
///
///  * [`SimpleDialog`], for a dialog in which to use this widget.
///  * [`show_dialog`], which actually displays the dialog and returns its result.
///  * `TextButton`, which are commonly used as actions in other kinds of
///    dialogs, such as [`AlertDialog`]s.
///  * <https://material.io/design/components/dialogs.html#simple-dialog>
#[derive(Debug, Default)]
pub struct SimpleDialogOption {
    pub key: Option<KeyRef>,

    /// The callback that is called when this option is selected.
    ///
    /// If this is set to null, the option cannot be selected.
    ///
    /// When used in a [`SimpleDialog`], this will typically call [`Navigator::pop`]
    /// with a value for [`show_dialog`] to complete its future with.
    pub on_pressed: Option<waft_foundation::Listener>,

    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    pub child: Option<WidgetRef>,

    /// The amount of space to surround the [`child`](Self::child) with.
    ///
    /// Defaults to EdgeInsets.symmetric(vertical: 8.0, horizontal: 24.0).
    pub padding: Option<EdgeInsets>,
}

impl SimpleDialogOption {
    /// Creates an option for a [`SimpleDialog`].
    pub fn new() -> SimpleDialogOption {
        SimpleDialogOption::default()
    }

    /// Dart `SimpleDialogOption(key:)`.
    pub fn key(mut self, key: KeyRef) -> SimpleDialogOption {
        self.key = Some(key);
        self
    }

    /// Dart `SimpleDialogOption(onPressed:)`.
    pub fn on_pressed(mut self, on_pressed: waft_foundation::Listener) -> SimpleDialogOption {
        self.on_pressed = Some(on_pressed);
        self
    }

    /// Dart `SimpleDialogOption(padding:)`.
    pub fn padding(mut self, padding: EdgeInsets) -> SimpleDialogOption {
        self.padding = Some(padding);
        self
    }

    /// Dart `SimpleDialogOption(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SimpleDialogOption {
        self.child = Some(child.into_widget());
        self
    }
}

impl StatelessWidget for SimpleDialogOption {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut padding = Padding::new(
            self.padding
                .unwrap_or(EdgeInsets::symmetric(8.0, 24.0))
                .into(),
        );
        padding.child = self.child.clone();
        let mut ink_well = InkWell::new().child(padding);
        ink_well.ink_response.on_tap = self.on_pressed.clone();
        ink_well.into_widget()
    }
}

/// A simple Material Design dialog.
///
/// A simple dialog offers the user a choice between several options. A simple
/// dialog has an optional title that is displayed above the choices.
///
/// Choices are normally represented using [`SimpleDialogOption`] widgets. If
/// other widgets are used, see [`content_padding`](Self::content_padding) for notes
/// regarding the conventions for obtaining the spacing expected by Material
/// Design.
///
/// For dialogs that inform the user about a situation, consider using an
/// [`AlertDialog`].
///
/// Typically passed as the child widget to [`show_dialog`], which displays the
/// dialog.
///
/// See also:
///
///  * [`SimpleDialogOption`], which are options used in this type of dialog.
///  * [`AlertDialog`], for dialogs that have a row of buttons below the body.
///  * [`Dialog`], on which [`SimpleDialog`] and [`AlertDialog`] are based.
///  * [`show_dialog`], which actually displays the dialog and returns its result.
///  * <https://material.io/design/components/dialogs.html#simple-dialog>
#[derive(Debug)]
pub struct SimpleDialog {
    pub key: Option<KeyRef>,

    /// The (optional) title of the dialog is displayed in a large font at the top
    /// of the dialog.
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,

    /// Padding around the title.
    ///
    /// If there is no title, no padding will be provided.
    ///
    /// By default, this provides the recommend Material Design padding of 24
    /// pixels around the left, top, and right edges of the title.
    ///
    /// See [`content_padding`](Self::content_padding) for the conventions regarding
    /// padding between the [`title`](Self::title) and the [`children`](Self::children).
    pub title_padding: EdgeInsetsGeometry,

    /// Style for the text in the [`title`](Self::title) of this [`SimpleDialog`].
    ///
    /// If null, [`DialogThemeData::title_text_style`] is used. If that's null,
    /// defaults to `TextTheme::title_large` of `ThemeData::text_theme`.
    pub title_text_style: Option<TextStyle>,

    /// The (optional) content of the dialog is displayed in a
    /// `SingleChildScrollView` underneath the title.
    ///
    /// Typically a list of [`SimpleDialogOption`]s.
    pub children: Option<Vec<WidgetRef>>,

    /// Padding around the content.
    ///
    /// By default, this is 12 pixels on the top and 16 pixels on the bottom. This
    /// is intended to be combined with children that have 24 pixels of padding on
    /// the left and right, and 8 pixels of padding on the top and bottom, so that
    /// the content ends up being indented 20 pixels from the title, 24 pixels from
    /// the bottom, and 24 pixels from the sides.
    ///
    /// The [`SimpleDialogOption`] widget uses such padding.
    ///
    /// If there is no [`title`](Self::title), the [`content_padding`](Self::content_padding)
    /// should be adjusted so that the top padding ends up being 24 pixels.
    pub content_padding: EdgeInsetsGeometry,

    /// See [`Dialog::background_color`].
    pub background_color: Option<Color>,

    /// See [`Dialog::elevation`].
    pub elevation: Option<f64>,

    /// See [`Dialog::shadow_color`].
    pub shadow_color: Option<Color>,

    /// See [`Dialog::surface_tint_color`].
    pub surface_tint_color: Option<Color>,

    /// Style for the text in the [`children`](Self::children) of this [`SimpleDialog`].
    ///
    /// If null, [`DialogThemeData::content_text_style`] is used. If that's null,
    /// defaults to `TextTheme::title_medium` of `ThemeData::text_theme`.
    pub content_text_style: Option<TextStyle>,

    /// The semantic label of the dialog used by accessibility frameworks to
    /// announce screen transitions when the dialog is opened and closed.
    ///
    /// If this label is not provided, a semantic label will be inferred from the
    /// [`title`](Self::title) if it is not null. If there is no title, the label will be
    /// taken from `MaterialLocalizations::dialog_label`.
    pub semantic_label: Option<String>,

    /// See [`Dialog::inset_padding`].
    pub inset_padding: Option<EdgeInsets>,

    /// See [`Dialog::clip_behavior`].
    pub clip_behavior: Option<Clip>,

    /// See [`Dialog::shape`].
    pub shape: Option<ShapeBorderRef>,

    /// See [`Dialog::alignment`].
    pub alignment: Option<AlignmentGeometry>,

    /// See [`Dialog::constraints`].
    pub constraints: Option<BoxConstraints>,
}

impl SimpleDialog {
    /// Creates a simple dialog.
    ///
    /// Typically used in conjunction with [`show_dialog`].
    pub fn new() -> SimpleDialog {
        SimpleDialog {
            key: None,
            title: None,
            title_padding: EdgeInsetsGeometry::from_ltrb(24.0, 24.0, 24.0, 0.0),
            title_text_style: None,
            children: None,
            content_padding: EdgeInsetsGeometry::from_ltrb(0.0, 12.0, 0.0, 16.0),
            content_text_style: None,
            background_color: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            semantic_label: None,
            inset_padding: None,
            clip_behavior: None,
            shape: None,
            alignment: None,
            constraints: None,
        }
    }

    /// Dart `SimpleDialog(key:)`.
    pub fn key(mut self, key: KeyRef) -> SimpleDialog {
        self.key = Some(key);
        self
    }

    /// Dart `SimpleDialog(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> SimpleDialog {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `SimpleDialog(titlePadding:)`.
    pub fn title_padding(mut self, title_padding: EdgeInsetsGeometry) -> SimpleDialog {
        self.title_padding = title_padding;
        self
    }

    /// Dart `SimpleDialog(titleTextStyle:)`.
    pub fn title_text_style(mut self, title_text_style: TextStyle) -> SimpleDialog {
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `SimpleDialog(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> SimpleDialog {
        self.children = Some(children.into_iter().collect());
        self
    }

    /// Dart `SimpleDialog(contentPadding:)`.
    pub fn content_padding(mut self, content_padding: EdgeInsetsGeometry) -> SimpleDialog {
        self.content_padding = content_padding;
        self
    }

    /// Dart `SimpleDialog(contentTextStyle:)`.
    pub fn content_text_style(mut self, content_text_style: TextStyle) -> SimpleDialog {
        self.content_text_style = Some(content_text_style);
        self
    }

    /// Dart `SimpleDialog(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> SimpleDialog {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `SimpleDialog(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> SimpleDialog {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `SimpleDialog(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> SimpleDialog {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `SimpleDialog(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> SimpleDialog {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `SimpleDialog(semanticLabel:)`.
    pub fn semantic_label(mut self, semantic_label: impl Into<String>) -> SimpleDialog {
        self.semantic_label = Some(semantic_label.into());
        self
    }

    /// Dart `SimpleDialog(insetPadding:)`.
    pub fn inset_padding(mut self, inset_padding: EdgeInsets) -> SimpleDialog {
        self.inset_padding = Some(inset_padding);
        self
    }

    /// Dart `SimpleDialog(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> SimpleDialog {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `SimpleDialog(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> SimpleDialog {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `SimpleDialog(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> SimpleDialog {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `SimpleDialog(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> SimpleDialog {
        self.constraints = Some(constraints);
        self
    }
}

impl Default for SimpleDialog {
    fn default() -> SimpleDialog {
        SimpleDialog::new()
    }
}

impl StatelessWidget for SimpleDialog {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        let theme = Theme::of(app, context);

        let dialog_theme = DialogTheme::of(app, context);
        let defaults: Box<dyn DialogThemeDataGetters> = if theme.use_material3 {
            Box::new(DialogDefaultsM3::new(context))
        } else {
            Box::new(DialogDefaultsM2::new(context))
        };

        let target_platform = app.platform().target_platform();
        let mut label = self.semantic_label.clone();
        match target_platform {
            TargetPlatform::MacOS | TargetPlatform::IOS => {}
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                if label.is_none() {
                    label = Some(<dyn MaterialLocalizations>::of(app, context).dialog_label());
                }
            }
        }

        // The paddingScaleFactor is used to adjust the padding of Dialog
        // children.
        let effective_title_text_style = self
            .title_text_style
            .clone()
            .or(dialog_theme.title_text_style.clone())
            .or(theme.text_theme.title_large.clone())
            .expect("textTheme.titleLarge");
        let font_size = effective_title_text_style
            .font_size
            .unwrap_or(K_DEFAULT_FONT_SIZE);
        let font_size_to_scale = if font_size == 0.0 {
            K_DEFAULT_FONT_SIZE
        } else {
            font_size
        };
        let effective_text_scale =
            MediaQuery::text_scaler_of(app, context).scale(font_size_to_scale) / font_size_to_scale;
        let padding_scale_factor = scale_padding(effective_text_scale);
        let text_direction = Directionality::maybe_of(app, context);

        let mut title_widget = None;
        if let Some(title) = &self.title {
            let effective_title_padding = self.title_padding.resolve(text_direction);
            title_widget = Some(
                Padding::new(
                    EdgeInsets::only(
                        effective_title_padding.left * padding_scale_factor,
                        effective_title_padding.top * padding_scale_factor,
                        effective_title_padding.right * padding_scale_factor,
                        if self.children.is_none() {
                            effective_title_padding.bottom * padding_scale_factor
                        } else {
                            effective_title_padding.bottom
                        },
                    )
                    .into(),
                )
                .child(DefaultTextStyle::new(
                    effective_title_text_style,
                    Semantics::new()
                        // For iOS platform, the focus always lands on the title.
                        // Set nameRoute to false to avoid title being announce twice.
                        .names_route(label.is_none() && target_platform != TargetPlatform::IOS)
                        .container(true)
                        .child(title.clone()),
                ))
                .into_widget(),
            );
        }

        let mut content_widget = None;
        if let Some(children) = &self.children {
            let effective_content_padding = self.content_padding.resolve(text_direction);
            content_widget = Some(
                Flexible::new(
                    SingleChildScrollView::new()
                        .padding(
                            EdgeInsets::only(
                                effective_content_padding.left * padding_scale_factor,
                                if self.title.is_none() {
                                    effective_content_padding.top * padding_scale_factor
                                } else {
                                    effective_content_padding.top
                                },
                                effective_content_padding.right * padding_scale_factor,
                                effective_content_padding.bottom * padding_scale_factor,
                            )
                            .into(),
                        )
                        .child(DefaultTextStyle::new(
                            self.content_text_style
                                .clone()
                                .or(dialog_theme.content_text_style.clone())
                                .or_else(|| defaults.content_text_style(app))
                                .expect("the defaults give a content text style"),
                            ListBody::new().children(children.clone()),
                        )),
                )
                .into_widget(),
            );
        }

        let mut dialog_child = IntrinsicWidth::new()
            .step_width(56.0)
            .child(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children([title_widget, content_widget].into_iter().flatten()),
            )
            .into_widget();

        if let Some(label) = label {
            dialog_child = Semantics::new()
                .scopes_route(true)
                .explicit_child_nodes(true)
                .names_route(true)
                .label(label)
                .child(dialog_child)
                .into_widget();
        }
        let mut dialog = Dialog::new();
        dialog.background_color = self.background_color;
        dialog.elevation = self.elevation;
        dialog.shadow_color = self.shadow_color;
        dialog.surface_tint_color = self.surface_tint_color;
        dialog.inset_padding = self.inset_padding;
        dialog.clip_behavior = self.clip_behavior;
        dialog.shape = self.shape.clone();
        dialog.alignment = self.alignment;
        dialog.constraints = self.constraints;
        dialog.child = Some(dialog_child);
        dialog.into_widget()
    }
}

fn build_material_dialog_transitions(
    _app: &mut App,
    _context: BuildContext,
    _animation: AnyAnimation<f64>,
    _secondary_animation: AnyAnimation<f64>,
    child: WidgetRef,
) -> WidgetRef {
    child
}

// Wrapper that makes dialogs fill the entire window without insets or rounded corners.
#[derive(Debug)]
struct FullWindowDialogWrapper {
    child: WidgetRef,
}

impl StatelessWidget for FullWindowDialogWrapper {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let window_dialog_theme = DialogTheme::of(app, context)
            .copy_with()
            .inset_padding(EdgeInsets::ZERO)
            // No rounded corners.
            .shape(RoundedRectangleBorder::default())
            // Align to top-left so it fills from corner.
            .alignment(AlignmentGeometry::TOP_LEFT)
            // Remove default constraints so dialog can expand to fill available space.
            .constraints(BoxConstraints::expand(None, None));

        let without_view_padding = MediaQuery::remove_view_padding(
            app,
            context,
            true,
            true,
            true,
            true,
            self.child.clone(),
        )
        .into_widget();
        DialogTheme::new()
            .with_data(window_dialog_theme)
            .child(
                MediaQuery::remove_view_insets(
                    app,
                    context,
                    true,
                    true,
                    true,
                    true,
                    without_view_padding,
                )
                .into_widget(),
            )
            .into_widget()
    }
}

/// Dart's `void Function(Object?)`, which takes a dialog's result.
type DialogPopCallback = Rc<dyn Fn(&mut App, RouteResult)>;

// Provides a pop callback that dialog content can use.
// Wraps content to provide a Navigator-like interface for popping.
struct DialogPopScope {
    child: WidgetRef,
    on_pop: Option<DialogPopCallback>,
}

impl Debug for DialogPopScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DialogPopScope")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for DialogPopScope {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        // Wrap with PopupScope to handle back button and provide popNavigator function.
        let on_pop = self.on_pop.clone();
        let shim_on_pop = self.on_pop.clone();
        let child = self.child.clone();
        PopScope::new(Builder::new(move |_app, _context| {
            // Provide a way for child widgets to pop using Navigator.maybePop(context)
            // by wrapping in a minimal Navigator.
            NavigatorShim {
                on_pop: shim_on_pop.clone(),
                child: child.clone(),
            }
            .into_widget()
        }))
        .can_pop(false)
        .on_pop_invoked_with_result(Rc::new(move |app, did_pop, result| {
            if !did_pop && let Some(on_pop) = &on_pop {
                on_pop(app, result);
            }
        }))
        .into_widget()
    }
}

// Creates a minimal Navigator that intercepts pop calls.
struct NavigatorShim {
    on_pop: Option<DialogPopCallback>,
    child: WidgetRef,
}

impl Debug for NavigatorShim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigatorShim")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for NavigatorShim {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        // Create a Navigator with a single page that contains the child
        // This allows Navigator.pop(context) calls from within the dialog to work.
        // Wrap in HeroControllerScope.none() to prevent this Navigator from
        // inheriting the outer HeroController, which would cause a "HeroController
        // can not be shared by multiple Navigators" assertion.
        let on_pop = self.on_pop.clone();
        let page: PageRef = Rc::new(DialogContentPage {
            child: self.child.clone(),
        });
        HeroControllerScope::none(Navigator::new().pages([page]).on_pop_page(
            move |app, _route, result| {
                // When the page is popped, call our onPop callback
                if let Some(on_pop) = &on_pop {
                    on_pop(app, result);
                }
                // Return false to prevent the route from being removed from the Navigator
                // (since we're handling the pop externally by closing the dialog window).
                false
            },
        ))
        .into_widget()
    }
}

// A simple page for the dialog content.
#[derive(Debug)]
struct DialogContentPage {
    child: WidgetRef,
}

impl Page for DialogContentPage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_route(&self, app: &mut App, _context: BuildContext, this: &PageRef) -> AnyRoute {
        let child = self.child.clone();
        let page_builder: RoutePageBuilder =
            Rc::new(move |_app, _context, _animation, _secondary_animation| child.clone());
        PageRouteBuilder::new(app, page_builder)
            .settings(app, RouteSettingsRef::Page(Rc::clone(this)))
            .transition_duration(app, Duration::ZERO)
            .reverse_transition_duration(app, Duration::ZERO)
            .as_route()
    }
}

/// Displays a Material dialog above the current contents of the app, with
/// Material entrance and exit animations, modal barrier color, and modal
/// barrier behavior (dialog is dismissible with a tap on the barrier).
///
/// This function takes a `builder` which typically builds a [`Dialog`] widget.
/// Content below the dialog is dimmed with a `ModalBarrier`. The widget
/// returned by the `builder` does not share a context with the location that
/// [`show_dialog`] is originally called from. Use a `StatefulBuilder` or a
/// custom `StatefulWidget` if the dialog needs to update dynamically.
///
/// The `context` argument is used to look up the [`Navigator`] and [`Theme`] for
/// the dialog. It is only used when the method is called. Its corresponding
/// widget can be safely removed from the tree before the dialog is closed.
///
/// The `barrier_dismissible` argument is used to indicate whether tapping on the
/// barrier will dismiss the dialog. It is `true` by default and can not be `null`.
///
/// The `barrier_color` argument is used to specify the color of the modal
/// barrier that darkens everything below the dialog. If `null` the `barrier_color`
/// field from `DialogThemeData` is used. If that is `null` the default color
/// `Colors::BLACK54` is used.
///
/// The `use_safe_area` argument is used to indicate if the dialog should only
/// display in 'safe' areas of the screen not used by the operating system
/// (see [`SafeArea`] for more details). It is `true` by default, which means
/// the dialog will not overlap operating system areas. If it is set to `false`
/// the dialog will only be constrained by the screen size. It can not be `null`.
///
/// The `use_root_navigator` argument is used to determine whether to push the
/// dialog to the [`Navigator`] furthest from or nearest to the given `context`.
/// By default, `use_root_navigator` is `true` and the dialog route created by
/// this method is pushed to the root navigator. It can not be `null`.
///
/// The `route_settings` argument is passed to [`DialogRoute::settings`].
///
/// If not null, the `traversal_edge_behavior` argument specifies the transfer of
/// focus beyond the first and the last items of the dialog route. By default,
/// [`TraversalEdgeBehavior::ClosedLoop`] is used, because it's typical for dialogs
/// to allow users to cycle through dialog widgets without leaving the dialog.
///
/// If `animation_style` is provided, it will be used to override the default
/// animation duration and curves of the dialog.
///
/// Returns a future that resolves to the value (if any) that was passed to
/// [`Navigator::pop`] when the dialog was closed.
///
/// See also:
///
///  * [`AlertDialog`], for dialogs that have a row of buttons below a body.
///  * [`SimpleDialog`], which handles the scrolling of the contents and does
///    not show buttons below its body.
///  * [`Dialog`], on which [`SimpleDialog`] and [`AlertDialog`] are based.
///  * `show_cupertino_dialog`, which displays an iOS-style dialog.
///  * `show_general_dialog`, which allows for customization of the dialog popup.
///  * [`DialogRoute`], the route this pushes.
///  * <https://material.io/design/components/dialogs.html>
///  * <https://m3.material.io/components/dialogs>
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_dialog(
    app: &mut App,
    context: BuildContext,
    builder: WidgetBuilder,
    barrier_dismissible: bool,
    barrier_color: Option<Color>,
    barrier_label: Option<String>,
    use_safe_area: bool,
    use_root_navigator: bool,
    route_settings: Option<RouteSettingsRef>,
    traversal_edge_behavior: Option<TraversalEdgeBehavior>,
    fullscreen_dialog: bool,
    request_focus: Option<bool>,
    animation_style: Option<AnimationStyle>,
) -> CompleterFuture<RouteResult> {
    debug_assert!(debug_is_active(app, context));
    debug_assert!(debug_check_has_material_localizations(app, context));

    let navigator_context = Navigator::of(app, context, use_root_navigator).context(app);
    let themes = Rc::new(<dyn InheritedTheme>::capture(
        app,
        context,
        Some(navigator_context),
    ));
    let navigator = Navigator::of(app, context, use_root_navigator);

    let route_builder_builder = Rc::clone(&builder);
    let route_settings_for_route = route_settings.clone();
    let route_builder: RawDialogRouteBuilder = Rc::new(move |app, route_context, _builder| {
        let barrier_color = barrier_color
            .or(DialogTheme::of(app, context).barrier_color)
            .or(Theme::of(app, context).dialog_theme.barrier_color)
            .unwrap_or(Colors::BLACK54);
        let route = DialogRoute::new(app, route_context, Rc::clone(&route_builder_builder))
            .barrier_color(app, barrier_color)
            .barrier_dismissible(app, barrier_dismissible)
            .use_safe_area(app, use_safe_area)
            .themes(app, Rc::clone(&themes))
            .traversal_edge_behavior(
                app,
                traversal_edge_behavior.unwrap_or(TraversalEdgeBehavior::ClosedLoop),
            )
            .fullscreen_dialog(app, fullscreen_dialog);
        if let Some(barrier_label) = barrier_label.clone() {
            route.barrier_label(app, barrier_label);
        }
        if let Some(route_settings) = route_settings_for_route.clone() {
            route.settings(app, route_settings);
        }
        if let Some(request_focus) = request_focus {
            route.request_focus(app, request_focus);
        }
        if let Some(animation_style) = animation_style.clone() {
            route.animation_style(app, animation_style);
        }
        route.as_route()
    });
    let window_builder: WidgetBuilder = Rc::new(move |app, _route_context| {
        // Wrap the build dialog with the theme, text direction, and media
        // query data from the caller's context (not the navigator's context,
        // which would resolve to the root app theme and override captured themes).
        let text_direction = Directionality::of(app, context);
        let theme_data = Theme::of(app, context);
        let media_query = MediaQuery::of(app, context);
        let pop_navigator = Navigator::of(app, navigator.context(app), false);
        let content_builder = Rc::clone(&builder);
        let dialog_content = DialogPopScope {
            on_pop: Some(Rc::new(move |app, result| pop_navigator.pop(app, result))),
            child: Builder::new(move |app, inner_context| {
                FullWindowDialogWrapper {
                    child: content_builder(app, inner_context),
                }
                .into_widget()
            })
            .into_widget(),
        };

        Directionality::new(
            text_direction,
            Theme::new(
                theme_data,
                MediaQuery::new(media_query, dialog_content).into_widget(),
            ),
        )
        .into_widget()
    });

    show_raw_dialog(
        app,
        context,
        window_builder,
        Some(route_builder),
        use_root_navigator,
        route_settings,
        fullscreen_dialog,
    )
}

/// Displays either a Material or Cupertino dialog depending on platform.
///
/// On most platforms this function will act the same as [`show_dialog`], except
/// for iOS and macOS, in which case it will act more like
/// [`show_cupertino_dialog`].
///
/// On Cupertino platforms, `barrier_color`, `use_safe_area`, and
/// `traversal_edge_behavior` are ignored.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_adaptive_dialog(
    app: &mut App,
    context: BuildContext,
    builder: WidgetBuilder,
    barrier_dismissible: Option<bool>,
    barrier_color: Option<Color>,
    barrier_label: Option<String>,
    use_safe_area: bool,
    use_root_navigator: bool,
    route_settings: Option<RouteSettingsRef>,
    traversal_edge_behavior: Option<TraversalEdgeBehavior>,
    request_focus: Option<bool>,
    animation_style: Option<AnimationStyle>,
) -> CompleterFuture<RouteResult> {
    let theme = Theme::of(app, context);
    match theme.platform {
        TargetPlatform::Android
        | TargetPlatform::Fuchsia
        | TargetPlatform::Linux
        | TargetPlatform::Windows => show_dialog(
            app,
            context,
            builder,
            barrier_dismissible.unwrap_or(true),
            barrier_color,
            barrier_label,
            use_safe_area,
            use_root_navigator,
            route_settings,
            traversal_edge_behavior,
            false,
            request_focus,
            animation_style,
        ),
        TargetPlatform::IOS | TargetPlatform::MacOS => show_cupertino_dialog(
            app,
            context,
            builder,
            barrier_label,
            None,
            use_root_navigator,
            barrier_dismissible.unwrap_or(false),
            route_settings,
            request_focus,
        ),
    }
}

fn debug_is_active(app: &App, context: BuildContext) -> bool {
    if !context.debug_is_active(app) {
        panic!(
            "This BuildContext is no longer valid.\nThe showDialog function context parameter \
             is a BuildContext that is no longer valid.\nThis can commonly occur when the \
             showDialog function is called after awaiting a Future. In this situation the \
             BuildContext might refer to a widget that has already been disposed during the \
             await. Consider using a parent context instead."
        );
    }
    true
}

/// Dart's `pageBuilder` closure in `DialogRoute`'s constructor, over the constructor's
/// `builder`, `themes` and `useSafeArea`.
fn dialog_page_builder(
    builder: WidgetBuilder,
    themes: Option<Rc<CapturedThemes>>,
    use_safe_area: bool,
) -> RoutePageBuilder {
    Rc::new(
        move |_app, _build_context, _animation, _secondary_animation| {
            let builder = Rc::clone(&builder);
            let page_child = Builder::new(move |app, context| builder(app, context)).into_widget();
            let mut dialog = match &themes {
                Some(themes) => themes.wrap(page_child),
                None => page_child,
            };
            if use_safe_area {
                dialog = SafeArea::new(dialog).into_widget();
            }
            // Prevent clicks inside the dialog from passing through to the barrier
            Semantics::new()
                .hit_test_behavior(SemanticsHitTestBehavior::Opaque)
                .child(dialog)
                .into_widget()
        },
    )
}

/// A dialog route with Material entrance and exit animations,
/// modal barrier color, and modal barrier behavior (dialog is dismissible
/// with a tap on the barrier).
///
/// It is used internally by [`show_dialog`] or can be directly pushed
/// onto the [`Navigator`] stack to enable state restoration. See
/// [`show_dialog`] for a state restoration app example.
///
/// This function takes a `builder` which typically builds a [`Dialog`] widget.
/// Content below the dialog is dimmed with a `ModalBarrier`. The widget
/// returned by the `builder` does not share a context with the location that
/// [`show_dialog`] is originally called from. Use a `StatefulBuilder` or a
/// custom `StatefulWidget` if the dialog needs to update dynamically.
///
/// The `context` argument is used to look up
/// `MaterialLocalizations::modal_barrier_dismiss_label`, which provides the
/// modal with a localized accessibility label that will be used for the
/// modal's barrier. However, a custom `barrier_label` can be passed in as well.
///
/// The `barrier_dismissible` argument is used to indicate whether tapping on the
/// barrier will dismiss the dialog. It is `true` by default and can not be `null`.
///
/// The `barrier_color` argument is used to specify the color of the modal
/// barrier that darkens everything below the dialog. If `null`, the default
/// color `Colors::BLACK54` is used.
///
/// The `use_safe_area` argument is used to indicate if the dialog should only
/// display in 'safe' areas of the screen not used by the operating system
/// (see [`SafeArea`] for more details). It is `true` by default, which means
/// the dialog will not overlap operating system areas. If it is set to `false`
/// the dialog will only be constrained by the screen size. It can not be `null`.
///
/// The `settings` argument define the settings for this route. See
/// `RouteSettings` for details.
///
/// The `animation_style` argument is used to override the default animation
/// duration and curves of the dialog route.
///
/// See also:
///
///  * [`show_dialog`], which is a way to display a [`DialogRoute`].
///  * `show_general_dialog`, which allows for customization of the dialog popup.
///  * `show_cupertino_dialog`, which displays an iOS-style dialog.
pub struct DialogRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    raw_dialog_route: RawDialogRouteData,
    /// The constructor's `builder`, `themes` and `useSafeArea`, which its page builder closes
    /// over.
    builder: WidgetBuilder,
    themes: Option<Rc<CapturedThemes>>,
    use_safe_area: bool,
    curved_animation: Option<Handle<CurvedAnimation>>,
    animation_style: Option<AnimationStyle>,
}

impl DialogRoute {
    /// A dialog route with Material entrance and exit animations; Dart's optional arguments
    /// are the setters.
    ///
    /// Dart's defaults: `barrier_color` `Colors::BLACK54`, `barrier_dismissible` true,
    /// `barrier_label` the context's `MaterialLocalizations::modal_barrier_dismiss_label`,
    /// `use_safe_area` true, and a transition of 150 ms.
    pub fn new(
        app: &mut App,
        context: BuildContext,
        builder: WidgetBuilder,
    ) -> Handle<DialogRoute> {
        let barrier_label =
            <dyn MaterialLocalizations>::of(app, context).modal_barrier_dismiss_label();
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let mut raw_dialog_route =
            RawDialogRouteData::new(dialog_page_builder(Rc::clone(&builder), None, true));
        raw_dialog_route.barrier_color = Some(Colors::BLACK54);
        raw_dialog_route.barrier_label = Some(barrier_label);
        raw_dialog_route.transition_duration = Duration::from_millis(150);
        raw_dialog_route.transition_builder = Some(Rc::new(build_material_dialog_transitions));
        app.create(DialogRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            raw_dialog_route,
            builder,
            themes: None,
            use_safe_area: true,
            curved_animation: None,
            animation_style: None,
        })
    }

    /// Dart `DialogRoute(themes:)`.
    pub fn themes(self: Handle<Self>, app: &mut App, themes: Rc<CapturedThemes>) -> Handle<Self> {
        app.get_mut(self).themes = Some(themes);
        self.build_page_builder(app);
        self
    }

    /// Dart `DialogRoute(barrierColor:)`.
    pub fn barrier_color(self: Handle<Self>, app: &mut App, barrier_color: Color) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_color = Some(barrier_color);
        self
    }

    /// Dart `DialogRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Handle<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `DialogRoute(barrierLabel:)`.
    pub fn barrier_label(self: Handle<Self>, app: &mut App, barrier_label: String) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_label = Some(barrier_label);
        self
    }

    /// Dart `DialogRoute(useSafeArea:)`.
    pub fn use_safe_area(self: Handle<Self>, app: &mut App, use_safe_area: bool) -> Handle<Self> {
        app.get_mut(self).use_safe_area = use_safe_area;
        self.build_page_builder(app);
        self
    }

    /// Dart `DialogRoute(settings:)`.
    pub fn settings(self: Handle<Self>, app: &mut App, settings: RouteSettingsRef) -> Handle<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `DialogRoute(requestFocus:)`.
    pub fn request_focus(self: Handle<Self>, app: &mut App, request_focus: bool) -> Handle<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `DialogRoute(traversalEdgeBehavior:)`.
    pub fn traversal_edge_behavior(
        self: Handle<Self>,
        app: &mut App,
        behavior: TraversalEdgeBehavior,
    ) -> Handle<Self> {
        self.modal_route_data_mut(app).traversal_edge_behavior = Some(behavior);
        self
    }

    /// Dart `DialogRoute(fullscreenDialog:)`.
    pub fn fullscreen_dialog(
        self: Handle<Self>,
        app: &mut App,
        fullscreen_dialog: bool,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `DialogRoute(animationStyle:)`, which also sets the transition's duration.
    pub fn animation_style(
        self: Handle<Self>,
        app: &mut App,
        animation_style: AnimationStyle,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).transition_duration = animation_style
            .duration
            .unwrap_or(Duration::from_millis(150));
        app.get_mut(self).animation_style = Some(animation_style);
        self
    }

    /// Makes Dart's page builder again over the constructor arguments as they are now.
    fn build_page_builder(self: Handle<Self>, app: &mut App) {
        let route = app.get(self);
        let page_builder = dialog_page_builder(
            Rc::clone(&route.builder),
            route.themes.clone(),
            route.use_safe_area,
        );
        self.raw_dialog_route_data_mut(app).page_builder = page_builder;
    }

    fn set_animation(self: Handle<Self>, app: &mut App, animation: AnyAnimation<f64>) {
        let current = app.get(self).curved_animation;
        if current.map(|curved| curved.parent(app)) != Some(animation) {
            if let Some(curved) = current {
                curved.dispose(app);
                app.destroy_lingering(curved);
            }
            let style = app.get(self).animation_style.clone();
            let curve = style
                .as_ref()
                .and_then(|style| style.curve.clone())
                .unwrap_or_else(Curves::ease_out);
            let reverse_curve = style
                .as_ref()
                .and_then(|style| style.reverse_curve.clone())
                .unwrap_or_else(Curves::ease_out);
            let curved = CurvedAnimation::create(app, animation, curve, Some(reverse_curve));
            app.get_mut(self).curved_animation = Some(curved);
        }
    }
}

impl Route for DialogRoute {
    waft_widgets::route_accessors!();

    fn as_transition_route(self: Handle<Self>) -> Option<waft_widgets::AnyTransitionRoute> {
        Some(TransitionRoute::as_transition_route(self))
    }

    fn as_modal_route(self: Handle<Self>) -> Option<waft_widgets::AnyModalRoute> {
        Some(ModalRoute::as_modal_route(self))
    }

    fn overlay_entries(self: Handle<Self>, app: &App) -> Vec<Handle<waft_widgets::OverlayEntry>> {
        OverlayRoute::overlay_entries(self, app)
    }

    fn install(self: Handle<Self>, app: &mut App) {
        ModalRoute::install(self, app)
    }

    fn did_push(self: Handle<Self>, app: &mut App) -> waft_scheduler::TickerFuture {
        ModalRoute::did_push(self, app)
    }

    fn did_add(self: Handle<Self>, app: &mut App) {
        ModalRoute::did_add(self, app)
    }

    fn did_replace(self: Handle<Self>, app: &mut App, old_route: Option<AnyRoute>) {
        TransitionRoute::did_replace(self, app, old_route)
    }

    fn will_pop(self: Handle<Self>, app: &mut App) -> waft_widgets::RoutePopDisposition {
        LocalHistoryRoute::will_pop(self, app)
    }

    fn pop_disposition(self: Handle<Self>, app: &mut App) -> waft_widgets::RoutePopDisposition {
        ModalRoute::pop_disposition(self, app)
    }

    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    ) {
        ModalRoute::on_pop_invoked_with_result(self, app, did_pop, result)
    }

    fn will_handle_pop_internally(self: Handle<Self>, app: &App) -> bool {
        LocalHistoryRoute::will_handle_pop_internally(self, app)
    }

    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        LocalHistoryRoute::did_pop(self, app, result)
    }

    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: AnyRoute) {
        ModalRoute::did_pop_next(self, app, next_route)
    }

    fn did_change_next(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        ModalRoute::did_change_next(self, app, next_route)
    }

    fn did_change_previous(self: Handle<Self>, app: &mut App, previous_route: Option<AnyRoute>) {
        ModalRoute::did_change_previous(self, app, previous_route)
    }

    fn changed_internal_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_internal_state(self, app)
    }

    fn changed_external_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_external_state(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(curved) = app.get(self).curved_animation {
            curved.dispose(app);
            app.destroy_lingering(curved);
        }
        TransitionRoute::dispose(self, app)
    }
}

impl OverlayRoute for DialogRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for DialogRoute {
    waft_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for DialogRoute {
    waft_widgets::raw_dialog_route_transition_route_overrides!();
}

impl LocalHistoryRoute for DialogRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for DialogRoute {
    waft_widgets::raw_dialog_route_modal_route_overrides!();
}

impl PopupRoute for DialogRoute {}

impl RawDialogRouteLeaf for DialogRoute {
    waft_widgets::raw_dialog_route_accessors!();

    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        self.set_animation(app, animation);
        let curved = app
            .get(self)
            .curved_animation
            .expect("set_animation made it");
        FadeTransition::new(curved.as_animation())
            .child(RawDialogRoute::build_transitions(
                self,
                app,
                context,
                animation,
                secondary_animation,
                child,
            ))
            .into_widget()
    }
}

fn scale_padding(text_scale_factor: f64) -> f64 {
    let clamped_text_scale_factor = text_scale_factor.clamp(1.0, 2.0);
    // The final padding scale factor is clamped between 1/3 and 1. For example,
    // a non-scaled padding of 24 will produce a padding between 24 and 8.
    lerp_double(Some(1.0), Some(1.0 / 3.0), clamped_text_scale_factor - 1.0)
        .expect("both ends are given")
}

/// Dart's `_DialogDefaultsM2`: hand coded defaults based on Material Design 2.
struct DialogDefaultsM2 {
    /// Dart's `super(..)` arguments.
    dialog_theme_data: DialogThemeData,
    context: DefaultsContext,
}

impl DialogDefaultsM2 {
    fn new(context: BuildContext) -> DialogDefaultsM2 {
        DialogDefaultsM2 {
            dialog_theme_data: DialogThemeData::new()
                .alignment(AlignmentGeometry::CENTER)
                .elevation(24.0)
                .shape(RoundedRectangleBorder::new(
                    waft_painting::BorderSide::NONE,
                    waft_painting::BorderRadiusGeometry::all(waft_embedder::Radius::circular(4.0)),
                ))
                .clip_behavior(Clip::None),
            context: DefaultsContext::new(context),
        }
    }
}

impl DialogThemeDataGetters for DialogDefaultsM2 {
    fn dialog_theme_data(&self) -> &DialogThemeData {
        &self.dialog_theme_data
    }

    fn icon_color(&self, app: &mut App) -> Option<Color> {
        self.context
            .theme(app)
            .icon_theme
            .color
            .as_ref()
            .map(|color| color.color())
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(
            if self.context.theme(app).brightness() == Brightness::Dark {
                Colors::GREY.shade800()
            } else {
                Colors::WHITE
            },
        )
    }

    fn shadow_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.theme(app).shadow_color)
    }

    fn title_text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_large.clone()
    }

    fn content_text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_medium.clone()
    }

    fn actions_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsetsGeometry::ZERO)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/dialog_test.dart`; the semantics-tree cases wait for the semantics
    //! tree, the goldens are not ported, and the display-feature cases wait for
    //! `DisplayFeatureSubScreen`.

    use std::cell::Cell;
    use waft_painting::ShapeBorder;

    use waft_embedder::{Offset, Radius, Rect, Size};
    use waft_painting::{Border, BorderRadiusGeometry, BorderSide};
    use waft_rendering::RenderParagraph;
    use waft_widgets::{AnyElement, Center, Container, GlobalKey, MediaQueryData, SizedBox, Text};

    use super::*;
    use crate::elevated_button::ElevatedButton;
    use crate::test_support::{WidgetTester, with_main_thread_stack};
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app, tester};

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(radius)),
        )
    }

    /// Dart's `_defaultM2DialogShape`.
    fn default_m2_dialog_shape() -> ShapeBorderRef {
        ShapeBorderRef::new(rounded(4.0))
    }

    /// Dart's `_defaultM3DialogShape`.
    fn default_m3_dialog_shape() -> ShapeBorderRef {
        ShapeBorderRef::new(rounded(28.0))
    }

    /// Dart's `material3Theme`.
    fn material3_theme(app: &App) -> ThemeData {
        ThemeData::builder().brightness(Brightness::Dark).build(app)
    }

    /// Dart's `material2Theme`.
    fn material2_theme(app: &App) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .brightness(Brightness::Dark)
            .build(app)
    }

    /// `showDialog`'s optional arguments with Dart's defaults.
    #[derive(Clone)]
    struct Show {
        barrier_dismissible: bool,
        barrier_color: Option<Color>,
        barrier_label: Option<String>,
        use_safe_area: bool,
        use_root_navigator: bool,
        route_settings: Option<RouteSettingsRef>,
        traversal_edge_behavior: Option<TraversalEdgeBehavior>,
        fullscreen_dialog: bool,
        request_focus: Option<bool>,
        animation_style: Option<AnimationStyle>,
    }

    impl Default for Show {
        fn default() -> Show {
            Show {
                barrier_dismissible: true,
                barrier_color: None,
                barrier_label: None,
                use_safe_area: true,
                use_root_navigator: true,
                route_settings: None,
                traversal_edge_behavior: None,
                fullscreen_dialog: false,
                request_focus: None,
                animation_style: None,
            }
        }
    }

    impl Show {
        fn show(
            self,
            app: &mut App,
            context: BuildContext,
            builder: WidgetBuilder,
        ) -> CompleterFuture<RouteResult> {
            show_dialog(
                app,
                context,
                builder,
                self.barrier_dismissible,
                self.barrier_color,
                self.barrier_label,
                self.use_safe_area,
                self.use_root_navigator,
                self.route_settings,
                self.traversal_edge_behavior,
                self.fullscreen_dialog,
                self.request_focus,
                self.animation_style,
            )
        }

        fn on(
            self,
            tester: &WidgetTester,
            context: BuildContext,
            builder: WidgetBuilder,
        ) -> CompleterFuture<RouteResult> {
            let future = self.show(&mut tester.cell.borrow_mut(), context, builder);
            tester.cell.checkpoint();
            future
        }
    }

    /// Dart's `_buildAppWithDialog(dialog, theme:, textScaleFactor:, traversalEdgeBehavior:)`.
    fn app_with_dialog(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        dialog: WidgetRef,
        text_scale_factor: f64,
        traversal_edge_behavior: Option<TraversalEdgeBehavior>,
    ) {
        pump_app(tester, theme, move |_app| {
            Material::new()
                .child(Builder::new(move |_app, context| {
                    let dialog = dialog.clone();
                    Center::new()
                        .child(ElevatedButton::new(
                            Some(waft_foundation::Listener::new(move |app| {
                                let dialog = dialog.clone();
                                let show = Show {
                                    traversal_edge_behavior,
                                    ..Show::default()
                                };
                                drop(show.show(
                                    app,
                                    context,
                                    Rc::new(move |_app, _context| {
                                        MediaQuery::with_clamped_text_scaling(
                                            None,
                                            text_scale_factor,
                                            text_scale_factor,
                                            dialog.clone(),
                                        )
                                    }),
                                ));
                            })),
                            Some(Text::new("X")),
                        ))
                        .into_widget()
                }))
                .into_widget()
        });
    }

    /// [`app_with_dialog`] with Dart's defaults.
    fn pump_with_dialog<K>(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        dialog: impl IntoWidget<K>,
    ) {
        app_with_dialog(tester, theme, dialog.into_widget(), 1.0, None);
    }

    /// Dart's `tester.tap(find.text('X'))` then `tester.pumpAndSettle()`.
    pub(crate) fn open(tester: &mut WidgetTester) {
        tap_text(tester, "X");
        tester.pump_and_settle();
    }

    pub(crate) fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    pub(crate) fn rect_of(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.get_rect(tester.render_box_of(element))
    }

    /// Dart's `tester.getRect(find.text(text))`.
    fn text_rect(tester: &WidgetTester, text: &str) -> Rect {
        rect_of(tester, only(tester.find_onstage_text(text)))
    }

    /// Dart's `find.text(text)` with `findsOneWidget` or `findsNothing`.
    pub(crate) fn shows(tester: &WidgetTester, text: &str) -> usize {
        tester.find_onstage_text(text).len()
    }

    /// Dart's `tester.tap(find.text(text))`.
    pub(crate) fn tap_text(tester: &mut WidgetTester, text: &str) {
        let center = text_rect(tester, text).center();
        tester.tap(center, 1);
    }

    /// Dart's `tester.tapAt(at)`.
    pub(crate) fn tap_at(tester: &mut WidgetTester, at: Offset) {
        tester.tap(at, 1);
    }

    pub(crate) fn rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        tester.get_rect(tester.render_object(key).as_box().expect("a render box"))
    }

    /// Dart's `find.descendant(of: find.byType(Dialog), matching: find.byType(Material))`, the
    /// first.
    pub(crate) fn dialog_material(tester: &WidgetTester) -> AnyElement {
        let dialog = tester.find_by_type::<Dialog>()[0];
        tester.descendants_of_type::<Material>(dialog)[0]
    }

    /// Dart's `_getMaterialFromDialog(tester)`, read through `read`.
    pub(crate) fn material_from_dialog<R>(
        tester: &WidgetTester,
        read: impl FnOnce(&Material) -> R,
    ) -> R {
        tester.widget::<Material, R>(dialog_material(tester), read)
    }

    /// Dart's `_getTextRenderObjectFromDialog(tester, text).text.style`.
    pub(crate) fn text_style_in_dialog(tester: &WidgetTester, text: &str) -> Option<TextStyle> {
        let dialog = tester.find_by_type::<Dialog>()[0];
        let element = tester
            .find_text(text)
            .into_iter()
            .find(|element| tester.ancestors(*element).contains(&dialog))
            .expect("the text is in the dialog");
        let app = tester.cell.borrow();
        let paragraph = element
            .find_render_object(&app)
            .and_then(|object| object.downcast::<RenderParagraph>(&app))
            .expect("a paragraph");
        paragraph.text(&app).style().cloned()
    }

    /// Dart's `_findOverflowBar()`: the `Padding` the `OverflowBar` is the child of.
    fn overflow_bar_padding(tester: &WidgetTester) -> Rect {
        let bar = tester.find_by_type::<OverflowBar>()[0];
        rect_of(tester, tester.ancestor_of_type::<Padding>(bar))
    }

    fn green_box(height: f64, width: f64) -> WidgetRef {
        Container::new()
            .height(height)
            .width(width)
            .color(Colors::GREEN.shade500())
            .into_widget()
    }

    /// Dart's `MaterialApp(home: Material(child: Center(child: ElevatedButton(onPressed: null,
    /// child: Text(text)))))`, and the context of the text.
    fn app_with_disabled_button(tester: &mut WidgetTester, text: &'static str) -> BuildContext {
        pump_app(tester, m3, move |_app| {
            Material::new()
                .child(Center::new().child(ElevatedButton::new(None, Some(Text::new(text)))))
                .into_widget()
        });
        tester.find_text(text)[0]
    }

    /// Records the value `future` resolves with: Dart's `await result`.
    fn result_of<T: Clone + 'static>(
        tester: &WidgetTester,
        future: CompleterFuture<RouteResult>,
    ) -> Rc<Cell<Option<Option<T>>>> {
        let result = Rc::new(Cell::new(None));
        let out = Rc::clone(&result);
        future.then(&mut tester.cell.borrow_mut(), move |_app, value| {
            out.set(Some(
                value
                    .as_ref()
                    .and_then(|value| value.downcast_ref::<T>().cloned()),
            ));
        });
        result
    }

    /// Not a Dart case: what Flutter's leak tracking checks, that opening and closing a dialog
    /// leaves nothing in the arena. Until the ownership migration is done, what it still
    /// leaves is written in.
    #[test]
    fn opening_and_closing_a_dialog_leaves_only_the_known_leak() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_with_dialog(
                &mut tester,
                m3,
                AlertDialog::new().content(Text::new("Body")),
            );
            // What the cycle leaves today; each step of the ownership migration
            // (experiments/2026-11-15-waft-ownership) brings these down to none.
            tester.assert_leaves_behind(
                &[
                    ("waft_animation::animation_controller::AnimationController", 1),
                    ("waft_animation::animations::ProxyAnimation", 3),
                    ("waft_foundation::change_notifier::ValueNotifier<bool>", 1),
                    ("waft_foundation::change_notifier::ValueNotifier<core::option::Option<alloc::string::String>>", 1),
                    ("waft_foundation::change_notifier::ValueNotifier<core::option::Option<waft_foundation::app::Handle<waft_widgets::widgets::overlay::OverlayEntryWidgetState>>>", 2),
                    ("waft_material::dialog::DialogRoute", 1),
                    ("waft_scheduler::ticker::Ticker", 6),
                    ("waft_widgets::widgets::focus_manager::FocusAttachment", 1),
                    ("waft_widgets::widgets::focus_manager::FocusScopeNode", 1),
                    ("waft_widgets::widgets::navigator::RouteEntry", 1),
                    ("waft_widgets::widgets::overlay::OverlayEntry", 2),
                    ("waft_widgets::widgets::page_storage::PageStorageBucket", 1),
                    ("waft_widgets::widgets::scroll_controller::ScrollController", 1),
                ],
                |tester| {
                    open(tester);
                    let body = only(tester.find_onstage_text("Body"));
                    Navigator::pop(&mut tester.cell.borrow_mut(), body, None);
                    tester.pump_and_settle();
                    assert_eq!(shows(tester, "Body"), 0);
                },
            );
        });
    }

    #[test]
    fn dialog_is_scrollable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let did_press_ok = Rc::new(Cell::new(false));
            let pressed = Rc::clone(&did_press_ok);
            let dialog = AlertDialog::new()
                .content(green_box(5000.0, 300.0))
                .actions([TextButton::new(
                    Some(waft_foundation::Listener::new(move |_app| {
                        pressed.set(true)
                    })),
                    Some(Text::new("OK")),
                )
                .into_widget()]);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert!(!did_press_ok.get());
            tap_text(&mut tester, "OK");
            assert!(did_press_ok.get());
        });
    }

    #[test]
    fn dialog_background_color_from_alert_dialog() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_color = Colors::PINK.primary();
            let dialog = AlertDialog::new()
                .background_color(custom_color)
                .actions([]);
            pump_with_dialog(
                &mut tester,
                |app| ThemeData::builder().brightness(Brightness::Dark).build(app),
                dialog,
            );

            open(&mut tester);

            assert_eq!(
                material_from_dialog(&tester, |material| material.color),
                Some(custom_color)
            );
        });
    }

    #[test]
    #[allow(
        deprecated,
        reason = "Dart's case sets the deprecated `background` too"
    )]
    fn dialog_background_defaults_to_color_scheme_surface_container_high() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = {
                let app = tester.cell.borrow();
                let color_scheme = ThemeData::new(&app)
                    .color_scheme
                    .copy_with()
                    .with_surface(Colors::ORANGE.primary())
                    .with_background(Colors::GREEN.primary())
                    .with_surface_container_high(Colors::RED.primary());
                ThemeData::builder().color_scheme(color_scheme).build(&app)
            };
            let expected = theme.color_scheme.surface_container_high();
            let dialog = Dialog::new().child(SizedBox::new().width(200.0).height(200.0));
            pump_with_dialog(&mut tester, move |_app| theme, dialog);

            open(&mut tester);

            assert_eq!(
                material_from_dialog(&tester, |material| material.color),
                Some(expected)
            );
        });
    }

    fn assert_shape(tester: &WidgetTester, expected: &dyn ShapeBorder) {
        material_from_dialog(tester, |material| {
            assert_eq!(material.shape.as_deref(), Some(expected));
        });
    }

    #[test]
    fn material2_dialog_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = AlertDialog::new()
                .title(Text::new("Title"))
                .content(Text::new("Y"))
                .actions([]);
            pump_with_dialog(&mut tester, material2_theme, dialog);

            open(&mut tester);

            assert_eq!(
                material_from_dialog(&tester, |material| material.color),
                Some(Colors::GREY.shade800())
            );
            assert_shape(&tester, &*default_m2_dialog_shape());
            assert_eq!(
                material_from_dialog(&tester, |material| material.elevation),
                24.0
            );

            let bottom_left = rect_of(&tester, dialog_material(&tester)).bottom_left();
            assert_eq!(bottom_left.dy(), 360.0);
        });
    }

    #[test]
    fn material3_dialog_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let expected = material3_theme(&tester.cell.borrow())
                .color_scheme
                .surface_container_high();
            let dialog = AlertDialog::new()
                .title(Text::new("Title"))
                .content(Text::new("Y"))
                .actions([]);
            pump_with_dialog(&mut tester, material3_theme, dialog);

            open(&mut tester);

            assert_eq!(
                material_from_dialog(&tester, |material| material.color),
                Some(expected)
            );
            assert_shape(&tester, &*default_m3_dialog_shape());
            assert_eq!(
                material_from_dialog(&tester, |material| material.elevation),
                6.0
            );
        });
    }

    fn dismiss_with_escape(tester: &mut WidgetTester) {
        tester.send_key_event(
            waft_services::PhysicalKeyboardKey::ESCAPE,
            waft_services::LogicalKeyboardKey::ESCAPE,
        );
        tester.pump_and_settle();
    }

    #[test]
    fn material2_dialog_fullscreen_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog_text_m2 = "Fullscreen Dialog - M2";
            pump_with_dialog(
                &mut tester,
                material2_theme,
                Dialog::fullscreen().child(Text::new(dialog_text_m2)),
            );

            open(&mut tester);

            assert_eq!(shows(&tester, dialog_text_m2), 1);
            assert_eq!(
                material_from_dialog(&tester, |material| material.color),
                Some(Colors::GREY.shade800())
            );

            // Try to dismiss the fullscreen dialog with the escape key.
            dismiss_with_escape(&mut tester);

            assert_eq!(shows(&tester, dialog_text_m2), 0);
        });
    }

    #[test]
    fn material3_dialog_fullscreen_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog_text_m3 = "Fullscreen Dialog - M3";
            let expected = material3_theme(&tester.cell.borrow()).color_scheme.surface;
            pump_with_dialog(
                &mut tester,
                material3_theme,
                Dialog::fullscreen().child(Text::new(dialog_text_m3)),
            );

            open(&mut tester);

            assert_eq!(shows(&tester, dialog_text_m3), 1);
            assert_eq!(
                material_from_dialog(&tester, |material| material.color),
                Some(expected)
            );

            // Try to dismiss the fullscreen dialog with the escape key.
            dismiss_with_escape(&mut tester);

            assert_eq!(shows(&tester, dialog_text_m3), 0);
        });
    }

    #[test]
    fn custom_dialog_elevation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_elevation = 12.0;
            let shadow_color = Color::new(0xFF000001);
            let surface_tint_color = Color::new(0xFF000002);
            let dialog = AlertDialog::new()
                .actions([])
                .elevation(custom_elevation)
                .shadow_color(shadow_color)
                .surface_tint_color(surface_tint_color);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            material_from_dialog(&tester, |material| {
                assert_eq!(material.elevation, custom_elevation);
                assert_eq!(material.shadow_color, Some(shadow_color));
                assert_eq!(material.surface_tint_color, Some(surface_tint_color));
            });
        });
    }

    #[test]
    fn custom_title_text_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let title_text = "Title";
            let title_text_style = TextStyle::new().color(Colors::PINK.primary());
            let dialog = AlertDialog::new()
                .title(Text::new(title_text))
                .title_text_style(title_text_style.clone())
                .actions([]);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_eq!(
                text_style_in_dialog(&tester, title_text),
                Some(title_text_style)
            );
        });
    }

    #[test]
    fn custom_content_text_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content_text = "Content";
            let content_text_style = TextStyle::new().color(Colors::PINK.primary());
            let dialog = AlertDialog::new()
                .content(Text::new(content_text))
                .content_text_style(content_text_style.clone())
                .actions([]);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_eq!(
                text_style_in_dialog(&tester, content_text),
                Some(content_text_style)
            );
        });
    }

    #[test]
    fn alert_dialog_custom_clip_behavior() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = AlertDialog::new()
                .actions([])
                .clip_behavior(Clip::AntiAlias);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_eq!(
                material_from_dialog(&tester, |material| material.clip_behavior),
                Clip::AntiAlias
            );
        });
    }

    #[test]
    fn simple_dialog_custom_clip_behavior() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = SimpleDialog::new()
                .clip_behavior(Clip::AntiAlias)
                .children([]);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_eq!(
                material_from_dialog(&tester, |material| material.clip_behavior),
                Clip::AntiAlias
            );
        });
    }

    #[test]
    fn simple_dialog_custom_content_text_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content_text = "Content";
            let content_text_style = TextStyle::new().color(Colors::PINK.primary());
            let dialog = SimpleDialog::new()
                .content_text_style(content_text_style.clone())
                .children([Text::new(content_text).into_widget()]);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_eq!(
                text_style_in_dialog(&tester, content_text),
                Some(content_text_style)
            );
        });
    }

    #[test]
    fn simple_dialog_custom_content_text_style_dialog_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content_text = "Content";
            let content_text_style = TextStyle::new().color(Colors::ORANGE.primary());
            let dialog = SimpleDialog::new().children([Text::new(content_text).into_widget()]);
            let theme_style = content_text_style.clone();
            pump_with_dialog(
                &mut tester,
                move |app| {
                    ThemeData::builder()
                        .dialog_theme(DialogThemeData::new().content_text_style(theme_style))
                        .build(app)
                },
                dialog,
            );

            open(&mut tester);

            assert_eq!(
                text_style_in_dialog(&tester, content_text),
                Some(content_text_style)
            );
        });
    }

    #[test]
    fn material3_simple_dialog_custom_content_text_style_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content_text = "Content";
            let content_text_style = TextStyle::new().color(Colors::PURPLE.primary());
            let dialog = SimpleDialog::new().children([Text::new(content_text).into_widget()]);
            let theme_style = content_text_style.clone();
            pump_with_dialog(
                &mut tester,
                move |app| {
                    ThemeData::builder()
                        .text_theme(crate::text_theme::TextTheme::new().body_medium(theme_style))
                        .build(app)
                },
                dialog,
            );

            open(&mut tester);

            assert_eq!(
                text_style_in_dialog(&tester, content_text).and_then(|style| style.color),
                content_text_style.color
            );
        });
    }

    #[test]
    fn material2_simple_dialog_custom_content_text_style_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content_text = "Content";
            let content_text_style = TextStyle::new().color(Colors::TEAL.primary());
            let dialog = SimpleDialog::new().children([Text::new(content_text).into_widget()]);
            let theme_style = content_text_style.clone();
            pump_with_dialog(
                &mut tester,
                move |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .text_theme(crate::text_theme::TextTheme::new().title_medium(theme_style))
                        .build(app)
                },
                dialog,
            );

            open(&mut tester);

            assert_eq!(
                text_style_in_dialog(&tester, content_text).and_then(|style| style.color),
                content_text_style.color
            );
        });
    }

    #[test]
    fn custom_dialog_shape() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_border = rounded(16.0);
            let dialog = AlertDialog::new().actions([]).shape(custom_border);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_shape(&tester, &custom_border);
        });
    }

    #[test]
    fn null_dialog_shape() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = AlertDialog::new().actions([]);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_shape(&tester, &*default_m3_dialog_shape());
        });
    }

    #[test]
    fn rectangular_dialog_shape() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_border = Border::default();
            let dialog = AlertDialog::new().actions([]).shape(custom_border);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            assert_shape(&tester, &custom_border);
        });
    }

    #[test]
    fn custom_dialog_alignment() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = AlertDialog::new()
                .actions([])
                .alignment(AlignmentGeometry::BOTTOM_LEFT);
            pump_with_dialog(&mut tester, m3, dialog);

            open(&mut tester);

            let bottom_left = rect_of(&tester, dialog_material(&tester)).bottom_left();
            assert_eq!(bottom_left.dx(), 40.0);
            assert_eq!(bottom_left.dy(), 576.0);
        });
    }

    /// Dart's `SimpleDialog(title: Text('Title'), children: [option(42), Text('Second
    /// option')])` whose first option pops 42.
    fn simple_dialog_popping_42() -> WidgetBuilder {
        Rc::new(|_app, context| {
            SimpleDialog::new()
                .title(Text::new("Title"))
                .children([
                    SimpleDialogOption::new()
                        .on_pressed(waft_foundation::Listener::new(move |app| {
                            Navigator::pop(app, context, Some(Rc::new(42_i32)));
                        }))
                        .child(Text::new("First option"))
                        .into_widget(),
                    SimpleDialogOption::new()
                        .child(Text::new("Second option"))
                        .into_widget(),
                ])
                .into_widget()
        })
    }

    #[test]
    fn simple_dialog_control_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let context = app_with_disabled_button(&mut tester, "Go");

            let result = result_of::<i32>(
                &tester,
                Show::default().on(&tester, context, simple_dialog_popping_42()),
            );

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Title"), 1);
            tap_text(&mut tester, "First option");

            assert_eq!(result.get(), Some(Some(42)));
        });
    }

    #[test]
    fn can_show_dialog_using_navigator_global_key() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let navigator = GlobalKey::new();
            let root = {
                let app = tester.cell.borrow();
                crate::app::MaterialApp::new()
                    .theme(m3(&app))
                    .navigator_key(navigator.clone())
                    .home(Material::new().child(Center::new().child(Text::new("Go"))))
                    .into_widget()
            };
            tester.pump_widget(root);

            let context = navigator
                .current_context(&tester.cell.borrow())
                .expect("the navigator is mounted");
            let result = result_of::<i32>(
                &tester,
                Show::default().on(&tester, context, simple_dialog_popping_42()),
            );

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Title"), 1);
            tap_text(&mut tester, "First option");

            assert_eq!(result.get(), Some(Some(42)));
        });
    }

    #[test]
    fn custom_padding_on_simple_dialog_option() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_padding = EdgeInsets::from_ltrb(4.0, 10.0, 8.0, 6.0);
            let dialog =
                SimpleDialog::new()
                    .title(Text::new("Title"))
                    .children([SimpleDialogOption::new()
                        .on_pressed(waft_foundation::Listener::new(|_app| {}))
                        .padding(custom_padding)
                        .child(Text::new("First option"))
                        .into_widget()]);
            pump_with_dialog(&mut tester, m3, dialog);
            open(&mut tester);

            let dialog_rect = rect_of(
                &tester,
                only(tester.find_onstage_by_type::<SimpleDialogOption>()),
            );
            let text_rect = text_rect(&tester, "First option");

            assert_eq!(text_rect.left, dialog_rect.left + custom_padding.left);
            assert_eq!(text_rect.top, dialog_rect.top + custom_padding.top);
            assert_eq!(text_rect.right, dialog_rect.right - custom_padding.right);
            assert_eq!(text_rect.bottom, dialog_rect.bottom - custom_padding.bottom);
        });
    }

    /// Dart's `Container(width: 100.0, height: 100.0, alignment: Alignment.center, child:
    /// Text(text))`.
    fn hundred_square(text: &'static str) -> WidgetBuilder {
        Rc::new(move |_app, _context| {
            Container::new()
                .width(100.0)
                .height(100.0)
                .alignment(AlignmentGeometry::CENTER)
                .child(Text::new(text))
                .into_widget()
        })
    }

    #[test]
    fn barrier_dismissible() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let context = app_with_disabled_button(&mut tester, "Go");

            drop(Show::default().on(&tester, context, hundred_square("Dialog1")));

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog1"), 1);

            // Tap on the barrier.
            tap_at(&mut tester, Offset::new(10.0, 10.0));

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog1"), 0);

            let show = Show {
                barrier_dismissible: false,
                ..Show::default()
            };
            drop(show.on(&tester, context, hundred_square("Dialog2")));

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog2"), 1);

            // Tap on the barrier, which shouldn't do anything this time.
            tap_at(&mut tester, Offset::new(10.0, 10.0));

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog2"), 1);
        });
    }

    pub(crate) fn last_barrier_color(tester: &WidgetTester) -> Option<Color> {
        let barriers = tester.find_by_type::<waft_widgets::ModalBarrier>();
        tester.widget::<waft_widgets::ModalBarrier, _>(
            *barriers.last().expect("a modal barrier"),
            |barrier| barrier.color,
        )
    }

    #[test]
    fn barrier_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new().child(Text::new("Test")).into_widget()
            });
            let context = tester.find_text("Test")[0];
            let dialog: WidgetBuilder = Rc::new(|_app, _context| Text::new("Dialog").into_widget());

            // Test default barrier color
            drop(Show::default().on(&tester, context, Rc::clone(&dialog)));
            tester.pump_and_settle();
            assert_eq!(last_barrier_color(&tester), Some(Colors::BLACK54));

            // Dismiss it and test a custom barrier color
            tap_at(&mut tester, Offset::new(10.0, 10.0));
            let show = Show {
                barrier_color: Some(Colors::PINK.primary()),
                ..Show::default()
            };
            drop(show.on(&tester, context, dialog));
            tester.pump_and_settle();
            assert_eq!(last_barrier_color(&tester), Some(Colors::PINK.primary()));
        });
    }

    /// Dart's `expect(actual, expected)` and `moreOrLessEquals` on positions that went through
    /// a render object's transform: `Matrix4` is valo's f32 matrix, so they carry f32 rounding
    /// that Dart's f64 has not, and nothing more.
    fn assert_near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= f64::from(f32::EPSILON) * expected.abs().max(1.0),
            "{actual} is not {expected}"
        );
    }

    /// Dart's `ElevatedButton(key: key, onPressed: () {}, child: Text(text))`.
    fn button(key: Option<&GlobalKey>, text: &str) -> WidgetRef {
        let button = ElevatedButton::new(
            Some(waft_foundation::Listener::new(|_app| {})),
            Some(Text::new(text)),
        );
        match key {
            Some(key) => button.key(key.clone().into()).into_widget(),
            None => button.into_widget(),
        }
    }

    /// The size of the first `Material` under the `AlertDialog`: the dialog itself.
    fn alert_dialog_material_size(tester: &WidgetTester) -> Size {
        let dialog = tester.find_by_type::<AlertDialog>()[0];
        rect_of(tester, tester.descendants_of_type::<Material>(dialog)[0]).size()
    }

    #[test]
    fn alert_dialog_actions_padding_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = AlertDialog::new()
                .title(Text::new("title"))
                .content(Text::new("content"))
                .actions([button(None, "button")]);
            pump_with_dialog(&mut tester, m3, dialog);
            open(&mut tester);

            // The [AlertDialog] is the entire screen, since it also contains the scrim.
            // The first [Material] child of [AlertDialog] is the actual dialog
            // itself.
            let dialog_size = alert_dialog_material_size(&tester);
            let actions_size = overflow_bar_padding(&tester).size();

            assert_eq!(actions_size.width(), dialog_size.width());
        });
    }

    #[test]
    fn alert_dialog_actions_padding_surrounds_actions_with_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = AlertDialog::new()
                .title(Text::new("title"))
                .content(Text::new("content"))
                .actions([button(None, "button")])
                // The OverflowBar is inset by the buttonPadding/2 + actionsPadding
                .button_padding(EdgeInsetsGeometry::ZERO)
                .actions_padding(EdgeInsetsGeometry::all(30.0));
            pump_with_dialog(&mut tester, m3, dialog);
            open(&mut tester);

            let dialog_size = alert_dialog_material_size(&tester);
            let actions_size = rect_of(&tester, tester.find_by_type::<OverflowBar>()[0]).size();

            assert_eq!(actions_size.width(), dialog_size.width() - (30.0 * 2.0));
        });
    }

    /// Dart's `AlertDialog.buttonPadding` cases: `spacing` between the buttons and
    /// `top`, `bottom` and `right` between them and the bar's padding edges.
    fn check_button_padding(
        theme: impl FnOnce(&App) -> ThemeData,
        button_padding: Option<EdgeInsetsGeometry>,
        spacing: f64,
        top: f64,
        bottom: f64,
        right: f64,
    ) {
        let mut tester = tester();
        let key1 = GlobalKey::new();
        let key2 = GlobalKey::new();
        let mut dialog = AlertDialog::new()
            .title(Text::new("title"))
            .content(Text::new("content"))
            .actions([
                button(Some(&key1), "button 1"),
                button(Some(&key2), "button 2"),
            ]);
        dialog.button_padding = button_padding;
        pump_with_dialog(&mut tester, theme, dialog);
        open(&mut tester);

        let rect1 = rect_of_key(&tester, &key1);
        let rect2 = rect_of_key(&tester, &key2);
        let bar = overflow_bar_padding(&tester);

        // Padding between both buttons
        assert_near(
            rect2.bottom_left().dx(),
            rect1.bottom_right().dx() + spacing,
        );

        // Padding between button and edges of the button bar
        // First button
        assert_near(rect1.top_right().dy(), bar.top_right().dy() + top);
        assert_near(rect1.bottom_right().dy(), bar.bottom_right().dy() - bottom);

        // Second button
        assert_near(rect2.top_right().dy(), bar.top_right().dy() + top);
        assert_near(rect2.bottom_right().dy(), bar.bottom_right().dy() - bottom);
        assert_near(rect2.bottom_right().dx(), bar.bottom_right().dx() - right);
    }

    #[test]
    fn material2_alert_dialog_button_padding_defaults() {
        with_main_thread_stack(|| {
            check_button_padding(material2_theme, None, 8.0, 8.0, 8.0, 8.0);
        });
    }

    #[test]
    fn material3_alert_dialog_button_padding_defaults() {
        with_main_thread_stack(|| {
            check_button_padding(material3_theme, None, 8.0, 0.0, 24.0, 24.0);
        });
    }

    #[test]
    fn alert_dialog_button_padding_custom_values() {
        with_main_thread_stack(|| {
            let half = (10.0 + 20.0) / 2.0;
            check_button_padding(
                m2,
                Some(EdgeInsetsGeometry::only(10.0, 0.0, 20.0, 0.0)),
                half,
                half,
                half,
                half,
            );
        });
    }

    /// Dart's `Dialog children padding is correct` group.
    mod children_padding {
        use waft_widgets::Icon;

        use super::*;
        use crate::icons::Icons;

        const TEXT_SCALE_FACTORS: [f64; 5] = [0.5, 1.0, 1.5, 2.0, 3.0];

        fn padding_scale_factor(text_scale_factor: f64) -> f64 {
            match text_scale_factor {
                0.5 | 1.0 => 1.0,
                1.5 => 2.0 / 3.0,
                _ => 1.0 / 3.0,
            }
        }

        /// The group's keys and widgets, made for each case.
        struct Parts {
            icon_key: GlobalKey,
            title_key: GlobalKey,
            content_key: GlobalKey,
            children_key: GlobalKey,
        }

        impl Parts {
            fn new() -> Parts {
                Parts {
                    icon_key: GlobalKey::new(),
                    title_key: GlobalKey::new(),
                    content_key: GlobalKey::new(),
                    children_key: GlobalKey::new(),
                }
            }

            fn icon(&self) -> WidgetRef {
                Icon::new(Some(Icons::ac_unit()))
                    .key(self.icon_key.clone().into())
                    .into_widget()
            }

            fn title(&self) -> WidgetRef {
                Text::new("title")
                    .key(self.title_key.clone().into())
                    .into_widget()
            }

            fn content(&self) -> WidgetRef {
                Text::new("content")
                    .key(self.content_key.clone().into())
                    .into_widget()
            }

            fn actions(&self) -> Vec<WidgetRef> {
                vec![button(None, "button")]
            }

            fn children(&self) -> Vec<WidgetRef> {
                vec![
                    SimpleDialogOption::new()
                        .key(self.children_key.clone().into())
                        .child(Text::new("child"))
                        .on_pressed(waft_foundation::Listener::new(|_app| {}))
                        .into_widget(),
                ]
            }
        }

        /// What the group's finders find.
        enum Part<'a> {
            Key(&'a GlobalKey),
            Actions,
        }

        fn rect(tester: &WidgetTester, part: Part<'_>) -> Rect {
            match part {
                Part::Key(key) => rect_of_key(tester, key),
                Part::Actions => overflow_bar_padding(tester),
            }
        }

        fn dialog_rect(tester: &WidgetTester) -> Rect {
            rect_of(tester, dialog_material(tester))
        }

        /// Dart's `openDialog(tester, dialog, textScaleFactor, isM3:)`.
        fn open_dialog<K>(
            dialog: impl IntoWidget<K>,
            text_scale_factor: f64,
            is_m3: bool,
        ) -> WidgetTester {
            let mut tester = tester();
            app_with_dialog(
                &mut tester,
                move |app| ThemeData::builder().use_material3(is_m3).build(app),
                dialog.into_widget(),
                text_scale_factor,
                None,
            );
            open(&mut tester);
            tester
        }

        fn expect_left_edge_padding(
            tester: &WidgetTester,
            finder: Part<'_>,
            text_scale_factor: f64,
            unscaled_value: f64,
        ) {
            let dialog = dialog_rect(tester);
            let found = rect(tester, finder);
            let padding = unscaled_value * padding_scale_factor(text_scale_factor);
            assert_near(dialog.top_left().dx(), found.top_left().dx() - padding);
            assert_near(
                dialog.bottom_left().dx(),
                found.bottom_left().dx() - padding,
            );
        }

        fn expect_right_edge_padding(
            tester: &WidgetTester,
            finder: Part<'_>,
            text_scale_factor: f64,
            unscaled_value: f64,
        ) {
            let dialog = dialog_rect(tester);
            let found = rect(tester, finder);
            let padding = unscaled_value * padding_scale_factor(text_scale_factor);
            assert_near(dialog.top_right().dx(), found.top_right().dx() + padding);
            assert_near(
                dialog.bottom_right().dx(),
                found.bottom_right().dx() + padding,
            );
        }

        fn expect_top_edge_padding(
            tester: &WidgetTester,
            finder: Part<'_>,
            text_scale_factor: f64,
            unscaled_value: f64,
        ) {
            let dialog = dialog_rect(tester);
            let found = rect(tester, finder);
            let padding = unscaled_value * padding_scale_factor(text_scale_factor);
            assert_near(dialog.top_left().dy(), found.top_left().dy() - padding);
            assert_near(dialog.top_right().dy(), found.top_right().dy() - padding);
        }

        fn expect_bottom_edge_padding(
            tester: &WidgetTester,
            finder: Part<'_>,
            text_scale_factor: f64,
            unscaled_value: f64,
        ) {
            let dialog = dialog_rect(tester);
            let found = rect(tester, finder);
            let padding = unscaled_value * padding_scale_factor(text_scale_factor);
            assert_near(
                dialog.bottom_left().dy(),
                found.bottom_right().dy() + padding,
            );
            assert_near(
                dialog.bottom_right().dy(),
                found.bottom_right().dy() + padding,
            );
        }

        fn expect_vertical_inner_padding(
            tester: &WidgetTester,
            top: Part<'_>,
            bottom: Part<'_>,
            value: f64,
        ) {
            let top = rect(tester, top);
            let bottom = rect(tester, bottom);
            assert_near(top.bottom_left().dy(), bottom.top_left().dy() - value);
            assert_near(top.bottom_right().dy(), bottom.top_right().dy() - value);
        }

        fn expect_actions_at_the_edges(tester: &WidgetTester, text_scale_factor: f64) {
            expect_left_edge_padding(tester, Part::Actions, text_scale_factor, 0.0);
            expect_right_edge_padding(tester, Part::Actions, text_scale_factor, 0.0);
            expect_bottom_edge_padding(tester, Part::Actions, text_scale_factor, 0.0);
        }

        #[test]
        fn alert_dialog_padding_is_correct_when_only_icon_and_actions_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    let parts = Parts::new();
                    let dialog = AlertDialog::new()
                        .icon(parts.icon())
                        .actions(parts.actions());
                    let tester = open_dialog(dialog, text_scale_factor, false);
                    let icon = || Part::Key(&parts.icon_key);

                    expect_top_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                    expect_left_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, icon(), Part::Actions, 24.0);
                    expect_actions_at_the_edges(&tester, text_scale_factor);
                }
            });
        }

        #[test]
        fn alert_dialog_padding_is_correct_when_only_icon_title_and_actions_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    let parts = Parts::new();
                    let dialog = AlertDialog::new()
                        .icon(parts.icon())
                        .title(parts.title())
                        .actions(parts.actions());
                    let tester = open_dialog(dialog, text_scale_factor, false);
                    let icon = || Part::Key(&parts.icon_key);
                    let title = || Part::Key(&parts.title_key);

                    expect_top_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                    expect_left_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, icon(), title(), 16.0);
                    expect_left_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, title(), Part::Actions, 20.0);
                    expect_actions_at_the_edges(&tester, text_scale_factor);
                }
            });
        }

        #[test]
        fn alert_dialog_padding_is_correct_when_only_icon_content_and_actions_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    for is_m3 in [true, false] {
                        let parts = Parts::new();
                        let dialog = AlertDialog::new()
                            .icon(parts.icon())
                            .content(parts.content())
                            .actions(parts.actions());
                        let tester = open_dialog(dialog, text_scale_factor, is_m3);
                        let icon = || Part::Key(&parts.icon_key);
                        let content = || Part::Key(&parts.content_key);

                        expect_top_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                        expect_left_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                        expect_right_edge_padding(&tester, icon(), text_scale_factor, 24.0);
                        expect_vertical_inner_padding(
                            &tester,
                            icon(),
                            content(),
                            if is_m3 { 16.0 } else { 20.0 },
                        );
                        expect_left_edge_padding(&tester, content(), text_scale_factor, 24.0);
                        expect_right_edge_padding(&tester, content(), text_scale_factor, 24.0);
                        expect_vertical_inner_padding(&tester, content(), Part::Actions, 24.0);
                        expect_actions_at_the_edges(&tester, text_scale_factor);
                    }
                }
            });
        }

        #[test]
        fn alert_dialog_padding_is_correct_when_only_title_and_actions_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    let parts = Parts::new();
                    let dialog = AlertDialog::new()
                        .title(parts.title())
                        .actions(parts.actions());
                    let tester = open_dialog(dialog, text_scale_factor, false);
                    let title = || Part::Key(&parts.title_key);

                    expect_top_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_left_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, title(), Part::Actions, 20.0);
                    expect_actions_at_the_edges(&tester, text_scale_factor);
                }
            });
        }

        #[test]
        fn alert_dialog_padding_is_correct_when_only_content_and_actions_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    let parts = Parts::new();
                    let dialog = AlertDialog::new()
                        .content(parts.content())
                        .actions(parts.actions());
                    let tester = open_dialog(dialog, text_scale_factor, false);
                    let content = || Part::Key(&parts.content_key);

                    expect_top_edge_padding(&tester, content(), text_scale_factor, 20.0);
                    expect_left_edge_padding(&tester, content(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, content(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, content(), Part::Actions, 24.0);
                    expect_actions_at_the_edges(&tester, text_scale_factor);
                }
            });
        }

        #[test]
        fn alert_dialog_padding_is_correct_when_title_content_and_actions_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    let parts = Parts::new();
                    let dialog = AlertDialog::new()
                        .title(parts.title())
                        .content(parts.content())
                        .actions(parts.actions());
                    let tester = open_dialog(dialog, text_scale_factor, false);
                    let title = || Part::Key(&parts.title_key);
                    let content = || Part::Key(&parts.content_key);

                    expect_top_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_left_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, title(), content(), 20.0);
                    expect_left_edge_padding(&tester, content(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, content(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, content(), Part::Actions, 24.0);
                    expect_actions_at_the_edges(&tester, text_scale_factor);
                }
            });
        }

        #[test]
        fn simple_dialog_padding_is_correct_when_only_children_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    let parts = Parts::new();
                    let dialog = SimpleDialog::new().children(parts.children());
                    let tester = open_dialog(dialog, text_scale_factor, false);
                    let children = || Part::Key(&parts.children_key);

                    expect_top_edge_padding(&tester, children(), text_scale_factor, 12.0);
                    expect_left_edge_padding(&tester, children(), text_scale_factor, 0.0);
                    expect_right_edge_padding(&tester, children(), text_scale_factor, 0.0);
                    expect_bottom_edge_padding(&tester, children(), text_scale_factor, 16.0);
                }
            });
        }

        #[test]
        fn simple_dialog_padding_is_correct_when_title_and_children_are_specified() {
            with_main_thread_stack(|| {
                for text_scale_factor in TEXT_SCALE_FACTORS {
                    let parts = Parts::new();
                    let dialog = SimpleDialog::new()
                        .title(parts.title())
                        .children(parts.children());
                    let tester = open_dialog(dialog, text_scale_factor, false);
                    let title = || Part::Key(&parts.title_key);
                    let children = || Part::Key(&parts.children_key);

                    expect_top_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_left_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_right_edge_padding(&tester, title(), text_scale_factor, 24.0);
                    expect_vertical_inner_padding(&tester, title(), children(), 12.0);
                    expect_left_edge_padding(&tester, children(), text_scale_factor, 0.0);
                    expect_right_edge_padding(&tester, children(), text_scale_factor, 0.0);
                    expect_bottom_edge_padding(&tester, children(), text_scale_factor, 16.0);
                }
            });
        }
    }

    /// Dart's two actions with long labels, keyed.
    fn long_buttons(
        key1: &GlobalKey,
        key2: &GlobalKey,
        label1: &str,
        label2: &str,
    ) -> [WidgetRef; 2] {
        [button(Some(key1), label1), button(Some(key2), label2)]
    }

    fn overflowing_dialog(key1: &GlobalKey, key2: &GlobalKey) -> AlertDialog {
        AlertDialog::new()
            .title(Text::new("title"))
            .content(Text::new("content"))
            .actions(long_buttons(
                key1,
                key2,
                "Looooooooooooooong button 1",
                "Looooooooooooooong button 2",
            ))
    }

    #[test]
    fn dialogs_can_set_the_vertical_direction_of_overflowing_actions() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
            let dialog =
                overflowing_dialog(&key1, &key2).actions_overflow_direction(VerticalDirection::Up);
            pump_with_dialog(&mut tester, m3, dialog);
            open(&mut tester);

            let button_one_rect = rect_of_key(&tester, &key1);
            let button_two_rect = rect_of_key(&tester, &key2);
            // Second [ElevatedButton] should appear above the first.
            assert!(button_two_rect.bottom <= button_one_rect.top);
        });
    }

    #[test]
    fn dialogs_have_no_spacing_by_default_for_overflowing_actions() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
            pump_with_dialog(&mut tester, m3, overflowing_dialog(&key1, &key2));
            open(&mut tester);

            let button_one_rect = rect_of_key(&tester, &key1);
            let button_two_rect = rect_of_key(&tester, &key2);
            assert_near(button_one_rect.bottom, button_two_rect.top);
        });
    }

    #[test]
    fn dialogs_can_set_the_button_spacing_of_overflowing_actions() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
            let dialog = overflowing_dialog(&key1, &key2).actions_overflow_button_spacing(10.0);
            pump_with_dialog(&mut tester, m3, dialog);
            open(&mut tester);

            let button_one_rect = rect_of_key(&tester, &key1);
            let button_two_rect = rect_of_key(&tester, &key2);
            assert_near(button_one_rect.bottom, button_two_rect.top - 10.0);
        });
    }

    #[test]
    fn dialogs_can_set_the_alignment_of_the_overflow_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
            let dialog = AlertDialog::new()
                .title(Text::new("title"))
                .content(Text::new("content"))
                .actions(long_buttons(
                    &key1,
                    &key2,
                    "Loooooooooong button 1",
                    "Loooooooooooooonger button 2",
                ))
                .actions_overflow_alignment(OverflowBarAlignment::Center);
            pump_with_dialog(&mut tester, m3, dialog);
            open(&mut tester);

            let button_one_rect = rect_of_key(&tester, &key1);
            let button_two_rect = rect_of_key(&tester, &key2);
            assert_near(button_one_rect.center().dx(), button_two_rect.center().dx());
        });
    }

    /// Dart's `Localizations(locale: Locale('en', 'US'), delegates: [widgets, material],
    /// child: child)`.
    fn localized(child: WidgetRef) -> WidgetRef {
        waft_widgets::Localizations::new(
            waft_embedder::Locale::new("en").country_code("US"),
            vec![
                waft_widgets::DefaultWidgetsLocalizations::delegate(),
                crate::material_localizations::DefaultMaterialLocalizations::delegate(),
            ],
        )
        .child(child)
        .into_widget()
    }

    /// A navigator whose first route is a `PageRouteBuilder` building `page`.
    fn navigator_of_page(page: WidgetBuilder) -> Navigator {
        Navigator::new().on_generate_route(move |app, _settings| {
            let page = Rc::clone(&page);
            let page_builder: RoutePageBuilder =
                Rc::new(move |app, context, _animation, _secondary_animation| page(app, context));
            Some(PageRouteBuilder::new(app, page_builder).as_route())
        })
    }

    /// A context `builder` saw: Dart's `late BuildContext`.
    fn context_cell() -> Rc<Cell<Option<BuildContext>>> {
        Rc::new(Cell::new(None))
    }

    fn seen(context: &Cell<Option<BuildContext>>) -> BuildContext {
        context.get().expect("the builder ran")
    }

    /// Dart's `Placeholder()` where its constraints are bounded: a box filling them, keyed so a
    /// case can find it (Waft has no `Placeholder`).
    pub(crate) fn placeholder(key: &GlobalKey) -> WidgetRef {
        SizedBox::expand().key(key.clone().into()).into_widget()
    }

    #[test]
    fn dialogs_removes_media_query_padding_and_view_insets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let outer_context = context_cell();
            let route_context = context_cell();
            let dialog_context = context_cell();
            let outer = Rc::clone(&outer_context);
            tester.pump_widget(localized(
                MediaQuery::new(
                    MediaQueryData::new()
                        .padding(EdgeInsets::all(50.0))
                        .view_insets(EdgeInsets::only(25.0, 0.0, 0.0, 75.0)),
                    navigator_of_page(Rc::new(move |_app, context| {
                        outer.set(Some(context));
                        Container::new().into_widget()
                    })),
                )
                .into_widget(),
            ));

            let route = Rc::clone(&route_context);
            let dialog = Rc::clone(&dialog_context);
            let show = Show {
                barrier_dismissible: false,
                ..Show::default()
            };
            drop(show.on(
                &tester,
                seen(&outer_context),
                Rc::new(move |_app, context| {
                    route.set(Some(context));
                    let dialog = Rc::clone(&dialog);
                    Dialog::new()
                        .child(Builder::new(move |_app, context| {
                            dialog.set(Some(context));
                            SizedBox::expand().into_widget()
                        }))
                        .into_widget()
                }),
            ));

            tester.pump(Duration::ZERO);

            let mut app = tester.cell.borrow_mut();
            let padding = |app: &mut App, context: &Cell<Option<BuildContext>>| {
                MediaQuery::of(app, seen(context)).padding
            };
            let view_insets = |app: &mut App, context: &Cell<Option<BuildContext>>| {
                MediaQuery::of(app, seen(context)).view_insets
            };
            assert_eq!(padding(&mut app, &outer_context), EdgeInsets::all(50.0));
            assert_eq!(padding(&mut app, &route_context), EdgeInsets::ZERO);
            assert_eq!(padding(&mut app, &dialog_context), EdgeInsets::ZERO);
            assert_eq!(
                view_insets(&mut app, &outer_context),
                EdgeInsets::only(25.0, 0.0, 0.0, 75.0)
            );
            assert_eq!(
                view_insets(&mut app, &route_context),
                EdgeInsets::only(25.0, 0.0, 0.0, 75.0)
            );
            assert_eq!(view_insets(&mut app, &dialog_context), EdgeInsets::ZERO);
        });
    }

    #[test]
    fn dialog_widget_insets_by_view_insets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new()
                        .view_insets(EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0)),
                    Dialog::new().child(placeholder(&key)),
                )
                .into_widget(),
            );
            let inset = Rect::from_ltrb(
                10.0 + 40.0,
                20.0 + 24.0,
                800.0 - (40.0 + 30.0),
                600.0 - (24.0 + 40.0),
            );
            assert_eq!(rect_of_key(&tester, &key), inset);
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new(),
                    Dialog::new().child(placeholder(&key)),
                )
                .into_widget(),
            );
            // no change because this is an animation
            assert_eq!(rect_of_key(&tester, &key), inset);
            tester.pump(Duration::from_secs(1));
            // animation finished
            assert_eq!(
                rect_of_key(&tester, &key),
                Rect::from_ltrb(40.0, 24.0, 800.0 - 40.0, 600.0 - 24.0)
            );
        });
    }

    #[test]
    fn dialog_inset_padding_added_to_outside_of_dialog() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            // The default testing screen (800, 600)
            let screen_rect = Rect::from_ltrb(0.0, 0.0, 800.0, 600.0);

            // Test with no padding.
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new(),
                    Dialog::new()
                        .inset_padding(EdgeInsets::ZERO)
                        .child(placeholder(&key)),
                )
                .into_widget(),
            );
            tester.pump_and_settle();
            assert_eq!(rect_of_key(&tester, &key), screen_rect);

            // Test with an insetPadding.
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new(),
                    Dialog::new()
                        .inset_padding(EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0))
                        .child(placeholder(&key)),
                )
                .into_widget(),
            );
            tester.pump_and_settle();
            assert_eq!(
                rect_of_key(&tester, &key),
                Rect::from_ltrb(
                    screen_rect.left + 10.0,
                    screen_rect.top + 20.0,
                    screen_rect.right - 30.0,
                    screen_rect.bottom - 40.0,
                )
            );
        });
    }

    #[test]
    fn dialog_fullscreen_material_extends_behind_the_keyboard_child_stays_above_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let child_context = context_cell();
            let dialog = |key: &GlobalKey, child_context: &Rc<Cell<Option<BuildContext>>>| {
                let key = key.clone();
                let child_context = Rc::clone(child_context);
                Dialog::fullscreen().child(Builder::new(move |_app, context| {
                    child_context.set(Some(context));
                    placeholder(&key)
                }))
            };
            let material_rect = |tester: &WidgetTester| rect_of(tester, dialog_material(tester));

            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new().view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 200.0)),
                    dialog(&key, &child_context),
                )
                .into_widget(),
            );
            tester.pump_and_settle();
            // The child is kept above the keyboard and does not see the inset.
            assert_eq!(
                rect_of_key(&tester, &key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 400.0)
            );
            assert_eq!(
                MediaQuery::view_insets_of(&mut tester.cell.borrow_mut(), seen(&child_context)),
                EdgeInsets::ZERO
            );
            // The dialog's surface keeps covering the whole screen, including the
            // area behind the keyboard.
            assert_eq!(
                material_rect(&tester),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0)
            );

            // Hiding the keyboard gives the child the whole screen back.
            tester.pump_widget(
                MediaQuery::new(MediaQueryData::new(), dialog(&key, &child_context)).into_widget(),
            );
            tester.pump_and_settle();
            assert_eq!(
                rect_of_key(&tester, &key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0)
            );
            assert_eq!(
                material_rect(&tester),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0)
            );
        });
    }

    /// Dart's `AlertDialog.scrollable` cases: drags `drag` by `dy` and expects every one of
    /// `moved` to have moved by `dy` from where it was.
    fn drag_key(tester: &mut WidgetTester, key: &GlobalKey, dy: f64) {
        let start = tester.center_of(key);
        tester.drag_from(start, Offset::new(0.0, dy), 1);
    }

    fn origin_of(tester: &WidgetTester, key: &GlobalKey) -> Offset {
        rect_of_key(tester, key).top_left()
    }

    fn keyed_box(key: &GlobalKey, color: Color, height: f64) -> WidgetRef {
        Container::new()
            .key(key.clone().into())
            .color(color)
            .height(height)
            .into_widget()
    }

    mod alert_dialog_scrollable {
        use super::*;

        #[test]
        fn title_is_scrollable() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let title_key = GlobalKey::new();
                let dialog = AlertDialog::new()
                    .title(keyed_box(&title_key, Colors::GREEN.primary(), 1000.0))
                    .scrollable(true);
                pump_with_dialog(&mut tester, m3, dialog);
                open(&mut tester);

                let original_offset = origin_of(&tester, &title_key);
                drag_key(&mut tester, &title_key, -200.0);
                assert_eq!(
                    origin_of(&tester, &title_key),
                    original_offset.translate(0.0, -200.0)
                );
            });
        }

        #[test]
        fn content_is_scrollable() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let content_key = GlobalKey::new();
                let dialog = AlertDialog::new()
                    .content(keyed_box(&content_key, Colors::ORANGE.primary(), 1000.0))
                    .scrollable(true);
                pump_with_dialog(&mut tester, m3, dialog);
                open(&mut tester);

                let original_offset = origin_of(&tester, &content_key);
                drag_key(&mut tester, &content_key, -200.0);
                assert_eq!(
                    origin_of(&tester, &content_key),
                    original_offset.translate(0.0, -200.0)
                );
            });
        }

        #[test]
        fn title_and_content_are_scrollable() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let title_key = GlobalKey::new();
                let content_key = GlobalKey::new();
                let dialog = AlertDialog::new()
                    .title(keyed_box(&title_key, Colors::GREEN.primary(), 400.0))
                    .content(keyed_box(&content_key, Colors::ORANGE.primary(), 400.0))
                    .scrollable(true);
                pump_with_dialog(&mut tester, m3, dialog);
                open(&mut tester);

                let title_original_offset = origin_of(&tester, &title_key);
                let content_original_offset = origin_of(&tester, &content_key);

                // Dragging the title widget should scroll both the title
                // and the content widgets.
                drag_key(&mut tester, &title_key, -200.0);
                assert_eq!(
                    origin_of(&tester, &title_key),
                    title_original_offset.translate(0.0, -200.0)
                );
                assert_eq!(
                    origin_of(&tester, &content_key),
                    content_original_offset.translate(0.0, -200.0)
                );

                // Dragging the content widget should scroll both the title
                // and the content widgets.
                drag_key(&mut tester, &content_key, 200.0);
                assert_eq!(origin_of(&tester, &title_key), title_original_offset);
                assert_eq!(origin_of(&tester, &content_key), content_original_offset);
            });
        }
    }

    /// Dart's `_ClosureNavigatorObserver.onDidChange`.
    type OnDidChange = Rc<dyn Fn(&mut App, AnyRoute)>;

    /// Dart's `_ClosureNavigatorObserver`.
    struct ClosureNavigatorObserver {
        navigator_observer: waft_widgets::NavigatorObserverData,
        on_did_change: OnDidChange,
    }

    impl waft_widgets::NavigatorObserver for ClosureNavigatorObserver {
        waft_widgets::navigator_observer_accessors!();

        fn did_push(
            self: Handle<Self>,
            app: &mut App,
            route: AnyRoute,
            _previous: Option<AnyRoute>,
        ) {
            let on_did_change = Rc::clone(&app.get(self).on_did_change);
            on_did_change(app, route);
        }

        fn did_pop(
            self: Handle<Self>,
            app: &mut App,
            _route: AnyRoute,
            previous: Option<AnyRoute>,
        ) {
            let on_did_change = Rc::clone(&app.get(self).on_did_change);
            on_did_change(app, previous.expect("a previous route"));
        }

        fn did_remove(
            self: Handle<Self>,
            app: &mut App,
            _route: AnyRoute,
            previous: Option<AnyRoute>,
        ) {
            let on_did_change = Rc::clone(&app.get(self).on_did_change);
            on_did_change(app, previous.expect("a previous route"));
        }

        fn did_replace(
            self: Handle<Self>,
            app: &mut App,
            new_route: Option<AnyRoute>,
            _old_route: Option<AnyRoute>,
        ) {
            let on_did_change = Rc::clone(&app.get(self).on_did_change);
            on_did_change(app, new_route.expect("a new route"));
        }
    }

    #[test]
    fn dialog_with_route_settings() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let current_route_setting: Rc<std::cell::RefCell<Option<RouteSettingsRef>>> =
                Rc::default();
            let current = Rc::clone(&current_route_setting);
            let root = {
                let mut app = tester.cell.borrow_mut();
                let observer = app.create(ClosureNavigatorObserver {
                    navigator_observer: waft_widgets::NavigatorObserverData::new(),
                    on_did_change: Rc::new(move |app, new_route| {
                        *current.borrow_mut() = Some(new_route.settings(app));
                    }),
                });
                crate::app::MaterialApp::new()
                    .theme(m3(&app))
                    .navigator_observers([waft_widgets::NavigatorObserver::as_observer(observer)])
                    .home(Material::new().child(
                        Center::new().child(ElevatedButton::new(None, Some(Text::new("Go")))),
                    ))
                    .into_widget()
            };
            tester.pump_widget(root);

            let context = tester.find_text("Go")[0];
            let example_setting =
                RouteSettingsRef::Settings(waft_widgets::RouteSettings::default().name("simple"));

            let show = Show {
                route_settings: Some(example_setting.clone()),
                ..Show::default()
            };
            let result = result_of::<i32>(
                &tester,
                show.on(
                    &tester,
                    context,
                    Rc::new(|_app, context| {
                        SimpleDialog::new()
                            .title(Text::new("Title"))
                            .children([SimpleDialogOption::new()
                                .child(Text::new("X"))
                                .on_pressed(waft_foundation::Listener::new(move |app| {
                                    Navigator::of(app, context, false).pop(app, None);
                                }))
                                .into_widget()])
                            .into_widget()
                    }),
                ),
            );

            tester.pump_and_settle();
            assert_eq!(shows(&tester, "Title"), 1);
            assert_eq!(
                current_route_setting.borrow().clone(),
                Some(example_setting)
            );

            tap_text(&mut tester, "X");
            tester.pump_and_settle();

            assert_eq!(result.get(), Some(None));
            tester.pump_and_settle();
            assert_eq!(
                current_route_setting
                    .borrow()
                    .as_ref()
                    .and_then(|settings| settings.name().map(str::to_owned)),
                Some("/".to_owned())
            );
        });
    }

    #[test]
    fn alert_dialog_actions_alignment() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let action_key = GlobalKey::new();
            let build_frame = |tester: &mut WidgetTester, alignment: Option<MainAxisAlignment>| {
                let action_key = action_key.clone();
                pump_app(tester, m2, move |_app| {
                    let mut dialog = AlertDialog::new()
                        .content(SizedBox::new().width(800.0))
                        .actions([SizedBox::new()
                            .key(action_key.into())
                            .width(20.0)
                            .height(20.0)
                            .into_widget()])
                        .button_padding(EdgeInsetsGeometry::ZERO)
                        .inset_padding(EdgeInsets::ZERO);
                    dialog.actions_alignment = alignment;
                    crate::scaffold::Scaffold::new().body(dialog).into_widget()
                });
            };
            let alert_dialog =
                |tester: &WidgetTester| rect_of(tester, tester.find_by_type::<AlertDialog>()[0]);
            let action = |tester: &WidgetTester| rect_of_key(tester, &action_key);

            // Default configuration
            build_frame(&mut tester, None);
            assert_eq!(alert_dialog(&tester).top_left().dx(), 0.0);
            assert_eq!(alert_dialog(&tester).top_right().dx(), 800.0);
            assert_eq!(
                rect_of(&tester, tester.find_by_type::<OverflowBar>()[0]).width(),
                800.0
            );
            assert_eq!(action(&tester).top_left().dx(), 800.0 - 20.0);
            assert_eq!(action(&tester).top_right().dx(), 800.0);

            // All possible alignment values
            let centered = ((800.0 - 20.0) / 2.0, (800.0 - 20.0) / 2.0 + 20.0);
            for (alignment, (left, right)) in [
                (MainAxisAlignment::Start, (0.0, 20.0)),
                (MainAxisAlignment::Center, centered),
                (MainAxisAlignment::End, (800.0 - 20.0, 800.0)),
                (MainAxisAlignment::SpaceBetween, (0.0, 20.0)),
                (MainAxisAlignment::SpaceAround, centered),
                (MainAxisAlignment::SpaceEvenly, centered),
            ] {
                build_frame(&mut tester, Some(alignment));
                assert_eq!(action(&tester).top_left().dx(), left, "{alignment:?}");
                assert_eq!(action(&tester).top_right().dx(), right, "{alignment:?}");
            }
        });
    }

    /// Dart's `Actions.invoke(primaryFocus!.context!, intent)! as bool` then `tester.pump()`.
    fn move_focus(tester: &mut WidgetTester, intent: &dyn waft_widgets::Intent) -> bool {
        let result = {
            let mut app = tester.cell.borrow_mut();
            let context = waft_widgets::primary_focus(&mut app)
                .and_then(|focus| focus.context(&app))
                .expect("a focused node with a context");
            waft_widgets::Actions::invoke(&mut app, context, intent)
        };
        tester.pump(Duration::ZERO);
        result
            .and_then(|result| result.downcast_ref::<bool>().copied())
            .expect("the focus actions answer whether focus moved")
    }

    /// The traversal cases' dialog: two buttons, OK and Cancel, with focus nodes of their own.
    fn ok_cancel_dialog(
        tester: &WidgetTester,
    ) -> (
        AlertDialog,
        waft_widgets::AnyFocusNode,
        waft_widgets::AnyFocusNode,
    ) {
        use waft_widgets::FocusNodeLeaf;
        let (ok_node, cancel_node) = {
            let mut app = tester.cell.borrow_mut();
            (
                waft_widgets::FocusNode::new(&mut app).as_node(),
                waft_widgets::FocusNode::new(&mut app).as_node(),
            )
        };
        let pressed = || Some(waft_foundation::Listener::new(|_app| {}));
        let dialog = AlertDialog::new()
            .content(Text::new("Test dialog"))
            .actions([
                TextButton::new(pressed(), Some(Text::new("OK")))
                    .focus_node(ok_node)
                    .into_widget(),
                TextButton::new(pressed(), Some(Text::new("Cancel")))
                    .focus_node(cancel_node)
                    .into_widget(),
            ]);
        (dialog, ok_node, cancel_node)
    }

    fn focus_of(tester: &WidgetTester, node: waft_widgets::AnyFocusNode) -> bool {
        node.has_focus(&mut tester.cell.borrow_mut())
    }

    #[test]
    fn uses_closed_loop_focus_traversal() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (dialog, ok_node, cancel_node) = ok_cancel_dialog(&tester);
            pump_with_dialog(&mut tester, m3, dialog);
            open(&mut tester);

            // Start at OK
            ok_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            assert!(focus_of(&tester, ok_node));
            assert!(!focus_of(&tester, cancel_node));

            for (intent, ok_focused) in [
                // OK -> Cancel
                (
                    &waft_widgets::NextFocusIntent as &dyn waft_widgets::Intent,
                    false,
                ),
                // Cancel -> OK
                (&waft_widgets::NextFocusIntent, true),
                // Cancel <- OK
                (&waft_widgets::PreviousFocusIntent, false),
                // OK <- Cancel
                (&waft_widgets::PreviousFocusIntent, true),
            ] {
                assert!(move_focus(&mut tester, intent));
                assert_eq!(focus_of(&tester, ok_node), ok_focused);
                assert_eq!(focus_of(&tester, cancel_node), !ok_focused);
            }
        });
    }

    #[test]
    fn uses_open_focus_traversal_when_overridden() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (dialog, ok_node, cancel_node) = ok_cancel_dialog(&tester);
            app_with_dialog(
                &mut tester,
                m3,
                dialog.into_widget(),
                1.0,
                Some(TraversalEdgeBehavior::LeaveFlutterView),
            );
            open(&mut tester);

            // Start at OK
            ok_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            assert!(focus_of(&tester, ok_node));
            assert!(!focus_of(&tester, cancel_node));

            // OK -> Cancel
            assert!(move_focus(&mut tester, &waft_widgets::NextFocusIntent));
            assert!(!focus_of(&tester, ok_node));
            assert!(focus_of(&tester, cancel_node));

            // Cancel -> nothing
            assert!(!move_focus(&mut tester, &waft_widgets::NextFocusIntent));
            assert!(!focus_of(&tester, ok_node));
            assert!(!focus_of(&tester, cancel_node));
        });
    }

    #[test]
    fn adaptive_alert_dialog_shows_correct_widget_on_each_platform() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dialog = AlertDialog::adaptive()
                .content(green_box(5000.0, 300.0))
                .actions([TextButton::new(
                    Some(waft_foundation::Listener::new(|_app| {})),
                    Some(Text::new("OK")),
                )
                .into_widget()])
                .into_widget();

            for (platform, cupertino) in [
                (TargetPlatform::IOS, 1),
                (TargetPlatform::MacOS, 1),
                (TargetPlatform::Android, 0),
                (TargetPlatform::Fuchsia, 0),
                (TargetPlatform::Linux, 0),
                (TargetPlatform::Windows, 0),
            ] {
                app_with_dialog(
                    &mut tester,
                    move |app| ThemeData::builder().platform(platform).build(app),
                    dialog.clone(),
                    1.0,
                    None,
                );
                tester.pump_and_settle();

                open(&mut tester);

                assert_eq!(
                    tester.find_onstage_by_type::<CupertinoAlertDialog>().len(),
                    cupertino,
                    "on {platform:?}"
                );

                tap_at(&mut tester, Offset::new(10.0, 10.0));
                tester.pump_and_settle();
            }
        });
    }

    /// Dart's `showAdaptiveDialog`'s optional arguments with Dart's defaults.
    fn show_adaptive(
        tester: &WidgetTester,
        context: BuildContext,
        builder: WidgetBuilder,
        animation_style: Option<AnimationStyle>,
    ) {
        let future = show_adaptive_dialog(
            &mut tester.cell.borrow_mut(),
            context,
            builder,
            None,
            None,
            None,
            true,
            true,
            None,
            None,
            None,
            animation_style,
        );
        drop(future);
        tester.cell.checkpoint();
    }

    #[test]
    fn show_adaptive_dialog_should_not_allow_dismiss_on_barrier_on_ios_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .platform(TargetPlatform::IOS)
                        .build(app)
                },
                |_app| {
                    Material::new()
                        .child(
                            Center::new().child(ElevatedButton::new(None, Some(Text::new("Go")))),
                        )
                        .into_widget()
                },
            );
            let context = tester.find_text("Go")[0];

            drop(Show::default().on(&tester, context, hundred_square("Dialog1")));

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog1"), 1);

            // Tap on the barrier.
            tap_at(&mut tester, Offset::new(10.0, 10.0));

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog1"), 0);

            show_adaptive(&tester, context, hundred_square("Dialog2"), None);

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog2"), 1);

            // Tap on the barrier, which shouldn't do anything this time.
            tap_at(&mut tester, Offset::new(10.0, 10.0));

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog2"), 1);
        });
    }

    #[test]
    fn applies_animation_style_to_show_adaptive_dialog() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let animation_style = AnimationStyle::new(
                Some(Curves::ease_in_out()),
                Some(Duration::from_secs(1)),
                None,
                None,
            );
            let context = app_with_disabled_button(&mut tester, "Go");
            show_adaptive(
                &tester,
                context,
                hundred_square("Dialog1"),
                Some(animation_style),
            );

            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog1"), 1);

            tap_at(&mut tester, Offset::new(10.0, 10.0));
            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(shows(&tester, "Dialog1"), 0);
        });
    }

    #[test]
    fn dialog_inset_padding_is_nullable() {
        let dialog = Dialog::new();
        assert_eq!(dialog.inset_padding, None);
    }

    #[test]
    fn alert_dialog_inset_padding_is_nullable() {
        let alert_dialog = AlertDialog::new();
        assert_eq!(alert_dialog.inset_padding, None);
    }

    #[test]
    fn simple_dialog_inset_padding_is_nullable() {
        let simple_dialog = SimpleDialog::new();
        assert_eq!(simple_dialog.inset_padding, None);
    }

    #[test]
    fn can_pass_a_null_value_to_alert_dialog_adaptive_clip_behavior() {
        for clip_behavior in [
            None,
            Some(Clip::None),
            Some(Clip::HardEdge),
            Some(Clip::AntiAlias),
            Some(Clip::AntiAliasWithSaveLayer),
        ] {
            let mut dialog = AlertDialog::adaptive();
            dialog.clip_behavior = clip_behavior;
            assert_eq!(dialog.clip_behavior, clip_behavior);
        }
    }

    fn has_focus_of(tester: &WidgetTester, node: Handle<waft_widgets::FocusNode>) -> bool {
        use waft_widgets::FocusNodeLeaf;
        node.as_node().has_focus(&mut tester.cell.borrow_mut())
    }

    fn request_focus_of(tester: &mut WidgetTester, node: Handle<waft_widgets::FocusNode>) {
        use waft_widgets::FocusNodeLeaf;
        node.as_node()
            .request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump(Duration::ZERO);
    }

    #[test]
    fn setting_dialog_route_request_focus_to_false_does_not_request_focus_on_the_dialog() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved_context = context_cell();
            let focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut());
            let dialog_text = "Dialog Text";
            let saved = Rc::clone(&saved_context);
            pump_app(&mut tester, m3, move |_app| {
                Material::new()
                    .child(Builder::new(move |_app, context| {
                        saved.set(Some(context));
                        crate::text_field::TextField::new()
                            .focus_node(focus_node)
                            .into_widget()
                    }))
                    .into_widget()
            });
            tester.pump(Duration::ZERO);

            // Initially, there is no dialog and the text field has no focus.
            assert_eq!(shows(&tester, dialog_text), 0);
            assert!(!has_focus_of(&tester, focus_node));

            // Request focus on the text field.
            request_focus_of(&mut tester, focus_node);
            assert!(has_focus_of(&tester, focus_node));

            // Bring up dialog.
            let navigator =
                Navigator::of(&mut tester.cell.borrow_mut(), seen(&saved_context), false);
            {
                let mut app = tester.cell.borrow_mut();
                let route = DialogRoute::new(
                    &mut app,
                    seen(&saved_context),
                    Rc::new(move |_app, _context| Text::new(dialog_text).into_widget()),
                );
                drop(navigator.push(&mut app, route.as_route()));
            }
            tester.pump(Duration::ZERO);

            // The dialog is showing and the text field has lost focus.
            assert_eq!(shows(&tester, dialog_text), 1);
            assert!(!has_focus_of(&tester, focus_node));

            // Dismiss the dialog.
            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);

            // The dialog is dismissed and the focus is shifted back to the text field.
            assert_eq!(shows(&tester, dialog_text), 0);
            assert!(has_focus_of(&tester, focus_node));

            // Bring up dialog again with requestFocus to false.
            {
                let mut app = tester.cell.borrow_mut();
                let route = crate::bottom_sheet::ModalBottomSheetRoute::new(
                    &mut app,
                    Rc::new(move |_app, _context| Text::new(dialog_text).into_widget()),
                    false,
                );
                route.request_focus(&mut app, false);
                drop(navigator.push(&mut app, Route::as_route(route)));
            }
            tester.pump(Duration::ZERO);

            // The dialog is showing and the text field still has focus.
            assert_eq!(shows(&tester, dialog_text), 1);
            assert!(has_focus_of(&tester, focus_node));
        });
    }

    #[test]
    fn request_focus_works_correctly_in_show_dialog() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let navigator_key = GlobalKey::new();
            let focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut());
            let root = {
                let app = tester.cell.borrow();
                crate::app::MaterialApp::new()
                    .theme(m3(&app))
                    .navigator_key(navigator_key.clone())
                    .home(
                        crate::scaffold::Scaffold::new()
                            .body(crate::text_field::TextField::new().focus_node(focus_node)),
                    )
                    .into_widget()
            };
            tester.pump_widget(root);
            request_focus_of(&mut tester, focus_node);
            assert!(has_focus_of(&tester, focus_node));

            let dialog_scope_has_focus = |tester: &WidgetTester| {
                use waft_widgets::FocusNodeLeaf;
                let element = tester.find_text("dialog")[0];
                let mut app = tester.cell.borrow_mut();
                let scope = waft_widgets::FocusScope::of(&mut app, element, true);
                scope.as_node().has_focus(&mut app)
            };
            let navigator_context = |tester: &WidgetTester| {
                navigator_key
                    .current_context(&tester.cell.borrow())
                    .expect("the navigator is mounted")
            };
            let dialog: WidgetBuilder = Rc::new(|_app, _context| Text::new("dialog").into_widget());

            let show = Show {
                request_focus: Some(true),
                ..Show::default()
            };
            drop(show.on(&tester, navigator_context(&tester), Rc::clone(&dialog)));
            tester.pump_and_settle();
            assert!(dialog_scope_has_focus(&tester));
            assert!(!has_focus_of(&tester, focus_node));

            {
                let mut app = tester.cell.borrow_mut();
                navigator_key
                    .current_state::<waft_widgets::NavigatorState>(&app)
                    .expect("the navigator is mounted")
                    .pop(&mut app, None);
            }
            tester.pump_and_settle();
            assert!(has_focus_of(&tester, focus_node));

            let show = Show {
                request_focus: Some(false),
                ..Show::default()
            };
            drop(show.on(&tester, navigator_context(&tester), dialog));
            tester.pump_and_settle();
            assert!(!dialog_scope_has_focus(&tester));
            assert!(has_focus_of(&tester, focus_node));
        });
    }

    /// The width of the only `SizedBox` in the dialog: Dart's
    /// `tester.getSize(find.byType(SizedBox)).width`.
    fn sized_box_width(tester: &WidgetTester, key: &GlobalKey) -> f64 {
        rect_of_key(tester, key).width()
    }

    #[test]
    fn dialog_respects_the_given_constraints() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_with_dialog(
                &mut tester,
                m3,
                Dialog::new()
                    .constraints(BoxConstraints::new().max_width(560.0))
                    .child(
                        SizedBox::new()
                            .key(key.clone().into())
                            .width(1000.0)
                            .height(100.0),
                    ),
            );
            open(&mut tester);

            assert_eq!(sized_box_width(&tester, &key), 560.0);
        });
    }

    #[test]
    fn alert_dialog_respects_the_default_constraints() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_with_dialog(
                &mut tester,
                m3,
                AlertDialog::new()
                    .content(SizedBox::new().key(key.clone().into()))
                    .content_padding(EdgeInsetsGeometry::ZERO),
            );
            open(&mut tester);

            assert_eq!(sized_box_width(&tester, &key), 280.0);
        });
    }

    #[test]
    fn alert_dialog_respects_the_given_constraints() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_with_dialog(
                &mut tester,
                m3,
                AlertDialog::new()
                    .constraints(BoxConstraints::new().max_width(560.0))
                    .content(
                        SizedBox::new()
                            .key(key.clone().into())
                            .width(1000.0)
                            .height(100.0),
                    )
                    .content_padding(EdgeInsetsGeometry::ZERO),
            );
            open(&mut tester);

            assert_eq!(sized_box_width(&tester, &key), 560.0);
        });
    }

    #[test]
    fn simple_dialog_respects_the_default_constraints() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_with_dialog(
                &mut tester,
                m3,
                SimpleDialog::new()
                    .content_padding(EdgeInsetsGeometry::ZERO)
                    .children([SizedBox::new().key(key.clone().into()).into_widget()]),
            );
            open(&mut tester);

            assert_eq!(sized_box_width(&tester, &key), 280.0);
        });
    }

    #[test]
    fn simple_dialog_respects_the_given_constraints() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_with_dialog(
                &mut tester,
                m3,
                SimpleDialog::new()
                    .constraints(BoxConstraints::new().max_width(560.0))
                    .content_padding(EdgeInsetsGeometry::ZERO)
                    .children([SizedBox::new()
                        .key(key.clone().into())
                        .width(1000.0)
                        .height(100.0)
                        .into_widget()]),
            );
            open(&mut tester);

            assert_eq!(sized_box_width(&tester, &key), 560.0);
        });
    }

    #[test]
    fn test_no_back_gesture_on_fullscreen_dialogs() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // no back button in app bar for RawDialogRoute with full screen dialog set to true
            pump_app(&mut tester, m3, |_app| {
                crate::scaffold::Scaffold::new()
                    .body(Builder::new(|_app, context| {
                        TextButton::new(
                            Some(waft_foundation::Listener::new(move |app| {
                                let page_builder: RoutePageBuilder =
                                    Rc::new(|_app, _context, _animation, _secondary| {
                                        crate::scaffold::Scaffold::new()
                                            .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                                                crate::app_bar::AppBar::new()
                                                    .title(Text::new("title")),
                                            ))
                                            .body(Text::new("body"))
                                            .into_widget()
                                    });
                                let route = RawDialogRoute::new(app, page_builder)
                                    .fullscreen_dialog(app, true);
                                drop(
                                    Navigator::of(app, context, false).push(app, route.as_route()),
                                );
                            })),
                            Some(Text::new("X")),
                        )
                        .into_widget()
                    }))
                    .into_widget()
            });

            open(&mut tester);

            assert_eq!(
                tester
                    .find_onstage_by_type::<crate::action_buttons::BackButton>()
                    .len(),
                0
            );
            assert_eq!(
                tester
                    .find_onstage_by_type::<crate::action_buttons::CloseButton>()
                    .len(),
                1
            );
        });
    }

    fn size_of_only<W: 'static>(tester: &WidgetTester) -> Size {
        rect_of(tester, only(tester.find_by_type::<W>())).size()
    }

    #[test]
    fn simple_dialog_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(SimpleDialog::new().title(Text::new("X"))))
                    .into_widget()
            });
            assert_eq!(size_of_only::<SimpleDialog>(&tester), Size::ZERO);
        });
    }

    #[test]
    fn simple_dialog_option_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Material::new()
                    .child(Center::new().child(
                        SizedBox::shrink().child(SimpleDialogOption::new().child(Text::new("X"))),
                    ))
                    .into_widget()
            });
            assert_eq!(size_of_only::<SimpleDialogOption>(&tester), Size::ZERO);
        });
    }

    #[test]
    fn dialog_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(Dialog::new().child(Text::new("X"))))
                    .into_widget()
            });
            assert!(size_of_only::<Dialog>(&tester).is_empty());
        });
    }

    #[test]
    fn alert_dialog_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(AlertDialog::new().content(Text::new("X"))))
                    .into_widget()
            });
            assert!(size_of_only::<AlertDialog>(&tester).is_empty());
        });
    }

    /// Dart's `tester.widget<Semantics>(find.ancestor(of: find.byType(Dialog), matching:
    /// find.byType(Semantics)).first).properties.hitTestBehavior`.
    fn route_semantics_hit_test_behavior(
        tester: &WidgetTester,
    ) -> Option<SemanticsHitTestBehavior> {
        let dialog = only(tester.find_by_type::<Dialog>());
        let semantics = tester.ancestor_of_type::<Semantics>(dialog);
        tester.widget::<Semantics, _>(semantics, |semantics| {
            semantics.properties.hit_test_behavior
        })
    }

    /// Dart's `MaterialApp(home: Material(child: Builder(builder: (context) =>
    /// ElevatedButton(onPressed: () => showDialog(context:, builder:), child: Text('Show
    /// Dialog')))))`.
    fn app_showing_on_press(tester: &mut WidgetTester, builder: WidgetBuilder) {
        pump_app(tester, m3, move |_app| {
            Material::new()
                .child(Builder::new(move |_app, context| {
                    let builder = Rc::clone(&builder);
                    ElevatedButton::new(
                        Some(waft_foundation::Listener::new(move |app| {
                            drop(Show::default().show(app, context, Rc::clone(&builder)));
                        })),
                        Some(Text::new("Show Dialog")),
                    )
                    .into_widget()
                }))
                .into_widget()
        });
    }

    #[test]
    fn dialog_has_hit_test_behavior_opaque_to_prevent_dismissal_on_empty_areas() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            app_showing_on_press(
                &mut tester,
                Rc::new(|_app, _context| {
                    Dialog::new()
                        .child(SizedBox::new().width(200.0).height(200.0))
                        .into_widget()
                }),
            );

            tap_text(&mut tester, "Show Dialog");
            tester.pump_and_settle();

            assert_eq!(tester.find_onstage_by_type::<Dialog>().len(), 1);
            assert_eq!(
                route_semantics_hit_test_behavior(&tester),
                Some(SemanticsHitTestBehavior::Opaque)
            );
        });
    }

    #[test]
    fn alert_dialog_has_hit_test_behavior_opaque_via_dialog() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            app_showing_on_press(
                &mut tester,
                Rc::new(|_app, context| {
                    AlertDialog::new()
                        .title(Text::new("Test Dialog"))
                        .content(SizedBox::new().width(200.0).height(100.0))
                        .actions([TextButton::new(
                            Some(waft_foundation::Listener::new(move |app| {
                                Navigator::pop(app, context, None);
                            })),
                            Some(Text::new("OK")),
                        )
                        .into_widget()])
                        .into_widget()
                }),
            );

            tap_text(&mut tester, "Show Dialog");
            tester.pump_and_settle();

            assert_eq!(tester.find_onstage_by_type::<AlertDialog>().len(), 1);
            assert_eq!(tester.find_onstage_by_type::<Dialog>().len(), 1);
            // Find the route-level Semantics with hitTestBehavior.opaque
            // (wraps the entire dialog content, above the Dialog widget)
            assert_eq!(
                route_semantics_hit_test_behavior(&tester),
                Some(SemanticsHitTestBehavior::Opaque)
            );
        });
    }

    #[test]
    fn dismissible_confirm_dismiss_defers_to_an_alert_dialog() {
        use std::cell::RefCell;
        use waft_widgets::{DismissDirection, Dismissible, ListView, StateSetter, StatefulBuilder};

        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            let dismissed_items: Rc<RefCell<Vec<i32>>> = Rc::default();

            // Dismiss is confirmed IFF confirmDismiss() returns true.
            let confirm_key = scaffold_key.clone();
            let confirm_dismiss = Rc::new(move |app: &mut App, _direction: DismissDirection| {
                let context = confirm_key
                    .current_context(app)
                    .expect("the scaffold is mounted");
                let popped = Show::default().show(
                    app,
                    context,
                    Rc::new(|_app, context| {
                        let pop_with = move |value: bool| {
                            Some(waft_foundation::Listener::new(move |app| {
                                // showDialog() returns `value`
                                Navigator::pop(app, context, Some(Rc::new(value)));
                            }))
                        };
                        AlertDialog::new()
                            .actions([
                                TextButton::new(pop_with(true), Some(Text::new("TRUE")))
                                    .into_widget(),
                                TextButton::new(pop_with(false), Some(Text::new("FALSE")))
                                    .into_widget(),
                            ])
                            .into_widget()
                    }),
                );
                app.spawn(async move |_cx| {
                    popped
                        .await
                        .and_then(|value| value.downcast_ref::<bool>().copied())
                })
            });

            let build_dismissible_item = {
                let dismissed_items = Rc::clone(&dismissed_items);
                move |item: i32, set_state: StateSetter| {
                    let dismissed_items = Rc::clone(&dismissed_items);
                    let confirm_dismiss = Rc::clone(&confirm_dismiss);
                    Dismissible::new(
                        waft_foundation::ValueKey::new(item).into(),
                        SizedBox::new()
                            .height(100.0)
                            .child(Text::new(item.to_string())),
                    )
                    .confirm_dismiss(move |app, direction| confirm_dismiss(app, direction))
                    .on_dismissed(move |app, _direction| {
                        let dismissed_items = Rc::clone(&dismissed_items);
                        set_state(app, &mut |_app| {
                            assert!(!dismissed_items.borrow().contains(&item));
                            dismissed_items.borrow_mut().push(item);
                        });
                    })
                    .into_widget()
                }
            };
            let build_dismissible_item = Rc::new(build_dismissible_item);

            let build_frame = |tester: &mut WidgetTester| {
                let scaffold_key = scaffold_key.clone();
                let dismissed_items = Rc::clone(&dismissed_items);
                let build_dismissible_item = Rc::clone(&build_dismissible_item);
                pump_app(tester, m3, move |_app| {
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let items: Vec<WidgetRef> = [0, 1, 2, 3, 4]
                            .into_iter()
                            .filter(|i| !dismissed_items.borrow().contains(i))
                            .map(|item| build_dismissible_item(item, Rc::clone(&set_state)))
                            .collect();
                        crate::scaffold::Scaffold::new()
                            .key(scaffold_key.clone().into())
                            .body(
                                Padding::new(EdgeInsetsGeometry::all(16.0))
                                    .child(ListView::new().item_extent(100.0).children(items)),
                            )
                            .into_widget()
                    })
                    .into_widget()
                });
            };

            let dismiss_item = |tester: &mut WidgetTester, item: i32| {
                let start = text_rect(tester, &item.to_string()).center();
                tester.fling_from(start, Offset::new(300.0, 0.0), 1000.0, 1); // fling to the right
                tester.pump(Duration::ZERO); // start the slide
                tester.pump(Duration::from_secs(1)); // finish the slide and start shrinking...
                tester.pump(Duration::ZERO); // first frame of shrinking animation
                tester.pump(Duration::from_secs(1)); // finish the shrinking and call the callback...
                tester.pump(Duration::ZERO); // rebuild after the callback removes the entry
            };
            let dismissed = || dismissed_items.borrow().clone();

            // Dismiss item 0 is confirmed via the AlertDialog
            build_frame(&mut tester);
            assert!(dismissed().is_empty());
            dismiss_item(&mut tester, 0); // Causes the AlertDialog to appear per confirmDismiss
            tester.pump_and_settle();
            tap_text(&mut tester, "TRUE"); // AlertDialog action
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "TRUE"), 0); // Dialog was dismissed
            assert_eq!(shows(&tester, "FALSE"), 0);
            assert_eq!(dismissed(), vec![0]);
            assert_eq!(shows(&tester, "0"), 0);

            // Dismiss item 1 is not confirmed via the AlertDialog
            build_frame(&mut tester);
            assert_eq!(dismissed(), vec![0]);
            dismiss_item(&mut tester, 1); // Causes the AlertDialog to appear per confirmDismiss
            tester.pump_and_settle();
            tap_text(&mut tester, "FALSE"); // AlertDialog action
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "TRUE"), 0); // Dialog was dismissed
            assert_eq!(shows(&tester, "FALSE"), 0);
            assert_eq!(dismissed(), vec![0]);
            assert_eq!(shows(&tester, "0"), 0);
            assert_eq!(shows(&tester, "1"), 1);

            // Dismiss item 1 is not confirmed via the AlertDialog
            build_frame(&mut tester);
            assert_eq!(dismissed(), vec![0]);
            dismiss_item(&mut tester, 1); // Causes the AlertDialog to appear per confirmDismiss
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "FALSE"), 1);
            assert_eq!(shows(&tester, "TRUE"), 1);
            tap_at(&mut tester, Offset::ZERO); // Tap outside of the AlertDialog
            tester.pump_and_settle();
            assert_eq!(dismissed(), vec![0]);
            assert_eq!(shows(&tester, "0"), 0);
            assert_eq!(shows(&tester, "1"), 1);
            assert_eq!(shows(&tester, "TRUE"), 0); // Dialog was dismissed
            assert_eq!(shows(&tester, "FALSE"), 0);

            // Dismiss item 1 is confirmed via the AlertDialog
            build_frame(&mut tester);
            assert_eq!(dismissed(), vec![0]);
            dismiss_item(&mut tester, 1); // Causes the AlertDialog to appear per confirmDismiss
            tester.pump_and_settle();
            tap_text(&mut tester, "TRUE"); // AlertDialog action
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "TRUE"), 0); // Dialog was dismissed
            assert_eq!(shows(&tester, "FALSE"), 0);
            assert_eq!(dismissed(), vec![0, 1]);
            assert_eq!(shows(&tester, "0"), 0);
            assert_eq!(shows(&tester, "1"), 0);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/28505.
    #[test]
    fn show_dialog_only_gets_theme_from_context_on_the_first_call() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_frame = |tester: &mut WidgetTester, builder_key: KeyRef| {
                pump_app(tester, m3, move |_app| {
                    Center::new()
                        .child(
                            Builder::new(|_app, outer_context| {
                                ElevatedButton::new(
                                    Some(waft_foundation::Listener::new(move |app| {
                                        drop(Show::default().show(
                                            app,
                                            outer_context,
                                            Rc::new(|_app, _inner_context| {
                                                AlertDialog::new()
                                                    .title(Text::new("Title"))
                                                    .into_widget()
                                            }),
                                        ));
                                    })),
                                    Some(Text::new("Show Dialog")),
                                )
                                .into_widget()
                            })
                            .key(builder_key),
                        )
                        .into_widget()
                });
            };

            build_frame(&mut tester, waft_foundation::UniqueKey::new().into());

            // Open the dialog.
            let button = only(tester.find_by_type::<ElevatedButton>());
            let center = rect_of(&tester, button).center();
            tester.tap(center, 1);
            tester.pump_and_settle();

            // Force the Builder to be recreated (new key) which causes outerContext to
            // be deactivated. If showDialog()'s implementation were to refer to
            // outerContext again, it would crash.
            build_frame(&mut tester, waft_foundation::UniqueKey::new().into());
            tester.pump(Duration::ZERO);
        });
    }

    #[test]
    fn show_dialog_safe_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                crate::app::MaterialApp::new()
                    .theme(m3(&app))
                    .builder(|_app, _context, child| {
                        MediaQuery::new(
                            // Set up the safe area to be 20 pixels in from each side
                            MediaQueryData::new().padding(EdgeInsets::all(20.0)),
                            child.expect("the navigator").clone(),
                        )
                        .into_widget()
                    })
                    .home(Center::new().child(Text::new("Test")))
                    .into_widget()
            };
            tester.pump_widget(root);
            let context = tester.find_text("Test")[0];
            let key = GlobalKey::new();
            let page = {
                let key = key.clone();
                move || -> WidgetBuilder {
                    let key = key.clone();
                    Rc::new(move |_app, _context| placeholder(&key))
                }
            };

            // By default it should honor the safe area
            drop(Show::default().on(&tester, context, page()));
            tester.pump_and_settle();
            assert_eq!(
                rect_of_key(&tester, &key).top_left(),
                Offset::new(20.0, 20.0)
            );
            assert_eq!(
                rect_of_key(&tester, &key).bottom_right(),
                Offset::new(780.0, 580.0)
            );

            // Dismiss it and test with useSafeArea off
            tap_at(&mut tester, Offset::new(10.0, 10.0));
            let show = Show {
                use_safe_area: false,
                ..Show::default()
            };
            drop(show.on(&tester, context, page()));
            tester.pump_and_settle();
            // Should take up the whole screen
            assert_eq!(rect_of_key(&tester, &key).top_left(), Offset::ZERO);
            assert_eq!(
                rect_of_key(&tester, &key).bottom_right(),
                Offset::new(800.0, 600.0)
            );
        });
    }

    /// Dart's `DialogObserver`: counts the `DialogRoute`s pushed.
    #[derive(Default)]
    struct DialogObserver {
        navigator_observer: waft_widgets::NavigatorObserverData,
        dialog_count: Cell<u32>,
    }

    impl waft_widgets::NavigatorObserver for DialogObserver {
        waft_widgets::navigator_observer_accessors!();

        fn did_push(
            self: Handle<Self>,
            app: &mut App,
            route: AnyRoute,
            _previous: Option<AnyRoute>,
        ) {
            if route.downcast::<DialogRoute>(app).is_some() {
                let count = &app.get(self).dialog_count;
                count.set(count.get() + 1);
            }
        }
    }

    /// Dart's `showDialog uses root navigator by default` and `... nested navigator if
    /// useRootNavigator is false`: the dialogs the root and the nested observer saw.
    fn dialogs_seen_by_root_and_nested(use_root_navigator: bool) -> (u32, u32) {
        let mut tester = tester();
        let (root_observer, nested_observer) = {
            let mut app = tester.cell.borrow_mut();
            (
                app.create(DialogObserver::default()),
                app.create(DialogObserver::default()),
            )
        };
        let root = {
            let app = tester.cell.borrow();
            crate::app::MaterialApp::new()
                .theme(m3(&app))
                .navigator_observers([waft_widgets::NavigatorObserver::as_observer(root_observer)])
                .home(
                    Navigator::new()
                        .observers([waft_widgets::NavigatorObserver::as_observer(
                            nested_observer,
                        )])
                        .on_generate_route(move |app, _settings| {
                            let route = crate::page::MaterialPageRoute::new(
                                app,
                                Rc::new(move |_app, context| {
                                    ElevatedButton::new(
                                        Some(waft_foundation::Listener::new(move |app| {
                                            let show = Show {
                                                use_root_navigator,
                                                ..Show::default()
                                            };
                                            drop(show.show(
                                                app,
                                                context,
                                                Rc::new(|_app, _inner_context| {
                                                    AlertDialog::new()
                                                        .title(Text::new("Title"))
                                                        .into_widget()
                                                }),
                                            ));
                                        })),
                                        Some(Text::new("Show Dialog")),
                                    )
                                    .into_widget()
                                }),
                            );
                            Some(Route::as_route(route))
                        }),
                )
                .into_widget()
        };
        tester.pump_widget(root);

        // Open the dialog.
        let button = only(tester.find_by_type::<ElevatedButton>());
        let center = rect_of(&tester, button).center();
        tester.tap(center, 1);

        let app = tester.cell.borrow();
        (
            app.get(root_observer).dialog_count.get(),
            app.get(nested_observer).dialog_count.get(),
        )
    }

    #[test]
    fn show_dialog_uses_root_navigator_by_default() {
        with_main_thread_stack(|| {
            assert_eq!(dialogs_seen_by_root_and_nested(true), (1, 0));
        });
    }

    #[test]
    fn show_dialog_uses_nested_navigator_if_use_root_navigator_is_false() {
        with_main_thread_stack(|| {
            assert_eq!(dialogs_seen_by_root_and_nested(false), (0, 1));
        });
    }

    #[test]
    fn show_dialog_throws_a_friendly_user_message_when_context_is_not_active() {
        // Regression test for https://github.com/flutter/flutter/issues/12467
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new().child(Text::new("Test")).into_widget()
            });
            let context = tester.find_text("Test")[0];

            pump_app(&mut tester, m3, |_app| Center::new().into_widget());

            let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                drop(Show::default().on(
                    &tester,
                    context,
                    Rc::new(|_app, _inner_context| {
                        AlertDialog::new().title(Text::new("Title")).into_widget()
                    }),
                ));
            }))
            .expect_err("showDialog throws");

            let message = error
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| error.downcast_ref::<&str>().map(|text| (*text).to_owned()))
                .expect("a message");
            assert_eq!(
                message.lines().next(),
                Some("This BuildContext is no longer valid.")
            );
        });
    }

    /// Dart's collector frees the `CurvedAnimation` a `DialogRoute` makes for its fade once
    /// the route is gone; here the route frees it in its `dispose`.
    #[test]
    fn a_disposed_dialog_route_frees_its_fade() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved_context = context_cell();
            let saved = Rc::clone(&saved_context);
            pump_app(&mut tester, m3, move |_app| {
                Material::new()
                    .child(Builder::new(move |_app, context| {
                        saved.set(Some(context));
                        SizedBox::new().into_widget()
                    }))
                    .into_widget()
            });
            let navigator =
                Navigator::of(&mut tester.cell.borrow_mut(), seen(&saved_context), false);
            let route = {
                let mut app = tester.cell.borrow_mut();
                let route = DialogRoute::new(
                    &mut app,
                    seen(&saved_context),
                    Rc::new(move |_app, _context| Text::new("Dialog").into_widget()),
                );
                drop(navigator.push(&mut app, route.as_route()));
                route
            };
            tester.pump_and_settle();
            let fade = tester
                .cell
                .borrow()
                .get(route)
                .curved_animation
                .expect("the route built its transitions");

            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            tester.cell.checkpoint();
            assert!(!tester.cell.borrow().contains(fade));
        });
    }
}
