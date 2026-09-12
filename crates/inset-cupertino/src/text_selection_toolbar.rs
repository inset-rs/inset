//! Flutter counterpart: `cupertino/text_selection_toolbar.dart`.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use inset_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationStatus, AnimationStatusListener,
    Curves,
};
use inset_embedder::valo::{Cap, Join};
use inset_embedder::{
    BlurStyle, Brightness, Canvas, Clip, Color, FillRule, Offset, Paint, PaintStyle, Path,
    PathBuilder, RRect, Radius, Rect, Size, Stroke, clamp_double, rrect_radii_elliptical,
};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::DragEndDetails;
use inset_painting::{BoxShadow, ClipContext, EdgeInsetsGeometry, draw_rrect};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData, ClipPathLayer,
    ContainerBoxParentData, ContainerParentDataMixin, ContainerRenderObjectData,
    ContainerRenderObjectMixin, CustomPainter, ErasedRenderObject, LayerHandle, PaintingContext,
    RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderShiftedBox,
};
use inset_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use inset_widgets::{
    AnimatedSize, AnyElement, BuildContext, Center, ColoredBox, CustomPaint,
    CustomSingleChildLayout, Element, ElementData, FadeTransition, GestureDetector, GlobalKey,
    IgnorePointer, IndexedSlot, IntoWidget, KeyRef, MediaQuery, Padding, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget,
    SingleChildRenderObjectWidget, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, Slot, State, StateData, StatefulWidget, StatelessWidget,
    TextSelectionToolbarLayoutDelegate, ToolbarItemsParentData, Widget, WidgetKind, WidgetRef,
    downcast_widget,
};

use crate::colors::CupertinoDynamicColor;
use crate::text_selection_toolbar_button::CupertinoTextSelectionToolbarButton;
use crate::theme::CupertinoTheme;

const K_TOOLBAR_BORDER_RADIUS: Radius = Radius::circular(8.0);
const K_TOOLBAR_CONTENT_DISTANCE: f64 = 8.0;
const K_TOOLBAR_ARROW_SIZE: Size = Size::new(14.0, 7.0);
const K_ARROW_SCREEN_PADDING: f64 = 26.0;
const K_TOOLBAR_CHEVRON_SIZE: f64 = 10.0;
const K_TOOLBAR_CHEVRON_THICKNESS: f64 = 2.0;

const K_TOOLBAR_BACKGROUND_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFF6F6F6), Color::new(0xFF222222));
const K_TOOLBAR_DIVIDER_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFD6D6D6), Color::new(0xFF424242));
const K_TOOLBAR_TEXT_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFF000000), Color::new(0xFFFFFFFF));

const K_TOOLBAR_TRANSITION_DURATION: Duration = Duration::from_millis(125);

/// The type for a Function that builds a toolbar's container with the given child.
///
/// The anchor is provided in global coordinates.
pub type CupertinoToolbarBuilder =
    Rc<dyn Fn(&mut App, BuildContext, Offset, Offset, WidgetRef) -> WidgetRef>;

/// An iOS-style text selection toolbar.
///
/// Typically displays buttons for text manipulation, e.g. copying and pasting text.
///
/// Tries to position itself above [`anchor_above`](Self::anchor_above), but if it doesn't fit,
/// then it positions itself below [`anchor_below`](Self::anchor_below).
///
/// If any children don't fit in the menu, an overflow menu will automatically be created.
pub struct CupertinoTextSelectionToolbar {
    pub key: Option<KeyRef>,
    /// {@macro flutter.material.TextSelectionToolbar.anchorAbove}
    pub anchor_above: Offset,
    /// {@macro flutter.material.TextSelectionToolbar.anchorBelow}
    pub anchor_below: Offset,
    /// {@macro flutter.material.TextSelectionToolbar.children}
    pub children: Vec<WidgetRef>,
    /// {@macro flutter.material.TextSelectionToolbar.toolbarBuilder}
    pub toolbar_builder: CupertinoToolbarBuilder,
}

impl Debug for CupertinoTextSelectionToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoTextSelectionToolbar")
            .finish_non_exhaustive()
    }
}

impl CupertinoTextSelectionToolbar {
    /// Minimal padding from all edges of the selection toolbar to all edges of the viewport.
    pub const K_TOOLBAR_SCREEN_PADDING: f64 = 8.0;

    /// Creates an instance of [`CupertinoTextSelectionToolbar`].
    pub fn new(
        anchor_above: Offset,
        anchor_below: Offset,
        children: impl IntoIterator<Item = WidgetRef>,
    ) -> CupertinoTextSelectionToolbar {
        let children: Vec<WidgetRef> = children.into_iter().collect();
        debug_assert!(!children.is_empty());
        CupertinoTextSelectionToolbar {
            key: None,
            anchor_above,
            anchor_below,
            children,
            toolbar_builder: Rc::new(Self::default_toolbar_builder),
        }
    }

    /// Dart `CupertinoTextSelectionToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoTextSelectionToolbar {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoTextSelectionToolbar(toolbarBuilder:)`.
    pub fn toolbar_builder(mut self, toolbar_builder: CupertinoToolbarBuilder) -> Self {
        self.toolbar_builder = toolbar_builder;
        self
    }

    fn default_toolbar_builder(
        app: &mut App,
        context: BuildContext,
        anchor_above: Offset,
        anchor_below: Offset,
        child: WidgetRef,
    ) -> WidgetRef {
        let shadow_color = if CupertinoTheme::brightness_of(app, context) == Brightness::Light {
            Some(Color::from_argb(51, 0, 0, 0))
        } else {
            None
        };
        CupertinoTextSelectionToolbarShape {
            key: None,
            anchor_above,
            anchor_below,
            shadow_color,
            child: Some(
                ColoredBox::new(K_TOOLBAR_BACKGROUND_COLOR.resolve_from(app, context))
                    .child(child)
                    .into_widget(),
            ),
        }
        .into_widget()
    }
}

impl StatelessWidget for CupertinoTextSelectionToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let media_query_padding = MediaQuery::padding_of(app, context);
        let padding_above =
            media_query_padding.top + CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING;
        let left_margin = K_ARROW_SCREEN_PADDING + media_query_padding.left;
        let right_margin =
            MediaQuery::width_of(app, context) - media_query_padding.right - K_ARROW_SCREEN_PADDING;
        let anchor_above_adjusted = Offset::new(
            clamp_double(self.anchor_above.dx(), left_margin, right_margin),
            self.anchor_above.dy() - K_TOOLBAR_CONTENT_DISTANCE - padding_above,
        );
        let anchor_below_adjusted = Offset::new(
            clamp_double(self.anchor_below.dx(), left_margin, right_margin),
            self.anchor_below.dy() + K_TOOLBAR_CONTENT_DISTANCE - padding_above,
        );
        Padding::new(EdgeInsetsGeometry::from_ltrb(
            CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING,
            padding_above,
            CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING,
            CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING,
        ))
        .child(
            CustomSingleChildLayout::new(Rc::new(TextSelectionToolbarLayoutDelegate::new(
                anchor_above_adjusted,
                anchor_below_adjusted,
            )))
            .child(
                CupertinoTextSelectionToolbarContent {
                    anchor_above: anchor_above_adjusted,
                    anchor_below: anchor_below_adjusted,
                    toolbar_builder: self.toolbar_builder.clone(),
                    children: self.children.clone(),
                }
                .into_widget(),
            ),
        )
        .into_widget()
    }
}

struct CupertinoTextSelectionToolbarShape {
    key: Option<KeyRef>,
    anchor_above: Offset,
    anchor_below: Offset,
    shadow_color: Option<Color>,
    child: Option<WidgetRef>,
}

impl Debug for CupertinoTextSelectionToolbarShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_CupertinoTextSelectionToolbarShape")
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for CupertinoTextSelectionToolbarShape {
    type RenderObject = RenderCupertinoTextSelectionToolbarShape;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderCupertinoTextSelectionToolbarShape::new(
            app,
            self.anchor_above,
            self.anchor_below,
            self.shadow_color,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCupertinoTextSelectionToolbarShape>,
    ) {
        render_object.set_anchor_above(app, self.anchor_above);
        render_object.set_anchor_below(app, self.anchor_below);
        render_object.set_shadow_color(app, self.shadow_color);
    }
}

impl SingleChildRenderObjectWidget for CupertinoTextSelectionToolbarShape {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

struct RenderCupertinoTextSelectionToolbarShape {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    anchor_above: Offset,
    anchor_below: Offset,
    shadow_color: Option<Color>,
    clip_path_layer: LayerHandle<Handle<ClipPathLayer>>,
}

impl RenderCupertinoTextSelectionToolbarShape {
    fn new(
        app: &mut App,
        anchor_above: Offset,
        anchor_below: Offset,
        shadow_color: Option<Color>,
    ) -> RenderHandle<RenderCupertinoTextSelectionToolbarShape> {
        RenderHandle::new_box(
            app,
            RenderCupertinoTextSelectionToolbarShape {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                anchor_above,
                anchor_below,
                shadow_color,
                clip_path_layer: LayerHandle::new(),
            },
        )
    }

    fn set_anchor_above(self: RenderHandle<Self>, app: &mut App, value: Offset) {
        if value == self.get(app).anchor_above {
            return;
        }
        self.get_mut(app).anchor_above = value;
        self.mark_needs_layout(app);
    }

    fn set_anchor_below(self: RenderHandle<Self>, app: &mut App, value: Offset) {
        if value == self.get(app).anchor_below {
            return;
        }
        self.get_mut(app).anchor_below = value;
        self.mark_needs_layout(app);
    }

    fn set_shadow_color(self: RenderHandle<Self>, app: &mut App, value: Option<Color>) {
        if value == self.get(app).shadow_color {
            return;
        }
        self.get_mut(app).shadow_color = value;
        self.mark_needs_paint(app);
    }

    fn is_above(self: RenderHandle<Self>, app: &App, child_height: f64) -> bool {
        self.get(app).anchor_above.dy() >= child_height - K_TOOLBAR_ARROW_SIZE.height()
    }

    fn constraints_for_child(constraints: BoxConstraints) -> BoxConstraints {
        BoxConstraints::new()
            .min_width(K_TOOLBAR_ARROW_SIZE.width() + K_TOOLBAR_BORDER_RADIUS.x * 2.0)
            .enforce(constraints.loosen())
    }

    fn compute_child_offset(self: RenderHandle<Self>, app: &App, child_size: Size) -> Offset {
        Offset::new(
            0.0,
            if self.is_above(app, child_size.height()) {
                -K_TOOLBAR_ARROW_SIZE.height()
            } else {
                0.0
            },
        )
    }

    fn shape_rrect(child_size: Size) -> RRect {
        let rect = Offset::new(0.0, K_TOOLBAR_ARROW_SIZE.height())
            & Size::new(
                child_size.width(),
                child_size.height() - K_TOOLBAR_ARROW_SIZE.height() * 2.0,
            );
        RRect::from_rect_and_radius(rect, K_TOOLBAR_BORDER_RADIUS).scale_radii()
    }

    fn add_rrect_to_path(path: &mut PathBuilder, rrect: RRect, start_angle: f64) {
        let half_pi = PI / 2.0;
        debug_assert!((start_angle % half_pi).abs() < 1e-9);
        let rect = rrect.outer_rect();
        let rrect_corners = [
            (
                rect.bottom_right(),
                Offset::new(-rrect.br_radius_x, -rrect.br_radius_y),
            ),
            (
                rect.bottom_left(),
                Offset::new(rrect.bl_radius_x, -rrect.bl_radius_y),
            ),
            (
                rect.top_left(),
                Offset::new(rrect.tl_radius_x, rrect.tl_radius_y),
            ),
            (
                rect.top_right(),
                Offset::new(-rrect.tr_radius_x, rrect.tr_radius_y),
            ),
        ];
        let start_quadrant_index = (start_angle / half_pi).trunc() as i32;
        let mut i = start_quadrant_index;
        while i < rrect_corners.len() as i32 + start_quadrant_index {
            let (vertex, rect_center_offset) =
                rrect_corners[i.rem_euclid(rrect_corners.len() as i32) as usize];
            let other_vertex = Offset::new(
                vertex.dx() + 2.0 * rect_center_offset.dx(),
                vertex.dy() + 2.0 * rect_center_offset.dy(),
            );
            let corner = Rect::from_points(vertex, other_vertex);
            path.ellipse(
                corner.center(),
                [
                    (corner.width() / 2.0) as f32,
                    (corner.height() / 2.0) as f32,
                ],
                0.0,
                (half_pi * i as f64) as f32,
                half_pi as f32,
            );
            i += 1;
        }
    }

    fn clip_path(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderBox,
        rrect: RRect,
    ) -> Arc<Path> {
        let mut path = PathBuilder::new();
        let size = self.size(app);
        if K_TOOLBAR_BORDER_RADIUS.x * 2.0 + K_TOOLBAR_ARROW_SIZE.width() > size.width() {
            path.rrect_radii_elliptical(rrect.outer_rect(), rrect_radii_elliptical(rrect));
            return path.build();
        }
        let child_size = child.size(app);
        let is_above = self.is_above(app, child_size.height());
        let local_anchor = self.as_box().global_to_local(
            app,
            if is_above {
                self.get(app).anchor_above
            } else {
                self.get(app).anchor_below
            },
            None,
        );
        let arrow_tip_x = clamp_double(
            local_anchor.dx(),
            K_TOOLBAR_BORDER_RADIUS.x + K_TOOLBAR_ARROW_SIZE.width() / 2.0,
            size.width() - K_TOOLBAR_ARROW_SIZE.width() / 2.0 - K_TOOLBAR_BORDER_RADIUS.x,
        );
        if is_above {
            let arrow_base_y = child_size.height() - K_TOOLBAR_ARROW_SIZE.height();
            let arrow_tip_y = child_size.height();
            path.move_to(Offset::new(
                arrow_tip_x + K_TOOLBAR_ARROW_SIZE.width() / 2.0,
                arrow_base_y,
            ));
            path.line_to(Offset::new(arrow_tip_x, arrow_tip_y));
            path.line_to(Offset::new(
                arrow_tip_x - K_TOOLBAR_ARROW_SIZE.width() / 2.0,
                arrow_base_y,
            ));
        } else {
            let arrow_base_y = K_TOOLBAR_ARROW_SIZE.height();
            let arrow_tip_y = 0.0;
            path.move_to(Offset::new(
                arrow_tip_x - K_TOOLBAR_ARROW_SIZE.width() / 2.0,
                arrow_base_y,
            ));
            path.line_to(Offset::new(arrow_tip_x, arrow_tip_y));
            path.line_to(Offset::new(
                arrow_tip_x + K_TOOLBAR_ARROW_SIZE.width() / 2.0,
                arrow_base_y,
            ));
        }
        let start_angle = if is_above { PI / 2.0 } else { -PI / 2.0 };
        Self::add_rrect_to_path(&mut path, rrect, start_angle);
        path.close();
        path.build()
    }
}

impl RenderObjectWithChildMixin for RenderCupertinoTextSelectionToolbarShape {
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

impl RenderShiftedBox for RenderCupertinoTextSelectionToolbarShape {}

impl RenderObject for RenderCupertinoTextSelectionToolbarShape {
    inset_rendering::render_object_accessors!();

    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let Some(child) = self.child(app) else {
            return;
        };
        let constraints = Self::constraints_for_child(self.constraints(app));
        child.layout(app, constraints, true);
        let child_size = child.size(app);
        let child_offset = self.compute_child_offset(app, child_size);
        child
            .as_object()
            .parent_data_of_mut::<BoxParentData>(app)
            .offset = child_offset;
        self.set_size(
            app,
            Size::new(
                child_size.width(),
                child_size.height() - K_TOOLBAR_ARROW_SIZE.height(),
            ),
        );
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            return;
        };
        let child_parent_data_offset = child
            .as_object()
            .parent_data_of::<BoxParentData>(app)
            .offset;
        let rrect = Self::shape_rrect(child.size(app));
        let clip_path = self.clip_path(app, child, rrect);
        if let Some(shadow_color) = self.get(app).shadow_color {
            let box_shadow =
                BoxShadow::new(shadow_color, Offset::ZERO, 15.0, 0.0, BlurStyle::Normal);
            let shadow_rrect = RRect::from_ltrbr(
                rrect.left,
                rrect.top,
                rrect.right,
                rrect.bottom + K_TOOLBAR_ARROW_SIZE.height(),
                K_TOOLBAR_BORDER_RADIUS,
            )
            .shift(offset + child_parent_data_offset + box_shadow.offset);
            draw_rrect(context.canvas(), shadow_rrect, &box_shadow.to_paint());
        }
        let needs_compositing = self.as_object().needs_compositing(app);
        let old = self.get(app).clip_path_layer.layer();
        let layer = context.push_clip_path(
            app,
            needs_compositing,
            offset + child_parent_data_offset,
            Offset::ZERO & child.size(app),
            clip_path,
            |app, inner_context, inner_offset| {
                inner_context.paint_child(app, child.as_object(), inner_offset);
            },
            Clip::AntiAlias,
            old,
        );
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_path_layer, layer);
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

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_path_layer, None);
        inset_rendering::RenderObjectBase::dispose(self, app);
    }
}

impl RenderBox for RenderCupertinoTextSelectionToolbarShape {
    inset_rendering::render_box_accessors!();

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: inset_embedder::TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let enforced = Self::constraints_for_child(constraints);
        let result = child.get_dry_baseline(app, enforced, baseline)?;
        let dry_size = child.get_dry_layout(app, enforced);
        Some(result + self.compute_child_offset(app, dry_size).dy())
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };
        let child_parent_data_offset = child
            .as_object()
            .parent_data_of::<BoxParentData>(app)
            .offset;
        let hit_box = Rect::from_ltwh(
            child_parent_data_offset.dx(),
            child_parent_data_offset.dy() + K_TOOLBAR_ARROW_SIZE.height(),
            child.size(app).width(),
            child.size(app).height() - K_TOOLBAR_ARROW_SIZE.height() * 2.0,
        );
        if !hit_box.contains(position) {
            return false;
        }
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }
}

struct CupertinoTextSelectionToolbarContent {
    anchor_above: Offset,
    anchor_below: Offset,
    toolbar_builder: CupertinoToolbarBuilder,
    children: Vec<WidgetRef>,
}

impl Debug for CupertinoTextSelectionToolbarContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_CupertinoTextSelectionToolbarContent")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoTextSelectionToolbarContent {
    type State = CupertinoTextSelectionToolbarContentState;

    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_state(&self) -> CupertinoTextSelectionToolbarContentState {
        debug_assert!(!self.children.is_empty());
        CupertinoTextSelectionToolbarContentState {
            state: StateData::new(),
            controller: None,
            next_page: None,
            page: 0,
            toolbar_items_key: GlobalKey::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
        }
    }
}

struct CupertinoTextSelectionToolbarContentState {
    state: StateData<CupertinoTextSelectionToolbarContent>,
    controller: Option<Handle<AnimationController>>,
    next_page: Option<i32>,
    page: i32,
    toolbar_items_key: GlobalKey,
    single_ticker_provider: SingleTickerProviderStateMixinData,
}

impl CupertinoTextSelectionToolbarContentState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .controller
            .expect("the controller is created in initState")
    }

    fn on_horizontal_drag_end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        let Some(velocity) = details.primary_velocity else {
            return;
        };
        if velocity == 0.0 {
            return;
        }
        if velocity > 0.0 {
            self.handle_previous_page(app);
        } else {
            self.handle_next_page(app);
        }
    }

    fn handle_next_page(self: Handle<Self>, app: &mut App) {
        let Some(context) = app.get(self).toolbar_items_key.clone().current_context(app) else {
            return;
        };
        let Some(render_toolbar) = context.find_render_object(app) else {
            return;
        };
        let Some(render_toolbar) =
            render_toolbar.downcast::<RenderCupertinoTextSelectionToolbarItems>(app)
        else {
            return;
        };
        if render_toolbar.has_next_page(app) {
            let controller = self.controller(app);
            controller.reverse(app, None);
            controller.add_status_listener(
                app,
                AnimationStatusListener::handle_method(self, Self::status_listener),
            );
            app.get_mut(self).next_page = Some(app.get(self).page + 1);
        }
    }

    fn handle_previous_page(self: Handle<Self>, app: &mut App) {
        let Some(context) = app.get(self).toolbar_items_key.clone().current_context(app) else {
            return;
        };
        let Some(render_toolbar) = context.find_render_object(app) else {
            return;
        };
        let Some(render_toolbar) =
            render_toolbar.downcast::<RenderCupertinoTextSelectionToolbarItems>(app)
        else {
            return;
        };
        if render_toolbar.has_previous_page(app) {
            let controller = self.controller(app);
            controller.reverse(app, None);
            controller.add_status_listener(
                app,
                AnimationStatusListener::handle_method(self, Self::status_listener),
            );
            app.get_mut(self).next_page = Some(app.get(self).page - 1);
        }
    }

    fn status_listener(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if !status.is_dismissed() {
            return;
        }
        let next_page = app.get(self).next_page.expect("set before reverse");
        self.set_state(app, |state| {
            state.page = next_page;
            state.next_page = None;
        });
        let controller = self.controller(app);
        controller.forward(app, None);
        controller.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::status_listener),
        );
    }

    fn children_equal(a: &[WidgetRef], b: &[WidgetRef]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(left, right)| Rc::ptr_eq(left, right))
    }
}

impl SingleTickerProviderStateMixin for CupertinoTextSelectionToolbarContentState {
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

impl TickerProviderObject for CupertinoTextSelectionToolbarContentState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoTextSelectionToolbarContentState {
    type Widget = CupertinoTextSelectionToolbarContent;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::create(
            app,
            Some(1.0),
            Some(K_TOOLBAR_TRANSITION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &CupertinoTextSelectionToolbarContent,
    ) {
        if !Self::children_equal(&self.widget(app).children, &old_widget.children) {
            app.get_mut(self).page = 0;
            app.get_mut(self).next_page = None;
            let controller = self.controller(app);
            controller.forward(app, None);
            controller.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self, Self::status_listener),
            );
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        app.get_mut(self).controller = None;
        app.destroy(controller);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let chevron_color = K_TOOLBAR_TEXT_COLOR
            .resolve_from(app, context)
            .effective_color();
        let back_button = Center::new()
            .width_factor(1.0)
            .height_factor(1.0)
            .child(CupertinoTextSelectionToolbarButton::new(
                Some(Listener::handle_method(self, Self::handle_previous_page)),
                IgnorePointer::new().child(
                    CustomPaint::new()
                        .painter(LeftCupertinoChevronPainter {
                            color: chevron_color,
                        })
                        .size(Size::square(K_TOOLBAR_CHEVRON_SIZE)),
                ),
            ))
            .into_widget();
        let next_button = Center::new()
            .width_factor(1.0)
            .height_factor(1.0)
            .child(CupertinoTextSelectionToolbarButton::new(
                Some(Listener::handle_method(self, Self::handle_next_page)),
                IgnorePointer::new().child(
                    CustomPaint::new()
                        .painter(RightCupertinoChevronPainter {
                            color: chevron_color,
                        })
                        .size(Size::square(K_TOOLBAR_CHEVRON_SIZE)),
                ),
            ))
            .into_widget();
        let children: Vec<WidgetRef> = self
            .widget(app)
            .children
            .iter()
            .cloned()
            .map(|child| {
                Center::new()
                    .width_factor(1.0)
                    .height_factor(1.0)
                    .child(child)
                    .into_widget()
            })
            .collect();
        let widget = self.widget(app);
        let (anchor_above, anchor_below, toolbar_builder) = (
            widget.anchor_above,
            widget.anchor_below,
            widget.toolbar_builder.clone(),
        );
        let page = app.get(self).page;
        let toolbar_items_key = app.get(self).toolbar_items_key.clone();
        let opacity = self.controller(app).view();
        let this = self;
        let divider_color = K_TOOLBAR_DIVIDER_COLOR
            .resolve_from(app, context)
            .effective_color();
        let divider_width = 1.0 / MediaQuery::device_pixel_ratio_of(app, context);
        (toolbar_builder)(
            app,
            context,
            anchor_above,
            anchor_below,
            FadeTransition::new(opacity)
                .child(
                    AnimatedSize::new(K_TOOLBAR_TRANSITION_DURATION)
                        .curve(Curves::decelerate())
                        .child(
                            GestureDetector::new()
                                .on_horizontal_drag_end(Rc::new(move |app, details| {
                                    this.on_horizontal_drag_end(app, details);
                                }))
                                .child(
                                    CupertinoTextSelectionToolbarItems {
                                        key: Some(Rc::new(toolbar_items_key)),
                                        page,
                                        back_button,
                                        divider_color,
                                        divider_width,
                                        next_button,
                                        children,
                                    }
                                    .into_widget(),
                                ),
                        ),
                )
                .into_widget(),
        )
    }
}

struct LeftCupertinoChevronPainter {
    color: Color,
}

struct RightCupertinoChevronPainter {
    color: Color,
}

impl CustomPainter for LeftCupertinoChevronPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        paint_chevron(canvas, size, self.color, true);
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<LeftCupertinoChevronPainter>()
            .is_none_or(|old| old.color != self.color)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl CustomPainter for RightCupertinoChevronPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        paint_chevron(canvas, size, self.color, false);
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<RightCupertinoChevronPainter>()
            .is_none_or(|old| old.color != self.color)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn paint_chevron(canvas: &mut Canvas, size: Size, color: Color, is_left: bool) {
    debug_assert_eq!(
        size.height(),
        size.width(),
        "size must have the same height and width: {size:?}"
    );
    let icon_size = size.height();
    let center_offset = Offset::new(icon_size / 4.0 * if is_left { 1.0 } else { -1.0 }, 0.0);
    let first_point = Offset::new(icon_size / 2.0, 0.0) + center_offset;
    let middle_point =
        Offset::new(if is_left { 0.0 } else { icon_size }, icon_size / 2.0) + center_offset;
    let lower_point = Offset::new(icon_size / 2.0, icon_size) + center_offset;
    let paint = Paint {
        color: color.into(),
        style: PaintStyle::Stroke(Stroke {
            width: K_TOOLBAR_CHEVRON_THICKNESS as f32,
            cap: Cap::Round,
            join: Join::Round,
            miter_limit: 4.0,
            dash: None,
        }),
        ..Paint::default()
    };
    let mut path = PathBuilder::new();
    path.move_to(first_point);
    path.line_to(middle_point);
    path.move_to(middle_point);
    path.line_to(lower_point);
    canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
}

struct CupertinoTextSelectionToolbarItems {
    key: Option<KeyRef>,
    page: i32,
    children: Vec<WidgetRef>,
    back_button: WidgetRef,
    divider_color: Color,
    divider_width: f64,
    next_button: WidgetRef,
}

impl Debug for CupertinoTextSelectionToolbarItems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_CupertinoTextSelectionToolbarItems")
            .finish_non_exhaustive()
    }
}

impl CupertinoTextSelectionToolbarItems {
    fn into_widget(self) -> WidgetRef {
        debug_assert!(!self.children.is_empty());
        Rc::new(self)
    }
}

impl Widget for CupertinoTextSelectionToolbarItems {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        CupertinoTextSelectionToolbarItemsElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<CupertinoTextSelectionToolbarItems>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl RenderObjectWidget for CupertinoTextSelectionToolbarItems {
    type RenderObject = RenderCupertinoTextSelectionToolbarItems;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderCupertinoTextSelectionToolbarItems::new(
            app,
            self.divider_color,
            self.divider_width,
            self.page,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCupertinoTextSelectionToolbarItems>,
    ) {
        render_object.set_page(app, self.page);
        render_object.set_divider_color(app, self.divider_color);
        render_object.set_divider_width(app, self.divider_width);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CupertinoTextSelectionToolbarItemsSlot {
    BackButton,
    NextButton,
}

struct CupertinoTextSelectionToolbarItemsElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    children: Vec<AnyElement>,
    slot_to_child: HashMap<CupertinoTextSelectionToolbarItemsSlot, AnyElement>,
    forgotten_children: HashSet<AnyElement>,
    back_button_slot: Rc<dyn Any>,
    next_button_slot: Rc<dyn Any>,
}

impl CupertinoTextSelectionToolbarItemsElement {
    fn create(
        app: &mut App,
        widget: WidgetRef,
    ) -> Handle<CupertinoTextSelectionToolbarItemsElement> {
        debug_assert!(downcast_widget::<CupertinoTextSelectionToolbarItems>(&*widget).is_some());
        app.create(CupertinoTextSelectionToolbarItemsElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            children: Vec::new(),
            slot_to_child: HashMap::new(),
            forgotten_children: HashSet::new(),
            back_button_slot: Rc::new(CupertinoTextSelectionToolbarItemsSlot::BackButton),
            next_button_slot: Rc::new(CupertinoTextSelectionToolbarItemsSlot::NextButton),
        })
    }

    fn toolbar_items_slot(
        self: Handle<Self>,
        app: &App,
        slot: Option<&Slot>,
    ) -> Option<CupertinoTextSelectionToolbarItemsSlot> {
        let Some(Slot::Custom(rc)) = slot else {
            return None;
        };
        let this = app.get(self);
        if Rc::ptr_eq(rc, &this.back_button_slot) {
            Some(CupertinoTextSelectionToolbarItemsSlot::BackButton)
        } else if Rc::ptr_eq(rc, &this.next_button_slot) {
            Some(CupertinoTextSelectionToolbarItemsSlot::NextButton)
        } else {
            None
        }
    }

    fn update_render_object_slot(
        self: Handle<Self>,
        app: &mut App,
        child: Option<AnyRenderBox>,
        slot: CupertinoTextSelectionToolbarItemsSlot,
    ) {
        let render_object = self.typed_render_object(app);
        match slot {
            CupertinoTextSelectionToolbarItemsSlot::BackButton => {
                render_object.set_back_button(app, child);
            }
            CupertinoTextSelectionToolbarItemsSlot::NextButton => {
                render_object.set_next_button(app, child);
            }
        }
    }

    fn mount_child(
        self: Handle<Self>,
        app: &mut App,
        widget: WidgetRef,
        slot: CupertinoTextSelectionToolbarItemsSlot,
    ) {
        let slot_rc = match slot {
            CupertinoTextSelectionToolbarItemsSlot::BackButton => {
                app.get(self).back_button_slot.clone()
            }
            CupertinoTextSelectionToolbarItemsSlot::NextButton => {
                app.get(self).next_button_slot.clone()
            }
        };
        let old_child = app.get(self).slot_to_child.get(&slot).copied();
        let new_child = self.as_element().update_child(
            app,
            old_child,
            Some(widget),
            Some(Slot::Custom(slot_rc)),
        );
        if old_child.is_some() {
            app.get_mut(self).slot_to_child.remove(&slot);
        }
        if let Some(new_child) = new_child {
            app.get_mut(self).slot_to_child.insert(slot, new_child);
        }
    }
}

impl RenderObjectElementWidget for CupertinoTextSelectionToolbarItemsElement {
    type Widget = CupertinoTextSelectionToolbarItems;

    fn widget_of(widget: &WidgetRef) -> &CupertinoTextSelectionToolbarItems {
        downcast_widget::<CupertinoTextSelectionToolbarItems>(&**widget)
            .expect("a _CupertinoTextSelectionToolbarItemsElement holds its widget")
    }
}

impl RenderObjectElement for CupertinoTextSelectionToolbarItemsElement {
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl Element for CupertinoTextSelectionToolbarItemsElement {
    inset_widgets::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        for child in app.get(self).slot_to_child.values().copied() {
            visitor(child);
        }
        let forgotten = &app.get(self).forgotten_children;
        for child in &app.get(self).children {
            if !forgotten.contains(child) {
                visitor(*child);
            }
        }
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        let slot = child.slot(app);
        if self.toolbar_items_slot(app, slot.as_ref()).is_some() {
            if let Some(kind) = self.toolbar_items_slot(app, slot.as_ref()) {
                app.get_mut(self).slot_to_child.remove(&kind);
            }
        } else {
            app.get_mut(self).forgotten_children.insert(child);
        }
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
        let toolbar_items = Self::widget_of(self.as_element().widget(app));
        let back_button = toolbar_items.back_button.clone();
        let next_button = toolbar_items.next_button.clone();
        let child_widgets = toolbar_items.children.clone();
        self.mount_child(
            app,
            back_button,
            CupertinoTextSelectionToolbarItemsSlot::BackButton,
        );
        self.mount_child(
            app,
            next_button,
            CupertinoTextSelectionToolbarItemsSlot::NextButton,
        );
        let mut previous_child: Option<AnyElement> = None;
        let mut children = Vec::with_capacity(child_widgets.len());
        for (index, child_widget) in child_widgets.into_iter().enumerate() {
            let result = self.as_element().inflate_widget(
                app,
                child_widget,
                Some(Slot::Indexed(IndexedSlot {
                    index,
                    value: previous_child,
                })),
            );
            previous_child = Some(result);
            children.push(result);
        }
        app.get_mut(self).children = children;
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        RenderObjectElement::update(self, app, new_widget);
        let toolbar_items = Self::widget_of(self.as_element().widget(app));
        let back_button = toolbar_items.back_button.clone();
        let next_button = toolbar_items.next_button.clone();
        let child_widgets = toolbar_items.children.clone();
        self.mount_child(
            app,
            back_button,
            CupertinoTextSelectionToolbarItemsSlot::BackButton,
        );
        self.mount_child(
            app,
            next_button,
            CupertinoTextSelectionToolbarItemsSlot::NextButton,
        );
        let old_children = app.get(self).children.clone();
        let is_forgotten = |app: &App, child: AnyElement| {
            self.as_element();
            app.get(self).forgotten_children.contains(&child)
        };
        let children = self.as_element().update_children(
            app,
            &old_children,
            &child_widgets,
            Some(&is_forgotten),
            None,
        );
        let this = app.get_mut(self);
        this.children = children;
        this.forgotten_children.clear();
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        if let Some(kind) = self.toolbar_items_slot(app, slot.as_ref()) {
            self.update_render_object_slot(app, Some(AnyRenderBox::from_object(child)), kind);
            return;
        }
        if matches!(slot, Some(Slot::Indexed(_))) {
            let render_object = self.typed_render_object(app);
            let after = match slot.as_ref() {
                Some(Slot::Indexed(indexed)) => indexed
                    .value
                    .and_then(|element| element.render_object(app))
                    .map(AnyRenderBox::from_object),
                _ => None,
            };
            render_object.insert(app, AnyRenderBox::from_object(child), after);
            return;
        }
        panic!("slot must be _CupertinoTextSelectionToolbarItemsSlot or IndexedSlot");
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        debug_assert!(matches!(new_slot, Some(Slot::Indexed(_))));
        let render_object = self.typed_render_object(app);
        debug_assert!(child.parent(app) == Some(RenderObjectElement::render_object(self, app)));
        let after = match new_slot.as_ref() {
            Some(Slot::Indexed(indexed)) => indexed
                .value
                .and_then(|element| element.render_object(app))
                .map(AnyRenderBox::from_object),
            _ => None,
        };
        render_object.move_child(app, AnyRenderBox::from_object(child), after);
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        if let Some(kind) = self.toolbar_items_slot(app, slot.as_ref()) {
            self.update_render_object_slot(app, None, kind);
            return;
        }
        debug_assert!(matches!(slot, Some(Slot::Indexed(_))));
        let render_object = self.typed_render_object(app);
        debug_assert!(child.parent(app) == Some(RenderObjectElement::render_object(self, app)));
        render_object.remove(app, AnyRenderBox::from_object(child));
    }
}

struct RenderCupertinoTextSelectionToolbarItems {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    slotted_children: HashMap<CupertinoTextSelectionToolbarItemsSlot, AnyRenderBox>,
    has_next_page: bool,
    has_previous_page: bool,
    page: i32,
    divider_color: Color,
    divider_width: f64,
    back_button: Option<AnyRenderBox>,
    next_button: Option<AnyRenderBox>,
}

impl RenderCupertinoTextSelectionToolbarItems {
    fn new(
        app: &mut App,
        divider_color: Color,
        divider_width: f64,
        page: i32,
    ) -> RenderHandle<RenderCupertinoTextSelectionToolbarItems> {
        RenderHandle::new_box(
            app,
            RenderCupertinoTextSelectionToolbarItems {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                slotted_children: HashMap::new(),
                has_next_page: false,
                has_previous_page: false,
                page,
                divider_color,
                divider_width,
                back_button: None,
                next_button: None,
            },
        )
    }

    fn has_next_page(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).has_next_page
    }

    fn has_previous_page(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).has_previous_page
    }

    fn set_page(self: RenderHandle<Self>, app: &mut App, value: i32) {
        if value == self.get(app).page {
            return;
        }
        self.get_mut(app).page = value;
        self.mark_needs_layout(app);
    }

    fn set_divider_color(self: RenderHandle<Self>, app: &mut App, value: Color) {
        if value == self.get(app).divider_color {
            return;
        }
        self.get_mut(app).divider_color = value;
        self.mark_needs_layout(app);
    }

    fn set_divider_width(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if value == self.get(app).divider_width {
            return;
        }
        self.get_mut(app).divider_width = value;
        self.mark_needs_layout(app);
    }

    fn update_child(
        self: RenderHandle<Self>,
        app: &mut App,
        old_child: Option<AnyRenderBox>,
        new_child: Option<AnyRenderBox>,
        slot: CupertinoTextSelectionToolbarItemsSlot,
    ) -> Option<AnyRenderBox> {
        if let Some(old_child) = old_child {
            self.drop_child(app, old_child.as_object());
            self.get_mut(app).slotted_children.remove(&slot);
        }
        if let Some(new_child) = new_child {
            self.get_mut(app).slotted_children.insert(slot, new_child);
            self.adopt_child(app, new_child.as_object());
        }
        new_child
    }

    fn set_back_button(self: RenderHandle<Self>, app: &mut App, value: Option<AnyRenderBox>) {
        let old = self.get(app).back_button;
        let child = self.update_child(
            app,
            old,
            value,
            CupertinoTextSelectionToolbarItemsSlot::BackButton,
        );
        self.get_mut(app).back_button = child;
    }

    fn set_next_button(self: RenderHandle<Self>, app: &mut App, value: Option<AnyRenderBox>) {
        let old = self.get(app).next_button;
        let child = self.update_child(
            app,
            old,
            value,
            CupertinoTextSelectionToolbarItemsSlot::NextButton,
        );
        self.get_mut(app).next_button = child;
    }

    fn hit_test_child(
        child: Option<AnyRenderBox>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = child else {
            return false;
        };
        let child_parent_data = child
            .as_object()
            .parent_data_of::<ToolbarItemsParentData>(app);
        if !child_parent_data.should_paint {
            return false;
        }
        let offset = child_parent_data.offset();
        result.add_with_paint_offset(Some(offset), position, |result, transformed| {
            debug_assert_eq!(transformed, position - offset);
            child.hit_test(app, result, transformed)
        })
    }
}

impl ContainerRenderObjectMixin for RenderCupertinoTextSelectionToolbarItems {
    type ChildType = AnyRenderBox;
    type ParentDataType = ToolbarItemsParentData;

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

impl RenderObject for RenderCupertinoTextSelectionToolbarItems {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        if self.first_child(app).is_none() {
            self.set_size(app, self.constraints(app).smallest());
            return;
        }
        let constraints = self.constraints(app);
        let mut visited = Vec::new();
        RenderObject::visit_children(self, app, &mut |child| visited.push(child));
        let mut greatest_height = 0.0;
        for child in &visited {
            let child = AnyRenderBox::from_object(*child);
            let child_height = child.get_max_intrinsic_height(app, constraints.max_width);
            if child_height > greatest_height {
                greatest_height = child_height;
            }
        }
        let slotted_constraints = BoxConstraints::new()
            .max_width(constraints.max_width)
            .min_height(greatest_height)
            .max_height(greatest_height);
        let back_button = self.get(app).back_button.expect("mounted");
        let next_button = self.get(app).next_button.expect("mounted");
        back_button.layout(app, slotted_constraints, true);
        next_button.layout(app, slotted_constraints, true);
        let subsequent_page_buttons_width =
            back_button.size(app).width() + next_button.size(app).width();
        let mut current_button_position = 0.0;
        let mut toolbar_width = 0.0;
        let mut current_page = 0_i32;
        let page = self.get(app).page;
        let divider_width = self.get(app).divider_width;
        let child_count = self.child_count(app) as i32;
        let mut i = -1_i32;
        for render_object_child in visited {
            i += 1;
            let child = AnyRenderBox::from_object(render_object_child);
            child
                .as_object()
                .parent_data_of_mut::<ToolbarItemsParentData>(app)
                .should_paint = false;
            if child == back_button || child == next_button || current_page > page {
                continue;
            }
            let mut pagination_buttons_width = if current_page == 0 {
                if i == child_count + 1 {
                    0.0
                } else {
                    next_button.size(app).width()
                }
            } else {
                subsequent_page_buttons_width
            };
            child.layout(
                app,
                BoxConstraints::new()
                    .max_width(constraints.max_width - pagination_buttons_width)
                    .min_height(greatest_height)
                    .max_height(greatest_height),
                true,
            );
            let current_width =
                current_button_position + pagination_buttons_width + child.size(app).width();
            if current_width > constraints.max_width {
                current_page += 1;
                current_button_position = back_button.size(app).width() + divider_width;
                pagination_buttons_width =
                    back_button.size(app).width() + next_button.size(app).width();
                child.layout(
                    app,
                    BoxConstraints::new()
                        .max_width(constraints.max_width - pagination_buttons_width)
                        .min_height(greatest_height)
                        .max_height(greatest_height),
                    true,
                );
            }
            child
                .as_object()
                .parent_data_of_mut::<ToolbarItemsParentData>(app)
                .set_offset(Offset::new(current_button_position, 0.0));
            current_button_position += child.size(app).width() + divider_width;
            child
                .as_object()
                .parent_data_of_mut::<ToolbarItemsParentData>(app)
                .should_paint = current_page == page;
            if current_page == page {
                toolbar_width = current_button_position;
            }
        }
        debug_assert!(page <= current_page);
        if current_page > 0 {
            if page != current_page {
                next_button
                    .as_object()
                    .parent_data_of_mut::<ToolbarItemsParentData>(app)
                    .set_offset(Offset::new(toolbar_width, 0.0));
                next_button
                    .as_object()
                    .parent_data_of_mut::<ToolbarItemsParentData>(app)
                    .should_paint = true;
                toolbar_width += next_button.size(app).width();
            }
            if page > 0 {
                back_button
                    .as_object()
                    .parent_data_of_mut::<ToolbarItemsParentData>(app)
                    .set_offset(Offset::ZERO);
                back_button
                    .as_object()
                    .parent_data_of_mut::<ToolbarItemsParentData>(app)
                    .should_paint = true;
            }
        } else {
            toolbar_width -= divider_width;
        }
        self.get_mut(app).has_next_page = page != current_page;
        self.get_mut(app).has_previous_page = page > 0;
        self.set_size(
            app,
            constraints.constrain(Size::new(toolbar_width, greatest_height)),
        );
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let mut visited = Vec::new();
        RenderObject::visit_children(self, app, &mut |child| visited.push(child));
        let divider_color = self.get(app).divider_color;
        let back_button = self.get(app).back_button;
        for render_object_child in visited {
            let child = AnyRenderBox::from_object(render_object_child);
            let child_parent_data = child
                .as_object()
                .parent_data_of::<ToolbarItemsParentData>(app);
            if !child_parent_data.should_paint {
                continue;
            }
            let child_offset = child_parent_data.offset() + offset;
            let next_sibling = child_parent_data.next_sibling();
            context.paint_child(app, child.as_object(), child_offset);
            if next_sibling.is_some() || Some(child) == back_button {
                let size = child.size(app);
                let paint = Paint {
                    color: divider_color.into(),
                    ..Paint::default()
                };
                let mut path = PathBuilder::new();
                path.move_to(Offset::new(size.width(), 0.0) + child_offset);
                path.line_to(Offset::new(size.width(), size.height()) + child_offset);
                context
                    .canvas()
                    .draw_path(&path.build(), FillRule::NonZero, &paint);
            }
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(back_button) = self.get(app).back_button {
            visitor(back_button.as_object());
        }
        if let Some(next_button) = self.get(app).next_button {
            visitor(next_button.as_object());
        }
        ContainerRenderObjectMixin::visit_children(self, app, visitor);
    }

    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: Handle<inset_rendering::PipelineOwner>,
    ) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
        let slotted: Vec<AnyRenderBox> = self.get(app).slotted_children.values().copied().collect();
        for child in slotted {
            child.as_object().attach(app, owner);
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app);
        let slotted: Vec<AnyRenderBox> = self.get(app).slotted_children.values().copied().collect();
        for child in slotted {
            child.as_object().detach(app);
        }
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        let mut children = Vec::new();
        RenderObject::visit_children(self, app, &mut |child| children.push(child));
        for child in children {
            self.as_object().redepth_child(app, child);
        }
    }
}

impl RenderBox for RenderCupertinoTextSelectionToolbarItems {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<ToolbarItemsParentData>(app) {
            child.set_parent_data(app, ToolbarItemsParentData::new());
        }
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let mut child = self.last_child(app);
        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of::<ToolbarItemsParentData>(app);
            if !child_parent_data.should_paint {
                child = child_parent_data.previous_sibling();
                continue;
            }
            if Self::hit_test_child(Some(current), app, result, position) {
                return true;
            }
            child = current
                .as_object()
                .parent_data_of::<ToolbarItemsParentData>(app)
                .previous_sibling();
        }
        if Self::hit_test_child(self.get(app).back_button, app, result, position) {
            return true;
        }
        if Self::hit_test_child(self.get(app).next_button, app, result, position) {
            return true;
        }
        false
    }
}
