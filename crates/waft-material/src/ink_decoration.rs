//! Flutter counterpart: material_ui's `ink_decoration.dart`.
//!
//! `Ink.image` waits with `DecorationImage`.

use std::fmt;
use std::rc::Rc;

use waft_embedder::{Canvas, Color, Matrix4, Offset};
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref};
use waft_painting::{
    BoxDecoration, BoxPainter, DecorationRef, EdgeInsets, EdgeInsetsGeometry, ImageConfiguration,
    get_as_translation,
};
use waft_rendering::{AnyRenderBox, BoxConstraints};
use waft_widgets::{
    BuildContext, Builder, ConstrainedBox, GlobalKey, IntoWidget, KeyRef, Padding, SizedBox, State,
    StateData, StatefulWidget, Visibility, WidgetRef, create_local_image_configuration,
};

use crate::debug::debug_check_has_material;
use crate::material::{
    AnyMaterialInkController, InkFeature, InkFeatureData, InkFeatureLeaf, InkFeatureLeafData,
    Material,
};

/// A convenience widget for drawing images and other decorations on [`Material`]
/// widgets, so that [`InkWell`](crate::InkWell) and [`InkResponse`](crate::InkResponse)
/// splashes will render over them.
///
/// Ink splashes and highlights, as rendered by [`InkWell`](crate::InkWell) and similar
/// widgets, are actually painted on the underlying [`Material`], which is typically
/// below the children of the [`Material`] widget. This is problematic if a child of the
/// [`Material`] has its own decoration, which would cover the ink effects.
///
/// The [`Ink`] widget fixes this by painting on the [`Material`] directly, below the ink
/// effects.
///
/// The [`padding`](Self::padding) is applied around the [`child`](Self::child); the
/// decoration's padding is added to it.
///
/// [`Ink`] is not a replacement for a `Container`: it paints a decoration on the
/// material, not around it, so the [`Material`]'s clip is the decoration's clip.
///
/// See also:
///
///  * `Container`, a more generic form of this widget which paints itself,
///    rather that deferring to the nearest [`Material`] widget.
///  * [`InkDecoration`], the [`InkFeature`] subclass used by this widget to paint
///    on [`Material`] widgets.
///  * [`InkWell`](crate::InkWell) and [`InkResponse`](crate::InkResponse), which also draw on
///    [`Material`] widgets.
pub struct Ink {
    pub key: Option<KeyRef>,

    /// The [`child`](Self::child) contained by the container.
    ///
    /// If null, this widget's size will expand to fill the box constraints, or, when
    /// [`width`](Self::width) or [`height`](Self::height) is given, that size.
    pub child: Option<WidgetRef>,

    /// Empty space to inscribe inside the [`decoration`](Self::decoration). The
    /// [`child`](Self::child), if any, is placed inside this padding.
    ///
    /// This padding is in addition to any padding inherent in the
    /// [`decoration`](Self::decoration) (see `Decoration::padding`).
    pub padding: Option<EdgeInsetsGeometry>,

    /// Dart's `color:` argument: a shorthand for "decoration: BoxDecoration(color: color)".
    pub color: Option<Color>,

    /// The decoration to paint on the nearest ancestor [`Material`] widget.
    ///
    /// A shorthand for specifying just a solid color is available in the
    /// constructor: set the `color` argument instead of the `decoration`
    /// argument.
    ///
    /// A shorthand for specifying just an image is also available using the
    /// `Ink.image` constructor.
    pub decoration: Option<DecorationRef>,

    /// A width to apply to the [`decoration`](Self::decoration) and the
    /// [`child`](Self::child). The width includes any [`padding`](Self::padding).
    pub width: Option<f64>,

    /// A height to apply to the [`decoration`](Self::decoration) and the
    /// [`child`](Self::child). The height includes any [`padding`](Self::padding).
    pub height: Option<f64>,
}

impl fmt::Debug for Ink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ink")
            .field("padding", &self.padding)
            .field("bg", &self.effective_decoration())
            .finish_non_exhaustive()
    }
}

impl Ink {
    /// Paints a decoration (which can be a simple color) on a [`Material`]; Dart's named
    /// arguments are the setters.
    ///
    /// The [`height`](Self::height) and [`width`](Self::width) values include the
    /// [`padding`](Self::padding).
    ///
    /// The `color` argument is a shorthand for
    /// `decoration: BoxDecoration::new().color(color)`, which means you cannot supply
    /// both a `color` and a `decoration` argument. If you want to have both a
    /// `color` and a `decoration`, you can pass the color as the `color`
    /// argument to the `BoxDecoration`.
    ///
    /// If there is no intention to render anything on this decoration, consider
    /// using a `Container` with a `BoxConstraints` instead.
    pub fn new() -> Ink {
        Ink {
            key: None,
            child: None,
            padding: None,
            color: None,
            decoration: None,
            width: None,
            height: None,
        }
    }

    /// Dart `Ink(key:)`.
    pub fn key(mut self, key: KeyRef) -> Ink {
        self.key = Some(key);
        self
    }

    /// Dart `Ink(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> Ink {
        let padding = padding.into();
        debug_assert!(padding.is_non_negative());
        self.padding = Some(padding);
        self
    }

    /// Dart `Ink(color:)`.
    pub fn color(mut self, color: Color) -> Ink {
        self.color = Some(color);
        self.debug_assert_color_or_decoration();
        self
    }

    /// Dart `Ink(decoration:)`.
    pub fn decoration(mut self, decoration: impl Into<DecorationRef>) -> Ink {
        let decoration = decoration.into();
        debug_assert!(decoration.debug_assert_is_valid());
        self.decoration = Some(decoration);
        self.debug_assert_color_or_decoration();
        self
    }

    /// Dart `Ink(width:)`.
    pub fn width(mut self, width: f64) -> Ink {
        self.width = Some(width);
        self
    }

    /// Dart `Ink(height:)`.
    pub fn height(mut self, height: f64) -> Ink {
        self.height = Some(height);
        self
    }

    /// Dart `Ink(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Ink {
        self.child = Some(child.into_widget());
        self
    }

    fn debug_assert_color_or_decoration(&self) {
        debug_assert!(
            self.color.is_none() || self.decoration.is_none(),
            "Cannot provide both a color and a decoration\nThe color argument is just a \
             shorthand for \"decoration: BoxDecoration(color: color)\"."
        );
    }

    /// Dart's `decoration` field, as its constructor fills it.
    pub fn effective_decoration(&self) -> Option<DecorationRef> {
        match (&self.decoration, self.color) {
            (Some(decoration), _) => Some(decoration.clone()),
            (None, Some(color)) => Some(DecorationRef::new(BoxDecoration::new().color(color))),
            (None, None) => None,
        }
    }

    fn padding_including_decoration(&self) -> EdgeInsetsGeometry {
        let decoration_padding = self
            .effective_decoration()
            .map(|decoration| decoration.padding());
        match (self.padding, decoration_padding) {
            (None, None) => EdgeInsets::ZERO.into(),
            (None, Some(padding)) => padding,
            (Some(padding), None) => padding,
            (Some(padding), Some(decoration_padding)) => padding.add(decoration_padding),
        }
    }
}

impl Default for Ink {
    fn default() -> Ink {
        Ink::new()
    }
}

impl StatefulWidget for Ink {
    type State = InkState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> InkState {
        InkState {
            state: StateData::new(),
            box_key: GlobalKey::new(),
            ink: None,
        }
    }
}

/// Dart's `_InkState`.
pub struct InkState {
    state: StateData<Ink>,
    box_key: GlobalKey,
    ink: Option<Ref<InkDecoration>>,
}

impl InkState {
    fn handle_removed(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).ink = None;
    }

    fn build_child(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // By creating the InkDecoration from within a Builder widget, we can
        // use the RenderBox of the Padding widget.
        let decoration = self.widget(app).effective_decoration();
        let is_visible = Visibility::of(app, context);
        let configuration = create_local_image_configuration(app, context, None);
        match app.get(self).ink.clone() {
            None => {
                let controller = Material::of(app, context);
                let box_key = app.get(self).box_key.clone();
                let reference_box = box_key
                    .current_context(app)
                    .expect("the padding is mounted")
                    .find_render_object(app)
                    .and_then(|render_object| render_object.as_box())
                    .expect("the padding is a render box");
                let ink = InkDecoration::new(
                    app,
                    decoration,
                    is_visible,
                    configuration,
                    controller,
                    reference_box,
                    Some(Listener::handle_method(self, Self::handle_removed)),
                );
                app.get_mut(self).ink = Some(ink);
            }
            Some(ink) => {
                ink.set_decoration(app, decoration);
                ink.set_is_visible(app, is_visible);
                ink.set_configuration(app, configuration);
            }
        }
        self.widget(app).child.clone().unwrap_or_else(|| {
            ConstrainedBox::new(BoxConstraints::expand(None, None)).into_widget()
        })
    }
}

impl State for InkState {
    type Widget = Ink;
    waft_widgets::state_accessors!();

    fn deactivate(self: Handle<Self>, app: &mut App) {
        if let Some(ink) = app.get(self).ink.handle() {
            InkFeatureLeaf::dispose(ink, app);
        }
        debug_assert!(app.get(self).ink.is_none());
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        let widget = self.widget(app);
        let padding = widget.padding_including_decoration();
        let (width, height) = (widget.width, widget.height);
        let box_key = app.get(self).box_key.clone();
        let mut result = Padding::new(padding)
            .key(box_key.into())
            .child(Builder::new(move |app, context| {
                self.build_child(app, context)
            }))
            .into_widget();
        if width.is_some() || height.is_some() {
            let mut sized = SizedBox::new();
            if let Some(width) = width {
                sized = sized.width(width);
            }
            if let Some(height) = height {
                sized = sized.height(height);
            }
            result = sized.child(result).into_widget();
        }
        result
    }
}

/// A decoration on a part of a [`Material`].
///
/// This object is rarely created directly. Instead of creating an ink
/// decoration directly, consider using an [`Ink`] widget, which uses this class
/// in combination with [`Padding`] and [`ConstrainedBox`] to draw a decoration on a
/// [`Material`].
///
/// See also:
///
///  * [`Ink`], the corresponding widget.
///  * [`InkResponse`](crate::InkResponse), which uses gestures to trigger ink highlights and
///    ink splashes in the parent [`Material`].
///  * [`InkWell`](crate::InkWell), which is a rectangular [`InkResponse`](crate::InkResponse)
///    (the most common type of ink response).
///  * [`Material`], which is the widget on which the ink is painted.
pub struct InkDecoration {
    ink_feature: InkFeatureData,
    painter: Option<Box<dyn BoxPainter>>,
    decoration: Option<DecorationRef>,
    is_visible: bool,
    configuration: ImageConfiguration,
}

impl InkDecoration {
    /// Draws a decoration on a [`Material`].
    ///
    /// Dart's default `is_visible` is true.
    pub fn new(
        app: &mut App,
        decoration: Option<DecorationRef>,
        is_visible: bool,
        configuration: ImageConfiguration,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        on_removed: Option<Listener>,
    ) -> Ref<InkDecoration> {
        let ink_feature = InkFeatureData::new(app, controller, reference_box, on_removed);
        let this = app.create(InkDecoration {
            ink_feature,
            painter: None,
            decoration: None,
            is_visible: true,
            configuration,
        });
        this.set_decoration(app, decoration);
        this.set_is_visible(app, is_visible);
        controller.add_ink_feature(app, this.erase());
        this
    }

    /// What to paint on the [`Material`].
    ///
    /// The decoration is painted at the position and size of the `reference_box`,
    /// on the [`Material`] that owns the `controller`.
    pub fn decoration(self: Handle<Self>, app: &App) -> Option<&DecorationRef> {
        app.get(self).decoration.as_ref()
    }

    /// Sets [`decoration`](Self::decoration).
    pub fn set_decoration(self: Handle<Self>, app: &mut App, value: Option<DecorationRef>) {
        if value == app.get(self).decoration {
            return;
        }
        let controller = InkFeature::controller(self, app);
        let painter = value.as_ref().map(|decoration| {
            decoration.create_box_painter(Some(Rc::new(move |app: &mut App| {
                controller.mark_needs_paint(app)
            })))
        });
        let ink = app.get_mut(self);
        ink.decoration = value;
        if let Some(mut old_painter) = std::mem::replace(&mut ink.painter, painter) {
            old_painter.dispose();
        }
        controller.mark_needs_paint(app);
    }

    /// Whether the decoration should be painted.
    ///
    /// Defaults to true.
    pub fn is_visible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_visible
    }

    /// Sets [`is_visible`](Self::is_visible).
    pub fn set_is_visible(self: Handle<Self>, app: &mut App, value: bool) {
        if value == app.get(self).is_visible {
            return;
        }
        app.get_mut(self).is_visible = value;
        InkFeature::controller(self, app).mark_needs_paint(app);
    }

    /// The configuration to pass to the `BoxPainter` obtained from the
    /// [`decoration`](Self::decoration), when painting.
    ///
    /// The `ImageConfiguration::size` field is ignored (and replaced by the size
    /// of the `reference_box`, at paint time).
    pub fn configuration(self: Handle<Self>, app: &App) -> &ImageConfiguration {
        &app.get(self).configuration
    }

    /// Sets [`configuration`](Self::configuration).
    pub fn set_configuration(self: Handle<Self>, app: &mut App, value: ImageConfiguration) {
        if value == app.get(self).configuration {
            return;
        }
        app.get_mut(self).configuration = value;
        InkFeature::controller(self, app).mark_needs_paint(app);
    }
}

impl InkFeatureLeafData for InkDecoration {
    fn ink_feature(&self) -> &InkFeatureData {
        &self.ink_feature
    }

    fn ink_feature_mut(&mut self) -> &mut InkFeatureData {
        &mut self.ink_feature
    }
}

impl InkFeatureLeaf for InkDecoration {
    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(mut painter) = app.get_mut(self).painter.take() {
            painter.dispose();
        }
        InkFeature::dispose(self, app);
    }

    fn paint_feature(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, transform: &Matrix4) {
        if app.get(self).painter.is_none() || !self.is_visible(app) {
            return;
        }
        let origin_offset = get_as_translation(*transform);
        let size = InkFeature::reference_box(self, app).size(app);
        let sized_configuration =
            self.configuration(app)
                .copy_with(None, None, None, None, Some(size), None);
        let painter = app.get_mut(self).painter.as_mut().expect("checked above");
        match origin_offset {
            None => {
                canvas.save();
                canvas.concat(transform);
                painter.paint(canvas, Offset::ZERO, &sized_configuration);
                canvas.restore();
            }
            Some(origin_offset) => {
                painter.paint(canvas, origin_offset, &sized_configuration);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use waft_embedder::{Rect, Size, TargetPlatform, valo};
    use waft_widgets::Center;

    use super::*;
    use crate::colors::Colors;
    use crate::ink_well::InkWell;
    use crate::test_support::{Drawn, PaintInspection, WidgetTester, with_main_thread_stack};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;

    fn tester() -> WidgetTester {
        WidgetTester::on(TargetPlatform::Android)
    }

    fn size_of(tester: &WidgetTester, key: &GlobalKey) -> Size {
        tester
            .render_object(key)
            .as_box()
            .expect("a box")
            .size(&tester.cell.borrow())
    }

    #[test]
    fn the_ink_widget_expands_when_no_dimensions_are_set() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(
                Material::new()
                    .child(Ink::new().key(key.clone().into()))
                    .into_widget(),
            );
            assert_eq!(size_of(&tester, &key), Size::new(800.0, 600.0));
        });
    }

    #[test]
    fn the_ink_widget_fits_the_specified_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(
                Material::new()
                    .child(
                        // used to constrain to child's size
                        Center::new().child(
                            Ink::new()
                                .key(key.clone().into())
                                .height(150.0)
                                .width(200.0),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(200.0, 150.0));
        });
    }

    #[test]
    fn the_ink_widget_expands_on_a_unspecified_dimension() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(
                Material::new()
                    .child(
                        // used to constrain to child's size
                        Center::new().child(Ink::new().key(key.clone().into()).height(150.0)),
                    )
                    .into_widget(),
            );
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(800.0, 150.0));
        });
    }

    fn m2_ink_frame(tester: &WidgetTester, ink: Option<Color>) -> WidgetRef {
        let ink_well = InkWell::new()
            .splash_color(Colors::GREEN.primary())
            .on_tap(Listener::new(|_app| {}));
        let child = match ink {
            Some(color) => Ink::new()
                .color(color)
                .width(200.0)
                .height(200.0)
                .child(ink_well)
                .into_widget(),
            None => ink_well.into_widget(),
        };
        let data = ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow());
        Theme::new(
            data,
            waft_widgets::Directionality::new(
                waft_embedder::TextDirection::Ltr,
                Material::new().child(Center::new().child(child)),
            ),
        )
        .into_widget()
    }

    fn op_argb(color: valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    /// Dart's `paints..rect(rect: rect, color: rect_color)..circle(color: circle_color)` on the
    /// material: a `drawRect` of that rect and color, then, later in the list, a `drawCircle` of
    /// that color; the calls between them are skipped.
    fn paints_rect_then_circle(
        tester: &WidgetTester,
        rect_color: Color,
        rect: Rect,
        circle_color: Color,
    ) -> bool {
        let list = tester.ink_list();
        let ops = list.ops();
        let is_rect = |op: &valo::Op| match op {
            valo::Op::DrawRect {
                rect: drawn, paint, ..
            } => {
                op_argb(paint.color) == rect_color.to_argb32()
                    && Rect::from_ltwh(
                        f64::from(drawn.x),
                        f64::from(drawn.y),
                        f64::from(drawn.width),
                        f64::from(drawn.height),
                    ) == rect
            }
            _ => false,
        };
        let is_circle = |op: &valo::Op| match op {
            valo::Op::DrawPath { path, paint, .. } => {
                path.as_oval().is_some() && op_argb(paint.color) == circle_color.to_argb32()
            }
            _ => false,
        };
        ops.iter()
            .position(is_rect)
            .is_some_and(|rect_at| ops[rect_at + 1..].iter().any(is_circle))
    }

    #[test]
    fn material2_does_the_ink_widget_render_anything() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let blue = Colors::BLUE.primary();
            let red = Colors::RED.primary();
            let green = Colors::GREEN.primary();
            let ink_rect = Rect::from_ltrb(300.0, 200.0, 500.0, 400.0);
            let widget = m2_ink_frame(&tester, Some(blue));
            tester.pump_widget(widget);

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, WidgetTester::SIZE.center(Offset::ZERO));
            tester.pump(Duration::ZERO); // start gesture
            tester.pump(Duration::from_millis(200)); // wait for splash to be well under way

            let box_ = tester.first::<crate::material::RenderInkFeatures>();
            assert!(
                paints_rect_then_circle(&tester, blue, ink_rect, green),
                "{:?}",
                tester.ink_drawn()
            );

            let widget = m2_ink_frame(&tester, Some(red));
            tester.pump_widget(widget);
            assert_eq!(tester.first::<crate::material::RenderInkFeatures>(), box_);
            assert!(
                paints_rect_then_circle(&tester, red, ink_rect, green),
                "{:?}",
                tester.ink_drawn()
            );

            // this is at a different depth in the tree so it's now a new InkWell
            let widget = m2_ink_frame(&tester, None);
            tester.pump_widget(widget);
            assert_eq!(tester.first::<crate::material::RenderInkFeatures>(), box_);
            let drawn = tester.ink_drawn();
            assert!(!drawn.iter().any(|drawn| matches!(drawn, Drawn::Rect(..))));
            assert!(!drawn.iter().any(|drawn| matches!(drawn, Drawn::Path(..))));

            tester.up(&gesture);
        });
    }

    #[test]
    fn ink_with_is_visible_false_does_not_paint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let test_color = Color::new(0xffff1234);
            let ink_widget = |is_visible: bool| {
                Material::new()
                    .child(
                        Visibility::maintain(
                            Ink::new().decoration(BoxDecoration::new().color(test_color)),
                        )
                        .visible(is_visible),
                    )
                    .into_widget()
            };
            let has_test_color = |drawn: &[Drawn]| {
                drawn
                    .iter()
                    .any(|drawn| matches!(drawn, Drawn::Rect(color, _) if *color == test_color.to_argb32()))
            };

            tester.pump_widget(ink_widget(true));
            assert!(has_test_color(&tester.ink_drawn()));

            tester.pump_widget(ink_widget(false));
            assert!(!has_test_color(&tester.ink_drawn()));
        });
    }
}
