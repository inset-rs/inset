//! Flutter counterpart: material_ui's `text_selection_toolbar.dart`.
//!
//! Dart's `_TextSelectionToolbarItemsLayoutElement` exists only to override
//! `debugVisitOnstageChildren`, which Waft's `Element` has not got: the items layout takes the
//! default `MultiChildRenderObjectElement`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_cupertino::CupertinoTextSelectionToolbar;
use waft_embedder::{Brightness, Clip, Color, Matrix4, Offset, Radius, Size, TextBaseline};
use waft_foundation::{App, Handle, Listener, UniqueKey};
use waft_painting::{BorderRadius, EdgeInsetsGeometry, TextDirection};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, ContainerBoxParentData,
    ContainerParentDataMixin, ContainerRenderObjectData, ContainerRenderObjectMixin,
    PaintingContext, PipelineOwner, RenderBox, RenderBoxData, RenderHandle, RenderObject,
    RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
};
use waft_widgets::AnyElement;
use waft_widgets::{
    AnimatedSize, BuildContext, CustomSingleChildLayout, Directionality, Icon, IntoWidget, KeyRef,
    MediaQuery, MultiChildRenderObjectWidget, Padding, RenderObjectWidget,
    SingleChildRenderObjectWidget, StandardComponentType, State, StateData, StatefulWidget,
    StatelessWidget, TextSelectionToolbarLayoutDelegate, ToolbarBuilder, ToolbarItemsParentData,
    WidgetRef,
};

use crate::color_scheme::ColorScheme;
use crate::debug::debug_check_has_material_localizations;
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

const K_TOOLBAR_HEIGHT: f64 = 44.0;
const K_TOOLBAR_CONTENT_DISTANCE: f64 = 8.0;

/// A fully-functional Material-style text selection toolbar.
///
/// Tries to position itself above [`anchor_above`](Self::anchor_above), but if it doesn't fit,
/// then it positions itself below [`anchor_below`](Self::anchor_below).
///
/// If any children don't fit in the menu, an overflow menu will automatically
/// be created.
///
/// See also:
///
///  * `AdaptiveTextSelectionToolbar`, which builds the toolbar for the current
///    platform.
///  * [`CupertinoTextSelectionToolbar`], which is similar, but builds an iOS-
///    style toolbar.
pub struct TextSelectionToolbar {
    pub key: Option<KeyRef>,
    /// The focal point above which the toolbar attempts to position itself.
    ///
    /// If there is not enough room above before reaching the top of the screen,
    /// then the toolbar will position itself below
    /// [`anchor_below`](Self::anchor_below).
    pub anchor_above: Offset,
    /// The focal point below which the toolbar attempts to position itself, if
    /// it doesn't fit above [`anchor_above`](Self::anchor_above).
    pub anchor_below: Offset,
    /// The children that will be displayed in the text selection toolbar.
    ///
    /// Typically these are buttons.
    ///
    /// Must not be empty.
    ///
    /// See also:
    ///   * `TextSelectionToolbarTextButton`, which builds a default Material-
    ///     style text selection toolbar text button.
    pub children: Vec<WidgetRef>,
    /// Builds the toolbar container.
    ///
    /// Useful for customizing the high-level background of the toolbar. The
    /// given child Widget will contain all of the children.
    pub toolbar_builder: ToolbarBuilder,
}

impl Debug for TextSelectionToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSelectionToolbar")
            .field("anchor_above", &self.anchor_above)
            .field("anchor_below", &self.anchor_below)
            .finish_non_exhaustive()
    }
}

impl TextSelectionToolbar {
    /// The size of the text selection handles.
    ///
    /// See also:
    ///
    ///  * `SpellCheckSuggestionsToolbar`, which references this value to calculate
    ///    the padding between the toolbar and anchor.
    pub const K_HANDLE_SIZE: f64 = 22.0;

    /// Padding between the toolbar and the anchor.
    pub const K_TOOLBAR_CONTENT_DISTANCE_BELOW: f64 = Self::K_HANDLE_SIZE - 2.0;

    /// Creates an instance of TextSelectionToolbar.
    pub fn new(
        anchor_above: Offset,
        anchor_below: Offset,
        children: impl IntoIterator<Item = WidgetRef>,
    ) -> TextSelectionToolbar {
        let children: Vec<WidgetRef> = children.into_iter().collect();
        debug_assert!(!children.is_empty());
        TextSelectionToolbar {
            key: None,
            anchor_above,
            anchor_below,
            children,
            toolbar_builder: Rc::new(Self::default_toolbar_builder),
        }
    }

    /// Dart `TextSelectionToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextSelectionToolbar {
        self.key = Some(key);
        self
    }

    /// Dart `TextSelectionToolbar(toolbarBuilder:)`.
    pub fn toolbar_builder(mut self, toolbar_builder: ToolbarBuilder) -> TextSelectionToolbar {
        self.toolbar_builder = toolbar_builder;
        self
    }

    // Build the default Android Material text selection menu toolbar.
    fn default_toolbar_builder(
        _app: &mut App,
        _context: BuildContext,
        child: WidgetRef,
    ) -> WidgetRef {
        TextSelectionToolbarContainer { child }.into_widget()
    }
}

impl StatelessWidget for TextSelectionToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        // Incorporate the padding distance between the content and toolbar.
        let anchor_above_padded = self.anchor_above - Offset::new(0.0, K_TOOLBAR_CONTENT_DISTANCE);
        let anchor_below_padded =
            self.anchor_below + Offset::new(0.0, Self::K_TOOLBAR_CONTENT_DISTANCE_BELOW);

        const SCREEN_PADDING: f64 = CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING;
        let padding_above = MediaQuery::padding_of(app, context).top + SCREEN_PADDING;
        let available_height =
            anchor_above_padded.dy() - K_TOOLBAR_CONTENT_DISTANCE - padding_above;
        let fits_above = K_TOOLBAR_HEIGHT <= available_height;
        // Makes up for the Padding above the Stack.
        let local_adjustment = Offset::new(SCREEN_PADDING, padding_above);

        Padding::new(EdgeInsetsGeometry::from_ltrb(
            SCREEN_PADDING,
            padding_above,
            SCREEN_PADDING,
            SCREEN_PADDING,
        ))
        .child(
            CustomSingleChildLayout::new(Rc::new(
                TextSelectionToolbarLayoutDelegate::new(
                    anchor_above_padded - local_adjustment,
                    anchor_below_padded - local_adjustment,
                )
                .fits_above(fits_above),
            ))
            .child(TextSelectionToolbarOverflowable {
                is_above: fits_above,
                toolbar_builder: self.toolbar_builder.clone(),
                children: self.children.clone(),
            }),
        )
        .into_widget()
    }
}

// A toolbar containing the given children. If they overflow the width
// available, then the overflowing children will be displayed in an overflow
// menu.
struct TextSelectionToolbarOverflowable {
    children: Vec<WidgetRef>,
    // When true, the toolbar fits above its anchor and will be positioned there.
    is_above: bool,
    // Builds the toolbar that will be populated with the children and fit inside
    // of the layout that adjusts to overflow.
    toolbar_builder: ToolbarBuilder,
}

impl Debug for TextSelectionToolbarOverflowable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_TextSelectionToolbarOverflowable")
            .field("is_above", &self.is_above)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for TextSelectionToolbarOverflowable {
    type State = TextSelectionToolbarOverflowableState;

    fn create_state(&self) -> TextSelectionToolbarOverflowableState {
        debug_assert!(!self.children.is_empty());
        TextSelectionToolbarOverflowableState {
            state: StateData::new(),
            overflow_open: false,
            container_key: UniqueKey::new().into(),
        }
    }
}

struct TextSelectionToolbarOverflowableState {
    state: StateData<TextSelectionToolbarOverflowable>,
    // Whether or not the overflow menu is open. When it is closed, the menu
    // items that don't overflow are shown. When it is open, only the overflowing
    // menu items are shown.
    overflow_open: bool,
    // The key for _TextSelectionToolbarTrailingEdgeAlign.
    container_key: KeyRef,
}

impl TextSelectionToolbarOverflowableState {
    // Close the menu and reset layout calculations, as in when the menu has
    // changed and saved values are no longer relevant. This should be called in
    // setState or another context where a rebuild is happening.
    fn reset(self: Handle<Self>, app: &mut App) {
        let this = app.get_mut(self);
        // Change _TextSelectionToolbarTrailingEdgeAlign's key when the menu changes
        // in order to cause it to rebuild. This lets it recalculate its
        // saved width for the new set of children, and it prevents AnimatedSize
        // from animating the size change.
        this.container_key = UniqueKey::new().into();
        // If the menu items change, make sure the overflow menu is closed. This
        // prevents getting into a broken state where _overflowOpen is true when
        // there are not enough children to cause overflow.
        this.overflow_open = false;
    }

    /// Dart's `listEquals` over widgets, which compare by identity.
    fn children_equal(a: &[WidgetRef], b: &[WidgetRef]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(left, right)| Rc::ptr_eq(left, right))
    }
}

impl State for TextSelectionToolbarOverflowableState {
    type Widget = TextSelectionToolbarOverflowable;
    waft_widgets::state_accessors!();

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &TextSelectionToolbarOverflowable,
    ) {
        // If the children are changing at all, the current page should be reset.
        if !Self::children_equal(&self.widget(app).children, &old_widget.children) {
            self.reset(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let text_direction = Directionality::of(app, context);

        let overflow_open = app.get(self).overflow_open;
        let container_key = app.get(self).container_key.clone();
        let is_above = self.widget(app).is_above;
        let toolbar_builder = self.widget(app).toolbar_builder.clone();
        let children = self.widget(app).children.clone();

        let mut items: Vec<WidgetRef> = Vec::with_capacity(children.len() + 1);
        // TODO(justinmc): This overflow button should have its own slot in
        // _TextSelectionToolbarItemsLayout separate from children, similar
        // to how it's done in Cupertino's text selection menu.
        // https://github.com/flutter/flutter/issues/69908
        // The navButton that shows and hides the overflow menu is the
        // first child.
        items.push(
            TextSelectionToolbarOverflowButton {
                key: Some(if overflow_open {
                    StandardComponentType::BackButton.key()
                } else {
                    StandardComponentType::MoreButton.key()
                }),
                icon: Icon::new(Some(if overflow_open {
                    Icons::arrow_back()
                } else {
                    Icons::more_vert()
                }))
                .into_widget(),
                on_pressed: Some(Listener::new(move |app| {
                    self.set_state(app, |state| {
                        state.overflow_open = !state.overflow_open;
                    });
                })),
                tooltip: Some(if overflow_open {
                    localizations.back_button_tooltip()
                } else {
                    localizations.more_button_tooltip()
                }),
            }
            .into_widget(),
        );
        items.extend(children);

        let toolbar = toolbar_builder(
            app,
            context,
            TextSelectionToolbarItemsLayout {
                is_above,
                overflow_open,
                text_direction,
                children: items,
            }
            .into_widget(),
        );
        TextSelectionToolbarTrailingEdgeAlign {
            key: Some(container_key),
            overflow_open,
            text_direction,
            // This duration was eyeballed on a Pixel 2 emulator running Android
            // API 28.
            child: AnimatedSize::new(Duration::from_millis(140))
                .child(toolbar)
                .into_widget(),
        }
        .into_widget()
    }
}

// When the overflow menu is open, it tries to align its trailing edge to the
// trailing edge of the closed menu. This widget handles this effect by
// measuring and maintaining the width of the closed menu and aligning the child
// to that side.
#[derive(Debug)]
struct TextSelectionToolbarTrailingEdgeAlign {
    key: Option<KeyRef>,
    child: WidgetRef,
    overflow_open: bool,
    text_direction: TextDirection,
}

impl RenderObjectWidget for TextSelectionToolbarTrailingEdgeAlign {
    type RenderObject = TextSelectionToolbarTrailingEdgeAlignRenderBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        TextSelectionToolbarTrailingEdgeAlignRenderBox::new(
            app,
            self.overflow_open,
            self.text_direction,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<TextSelectionToolbarTrailingEdgeAlignRenderBox>,
    ) {
        render_object.set_overflow_open(app, self.overflow_open);
        render_object.set_text_direction(app, self.text_direction);
    }
}

impl SingleChildRenderObjectWidget for TextSelectionToolbarTrailingEdgeAlign {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

pub(crate) struct TextSelectionToolbarTrailingEdgeAlignRenderBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    // The width of the menu when it was closed. This is used to achieve the
    // behavior where the open menu aligns its trailing edge to the closed menu's
    // trailing edge.
    closed_width: Option<f64>,
    overflow_open: bool,
    text_direction: TextDirection,
}

impl TextSelectionToolbarTrailingEdgeAlignRenderBox {
    fn new(
        app: &mut App,
        overflow_open: bool,
        text_direction: TextDirection,
    ) -> RenderHandle<TextSelectionToolbarTrailingEdgeAlignRenderBox> {
        RenderHandle::new_box(
            app,
            TextSelectionToolbarTrailingEdgeAlignRenderBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                closed_width: None,
                overflow_open,
                text_direction,
            },
        )
    }

    fn overflow_open(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).overflow_open
    }

    fn set_overflow_open(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.overflow_open(app) {
            return;
        }
        self.get_mut(app).overflow_open = value;
        self.mark_needs_layout(app);
    }

    fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if value == self.text_direction(app) {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_layout(app);
    }

    fn child_offset(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> Offset {
        child
            .as_object()
            .parent_data_of::<ToolbarItemsParentData>(app)
            .offset()
    }
}

impl RenderObjectWithChildMixin for TextSelectionToolbarTrailingEdgeAlignRenderBox {
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

impl RenderProxyBoxMixin for TextSelectionToolbarTrailingEdgeAlignRenderBox {}

impl RenderObject for TextSelectionToolbarTrailingEdgeAlignRenderBox {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let child = self.child(app).expect("the toolbar's child");
        let constraints = self.constraints(app);
        child.layout(app, constraints.loosen(), true);
        let child_size = child.size(app);

        // Save the width when the menu is closed. If the menu changes, this width
        // is invalid, so it's important that this RenderBox be recreated in that
        // case. Currently, this is achieved by providing a new key to
        // _TextSelectionToolbarTrailingEdgeAlign.
        if !self.overflow_open(app) && self.get(app).closed_width.is_none() {
            self.get_mut(app).closed_width = Some(child_size.width());
        }

        let closed_width = self.get(app).closed_width;
        let size = constraints.constrain(Size::new(
            // If the open menu is wider than the closed menu, just use its own width
            // and don't worry about aligning the trailing edges.
            // _closedWidth is used even when the menu is closed to allow it to
            // animate its size while keeping the same edge alignment.
            match closed_width {
                Some(closed_width) if child_size.width() <= closed_width => closed_width,
                _ => child_size.width(),
            },
            child_size.height(),
        ));
        self.set_size(app, size);

        // Set the offset in the parent data such that the child will be aligned to
        // the trailing edge, depending on the text direction.
        let dx = if self.text_direction(app) == TextDirection::Rtl {
            0.0
        } else {
            size.width() - child_size.width()
        };
        child
            .as_object()
            .parent_data_of_mut::<ToolbarItemsParentData>(app)
            .set_offset(Offset::new(dx, 0.0));
    }

    // Paint at the offset set in the parent data.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let child = self.child(app).expect("the toolbar's child");
        let child_offset = self.child_offset(app, child);
        context.paint_child(app, child.as_object(), child_offset + offset);
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

impl RenderBox for TextSelectionToolbarTrailingEdgeAlignRenderBox {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<ToolbarItemsParentData>(app) {
            child.set_parent_data(app, ToolbarItemsParentData::new());
        }
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let offset = child.parent_data_of::<ToolbarItemsParentData>(app).offset();
        *transform = transform.then(&Matrix4::translation(
            offset.dx() as f32,
            offset.dy() as f32,
        ));
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    // Include the parent data offset in the hit test.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        // The x, y parameters have the top left of the node's box as the origin.
        let child = self.child(app).expect("the toolbar's child");
        let child_offset = self.child_offset(app, child);
        result.add_with_paint_offset(Some(child_offset), position, |result, transformed| {
            debug_assert_eq!(transformed, position - child_offset);
            child.hit_test(app, result, transformed)
        })
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

// Renders the menu items in the correct positions in the menu and its overflow
// submenu based on calculating which item would first overflow.
#[derive(Debug)]
struct TextSelectionToolbarItemsLayout {
    is_above: bool,
    overflow_open: bool,
    text_direction: TextDirection,
    children: Vec<WidgetRef>,
}

/// Dart's `_TextSelectionToolbarItemsLayoutElement._shouldPaint`.
fn should_paint(app: &App, child: AnyElement) -> bool {
    child
        .render_object(app)
        .and_then(|render| render.parent_data(app))
        .and_then(|data| data.part::<ToolbarItemsParentData>())
        .is_some_and(|data| data.should_paint)
}

impl RenderObjectWidget for TextSelectionToolbarItemsLayout {
    type RenderObject = RenderTextSelectionToolbarItemsLayout;

    /// Dart's `_TextSelectionToolbarItemsLayoutElement.debugVisitOnstageChildren`: the children
    /// the layout paints.
    fn debug_visit_onstage_children(
        &self,
        app: &App,
        children: &[AnyElement],
        visitor: &mut dyn FnMut(AnyElement),
    ) {
        children
            .iter()
            .filter(|child| should_paint(app, **child))
            .for_each(|child| visitor(*child));
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderTextSelectionToolbarItemsLayout::new(
            app,
            self.is_above,
            self.overflow_open,
            self.text_direction,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderTextSelectionToolbarItemsLayout>,
    ) {
        render_object.set_is_above(app, self.is_above);
        render_object.set_text_direction(app, self.text_direction);
        render_object.set_overflow_open(app, self.overflow_open);
    }
}

impl MultiChildRenderObjectWidget for TextSelectionToolbarItemsLayout {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_RenderTextSelectionToolbarItemsLayout`: crate-visible so the test finders can
/// skip the children it does not paint, as Dart's element does for them.
pub(crate) struct RenderTextSelectionToolbarItemsLayout {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    // The index of the last item that doesn't overflow.
    last_index_that_fits: i32,
    is_above: bool,
    overflow_open: bool,
    text_direction: TextDirection,
}

impl RenderTextSelectionToolbarItemsLayout {
    fn new(
        app: &mut App,
        is_above: bool,
        overflow_open: bool,
        text_direction: TextDirection,
    ) -> RenderHandle<RenderTextSelectionToolbarItemsLayout> {
        RenderHandle::new_box(
            app,
            RenderTextSelectionToolbarItemsLayout {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                last_index_that_fits: -1,
                is_above,
                overflow_open,
                text_direction,
            },
        )
    }

    fn is_above(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).is_above
    }

    fn set_is_above(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.is_above(app) {
            return;
        }
        self.get_mut(app).is_above = value;
        self.mark_needs_layout(app);
    }

    fn overflow_open(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).overflow_open
    }

    fn set_overflow_open(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.overflow_open(app) {
            return;
        }
        self.get_mut(app).overflow_open = value;
        self.mark_needs_layout(app);
    }

    fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if value == self.text_direction(app) {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_layout(app);
    }

    fn parent_data_of(app: &App, child: AnyRenderBox) -> &ToolbarItemsParentData {
        child
            .as_object()
            .parent_data_of::<ToolbarItemsParentData>(app)
    }

    fn parent_data_of_mut(app: &mut App, child: AnyRenderBox) -> &mut ToolbarItemsParentData {
        child
            .as_object()
            .parent_data_of_mut::<ToolbarItemsParentData>(app)
    }

    // Layout the necessary children, and figure out where the children first
    // overflow, if at all.
    fn layout_children(self: RenderHandle<Self>, app: &mut App) {
        // When overflow is not open, the toolbar is always a specific height.
        let constraints = self.constraints(app);
        let overflow_open = self.overflow_open(app);
        let sized_constraints = if overflow_open {
            constraints
        } else {
            BoxConstraints::loose(Size::new(constraints.max_width, K_TOOLBAR_HEIGHT))
        };

        let mut i = -1;
        let mut width = 0.0;
        for child in self.children_as_list(app) {
            i += 1;

            // No need to layout children inside the overflow menu when it's closed.
            // The opposite is not true. It is necessary to layout the children that
            // don't overflow when the overflow menu is open in order to calculate
            // _lastIndexThatFits.
            if self.get(app).last_index_that_fits != -1 && !overflow_open {
                continue;
            }

            child.layout(app, sized_constraints.loosen(), true);
            width += child.size(app).width();

            if width > sized_constraints.max_width && self.get(app).last_index_that_fits == -1 {
                self.get_mut(app).last_index_that_fits = i - 1;
            }
        }

        // If the last child overflows, but only because of the width of the
        // overflow button, then just show it and hide the overflow button.
        let nav_button = self.first_child(app).expect("the overflow button");
        let last_index_that_fits = self.get(app).last_index_that_fits;
        if last_index_that_fits != -1
            && last_index_that_fits == self.child_count(app) as i32 - 2
            && width - nav_button.size(app).width() <= sized_constraints.max_width
        {
            self.get_mut(app).last_index_that_fits = -1;
        }
    }

    // Returns true when the child should be painted, false otherwise.
    fn should_paint_child(
        self: RenderHandle<Self>,
        app: &App,
        render_object_child: AnyRenderBox,
        index: i32,
    ) -> bool {
        let last_index_that_fits = self.get(app).last_index_that_fits;
        // Paint the navButton when there is overflow.
        if Some(render_object_child) == self.first_child(app) {
            return last_index_that_fits != -1;
        }

        // If there is no overflow, all children besides the navButton are painted.
        if last_index_that_fits == -1 {
            return true;
        }

        // When there is overflow, paint if the child is in the part of the menu
        // that is currently open. Overflowing children are painted when the
        // overflow menu is open, and the children that fit are painted when the
        // overflow menu is closed.
        (index > last_index_that_fits) == self.overflow_open(app)
    }

    /// Horizontal layout.
    fn place_children_horizontally(self: RenderHandle<Self>, app: &mut App) -> Size {
        let nav_button = self.first_child(app).expect("the overflow button");
        let is_rtl = self.text_direction(app) == TextDirection::Rtl;

        let mut content_items: Vec<AnyRenderBox> = Vec::new();

        let mut total_width = 0.0;
        let mut max_height: f64 = 0.0;

        // First pass: calculate dimensions and collect items.
        let mut i = -1;
        for child in self.children_as_list(app) {
            i += 1;

            if !self.should_paint_child(app, child, i) {
                // There is no need to update children that won't be painted.
                Self::parent_data_of_mut(app, child).should_paint = false;
            } else {
                Self::parent_data_of_mut(app, child).should_paint = true;

                total_width += child.size(app).width();
                max_height = max_height.max(child.size(app).height());

                if child != nav_button {
                    content_items.push(child);
                }
            }
        }

        // Position items based on text direction.
        let mut current_x = 0.0;
        let show_nav_button = self.get(app).last_index_that_fits >= 0;

        if is_rtl {
            // In RTL, we want the nav button on the left and items right-aligned.
            if show_nav_button {
                Self::parent_data_of_mut(app, nav_button).set_offset(Offset::ZERO);
            }

            // Position content items from right to left.
            let mut right_edge = total_width;
            for item in content_items {
                right_edge -= item.size(app).width();
                Self::parent_data_of_mut(app, item).set_offset(Offset::new(right_edge, 0.0));
            }
        } else {
            // LTR: Place content items first, then nav button.
            // First position all content items from left to right.
            for item in content_items {
                Self::parent_data_of_mut(app, item).set_offset(Offset::new(current_x, 0.0));
                current_x += item.size(app).width();
            }

            // Then place the nav button at the end.
            if show_nav_button {
                Self::parent_data_of_mut(app, nav_button).set_offset(Offset::new(current_x, 0.0));
            }
        }

        Size::new(total_width, max_height)
    }

    /// Vertical layout (overflow menu).
    fn place_children_vertically(self: RenderHandle<Self>, app: &mut App) -> Size {
        let nav_button = self.first_child(app).expect("the overflow button");

        let mut current_y = 0.0;
        let mut max_width: f64 = 0.0;

        if self.should_paint_child(app, nav_button, 0) {
            Self::parent_data_of_mut(app, nav_button).should_paint = true;
            if !self.is_above(app) {
                Self::parent_data_of_mut(app, nav_button).set_offset(Offset::ZERO);
                current_y += nav_button.size(app).height();
                max_width = max_width.max(nav_button.size(app).width());
            }
        } else {
            Self::parent_data_of_mut(app, nav_button).should_paint = false;
        }

        let mut i = -1;
        for child in self.children_as_list(app) {
            i += 1;

            // Ignore the navigation button.
            if child == nav_button {
                continue;
            }

            // There is no need to update children that won't be painted.
            if !self.should_paint_child(app, child, i) {
                Self::parent_data_of_mut(app, child).should_paint = false;
                continue;
            }

            let child_parent_data = Self::parent_data_of_mut(app, child);
            child_parent_data.should_paint = true;
            child_parent_data.set_offset(Offset::new(0.0, current_y));
            current_y += child.size(app).height();
            max_width = max_width.max(child.size(app).width());
        }

        if self.is_above(app) && Self::parent_data_of(app, nav_button).should_paint {
            Self::parent_data_of_mut(app, nav_button).set_offset(Offset::new(0.0, current_y));
            current_y += nav_button.size(app).height();
            max_width = max_width.max(nav_button.size(app).width());
        }

        Size::new(max_width, current_y)
    }

    // Decide which children will be painted, set their shouldPaint, and set the
    // offset that painted children will be placed at.
    fn place_children(self: RenderHandle<Self>, app: &mut App) {
        let size = if self.overflow_open(app) {
            self.place_children_vertically(app)
        } else {
            self.place_children_horizontally(app)
        };
        self.set_size(app, size);
    }

    // Horizontally expand the children when the menu overflows so they can react to
    // pointer events into their whole area.
    fn resize_children_when_overflow(self: RenderHandle<Self>, app: &mut App) {
        if !self.overflow_open(app) {
            return;
        }

        let nav_button = self.first_child(app).expect("the overflow button");
        let mut i = -1;

        for child in self.children_as_list(app) {
            i += 1;

            // Ignore the navigation button.
            if child == nav_button {
                continue;
            }

            // There is no need to update children that won't be painted.
            if !self.should_paint_child(app, child, i) {
                Self::parent_data_of_mut(app, child).should_paint = false;
                continue;
            }

            let width = self.size(app).width();
            child.layout(app, BoxConstraints::tight_for(Some(width), None), true);
        }
    }
}

impl ContainerRenderObjectMixin for RenderTextSelectionToolbarItemsLayout {
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

impl RenderObject for RenderTextSelectionToolbarItemsLayout {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).last_index_that_fits = -1;
        if self.first_child(app).is_none() {
            let smallest = self.constraints(app).smallest();
            self.set_size(app, smallest);
            return;
        }

        self.layout_children(app);
        self.place_children(app);
        self.resize_children_when_overflow(app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        for child in self.children_as_list(app) {
            let child_parent_data = Self::parent_data_of(app, child);
            if !child_parent_data.should_paint {
                continue;
            }

            let child_offset = child_parent_data.offset();
            context.paint_child(app, child.as_object(), child_offset + offset);
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor);
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app);
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app);
    }
}

impl RenderBox for RenderTextSelectionToolbarItemsLayout {
    waft_rendering::render_box_accessors!();

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
            // The x, y parameters have the top left of the node's box as the origin.
            let child_parent_data = Self::parent_data_of(app, current);

            // Don't hit test children aren't shown.
            if !child_parent_data.should_paint {
                child = child_parent_data.previous_sibling();
                continue;
            }

            let offset = child_parent_data.offset();
            let is_hit =
                result.add_with_paint_offset(Some(offset), position, |result, transformed| {
                    debug_assert_eq!(transformed, position - offset);
                    current.hit_test(app, result, transformed)
                });
            if is_hit {
                return true;
            }
            child = Self::parent_data_of(app, current).previous_sibling();
        }
        false
    }
}

// The Material-styled toolbar outline. Fill it with any widgets you want. No
// overflow ability.
#[derive(Debug)]
struct TextSelectionToolbarContainer {
    child: WidgetRef,
}

impl TextSelectionToolbarContainer {
    // These colors were taken from a screenshot of a Pixel 6 emulator running
    // Android API level 34.
    const DEFAULT_COLOR_LIGHT: Color = Color::new(0xffffffff);
    const DEFAULT_COLOR_DARK: Color = Color::new(0xff424242);

    fn get_color(app: &App, color_scheme: &ColorScheme) -> Color {
        let is_default_surface = match color_scheme.brightness {
            Brightness::Light => ThemeData::new(app).color_scheme.surface == color_scheme.surface,
            Brightness::Dark => {
                ThemeData::dark(app, None).color_scheme.surface == color_scheme.surface
            }
        };
        if !is_default_surface {
            return color_scheme.surface;
        }
        match color_scheme.brightness {
            Brightness::Light => Self::DEFAULT_COLOR_LIGHT,
            Brightness::Dark => Self::DEFAULT_COLOR_DARK,
        }
    }
}

impl StatelessWidget for TextSelectionToolbarContainer {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let color_scheme = Theme::of(app, context).color_scheme.clone();
        Material::new()
            // This value was eyeballed to match the native text selection menu on
            // a Pixel 6 emulator running Android API level 34.
            .border_radius(BorderRadius::all(Radius::circular(K_TOOLBAR_HEIGHT / 2.0)))
            .clip_behavior(Clip::AntiAlias)
            .color(Self::get_color(app, &color_scheme))
            .elevation(1.0)
            .r#type(MaterialType::Card)
            .child(self.child.clone())
            .into_widget()
    }
}

// A button styled like a Material native Android text selection overflow menu
// forward and back controls.
#[derive(Debug)]
struct TextSelectionToolbarOverflowButton {
    key: Option<KeyRef>,
    // An `Icon`.
    icon: WidgetRef,
    on_pressed: Option<Listener>,
    tooltip: Option<String>,
}

impl StatelessWidget for TextSelectionToolbarOverflowButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        // TODO(justinmc): This should be an AnimatedIcon, but
        // AnimatedIcons doesn't yet support arrow_back to more_vert.
        // https://github.com/flutter/flutter/issues/51209
        let mut icon_button = IconButton::new(self.on_pressed.clone(), self.icon.clone());
        if let Some(tooltip) = self.tooltip.clone() {
            icon_button = icon_button.tooltip(tooltip);
        }
        Material::new()
            .r#type(MaterialType::Card)
            .color(Color::new(0x00000000))
            .child(icon_button)
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use waft_embedder::Rect;
    use waft_foundation::ValueKey;
    use waft_widgets::{
        AnyElement, Center, Container, SizedBox, StateSetter, StatefulBuilder, Text,
        downcast_widget,
    };

    use waft_widgets::testing::{
        WidgetTester, expect, find, finds_n_widgets, finds_nothing, finds_one_widget, test_widgets,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::text_selection_toolbar_text_button::TextSelectionToolbarTextButton;

    /// Dart's `TestBox`: a `SizedBox` of `ITEM_WIDTH` by `ITEM_HEIGHT`.
    #[derive(Debug)]
    struct TestBox {
        key: Option<KeyRef>,
        child: Option<WidgetRef>,
    }

    impl TestBox {
        const ITEM_HEIGHT: f64 = 44.0;
        const ITEM_WIDTH: f64 = 100.0;

        fn new() -> TestBox {
            TestBox {
                key: None,
                child: None,
            }
        }
    }

    impl StatelessWidget for TestBox {
        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            let sized = SizedBox::new()
                .width(Self::ITEM_WIDTH)
                .height(Self::ITEM_HEIGHT);
            match self.child.clone() {
                Some(child) => sized.child(child).into_widget(),
                None => sized.into_widget(),
            }
        }
    }

    /// `MaterialApp(home: Scaffold(body: body))` under the default theme.
    fn pump_in_scaffold(tester: &mut WidgetTester, body: WidgetRef) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::new(&app),
                Scaffold::new().body(body).into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    /// A `StatefulBuilder` building a toolbar of `children`, and its setter once built.
    fn toolbar_of(
        children: Rc<RefCell<Vec<WidgetRef>>>,
        anchor_above: Rc<RefCell<Offset>>,
        anchor_below: Offset,
    ) -> (WidgetRef, Rc<RefCell<Option<StateSetter>>>) {
        let set_state: Rc<RefCell<Option<StateSetter>>> = Rc::new(RefCell::new(None));
        let builder = {
            let set_state = Rc::clone(&set_state);
            StatefulBuilder::new(move |_app, _context, setter| {
                *set_state.borrow_mut() = Some(setter);
                TextSelectionToolbar::new(
                    *anchor_above.borrow(),
                    anchor_below,
                    children.borrow().clone(),
                )
                .into_widget()
            })
            .into_widget()
        };
        (builder, set_state)
    }

    /// Runs `update` through the builder's `setState`, then Dart's `pumpAndSettle`.
    fn set_state_and_settle(
        tester: &mut WidgetTester,
        set_state: &Rc<RefCell<Option<StateSetter>>>,
        update: impl FnOnce(),
    ) {
        let setter = set_state.borrow().clone().expect("the builder built");
        let mut update = Some(update);
        setter(&mut tester.cell.borrow_mut(), &mut |_app| {
            if let Some(update) = update.take() {
                update();
            }
        });
        tester.pump_and_settle();
    }

    /// Dart's `findOverflowButton()`.
    fn find_overflow_button(tester: &WidgetTester) -> Vec<AnyElement> {
        find::by_type::<TextSelectionToolbarOverflowButton>().evaluate(tester)
    }

    /// Dart's `tester.tap(findOverflowButton())` and `pumpAndSettle`.
    fn tap_overflow_button(tester: &mut WidgetTester) {
        let buttons = find_overflow_button(tester);
        assert_eq!(buttons.len(), 1, "one overflow button");
        let center = tester.get_rect(buttons[0]).center();
        tester.tap_at(center, 1);
        tester.pump_and_settle();
    }

    /// Dart's `tester.getRect(finder)` of the first element found.
    fn rect_of(tester: &WidgetTester, elements: &[AnyElement]) -> Rect {
        tester.get_rect(elements[0])
    }

    fn test_boxes(count: usize) -> Vec<WidgetRef> {
        (0..count).map(|_| TestBox::new().into_widget()).collect()
    }

    fn overflows_into_a_menu(tester: &mut WidgetTester, text_direction: TextDirection) {
        let children = Rc::new(RefCell::new(test_boxes(7)));
        let (toolbar, set_state) = toolbar_of(
            Rc::clone(&children),
            Rc::new(RefCell::new(Offset::new(50.0, 100.0))),
            Offset::new(50.0, 200.0),
        );
        pump_in_scaffold(
            tester,
            Directionality::new(text_direction, toolbar).into_widget(),
        );

        // All children fit on the screen, so they are all rendered.
        let count = children.borrow().len();
        assert_eq!(find::by_type::<TestBox>().evaluate(tester).len(), count);
        assert!(find_overflow_button(tester).is_empty());

        // Adding one more child makes the children overflow.
        set_state_and_settle(tester, &set_state, || {
            children.borrow_mut().push(TestBox::new().into_widget());
        });
        let count = children.borrow().len();
        assert_eq!(find::by_type::<TestBox>().evaluate(tester).len(), count - 1);
        assert_eq!(find_overflow_button(tester).len(), 1);

        // Tap the overflow button to show the overflow menu.
        tap_overflow_button(tester);
        expect(tester, &find::by_type::<TestBox>(), finds_one_widget());
        assert_eq!(find_overflow_button(tester).len(), 1);

        // Tap the overflow button again to hide the overflow menu.
        tap_overflow_button(tester);
        assert_eq!(find::by_type::<TestBox>().evaluate(tester).len(), count - 1);
        assert_eq!(find_overflow_button(tester).len(), 1);
    }

    #[test]
    fn puts_children_in_an_overflow_menu_if_they_overflow() {
        test_widgets(|tester| overflows_into_a_menu(tester, TextDirection::Ltr));
    }

    #[test]
    fn positions_itself_at_anchor_above_if_it_fits() {
        test_widgets(|tester| {
            const HEIGHT: f64 = 44.0;
            const ANCHOR_BELOW_Y: f64 = 500.0;
            let anchor_above = Rc::new(RefCell::new(Offset::new(50.0, 0.0)));
            let children = Rc::new(RefCell::new(vec![
                Container::new()
                    .color(Colors::RED.primary())
                    .width(50.0)
                    .height(HEIGHT)
                    .into_widget(),
                Container::new()
                    .color(Colors::GREEN.primary())
                    .width(50.0)
                    .height(HEIGHT)
                    .into_widget(),
                Container::new()
                    .color(Colors::BLUE.primary())
                    .width(50.0)
                    .height(HEIGHT)
                    .into_widget(),
            ]));
            let (toolbar, set_state) = toolbar_of(
                children,
                Rc::clone(&anchor_above),
                Offset::new(50.0, ANCHOR_BELOW_Y),
            );
            pump_in_scaffold(tester, toolbar);
            let toolbar_y = |tester: &WidgetTester| {
                rect_of(
                    tester,
                    &find::by_type::<TextSelectionToolbarOverflowable>().evaluate(tester),
                )
                .top
            };

            // When the toolbar doesn't fit above aboveAnchor, it positions itself below
            // belowAnchor.
            assert_eq!(
                toolbar_y(tester),
                ANCHOR_BELOW_Y + TextSelectionToolbar::K_TOOLBAR_CONTENT_DISTANCE_BELOW
            );

            // Even when it barely doesn't fit.
            set_state_and_settle(tester, &set_state, || {
                *anchor_above.borrow_mut() = Offset::new(50.0, 60.0);
            });
            assert_eq!(
                toolbar_y(tester),
                ANCHOR_BELOW_Y + TextSelectionToolbar::K_TOOLBAR_CONTENT_DISTANCE_BELOW
            );

            // When it does fit above aboveAnchor, it positions itself there.
            set_state_and_settle(tester, &set_state, || {
                *anchor_above.borrow_mut() = Offset::new(50.0, 70.0);
            });
            assert_eq!(
                toolbar_y(tester),
                70.0 - HEIGHT - K_TOOLBAR_CONTENT_DISTANCE
            );
        });
    }

    /// Dart's `toolbarContainer.color`: the first `Material` under the
    /// `_TextSelectionToolbarContainer`.
    fn toolbar_container_color(tester: &WidgetTester) -> Option<Color> {
        let containers = find::by_type::<TextSelectionToolbarContainer>().evaluate(tester);
        assert!(!containers.is_empty());
        let materials = tester.descendants_of_type::<Material>(containers[0]);
        assert!(!materials.is_empty());
        tester.widget(materials[0], |material: &Material| material.color)
    }

    fn pump_toolbar_under(tester: &mut WidgetTester, color_scheme: ColorScheme) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::builder().color_scheme(color_scheme).build(&app),
                Scaffold::new()
                    .body(
                        Center::new().child(TextSelectionToolbar::new(
                            Offset::ZERO,
                            Offset::ZERO,
                            [TextSelectionToolbarTextButton::new(
                                Text::new("Custom button"),
                                TextSelectionToolbarTextButton::get_padding(0, 1),
                            )
                            .on_pressed(Listener::new(|_app| {}))
                            .into_widget()],
                        )),
                    )
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    /// Dart's `<ColorScheme>[ThemeData().colorScheme, ThemeData.dark().colorScheme]`.
    fn color_schemes(tester: &WidgetTester) -> [ColorScheme; 2] {
        let app = tester.cell.borrow();
        [
            ThemeData::new(&app).color_scheme,
            ThemeData::dark(&app, None).color_scheme,
        ]
    }

    #[test]
    fn default_background_color() {
        for index in 0..2 {
            test_widgets(|tester| {
                let color_scheme = color_schemes(tester)[index].clone();
                let brightness = color_scheme.brightness;
                pump_toolbar_under(tester, color_scheme);
                assert_eq!(
                    toolbar_container_color(tester),
                    // The default colors are hardcoded and don't take the default value of
                    // the theme's surface color.
                    Some(match brightness {
                        Brightness::Light => Color::new(0xffffffff),
                        Brightness::Dark => Color::new(0xff424242),
                    })
                );
            });
        }
    }

    #[test]
    fn custom_background_color() {
        for index in 0..2 {
            test_widgets(|tester| {
                let color_scheme = color_schemes(tester)[index].clone();
                let custom_background_color = Colors::RED.primary();
                pump_toolbar_under(
                    tester,
                    color_scheme
                        .copy_with()
                        .with_surface(custom_background_color),
                );
                assert_eq!(
                    toolbar_container_color(tester),
                    Some(custom_background_color)
                );
            });
        }
    }

    #[test]
    fn overflowed_menu_expands_children_horizontally() {
        // Regression test for https://github.com/flutter/flutter/issues/144089.
        test_widgets(|tester| {
            let children = Rc::new(RefCell::new(test_boxes(7)));
            let (toolbar, set_state) = toolbar_of(
                Rc::clone(&children),
                Rc::new(RefCell::new(Offset::new(50.0, 100.0))),
                Offset::new(50.0, 200.0),
            );
            pump_in_scaffold(tester, toolbar);

            // All children fit on the screen, so they are all rendered.
            let count = children.borrow().len();
            assert_eq!(find::by_type::<TestBox>().evaluate(tester).len(), count);
            assert!(find_overflow_button(tester).is_empty());

            const SHORT: &str = "Short";
            const MEDIUM: &str = "Medium length";
            const LONG: &str = "Long label in the overflow menu";

            // Adding several children makes the menu overflow.
            set_state_and_settle(tester, &set_state, || {
                children.borrow_mut().extend([
                    Text::new(SHORT).into_widget(),
                    Text::new(MEDIUM).into_widget(),
                    Text::new(LONG).into_widget(),
                ]);
            });
            assert_eq!(find_overflow_button(tester).len(), 1);

            // Tap the overflow button to show the overflow menu.
            tap_overflow_button(tester);
            expect(tester, &find::by_type::<TestBox>(), finds_nothing());
            expect(tester, &find::by_type::<Text>(), finds_n_widgets(3));
            assert_eq!(find_overflow_button(tester).len(), 1);

            let containers = find::by_type::<TextSelectionToolbarContainer>().evaluate(tester);
            assert!(!containers.is_empty());

            // Buttons have their width set to the container width.
            let overflow_menu_width = rect_of(tester, &containers).width();
            let text_width = |text: &str| {
                let found: Vec<AnyElement> = find::text(text).evaluate(tester);
                rect_of(tester, &found).width()
            };
            assert_eq!(text_width(LONG), overflow_menu_width);
            assert_eq!(text_width(MEDIUM), overflow_menu_width);
            assert_eq!(text_width(SHORT), overflow_menu_width);
        });
    }

    #[test]
    fn items_are_ordered_right_to_left_in_rtl() {
        test_widgets(|tester| {
            const ITEM_COUNT: usize = 3;
            let children: Vec<WidgetRef> = (0..ITEM_COUNT)
                .map(|i| {
                    TestBox {
                        key: Some(ValueKey::new(format!("item_{i}")).into()),
                        child: Some(Text::new(format!("{i}")).into_widget()),
                    }
                    .into_widget()
                })
                .collect();

            pump_in_scaffold(
                tester,
                Directionality::new(
                    TextDirection::Rtl,
                    TextSelectionToolbar::new(
                        Offset::new(50.0, 100.0),
                        Offset::new(50.0, 200.0),
                        children,
                    ),
                )
                .into_widget(),
            );

            // Verify all items are visible.
            assert_eq!(
                find::by_type::<TestBox>().evaluate(tester).len(),
                ITEM_COUNT
            );

            // Find all text widgets by their content and get their positions.
            let text_rects: Vec<Rect> = (0..ITEM_COUNT)
                .map(|i| tester.get_rect(find::text(&format!("{i}"))))
                .collect();

            // In RTL, items should be in reverse order (2, 1, 0).
            // So item 2 should be leftmost, then 1, then 0.
            for i in 0..ITEM_COUNT - 1 {
                let current = text_rects[i];
                let next = text_rects[i + 1];

                // In RTL, each item should be to the left of the previous one.
                assert!(
                    next.right <= current.left,
                    "In RTL, item {} should be to the left of item {i}",
                    i + 1
                );
            }

            // Verify the visual order by checking the rightmost position.
            let right_edges: Vec<f64> = text_rects.iter().map(|rect| rect.right).collect();
            let mut sorted_right_edges = right_edges.clone();
            sorted_right_edges.sort_by(|a, b| b.total_cmp(a));
            assert_eq!(
                right_edges, sorted_right_edges,
                "Items should be ordered right-to-left in RTL"
            );
        });
    }

    #[test]
    fn puts_children_in_an_overflow_menu_if_they_overflow_in_rtl() {
        test_widgets(|tester| overflows_into_a_menu(tester, TextDirection::Rtl));
    }

    #[test]
    fn the_overflow_button_is_keyed_as_a_standard_more_and_back_button() {
        test_widgets(|tester| {
            pump_in_scaffold(
                tester,
                TextSelectionToolbar::new(
                    Offset::new(50.0, 100.0),
                    Offset::new(50.0, 200.0),
                    test_boxes(8),
                )
                .into_widget(),
            );
            let more = StandardComponentType::MoreButton.key();
            let back = StandardComponentType::BackButton.key();
            expect(tester, &find::by_key(more.clone()), finds_one_widget());
            expect(tester, &find::by_key(back.clone()), finds_nothing());

            tap_overflow_button(tester);
            expect(tester, &find::by_key(more.clone()), finds_nothing());
            expect(tester, &find::by_key(back.clone()), finds_one_widget());
            let back_button = tester.element(find::by_key(back.clone()));
            let app = tester.cell.borrow();
            assert!(
                downcast_widget::<TextSelectionToolbarOverflowButton>(&**back_button.widget(&app))
                    .is_some()
            );
        });
    }
}
