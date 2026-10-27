//! ComboBox_Partial.cpp's popup placement for the mouse and the keyboard: `ArrangePopup` with
//! `GetNonPannablePopupLayout`, `GetEditableComboBoxPopupLayout` and `UpdateIsPopupPannable`,
//! the delegate that places `PopupBorder` where they say, and the probe that measures the items
//! they read.

use crate::*;
use inset_embedder::{Matrix4, Offset, Rect, Size, TextBaseline};
use inset_foundation::App;
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, PaintingContext,
    PipelineOwner, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
    SingleChildLayoutDelegate,
};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_widgets::*;
use std::{any::Any, cell::Cell, fmt, rc::Rc};

/// `PopupBorder`'s `Margin`: half a pixel up and one down, which `ArrangePopup` moves into the
/// popup's offset.
const POPUP_BORDER_MARGIN: [f64; 4] = [0.0, -0.5, 0.0, -1.0];

/// What `ArrangePopup` reads: where the box is, the window, the items' heights and the
/// properties that bound the popup.
#[derive(Clone, Debug)]
pub(super) struct ArrangeInput<'a> {
    /// The template's `Background` part in the overlay's coordinates.
    pub(super) target: Rect,

    /// The visible window: the overlay's size.
    pub(super) window: Size,

    /// `GetItemLayoutHeight` for each item, in item order.
    pub(super) heights: &'a [f64],

    /// The height `GetItemLayoutHeight` gives an index with no container: the first realized
    /// child's.
    pub(super) fallback_height: f64,

    /// `SelectedIndex`.
    pub(super) selected: Option<usize>,

    /// `MaxDropDownHeight`.
    pub(super) max_drop_down_height: f64,

    /// The editable text box's height, in an editable box.
    pub(super) editable: Option<f64>,

    /// The width the popup's content asks for.
    pub(super) content_width: f64,

    /// The popup's height in its last layout, `UpdateIsPopupPannable`'s `childHeight`.
    pub(super) last_height: f64,

    /// Whether the box flows right to left.
    pub(super) rtl: bool,
}

/// Where `ArrangePopup` puts `PopupBorder`, and how big it lets it be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Arrangement {
    /// The border's left edge in the overlay.
    pub(super) left: f64,

    /// The border's top edge in the overlay, `PopupBorder`'s margin included.
    pub(super) top: f64,

    /// The border's width: at least the box's, at most the window's.
    pub(super) width: f64,

    /// `MinHeight`: the box's height.
    pub(super) min_height: f64,

    /// `MaxHeight`: the height the items it shows take.
    pub(super) max_height: f64,

    /// `offset`: the first item shown, the one the list scrolls to when it opens pannable.
    pub(super) first_item: usize,

    /// `m_bIsPopupPannable`.
    pub(super) pannable: bool,

    /// `m_openedUp`: an editable box's popup opened above the box.
    pub(super) opened_up: bool,
}

/// What `GetNonPannablePopupLayout` and `GetEditableComboBoxPopupLayout` work out.
#[derive(Clone, Copy, Debug, PartialEq)]
struct VerticalLayout {
    /// `popupY`.
    top: f64,
    /// `popupMaxHeight`.
    height: f64,
    /// `offset`.
    first_item: usize,
    /// `m_openedUp`.
    opened_up: bool,
}

/// `GetMeasureDeltaForVisualsBetweenPopupAndCarouselPanel`: the height between `PopupBorder`
/// and the items, which the panel measures as what its first measure lacked of the popup's
/// initial height.
fn chrome_height(r: &ThemeResources) -> f64 {
    r.combo_box_dropdown_border_thickness[1]
        + r.combo_box_dropdown_border_thickness[3]
        + r.combo_box_dropdown_border_padding[1]
        + r.combo_box_dropdown_border_padding[3]
        + r.combo_box_dropdown_content_margin[1]
        + r.combo_box_dropdown_content_margin[3]
}

/// `TemplateSettings.DropDownContentMinWidth` for the mouse and the keyboard, plus the border:
/// the narrowest the popup is.
pub(super) fn popup_min_width(r: &ThemeResources, window: Size) -> f64 {
    window.width().min(r.combo_box_popup_theme_min_width)
        + r.combo_box_dropdown_border_thickness[0]
        + r.combo_box_dropdown_border_thickness[2]
}

/// `ArrangePopup` for a popup opened with the mouse or the keyboard: the popup at least as wide
/// as the box and within the window, with the selected item over the box where it fits, or, for
/// an editable box, below the box or else above it. None while the box has no size.
pub(super) fn arrange(r: &ThemeResources, input: &ArrangeInput) -> Option<Arrangement> {
    let target = input.target;
    let window = input.window;
    if window.width() == 0.0 || window.height() == 0.0 {
        return None;
    }
    let (cb_width, cb_height) = (target.width(), target.height());
    if cb_width == 0.0 || cb_height == 0.0 {
        return None;
    }
    let max_drop_down_height = if input.max_drop_down_height.is_finite() {
        input.max_drop_down_height
    } else {
        window.height()
    };
    let width = input
        .content_width
        .min(cb_width.max(window.width()))
        .max(cb_width);
    let child_width = width.min(window.width()).max(cb_width);
    let left = if input.rtl {
        // The source works in the box's mirrored coordinates, where the box's right edge is
        // its left.
        let mirrored = popup_left(window.width() - target.right, child_width, window.width());
        window.width() - mirrored - child_width
    } else {
        popup_left(target.left, child_width, window.width())
    };
    let pannable = is_pannable(
        r,
        input.heights,
        input.fallback_height,
        max_drop_down_height,
        window,
        input.last_height,
    );
    let vertical = match input.editable {
        Some(text_box_height) => editable_layout(
            r,
            input.heights,
            target.top,
            cb_height.max(text_box_height),
            window,
        ),
        None => non_pannable_layout(r, input, max_drop_down_height),
    };
    // The layouts counted the items and their margin; the border remains.
    let height = vertical.height + chrome_height(r)
        - r.combo_box_dropdown_content_margin[1]
        - r.combo_box_dropdown_content_margin[3];
    let mut top = vertical.top;
    if top + height > window.height() {
        top = (top - (top + height - window.height())).max(0.0);
    }
    Some(Arrangement {
        left: left + POPUP_BORDER_MARGIN[0],
        top: top + POPUP_BORDER_MARGIN[1],
        width,
        min_height: cb_height,
        max_height: height.max(0.0),
        first_item: vertical.first_item,
        pannable,
        opened_up: vertical.opened_up,
    })
}

/// `ArrangePopup`'s `popupX` added to the box's left edge `box_left`: the popup's left edge on
/// the box's, moved into a window `window_width` wide.
fn popup_left(box_left: f64, child_width: f64, window_width: f64) -> f64 {
    let mut popup_x = 0.0;
    if box_left < 0.0 {
        popup_x = -box_left;
    }
    if window_width < box_left + child_width {
        popup_x = window_width - (child_width + box_left);
    }
    box_left + popup_x
}

/// `UpdateIsPopupPannable`: whether the items do not all fit, which is always so for more than
/// `ComboBoxPopupMaxNumberOfItems` of them. Like the source, it counts the last item twice.
fn is_pannable(
    r: &ThemeResources,
    heights: &[f64],
    fallback_height: f64,
    max_drop_down_height: f64,
    window: Size,
    last_height: f64,
) -> bool {
    let count = heights.len();
    if count == 0 {
        return false;
    }
    if count > r.combo_box_popup_max_number_of_items as usize {
        return true;
    }
    let max_allowed = max_drop_down_height.min(window.height());
    if last_height > 0.0 && last_height > max_allowed {
        return true;
    }
    let height = |index: usize| heights.get(index).copied().unwrap_or(fallback_height);
    let mut total = 2.0 * height(count - 1);
    for index in 0..count - 1 {
        total += height(index);
        if total > max_allowed {
            return true;
        }
    }
    false
}

/// `GetNonPannablePopupLayout`: the selected item, or the middle one, over the box, as many items
/// above it as fit up to `ComboBoxPopupMaxNumberOfItemsThatCanBeShownOnOneSide`, then items below
/// and, while room is left, more on either side, up to `ComboBoxPopupMaxNumberOfItems` in all.
fn non_pannable_layout(
    r: &ThemeResources,
    input: &ArrangeInput,
    max_height: f64,
) -> VerticalLayout {
    let (heights, fallback_height, selected, window) = (
        input.heights,
        input.fallback_height,
        input.selected,
        input.window,
    );
    let (cb_y, cb_height) = (input.target.top, input.target.height());
    let count = heights.len() as i64;
    let margin = r.combo_box_dropdown_content_margin;
    let height = |index: i64| {
        heights
            .get(index as usize)
            .copied()
            .unwrap_or(fallback_height)
    };
    let center = match selected {
        Some(index) if (index as i64) <= count => index as i64,
        _ => count / 2,
    };
    if count == 0 {
        return VerticalLayout {
            top: cb_y,
            height: cb_height,
            first_item: 0,
            opened_up: false,
        };
    }
    let mut current = height(center);
    // A box beyond the window's bottom is placed as if it were inside it.
    let cb_y = if cb_y + cb_height >= window.height() {
        window.height() - cb_height
    } else {
        cb_y
    };
    let calculated_above = cb_y + cb_height / 2.0 - current / 2.0 - margin[1];
    let mut above = calculated_above.max(0.0);
    let upper_limit = (cb_y + cb_height / 2.0 - max_height / 2.0).max(0.0);
    let calculated_below = above + current + margin[1] + margin[3];
    let mut below = calculated_below.min(window.height());
    let lower_limit = (upper_limit + max_height).min(window.height());
    let mut index_above = center - 1;
    let mut index_below = center + 1;
    let mut laid = 1;
    let max_on_one_side =
        (r.combo_box_popup_max_number_of_items_that_can_be_shown_on_one_side as i64).min(count);
    let max_allowed = (r.combo_box_popup_max_number_of_items as i64).min(count);
    if calculated_below > window.height() {
        above = (above - calculated_below + window.height()).max(0.0);
    }
    if index_above >= 0 {
        current = height(index_above);
        while index_above >= 0 && above - current >= upper_limit && laid < max_on_one_side {
            above -= current;
            laid += 1;
            index_above -= 1;
            if index_above >= 0 {
                current = height(index_above);
            }
        }
    }
    if index_below < count {
        current = height(index_below);
        while index_below < count
            && below + current < lower_limit
            && below - above < max_height
            && laid < max_allowed
        {
            below += current;
            laid += 1;
            index_below += 1;
            if index_below < count {
                current = height(index_below);
            }
        }
    }
    if index_above >= 0 || index_below < count {
        let mut is_above = index_above >= 0;
        current = height(if is_above { index_above } else { index_below });
        while below - above + current <= max_height
            && (below + current < window.height() || above - current >= 0.0)
            && laid < max_allowed
        {
            if is_above {
                index_above -= 1;
            } else {
                index_below += 1;
            }
            if above - current <= 0.0 {
                below += current;
            } else {
                above -= current;
            }
            laid += 1;
            if index_above >= 0 || index_below < count {
                is_above = index_above >= 0;
                current = height(if is_above { index_above } else { index_below });
            }
        }
    }
    VerticalLayout {
        top: above,
        height: below - above,
        first_item: (index_above + 1) as usize,
        opened_up: false,
    }
}

/// `GetEditableComboBoxPopupLayout`: up to `ComboBoxPopupMaxNumberOfItems` items below the box,
/// or above it when they do not fit below and do above.
fn editable_layout(
    r: &ThemeResources,
    heights: &[f64],
    cb_y: f64,
    cb_height: f64,
    window: Size,
) -> VerticalLayout {
    if heights.is_empty() {
        return VerticalLayout {
            top: cb_y,
            height: cb_height,
            first_item: 0,
            opened_up: false,
        };
    }
    let margin = r.combo_box_dropdown_content_margin;
    // The IME candidate window's height, which this host does not report, would be added here.
    let above = (cb_y + cb_height).max(0.0);
    let shown = heights
        .iter()
        .take(r.combo_box_popup_max_number_of_items as usize);
    let below = above + margin[1] + margin[3] + shown.sum::<f64>();
    let height = below - above;
    let (top, opened_up) = if above + height > window.height() && cb_y - height >= 0.0 {
        ((cb_y - height).max(0.0), true)
    } else {
        (above, false)
    };
    VerticalLayout {
        top,
        height,
        first_item: 0,
        opened_up,
    }
}

/// Places `PopupBorder` where `ArrangePopup` puts it, as wide as it says and no taller than its
/// `MaxHeight`, and records the height the border takes for the next `UpdateIsPopupPannable`.
#[derive(Clone)]
pub(super) struct PopupPosition {
    /// The arrangement to apply.
    pub(super) arrangement: Arrangement,

    /// Receives the border's height after each layout.
    pub(super) laid_out_height: Rc<Cell<f64>>,
}

impl fmt::Debug for PopupPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupPosition")
            .field("arrangement", &self.arrangement)
            .finish_non_exhaustive()
    }
}

impl SingleChildLayoutDelegate for PopupPosition {
    fn get_constraints_for_child(&self, _constraints: BoxConstraints) -> BoxConstraints {
        let a = self.arrangement;
        BoxConstraints::new()
            .min_width(a.width)
            .max_width(a.width)
            .min_height(a.min_height.min(a.max_height))
            .max_height(a.max_height)
    }

    fn get_position_for_child(&self, _size: Size, child_size: Size) -> Offset {
        self.laid_out_height.set(child_size.height());
        Offset::new(self.arrangement.left, self.arrangement.top)
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<Self>()
            .is_none_or(|old| old.arrangement != self.arrangement)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Receives an item container's natural width and its height, after the frame that laid it out.
pub(super) type ItemMeasured = Rc<dyn Fn(&mut App, Size)>;

/// Measures one item container where the list lays it out: its height there, and the width it
/// would take on its own, which the source's panel reads as its desired size.
pub(super) struct ItemProbe {
    /// The item container.
    pub(super) child: WidgetRef,

    /// Receives changes of what was measured.
    pub(super) measured: ItemMeasured,
}

impl ItemProbe {
    /// Measures `child`.
    pub(super) fn new<K>(child: impl IntoWidget<K>, measured: ItemMeasured) -> Self {
        Self {
            child: child.into_widget(),
            measured,
        }
    }
}

impl fmt::Debug for ItemProbe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemProbe").finish_non_exhaustive()
    }
}

impl RenderObjectWidget for ItemProbe {
    type RenderObject = RenderItemProbe;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderItemProbe {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                measured: self.measured.clone(),
                observed: Size::ZERO,
                reported: Size::ZERO,
                pending: false,
                epoch: 0,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderItemProbe>,
    ) {
        render_object.get_mut(app).measured = self.measured.clone();
    }
}

impl SingleChildRenderObjectWidget for ItemProbe {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// The layout proxy of `ItemProbe`, reporting once per frame without changing the tree during
/// layout.
pub(super) struct RenderItemProbe {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    measured: ItemMeasured,
    /// The natural width and the height of the last layout.
    observed: Size,
    /// What the last callback received.
    reported: Size,
    /// Whether a report is queued.
    pending: bool,
    /// Invalidates reports queued before the render object detached.
    epoch: u64,
}

impl RenderItemProbe {
    /// Queues one report for the end of the frame, if what was measured changed.
    fn schedule_report(self: RenderHandle<Self>, app: &mut App) {
        if self.get(app).pending || self.get(app).observed == self.get(app).reported {
            return;
        }
        self.get_mut(app).pending = true;
        let epoch = self.get(app).epoch;
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self.handle()) || app.is_disposed(self.handle()) {
                    return;
                }
                if self.get(app).epoch != epoch || !self.as_object().attached(app) {
                    return;
                }
                self.get_mut(app).pending = false;
                let observed = self.get(app).observed;
                if observed == self.get(app).reported {
                    return;
                }
                self.get_mut(app).reported = observed;
                let measured = self.get(app).measured.clone();
                measured(app, observed);
            }),
            // Queued during layout, which every view of a frame shares.
            None,
        );
    }
}

impl RenderObjectWithChildMixin for RenderItemProbe {
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

impl RenderProxyBoxMixin for RenderItemProbe {}

impl RenderObject for RenderItemProbe {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
        let height = self.as_box().size(app).height();
        let width = self
            .child(app)
            .map_or(0.0, |child| child.get_max_intrinsic_width(app, height));
        self.get_mut(app).observed = Size::new(width, height);
        self.schedule_report(app);
    }

    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: inset_foundation::Handle<PipelineOwner>,
    ) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        self.schedule_report(app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).epoch += 1;
        self.get_mut(app).pending = false;
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
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

impl RenderBox for RenderItemProbe {
    inset_rendering::render_box_accessors!();

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

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size::new(1080.0, 780.0);

    /// WinUI's own values.
    fn winui() -> ThemeResources {
        ThemeResources::new(Theme::Light, AccentPalette::default())
    }

    fn input(heights: &[f64], selected: Option<usize>, top: f64) -> ArrangeInput<'_> {
        ArrangeInput {
            target: Rect::from_ltwh(100.0, top, 200.0, 32.0),
            window: WINDOW,
            heights,
            fallback_height: 36.0,
            selected,
            max_drop_down_height: 504.0,
            editable: None,
            content_width: 0.0,
            last_height: 0.0,
            rtl: false,
        }
    }

    #[test]
    fn a_short_list_puts_the_selected_item_over_the_box() {
        let heights = [36.0; 5];
        let a = arrange(&winui(), &input(&heights, Some(2), 300.0)).unwrap();
        // The items and their 4-pixel margins, and the border.
        assert_eq!(a.max_height, 5.0 * 36.0 + 8.0 + 2.0);
        assert_eq!(a.top, 222.0 - 0.5);
        assert_eq!(a.first_item, 0);
        assert!(!a.pannable);
        // The selected item's top: the border, the margin, two items.
        let item_top = a.top + 1.0 + 4.0 + 2.0 * 36.0;
        assert_eq!(item_top + 18.0, 316.5);
    }

    #[test]
    fn a_long_list_shows_up_to_seven_items_above_and_scrolls_to_the_first() {
        let heights = vec![36.0; 300];
        let a = arrange(&winui(), &input(&heights, Some(150), 300.0)).unwrap();
        assert!(a.pannable);
        assert_eq!(a.first_item, 144);
        assert_eq!(a.top, 78.0 - 0.5);
        assert_eq!(a.max_height, 476.0 + 2.0);
    }

    #[test]
    fn near_the_window_top_the_items_above_go_below() {
        let heights = [36.0; 5];
        let a = arrange(&winui(), &input(&heights, Some(2), 10.0)).unwrap();
        assert_eq!(a.top, 4.0 - 0.5);
        assert_eq!(a.first_item, 0);
        assert_eq!(a.max_height, 188.0 + 2.0);
    }

    #[test]
    fn without_a_selection_the_middle_item_is_over_the_box() {
        let heights = [36.0; 3];
        let a = arrange(&winui(), &input(&heights, None, 300.0)).unwrap();
        // Item 1 is centered on the box: one item above it.
        assert_eq!(a.top, 300.0 + 16.0 - 18.0 - 4.0 - 36.0 - 0.5);
    }

    #[test]
    fn the_popup_is_at_least_as_wide_as_the_box_and_stays_in_the_window() {
        let heights = [36.0; 3];
        let mut wide = input(&heights, None, 300.0);
        wide.content_width = 120.0;
        assert_eq!(arrange(&winui(), &wide).unwrap().width, 200.0);
        wide.content_width = 400.0;
        wide.target = Rect::from_ltwh(900.0, 300.0, 100.0, 32.0);
        let a = arrange(&winui(), &wide).unwrap();
        assert_eq!(a.width, 400.0);
        assert_eq!(a.left, WINDOW.width() - 400.0);
    }

    #[test]
    fn right_to_left_the_popup_ends_at_the_box_and_stays_in_the_window() {
        let heights = [36.0; 3];
        let mut rtl = input(&heights, None, 300.0);
        rtl.rtl = true;
        rtl.content_width = 400.0;
        // Its right edge on the box's.
        rtl.target = Rect::from_ltwh(600.0, 300.0, 200.0, 32.0);
        assert_eq!(arrange(&winui(), &rtl).unwrap().left, 400.0);
        // Moved in from the window's left edge, and from its right where the box goes past it.
        rtl.target = Rect::from_ltwh(100.0, 300.0, 200.0, 32.0);
        assert_eq!(arrange(&winui(), &rtl).unwrap().left, 0.0);
        rtl.target = Rect::from_ltwh(1000.0, 300.0, 200.0, 32.0);
        assert_eq!(
            arrange(&winui(), &rtl).unwrap().left,
            WINDOW.width() - 400.0
        );
    }

    #[test]
    fn an_editable_box_opens_below_or_else_above() {
        let heights = [36.0; 5];
        let mut below = input(&heights, Some(4), 300.0);
        below.editable = Some(32.0);
        let a = arrange(&winui(), &below).unwrap();
        assert!(!a.opened_up);
        assert_eq!(a.top, 332.0 - 0.5);
        assert_eq!(a.max_height, 188.0 + 2.0);
        below.target = Rect::from_ltwh(100.0, 700.0, 200.0, 32.0);
        let a = arrange(&winui(), &below).unwrap();
        assert!(a.opened_up);
        assert_eq!(a.top, 700.0 - 188.0 - 0.5);
    }

    #[test]
    fn fifteen_items_of_one_height_fit_and_sixteen_pan() {
        assert!(!is_pannable(
            &winui(),
            &[30.0; 15],
            30.0,
            504.0,
            WINDOW,
            0.0
        ));
        assert!(is_pannable(&winui(), &[30.0; 16], 30.0, 504.0, WINDOW, 0.0));
        // The last item counts twice: 13 items of 36 make 504.
        assert!(!is_pannable(
            &winui(),
            &[36.0; 13],
            36.0,
            504.0,
            WINDOW,
            0.0
        ));
        assert!(is_pannable(&winui(), &[36.0; 14], 36.0, 504.0, WINDOW, 0.0));
    }
}
