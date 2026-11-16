//! Flutter counterpart: material_ui's `material.dart`.

use std::any::Any;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Animatable, ColorTween, Curves, Tween};
use waft_embedder::{
    Canvas, ClipOp, Color, Matrix4, Offset, Path, Radius, Rect, Size, TextBaseline, TextDirection,
};
use waft_foundation::{App, Handle, HandleId, Listener, Ref, Retained};
use waft_painting::{
    BorderRadius, BorderRadiusGeometry, BorderSide, CircleBorder, Clip, ClipContext,
    RoundedRectangleBorder, ShapeBorderRef, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, CustomClipper, CustomPainter,
    PaintingContext, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin, ShapeBorderClipper,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProvider, TickerProviderObject};
use waft_widgets::{
    AnimatedDefaultTextStyle, AnimatedPhysicalModel, AnimatedWidgetBaseState, BuildContext,
    ClipPath, CustomPaint, Directionality, GlobalKey, ImplicitlyAnimatedWidget,
    ImplicitlyAnimatedWidgetData, ImplicitlyAnimatedWidgetState, ImplicitlyAnimatedWidgetStateData,
    IntoWidget, KeyRef, LayoutChangedNotification, LookupBoundary, NotificationListener,
    PhysicalShape, RenderObjectWidget, SingleChildRenderObjectWidget,
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, State, StateData,
    StatefulWidget, StatelessWidget, TickerProviderStateMixin, TickerProviderStateMixinData,
    TweenSlot, TweenVisitor, WidgetRef,
};

use crate::constants::K_THEME_CHANGE_DURATION;
use crate::elevation_overlay::ElevationOverlay;
use crate::theme::Theme;

/// Signature for the callback used by ink effects to obtain the rectangle for the effect.
///
/// Used by [`InkHighlight`](crate::InkHighlight) and [`InkSplash`](crate::InkSplash), for
/// example.
pub type RectCallback = Rc<dyn Fn(&App) -> Rect>;

/// The various kinds of material in Material Design. Used to
/// configure the default behavior of [`Material`] widgets.
///
/// See also:
///
///  * [`Material`], in particular [`Material::type`](Material::r#type).
///  * [`k_material_edges`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialType {
    /// Rectangle using default theme canvas color.
    Canvas,

    /// Rounded edges, card theme color.
    Card,

    /// A circle, no color by default (used for floating action buttons).
    Circle,

    /// Rounded edges, no color by default (used for `MaterialButton` buttons).
    Button,

    /// A transparent piece of material that draws ink splashes and highlights.
    ///
    /// While the material metaphor describes child widgets as printed on the
    /// material itself and do not hide ink effects, in practice the [`Material`]
    /// widget draws child widgets on top of the ink effects.
    /// A [`Material`] with type transparency can be placed on top of opaque widgets
    /// to show ink effects on top of them.
    ///
    /// Prefer using the [`Ink`](crate::Ink) widget for showing ink effects on top of opaque
    /// widgets.
    Transparency,
}

/// The border radii used by the various kinds of material in Material Design.
///
/// Dart's `const Map<MaterialType, BorderRadius?> kMaterialEdges`.
///
/// See also:
///
///  * [`MaterialType`]
///  * [`Material`]
pub const fn k_material_edges(r#type: MaterialType) -> Option<BorderRadius> {
    match r#type {
        MaterialType::Canvas => None,
        MaterialType::Card => Some(BorderRadius::all(Radius::circular(2.0))),
        MaterialType::Circle => None,
        MaterialType::Button => Some(BorderRadius::all(Radius::circular(2.0))),
        MaterialType::Transparency => None,
    }
}

/// An interface for creating [`InkSplash`](crate::InkSplash)s and
/// [`InkHighlight`](crate::InkHighlight)s on a [`Material`].
///
/// Typically obtained via [`Material::of`], as the erased [`AnyMaterialInkController`].
pub trait MaterialInkController: Sized + 'static {
    /// This controller as the erased [`AnyMaterialInkController`] — what to pass where a Dart
    /// API takes a `MaterialInkController`.
    fn as_material_ink_controller(self: Handle<Self>) -> AnyMaterialInkController {
        AnyMaterialInkController {
            id: self.id(),
            vtable: const { &MaterialInkControllerVTable::of::<Self>() },
        }
    }

    /// The color of the material.
    fn color(self: Handle<Self>, app: &App) -> Option<Color>;

    /// The ticker provider used by the controller.
    ///
    /// Ink features that are added to this controller with
    /// [`add_ink_feature`](Self::add_ink_feature) should use this vsync to drive their
    /// animations.
    fn vsync(self: Handle<Self>, app: &App) -> Rc<dyn TickerProvider>;

    /// Add an [`InkFeature`], such as an [`InkSplash`](crate::InkSplash) or an
    /// [`InkHighlight`](crate::InkHighlight).
    ///
    /// The ink feature will paint as part of this controller.
    fn add_ink_feature(self: Handle<Self>, app: &mut App, feature: InkFeatureRef);

    /// Notifies the controller that one of its ink features needs to repaint.
    fn mark_needs_paint(self: Handle<Self>, app: &mut App);
}

/// The vtable of an erased [`AnyMaterialInkController`]: one `&'static` table per type.
struct MaterialInkControllerVTable {
    type_name: fn() -> &'static str,
    color: fn(&App, HandleId) -> Option<Color>,
    vsync: fn(&App, HandleId) -> Rc<dyn TickerProvider>,
    add_ink_feature: fn(&mut App, HandleId, InkFeatureRef),
    mark_needs_paint: fn(&mut App, HandleId),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl MaterialInkControllerVTable {
    const fn of<T: MaterialInkController>() -> MaterialInkControllerVTable {
        MaterialInkControllerVTable {
            type_name: std::any::type_name::<T>,
            color: |app, id| T::color(resolve(id), app),
            vsync: |app, id| T::vsync(resolve(id), app),
            add_ink_feature: |app, id, feature| T::add_ink_feature(resolve(id), app, feature),
            mark_needs_paint: |app, id| T::mark_needs_paint(resolve(id), app),
        }
    }
}

/// Erased `MaterialInkController`: one identity and a static vtable.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyMaterialInkController {
    id: HandleId,
    vtable: &'static MaterialInkControllerVTable,
}

impl PartialEq for AnyMaterialInkController {
    fn eq(&self, other: &AnyMaterialInkController) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyMaterialInkController {}

impl fmt::Debug for AnyMaterialInkController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyMaterialInkController {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// See [`MaterialInkController::color`].
    pub fn color(self, app: &App) -> Option<Color> {
        (self.vtable.color)(app, self.id)
    }

    /// See [`MaterialInkController::vsync`].
    pub fn vsync(self, app: &App) -> Rc<dyn TickerProvider> {
        (self.vtable.vsync)(app, self.id)
    }

    /// See [`MaterialInkController::add_ink_feature`].
    pub fn add_ink_feature(self, app: &mut App, feature: InkFeatureRef) {
        (self.vtable.add_ink_feature)(app, self.id, feature);
    }

    /// See [`MaterialInkController::mark_needs_paint`].
    pub fn mark_needs_paint(self, app: &mut App) {
        (self.vtable.mark_needs_paint)(app, self.id);
    }

    /// Dart's `controller as T`: the typed handle when this controller is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// Dart's `controller.markNeedsPaint` tear-off, as a listener an ink feature's animation
    /// controllers call.
    pub fn mark_needs_paint_listener(self) -> Listener {
        Listener::new(move |app| self.mark_needs_paint(app))
    }
}

/// A piece of material.
///
/// The Material widget is responsible for:
///
/// 1. Clipping: If [`clip_behavior`](Self::clip_behavior) is not [`Clip::None`], Material
///    clips its widget sub-tree to the shape specified by [`shape`](Self::shape),
///    [`type`](Self::r#type), and [`border_radius`](Self::border_radius).
///    By default, [`clip_behavior`](Self::clip_behavior) is [`Clip::None`] for performance
///    considerations. See [`Ink`](crate::Ink) for an example of how this affects clipping
///    [`Ink`](crate::Ink) widgets.
/// 2. Elevation: Material elevates its widget sub-tree on the Z axis by
///    [`elevation`](Self::elevation) pixels, and draws the appropriate shadow.
/// 3. Ink effects: Material shows ink effects implemented by [`InkFeature`]s
///    like [`InkSplash`](crate::InkSplash) and [`InkHighlight`](crate::InkHighlight) below its
///    children.
///
/// ## The Material Metaphor
///
/// Material is the central metaphor in Material Design. Each piece of material
/// exists at a given elevation, which influences how that piece of material
/// visually relates to other pieces of material and how that material casts
/// shadows.
///
/// Most user interface elements are either conceptually printed on a piece of
/// material or themselves made of material. Material reacts to user input using
/// [`InkSplash`](crate::InkSplash) and [`InkHighlight`](crate::InkHighlight) effects. To
/// trigger a reaction on the material, use a [`MaterialInkController`] obtained via
/// [`Material::of`].
///
/// In general, the features of a [`Material`] should not change over time (e.g. a
/// [`Material`] should not change its [`color`](Self::color),
/// [`shadow_color`](Self::shadow_color) or [`type`](Self::r#type)).
/// Changes to [`elevation`](Self::elevation), [`shadow_color`](Self::shadow_color) and
/// [`surface_tint_color`](Self::surface_tint_color) are animated
/// for [`animation_duration`](Self::animation_duration). Changes to [`shape`](Self::shape) are
/// animated if [`type`](Self::r#type) is not [`MaterialType::Transparency`] and
/// `ShapeBorder::lerp` between the previous and next [`shape`](Self::shape) values is
/// supported. Shape changes are also animated for
/// [`animation_duration`](Self::animation_duration).
///
/// ## Shape
///
/// The shape for material is determined by [`shape`](Self::shape), [`type`](Self::r#type),
/// and [`border_radius`](Self::border_radius).
///
///  - If [`shape`](Self::shape) is non null, it determines the shape.
///  - If [`shape`](Self::shape) is null and [`border_radius`](Self::border_radius) is non
///    null, the shape is a rounded rectangle, with corners specified by
///    [`border_radius`](Self::border_radius).
///  - If [`shape`](Self::shape) and [`border_radius`](Self::border_radius) are null,
///    [`type`](Self::r#type) determines the shape as follows:
///    - [`MaterialType::Canvas`]: the default material shape is a rectangle.
///    - [`MaterialType::Card`]: the default material shape is a rectangle with
///      rounded edges. The edge radii is specified by [`k_material_edges`].
///    - [`MaterialType::Circle`]: the default material shape is a circle.
///    - [`MaterialType::Button`]: the default material shape is a rectangle with
///      rounded edges. The edge radii is specified by [`k_material_edges`].
///    - [`MaterialType::Transparency`]: the default material shape is a rectangle.
///
/// ## Border
///
/// If [`shape`](Self::shape) is not null, then its border will also be painted (if any).
///
/// ## Layout change notifications
///
/// If the layout changes (e.g. because there's a list on the material, and it's
/// been scrolled), a [`LayoutChangedNotification`] must be dispatched at the
/// relevant subtree. This in particular means that transitions (e.g.
/// `SlideTransition`) should not be placed inside [`Material`] widgets so as to
/// move subtrees that contain [`InkResponse`](crate::InkResponse)s,
/// [`InkWell`](crate::InkWell)s, [`Ink`](crate::Ink)s, or other widgets that use the
/// [`InkFeature`] mechanism. Otherwise, in-progress ink features (e.g., ink splashes and ink
/// highlights) won't move to account for the new layout.
///
/// ## Painting over the material
///
/// Material widgets will often trigger reactions on their nearest material
/// ancestor. For example, `ListTile.hoverColor` triggers a reaction on the
/// tile's material when a pointer is hovering over it. These reactions will be
/// obscured if any widget in between them and the material paints in such a
/// way as to obscure the material (such as setting a `BoxDecoration.color` on
/// a `DecoratedBox`). To avoid this behavior, use [`InkDecoration`](crate::InkDecoration) to
/// decorate the material itself.
///
/// See also:
///
///  * `MergeableMaterial`, a piece of material that can split and re-merge.
///  * `Card`, a wrapper for a [`Material`] of [`type`](Self::r#type) [`MaterialType::Card`].
///  * <https://material.io/design/>
///  * <https://m3.material.io/styles/color/the-color-system/color-roles>
#[derive(Clone, Debug)]
pub struct Material {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// The kind of material to show (e.g., card or canvas). This
    /// affects the shape of the widget, the roundness of its corners if
    /// the shape is rectangular, and the default color.
    pub r#type: MaterialType,

    /// Whether the color should be animated.
    pub animate_color: bool,

    /// The z-coordinate at which to place this material relative to its parent.
    ///
    /// This controls the size of the shadow below the material and the opacity
    /// of the elevation overlay color if it is applied.
    ///
    /// If this is non-zero, the contents of the material are clipped, because the
    /// widget conceptually defines an independent printed piece of material.
    ///
    /// Defaults to 0. Changing this value will cause the shadow and the elevation
    /// overlay or surface tint to animate over [`Material::animation_duration`].
    ///
    /// The value is non-negative.
    ///
    /// See also:
    ///
    ///  * `ThemeData::use_material3` which defines whether a surface tint or
    ///    elevation overlay is used to indicate elevation.
    ///  * `ThemeData::apply_elevation_overlay_color` which controls the whether
    ///    an overlay color will be applied to indicate elevation.
    ///  * [`Material::color`] which may have an elevation overlay applied.
    ///  * [`Material::shadow_color`] which will be used for the color of a drop shadow.
    ///  * [`Material::surface_tint_color`] which will be used as the overlay tint to
    ///    show elevation.
    pub elevation: f64,

    /// The color to paint the material.
    ///
    /// Must be opaque. To create a transparent piece of material, use
    /// [`MaterialType::Transparency`].
    ///
    /// If `ThemeData::use_material3` is true then an optional
    /// [`surface_tint_color`](Self::surface_tint_color) overlay may be applied on top of this
    /// color to indicate elevation.
    ///
    /// If `ThemeData::use_material3` is false and `ThemeData::apply_elevation_overlay_color`
    /// is true and `ThemeData::brightness` is `Brightness::Dark` then a
    /// semi-transparent overlay color will be composited on top of this
    /// color to indicate the elevation. This is no longer needed for Material
    /// Design 3, which uses [`surface_tint_color`](Self::surface_tint_color).
    ///
    /// By default, the color is derived from the [`type`](Self::r#type) of material.
    pub color: Option<Color>,

    /// The color to paint the shadow below the material.
    ///
    /// If null and `ThemeData::use_material3` is true then `ThemeData`'s
    /// `ColorScheme::shadow` will be used. If `ThemeData::use_material3` is false
    /// then `ThemeData::shadow_color` will be used.
    ///
    /// To remove the drop shadow when [`elevation`](Self::elevation) is greater than 0, set
    /// [`shadow_color`](Self::shadow_color) to `Colors::TRANSPARENT`.
    ///
    /// See also:
    ///  * `ThemeData::use_material3`, which determines the default value for this
    ///    property if it is null.
    ///  * `ThemeData::apply_elevation_overlay_color`, which turns elevation overlay
    ///    on or off for dark themes.
    pub shadow_color: Option<Color>,

    /// The color of the surface tint overlay applied to the material color
    /// to indicate elevation.
    ///
    /// Material Design 3 introduced a new way for some components to indicate
    /// their elevation by using a surface tint color overlay on top of the
    /// base material [`color`](Self::color). This overlay is painted with an opacity that is
    /// related to the [`elevation`](Self::elevation) of the material.
    ///
    /// If `ThemeData::use_material3` is false, then this property is not used.
    ///
    /// If `ThemeData::use_material3` is true and
    /// [`surface_tint_color`](Self::surface_tint_color) is not null and not
    /// `Colors::TRANSPARENT`, then it will be used to overlay the base
    /// [`color`](Self::color) with an opacity based on the [`elevation`](Self::elevation).
    ///
    /// Otherwise, no surface tint will be applied.
    ///
    /// See also:
    ///
    ///   * `ThemeData::use_material3`, which turns this feature on.
    ///   * [`ElevationOverlay::apply_surface_tint`], which is used to implement the
    ///     tint.
    ///   * <https://m3.material.io/styles/color/the-color-system/color-roles>
    ///     which specifies how the overlay is applied.
    pub surface_tint_color: Option<Color>,

    /// The typographical style to use for text within this material.
    pub text_style: Option<TextStyle>,

    /// Defines the material's shape as well its shadow.
    ///
    /// If shape is non null, the [`border_radius`](Self::border_radius) is ignored and the
    /// material's clip boundary and shadow are defined by the shape.
    ///
    /// A shadow is only displayed if the [`elevation`](Self::elevation) is greater than
    /// zero.
    pub shape: Option<ShapeBorderRef>,

    /// Whether to paint the [`shape`](Self::shape) border in front of the
    /// [`child`](Self::child).
    ///
    /// The default value is true.
    /// If false, the border will be painted behind the [`child`](Self::child).
    pub border_on_foreground: bool,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,

    /// Defines the duration of animated changes for [`shape`](Self::shape),
    /// [`elevation`](Self::elevation), [`shadow_color`](Self::shadow_color),
    /// [`surface_tint_color`](Self::surface_tint_color) and the elevation overlay if it is
    /// applied.
    ///
    /// The default value is [`K_THEME_CHANGE_DURATION`].
    pub animation_duration: Duration,

    /// If non-null, the corners of this box are rounded by this
    /// [`BorderRadiusGeometry`] value.
    ///
    /// Otherwise, the corners specified for the current [`type`](Self::r#type) of material
    /// are used.
    ///
    /// If [`shape`](Self::shape) is non null then the border radius is ignored.
    ///
    /// Must be null if [`type`](Self::r#type) is [`MaterialType::Circle`].
    pub border_radius: Option<BorderRadiusGeometry>,
}

impl Default for Material {
    fn default() -> Material {
        Material {
            key: None,
            child: None,
            r#type: MaterialType::Canvas,
            animate_color: false,
            elevation: 0.0,
            color: None,
            shadow_color: None,
            surface_tint_color: None,
            text_style: None,
            shape: None,
            border_on_foreground: true,
            clip_behavior: Clip::None,
            animation_duration: K_THEME_CHANGE_DURATION,
            border_radius: None,
        }
    }
}

impl Material {
    /// The default radius of an ink splash in logical pixels.
    pub const DEFAULT_SPLASH_RADIUS: f64 = 35.0;

    /// Creates a piece of material; Dart's named arguments are the setters.
    ///
    /// The [`elevation`](Self::elevation) must be non-negative.
    ///
    /// If a [`shape`](Self::shape) is specified, then the
    /// [`border_radius`](Self::border_radius) property must be null and the
    /// [`type`](Self::r#type) property must not be [`MaterialType::Circle`]. If the
    /// [`border_radius`](Self::border_radius) is specified, then the [`type`](Self::r#type)
    /// property must not be [`MaterialType::Circle`]. In both cases, these restrictions are
    /// intended to catch likely errors.
    pub fn new() -> Material {
        Material::default()
    }

    /// Dart `Material(key:)`.
    pub fn key(mut self, key: KeyRef) -> Material {
        self.key = Some(key);
        self
    }

    /// Dart `Material(type:)`.
    pub fn r#type(mut self, r#type: MaterialType) -> Material {
        self.r#type = r#type;
        self.debug_assert_shape_fits_type();
        self
    }

    /// Dart `Material(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> Material {
        debug_assert!(elevation >= 0.0);
        self.elevation = elevation;
        self
    }

    /// Dart `Material(color:)`.
    pub fn color(mut self, color: Color) -> Material {
        self.color = Some(color);
        self
    }

    /// Dart `Material(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> Material {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `Material(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> Material {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `Material(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> Material {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `Material(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: impl Into<BorderRadiusGeometry>) -> Material {
        self.border_radius = Some(border_radius.into());
        self.debug_assert_shape_fits_type();
        self
    }

    /// Dart `Material(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> Material {
        self.shape = Some(shape.into());
        self.debug_assert_shape_fits_type();
        self
    }

    /// Dart `Material(borderOnForeground:)`.
    pub fn border_on_foreground(mut self, border_on_foreground: bool) -> Material {
        self.border_on_foreground = border_on_foreground;
        self
    }

    /// Dart `Material(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Material {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Material(animationDuration:)`.
    pub fn animation_duration(mut self, animation_duration: Duration) -> Material {
        self.animation_duration = animation_duration;
        self
    }

    /// Dart `Material(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Material {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `Material(animateColor:)`.
    pub fn animate_color(mut self, animate_color: bool) -> Material {
        self.animate_color = animate_color;
        self
    }

    /// Dart's constructor asserts on `shape`, `borderRadius` and `type`.
    fn debug_assert_shape_fits_type(&self) {
        debug_assert!(!(self.shape.is_some() && self.border_radius.is_some()));
        debug_assert!(
            !(self.r#type == MaterialType::Circle
                && (self.border_radius.is_some() || self.shape.is_some()))
        );
    }

    /// The ink controller from the closest instance of this class that
    /// encloses the given context within the closest [`LookupBoundary`].
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// See also:
    ///
    /// * [`Material::of`], which is similar to this method, but asserts if
    ///   no [`Material`] ancestor is found.
    pub fn maybe_of(app: &App, context: BuildContext) -> Option<AnyMaterialInkController> {
        LookupBoundary::find_ancestor_render_object_of_type::<RenderInkFeatures>(app, context)
            .map(|controller| controller.handle().as_material_ink_controller())
    }

    /// The ink controller from the closest instance of [`Material`] that encloses
    /// the given context within the closest [`LookupBoundary`].
    ///
    /// If no [`Material`] widget ancestor can be found then this method panics.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// See also:
    ///
    /// * [`Material::maybe_of`], which is similar to this method, but returns null if
    ///   no [`Material`] ancestor is found.
    pub fn of(app: &App, context: BuildContext) -> AnyMaterialInkController {
        let Some(controller) = Material::maybe_of(app, context) else {
            if cfg!(debug_assertions)
                && LookupBoundary::debug_is_hiding_ancestor_render_object_of_type::<RenderInkFeatures>(
                    app, context,
                )
            {
                panic!(
                    "Material.of() was called with a context that does not have access to a \
                     Material widget.\nThe context provided to Material.of() does have a \
                     Material widget ancestor, but it is hidden by a LookupBoundary. This can \
                     happen because you are using a widget that looks for a Material ancestor, \
                     but no such ancestor exists within the closest LookupBoundary.\nThe \
                     context used was:\n  {:?}",
                    context.widget(app)
                );
            }
            panic!(
                "Material.of() was called with a context that does not contain a Material \
                 widget.\nNo Material widget ancestor could be found starting from the context \
                 that was passed to Material.of(). This can happen because you are using a \
                 widget that looks for a Material ancestor, but no such ancestor exists.\nThe \
                 context used was:\n  {:?}",
                context.widget(app)
            );
        };
        controller
    }
}

impl StatefulWidget for Material {
    type State = MaterialWidgetState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MaterialWidgetState {
        MaterialWidgetState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            ink_feature_renderer: GlobalKey::labeled("ink renderer"),
        }
    }
}

/// Dart's `_MaterialState`; `MaterialState` is `material_state.dart`'s name for `WidgetState`.
pub struct MaterialWidgetState {
    state: StateData<Material>,
    ticker_provider: TickerProviderStateMixinData,
    ink_feature_renderer: GlobalKey,
}

impl TickerProviderStateMixin for MaterialWidgetState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for MaterialWidgetState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for MaterialWidgetState {
    type Widget = Material;
    waft_widgets::state_accessors!();

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let widget = self.widget(app).clone();
        let background_color = widget.color.or(match widget.r#type {
            MaterialType::Canvas => Some(theme.canvas_color),
            MaterialType::Card => Some(theme.card_color),
            MaterialType::Button | MaterialType::Circle | MaterialType::Transparency => None,
        });
        let model_shadow_color = widget.shadow_color.unwrap_or(if theme.use_material3 {
            theme.color_scheme.shadow()
        } else {
            theme.shadow_color
        });
        debug_assert!(
            background_color.is_some() || widget.r#type == MaterialType::Transparency,
            "If Material type is not MaterialType.transparency, a color must either be passed \
             in through the `color` property, or be defined in the theme (ex. canvasColor != \
             null if type is set to MaterialType.canvas)"
        );

        let mut contents = widget.child.clone();
        if let Some(child) = contents {
            let style = widget.text_style.clone().unwrap_or_else(|| {
                Theme::of(app, context)
                    .text_theme
                    .body_medium
                    .clone()
                    .expect("every text theme has a body medium style")
            });
            contents = Some(
                AnimatedDefaultTextStyle::new(child, style, widget.animation_duration)
                    .into_widget(),
            );
        }
        let ink_feature_renderer = app.get(self).ink_feature_renderer.clone();
        let mut ink_features = InkFeatures::new(
            ink_feature_renderer.clone(),
            Rc::new(self),
            widget.r#type != MaterialType::Transparency,
        );
        ink_features.color = background_color;
        ink_features.child = contents;
        let contents = NotificationListener::<dyn LayoutChangedNotification>::new(ink_features)
            .on_notification(move |app, _notification| {
                let renderer = ink_feature_renderer
                    .current_context(app)
                    .expect("the ink renderer is mounted")
                    .find_render_object(app)
                    .expect("the ink renderer has a render object")
                    .downcast::<RenderInkFeatures>(app)
                    .expect("the ink renderer is a RenderInkFeatures");
                renderer.did_change_layout(app);
                false
            })
            .into_widget();

        let shape: Option<ShapeBorderRef> = match widget.border_radius {
            Some(border_radius) => Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
                BorderSide::NONE,
                border_radius,
            ))),
            None => widget.shape.clone(),
        };

        // PhysicalModel has a temporary workaround for a performance issue that
        // speeds up rectangular non transparent material (the workaround is to
        // skip the call to ui.Canvas.saveLayer if the border radius is 0).
        // Until the saveLayer performance issue is resolved, we're keeping this
        // special case here for canvas material type that is using the default
        // shape (rectangle). We could go down this fast path for explicitly
        // specified rectangles (e.g shape RoundedRectangleBorder with radius 0, but
        // we choose not to as we want the change from the fast-path to the
        // slow-path to be noticeable in the construction site of Material.
        if widget.r#type == MaterialType::Canvas && shape.is_none() {
            let background_color = background_color.expect("asserted above");
            let color = if theme.use_material3 {
                ElevationOverlay::apply_surface_tint(
                    background_color,
                    widget.surface_tint_color,
                    widget.elevation,
                )
            } else {
                ElevationOverlay::apply_overlay(app, context, background_color, widget.elevation)
            };

            return AnimatedPhysicalModel::new(
                contents,
                color,
                model_shadow_color,
                widget.animation_duration,
            )
            .curve(Curves::fast_out_slow_in())
            .clip_behavior(widget.clip_behavior)
            .elevation(widget.elevation)
            .animate_color(widget.animate_color)
            .into_widget();
        }

        let shape = shape.unwrap_or_else(|| match widget.r#type {
            MaterialType::Circle => ShapeBorderRef::new(CircleBorder::default()),
            MaterialType::Canvas | MaterialType::Transparency => {
                ShapeBorderRef::new(RoundedRectangleBorder::default())
            }
            MaterialType::Card | MaterialType::Button => {
                ShapeBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadius::all(Radius::circular(2.0)).into(),
                ))
            }
        });

        if widget.r#type == MaterialType::Transparency {
            let text_direction = Directionality::maybe_of(app, context);
            return ClipPath::new()
                .clipper(Rc::new(ShapeBorderClipper::new(
                    shape.clone(),
                    text_direction,
                )))
                .clip_behavior(widget.clip_behavior)
                .child(ShapeBorderPaint::new(contents, shape))
                .into_widget();
        }

        let mut interior = MaterialInterior::new(
            contents,
            shape,
            widget.elevation,
            background_color.expect("asserted above"),
            model_shadow_color,
            widget.surface_tint_color,
            widget.animation_duration,
        )
        .curve(Curves::fast_out_slow_in());
        interior.border_on_foreground = widget.border_on_foreground;
        interior.clip_behavior = widget.clip_behavior;
        interior.into_widget()
    }
}

/// Dart's `_RenderInkFeatures`: the proxy box under a [`Material`] that paints its ink
/// features below its child, and is the [`MaterialInkController`] that
/// [`Material::of`] finds.
pub(crate) struct RenderInkFeatures {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    // This class should exist in a 1:1 relationship with a WidgetState object,
    // since there's no current support for dynamically changing the ticker
    // provider.
    vsync: Rc<dyn TickerProvider>,
    // This is here to satisfy the MaterialInkController contract.
    // The actual painting of this color is done by a Container in the
    // WidgetState build method.
    color: Option<Color>,
    absorb_hit_test: bool,
    ink_features: Option<Vec<InkFeatureRef>>,
}

impl RenderInkFeatures {
    fn new(
        app: &mut App,
        vsync: Rc<dyn TickerProvider>,
        absorb_hit_test: bool,
        color: Option<Color>,
    ) -> RenderHandle<RenderInkFeatures> {
        RenderHandle::new_box(
            app,
            RenderInkFeatures {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                vsync,
                color,
                absorb_hit_test,
                ink_features: None,
            },
        )
    }

    /// The ink features this material paints, in the order they were added.
    #[cfg(test)]
    pub(crate) fn debug_ink_features(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<Vec<AnyInkFeature>> {
        if cfg!(debug_assertions) {
            return self.get(app).ink_features.as_ref().map(|ink_features| {
                ink_features
                    .iter()
                    .map(|ink_feature| ink_feature.handle())
                    .collect()
            });
        }
        None
    }

    /// The ink features' part of Dart's `paint`, below the child.
    pub(crate) fn paint_ink_features(
        self: RenderHandle<Self>,
        app: &mut App,
        canvas: &mut Canvas,
        offset: Offset,
    ) {
        let ink_features = self.get(app).ink_features.clone();
        if let Some(ink_features) = &ink_features
            && !ink_features.is_empty()
        {
            let size = self.size(app);
            canvas.save();
            canvas.translate(offset.dx() as f32, offset.dy() as f32);
            canvas.clip_rect(Offset::ZERO & size, ClipOp::Intersect, true);
            for ink_feature in ink_features {
                ink_feature.paint(app, canvas);
            }
            canvas.restore();
        }
        debug_assert!(ink_features == self.get(app).ink_features);
    }

    fn remove_feature(self: RenderHandle<Self>, app: &mut App, feature: AnyInkFeature) {
        let ink_features = self
            .get_mut(app)
            .ink_features
            .as_mut()
            .expect("a feature was added");
        if let Some(index) = ink_features.iter().position(|added| *added == feature) {
            ink_features.remove(index);
        }
        self.mark_needs_paint(app);
    }

    fn did_change_layout(self: RenderHandle<Self>, app: &mut App) {
        if self
            .get(app)
            .ink_features
            .as_ref()
            .is_some_and(|ink_features| !ink_features.is_empty())
        {
            self.mark_needs_paint(app);
        }
    }
}

impl MaterialInkController for RenderInkFeatures {
    fn color(self: Handle<Self>, app: &App) -> Option<Color> {
        app.get(self).color
    }

    fn vsync(self: Handle<Self>, app: &App) -> Rc<dyn TickerProvider> {
        Rc::clone(&app.get(self).vsync)
    }

    fn add_ink_feature(self: Handle<Self>, app: &mut App, feature: InkFeatureRef) {
        debug_assert!(!feature.debug_disposed(app));
        debug_assert!(feature.controller(app).id() == self.id());
        let ink_features = app.get_mut(self).ink_features.get_or_insert_with(Vec::new);
        debug_assert!(!ink_features.contains(&feature));
        ink_features.push(feature);
        RenderHandle::from_handle(self).mark_needs_paint(app);
    }

    fn mark_needs_paint(self: Handle<Self>, app: &mut App) {
        RenderHandle::from_handle(self).mark_needs_paint(app);
    }
}

impl RenderObjectWithChildMixin for RenderInkFeatures {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderProxyBoxMixin for RenderInkFeatures {}

impl RenderObject for RenderInkFeatures {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        self.paint_ink_features(app, context.canvas(), offset);
        RenderProxyBoxMixin::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderInkFeatures {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, _position: Offset) -> bool {
        self.get(app).absorb_hit_test
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Dart's `_InkFeatures`.
struct InkFeatures {
    key: KeyRef,

    // This widget must be owned by a WidgetState, which must be provided as the vsync.
    // This relationship must be 1:1 and cannot change for the lifetime of the WidgetState.
    color: Option<Color>,

    vsync: Rc<dyn TickerProvider>,

    absorb_hit_test: bool,

    child: Option<WidgetRef>,
}

impl InkFeatures {
    fn new(key: GlobalKey, vsync: Rc<dyn TickerProvider>, absorb_hit_test: bool) -> InkFeatures {
        InkFeatures {
            key: key.into(),
            color: None,
            vsync,
            absorb_hit_test,
            child: None,
        }
    }
}

impl fmt::Debug for InkFeatures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InkFeatures")
            .field("color", &self.color)
            .field("absorb_hit_test", &self.absorb_hit_test)
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for InkFeatures {
    type RenderObject = RenderInkFeatures;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderInkFeatures::new(
            app,
            Rc::clone(&self.vsync),
            self.absorb_hit_test,
            self.color,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderInkFeatures>,
    ) {
        let render = render_object.get_mut(app);
        render.color = self.color;
        render.absorb_hit_test = self.absorb_hit_test;
    }
}

impl SingleChildRenderObjectWidget for InkFeatures {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// The fields of Dart's `InkFeature`, which every ink feature leaf carries.
pub struct InkFeatureData {
    controller: Handle<RenderInkFeatures>,
    reference_box: AnyRenderBox,
    on_removed: Option<Listener>,
    debug_disposed: bool,
}

impl InkFeatureData {
    /// Initializes fields for subclasses.
    ///
    /// # Panics
    ///
    /// If `controller` is not a [`Material`]'s, as Dart's `controller as _RenderInkFeatures`.
    pub fn new(
        app: &App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        on_removed: Option<Listener>,
    ) -> InkFeatureData {
        InkFeatureData {
            controller: controller
                .downcast::<RenderInkFeatures>(app)
                .expect("an ink feature's controller is a Material's"),
            reference_box,
            on_removed,
            debug_disposed: false,
        }
    }
}

/// The [`InkFeatureData`] bag of every ink feature leaf.
pub trait InkFeatureLeafData {
    /// The leaf's [`InkFeature`] bag.
    fn ink_feature(&self) -> &InkFeatureData;

    /// The leaf's [`InkFeature`] bag, mutably.
    fn ink_feature_mut(&mut self) -> &mut InkFeatureData;
}

/// The virtuals of Dart's abstract `InkFeature`, which a leaf implements.
pub trait InkFeatureLeaf: InkFeatureLeafData + Sized + 'static {
    /// Free up the resources associated with this ink feature.
    ///
    /// Dart's `@mustCallSuper`: an override ends with [`InkFeature::dispose`].
    fn dispose(self: Handle<Self>, app: &mut App) {
        InkFeature::dispose(self, app);
    }

    /// Override this method to paint the ink feature.
    ///
    /// The transform argument gives the coordinate conversion from the coordinate
    /// system of the canvas to the coordinate system of the
    /// [`reference_box`](InkFeature::reference_box).
    fn paint_feature(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, transform: &Matrix4);
}

/// Mints the erased [`AnyInkFeature`] of a leaf.
pub trait InkFeatureObject: Sized + 'static {
    /// This feature as the erased [`AnyInkFeature`] — what to pass where a Dart API takes an
    /// `InkFeature`.
    fn as_ink_feature(self: Handle<Self>) -> AnyInkFeature;
}

/// What a field Dart types as `InkFeature` holds: the material's list, an ink well's state.
pub type InkFeatureRef = Retained<AnyInkFeature>;

waft_foundation::erased_handle!(AnyInkFeature: InkFeatureLeaf => InkFeatureObject::as_ink_feature);

impl<F: InkFeatureLeaf> InkFeatureObject for F {
    fn as_ink_feature(self: Handle<Self>) -> AnyInkFeature {
        AnyInkFeature {
            id: self.id(),
            vtable: const { &InkFeatureVTable::of::<F>() },
        }
    }
}

/// The vtable of an erased [`AnyInkFeature`]: one `&'static` table per leaf type.
struct InkFeatureVTable {
    type_name: fn() -> &'static str,
    paint: fn(&mut App, HandleId, &mut Canvas),
    dispose: fn(&mut App, HandleId),
    controller: fn(&App, HandleId) -> AnyMaterialInkController,
    reference_box: fn(&App, HandleId) -> AnyRenderBox,
    debug_disposed: fn(&App, HandleId) -> bool,
}

impl InkFeatureVTable {
    const fn of<F: InkFeatureLeaf>() -> InkFeatureVTable {
        InkFeatureVTable {
            type_name: std::any::type_name::<F>,
            paint: |app, id, canvas| InkFeature::paint(resolve::<F>(id), app, canvas),
            dispose: |app, id| F::dispose(resolve(id), app),
            controller: |app, id| InkFeature::controller(resolve::<F>(id), app),
            reference_box: |app, id| InkFeature::reference_box(resolve::<F>(id), app),
            debug_disposed: |app, id| app.get(resolve::<F>(id)).ink_feature().debug_disposed,
        }
    }
}

/// Erased `InkFeature`: one identity and a static vtable.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyInkFeature {
    id: HandleId,
    vtable: &'static InkFeatureVTable,
}

impl fmt::Debug for AnyInkFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyInkFeature {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `feature as T`: the typed handle when this feature is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// See [`InkFeature::controller`].
    pub fn controller(self, app: &App) -> AnyMaterialInkController {
        (self.vtable.controller)(app, self.id)
    }

    /// See [`InkFeature::reference_box`].
    pub fn reference_box(self, app: &App) -> AnyRenderBox {
        (self.vtable.reference_box)(app, self.id)
    }

    /// See [`InkFeatureLeaf::dispose`].
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }

    /// Dart's private `_paint`.
    fn paint(self, app: &mut App, canvas: &mut Canvas) {
        (self.vtable.paint)(app, self.id, canvas);
    }

    /// Dart's private `_debugDisposed`.
    fn debug_disposed(self, app: &App) -> bool {
        (self.vtable.debug_disposed)(app, self.id)
    }
}

/// Dart's abstract `InkFeature`: a visual reaction on a piece of [`Material`].
///
/// To add an ink feature to a piece of [`Material`], obtain the
/// [`MaterialInkController`] via [`Material::of`] and call
/// [`MaterialInkController::add_ink_feature`].
///
/// Its bodies are associated fns taking the leaf's `Handle`; a leaf's override calls one
/// where Dart writes `super`. Its fields are [`InkFeatureData`].
pub struct InkFeature;

impl InkFeature {
    /// The [`MaterialInkController`] associated with this [`InkFeature`].
    ///
    /// Typically used by subclasses to call
    /// [`MaterialInkController::mark_needs_paint`] when they need to repaint.
    pub fn controller<F: InkFeatureLeaf>(this: Handle<F>, app: &App) -> AnyMaterialInkController {
        app.get(this)
            .ink_feature()
            .controller
            .as_material_ink_controller()
    }

    /// The render box whose visual position defines the frame of reference for this ink
    /// feature.
    pub fn reference_box<F: InkFeatureLeaf>(this: Handle<F>, app: &App) -> AnyRenderBox {
        app.get(this).ink_feature().reference_box
    }

    /// Called when the ink feature is no longer visible on the material.
    pub fn on_removed<F: InkFeatureLeaf>(this: Handle<F>, app: &App) -> Option<Listener> {
        app.get(this).ink_feature().on_removed.clone()
    }

    /// Dart's `InkFeature.dispose`: frees up the resources associated with this ink feature.
    /// The feature itself goes once the controller's list and its owner let go of it.
    pub fn dispose<F: InkFeatureLeaf>(this: Handle<F>, app: &mut App) {
        debug_assert!(!app.get(this).ink_feature().debug_disposed);
        if cfg!(debug_assertions) {
            app.get_mut(this).ink_feature_mut().debug_disposed = true;
        }
        let controller = RenderHandle::from_handle(app.get(this).ink_feature().controller);
        controller.remove_feature(app, this.as_ink_feature());
        if let Some(on_removed) = InkFeature::on_removed(this, app) {
            on_removed.call(app);
        }
    }

    // Returns the paint transform that allows `fromRenderObject` to perform paint
    // in `toRenderObject`'s coordinate space.
    //
    // Returns null if either `fromRenderObject` or `toRenderObject` is not in the
    // same render tree, or either of them is in an offscreen subtree (see
    // RenderObject.paintsChild).
    fn get_paint_transform(
        app: &App,
        from_render_object: AnyRenderObject,
        to_render_object: AnyRenderObject,
    ) -> Option<Matrix4> {
        // The paths to fromRenderObject and toRenderObject's common ancestor.
        let mut from_path = vec![from_render_object];
        let mut to_path = vec![to_render_object];

        let mut from = from_render_object;
        let mut to = to_render_object;

        while from != to {
            let from_depth = from.depth(app);
            let to_depth = to.depth(app);

            if from_depth >= to_depth {
                let from_parent = from.parent(app);
                // Return early if the 2 render objects are not in the same render tree,
                // or either of them is offscreen and thus won't get painted.
                let from_parent = from_parent.filter(|parent| parent.paints_child(app, from))?;
                from_path.push(from_parent);
                from = from_parent;
            }

            if from_depth <= to_depth {
                let to_parent = to.parent(app);
                let to_parent = to_parent.filter(|parent| parent.paints_child(app, to))?;
                to_path.push(to_parent);
                to = to_parent;
            }
        }
        debug_assert!(from == to);

        let mut transform = Matrix4::IDENTITY;
        let mut inverse_transform = Matrix4::IDENTITY;

        for index in (1..to_path.len()).rev() {
            to_path[index].apply_paint_transform(app, to_path[index - 1], &mut transform);
        }
        for index in (1..from_path.len()).rev() {
            from_path[index].apply_paint_transform(
                app,
                from_path[index - 1],
                &mut inverse_transform,
            );
        }

        inverse_transform
            .invert()
            .map(|inverse_transform| inverse_transform.then(&transform))
    }

    /// Dart's private `_paint`.
    fn paint<F: InkFeatureLeaf>(this: Handle<F>, app: &mut App, canvas: &mut Canvas) {
        let reference_box = InkFeature::reference_box(this, app);
        debug_assert!(reference_box.as_object().attached(app));
        debug_assert!(!app.get(this).ink_feature().debug_disposed);
        // determine the transform that gets our coordinate system to be like theirs
        let controller = RenderHandle::from_handle(app.get(this).ink_feature().controller);
        let transform =
            InkFeature::get_paint_transform(app, controller.as_object(), reference_box.as_object());
        if let Some(transform) = transform {
            this.paint_feature(app, canvas, &transform);
        }
    }
}

/// An interpolation between two [`ShapeBorder`]s.
///
/// This class specializes the interpolation of `Tween` to use `ShapeBorder::lerp`.
///
/// A value rather than an arena object, as waft-widgets' `DecorationTween`: a driven
/// animation holds a clone.
#[derive(Clone, Debug)]
pub struct ShapeBorderTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<ShapeBorderRef>,

    /// The value this variable has at the end of the animation.
    pub end: Option<ShapeBorderRef>,
}

impl ShapeBorderTween {
    /// Creates a [`ShapeBorder`] tween.
    ///
    /// the [`begin`](Self::begin) and [`end`](Self::end) properties may be null; see
    /// `ShapeBorder::lerp` for the null handling semantics.
    pub fn new(begin: Option<ShapeBorderRef>, end: Option<ShapeBorderRef>) -> ShapeBorderTween {
        ShapeBorderTween { begin, end }
    }

    /// Returns the value this tween has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Option<ShapeBorderRef> {
        ShapeBorderRef::lerp(self.begin.as_ref(), self.end.as_ref(), t)
    }
}

/// Dart's `Tween.transform`, which hands back the nullable endpoints themselves at 0 and 1.
impl Animatable<Option<ShapeBorderRef>> for ShapeBorderTween {
    fn transform(&self, _app: &App, t: f64) -> Option<ShapeBorderRef> {
        if t == 0.0 {
            return self.begin.clone();
        }
        if t == 1.0 {
            return self.end.clone();
        }
        self.lerp(t)
    }
}

/// Dart's `Tween<ShapeBorder?>`: an unset endpoint and a null one are the same.
impl TweenSlot for ShapeBorderTween {
    type Value = Option<ShapeBorderRef>;

    fn begin(&self, _app: &App) -> Option<Option<ShapeBorderRef>> {
        self.begin.clone().map(Some)
    }

    fn set_begin(&mut self, _app: &mut App, begin: Option<Option<ShapeBorderRef>>) {
        self.begin = begin.flatten();
    }

    fn end(&self, _app: &App) -> Option<Option<ShapeBorderRef>> {
        self.end.clone().map(Some)
    }

    fn set_end(&mut self, _app: &mut App, end: Option<Option<ShapeBorderRef>>) {
        self.end = end.flatten();
    }
}

/// The interior of non-transparent material.
///
/// Animates [`elevation`](Self::elevation), [`shadow_color`](Self::shadow_color), and
/// [`shape`](Self::shape).
///
/// Dart's `_MaterialInterior`.
#[derive(Debug)]
struct MaterialInterior {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    child: WidgetRef,

    /// The border of the widget.
    ///
    /// This border will be painted, and in addition the outer path of the border
    /// determines the physical shape.
    shape: ShapeBorderRef,

    /// Whether to paint the border in front of the child.
    ///
    /// The default value is true.
    /// If false, the border will be painted behind the child.
    border_on_foreground: bool,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::None`].
    clip_behavior: Clip,

    /// The target z-coordinate at which to place this physical object relative
    /// to its parent.
    ///
    /// The value is non-negative.
    elevation: f64,

    /// The target background color.
    color: Color,

    /// The target shadow color.
    shadow_color: Color,

    /// The target surface tint color.
    surface_tint_color: Option<Color>,
}

impl MaterialInterior {
    /// Creates a const instance of [`MaterialInterior`].
    ///
    /// The [`elevation`](Self::elevation) must be specified and greater than or equal to zero.
    fn new(
        child: WidgetRef,
        shape: ShapeBorderRef,
        elevation: f64,
        color: Color,
        shadow_color: Color,
        surface_tint_color: Option<Color>,
        duration: Duration,
    ) -> MaterialInterior {
        debug_assert!(elevation >= 0.0);
        MaterialInterior {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child,
            shape,
            border_on_foreground: true,
            clip_behavior: Clip::None,
            elevation,
            color,
            shadow_color,
            surface_tint_color,
        }
    }

    fn curve(mut self, curve: Rc<dyn waft_animation::Curve>) -> MaterialInterior {
        self.implicitly_animated_widget.curve = curve;
        self
    }
}

impl ImplicitlyAnimatedWidget for MaterialInterior {
    waft_widgets::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for MaterialInterior {
    type State = MaterialInteriorState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> MaterialInteriorState {
        MaterialInteriorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            elevation: None,
            surface_tint_color: None,
            shadow_color: None,
            border: None,
        }
    }
}

/// Dart's `_MaterialInteriorState`.
struct MaterialInteriorState {
    state: StateData<MaterialInterior>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    elevation: Option<Ref<Tween<f64>>>,
    surface_tint_color: Option<Ref<ColorTween>>,
    shadow_color: Option<Ref<ColorTween>>,
    border: Option<ShapeBorderTween>,
}

impl SingleTickerProviderStateMixin for MaterialInteriorState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for MaterialInteriorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for MaterialInteriorState {
    waft_widgets::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let tween = app.get(self).elevation.clone();
        let target = Some(self.widget(app).elevation);
        app.get_mut(self).elevation = visitor.visit(app, tween, target, |app, value| {
            Tween::new(app, Some(*value), None)
        });
        let tween = app.get(self).shadow_color.clone();
        let target = Some(Some(self.widget(app).shadow_color));
        app.get_mut(self).shadow_color = visitor.visit(app, tween, target, |app, value| {
            ColorTween::new(app, *value, None)
        });
        let surface_tint_color = self.widget(app).surface_tint_color;
        app.get_mut(self).surface_tint_color = if surface_tint_color.is_some() {
            let tween = app.get(self).surface_tint_color.clone();
            visitor.visit(app, tween, Some(surface_tint_color), |app, value| {
                ColorTween::new(app, *value, None)
            })
        } else {
            None
        };
        let tween = app.get(self).border.clone();
        let target = Some(Some(self.widget(app).shape.clone()));
        app.get_mut(self).border = visitor.visit(app, tween, target, |_app, value| {
            ShapeBorderTween::new(value.clone(), None)
        });
    }
}

impl AnimatedWidgetBaseState for MaterialInteriorState {}

impl State for MaterialInteriorState {
    type Widget = MaterialInterior;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MaterialInterior) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let state = app.get(self);
        let border = state.border.clone().expect("for_each_tween made the tween");
        let elevation_tween = state
            .elevation
            .clone()
            .expect("for_each_tween made the tween");
        let surface_tint_color = state.surface_tint_color.clone();
        let shadow_color_tween = state
            .shadow_color
            .clone()
            .expect("for_each_tween made the tween");
        let shape = border
            .evaluate(app, animation)
            .expect("the shape tween has a shape");
        let elevation = elevation_tween.evaluate(app, animation);
        let widget_color = self.widget(app).color;
        let color = if Theme::of(app, context).use_material3 {
            let surface_tint_color =
                surface_tint_color.and_then(|tween| tween.evaluate(app, animation));
            ElevationOverlay::apply_surface_tint(widget_color, surface_tint_color, elevation)
        } else {
            ElevationOverlay::apply_overlay(app, context, widget_color, elevation)
        };
        let shadow_color = shadow_color_tween
            .evaluate(app, animation)
            .expect("the shadow color tween has a color");

        let widget = self.widget(app);
        let clip_behavior = widget.clip_behavior;
        let border_on_foreground = widget.border_on_foreground;
        let child = widget.child.clone();
        let text_direction = Directionality::maybe_of(app, context);
        let clipper: Rc<dyn CustomClipper<Path>> =
            Rc::new(ShapeBorderClipper::new(shape.clone(), text_direction));
        PhysicalShape::new(clipper, color)
            .clip_behavior(clip_behavior)
            .elevation(elevation)
            .shadow_color(shadow_color)
            .child(ShapeBorderPaint::new(child, shape).border_on_foreground(border_on_foreground))
            .into_widget()
    }
}

/// Dart's `_ShapeBorderPaint`.
#[derive(Debug)]
struct ShapeBorderPaint {
    child: WidgetRef,
    shape: ShapeBorderRef,
    border_on_foreground: bool,
}

impl ShapeBorderPaint {
    fn new(child: WidgetRef, shape: ShapeBorderRef) -> ShapeBorderPaint {
        ShapeBorderPaint {
            child,
            shape,
            border_on_foreground: true,
        }
    }

    fn border_on_foreground(mut self, border_on_foreground: bool) -> ShapeBorderPaint {
        self.border_on_foreground = border_on_foreground;
        self
    }
}

impl StatelessWidget for ShapeBorderPaint {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let painter =
            ShapeBorderPainter::new(self.shape.clone(), Directionality::maybe_of(app, context));
        let paint = CustomPaint::new().child(self.child.clone());
        if self.border_on_foreground {
            paint.foreground_painter(painter)
        } else {
            paint.painter(painter)
        }
        .into_widget()
    }
}

/// Dart's `_ShapeBorderPainter`.
struct ShapeBorderPainter {
    border: ShapeBorderRef,
    text_direction: Option<TextDirection>,
}

impl ShapeBorderPainter {
    fn new(border: ShapeBorderRef, text_direction: Option<TextDirection>) -> ShapeBorderPainter {
        ShapeBorderPainter {
            border,
            text_direction,
        }
    }
}

impl CustomPainter for ShapeBorderPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        self.border
            .paint(canvas, Offset::ZERO & size, self.text_direction);
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        let old_delegate = old_delegate
            .as_any()
            .downcast_ref::<ShapeBorderPainter>()
            .expect("compared with a painter of its own type");
        old_delegate.border != self.border
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use waft_painting::ShapeBorder;

    use waft_embedder::{Paint, TargetPlatform, TextDirection, ViewId, ViewRef};
    use waft_foundation::AppCell;
    use waft_painting::StadiumBorder;
    use waft_rendering::{
        RenderClipPath, RenderCustomClip, RenderCustomPaint, RenderPhysicalModel,
        RenderPhysicalModelBase, RenderPhysicalShape,
    };
    use waft_test::{TestPlatform, TestView};
    use waft_widgets::{
        Builder, Center, Column, Container, Expanded, ListView, Notification, Offstage,
        PlainLayoutChangedNotification, SizedBox, Stack, StatefulElement, View, ViewAnchor,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::ink_well::InkWell;
    use crate::test_support::{WidgetTester, build, test_view, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn tester() -> WidgetTester {
        WidgetTester::on(TargetPlatform::Android)
    }

    /// Dart's `buildMaterial`.
    fn build_material(
        elevation: f64,
        shadow_color: Color,
        surface_tint_color: Option<Color>,
        color: Color,
    ) -> Material {
        let mut material = Material::new()
            .color(color)
            .shadow_color(shadow_color)
            .elevation(elevation)
            .shape(CircleBorder::default());
        material.surface_tint_color = surface_tint_color;
        material
    }

    fn centered(material: Material) -> WidgetRef {
        Center::new()
            .child(SizedBox::square(Some(100.0)).child(material))
            .into_widget()
    }

    const GREEN: Color = Color::new(0xFF00FF00);
    const BLUE: Color = Color::new(0xFF0000FF);

    fn default_material(elevation: f64) -> WidgetRef {
        centered(build_material(elevation, GREEN, None, BLUE))
    }

    fn themed(
        tester: &WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        child: WidgetRef,
    ) -> WidgetRef {
        let data = theme(&tester.cell.borrow());
        Theme::new(data, child).into_widget()
    }

    fn model(tester: &WidgetTester) -> RenderHandle<RenderPhysicalShape> {
        tester.first::<RenderPhysicalShape>()
    }

    fn shape_of(tester: &WidgetTester) -> ShapeBorderRef {
        let model = model(tester);
        let app = tester.cell.borrow();
        let clipper = RenderCustomClip::clipper(model, &app).expect("a shape clipper");
        clipper
            .as_any()
            .downcast_ref::<ShapeBorderClipper>()
            .expect("a ShapeBorderClipper")
            .shape
            .clone()
    }

    // Regression test for https://github.com/flutter/flutter/issues/81504
    #[test]
    fn shadow_color_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_with_shadow = |shadow_color: Option<Color>| {
                let mut material = Material::new()
                    .elevation(10.0)
                    .shape(CircleBorder::default());
                material.shadow_color = shadow_color;
                centered(material)
            };

            // Default M2 shadow color
            let widget = themed(
                &tester,
                |app| ThemeData::builder().use_material3(false).build(app),
                build_with_shadow(None),
            );
            tester.pump_widget(widget);
            tester.pump_and_settle();
            let expected = ThemeData::new(&tester.cell.borrow()).shadow_color;
            assert_eq!(model(&tester).shadow_color(&tester.cell.borrow()), expected);

            // Default M3 shadow color
            let widget = themed(&tester, ThemeData::new, build_with_shadow(None));
            tester.pump_widget(widget);
            tester.pump_and_settle();
            let expected = ThemeData::new(&tester.cell.borrow()).color_scheme.shadow();
            assert_eq!(model(&tester).shadow_color(&tester.cell.borrow()), expected);

            // Drop shadow can be turned off with a transparent color.
            let widget = themed(
                &tester,
                ThemeData::new,
                build_with_shadow(Some(Colors::TRANSPARENT)),
            );
            tester.pump_widget(widget);
            tester.pump_and_settle();
            assert_eq!(
                model(&tester).shadow_color(&tester.cell.borrow()),
                Colors::TRANSPARENT
            );
        });
    }

    #[test]
    fn shadows_animate_smoothly() {
        // This code verifies that the PhysicalModel's elevation animates over
        // a kThemeChangeDuration time interval.
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(default_material(0.0));
            assert_eq!(model(&tester).elevation(&tester.cell.borrow()), 0.0);

            tester.pump_widget(default_material(9.0));
            assert_eq!(model(&tester).elevation(&tester.cell.borrow()), 0.0);

            tester.pump(Duration::from_millis(1));
            assert!(model(&tester).elevation(&tester.cell.borrow()).abs() < 0.001);

            tester.pump(K_THEME_CHANGE_DURATION / 2);
            assert!(model(&tester).elevation(&tester.cell.borrow()).abs() >= 0.001);

            tester.pump(K_THEME_CHANGE_DURATION);
            assert_eq!(model(&tester).elevation(&tester.cell.borrow()), 9.0);
        });
    }

    /// A shape with Dart's default `==`, identity, as a user's own border has.
    #[derive(Clone, Copy, Debug)]
    struct IdentityShape;

    impl ShapeBorder for IdentityShape {
        fn dimensions(&self) -> waft_painting::EdgeInsetsGeometry {
            waft_painting::EdgeInsetsGeometry::ZERO
        }

        fn scale(&self, _t: f64) -> ShapeBorderRef {
            ShapeBorderRef::new(*self)
        }

        fn get_outer_path(
            &self,
            rect: waft_embedder::Rect,
            _text_direction: Option<TextDirection>,
        ) -> waft_embedder::Path {
            let mut path = waft_embedder::PathBuilder::new();
            path.rect(rect.into());
            waft_embedder::Path::new(path.build())
        }

        fn get_inner_path(
            &self,
            rect: waft_embedder::Rect,
            text_direction: Option<TextDirection>,
        ) -> waft_embedder::Path {
            self.get_outer_path(rect, text_direction)
        }

        fn paint(
            &self,
            _canvas: &mut waft_embedder::Canvas,
            _rect: waft_embedder::Rect,
            _text_direction: Option<TextDirection>,
        ) {
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // ---- not in material_test.dart: a shape compared by identity ----

    #[test]
    fn a_material_given_its_own_shape_again_does_not_animate_its_shape() {
        with_main_thread_stack(|| {
            let shaped = |shape: &ShapeBorderRef| {
                centered(Material::new().color(GREEN).shape(shape.clone()))
            };
            let transient_callbacks = |tester: &WidgetTester| {
                waft_scheduler::SchedulerBinding::transient_callback_count(
                    &mut tester.cell.borrow_mut(),
                )
            };
            let mut tester = tester();
            let shape = ShapeBorderRef::new(IdentityShape);
            tester.pump_widget(shaped(&shape));
            tester.pump(K_THEME_CHANGE_DURATION);

            tester.pump_widget(shaped(&shape));
            assert_eq!(
                transient_callbacks(&tester),
                0,
                "the same shape is no new target"
            );
            assert_eq!(shape_of(&tester), shape);

            tester.pump_widget(shaped(&ShapeBorderRef::new(IdentityShape)));
            assert_eq!(
                transient_callbacks(&tester),
                1,
                "another shape of the same type is, as Dart's identity `==` makes it"
            );
        });
    }

    /// Dart's `isSameColorAs` matcher, with its default threshold.
    fn is_same_color_as(a: Color, b: Color) -> bool {
        const THRESHOLD: f64 = 0.004;
        (a.a - b.a).abs() <= THRESHOLD
            && (a.r - b.r).abs() <= THRESHOLD
            && (a.g - b.g).abs() <= THRESHOLD
            && (a.b - b.b).abs() <= THRESHOLD
    }

    fn within_one(a: Color, b: Color) -> bool {
        let channels = |color: Color| [color.a, color.r, color.g, color.b].map(|c| c * 255.0);
        channels(a)
            .iter()
            .zip(channels(b))
            .all(|(a, b)| (a - b).abs() <= 1.0)
    }

    #[test]
    fn shadow_colors_animate_smoothly() {
        // This code verifies that the PhysicalModel's shadowColor animates over
        // a kThemeChangeDuration time interval.
        with_main_thread_stack(|| {
            let mut tester = tester();
            let red = Color::new(0xFFFF0000);
            tester.pump_widget(default_material(0.0));
            assert_eq!(model(&tester).shadow_color(&tester.cell.borrow()), GREEN);

            tester.pump_widget(centered(build_material(0.0, red, None, BLUE)));
            assert_eq!(model(&tester).shadow_color(&tester.cell.borrow()), GREEN);

            tester.pump(Duration::from_millis(1));
            assert!(within_one(
                model(&tester).shadow_color(&tester.cell.borrow()),
                GREEN
            ));

            tester.pump(K_THEME_CHANGE_DURATION / 2);
            assert!(!within_one(
                model(&tester).shadow_color(&tester.cell.borrow()),
                GREEN
            ));

            tester.pump(K_THEME_CHANGE_DURATION);
            assert_eq!(model(&tester).shadow_color(&tester.cell.borrow()), red);
        });
    }

    #[test]
    fn apply_elevation_overlay_color_does_not_effect_anything_with_use_material3_set_to_true() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let surface_color = Color::new(0xFF121212);
            let widget = themed(
                &tester,
                |app| {
                    ThemeData::builder()
                        .apply_elevation_overlay_color(true)
                        .color_scheme(ColorScheme::dark().with_surface(surface_color))
                        .build(app)
                },
                centered(build_material(8.0, GREEN, None, surface_color)),
            );
            tester.pump_widget(widget);
            assert_eq!(model(&tester).color(&tester.cell.borrow()), surface_color);
        });
    }

    #[test]
    fn surface_tint_color_is_used_to_as_an_overlay_to_indicate_elevation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let base_color = Color::new(0xFF121212);
            let surface_tint_color = Color::new(0xff44ccff);

            // With no surfaceTintColor specified, it should not apply an overlay
            let widget = themed(
                &tester,
                ThemeData::new,
                centered(build_material(12.0, GREEN, None, base_color)),
            );
            tester.pump_widget(widget);
            tester.pump_and_settle();
            assert_eq!(model(&tester).color(&tester.cell.borrow()), base_color);

            // With transparent surfaceTintColor, it should not apply an overlay
            let widget = themed(
                &tester,
                ThemeData::new,
                centered(build_material(
                    12.0,
                    GREEN,
                    Some(Colors::TRANSPARENT),
                    base_color,
                )),
            );
            tester.pump_widget(widget);
            tester.pump_and_settle();
            assert_eq!(model(&tester).color(&tester.cell.borrow()), base_color);

            // With surfaceTintColor specified, it should not apply an overlay based
            // on the elevation.
            let widget = themed(
                &tester,
                ThemeData::new,
                centered(build_material(
                    12.0,
                    GREEN,
                    Some(surface_tint_color),
                    base_color,
                )),
            );
            tester.pump_widget(widget);
            tester.pump_and_settle();

            // Final color should be the base with a tint of 0.14 opacity or 0xff192c33
            assert!(is_same_color_as(
                model(&tester).color(&tester.cell.borrow()),
                Color::new(0xff192c33)
            ));
        });
    }

    #[test]
    fn apply_elevation_overlay_color_set_to_false_does_not_change_surface_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let surface_color = Color::new(0xFF121212);
            let widget = themed(
                &tester,
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .apply_elevation_overlay_color(false)
                        .color_scheme(ColorScheme::dark().with_surface(surface_color))
                        .build(app)
                },
                centered(build_material(8.0, GREEN, None, surface_color)),
            );
            tester.pump_widget(widget);
            assert_eq!(model(&tester).color(&tester.cell.borrow()), surface_color);
        });
    }

    #[test]
    fn apply_elevation_overlay_color_set_to_true_applies_a_semi_transparent_on_surface_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let surface_color = Color::new(0xFF121212);
            let on_surface_color = Colors::GREEN_ACCENT.primary();

            // The colors we should get with a base surface color of 0xFF121212 for
            // and a given elevation
            let elevation_colors = [
                (0.0, 0xFF121212),
                (1.0, 0xFF161D19),
                (2.0, 0xFF18211D),
                (3.0, 0xFF19241E),
                (4.0, 0xFF1A2620),
                (6.0, 0xFF1B2922),
                (8.0, 0xFF1C2C24),
                (12.0, 0xFF1D3027),
                (16.0, 0xFF1E3329),
                (24.0, 0xFF20362B),
            ];

            for (elevation, color) in elevation_colors {
                let widget = themed(
                    &tester,
                    |app| {
                        ThemeData::builder()
                            .use_material3(false)
                            .apply_elevation_overlay_color(true)
                            .color_scheme(
                                ColorScheme::dark()
                                    .with_surface(surface_color)
                                    .with_on_surface(on_surface_color),
                            )
                            .build(app)
                    },
                    centered(build_material(elevation, GREEN, None, surface_color)),
                );
                tester.pump_widget(widget);
                tester.pump_and_settle(); // wait for the elevation animation to finish
                let actual = model(&tester).color(&tester.cell.borrow());
                assert!(
                    is_same_color_as(actual, Color::new(color)),
                    "elevation {elevation}: {actual:?}"
                );
            }
        });
    }

    fn keyed(material: Material) -> (GlobalKey, WidgetRef) {
        let key = GlobalKey::new();
        let widget = material
            .key(key.clone().into())
            .child(SizedBox::new().width(100.0).height(100.0))
            .into_widget();
        (key, widget)
    }

    #[test]
    fn transparency_clipping_no_clip_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(Material::new().r#type(MaterialType::Transparency));
            tester.pump_widget(widget);
            let clip = tester.first::<RenderClipPath>();
            assert_eq!(
                RenderCustomClip::clip_behavior(clip, &tester.cell.borrow()),
                Clip::None
            );
        });
    }

    fn clip_shape(tester: &WidgetTester) -> (Clip, ShapeBorderRef) {
        let clip = tester.first::<RenderClipPath>();
        let app = tester.cell.borrow();
        let clipper = RenderCustomClip::clipper(clip, &app).expect("a shape clipper");
        let shape = clipper
            .as_any()
            .downcast_ref::<ShapeBorderClipper>()
            .expect("a ShapeBorderClipper")
            .shape
            .clone();
        (RenderCustomClip::clip_behavior(clip, &app), shape)
    }

    #[test]
    fn transparency_clipping_clips_to_bounding_rect_by_default_given_clip_anti_alias() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(
                Material::new()
                    .r#type(MaterialType::Transparency)
                    .clip_behavior(Clip::AntiAlias),
            );
            tester.pump_widget(widget);
            let (clip_behavior, shape) = clip_shape(&tester);
            assert_eq!(clip_behavior, Clip::AntiAlias);
            assert!(shape.eq_shape(&RoundedRectangleBorder::default()));
        });
    }

    #[test]
    fn transparency_clipping_clips_to_rounded_rect_when_border_radius_provided() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let radius = BorderRadius::all(Radius::circular(10.0));
            let (_key, widget) = keyed(
                Material::new()
                    .r#type(MaterialType::Transparency)
                    .border_radius(radius)
                    .clip_behavior(Clip::AntiAlias),
            );
            tester.pump_widget(widget);
            let (_, shape) = clip_shape(&tester);
            assert!(shape.eq_shape(&RoundedRectangleBorder::new(
                BorderSide::NONE,
                radius.into()
            )));
        });
    }

    #[test]
    fn transparency_clipping_clips_to_shape_when_provided() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(
                Material::new()
                    .r#type(MaterialType::Transparency)
                    .shape(StadiumBorder::default())
                    .clip_behavior(Clip::AntiAlias),
            );
            tester.pump_widget(widget);
            let (_, shape) = clip_shape(&tester);
            assert!(shape.eq_shape(&StadiumBorder::default()));
        });
    }

    #[test]
    fn physical_models_canvas() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(Material::new());
            tester.pump_widget(widget);
            let model = tester.first::<RenderPhysicalModel>();
            let app = tester.cell.borrow();
            assert_eq!(model.shape(&app), waft_painting::BoxShape::Rectangle);
            assert!(
                model
                    .border_radius(&app)
                    .is_none_or(|radius| radius == BorderRadius::ZERO)
            );
            assert_eq!(model.elevation(&app), 0.0);
        });
    }

    fn assert_physical_shape(
        material: Material,
        expected: impl ShapeBorder + 'static,
        elevation: f64,
    ) {
        let mut tester = tester();
        let (_key, widget) = keyed(material);
        tester.pump_widget(widget);
        assert!(
            shape_of(&tester).eq_shape(&expected),
            "{:?}",
            shape_of(&tester)
        );
        assert_eq!(model(&tester).elevation(&tester.cell.borrow()), elevation);
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(radius)).into(),
        )
    }

    #[test]
    fn physical_models_canvas_with_border_radius_and_elevation() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .border_radius(BorderRadius::all(Radius::circular(5.0)))
                    .elevation(1.0),
                rounded(5.0),
                1.0,
            );
        });
    }

    #[test]
    fn physical_models_canvas_with_shape_and_elevation() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .shape(StadiumBorder::default())
                    .elevation(1.0),
                StadiumBorder::default(),
                1.0,
            );
        });
    }

    #[test]
    fn physical_models_card() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new().r#type(MaterialType::Card),
                rounded(2.0),
                0.0,
            );
        });
    }

    #[test]
    fn physical_models_card_with_border_radius_and_elevation() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .r#type(MaterialType::Card)
                    .border_radius(BorderRadius::all(Radius::circular(5.0)))
                    .elevation(5.0),
                rounded(5.0),
                5.0,
            );
        });
    }

    #[test]
    fn physical_models_card_with_shape_and_elevation() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .r#type(MaterialType::Card)
                    .shape(StadiumBorder::default())
                    .elevation(5.0),
                StadiumBorder::default(),
                5.0,
            );
        });
    }

    #[test]
    fn physical_models_circle() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .r#type(MaterialType::Circle)
                    .color(Color::new(0xFF0000FF)),
                CircleBorder::default(),
                0.0,
            );
        });
    }

    #[test]
    fn physical_models_button() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .r#type(MaterialType::Button)
                    .color(Color::new(0xFF0000FF)),
                rounded(2.0),
                0.0,
            );
        });
    }

    #[test]
    fn physical_models_button_with_elevation_and_border_radius() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .r#type(MaterialType::Button)
                    .color(Color::new(0xFF0000FF))
                    .border_radius(BorderRadius::all(Radius::circular(6.0)))
                    .elevation(4.0),
                rounded(6.0),
                4.0,
            );
        });
    }

    #[test]
    fn physical_models_button_with_elevation_and_shape() {
        with_main_thread_stack(|| {
            assert_physical_shape(
                Material::new()
                    .r#type(MaterialType::Button)
                    .color(Color::new(0xFF0000FF))
                    .shape(StadiumBorder::default())
                    .elevation(4.0),
                StadiumBorder::default(),
                4.0,
            );
        });
    }

    /// What the border painter of the material draws.
    fn border_draws(tester: &WidgetTester) -> (bool, Vec<crate::test_support::Drawn>) {
        let paint = tester.first::<RenderCustomPaint>();
        let mut app = tester.cell.borrow_mut();
        let (foreground, painter) = match paint.foreground_painter(&app) {
            Some(painter) => (true, painter),
            None => (false, paint.painter(&app).expect("a border painter")),
        };
        let size = paint.size(&app);
        let mut canvas = Canvas::new();
        painter.paint(&mut app, &mut canvas, size);
        (
            foreground,
            crate::test_support::Drawn::draws(&canvas.build()),
        )
    }

    fn circle_with_side() -> CircleBorder {
        CircleBorder::new(
            BorderSide::new(
                Color::new(0xFF0000FF),
                2.0,
                waft_painting::BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            ),
            0.0,
        )
    }

    #[test]
    fn border_is_painted_on_physical_layers() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(
                Material::new()
                    .r#type(MaterialType::Button)
                    .color(Color::new(0xFF0000FF))
                    .shape(circle_with_side()),
            );
            tester.pump_widget(widget);
            let (foreground, draws) = border_draws(&tester);
            assert!(foreground);
            assert_eq!(draws.len(), 1, "{draws:?}");
        });
    }

    #[test]
    fn border_is_painted_for_transparent_material() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(
                Material::new()
                    .r#type(MaterialType::Transparency)
                    .shape(circle_with_side()),
            );
            tester.pump_widget(widget);
            let (_, draws) = border_draws(&tester);
            assert_eq!(draws.len(), 1, "{draws:?}");
        });
    }

    #[test]
    fn border_is_not_painted_for_when_border_side_is_none() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(
                Material::new()
                    .r#type(MaterialType::Transparency)
                    .shape(CircleBorder::default()),
            );
            tester.pump_widget(widget);
            let (_, draws) = border_draws(&tester);
            assert!(draws.is_empty(), "{draws:?}");
        });
    }

    /// The structure behind the `border_paint_above` and `border_paint_below` goldens.
    #[test]
    fn border_is_painted_above_the_child_by_default_and_below_when_specified() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (_key, widget) = keyed(Material::new().shape(rounded(8.0)));
            tester.pump_widget(widget);
            assert!(border_draws(&tester).0, "a foreground painter");

            let (_key, widget) = keyed(
                Material::new()
                    .shape(rounded(8.0))
                    .border_on_foreground(false),
            );
            tester.pump_widget(widget);
            assert!(!border_draws(&tester).0, "a background painter");
        });
    }

    /// Dart's `TrackPaintInkFeature`.
    struct TrackPaintInkFeature {
        ink_feature: InkFeatureData,
        paint_count: Rc<Cell<u32>>,
    }

    impl InkFeatureLeafData for TrackPaintInkFeature {
        fn ink_feature(&self) -> &InkFeatureData {
            &self.ink_feature
        }

        fn ink_feature_mut(&mut self) -> &mut InkFeatureData {
            &mut self.ink_feature
        }
    }

    impl InkFeatureLeaf for TrackPaintInkFeature {
        fn paint_feature(
            self: Handle<Self>,
            app: &mut App,
            _canvas: &mut Canvas,
            _transform: &Matrix4,
        ) {
            let count = &app.get(self).paint_count;
            count.set(count.get() + 1);
        }
    }

    fn offstage_material(
        material_key: &GlobalKey,
        sized_box_key: &GlobalKey,
        offstage: bool,
    ) -> WidgetRef {
        Material::new()
            .key(material_key.clone().into())
            .child(
                Offstage::new().offstage(offstage).child(
                    SizedBox::new()
                        .key(sized_box_key.clone().into())
                        .width(20.0)
                        .height(20.0),
                ),
            )
            .into_widget()
    }

    fn ink_features_under(tester: &WidgetTester) -> RenderHandle<RenderInkFeatures> {
        tester.first::<RenderInkFeatures>()
    }

    /// Paints the material's ink features as its `paint` does, onto a fresh canvas.
    fn repaint(tester: &WidgetTester) {
        let render = ink_features_under(tester);
        let mut canvas = Canvas::new();
        render.paint_ink_features(&mut tester.cell.borrow_mut(), &mut canvas, Offset::ZERO);
    }

    #[test]
    fn ink_feature_skips_painting_if_intermediate_node_skips() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let sized_box_key = GlobalKey::new();
            let material_key = GlobalKey::new();
            tester.pump_widget(offstage_material(&material_key, &sized_box_key, true));
            let paint_count = Rc::new(Cell::new(0));
            {
                let mut app = tester.cell.borrow_mut();
                let context = sized_box_key.current_context(&app).expect("mounted");
                let controller = Material::of(&app, context);
                let reference_box = context
                    .find_render_object(&app)
                    .and_then(|render_object| render_object.as_box())
                    .expect("a box");
                let ink_feature = InkFeatureData::new(&app, controller, reference_box, None);
                let tracker = app.create(TrackPaintInkFeature {
                    ink_feature,
                    paint_count: Rc::clone(&paint_count),
                });
                controller.add_ink_feature(&mut app, tracker.erase());
            }
            assert_eq!(paint_count.get(), 0);

            // Force a repaint. Since it's offstage, the ink feature should not get painted.
            repaint(&tester);
            assert_eq!(paint_count.get(), 0);

            tester.pump_widget(offstage_material(&material_key, &sized_box_key, false));
            // Gets a paint because the global keys have reused the elements and it is
            // now onstage.
            assert_eq!(paint_count.get(), 1);

            // Force a repaint again. This time, it gets repainted because it is onstage.
            repaint(&tester);
            assert_eq!(paint_count.get(), 2);

            let feature = ink_features_under(&tester)
                .debug_ink_features(&tester.cell.borrow())
                .expect("added")[0];
            feature.dispose(&mut tester.cell.borrow_mut());
            assert!(
                ink_features_under(&tester)
                    .debug_ink_features(&tester.cell.borrow())
                    .expect("added")
                    .is_empty()
            );
        });
    }

    #[test]
    fn layout_changed_notification_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(
                Material::new()
                    .child(Builder::new(|app, context| {
                        PlainLayoutChangedNotification.dispatch(app, Some(context));
                        SizedBox::shrink().into_widget()
                    }))
                    .into_widget(),
            );
        });
    }

    #[test]
    fn lookup_boundary_hides_material_from_material_maybe_of() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let material: Rc<Cell<Option<Option<AnyMaterialInkController>>>> = Rc::default();
            tester.pump_widget(
                Material::new()
                    .child(LookupBoundary::new(Builder::new({
                        let material = Rc::clone(&material);
                        move |app, context| {
                            material.set(Some(Material::maybe_of(app, context)));
                            SizedBox::shrink().into_widget()
                        }
                    })))
                    .into_widget(),
            );
            assert_eq!(material.get(), Some(None));
        });
    }

    #[test]
    fn lookup_boundary_hides_material_from_material_of() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tester.pump_widget(
                    Material::new()
                        .child(LookupBoundary::new(Builder::new(|app, context| {
                            Material::of(app, context);
                            SizedBox::shrink().into_widget()
                        })))
                        .into_widget(),
                );
            }));
            let panic = outcome.expect_err("the boundary hides the Material");
            let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
            assert!(
                message.starts_with(
                    "Material.of() was called with a context that does not have access to a \
                     Material widget.\nThe context provided to Material.of() does have a \
                     Material widget ancestor, but it is hidden by a LookupBoundary. This can \
                     happen because you are using a widget that looks for a Material ancestor, \
                     but no such ancestor exists within the closest LookupBoundary.\nThe \
                     context used was:\n  "
                ),
                "{message}"
            );
        });
    }

    /// Dart's `PaintRecorder`: logs the size of every paint.
    struct PaintRecorder {
        log: Rc<RefCell<Vec<Size>>>,
    }

    impl CustomPainter for PaintRecorder {
        fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
            self.log.borrow_mut().push(size);
            canvas.draw_rect(Offset::ZERO & size, &Paint::from_color(BLUE.into()));
        }

        fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
            false
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Dart's tree for "ListView scroll does not repaint": a recorder above a `Material` that
    /// holds a 2000-pixel list and a second recorder; `list_child` keys the list's child.
    fn scrolled_material(log: &Rc<RefCell<Vec<Size>>>, list_child: &GlobalKey) -> WidgetRef {
        let recorder = |dimension: f64| {
            SizedBox::square(Some(dimension))
                .child(CustomPaint::new().painter(PaintRecorder {
                    log: Rc::clone(log),
                }))
                .into_widget()
        };
        Directionality::new(
            TextDirection::Ltr,
            Column::new().children([
                SizedBox::new()
                    .width(150.0)
                    .height(150.0)
                    .child(CustomPaint::new().painter(PaintRecorder {
                        log: Rc::clone(log),
                    }))
                    .into_widget(),
                Expanded::new(
                    Material::new().child(
                        Column::new().children([
                            Expanded::new(
                                ListView::new().children([Container::new()
                                    .key(list_child.clone().into())
                                    .height(2000.0)
                                    .color(Color::new(0xFF00FF00))
                                    .into_widget()]),
                            )
                            .into_widget(),
                            recorder(100.0),
                        ]),
                    ),
                )
                .into_widget(),
            ]),
        )
        .into_widget()
    }

    /// Dart's `tester.drag(find.byType(ListView), offset)`: a touch down on the list, a move by
    /// `offset` and an up.
    fn drag_list(tester: &mut WidgetTester, list_child: &GlobalKey, offset: Offset) {
        let start = tester.center_of(list_child);
        let start = Offset::new(start.dx(), 300.0);
        let mut pointer = WidgetTester::touch(1);
        tester.down(&mut pointer, start);
        tester.move_to(&mut pointer, start + offset, true);
        tester.up(&pointer);
    }

    #[test]
    fn list_view_scroll_does_not_repaint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Rc<RefCell<Vec<Size>>> = Rc::default();
            let list_child = GlobalKey::new();
            tester.pump_widget(scrolled_material(&log, &list_child));

            // We paint twice because we have two CustomPaint widgets in the tree above
            // to test repainting both inside and outside the Material widget.
            assert_eq!(
                *log.borrow(),
                [Size::new(150.0, 150.0), Size::new(100.0, 100.0)]
            );
            log.borrow_mut().clear();

            drag_list(&mut tester, &list_child, Offset::new(0.0, -300.0));
            tester.pump(Duration::ZERO);

            assert!(log.borrow().is_empty(), "{:?}", log.borrow());
        });
    }

    // The ink half of "ListView scroll does not repaint": a scroll notification is a
    // `LayoutChangedNotification`, so a Material that holds ink repaints when its list scrolls.
    #[test]
    fn list_view_scroll_repaints_the_ink_of_its_material() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Rc<RefCell<Vec<Size>>> = Rc::default();
            let list_child = GlobalKey::new();
            tester.pump_widget(scrolled_material(&log, &list_child));
            let paint_count = Rc::new(Cell::new(0));
            {
                let mut app = tester.cell.borrow_mut();
                let context = list_child.current_context(&app).expect("mounted");
                let controller = Material::of(&app, context);
                let reference_box = context
                    .find_render_object(&app)
                    .and_then(|render_object| render_object.as_box())
                    .expect("a box");
                let ink_feature = InkFeatureData::new(&app, controller, reference_box, None);
                let tracker = app.create(TrackPaintInkFeature {
                    ink_feature,
                    paint_count: Rc::clone(&paint_count),
                });
                controller.add_ink_feature(&mut app, tracker.erase());
            }
            tester.pump(Duration::ZERO);
            assert_eq!(paint_count.get(), 1);

            drag_list(&mut tester, &list_child, Offset::new(0.0, -300.0));
            tester.pump(Duration::ZERO);

            assert_eq!(paint_count.get(), 2, "the ink repaints with the scroll");
        });
    }

    #[test]
    fn transparent_material_widget_does_not_absorb_hit_test() {
        // This is a regression test for https://github.com/flutter/flutter/issues/58665.
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pressed = Rc::new(Cell::new(false));
            let button = GlobalKey::new();
            let widget = Directionality::new(
                TextDirection::Ltr,
                Stack::new().children([
                    Material::new()
                        .child(
                            InkWell::new()
                                .key(button.clone().into())
                                .on_tap(Listener::new({
                                    let pressed = Rc::clone(&pressed);
                                    move |_app| pressed.set(true)
                                }))
                                .child(SizedBox::new().width(88.0).height(36.0)),
                        )
                        .into_widget(),
                    Material::new()
                        .r#type(MaterialType::Transparency)
                        .child(SizedBox::new().width(400.0).height(500.0))
                        .into_widget(),
                ]),
            )
            .into_widget();
            tester.pump_widget(widget);
            let center = tester.center_of(&button);
            tester.tap(center, 1);
            assert!(pressed.get());
        });
    }

    #[test]
    fn overlay_will_not_apply_to_materials_using_a_non_surface_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let widget = themed(
                &tester,
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .apply_elevation_overlay_color(true)
                        .color_scheme(ColorScheme::dark())
                        .build(app)
                },
                centered(build_material(8.0, GREEN, None, Colors::CYAN.primary())),
            );
            tester.pump_widget(widget);
            // Shouldn't change, as it is not using a ColorScheme.surface color
            assert_eq!(
                model(&tester).color(&tester.cell.borrow()),
                Colors::CYAN.primary()
            );
        });
    }

    #[test]
    fn overlay_will_not_apply_to_materials_using_a_light_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let widget = themed(
                &tester,
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .apply_elevation_overlay_color(true)
                        .color_scheme(ColorScheme::light())
                        .build(app)
                },
                centered(build_material(8.0, GREEN, None, Colors::CYAN.primary())),
            );
            tester.pump_widget(widget);
            // Shouldn't change, as it was under a light color scheme.
            assert_eq!(
                model(&tester).color(&tester.cell.borrow()),
                Colors::CYAN.primary()
            );
        });
    }

    #[test]
    #[allow(deprecated)]
    fn overlay_will_apply_to_materials_with_a_non_opaque_surface_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let surface_color = Color::new(0xFF121212);
            let surface_color_with_overlay = Color::new(0xC6353535);
            let widget = themed(
                &tester,
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .apply_elevation_overlay_color(true)
                        .color_scheme(ColorScheme::dark())
                        .build(app)
                },
                centered(build_material(
                    8.0,
                    GREEN,
                    None,
                    surface_color.with_opacity(0.75),
                )),
            );
            tester.pump_widget(widget);
            let color = model(&tester).color(&tester.cell.borrow());
            assert!(
                is_same_color_as(color, surface_color_with_overlay),
                "{color:?}"
            );
            assert!(!is_same_color_as(color, surface_color), "{color:?}");
        });
    }

    #[test]
    fn expected_overlay_color_can_be_computed_using_color_with_overlay() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let surface_color = Color::new(0xFF123456);
            let on_surface_color = Color::new(0xFF654321);
            let elevation = 8.0;

            let surface_color_with_overlay =
                ElevationOverlay::color_with_overlay(surface_color, on_surface_color, elevation);

            let widget = themed(
                &tester,
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .apply_elevation_overlay_color(true)
                        .color_scheme(
                            ColorScheme::dark()
                                .with_surface(surface_color)
                                .with_on_surface(on_surface_color),
                        )
                        .build(app)
                },
                centered(build_material(elevation, GREEN, None, surface_color)),
            );
            tester.pump_widget(widget);
            let color = model(&tester).color(&tester.cell.borrow());
            assert_eq!(color, surface_color_with_overlay);
            assert_ne!(color, surface_color);
        });
    }

    #[test]
    fn material_is_not_visible_from_sub_views() {
        with_main_thread_stack(|| {
            let fake_view: ViewRef =
                Rc::new(TestView::with_pixel_ratio(1600.0, 1200.0, 2.0).with_id(ViewId(100)));
            let platform = TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_view(test_view())
                .with_view(Rc::clone(&fake_view));
            let cell = AppCell::with_platform(Rc::new(platform));
            type Seen = Rc<Cell<Option<Option<AnyMaterialInkController>>>>;
            let outside_view: Seen = Rc::default();
            let inside_view: Seen = Rc::default();
            let outside_view_anchor: Seen = Rc::default();
            let seeing = |seen: &Seen, child: WidgetRef| {
                let seen = Rc::clone(seen);
                Builder::new(move |app, context| {
                    seen.set(Some(Material::maybe_of(app, context)));
                    child.clone()
                })
                .into_widget()
            };
            let tree = Material::new()
                .child(seeing(
                    &outside_view_anchor,
                    ViewAnchor::new(SizedBox::new())
                        .view(seeing(
                            &outside_view,
                            View::new(
                                fake_view,
                                seeing(&inside_view, SizedBox::new().into_widget()),
                            )
                            .into_widget(),
                        ))
                        .into_widget(),
                ))
                .into_widget();
            build(&cell, tree);

            assert!(matches!(outside_view_anchor.get(), Some(Some(_))));
            assert_eq!(outside_view.get(), Some(None));
            assert_eq!(inside_view.get(), Some(None));
        });
    }

    #[test]
    fn a_surface_tint_tween_is_freed_when_the_tint_goes_away() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let tinted = |surface_tint_color: Option<Color>| {
                centered(
                    build_material(4.0, GREEN, surface_tint_color, BLUE).r#type(MaterialType::Card),
                )
            };
            tester.pump_widget(tinted(Some(Color::new(0xFF44CCFF))));
            let elements = tester.elements();
            let app = tester.cell.borrow();
            let state = elements
                .into_iter()
                .find_map(|element| element.downcast::<StatefulElement<MaterialInterior>>(&app))
                .expect("a material interior")
                .state(&app);
            let tween = app
                .get(state)
                .surface_tint_color
                .as_ref()
                .expect("a tint tween")
                .handle();
            drop(app);
            tester.pump_widget(tinted(None));
            assert!(tester.cell.borrow().get(state).surface_tint_color.is_none());
            assert!(!tester.cell.borrow().contains(tween));
        });
    }

    #[test]
    fn material_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(
                Center::new()
                    .child(SizedBox::new().child(Material::new().key(key.clone().into())))
                    .into_widget(),
            );
            let size = tester
                .render_object(&key)
                .as_box()
                .expect("a box")
                .size(&tester.cell.borrow());
            assert_eq!(size, Size::ZERO);
        });
    }
}
