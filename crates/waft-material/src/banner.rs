//! Flutter counterpart: material_ui's `banner.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationStatus,
    AnimationStatusListener, AnyAnimation, Curve, CurvedAnimation, Curves, Threshold, Tween,
};
use waft_embedder::{Color, Offset};
use waft_foundation::{App, Handle, VoidCallback};
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, TextStyle};
use waft_rendering::BoxConstraints;
use waft_scheduler::TickerProvider;
use waft_widgets::{
    Align, AnimatedBuilder, BuildContext, ClipRect, Column, ConstrainedBox, DefaultTextStyle,
    Expanded, Hero, IntoWidget, KeyRef, MainAxisSize, MediaQuery, OverflowBar,
    OverflowBarAlignment, Padding, Row, SafeArea, Semantics, SlideTransition, State, StateData,
    StatefulWidget, WidgetRef, debug_check_has_media_query,
};

use crate::banner_theme::{
    MaterialBannerTheme, MaterialBannerThemeData, MaterialBannerThemeDataGetters,
};
use crate::divider::Divider;
use crate::generated::banner_defaults_m3::BannerDefaultsM3;
use crate::material::Material;
use crate::scaffold::ScaffoldMessenger;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

const MATERIAL_BANNER_TRANSITION_DURATION: Duration = Duration::from_millis(250);

fn material_banner_height_curve() -> Rc<dyn Curve> {
    Curves::fast_out_slow_in()
}

const K_MAX_CONTENT_TEXT_SCALE_FACTOR: f64 = 1.5;

/// Specify how a [`MaterialBanner`] was closed.
///
/// The `ScaffoldMessengerState::show_material_banner` function returns a
/// `ScaffoldFeatureController`. The value of the controller's closed property
/// is a future that resolves to a [`MaterialBannerClosedReason`]. Applications that need
/// to know how a [`MaterialBanner`] was closed can use this value.
///
/// Example:
///
/// ```text
/// ScaffoldMessenger::of(app, context)
///     .show_material_banner(app, MaterialBanner::new(Text::new("Message..."), vec![/* ... */]))
///     .closed()
///     .then(app, |app, reason: MaterialBannerClosedReason| {
///         // ...
///     });
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialBannerClosedReason {
    /// The material banner was closed through a `SemanticsAction.dismiss`.
    Dismiss,

    /// The material banner was closed by a user's swipe.
    Swipe,

    /// The material banner was closed by the `ScaffoldFeatureController` close callback
    /// or by calling `ScaffoldMessengerState::hide_current_material_banner` directly.
    Hide,

    /// The material banner was closed by a call to
    /// `ScaffoldMessengerState::remove_current_material_banner`.
    Remove,
}

/// A Material Design banner.
///
/// A banner displays an important, succinct message, and provides actions for
/// users to address (or dismiss the banner). A user action is required for it
/// to be dismissed.
///
/// Banners should be displayed at the top of the screen, below a top app bar.
/// They are persistent and non-modal, allowing the user to either ignore them or
/// interact with them at any time.
///
/// Banners placed directly into the widget tree are static.
///
/// MaterialBanner's can also be presented through a `ScaffoldMessenger`, with
/// `ScaffoldMessengerState::show_material_banner`.
///
/// The [`actions`](Self::actions) will be placed beside the [`content`](Self::content) if
/// there is only one. Otherwise, the [`actions`](Self::actions) will be placed below the
/// [`content`](Self::content). Use [`force_actions_below`](Self::force_actions_below) to
/// override this behavior.
///
/// If the [`actions`](Self::actions) placed below the [`content`](Self::content), they will
/// be laid out in a row. If there isn't sufficient room to display everything, they are laid
/// out in a column instead.
///
/// The [`actions`](Self::actions) and [`content`](Self::content) must be provided. An
/// optional leading widget (typically an `Image`) can also be provided. The
/// [`content_text_style`](Self::content_text_style) and
/// [`background_color`](Self::background_color) can be provided to customize the banner.
///
/// This widget is unrelated to the widgets library `Banner` widget.
#[derive(Clone)]
pub struct MaterialBanner {
    pub key: Option<KeyRef>,

    /// The content of the [`MaterialBanner`].
    ///
    /// Typically a `Text` widget.
    pub content: WidgetRef,

    /// Style for the text in the [`content`](Self::content) of the [`MaterialBanner`].
    ///
    /// If `None`, [`MaterialBannerThemeData::content_text_style`] is used. If that is
    /// also `None`, `TextTheme::body_medium` of `ThemeData::text_theme` is used.
    pub content_text_style: Option<TextStyle>,

    /// The set of actions that are displayed at the bottom or trailing side of
    /// the [`MaterialBanner`].
    ///
    /// Typically this is a list of `TextButton` widgets.
    ///
    /// When a [`MaterialBanner`] is shown by a `ScaffoldMessenger` above the
    /// `Navigator`, such as the one created by `MaterialApp`, it stays visible
    /// after the route that showed it is popped, and that route's `BuildContext`
    /// is deactivated. If an action callback dismisses the banner using
    /// `ScaffoldMessenger::of`, avoid passing a `BuildContext` from the route that
    /// showed the banner. Instead, either wrap the action widget in a `Builder` so
    /// its callback uses the banner's own `BuildContext`, or capture the
    /// `ScaffoldMessengerState` before calling
    /// `ScaffoldMessengerState::show_material_banner`.
    pub actions: Vec<WidgetRef>,

    /// The z-coordinate at which to place the material banner.
    ///
    /// This controls the size of the shadow below the material banner.
    ///
    /// Defines the banner's `Material::elevation`.
    ///
    /// If this property is `None`, then the ambient [`MaterialBannerThemeData::elevation`]
    /// is used, if that is also `None`, the default value is 0.
    /// If the elevation is 0, the `Scaffold`'s body will be pushed down by the
    /// MaterialBanner when used with `ScaffoldMessenger`.
    pub elevation: Option<f64>,

    /// The (optional) leading widget of the [`MaterialBanner`].
    ///
    /// Typically an `Icon` widget.
    pub leading: Option<WidgetRef>,

    /// The optional minimum action bar height.
    ///
    /// Default to 52.0.
    pub min_action_bar_height: f64,

    /// The color of the surface of this [`MaterialBanner`].
    ///
    /// If `None`, [`MaterialBannerThemeData::background_color`] is used. If that is
    /// also `None`, `ColorScheme::surface_container_low` of `ThemeData::color_scheme` is
    /// used.
    pub background_color: Option<Color>,

    /// The color used as an overlay on [`background_color`](Self::background_color) to
    /// indicate elevation.
    ///
    /// If `None`, [`MaterialBannerThemeData::surface_tint_color`] is used. If that
    /// is also `None`, the default value is `Colors::TRANSPARENT`.
    ///
    /// This is not recommended for use. [Material 3 spec](https://m3.material.io/styles/color/the-color-system/color-roles)
    /// introduced a set of tone-based surfaces and surface containers in its `ColorScheme`,
    /// which provide more flexibility. The intention is to eventually remove surface tint
    /// color from the framework.
    pub surface_tint_color: Option<Color>,

    /// The color of the shadow below the [`MaterialBanner`].
    ///
    /// If this property is `None`, then the ambient [`MaterialBannerThemeData::shadow_color`]
    /// is used. If that is also `None`, the default value is `None`.
    pub shadow_color: Option<Color>,

    /// The color of the divider.
    ///
    /// If this property is `None`, then the ambient [`MaterialBannerThemeData::divider_color`]
    /// is used. If that is also `None`, the default value is `ColorScheme::surface_variant`.
    pub divider_color: Option<Color>,

    /// The amount of space by which to inset the [`content`](Self::content).
    ///
    /// If the [`actions`](Self::actions) are below the [`content`](Self::content), this
    /// defaults to `EdgeInsetsDirectional.only(start: 16.0, top: 24.0, end: 16.0, bottom: 4.0)`.
    ///
    /// If the [`actions`](Self::actions) are trailing the [`content`](Self::content), this
    /// defaults to `EdgeInsetsDirectional.only(start: 16.0, top: 2.0)`.
    pub padding: Option<EdgeInsetsGeometry>,

    /// Empty space to surround the [`MaterialBanner`].
    ///
    /// If the [`margin`](Self::margin) is `None` then this defaults to
    /// 0 if the banner's [`elevation`](Self::elevation) is 0, 10 otherwise.
    pub margin: Option<EdgeInsetsGeometry>,

    /// The amount of space by which to inset the [`leading`](Self::leading) widget.
    ///
    /// This defaults to `EdgeInsetsDirectional.only(end: 16.0)`.
    pub leading_padding: Option<EdgeInsetsGeometry>,

    /// An override to force the [`actions`](Self::actions) to be below the
    /// [`content`](Self::content) regardless of how many there are.
    ///
    /// If this is true, the [`actions`](Self::actions) will be placed below the
    /// [`content`](Self::content). If this is false, the [`actions`](Self::actions) will be
    /// placed on the trailing side of the [`content`](Self::content) if
    /// [`actions`](Self::actions)'s length is 1 and below the [`content`](Self::content) if
    /// greater than 1.
    ///
    /// Defaults to false.
    pub force_actions_below: bool,

    /// The horizontal alignment of the [`actions`](Self::actions) when the
    /// [`actions`](Self::actions) laid out in a column.
    ///
    /// Defaults to [`OverflowBarAlignment::End`].
    pub overflow_alignment: OverflowBarAlignment,

    /// The animation driving the entrance and exit of the material banner when presented by
    /// the `ScaffoldMessenger`.
    pub animation: Option<AnyAnimation<f64>>,

    /// Called the first time that the material banner is visible within a `Scaffold` when
    /// presented by the `ScaffoldMessenger`.
    pub on_visible: Option<VoidCallback>,
}

impl MaterialBanner {
    /// Creates a [`MaterialBanner`]; Dart's optional arguments are the setters.
    ///
    /// The length of the [`actions`](Self::actions) list must not be empty. The
    /// [`elevation`](Self::elevation) must be `None` or non-negative.
    pub fn new<K>(content: impl IntoWidget<K>, actions: Vec<WidgetRef>) -> MaterialBanner {
        MaterialBanner {
            key: None,
            content: content.into_widget(),
            content_text_style: None,
            actions,
            elevation: None,
            leading: None,
            min_action_bar_height: 52.0,
            background_color: None,
            surface_tint_color: None,
            shadow_color: None,
            divider_color: None,
            padding: None,
            margin: None,
            leading_padding: None,
            force_actions_below: false,
            overflow_alignment: OverflowBarAlignment::End,
            animation: None,
            on_visible: None,
        }
    }

    /// Dart `MaterialBanner(key:)`.
    pub fn key(mut self, key: KeyRef) -> MaterialBanner {
        self.key = Some(key);
        self
    }

    /// Dart `MaterialBanner(contentTextStyle:)`.
    pub fn content_text_style(mut self, content_text_style: TextStyle) -> MaterialBanner {
        self.content_text_style = Some(content_text_style);
        self
    }

    /// Dart `MaterialBanner(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> MaterialBanner {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `MaterialBanner(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> MaterialBanner {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `MaterialBanner(minActionBarHeight:)`.
    pub fn min_action_bar_height(mut self, min_action_bar_height: f64) -> MaterialBanner {
        self.min_action_bar_height = min_action_bar_height;
        self
    }

    /// Dart `MaterialBanner(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> MaterialBanner {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `MaterialBanner(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> MaterialBanner {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `MaterialBanner(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> MaterialBanner {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `MaterialBanner(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> MaterialBanner {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `MaterialBanner(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> MaterialBanner {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `MaterialBanner(margin:)`.
    pub fn margin(mut self, margin: impl Into<EdgeInsetsGeometry>) -> MaterialBanner {
        self.margin = Some(margin.into());
        self
    }

    /// Dart `MaterialBanner(leadingPadding:)`.
    pub fn leading_padding(
        mut self,
        leading_padding: impl Into<EdgeInsetsGeometry>,
    ) -> MaterialBanner {
        self.leading_padding = Some(leading_padding.into());
        self
    }

    /// Dart `MaterialBanner(forceActionsBelow:)`.
    pub fn force_actions_below(mut self, force_actions_below: bool) -> MaterialBanner {
        self.force_actions_below = force_actions_below;
        self
    }

    /// Dart `MaterialBanner(overflowAlignment:)`.
    pub fn overflow_alignment(
        mut self,
        overflow_alignment: OverflowBarAlignment,
    ) -> MaterialBanner {
        self.overflow_alignment = overflow_alignment;
        self
    }

    /// Dart `MaterialBanner(animation:)`.
    pub fn animation(mut self, animation: AnyAnimation<f64>) -> MaterialBanner {
        self.animation = Some(animation);
        self
    }

    /// Dart `MaterialBanner(onVisible:)`.
    pub fn on_visible(mut self, on_visible: impl Fn(&mut App) + 'static) -> MaterialBanner {
        self.on_visible = Some(Rc::new(on_visible));
        self
    }

    // API for ScaffoldMessengerState.showMaterialBanner():

    /// Creates an animation controller useful for driving a [`MaterialBanner`]'s entrance and
    /// exit animation.
    pub fn create_animation_controller(
        app: &mut App,
        vsync: impl TickerProvider,
    ) -> Handle<AnimationController> {
        AnimationController::create(
            app,
            None,
            Some(MATERIAL_BANNER_TRANSITION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        )
    }

    /// Creates a copy of this material banner but with the animation replaced with the given
    /// animation.
    ///
    /// If the original material banner lacks a key, the newly created material banner will
    /// use the given fallback key.
    pub fn with_animation(
        &self,
        new_animation: AnyAnimation<f64>,
        fallback_key: Option<KeyRef>,
    ) -> MaterialBanner {
        MaterialBanner {
            key: self.key.clone().or(fallback_key),
            content: self.content.clone(),
            content_text_style: self.content_text_style.clone(),
            actions: self.actions.clone(),
            elevation: self.elevation,
            leading: self.leading.clone(),
            min_action_bar_height: self.min_action_bar_height,
            background_color: self.background_color,
            surface_tint_color: self.surface_tint_color,
            shadow_color: self.shadow_color,
            divider_color: self.divider_color,
            padding: self.padding,
            margin: self.margin,
            leading_padding: self.leading_padding,
            force_actions_below: self.force_actions_below,
            overflow_alignment: self.overflow_alignment,
            animation: Some(new_animation),
            on_visible: self.on_visible.clone(),
        }
    }
}

impl fmt::Debug for MaterialBanner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialBanner")
            .field("key", &self.key)
            .field("content", &self.content)
            .field("actions", &self.actions)
            .field("elevation", &self.elevation)
            .field("leading", &self.leading)
            .field("force_actions_below", &self.force_actions_below)
            .field("overflow_alignment", &self.overflow_alignment)
            .field("animation", &self.animation)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MaterialBanner {
    type State = MaterialBannerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MaterialBannerState {
        MaterialBannerState {
            state: StateData::new(),
            was_visible: false,
            height_animation: None,
            slide_out_curved_animation: None,
            slide_out_animation: None,
        }
    }
}

/// Dart's `_MaterialBannerState`.
pub struct MaterialBannerState {
    state: StateData<MaterialBanner>,
    was_visible: bool,
    height_animation: Option<Handle<CurvedAnimation>>,
    slide_out_curved_animation: Option<Handle<CurvedAnimation>>,
    /// The tween and the animation the last build slid the banner with, which Dart's collector
    /// frees once a build replaces them; the next build or `dispose` frees them.
    slide_out_animation: Option<(Handle<Tween<Offset>>, AnyAnimation<Offset>)>,
}

impl MaterialBannerState {
    fn status_listener(self: Handle<Self>) -> AnimationStatusListener {
        AnimationStatusListener::handle_method(
            self,
            MaterialBannerState::on_animation_status_changed,
        )
    }

    /// Dart's `curvedAnimation?.dispose()`, then frees it.
    fn dispose_curved_animation(app: &mut App, animation: Option<Handle<CurvedAnimation>>) {
        if let Some(animation) = animation {
            animation.dispose(app);
            app.destroy_lingering(animation);
        }
    }

    /// Frees the last build's slide tween and animation; the `SlideTransition` that listens to
    /// the animation lets go when it rebuilds, before the entries go at the next checkpoint.
    fn free_slide_out_animation(self: Handle<Self>, app: &mut App) {
        if let Some((tween, animation)) = app.get_mut(self).slide_out_animation.take() {
            app.destroy_lingering(animation.id());
            app.destroy_lingering(tween);
        }
    }

    fn set_curved_animations(self: Handle<Self>, app: &mut App) {
        MaterialBannerState::dispose_curved_animation(app, app.get(self).height_animation);
        MaterialBannerState::dispose_curved_animation(
            app,
            app.get(self).slide_out_curved_animation,
        );
        match self.widget(app).animation {
            Some(animation) => {
                let height_animation =
                    CurvedAnimation::create(app, animation, material_banner_height_curve(), None);
                let slide_out_curved_animation =
                    CurvedAnimation::create(app, animation, Rc::new(Threshold::new(0.0)), None);
                let this = app.get_mut(self);
                this.height_animation = Some(height_animation);
                this.slide_out_curved_animation = Some(slide_out_curved_animation);
            }
            None => {
                let this = app.get_mut(self);
                this.height_animation = None;
                this.slide_out_curved_animation = None;
            }
        }
    }

    fn on_animation_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status.is_completed() {
            let on_visible = self.widget(app).on_visible.clone();
            if let Some(on_visible) = on_visible
                && !app.get(self).was_visible
            {
                on_visible(app);
            }
            app.get_mut(self).was_visible = true;
        }
    }
}

impl State for MaterialBannerState {
    type Widget = MaterialBanner;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if let Some(animation) = self.widget(app).animation {
            animation.add_status_listener(app, self.status_listener());
        }
        self.set_curved_animations(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MaterialBanner) {
        let animation = self.widget(app).animation;
        if animation != old_widget.animation {
            if let Some(old_animation) = old_widget.animation {
                old_animation.remove_status_listener(app, &self.status_listener());
            }
            if let Some(animation) = animation {
                animation.add_status_listener(app, self.status_listener());
            }
            self.set_curved_animations(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(animation) = self.widget(app).animation {
            animation.remove_status_listener(app, &self.status_listener());
        }
        MaterialBannerState::dispose_curved_animation(app, app.get(self).height_animation);
        MaterialBannerState::dispose_curved_animation(
            app,
            app.get(self).slide_out_curved_animation,
        );
        self.free_slide_out_animation(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_media_query(app, context));
        let accessible_navigation = MediaQuery::accessible_navigation_of(app, context);

        debug_assert!(!self.widget(app).actions.is_empty());

        let theme = Theme::of(app, context);
        let banner_theme = MaterialBannerTheme::of(app, context);
        let defaults: Box<dyn MaterialBannerThemeDataGetters> = if theme.use_material3 {
            Box::new(BannerDefaultsM3::new(context))
        } else {
            Box::new(BannerDefaultsM2::new(app, context))
        };

        let widget = self.widget(app).clone();
        let is_single_row = widget.actions.len() == 1 && !widget.force_actions_below;
        let padding = widget
            .padding
            .or(banner_theme.padding)
            .unwrap_or(if is_single_row {
                EdgeInsetsGeometry::directional(16.0, 2.0, 0.0, 0.0)
            } else {
                EdgeInsetsGeometry::directional(16.0, 24.0, 16.0, 4.0)
            });
        let leading_padding = widget
            .leading_padding
            .or(banner_theme.leading_padding)
            .unwrap_or(EdgeInsetsGeometry::directional(0.0, 0.0, 16.0, 0.0));

        let actions_bar =
            ConstrainedBox::new(BoxConstraints::new().min_height(widget.min_action_bar_height))
                .child(
                    Padding::new(EdgeInsetsGeometry::symmetric(0.0, 8.0)).child(
                        Align::new().alignment(AlignmentGeometry::CENTER_END).child(
                            OverflowBar::new()
                                .overflow_alignment(widget.overflow_alignment)
                                .spacing(8.0)
                                .children(widget.actions.clone()),
                        ),
                    ),
                )
                .into_widget();

        let elevation = widget.elevation.or(banner_theme.elevation).unwrap_or(0.0);
        let margin = widget.margin.unwrap_or(EdgeInsetsGeometry::only(
            0.0,
            0.0,
            0.0,
            if elevation > 0.0 { 10.0 } else { 0.0 },
        ));
        let background_color = widget
            .background_color
            .or(banner_theme.background_color)
            .or_else(|| defaults.background_color(app))
            .expect("the banner defaults have a background color");
        let surface_tint_color = widget
            .surface_tint_color
            .or(banner_theme.surface_tint_color)
            .or_else(|| defaults.surface_tint_color(app));
        let shadow_color = widget.shadow_color.or(banner_theme.shadow_color);
        let divider_color = widget
            .divider_color
            .or(banner_theme.divider_color)
            .or_else(|| defaults.divider_color(app));
        let text_style = widget
            .content_text_style
            .clone()
            .or_else(|| banner_theme.content_text_style.clone())
            .or_else(|| defaults.content_text_style(app));

        let mut row_children: Vec<WidgetRef> = Vec::new();
        if let Some(leading) = &widget.leading {
            row_children.push(
                Padding::new(leading_padding)
                    .child(leading.clone())
                    .into_widget(),
            );
        }
        row_children.push(MediaQuery::with_clamped_text_scaling(
            None,
            // Set maximum text scale factor to _kMaxContentTextScaleFactor for the
            // content to keep the visual hierarchy the same even with larger font
            // sizes.
            0.0,
            K_MAX_CONTENT_TEXT_SCALE_FACTOR,
            Expanded::new(DefaultTextStyle::new(
                text_style.expect("the banner defaults have a content text style"),
                widget.content.clone(),
            )),
        ));
        if is_single_row {
            row_children.push(MediaQuery::with_clamped_text_scaling(
                None,
                // Set maximum text scale factor to _kMaxContentTextScaleFactor for the
                // actionsBar to keep the visual hierarchy the same even with larger font
                // sizes.
                0.0,
                K_MAX_CONTENT_TEXT_SCALE_FACTOR,
                actions_bar.clone(),
            ));
        }

        let mut column_children: Vec<WidgetRef> = vec![
            Padding::new(padding)
                .child(Row::new().children(row_children))
                .into_widget(),
        ];
        if !is_single_row {
            column_children.push(actions_bar);
        }
        if elevation == 0.0 {
            let mut divider = Divider::new().height(0.0);
            if let Some(divider_color) = divider_color {
                divider = divider.color(divider_color);
            }
            column_children.push(divider.into_widget());
        }

        let mut material = Material::new()
            .elevation(elevation)
            .color(background_color)
            .child(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .children(column_children),
            );
        material.surface_tint_color = surface_tint_color;
        material.shadow_color = shadow_color;
        let mut material_banner = Padding::new(margin).child(material).into_widget();

        // This provides a static banner for backwards compatibility.
        if widget.animation.is_none() {
            return material_banner;
        }

        material_banner = SafeArea::new(material_banner).into_widget();

        let slide_out_curved_animation = app
            .get(self)
            .slide_out_curved_animation
            .expect("set with the animation");
        let slide_out_tween = Tween::new(app, Some(Offset::new(0.0, -1.0)), Some(Offset::ZERO));
        let slide_out_animation =
            slide_out_tween.animate(app, slide_out_curved_animation.as_animation());
        self.free_slide_out_animation(app);
        app.get_mut(self).slide_out_animation = Some((slide_out_tween, slide_out_animation));

        material_banner = Semantics::new()
            .container(true)
            .live_region(true)
            .on_dismiss(Rc::new(move |app: &mut App| {
                ScaffoldMessenger::of(app, context)
                    .remove_current_material_banner(app, MaterialBannerClosedReason::Dismiss);
            }))
            .child(if accessible_navigation {
                material_banner
            } else {
                SlideTransition::new(slide_out_animation)
                    .child(material_banner)
                    .into_widget()
            })
            .into_widget();

        let material_banner_transition = if accessible_navigation {
            material_banner
        } else {
            let height_animation = app
                .get(self)
                .height_animation
                .expect("set with the animation")
                .as_animation();
            AnimatedBuilder::new(Rc::new(height_animation), move |app, _context, child| {
                Align::new()
                    .alignment(AlignmentGeometry::BOTTOM_START)
                    .height_factor(height_animation.value(app))
                    .child(child.cloned().expect("the builder's child"))
                    .into_widget()
            })
            .child(material_banner)
            .into_widget()
        };

        Hero::new(
            Rc::new(format!("<MaterialBanner Hero tag - {:?}>", widget.content)),
            ClipRect::new().child(material_banner_transition),
        )
        .into_widget()
    }
}

/// Dart's `_BannerDefaultsM2`.
struct BannerDefaultsM2 {
    /// Dart's `super(..)` arguments.
    material_banner_theme_data: MaterialBannerThemeData,
    /// Dart's `_theme`, which the constructor reads.
    theme: Rc<ThemeData>,
}

impl BannerDefaultsM2 {
    fn new(app: &mut App, context: BuildContext) -> BannerDefaultsM2 {
        BannerDefaultsM2 {
            material_banner_theme_data: MaterialBannerThemeData::new().elevation(0.0),
            theme: Theme::of(app, context),
        }
    }
}

impl MaterialBannerThemeDataGetters for BannerDefaultsM2 {
    fn material_banner_theme_data(&self) -> &MaterialBannerThemeData {
        &self.material_banner_theme_data
    }

    fn background_color(&self, _app: &mut App) -> Option<Color> {
        Some(self.theme.color_scheme.surface)
    }

    fn content_text_style(&self, _app: &mut App) -> Option<TextStyle> {
        self.theme.text_theme.body_medium.clone()
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/banner_test.dart` and the widget cases of `test/banner_theme_test.dart`.

    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use waft_embedder::{Rect, Size, TextDirection};
    use waft_foundation::Listener;
    use waft_painting::{EdgeInsets, TextScaler};
    use waft_rendering::HitTestBehavior;
    use waft_widgets::{
        AnyElement, Builder, Center, Directionality, GestureDetector, GlobalKey, MediaQueryData,
        SizedBox, Text,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::scaffold::{Scaffold, ScaffoldMessenger};
    use crate::snack_bar::SnackBar;
    use crate::test_support::{AhemText, WidgetTester, with_main_thread_stack};
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app, tester};
    use crate::typography::Typography;

    /// A banner's content: Dart's `Text(text)` laid out in Ahem, with the `DefaultTextStyle`
    /// it lays out in recorded, which is the style Dart reads from its `RenderParagraph`.
    fn content(text: &str) -> (WidgetRef, Rc<RefCell<Option<TextStyle>>>) {
        let style = Rc::new(RefCell::new(None));
        let text = text.to_owned();
        let seen = Rc::clone(&style);
        let widget = Builder::new(move |app, context| {
            *seen.borrow_mut() = Some(DefaultTextStyle::of(app, context).style);
            AhemText::new(&text).into_widget()
        })
        .into_widget();
        (widget, style)
    }

    /// Dart's `TextButton(child: Text(label), onPressed: () {})`, its label in Ahem.
    fn action(label: &str) -> WidgetRef {
        TextButton::new(Some(Listener::new(|_app| {})), Some(AhemText::new(label))).into_widget()
    }

    /// Dart's `TextButton(child: Text('DISMISS'), onPressed: () =>
    /// ScaffoldMessenger.of(context).hideCurrentMaterialBanner())`.
    fn dismiss(context: BuildContext) -> TextButton {
        TextButton::new(
            Some(Listener::new(move |app| {
                ScaffoldMessenger::of(app, context)
                    .hide_current_material_banner(app, MaterialBannerClosedReason::Hide);
            })),
            Some(AhemText::new("DISMISS")),
        )
    }

    /// Dart's `Scaffold(body: Builder(builder: (context) => GestureDetector(key: tapTarget,
    /// onTap: () => ScaffoldMessenger.of(context).showMaterialBanner(banner(app, context)),
    /// behavior: HitTestBehavior.opaque, child: SizedBox(height: 100.0, width: 100.0))))`;
    /// without `opaque`, the detector has neither the behavior nor the child.
    fn messenger_home(
        tap_target: &GlobalKey,
        opaque: bool,
        banner: impl Fn(&mut App, BuildContext) -> MaterialBanner + 'static,
    ) -> WidgetRef {
        let banner = Rc::new(banner);
        let tap_target = tap_target.clone();
        Scaffold::new()
            .body(Builder::new(move |_app, context| {
                let banner = Rc::clone(&banner);
                let detector = GestureDetector::new()
                    .key(tap_target.clone().into())
                    .on_tap(Listener::new(move |app| {
                        let banner = banner(app, context);
                        ScaffoldMessenger::of(app, context).show_material_banner(app, banner);
                    }));
                if opaque {
                    detector
                        .behavior(HitTestBehavior::Opaque)
                        .child(SizedBox::new().height(100.0).width(100.0))
                        .into_widget()
                } else {
                    detector.into_widget()
                }
            }))
            .into_widget()
    }

    /// Dart's `tester.tap(find.byKey(key))`.
    fn tap_key(tester: &mut WidgetTester, key: &GlobalKey) {
        let center = tester.center_of(key);
        tester.tap(center, 1);
    }

    /// Dart's `tester.tap(find.text(text))`.
    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let element = *tester
            .find_ahem_text(text)
            .first()
            .expect("a Text showing the text");
        let center = rect(tester, element).center();
        tester.tap(center, 1);
    }

    fn rect(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.global_rect(tester.render_box_of(element))
    }

    fn key_rect(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        let render = tester.render_object(key).as_box().expect("a render box");
        tester.global_rect(render)
    }

    fn banner(tester: &WidgetTester) -> AnyElement {
        *tester
            .find_by_type::<MaterialBanner>()
            .first()
            .expect("a MaterialBanner")
    }

    /// Dart's `find.descendant(of: find.byType(MaterialBanner), matching: find.byType(W))`.
    fn in_banner<W: 'static>(tester: &WidgetTester) -> Vec<AnyElement> {
        let banner = banner(tester);
        tester
            .find_by_type::<W>()
            .into_iter()
            .filter(|element| tester.ancestors(*element).contains(&banner))
            .collect()
    }

    /// Dart's `_getMaterialFromBanner` and `_materialFinder`.
    fn banner_material(tester: &WidgetTester) -> AnyElement {
        in_banner::<Material>(tester)[0]
    }

    /// Dart's `_getMaterialFromText`: the nearest `Material` around the content.
    fn material_around(tester: &WidgetTester, text: &str) -> AnyElement {
        let content = *tester.find_ahem_text(text).first().expect("the content");
        tester.ancestor_of_type::<Material>(content)
    }

    /// A material's `(elevation, color, surfaceTintColor, shadowColor)`.
    fn material_values(
        tester: &WidgetTester,
        material: AnyElement,
    ) -> (f64, Option<Color>, Option<Color>, Option<Color>) {
        tester.widget(material, |material: &Material| {
            (
                material.elevation,
                material.color,
                material.surface_tint_color,
                material.shadow_color,
            )
        })
    }

    fn divider_color(tester: &WidgetTester) -> Option<Color> {
        let divider = *tester.find_by_type::<Divider>().first().expect("a Divider");
        tester.widget(divider, |divider: &Divider| divider.color)
    }

    fn content_rect(tester: &WidgetTester, text: &str) -> Rect {
        let content = *tester.find_ahem_text(text).first().expect("the content");
        rect(tester, content)
    }

    fn overflow_bar_rect(tester: &WidgetTester) -> Rect {
        let bar = *tester
            .find_by_type::<OverflowBar>()
            .first()
            .expect("an OverflowBar");
        rect(tester, bar)
    }

    fn with_tester(body: impl FnOnce(&mut WidgetTester) + Send) {
        with_main_thread_stack(|| body(&mut tester()));
    }

    #[test]
    fn a_banner_respects_its_properties() {
        // Dart: 'MaterialBanner properties are respected'.
        with_tester(|tester| {
            let (content_widget, style) = content("Content");
            let content_text_style = TextStyle::new().color(Colors::PINK.primary());
            let banner_text_style = content_text_style.clone();
            pump_app(tester, m3, move |_app| {
                MaterialBanner::new(content_widget, vec![action("Action")])
                    .background_color(Colors::PINK.primary())
                    .surface_tint_color(Colors::GREEN.primary())
                    .shadow_color(Colors::BLUE.primary())
                    .divider_color(Colors::YELLOW.primary())
                    .content_text_style(banner_text_style)
                    .into_widget()
            });

            assert_eq!(
                material_values(tester, banner_material(tester)),
                (
                    0.0,
                    Some(Colors::PINK.primary()),
                    Some(Colors::GREEN.primary()),
                    Some(Colors::BLUE.primary())
                )
            );
            assert_eq!(*style.borrow(), Some(content_text_style));
            assert_eq!(divider_color(tester), Some(Colors::YELLOW.primary()));
        });
    }

    #[test]
    fn a_banner_shown_by_the_messenger_respects_its_properties() {
        // Dart: 'MaterialBanner properties are respected when presented by ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            let (content_widget, style) = content("Content");
            let content_text_style = TextStyle::new().color(Colors::PINK.primary());
            let banner_text_style = content_text_style.clone();
            pump_app(tester, m3, |_app| {
                messenger_home(&tap_target, true, move |_app, context| {
                    MaterialBanner::new(
                        content_widget.clone(),
                        vec![dismiss(context).into_widget()],
                    )
                    .background_color(Colors::PINK.primary())
                    .surface_tint_color(Colors::GREEN.primary())
                    .shadow_color(Colors::BLUE.primary())
                    .divider_color(Colors::YELLOW.primary())
                    .content_text_style(banner_text_style.clone())
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            assert_eq!(
                material_values(tester, material_around(tester, "Content")),
                (
                    0.0,
                    Some(Colors::PINK.primary()),
                    Some(Colors::GREEN.primary()),
                    Some(Colors::BLUE.primary())
                )
            );
            assert_eq!(*style.borrow(), Some(content_text_style));
            assert_eq!(divider_color(tester), Some(Colors::YELLOW.primary()));
        });
    }

    #[test]
    fn more_than_one_action_is_laid_out_below_the_content() {
        // Dart: 'Actions laid out below content if more than one action'.
        with_tester(|tester| {
            pump_app(tester, m3, |_app| {
                MaterialBanner::new(
                    AhemText::new("Content"),
                    vec![action("Action 1"), action("Action 2")],
                )
                .into_widget()
            });

            let content_bottom_left = content_rect(tester, "Content").bottom_left();
            let actions_top_left = overflow_bar_rect(tester).top_left();
            assert!(content_bottom_left.dy() < actions_top_left.dy());
            assert!(content_bottom_left.dx() < actions_top_left.dx());
        });
    }

    #[test]
    fn more_than_one_action_of_a_shown_banner_is_laid_out_below_the_content() {
        // Dart: 'Actions laid out below content if more than one action when presented by
        // ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                messenger_home(&tap_target, true, |_app, context| {
                    let ok = TextButton::new(
                        Some(Listener::new(move |app| {
                            ScaffoldMessenger::of(app, context).hide_current_material_banner(
                                app,
                                MaterialBannerClosedReason::Hide,
                            );
                        })),
                        Some(AhemText::new("OK")),
                    );
                    MaterialBanner::new(
                        AhemText::new("Content"),
                        vec![ok.into_widget(), dismiss(context).into_widget()],
                    )
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let content_bottom_left = content_rect(tester, "Content").bottom_left();
            let actions_top_left = overflow_bar_rect(tester).top_left();
            assert!(content_bottom_left.dy() < actions_top_left.dy());
            assert!(content_bottom_left.dx() < actions_top_left.dx());
        });
    }

    #[test]
    fn a_single_action_is_laid_out_beside_the_content() {
        // Dart: 'Actions laid out beside content if only one action'.
        with_tester(|tester| {
            pump_app(tester, m3, |_app| {
                MaterialBanner::new(AhemText::new("Content"), vec![action("Action")]).into_widget()
            });

            let content_bottom_left = content_rect(tester, "Content").bottom_left();
            let actions_top_right = overflow_bar_rect(tester).top_right();
            assert!(content_bottom_left.dy() > actions_top_right.dy());
            assert!(content_bottom_left.dx() < actions_top_right.dx());
        });
    }

    #[test]
    fn a_single_action_of_a_shown_banner_is_laid_out_beside_the_content() {
        // Dart: 'Actions laid out beside content if only one action when presented by
        // ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                messenger_home(&tap_target, true, |_app, context| {
                    MaterialBanner::new(
                        AhemText::new("Content"),
                        vec![dismiss(context).into_widget()],
                    )
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let content_bottom_left = content_rect(tester, "Content").bottom_left();
            let actions_top_right = overflow_bar_rect(tester).top_right();
            assert!(content_bottom_left.dy() > actions_top_right.dy());
            assert!(content_bottom_left.dx() < actions_top_right.dx());
        });
    }

    #[test]
    fn the_content_scales_with_the_text_up_to_the_maximum_scale_factor() {
        // Dart: 'material banner content can scale and has maxScaleFactor'.
        with_tester(|tester| {
            fn size_almost_equal(a: Size, b: Size) -> bool {
                (a.width() - b.width()).abs() <= 0.05 && (a.height() - b.height()).abs() <= 0.05
            }
            let build_app = |tester: &mut WidgetTester, text_scaler: TextScaler| {
                pump_app(tester, m3, move |_app| {
                    MediaQuery::new(
                        MediaQueryData::new().text_scaler(text_scaler),
                        MaterialBanner::new(
                            SizedBox::new().child(Center::new().child(AhemText::new("A"))),
                            vec![action("B")],
                        )
                        .force_actions_below(true),
                    )
                    .into_widget()
                });
            };

            build_app(tester, TextScaler::NO_SCALING);
            assert_eq!(tester.find_ahem_text("A").len(), 1);
            assert_eq!(content_rect(tester, "A").size(), Size::new(14.25, 20.0));

            build_app(tester, TextScaler::linear(1.1));
            tester.pump_and_settle();
            assert!(size_almost_equal(
                content_rect(tester, "A").size(),
                Size::new(15.65, 22.0)
            ));

            build_app(tester, TextScaler::linear(1.5));
            assert!(size_almost_equal(
                content_rect(tester, "A").size(),
                Size::new(21.25, 30.0)
            ));

            build_app(tester, TextScaler::linear(4.0));
            assert!(size_almost_equal(
                content_rect(tester, "A").size(),
                Size::new(21.25, 30.0)
            ));
        });
    }

    /// Dart's `buildBanner` of the 'MaterialBanner elevation' group.
    fn pump_elevation_banner(
        tester: &mut WidgetTester,
        tap_target: &GlobalKey,
        elevation: Option<f64>,
        theme_elevation: Option<f64>,
    ) {
        let mut banner_theme = MaterialBannerThemeData::new();
        banner_theme.elevation = theme_elevation;
        pump_app(
            tester,
            move |app| ThemeData::builder().banner_theme(banner_theme).build(app),
            |_app| {
                messenger_home(tap_target, true, move |_app, context| {
                    let mut banner = MaterialBanner::new(
                        AhemText::new("MaterialBanner"),
                        vec![dismiss(context).into_widget()],
                    );
                    banner.elevation = elevation;
                    banner
                })
            },
        );
    }

    fn banner_elevation(tester: &WidgetTester) -> f64 {
        material_values(tester, banner_material(tester)).0
    }

    #[test]
    fn the_elevation_defaults_to_zero() {
        // Dart: 'MaterialBanner elevation' / 'Elevation defaults to 0'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();

            pump_elevation_banner(tester, &tap_target, None, None);
            tap_key(tester, &tap_target);
            tester.pump_and_settle();
            assert_eq!(banner_elevation(tester), 0.0);
            tap_text(tester, "DISMISS");
            tester.pump_and_settle();

            pump_elevation_banner(tester, &tap_target, None, Some(6.0));
            tap_key(tester, &tap_target);
            tester.pump_and_settle();
            assert_eq!(banner_elevation(tester), 6.0);
            tap_text(tester, "DISMISS");
            tester.pump_and_settle();

            pump_elevation_banner(tester, &tap_target, Some(3.0), Some(6.0));
            tap_key(tester, &tap_target);
            tester.pump_and_settle();
            assert_eq!(banner_elevation(tester), 3.0);
            tap_text(tester, "DISMISS");
            tester.pump_and_settle();
        });
    }

    #[test]
    fn the_elevation_of_the_banner_theme_is_used_by_default() {
        // Dart: 'MaterialBanner elevation' / 'Uses elevation of MaterialBannerTheme by default'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();

            pump_elevation_banner(tester, &tap_target, None, Some(6.0));
            tap_key(tester, &tap_target);
            tester.pump_and_settle();
            assert_eq!(banner_elevation(tester), 6.0);
            tap_text(tester, "DISMISS");
            tester.pump_and_settle();

            pump_elevation_banner(tester, &tap_target, Some(3.0), Some(6.0));
            tap_key(tester, &tap_target);
            tester.pump_and_settle();
            assert_eq!(banner_elevation(tester), 3.0);
            tap_text(tester, "DISMISS");
            tester.pump_and_settle();
        });
    }

    #[test]
    fn a_banner_of_elevation_zero_pushes_the_scaffold_body_down() {
        // Dart: 'MaterialBanner elevation' / 'Scaffold body is pushed down if elevation is 0'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();

            pump_elevation_banner(tester, &tap_target, Some(0.0), None);
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let content_top_left = key_rect(tester, &tap_target).top_left();
            let banner_bottom_left = rect(tester, banner(tester)).bottom_left();

            assert_eq!(content_top_left.dx(), 0.0);
            assert!(content_top_left.dy() >= banner_bottom_left.dy());
        });
    }

    #[test]
    fn a_shown_banner_stays_until_dismissed() {
        // Dart: 'MaterialBanner control test'.
        with_tester(|tester| {
            const HELLO_MATERIAL_BANNER: &str = "Hello MaterialBanner";
            let tap_target = GlobalKey::new();
            let dismiss_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                let dismiss_target = dismiss_target.clone();
                messenger_home(&tap_target, true, move |_app, context| {
                    MaterialBanner::new(
                        AhemText::new(HELLO_MATERIAL_BANNER),
                        vec![
                            dismiss(context)
                                .key(dismiss_target.clone().into())
                                .into_widget(),
                        ],
                    )
                })
            });
            let shown = |tester: &WidgetTester| tester.find_ahem_text(HELLO_MATERIAL_BANNER).len();
            assert_eq!(shown(tester), 0);
            tap_key(tester, &tap_target);
            assert_eq!(shown(tester), 0);
            tester.pump(Duration::ZERO); // schedule animation
            assert_eq!(shown(tester), 1);
            tester.pump(Duration::ZERO); // begin animation
            assert_eq!(shown(tester), 1);
            // 0.75s // animation last frame; two second timer starts here
            tester.pump(Duration::from_millis(750));
            assert_eq!(shown(tester), 1);
            tester.pump(Duration::from_millis(750)); // 1.50s
            assert_eq!(shown(tester), 1);
            tester.pump(Duration::from_millis(750)); // 2.25s
            assert_eq!(shown(tester), 1);
            tap_key(tester, &dismiss_target);
            tester.pump(Duration::ZERO); // begin animation
            assert_eq!(shown(tester), 1); // frame 0 of dismiss animation
            // 3.75s // last frame of animation, material banner removed from build
            tester.pump_and_settle();
            assert_eq!(shown(tester), 0);
        });
    }

    #[test]
    fn a_second_banner_waits_for_the_first_to_close() {
        // Dart: 'MaterialBanner twice test'.
        with_tester(|tester| {
            let material_banner_count = Rc::new(Cell::new(0));
            let tap_target = GlobalKey::new();
            let dismiss_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                let dismiss_target = dismiss_target.clone();
                let material_banner_count = Rc::clone(&material_banner_count);
                messenger_home(&tap_target, true, move |_app, context| {
                    material_banner_count.set(material_banner_count.get() + 1);
                    MaterialBanner::new(
                        AhemText::new(&format!("banner{}", material_banner_count.get())),
                        vec![
                            dismiss(context)
                                .key(dismiss_target.clone().into())
                                .into_widget(),
                        ],
                    )
                })
            });
            let shown = |tester: &WidgetTester| {
                (
                    tester.find_ahem_text("banner1").len(),
                    tester.find_ahem_text("banner2").len(),
                )
            };
            assert_eq!(shown(tester), (0, 0));
            tap_key(tester, &tap_target); // queue banner1
            tap_key(tester, &tap_target); // queue banner2
            assert_eq!(shown(tester), (0, 0));
            tester.pump(Duration::ZERO); // schedule animation for banner1
            assert_eq!(shown(tester), (1, 0));
            tester.pump(Duration::ZERO); // begin animation
            assert_eq!(shown(tester), (1, 0));
            tester.pump(Duration::from_millis(750)); // 0.75s // animation last frame
            assert_eq!(shown(tester), (1, 0));
            tester.pump(Duration::from_millis(750)); // 1.50s
            assert_eq!(shown(tester), (1, 0));
            tester.pump(Duration::from_millis(750)); // 2.25s
            assert_eq!(shown(tester), (1, 0));
            tap_key(tester, &dismiss_target);
            tester.pump(Duration::ZERO); // begin animation
            assert_eq!(shown(tester), (1, 0));
            // 3.75s // last frame of animation, material banner removed from build, new
            // material banner put in its place
            tester.pump(Duration::from_millis(750));
            assert_eq!(shown(tester), (0, 1));
            tester.pump(Duration::ZERO); // begin animation
            assert_eq!(shown(tester), (0, 1));
            tester.pump(Duration::from_millis(750)); // 4.50s // animation last frame
            assert_eq!(shown(tester), (0, 1));
            tester.pump(Duration::from_millis(750)); // 5.25s
            assert_eq!(shown(tester), (0, 1));
            tester.pump(Duration::from_millis(750)); // 6.00s
            assert_eq!(shown(tester), (0, 1));
            tap_key(tester, &dismiss_target); // reverse animation is scheduled
            tester.pump(Duration::ZERO); // begin animation
            assert_eq!(shown(tester), (0, 1));
            // 7.50s // last frame of animation, material banner removed from build
            tester.pump(Duration::from_millis(750));
            assert_eq!(shown(tester), (0, 0));
        });
    }

    #[test]
    fn showing_a_snack_bar_does_not_duplicate_the_banner() {
        // Dart: 'ScaffoldMessenger does not duplicate a MaterialBanner when presenting a
        // SnackBar.'
        with_tester(|tester| {
            let material_banner_tap_target = GlobalKey::new();
            let snack_bar_tap_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                let material_banner_tap_target = material_banner_tap_target.clone();
                let snack_bar_tap_target = snack_bar_tap_target.clone();
                Scaffold::new()
                    .body(Builder::new(move |_app, context| {
                        Column::new()
                            .children([
                                GestureDetector::new()
                                    .key(snack_bar_tap_target.clone().into())
                                    .on_tap(Listener::new(move |app| {
                                        ScaffoldMessenger::of(app, context).show_snack_bar(
                                            app,
                                            SnackBar::new(Text::new("SnackBar")),
                                            None,
                                        );
                                    }))
                                    .behavior(HitTestBehavior::Opaque)
                                    .child(SizedBox::new().height(100.0).width(100.0))
                                    .into_widget(),
                                GestureDetector::new()
                                    .key(material_banner_tap_target.clone().into())
                                    .on_tap(Listener::new(move |app| {
                                        ScaffoldMessenger::of(app, context).show_material_banner(
                                            app,
                                            MaterialBanner::new(
                                                AhemText::new("MaterialBanner"),
                                                vec![dismiss(context).into_widget()],
                                            ),
                                        );
                                    }))
                                    .behavior(HitTestBehavior::Opaque)
                                    .child(SizedBox::new().height(100.0).width(100.0))
                                    .into_widget(),
                            ])
                            .into_widget()
                    }))
                    .into_widget()
            });
            tap_key(tester, &snack_bar_tap_target);
            tap_key(tester, &material_banner_tap_target);
            tester.pump_and_settle();

            assert_eq!(tester.find_text("SnackBar").len(), 1);
            assert_eq!(tester.find_ahem_text("MaterialBanner").len(), 1);
        });
    }

    #[test]
    fn a_single_action_beside_the_content_is_aligned_to_the_trailing_edge() {
        // Dart: 'Single action laid out beside content but aligned to the trailing edge'.
        with_tester(|tester| {
            pump_app(tester, m3, |_app| {
                MaterialBanner::new(AhemText::new("Content"), vec![action("Action")]).into_widget()
            });

            let actions_top_right = overflow_bar_rect(tester).top_right();
            let banner_top_right = rect(tester, banner(tester)).top_right();
            // actions OverflowBar is padded by 8
            // Within f32: global positions go through valo's f32 matrices (waft-gestures'
            // `Matrix4` entry), where Dart's are f64 and compare equal.
            assert!((actions_top_right.dx() + 8.0 - banner_top_right.dx()).abs() < 1e-4);
        });
    }

    #[test]
    fn a_single_action_beside_the_content_of_a_shown_banner_is_aligned_to_the_trailing_edge() {
        // Dart: 'Single action laid out beside content but aligned to the trailing edge when
        // presented by ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                messenger_home(&tap_target, true, |_app, context| {
                    MaterialBanner::new(
                        AhemText::new("Content"),
                        vec![dismiss(context).into_widget()],
                    )
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let actions_top_right = overflow_bar_rect(tester).top_right();
            let banner_top_right = rect(tester, banner(tester)).top_right();
            // actions OverflowBar is padded by 8
            // Within f32: global positions go through valo's f32 matrices (waft-gestures'
            // `Matrix4` entry), where Dart's are f64 and compare equal.
            assert!((actions_top_right.dx() + 8.0 - banner_top_right.dx()).abs() < 1e-4);
        });
    }

    #[test]
    fn a_single_action_beside_the_content_is_aligned_to_the_trailing_edge_in_rtl() {
        // Dart: 'Single action laid out beside content but aligned to the trailing edge - RTL'.
        with_tester(|tester| {
            pump_app(tester, m3, |_app| {
                Directionality::new(
                    TextDirection::Rtl,
                    MaterialBanner::new(AhemText::new("Content"), vec![action("Action")]),
                )
                .into_widget()
            });

            let actions_top_left = overflow_bar_rect(tester).top_left();
            let banner_top_left = rect(tester, banner(tester)).top_left();
            // actions OverflowBar is padded by 8
            assert!((actions_top_left.dx() - 8.0 - banner_top_left.dx()).abs() < 1e-10);
        });
    }

    #[test]
    fn a_single_action_beside_the_content_of_a_shown_banner_is_aligned_to_the_trailing_edge_in_rtl()
    {
        // Dart: 'Single action laid out beside content but aligned to the trailing edge when
        // presented by ScaffoldMessenger - RTL'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                Directionality::new(
                    TextDirection::Rtl,
                    messenger_home(&tap_target, true, |_app, context| {
                        MaterialBanner::new(
                            AhemText::new("Content"),
                            vec![dismiss(context).into_widget()],
                        )
                    }),
                )
                .into_widget()
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let actions_top_left = overflow_bar_rect(tester).top_left();
            let banner_top_left = rect(tester, banner(tester)).top_left();
            // actions OverflowBar is padded by 8
            assert!((actions_top_left.dx() - 8.0 - banner_top_left.dx()).abs() < 1e-10);
        });
    }

    #[test]
    fn forcing_actions_below_lays_one_action_out_below_the_content() {
        // Dart: 'Actions laid out below content if forced override'.
        with_tester(|tester| {
            pump_app(tester, m3, |_app| {
                MaterialBanner::new(AhemText::new("Content"), vec![action("Action")])
                    .force_actions_below(true)
                    .into_widget()
            });

            let content_bottom_left = content_rect(tester, "Content").bottom_left();
            let actions_top_left = overflow_bar_rect(tester).top_left();
            assert!(content_bottom_left.dy() < actions_top_left.dy());
            assert!(content_bottom_left.dx() < actions_top_left.dx());
        });
    }

    #[test]
    fn forcing_actions_below_lays_one_action_of_a_shown_banner_out_below_the_content() {
        // Dart: 'Actions laid out below content if forced override when presented by
        // ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                messenger_home(&tap_target, true, |_app, context| {
                    MaterialBanner::new(
                        AhemText::new("Content"),
                        vec![dismiss(context).into_widget()],
                    )
                    .force_actions_below(true)
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let content_bottom_left = content_rect(tester, "Content").bottom_left();
            let actions_top_left = overflow_bar_rect(tester).top_left();
            assert!(content_bottom_left.dy() < actions_top_left.dy());
            assert!(content_bottom_left.dx() < actions_top_left.dx());
        });
    }

    /// Dart's `SizedBox(width:, height:, key: ValueKey<int>(index))` actions, the first holding
    /// `first_child` when one is given; the keys find them.
    fn sized_actions(
        keys: &[GlobalKey],
        width: f64,
        height: f64,
        first_child: Option<WidgetRef>,
    ) -> Vec<WidgetRef> {
        keys.iter()
            .enumerate()
            .map(|(index, key)| {
                let action = SizedBox::new()
                    .width(width)
                    .height(height)
                    .key(key.clone().into());
                match (&first_child, index) {
                    (Some(child), 0) => action.child(child.clone()).into_widget(),
                    _ => action.into_widget(),
                }
            })
            .collect()
    }

    /// Dart's `GestureDetector(key: dismissTarget, onTap: () =>
    /// ScaffoldMessenger.of(context).hideCurrentMaterialBanner())`.
    fn dismiss_detector(dismiss_target: &GlobalKey, context: BuildContext) -> WidgetRef {
        GestureDetector::new()
            .key(dismiss_target.clone().into())
            .on_tap(Listener::new(move |app| {
                ScaffoldMessenger::of(app, context)
                    .hide_current_material_banner(app, MaterialBannerClosedReason::Hide);
            }))
            .into_widget()
    }

    fn keys(count: usize) -> Vec<GlobalKey> {
        (0..count).map(|_| GlobalKey::new()).collect()
    }

    fn top_left(tester: &WidgetTester, key: &GlobalKey) -> waft_embedder::Offset {
        key_rect(tester, key).top_left()
    }

    #[test]
    fn the_action_widgets_are_laid_out_as_the_button_bar_laid_them_out() {
        // Dart: 'Action widgets layout'.
        with_tester(|tester| {
            let build_frame = |tester: &mut WidgetTester,
                               actions: &[GlobalKey],
                               text_direction: TextDirection| {
                let actions = sized_actions(actions, 64.0, 48.0, None);
                pump_app(tester, m3, move |_app| {
                    Directionality::new(
                        text_direction,
                        MaterialBanner::new(SizedBox::new().width(100.0).height(100.0), actions),
                    )
                    .into_widget()
                });
            };
            let one = keys(1);
            let three = keys(3);
            // The action coordinates that follow were obtained by running
            // the test code, before ButtonBar was replaced by OverflowBar.

            build_frame(tester, &one, TextDirection::Ltr);
            assert_eq!(
                top_left(tester, &one[0]),
                waft_embedder::Offset::new(728.0, 28.0)
            );

            build_frame(tester, &one, TextDirection::Rtl);
            assert_eq!(
                top_left(tester, &one[0]),
                waft_embedder::Offset::new(8.0, 28.0)
            );

            build_frame(tester, &three, TextDirection::Ltr);
            assert_eq!(
                top_left(tester, &three[0]),
                waft_embedder::Offset::new(584.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[1]),
                waft_embedder::Offset::new(656.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[2]),
                waft_embedder::Offset::new(728.0, 130.0)
            );

            build_frame(tester, &three, TextDirection::Rtl);
            assert_eq!(
                top_left(tester, &three[0]),
                waft_embedder::Offset::new(152.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[1]),
                waft_embedder::Offset::new(80.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[2]),
                waft_embedder::Offset::new(8.0, 130.0)
            );
        });
    }

    /// Pumps Dart's `buildFrame` of the cases presented by the messenger: a scaffold whose
    /// body shows a banner of `actions`, the first of which dismisses it, then taps to show it.
    fn show_sized_actions(
        tester: &mut WidgetTester,
        tap_target: &GlobalKey,
        dismiss_target: &GlobalKey,
        actions: &[GlobalKey],
        size: (f64, f64),
        text_direction: TextDirection,
        overflow_alignment: Option<OverflowBarAlignment>,
    ) {
        let actions = actions.to_vec();
        let dismiss_target = dismiss_target.clone();
        pump_app(tester, m3, |_app| {
            Directionality::new(
                text_direction,
                messenger_home(tap_target, false, move |_app, context| {
                    let mut banner = MaterialBanner::new(
                        SizedBox::new().width(100.0).height(100.0),
                        sized_actions(
                            &actions,
                            size.0,
                            size.1,
                            Some(dismiss_detector(&dismiss_target, context)),
                        ),
                    );
                    if let Some(overflow_alignment) = overflow_alignment {
                        banner = banner.overflow_alignment(overflow_alignment);
                    }
                    banner
                }),
            )
            .into_widget()
        });
        tap_key(tester, tap_target);
        tester.pump_and_settle();
    }

    #[test]
    fn the_action_widgets_of_a_shown_banner_are_laid_out_as_the_button_bar_laid_them_out() {
        // Dart: 'Action widgets layout when presented by ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            let dismiss_target = GlobalKey::new();
            let one = keys(1);
            let three = keys(3);
            let size = (64.0, 48.0);
            // The action coordinates that follow were obtained by running
            // the test code, before ButtonBar was replaced by OverflowBar.

            show_sized_actions(
                tester,
                &tap_target,
                &dismiss_target,
                &one,
                size,
                TextDirection::Ltr,
                None,
            );
            assert_eq!(
                top_left(tester, &one[0]),
                waft_embedder::Offset::new(728.0, 28.0)
            );
            tap_key(tester, &dismiss_target);
            tester.pump_and_settle();

            show_sized_actions(
                tester,
                &tap_target,
                &dismiss_target,
                &one,
                size,
                TextDirection::Rtl,
                None,
            );
            assert_eq!(
                top_left(tester, &one[0]),
                waft_embedder::Offset::new(8.0, 28.0)
            );
            tap_key(tester, &dismiss_target);
            tester.pump_and_settle();

            show_sized_actions(
                tester,
                &tap_target,
                &dismiss_target,
                &three,
                size,
                TextDirection::Ltr,
                None,
            );
            assert_eq!(
                top_left(tester, &three[0]),
                waft_embedder::Offset::new(584.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[1]),
                waft_embedder::Offset::new(656.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[2]),
                waft_embedder::Offset::new(728.0, 130.0)
            );
            tap_key(tester, &dismiss_target);
            tester.pump_and_settle();

            show_sized_actions(
                tester,
                &tap_target,
                &dismiss_target,
                &three,
                size,
                TextDirection::Rtl,
                None,
            );
            assert_eq!(
                top_left(tester, &three[0]),
                waft_embedder::Offset::new(152.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[1]),
                waft_embedder::Offset::new(80.0, 130.0)
            );
            assert_eq!(
                top_left(tester, &three[2]),
                waft_embedder::Offset::new(8.0, 130.0)
            );
            tap_key(tester, &dismiss_target);
            tester.pump_and_settle();
        });
    }

    fn assert_stacked(tester: &WidgetTester, actions: &[GlobalKey], dx: f64) {
        for (index, key) in actions.iter().enumerate() {
            assert_eq!(
                top_left(tester, key),
                waft_embedder::Offset::new(dx, 134.0 + index as f64 * 10.0)
            );
        }
    }

    /// Pumps a banner of four 200x10 actions straight under the app: Dart's `buildFrame` of
    /// the overflow cases.
    fn pump_overflowing_actions(
        tester: &mut WidgetTester,
        actions: &[GlobalKey],
        text_direction: TextDirection,
        overflow_alignment: Option<OverflowBarAlignment>,
    ) {
        let actions = sized_actions(actions, 200.0, 10.0, None);
        pump_app(tester, m3, move |_app| {
            let mut banner =
                MaterialBanner::new(SizedBox::new().width(100.0).height(100.0), actions);
            if let Some(overflow_alignment) = overflow_alignment {
                banner = banner.overflow_alignment(overflow_alignment);
            }
            Directionality::new(text_direction, banner).into_widget()
        });
    }

    #[test]
    fn overflowing_action_widgets_are_laid_out_as_the_button_bar_laid_them_out() {
        // Dart: 'Action widgets layout with overflow'.
        with_tester(|tester| {
            let actions = keys(4);
            // The action coordinates that follow were obtained by running
            // the test code, before ButtonBar was replaced by OverflowBar.

            pump_overflowing_actions(tester, &actions, TextDirection::Ltr, None);
            assert_stacked(tester, &actions, 592.0);

            pump_overflowing_actions(tester, &actions, TextDirection::Rtl, None);
            assert_stacked(tester, &actions, 8.0);
        });
    }

    #[test]
    fn overflowing_action_widgets_of_a_shown_banner_are_laid_out_as_the_button_bar_laid_them_out() {
        // Dart: 'Action widgets layout with overflow when presented by ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            let dismiss_target = GlobalKey::new();
            let actions = keys(4);
            let size = (200.0, 10.0);
            // The action coordinates that follow were obtained by running
            // the test code, before ButtonBar was replaced by OverflowBar.

            show_sized_actions(
                tester,
                &tap_target,
                &dismiss_target,
                &actions,
                size,
                TextDirection::Ltr,
                None,
            );
            assert_stacked(tester, &actions, 592.0);
            tap_key(tester, &dismiss_target);
            tester.pump_and_settle();

            show_sized_actions(
                tester,
                &tap_target,
                &dismiss_target,
                &actions,
                size,
                TextDirection::Rtl,
                None,
            );
            assert_stacked(tester, &actions, 8.0);
            tap_key(tester, &dismiss_target);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn the_overflow_alignment_aligns_the_overflowing_actions() {
        // Dart: '[overflowAlignment] test'.
        with_tester(|tester| {
            let actions = keys(4);

            pump_overflowing_actions(
                tester,
                &actions,
                TextDirection::Ltr,
                Some(OverflowBarAlignment::Start),
            );
            assert_stacked(tester, &actions, 8.0);

            pump_overflowing_actions(
                tester,
                &actions,
                TextDirection::Ltr,
                Some(OverflowBarAlignment::Center),
            );
            assert_stacked(tester, &actions, 300.0);

            pump_overflowing_actions(
                tester,
                &actions,
                TextDirection::Ltr,
                Some(OverflowBarAlignment::End),
            );
            assert_stacked(tester, &actions, 592.0);
        });
    }

    #[test]
    fn the_overflow_alignment_aligns_the_overflowing_actions_of_a_shown_banner() {
        // Dart: '[overflowAlignment] test when presented by ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            let dismiss_target = GlobalKey::new();
            let actions = keys(4);
            let size = (200.0, 10.0);

            for (overflow_alignment, dx) in [
                (OverflowBarAlignment::Start, 8.0),
                (OverflowBarAlignment::Center, 300.0),
                (OverflowBarAlignment::End, 592.0),
            ] {
                show_sized_actions(
                    tester,
                    &tap_target,
                    &dismiss_target,
                    &actions,
                    size,
                    TextDirection::Ltr,
                    Some(overflow_alignment),
                );
                assert_stacked(tester, &actions, dx);
                tap_key(tester, &dismiss_target);
                tester.pump_and_settle();
            }
        });
    }

    #[test]
    fn the_messenger_asserts_when_it_has_no_scaffold_to_show_a_banner_in() {
        // Dart: 'ScaffoldMessenger will alert for MaterialBanners that cannot be presented'.
        with_tester(|tester| {
            let context = Rc::new(Cell::new(None));
            let seen = Rc::clone(&context);
            pump_app(tester, m3, move |_app| {
                Builder::new(move |_app, context| {
                    seen.set(Some(context));
                    Center::new().into_widget()
                })
                .into_widget()
            });
            let context = context.get().expect("the home built");

            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut app = tester.cell.borrow_mut();
                let scaffold_messenger_state = ScaffoldMessenger::of(&mut app, context);
                scaffold_messenger_state.show_material_banner(
                    &mut app,
                    MaterialBanner::new(Text::new("Banner"), Vec::new()),
                );
            }));
            let panic = result.expect_err("the assert fires");
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| message.to_string())
                })
                .expect("a panic message");
            assert!(message.contains(
                "ScaffoldMessenger.showMaterialBanner was called, but there are currently no \
                 descendant Scaffolds to present to."
            ));
        });
    }

    #[test]
    fn a_custom_margin_is_respected() {
        // Dart: 'Custom Margin respected'.
        with_tester(|tester| {
            let margin = EdgeInsets::all(30.0);
            pump_app(tester, m3, move |_app| {
                MaterialBanner::new(AhemText::new("I am a banner"), vec![action("Action")])
                    .margin(margin)
                    .into_widget()
            });

            let top_left = rect(tester, banner_material(tester)).top_left();

            // Compare the offset of banner from top left
            assert_eq!(top_left.dx(), margin.left);
        });
    }

    #[test]
    fn the_minimum_action_bar_height_is_respected() {
        // Dart: 'minActionBarHeight is respected'.
        with_tester(|tester| {
            const MIN_ACTION_BAR_HEIGHT: f64 = 20.0;
            pump_app(tester, m3, |_app| {
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new(),
                    ))
                    .body(
                        MaterialBanner::new(
                            SizedBox::shrink(),
                            vec![SizedBox::shrink().into_widget()],
                        )
                        .min_action_bar_height(MIN_ACTION_BAR_HEIGHT)
                        .padding(EdgeInsets::ZERO)
                        .margin(EdgeInsets::ZERO),
                    )
                    .into_widget()
            });

            let size = rect(tester, banner(tester)).size();
            assert_eq!(size.height(), MIN_ACTION_BAR_HEIGHT);
        });
    }

    #[test]
    fn the_minimum_action_bar_height_of_a_shown_banner_is_respected() {
        // Dart: 'minimumActionBarHeight is respected when presented by ScaffoldMessenger'.
        with_tester(|tester| {
            const MIN_ACTION_BAR_HEIGHT: f64 = 20.0;
            let tap_target = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                messenger_home(&tap_target, true, |_app, _context| {
                    MaterialBanner::new(SizedBox::shrink(), vec![SizedBox::shrink().into_widget()])
                        .padding(EdgeInsets::ZERO)
                        .margin(EdgeInsets::ZERO)
                        .min_action_bar_height(MIN_ACTION_BAR_HEIGHT)
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let material_bar_size = rect(tester, banner(tester)).size();
            assert_eq!(material_bar_size.height(), MIN_ACTION_BAR_HEIGHT);
        });
    }

    #[test]
    fn a_banner_renders_at_zero_size() {
        // Dart: 'MaterialBanner renders at zero size'.
        with_tester(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(MaterialBanner::new(
                        AhemText::new("X"),
                        vec![SizedBox::shrink().into_widget()],
                    )))
                    .into_widget()
            });
            assert!(content_rect(tester, "X").size().is_empty());
        });
    }

    // banner_theme_test.dart

    /// Dart's `_bannerTheme()`.
    fn theme_banner() -> MaterialBannerThemeData {
        MaterialBannerThemeData::new()
            .background_color(Colors::ORANGE.primary())
            .surface_tint_color(Colors::YELLOW.primary())
            .shadow_color(Colors::RED.primary())
            .divider_color(Colors::GREEN.primary())
            .content_text_style(TextStyle::new().color(Colors::PINK.primary()))
            .elevation(4.0)
            .padding(EdgeInsetsGeometry::all(5.0))
            .leading_padding(EdgeInsetsGeometry::all(6.0))
    }

    fn row_rect(tester: &WidgetTester) -> Rect {
        rect(tester, in_banner::<Row>(tester)[0])
    }

    fn icon_rect(tester: &WidgetTester, icon: &waft_widgets::IconData) -> Rect {
        rect(tester, tester.find_by_icon(icon))
    }

    /// Asserts the default single-line paddings Dart's defaults cases check.
    fn assert_default_paddings(tester: &WidgetTester) {
        let row_top_left = row_rect(tester).top_left();
        let material_top_left = rect(tester, banner_material(tester)).top_left();
        let leading_top_left = icon_rect(tester, &Icons::umbrella()).top_left();
        // Default single line top padding.
        assert_eq!(row_top_left.dy() - material_top_left.dy(), 2.0);
        // Default single line start padding.
        assert_eq!(row_top_left.dx() - material_top_left.dx(), 16.0);
        // Default leading padding.
        assert_eq!(leading_top_left.dy() - material_top_left.dy(), 16.0);
        assert_eq!(leading_top_left.dx() - material_top_left.dx(), 16.0);
    }

    /// Asserts where the content and the leading `Icons.ac_unit` sit inside the material.
    fn assert_content_and_leading(tester: &WidgetTester, content: (f64, f64), leading: (f64, f64)) {
        let content_top_left = content_rect(tester, "Content").top_left();
        let material_top_left = rect(tester, banner_material(tester)).top_left();
        let leading_top_left = icon_rect(tester, &Icons::ac_unit()).top_left();
        assert_eq!(content_top_left.dy() - material_top_left.dy(), content.0);
        assert_eq!(content_top_left.dx() - material_top_left.dx(), content.1);
        assert_eq!(leading_top_left.dy() - material_top_left.dy(), leading.0);
        assert_eq!(leading_top_left.dx() - material_top_left.dx(), leading.1);
    }

    #[test]
    fn material3_without_a_banner_theme_uses_the_defaults() {
        // Dart: 'Material3 - Passing no MaterialBannerThemeData returns defaults'.
        with_tester(|tester| {
            let (content_widget, style) = content("Content");
            let localized_theme = Rc::new(RefCell::new(None));
            let seen = Rc::clone(&localized_theme);
            pump_app(tester, m3, move |_app| {
                Scaffold::new()
                    .body(Builder::new(move |app, context| {
                        *seen.borrow_mut() = Some(Theme::of(app, context));
                        MaterialBanner::new(content_widget.clone(), vec![action("Action")])
                            .leading(waft_widgets::Icon::new(Some(Icons::umbrella())))
                            .into_widget()
                    }))
                    .into_widget()
            });
            let theme = ThemeData::new(&tester.cell.borrow());
            let localized_theme = localized_theme.borrow().clone().expect("the body built");

            let (elevation, color, surface_tint_color, shadow_color) =
                material_values(tester, material_around(tester, "Content"));
            assert_eq!(color, Some(theme.color_scheme.surface_container_low()));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shadow_color, None);
            assert_eq!(elevation, 0.0);

            assert_eq!(*style.borrow(), localized_theme.text_theme.body_medium);

            assert_default_paddings(tester);

            assert_eq!(
                divider_color(tester),
                Some(theme.color_scheme.outline_variant())
            );
        });
    }

    #[test]
    fn material3_without_a_banner_theme_uses_the_defaults_for_a_shown_banner() {
        // Dart: 'Material3 - Passing no MaterialBannerThemeData returns defaults when presented
        // by ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            let (content_widget, style) = content("Content");
            let localized_theme = Rc::new(RefCell::new(None));
            let seen = Rc::clone(&localized_theme);
            pump_app(tester, m3, |_app| {
                messenger_home(&tap_target, true, move |app, _context| {
                    // Dart reads `localizedTheme` in the body's builder, whose context this is.
                    *seen.borrow_mut() = Some(Theme::of(app, _context));
                    MaterialBanner::new(content_widget.clone(), vec![action("Action")])
                        .leading(waft_widgets::Icon::new(Some(Icons::umbrella())))
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();
            let theme = ThemeData::new(&tester.cell.borrow());
            let localized_theme = localized_theme
                .borrow()
                .clone()
                .expect("the banner was shown");

            let (elevation, color, surface_tint_color, shadow_color) =
                material_values(tester, material_around(tester, "Content"));
            assert_eq!(color, Some(theme.color_scheme.surface_container_low()));
            assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(shadow_color, None);
            assert_eq!(elevation, 0.0);

            assert_eq!(*style.borrow(), localized_theme.text_theme.body_medium);

            assert_default_paddings(tester);

            assert_eq!(
                divider_color(tester),
                Some(theme.color_scheme.outline_variant())
            );
        });
    }

    fn banner_theme_app(app: &App) -> ThemeData {
        ThemeData::builder().banner_theme(theme_banner()).build(app)
    }

    #[test]
    fn a_banner_uses_the_values_of_the_banner_theme() {
        // Dart: 'MaterialBanner uses values from MaterialBannerThemeData'.
        with_tester(|tester| {
            let banner_theme = theme_banner();
            let (content_widget, style) = content("Content");
            pump_app(tester, banner_theme_app, move |_app| {
                Scaffold::new()
                    .body(
                        MaterialBanner::new(content_widget, vec![action("Action")])
                            .leading(waft_widgets::Icon::new(Some(Icons::ac_unit()))),
                    )
                    .into_widget()
            });

            assert_eq!(
                material_values(tester, material_around(tester, "Content")),
                (
                    banner_theme.elevation.expect("set"),
                    banner_theme.background_color,
                    banner_theme.surface_tint_color,
                    banner_theme.shadow_color
                )
            );
            assert_eq!(*style.borrow(), banner_theme.content_text_style);

            assert_content_and_leading(tester, (24.0, 41.0), (19.0, 11.0));

            assert!(tester.find_by_type::<Divider>().is_empty());
        });
    }

    #[test]
    fn a_shown_banner_uses_the_values_of_the_banner_theme() {
        // Dart: 'MaterialBanner uses values from MaterialBannerThemeData when presented by
        // ScaffoldMessenger'.
        with_tester(|tester| {
            let banner_theme = theme_banner();
            let tap_target = GlobalKey::new();
            let (content_widget, style) = content("Content");
            pump_app(tester, banner_theme_app, |_app| {
                messenger_home(&tap_target, true, move |_app, _context| {
                    MaterialBanner::new(content_widget.clone(), vec![action("Action")])
                        .leading(waft_widgets::Icon::new(Some(Icons::ac_unit())))
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            assert_eq!(
                material_values(tester, material_around(tester, "Content")),
                (
                    banner_theme.elevation.expect("set"),
                    banner_theme.background_color,
                    banner_theme.surface_tint_color,
                    banner_theme.shadow_color
                )
            );
            assert_eq!(*style.borrow(), banner_theme.content_text_style);

            assert_content_and_leading(tester, (24.0, 41.0), (19.0, 11.0));

            assert!(tester.find_by_type::<Divider>().is_empty());
        });
    }

    #[test]
    fn the_banner_properties_take_priority_over_the_theme() {
        // Dart: 'MaterialBanner widget properties take priority over theme'.
        with_tester(|tester| {
            let text_style = TextStyle::new().color(Colors::GREEN.primary());
            let banner_text_style = text_style.clone();
            let (content_widget, style) = content("Content");
            pump_app(tester, banner_theme_app, move |_app| {
                Scaffold::new()
                    .body(
                        MaterialBanner::new(content_widget, vec![action("Action")])
                            .background_color(Colors::PURPLE.primary())
                            .surface_tint_color(Colors::RED.primary())
                            .shadow_color(Colors::ORANGE.primary())
                            .elevation(6.0)
                            .leading(waft_widgets::Icon::new(Some(Icons::ac_unit())))
                            .content_text_style(banner_text_style)
                            .padding(EdgeInsets::all(10.0))
                            .leading_padding(EdgeInsets::all(12.0)),
                    )
                    .into_widget()
            });

            assert_eq!(
                material_values(tester, material_around(tester, "Content")),
                (
                    6.0,
                    Some(Colors::PURPLE.primary()),
                    Some(Colors::RED.primary()),
                    Some(Colors::ORANGE.primary())
                )
            );
            assert_eq!(*style.borrow(), Some(text_style));

            assert_content_and_leading(tester, (29.0, 58.0), (24.0, 22.0));

            assert!(tester.find_by_type::<Divider>().is_empty());
        });
    }

    #[test]
    fn the_properties_of_a_shown_banner_take_priority_over_the_theme() {
        // Dart: 'MaterialBanner widget properties take priority over theme when presented by
        // ScaffoldMessenger'.
        with_tester(|tester| {
            const ELEVATION: f64 = 6.0;
            let text_style = TextStyle::new().color(Colors::GREEN.primary());
            let banner_text_style = text_style.clone();
            let tap_target = GlobalKey::new();
            let (content_widget, style) = content("Content");
            pump_app(tester, banner_theme_app, |_app| {
                messenger_home(&tap_target, true, move |_app, _context| {
                    MaterialBanner::new(content_widget.clone(), vec![action("Action")])
                        .background_color(Colors::PURPLE.primary())
                        .elevation(ELEVATION)
                        .leading(waft_widgets::Icon::new(Some(Icons::ac_unit())))
                        .content_text_style(banner_text_style.clone())
                        .padding(EdgeInsets::all(10.0))
                        .leading_padding(EdgeInsets::all(12.0))
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let (elevation, color, _, _) =
                material_values(tester, material_around(tester, "Content"));
            assert_eq!(color, Some(Colors::PURPLE.primary()));
            assert_eq!(elevation, ELEVATION);
            assert_eq!(*style.borrow(), Some(text_style));

            assert_content_and_leading(tester, (29.0, 58.0), (24.0, 22.0));

            assert!(tester.find_by_type::<Divider>().is_empty());
        });
    }

    fn purple_surface_scheme() -> ColorScheme {
        ColorScheme::light()
            .copy_with()
            .with_surface(Colors::PURPLE.primary())
    }

    #[test]
    fn a_banner_uses_the_color_scheme_when_necessary() {
        // Dart: 'MaterialBanner uses color scheme when necessary'.
        with_tester(|tester| {
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .color_scheme(purple_surface_scheme())
                        .build(app)
                },
                |_app| {
                    Scaffold::new()
                        .body(MaterialBanner::new(
                            AhemText::new("Content"),
                            vec![action("Action")],
                        ))
                        .into_widget()
                },
            );

            let (_, color, _, _) = material_values(tester, material_around(tester, "Content"));
            assert_eq!(color, Some(purple_surface_scheme().surface_container_low()));
        });
    }

    #[test]
    fn a_shown_banner_uses_the_color_scheme_when_necessary() {
        // Dart: 'MaterialBanner uses color scheme when necessary when presented by
        // ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .color_scheme(purple_surface_scheme())
                        .build(app)
                },
                |_app| {
                    messenger_home(&tap_target, true, |_app, _context| {
                        MaterialBanner::new(AhemText::new("Content"), vec![action("Action")])
                    })
                },
            );
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            let (_, color, _, _) = material_values(tester, material_around(tester, "Content"));
            assert_eq!(color, Some(purple_surface_scheme().surface_container_low()));
        });
    }

    /// Dart's `Typography.material2014().englishLike.bodyMedium!.merge(
    /// Typography.material2014().black.bodyMedium)`.
    fn material2014_body_medium() -> Option<TextStyle> {
        let typography = Typography::material2014().build();
        Some(
            typography
                .english_like
                .body_medium
                .as_ref()
                .expect("a body medium style")
                .merge(typography.black.body_medium.as_ref()),
        )
    }

    /// The Material 2 defaults Dart's 'Material 2' cases check.
    fn assert_material2_defaults(tester: &WidgetTester, style: &RefCell<Option<TextStyle>>) {
        assert_eq!(
            material_values(tester, material_around(tester, "Content")),
            (0.0, Some(Color::new(0xffffffff)), None, None)
        );

        // Default value for ThemeData.typography is Typography.material2014()
        assert_eq!(*style.borrow(), material2014_body_medium());

        assert_default_paddings(tester);

        assert_eq!(divider_color(tester), None);
    }

    #[test]
    fn material2_without_a_banner_theme_uses_the_defaults() {
        // Dart: 'Material 2' / 'Material2 - Passing no MaterialBannerThemeData returns
        // defaults'.
        with_tester(|tester| {
            let (content_widget, style) = content("Content");
            pump_app(tester, m2, move |_app| {
                Scaffold::new()
                    .body(
                        MaterialBanner::new(content_widget, vec![action("Action")])
                            .leading(waft_widgets::Icon::new(Some(Icons::umbrella()))),
                    )
                    .into_widget()
            });

            assert_material2_defaults(tester, &style);
        });
    }

    #[test]
    fn material2_without_a_banner_theme_uses_the_defaults_for_a_shown_banner() {
        // Dart: 'Material 2' / 'Material2 - Passing no MaterialBannerThemeData returns defaults
        // when presented by ScaffoldMessenger'.
        with_tester(|tester| {
            let tap_target = GlobalKey::new();
            let (content_widget, style) = content("Content");
            pump_app(tester, m2, |_app| {
                messenger_home(&tap_target, true, move |_app, _context| {
                    MaterialBanner::new(content_widget.clone(), vec![action("Action")])
                        .leading(waft_widgets::Icon::new(Some(Icons::umbrella())))
                })
            });
            tap_key(tester, &tap_target);
            tester.pump_and_settle();

            assert_material2_defaults(tester, &style);
        });
    }
}
