//! Flutter counterpart: `widgets/container.dart`.
//!
//! [`Container`]'s `clipBehavior` step and its `_DecorationClipper` wait on `ClipPath`.

use reveal_embedder::{Clip, Matrix4, Offset, Path, Size, TextDirection};
use reveal_foundation::App;
use reveal_painting::{AlignmentGeometry, AnyColor, Decoration, EdgeInsetsGeometry};
use reveal_rendering::{
    AnyRenderObject, BoxConstraints, Constraints, CustomClipper, DecorationPosition, RenderBox,
    RenderDecoratedBox, RenderHandle,
};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget,
    StatelessWidget, WidgetRef,
};
use crate::widgets::basic::{
    Align, ClipPath, ColoredBox, ConstrainedBox, Directionality, LimitedBox, Padding, Transform,
};
use crate::widgets::image::create_local_image_configuration;

/// A widget that paints a [`Decoration`] either before or after its child paints.
///
/// `Container` insets its child by the widths of the borders; this widget does
/// not.
///
/// Commonly used with `BoxDecoration`.
///
/// The [`child`](Self::child) is not clipped. To clip a child to the shape of a particular
/// `ShapeDecoration`, consider using a `ClipPath` widget.
///
/// This sample shows a radial gradient that draws a moon on a night sky:
///
/// ```text
/// DecoratedBox {
///   decoration: Box::new(BoxDecoration::new().gradient(RadialGradient { .. })),
///   ..
/// }
/// ```
///
/// See also:
///
///  * `Ink`, which paints a [`Decoration`] on a `Material`, allowing
///    `InkResponse` and `InkWell` splashes to paint over them.
///  * `DecoratedSliver`, which applies a [`Decoration`] to a sliver.
///  * `DecoratedBoxTransition`, the version of this class that animates on the
///    [`decoration`](Self::decoration).
///  * [`Decoration`], which you can extend to provide other effects with
///    [`DecoratedBox`].
///  * `CustomPaint`, another way to draw custom effects from the widget layer.
///
/// Flutter home: `widgets/container.dart`.
#[derive(Debug)]
pub struct DecoratedBox {
    pub key: Option<KeyRef>,
    /// What decoration to paint.
    ///
    /// Commonly a `BoxDecoration`.
    pub decoration: Box<dyn Decoration>,
    /// Whether to paint the box decoration behind or in front of the child.
    ///
    /// By default the decoration paints behind the child.
    pub position: DecorationPosition,
    pub child: Option<WidgetRef>,
}

impl DecoratedBox {
    /// Creates a `DecoratedBox`; Dart's optional named arguments are the setters.
    pub fn new(decoration: impl Decoration + 'static) -> DecoratedBox {
        DecoratedBox {
            key: None,
            decoration: Box::new(decoration),
            position: DecorationPosition::Background,
            child: None,
        }
    }

    /// Dart `DecoratedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> DecoratedBox {
        self.key = Some(key);
        self
    }

    /// Dart `DecoratedBox(position:)`.
    pub fn position(mut self, position: DecorationPosition) -> DecoratedBox {
        self.position = position;
        self
    }

    /// Dart `DecoratedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> DecoratedBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for DecoratedBox {
    type RenderObject = RenderDecoratedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let configuration = create_local_image_configuration(app, context, None);
        RenderDecoratedBox::new(
            app,
            self.decoration.clone_box(),
            self.position,
            configuration,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderDecoratedBox>,
    ) {
        render_object.set_decoration(app, self.decoration.clone_box());
        let configuration = create_local_image_configuration(app, context, None);
        render_object.set_configuration(app, configuration);
        render_object.set_position(app, self.position);
    }
}

impl SingleChildRenderObjectWidget for DecoratedBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A convenience widget that combines common painting, positioning, and sizing
/// widgets.
///
/// A container first surrounds the child with [`padding`](Self::padding) (inflated by any
/// borders present in the [`decoration`](Self::decoration)) and then applies additional
/// [`constraints`](Self::constraints) to the padded extent (incorporating the
/// [`width`](Self::width) and [`height`](Self::height) as constraints, if either is set). The
/// container is then surrounded by additional empty space described from the
/// [`margin`](Self::margin).
///
/// During painting, the container first applies the given [`transform`](Self::transform), then
/// paints the [`decoration`](Self::decoration) to fill the padded extent, then it paints the
/// child, and finally paints the [`foreground_decoration`](Self::foreground_decoration), also
/// filling the padded extent.
///
/// Containers with no children try to be as big as possible unless the incoming
/// constraints are unbounded, in which case they try to be as small as
/// possible. Containers with children size themselves to their children. The
/// [`width`](Self::width), [`height`](Self::height), and [`constraints`](Self::constraints)
/// arguments to the constructor override this.
///
/// By default, containers return false for all hit tests. If the [`color`](Self::color)
/// property is specified, the hit testing is handled by [`ColoredBox`], which
/// always returns true. If the [`decoration`](Self::decoration) or
/// [`foreground_decoration`](Self::foreground_decoration) properties are specified, hit
/// testing is handled by `Decoration::hit_test`.
///
/// ## Layout behavior
///
/// _See [`BoxConstraints`] for an introduction to box layout models._
///
/// Since [`Container`] combines a number of other widgets each with their own
/// layout behavior, [`Container`]'s layout behavior is somewhat complicated.
///
/// Summary: [`Container`] tries, in order: to honor [`alignment`](Self::alignment), to size
/// itself to the [`child`](Self::child), to honor the [`width`](Self::width),
/// [`height`](Self::height), and [`constraints`](Self::constraints), to expand to fit the
/// parent, to be as small as possible.
///
/// More specifically:
///
/// If the widget has no child, no height, no width, no [`constraints`](Self::constraints),
/// and the parent provides unbounded constraints, then [`Container`] tries to
/// size as small as possible.
///
/// If the widget has no child and no [`alignment`](Self::alignment), but a height, width, or
/// [`constraints`](Self::constraints) are provided, then the [`Container`] tries to be as
/// small as possible given the combination of those constraints and the parent's constraints.
///
/// If the widget has no child, no height, no width, no [`constraints`](Self::constraints), and
/// no [`alignment`](Self::alignment), but the parent provides bounded constraints, then
/// [`Container`] expands to fit the constraints provided by the parent.
///
/// If the widget has an [`alignment`](Self::alignment), and the parent provides unbounded
/// constraints, then the [`Container`] tries to size itself around the child.
///
/// If the widget has an [`alignment`](Self::alignment), and the parent provides bounded
/// constraints, then the [`Container`] tries to expand to fit the parent, and
/// then positions the child within itself as per the [`alignment`](Self::alignment).
///
/// Otherwise, the widget has a [`child`](Self::child) but no height, no width, no
/// [`constraints`](Self::constraints), and no [`alignment`](Self::alignment), and the
/// [`Container`] passes the constraints from the parent to the child and sizes itself to match
/// the child.
///
/// The [`margin`](Self::margin) and [`padding`](Self::padding) properties also affect the
/// layout, as described in the documentation for those properties. (Their effects merely
/// augment the rules described above.) The [`decoration`](Self::decoration) can implicitly
/// increase the [`padding`](Self::padding) (e.g. borders in a `BoxDecoration` contribute to
/// the [`padding`](Self::padding)); see `Decoration::padding`.
///
/// ## Example
///
/// This example shows a 48x48 amber square (placed inside a [`Center`](crate::Center) widget
/// in case the parent widget has its own opinions regarding the size that the [`Container`]
/// should take), with a margin so that it stays away from neighboring widgets:
///
/// ```text
/// Center::new().child(
///     Container::new()
///         .margin(EdgeInsetsGeometry::all(10.0))
///         .color(AMBER_600)
///         .width(48.0)
///         .height(48.0),
/// )
/// ```
///
/// See also:
///
///  * `AnimatedContainer`, a variant that smoothly animates the properties when
///    they change.
///  * `Border`, which has a sample which uses [`Container`] heavily.
///  * `Ink`, which paints a [`Decoration`] on a `Material`, allowing
///    `InkResponse` and `InkWell` splashes to paint over them.
///  * The [catalog of layout widgets](https://docs.flutter.dev/ui/widgets/layout).
///
/// Flutter's `clipBehavior` waits on `ClipPath`.
#[derive(Debug)]
pub struct Container {
    pub key: Option<KeyRef>,
    /// Align the [`child`](Self::child) within the container.
    ///
    /// If non-`None`, the container will expand to fill its parent and position its
    /// child within itself according to the given value. If the incoming
    /// constraints are unbounded, then the child will be shrink-wrapped instead.
    ///
    /// Ignored if [`child`](Self::child) is `None`.
    ///
    /// See also:
    ///
    ///  * `Alignment`, a class with convenient constants typically used to
    ///    specify an [`AlignmentGeometry`].
    ///  * `AlignmentDirectional`, like `Alignment` for specifying alignments
    ///    relative to text direction.
    pub alignment: Option<AlignmentGeometry>,
    /// Empty space to inscribe inside the [`decoration`](Self::decoration). The
    /// [`child`](Self::child), if any, is placed inside this padding.
    ///
    /// This padding is in addition to any padding inherent in the
    /// [`decoration`](Self::decoration); see `Decoration::padding`.
    pub padding: Option<EdgeInsetsGeometry>,
    /// The color to paint behind the [`child`](Self::child).
    ///
    /// This property should be preferred when the background is a simple color.
    /// For other cases, such as gradients or images, use the
    /// [`decoration`](Self::decoration) property.
    ///
    /// If the [`decoration`](Self::decoration) is used, this property must be `None`. A
    /// background color may still be painted by the [`decoration`](Self::decoration) even if
    /// this property is `None`.
    pub color: Option<AnyColor>,
    /// See [`ColoredBox::is_anti_alias`].
    pub is_anti_alias: bool,
    /// The decoration to paint behind the [`child`](Self::child).
    ///
    /// Use the [`color`](Self::color) property to specify a simple solid color.
    ///
    /// The [`child`](Self::child) is not clipped to the decoration. To clip a child to the
    /// shape of a particular `ShapeDecoration`, consider using a `ClipPath` widget.
    pub decoration: Option<Box<dyn Decoration>>,
    /// The decoration to paint in front of the [`child`](Self::child).
    pub foreground_decoration: Option<Box<dyn Decoration>>,
    /// Additional constraints to apply to the child.
    ///
    /// The [`width`](Self::width) and [`height`](Self::height) setters are combined with the
    /// given constraints: this is Dart's folded field.
    ///
    /// The [`padding`](Self::padding) goes inside the constraints.
    pub constraints: Option<BoxConstraints>,
    given_constraints: Option<BoxConstraints>,
    width: Option<f64>,
    height: Option<f64>,
    /// Empty space to surround the [`decoration`](Self::decoration) and [`child`](Self::child).
    pub margin: Option<EdgeInsetsGeometry>,
    /// The clip behavior when [`decoration`](Self::decoration) is not `None`.
    ///
    /// Defaults to [`Clip::None`]. Must be [`Clip::None`] if
    /// [`decoration`](Self::decoration) is `None`.
    ///
    /// If a clip is to be applied, the `Decoration::get_clip_path` method
    /// for the provided decoration must return a clip path. (This is not
    /// supported by all decorations; the default implementation of that
    /// method throws an `UnsupportedError`.)
    pub clip_behavior: Clip,
    /// The transformation matrix to apply before painting the container.
    pub transform: Option<Matrix4>,
    /// The alignment of the origin, relative to the size of the container, if
    /// [`transform`](Self::transform) is specified.
    ///
    /// When [`transform`](Self::transform) is `None`, the value of this property is ignored.
    ///
    /// See also:
    ///
    ///  * [`Transform::alignment`], which is set by this property.
    pub transform_alignment: Option<AlignmentGeometry>,
    /// The [`child`](Self::child) contained by the container.
    ///
    /// If `None`, and if the [`constraints`](Self::constraints) are unbounded or also `None`,
    /// the container will expand to fill all available space in its parent, unless
    /// the parent provides unbounded constraints, in which case the container
    /// will attempt to be as small as possible.
    pub child: Option<WidgetRef>,
}

impl Container {
    /// Creates a widget that combines common painting, positioning, and sizing widgets;
    /// Dart's named arguments are the setters.
    ///
    /// The [`height`](Self::height) and [`width`](Self::width) values include the padding.
    ///
    /// The [`color`](Self::color) and [`decoration`](Self::decoration) arguments cannot both
    /// be supplied, since it would potentially result in the decoration drawing over the
    /// background color. To supply a decoration with a color, use
    /// `.decoration(BoxDecoration::new().color(color))`.
    pub fn new() -> Container {
        Container::default()
    }

    /// Dart `Container(key:)`.
    pub fn key(mut self, key: KeyRef) -> Container {
        self.key = Some(key);
        self
    }

    /// Dart `Container(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> Container {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `Container(padding:)`; must be non-negative.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> Container {
        debug_assert!(padding.is_non_negative());
        self.padding = Some(padding);
        self
    }

    /// Dart `Container(color:)`; cannot be combined with
    /// [`decoration`](Self::decoration).
    pub fn color(mut self, color: impl Into<AnyColor>) -> Container {
        debug_assert!(self.decoration.is_none(), "{CANNOT_PROVIDE_BOTH}");
        self.color = Some(color.into());
        self
    }

    /// Dart `Container(isAntiAlias:)`.
    pub fn is_anti_alias(mut self, is_anti_alias: bool) -> Container {
        self.is_anti_alias = is_anti_alias;
        self
    }

    /// Dart `Container(decoration:)`; cannot be combined with [`color`](Self::color).
    pub fn decoration(mut self, decoration: impl Decoration + 'static) -> Container {
        debug_assert!(self.color.is_none(), "{CANNOT_PROVIDE_BOTH}");
        debug_assert!(decoration.debug_assert_is_valid());
        self.decoration = Some(Box::new(decoration));
        self
    }

    /// Dart `Container(foregroundDecoration:)`.
    pub fn foreground_decoration(
        mut self,
        foreground_decoration: impl Decoration + 'static,
    ) -> Container {
        self.foreground_decoration = Some(Box::new(foreground_decoration));
        self
    }

    /// Dart `Container(width:)`: tightens [`constraints`](Self::constraints).
    pub fn width(mut self, width: f64) -> Container {
        self.width = Some(width);
        self.fold_constraints();
        self
    }

    /// Dart `Container(height:)`: tightens [`constraints`](Self::constraints).
    pub fn height(mut self, height: f64) -> Container {
        self.height = Some(height);
        self.fold_constraints();
        self
    }

    /// Dart `Container(constraints:)`; must be valid.
    pub fn constraints(mut self, constraints: BoxConstraints) -> Container {
        debug_assert!(constraints.debug_assert_is_valid(false));
        self.given_constraints = Some(constraints);
        self.fold_constraints();
        self
    }

    /// Dart `Container(margin:)`; must be non-negative.
    pub fn margin(mut self, margin: EdgeInsetsGeometry) -> Container {
        debug_assert!(margin.is_non_negative());
        self.margin = Some(margin);
        self
    }

    /// Dart `Container(clipBehavior:)`; must be `Clip::None` without a decoration.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Container {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Container(transform:)`.
    pub fn transform(mut self, transform: Matrix4) -> Container {
        self.transform = Some(transform);
        self
    }

    /// Dart `Container(transformAlignment:)`.
    pub fn transform_alignment(mut self, transform_alignment: AlignmentGeometry) -> Container {
        self.transform_alignment = Some(transform_alignment);
        self
    }

    /// Dart `Container(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Container {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart's constructor line folding `width` and `height` into `constraints`.
    fn fold_constraints(&mut self) {
        self.constraints = if self.width.is_some() || self.height.is_some() {
            Some(match self.given_constraints {
                Some(constraints) => constraints.tighten(self.width, self.height),
                None => BoxConstraints::tight_for(self.width, self.height),
            })
        } else {
            self.given_constraints
        };
    }

    fn padding_including_decoration(&self) -> Option<EdgeInsetsGeometry> {
        let decoration_padding = self
            .decoration
            .as_ref()
            .map(|decoration| decoration.padding());
        match (self.padding, decoration_padding) {
            (None, padding) => padding,
            (padding, None) => padding,
            (Some(padding), Some(decoration_padding)) => Some(padding.add(decoration_padding)),
        }
    }
}

/// Dart's assert message when both a color and a decoration are supplied.
const CANNOT_PROVIDE_BOTH: &str = "Cannot provide both a color and a decoration.\n\
     The color argument is just a shorthand for \"decoration: BoxDecoration(color: color)\".\n\
     To use both a color and other decoration properties, set the color in the BoxDecoration \
     instead.";

impl Default for Container {
    fn default() -> Container {
        Container {
            key: None,
            alignment: None,
            padding: None,
            color: None,
            is_anti_alias: true,
            decoration: None,
            foreground_decoration: None,
            constraints: None,
            given_constraints: None,
            width: None,
            height: None,
            margin: None,
            clip_behavior: Clip::None,
            transform: None,
            transform_alignment: None,
            child: None,
        }
    }
}

/// Dart's `_DecorationClipper`: a clipper that uses `Decoration::get_clip_path` to clip.
struct DecorationClipper {
    text_direction: TextDirection,
    decoration: Box<dyn Decoration>,
}

impl DecorationClipper {
    fn new(text_direction: Option<TextDirection>, decoration: Box<dyn Decoration>) -> Self {
        DecorationClipper {
            text_direction: text_direction.unwrap_or(TextDirection::Ltr),
            decoration,
        }
    }
}

impl CustomClipper<Arc<Path>> for DecorationClipper {
    fn get_clip(&self, size: Size) -> Arc<Path> {
        self.decoration
            .get_clip_path(Offset::ZERO & size, self.text_direction)
    }

    fn should_reclip(&self, old_clipper: &dyn CustomClipper<Arc<Path>>) -> bool {
        let Some(old_clipper) = old_clipper.as_any().downcast_ref::<DecorationClipper>() else {
            return true;
        };
        !old_clipper.decoration.eq_decoration(&*self.decoration)
            || old_clipper.text_direction != self.text_direction
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl StatelessWidget for Container {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(self.decoration.is_some() || self.clip_behavior == Clip::None);
        let mut current = self.child.clone();

        if self.child.is_none()
            && self
                .constraints
                .is_none_or(|constraints| !constraints.is_tight())
        {
            current = Some(
                LimitedBox::new()
                    .max_width(0.0)
                    .max_height(0.0)
                    .child(ConstrainedBox::new(BoxConstraints::expand(None, None)))
                    .into_widget(),
            );
        } else if let Some(alignment) = self.alignment {
            current = Some(
                Align {
                    alignment,
                    child: current,
                    ..Align::new()
                }
                .into_widget(),
            );
        }

        if let Some(effective_padding) = self.padding_including_decoration() {
            current = Some(
                Padding {
                    key: None,
                    padding: effective_padding,
                    child: current,
                }
                .into_widget(),
            );
        }

        if let Some(color) = &self.color {
            current = Some(
                ColoredBox {
                    child: current,
                    ..ColoredBox::new(color.clone()).is_anti_alias(self.is_anti_alias)
                }
                .into_widget(),
            );
        }

        if self.clip_behavior != Clip::None {
            let decoration = self
                .decoration
                .as_ref()
                .expect("clip_behavior needs a decoration");
            let text_direction = Directionality::maybe_of(app, context);
            current = Some(
                ClipPath {
                    key: None,
                    clipper: Some(Rc::new(DecorationClipper::new(
                        text_direction,
                        decoration.clone_box(),
                    ))),
                    clip_behavior: self.clip_behavior,
                    child: current,
                }
                .into_widget(),
            );
        }

        if let Some(decoration) = &self.decoration {
            current = Some(
                DecoratedBox {
                    key: None,
                    decoration: decoration.clone_box(),
                    position: DecorationPosition::Background,
                    child: current,
                }
                .into_widget(),
            );
        }

        if let Some(foreground_decoration) = &self.foreground_decoration {
            current = Some(
                DecoratedBox {
                    key: None,
                    decoration: foreground_decoration.clone_box(),
                    position: DecorationPosition::Foreground,
                    child: current,
                }
                .into_widget(),
            );
        }

        if let Some(constraints) = self.constraints {
            current = Some(
                ConstrainedBox {
                    key: None,
                    constraints,
                    child: current,
                }
                .into_widget(),
            );
        }

        if let Some(margin) = self.margin {
            current = Some(
                Padding {
                    key: None,
                    padding: margin,
                    child: current,
                }
                .into_widget(),
            );
        }

        if let Some(transform) = self.transform {
            current = Some(
                Transform {
                    alignment: self.transform_alignment,
                    child: current,
                    ..Transform::new(transform)
                }
                .into_widget(),
            );
        }

        current.expect("a Container builds at least one widget")
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::{Clip, Color, Offset, Size, TextDirection};
    use reveal_painting::{Border, BorderSide, BorderStyle, BoxDecoration};
    use reveal_rendering::{
        AnyRenderBox, BoxParentData, RenderAligningShiftedBox, RenderBox, RenderColoredBox,
        RenderConstrainedBox, RenderDecoratedBox, RenderHandle, RenderLimitedBox, RenderObject,
        RenderObjectWithChildMixin, RenderPadding, RenderPositionedBox, RenderTransform,
    };

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    /// The typed render object under a single-child parent.
    fn typed_child<P, C: RenderObject>(parent: RenderHandle<P>, app: &App) -> RenderHandle<C>
    where
        P: RenderObject + RenderObjectWithChildMixin<ChildType = AnyRenderBox>,
    {
        parent
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<C>(app)
            .expect("the expected render object")
    }

    fn root_child<T: RenderObject>(harness: &Harness, app: &App) -> RenderHandle<T> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<T>(app)
            .expect("the expected render object")
    }

    #[test]
    fn decorated_box_paints_its_decoration_and_updates_in_place() {
        let mut app = App::new();
        let red = Color::from_argb(255, 255, 0, 0);
        let blue = Color::from_argb(255, 0, 0, 255);
        let decorated = |color: Color| {
            DecoratedBox {
                key: None,
                decoration: Box::new(BoxDecoration::new().color(color)),
                position: DecorationPosition::Background,
                child: Some(SizedBox::new().width(10.0).height(10.0).into_widget()),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, decorated(red));
        harness.pump(&mut app);
        let render_object = root_child::<RenderDecoratedBox>(&harness, &app);
        assert!(!render_object.as_object().debug_needs_paint(&app));
        let mut canvas = reveal_embedder::Canvas::new();
        harness
            .render_root(&app)
            .as_object()
            .debug_layer(&app)
            .expect("the root painted")
            .add_to_scene(&app, &mut canvas);
        let ops = canvas.build().ops().to_vec();
        assert!(
            ops.iter()
                .any(|op| matches!(op, reveal_embedder::valo::Op::DrawDisplayList { .. })),
            "{ops:?}"
        );

        harness.set_child(&mut app, decorated(blue));
        harness.pump(&mut app);
        let same = root_child::<RenderDecoratedBox>(&harness, &app);
        assert_eq!(
            same, render_object,
            "the same RenderDecoratedBox is reconfigured"
        );
        let decoration = render_object
            .decoration(&app)
            .as_any()
            .downcast_ref::<BoxDecoration>()
            .expect("a BoxDecoration");
        assert_eq!(decoration.color, Some(blue.into()));
    }

    #[test]
    fn a_container_builds_dart_s_chain_outside_in() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            Container::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .padding(EdgeInsetsGeometry::all(4.0))
                .color(Color::from_argb(255, 255, 0, 0))
                .width(80.0)
                .height(60.0)
                .margin(EdgeInsetsGeometry::all(2.0))
                .transform(Matrix4::scale(2.0, 2.0))
                .transform_alignment(AlignmentGeometry::CENTER)
                .child(SizedBox::new().width(10.0).height(10.0))
                .into_widget(),
        );
        harness.pump(&mut app);

        let transform = root_child::<RenderTransform>(&harness, &app);
        assert_eq!(transform.alignment(&app), Some(AlignmentGeometry::CENTER));
        let margin = typed_child::<_, RenderPadding>(transform, &app);
        assert_eq!(margin.padding(&app), EdgeInsetsGeometry::all(2.0));
        let constrained = typed_child::<_, RenderConstrainedBox>(margin, &app);
        assert_eq!(
            constrained.additional_constraints(&app),
            BoxConstraints::tight_for(Some(80.0), Some(60.0)),
            "width and height fold into the constraints"
        );
        let colored = typed_child::<_, RenderColoredBox>(constrained, &app);
        assert!(colored.is_anti_alias(&app));
        let padding = typed_child::<_, RenderPadding>(colored, &app);
        assert_eq!(padding.padding(&app), EdgeInsetsGeometry::all(4.0));
        let align = typed_child::<_, RenderPositionedBox>(padding, &app);
        assert_eq!(align.alignment(&app), AlignmentGeometry::TOP_LEFT);
        let child = typed_child::<_, RenderConstrainedBox>(align, &app);
        assert_eq!(child.size(&app), Size::new(10.0, 10.0));

        assert_eq!(
            transform.size(&app),
            Size::new(84.0, 64.0),
            "the margin is outside the constrained extent"
        );
    }

    #[test]
    fn a_childless_container_expands_inside_a_limited_box() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, Container::new().into_widget());
        harness.pump(&mut app);
        let limited = root_child::<RenderLimitedBox>(&harness, &app);
        assert_eq!(limited.max_width(&app), 0.0);
        assert_eq!(limited.max_height(&app), 0.0);
        let expanded = typed_child::<_, RenderConstrainedBox>(limited, &app);
        assert_eq!(
            expanded.additional_constraints(&app),
            BoxConstraints::expand(None, None)
        );
        assert_eq!(
            limited.size(&app),
            Size::new(300.0, 200.0),
            "bounded constraints win over the zero limits"
        );
    }

    #[test]
    fn a_decoration_adds_its_padding_and_paints_behind_the_child() {
        let mut app = App::new();
        let decoration = BoxDecoration::new()
            .color(Color::from_argb(255, 0, 0, 255))
            .border(Border::all(
                Color::from_argb(255, 0, 0, 0),
                3.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            ));
        let decoration_padding = decoration.padding();
        let harness = Harness::mount(
            &mut app,
            Container::new()
                .padding(EdgeInsetsGeometry::all(4.0))
                .decoration(decoration)
                .child(SizedBox::new().width(10.0).height(10.0))
                .into_widget(),
        );
        harness.pump(&mut app);

        let decorated = root_child::<RenderDecoratedBox>(&harness, &app);
        let padding = typed_child::<_, RenderPadding>(decorated, &app);
        assert_eq!(
            padding.padding(&app),
            EdgeInsetsGeometry::all(4.0).add(decoration_padding),
            "Dart's _paddingIncludingDecoration"
        );
        assert_eq!(
            decorated.size(&app),
            Size::new(24.0, 24.0),
            "the child, the padding, and the border's own inset"
        );
    }

    #[test]
    fn an_aligned_container_positions_its_child() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            Container::new()
                .alignment(AlignmentGeometry::BOTTOM_RIGHT)
                .child(SizedBox::new().width(10.0).height(10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        let align = root_child::<RenderPositionedBox>(&harness, &app);
        assert_eq!(
            align.size(&app),
            Size::new(300.0, 200.0),
            "an alignment expands to fill the parent"
        );
        let child = typed_child::<_, RenderConstrainedBox>(align, &app);
        assert_eq!(
            child
                .as_object()
                .parent_data_of::<BoxParentData>(&app)
                .offset,
            Offset::new(290.0, 190.0)
        );
    }

    #[test]
    fn width_and_height_tighten_the_constraints_in_any_setter_order() {
        let container = Container::new()
            .constraints(BoxConstraints::new().min_width(20.0).max_width(200.0))
            .width(80.0);
        assert_eq!(
            container.constraints,
            Some(BoxConstraints::new().min_width(80.0).max_width(80.0)),
            "Dart's constraints.tighten(width: ..)"
        );
        let reordered = Container::new()
            .width(80.0)
            .constraints(BoxConstraints::new().min_width(20.0).max_width(200.0));
        assert_eq!(reordered.constraints, container.constraints);

        let unconstrained = Container::new().height(60.0);
        assert_eq!(
            unconstrained.constraints,
            Some(BoxConstraints::tight_for(None, Some(60.0))),
            "Dart's BoxConstraints.tightFor(height: ..)"
        );
    }

    #[test]
    fn a_clip_behavior_clips_the_decoration_through_its_clip_path() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                Container::new()
                    .decoration(
                        BoxDecoration::new()
                            .color(Color::new(0xFF00FF00))
                            .border_radius(reveal_painting::BorderRadiusGeometry::circular(6.0)),
                    )
                    .clip_behavior(Clip::AntiAlias)
                    .child(SizedBox::shrink()),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let decorated = harness
            .render_root(&app)
            .child(&app)
            .expect("the container's tree")
            .as_object()
            .downcast::<reveal_rendering::RenderDecoratedBox>(&app)
            .expect("the background DecoratedBox");
        let clip = decorated
            .child(&app)
            .expect("the clip under the decoration")
            .as_object()
            .downcast::<reveal_rendering::RenderClipPath>(&app)
            .expect("a RenderClipPath");
        assert_eq!(
            reveal_rendering::RenderCustomClip::clip_behavior(clip, &app),
            Clip::AntiAlias
        );
    }
}
