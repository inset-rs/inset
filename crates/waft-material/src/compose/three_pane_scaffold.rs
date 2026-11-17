//! The pane scaffold: up to three panes laid out from a directive and a value, moving on
//! Compose's pane spring when the value changes.

use std::rc::Rc;

use waft_animation::{AnimationBehavior, AnimationController, AnyAnimation};
use waft_embedder::{Clip, Offset, Rect, Size, TextDirection};
use waft_foundation::{App, Handle, Listenable, MergingListenable, Ref, ValueKey};
use waft_painting::AlignmentGeometry;
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    BuildContext, ClipPath, ClipRect, Directionality, IgnorePointer, IntoWidget, KeyRef,
    LayoutBuilder, ListenableBuilder, Offstage, OverflowBox, Positioned, Stack, State, StateData,
    StatefulWidget, TickerProviderStateMixin, TickerProviderStateMixinData, Transform, WidgetRef,
};

use crate::compose::pane_motion::{
    PaneFrame, PaneMotion, PaneMotionData, PaneMotionDefaults, PaneMotionType,
    ThreePaneScaffoldMotionData, calculate_pane_motion,
};
use crate::compose::{
    Pane, PaneAdaptedValue, PaneScaffoldDirective, ThreePaneScaffoldHorizontalOrder,
    ThreePaneScaffoldRole, ThreePaneScaffoldValue,
};

/// Up to three panes laid out from a directive and a value.
///
/// `list_detail` and `supporting` differ only in pane order and slot names. When the value
/// changes, the panes move as Compose's do: shown panes move to their new bounds, and panes
/// that appear or disappear slide in or out from the free side. A hidden pane stays mounted
/// offstage, so a list keeps its scroll position while the detail covers it.
#[derive(Debug)]
pub struct ThreePaneScaffold {
    /// The scaffold's key.
    pub key: Option<KeyRef>,
    /// The pane sizes and partitions.
    pub directive: PaneScaffoldDirective,
    /// The panes shown.
    pub value: ThreePaneScaffoldValue,
    /// The panes' order from start to end.
    pub pane_order: ThreePaneScaffoldHorizontalOrder,
    /// The primary pane's content.
    pub primary_pane: Pane,
    /// The secondary pane's content.
    pub secondary_pane: Pane,
    /// The tertiary pane's content, when the scaffold has one.
    pub tertiary_pane: Option<Pane>,
}

impl ThreePaneScaffold {
    /// Creates a pane scaffold with the panes in `pane_order`.
    pub fn new(
        directive: PaneScaffoldDirective,
        value: ThreePaneScaffoldValue,
        pane_order: ThreePaneScaffoldHorizontalOrder,
        primary_pane: impl Into<Pane>,
        secondary_pane: impl Into<Pane>,
    ) -> ThreePaneScaffold {
        ThreePaneScaffold {
            key: None,
            directive,
            value,
            pane_order,
            primary_pane: primary_pane.into(),
            secondary_pane: secondary_pane.into(),
            tertiary_pane: None,
        }
    }

    /// A list-detail scaffold: list, detail, extra.
    pub fn list_detail(
        directive: PaneScaffoldDirective,
        value: ThreePaneScaffoldValue,
        list_pane: impl Into<Pane>,
        detail_pane: impl Into<Pane>,
    ) -> ThreePaneScaffold {
        ThreePaneScaffold::new(
            directive,
            value,
            ThreePaneScaffoldHorizontalOrder::LIST_DETAIL,
            detail_pane,
            list_pane,
        )
    }

    /// A supporting-pane scaffold: main, supporting, extra.
    pub fn supporting(
        directive: PaneScaffoldDirective,
        value: ThreePaneScaffoldValue,
        main_pane: impl Into<Pane>,
        supporting_pane: impl Into<Pane>,
    ) -> ThreePaneScaffold {
        ThreePaneScaffold::new(
            directive,
            value,
            ThreePaneScaffoldHorizontalOrder::SUPPORTING,
            main_pane,
            supporting_pane,
        )
    }

    /// Adds the tertiary pane.
    pub fn tertiary_pane(mut self, tertiary_pane: impl Into<Pane>) -> ThreePaneScaffold {
        self.tertiary_pane = Some(tertiary_pane.into());
        self
    }

    /// Sets the scaffold's key.
    pub fn key(mut self, key: KeyRef) -> ThreePaneScaffold {
        self.key = Some(key);
        self
    }

    fn pane(&self, role: ThreePaneScaffoldRole) -> Option<&Pane> {
        match role {
            ThreePaneScaffoldRole::Primary => Some(&self.primary_pane),
            ThreePaneScaffoldRole::Secondary => Some(&self.secondary_pane),
            ThreePaneScaffoldRole::Tertiary => self.tertiary_pane.as_ref(),
        }
    }
}

impl StatefulWidget for ThreePaneScaffold {
    type State = ThreePaneScaffoldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ThreePaneScaffoldState {
        ThreePaneScaffoldState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            motion: None,
            delayed_motion: None,
            motions: [PaneMotion::NoMotion; 3],
            origins: [None; 3],
            bounds: [None; 3],
            scaffold_width: 0.0,
        }
    }
}

/// The springs a [`ThreePaneScaffold`] moves its panes on, and each pane's motion.
///
/// The arrays are indexed by role. `bounds` and `scaffold_width` are written on every layout:
/// the bounds each pane was last laid out at, which a hidden pane keeps. A change of value reads
/// them to start the next motion from where the panes are drawn.
pub struct ThreePaneScaffoldState {
    state: StateData<ThreePaneScaffold>,
    ticker_provider: TickerProviderStateMixinData,
    motion: Option<Ref<AnimationController>>,
    delayed_motion: Option<Ref<AnimationController>>,
    motions: [PaneMotion; 3],
    origins: [Option<Rect>; 3],
    bounds: [Option<Rect>; 3],
    scaffold_width: f64,
}

impl ThreePaneScaffoldState {
    fn motion(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .motion
            .as_ref()
            .expect("created in init_state")
            .handle()
    }

    fn delayed_motion(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .delayed_motion
            .as_ref()
            .expect("created in init_state")
            .handle()
    }

    /// Starts each pane's motion from where it is drawn now towards `current`.
    fn start_motion(
        self: Handle<Self>,
        app: &mut App,
        previous: &ThreePaneScaffoldValue,
        current: &ThreePaneScaffoldValue,
        ltr_order: ThreePaneScaffoldHorizontalOrder,
    ) {
        let origins = self.drawn_content_rects(app, ltr_order);
        let motions = calculate_pane_motion(previous, current, ltr_order);
        let state = app.get_mut(self);
        for (index, role) in ltr_order.roles().into_iter().enumerate() {
            state.motions[role.index()] = motions[index];
            state.origins[role.index()] = origins[index];
        }
        let motion = self.motion(app);
        let delayed_motion = self.delayed_motion(app);
        motion.animate_with(app, Box::new(PaneMotionDefaults::simulation()));
        delayed_motion.animate_with(app, Box::new(PaneMotionDefaults::delayed_simulation()));
    }

    /// Where each pane's content is drawn now, left to right; `None` before its first layout.
    fn drawn_content_rects(
        self: Handle<Self>,
        app: &App,
        ltr_order: ThreePaneScaffoldHorizontalOrder,
    ) -> [Option<Rect>; 3] {
        let scaffold_width = app.get(self).scaffold_width;
        let motion_data = self.motion_data(app, ltr_order, scaffold_width, Rect::ZERO);
        let (progress, delayed_progress) = self.progress(app);
        std::array::from_fn(|index| {
            app.get(self).bounds[ltr_order.get(index).index()].map(|_| {
                motion_data
                    .frame(index, progress, delayed_progress)
                    .content_rect()
            })
        })
    }

    fn progress(self: Handle<Self>, app: &App) -> (f64, f64) {
        (
            self.motion(app).value(app),
            self.delayed_motion(app).value(app),
        )
    }

    /// Each pane's motion data, left to right; a pane never laid out uses `unplaced`.
    fn motion_data(
        self: Handle<Self>,
        app: &App,
        ltr_order: ThreePaneScaffoldHorizontalOrder,
        scaffold_width: f64,
        unplaced: Rect,
    ) -> ThreePaneScaffoldMotionData {
        let state = app.get(self);
        ThreePaneScaffoldMotionData {
            scaffold_width,
            panes: ltr_order.roles().map(|role| {
                let target = state.bounds[role.index()].unwrap_or(unplaced);
                PaneMotionData {
                    motion: state.motions[role.index()],
                    origin: state.origins[role.index()].unwrap_or(target),
                    target,
                }
            }),
        }
    }

    /// Lays the panes out in `size` and draws each where its motion has got to.
    fn build_panes(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        size: Size,
    ) -> WidgetRef {
        let text_direction = Directionality::of(app, context);
        let ltr_order = self.widget(app).pane_order.to_ltr(text_direction);
        self.lay_out(app, size, ltr_order, text_direction);
        let unplaced = Rect::from_ltwh(0.0, 0.0, size.width(), size.height());
        let motion_data = self.motion_data(app, ltr_order, size.width(), unplaced);
        let (progress, delayed_progress) = self.progress(app);
        let settled =
            !self.motion(app).is_animating(app) && !self.delayed_motion(app).is_animating(app);

        let widget = self.widget(app);
        let mut hidden = Vec::new();
        let mut shown = Vec::new();
        for (index, role) in ltr_order.roles().into_iter().enumerate() {
            let Some(pane) = widget.pane(role) else {
                continue;
            };
            let is_shown = widget.value.get(role) != PaneAdaptedValue::Hidden;
            let is_exiting =
                motion_data.panes[index].motion.motion_type() == PaneMotionType::Exiting;
            let drawn = drawn_pane(
                role,
                pane,
                motion_data.frame(index, progress, delayed_progress),
                is_shown,
                !is_shown && (settled || !is_exiting),
            );
            if is_shown {
                shown.push(drawn);
            } else {
                hidden.push(drawn);
            }
        }
        // Panes on their way out are drawn under the shown ones, as Compose's hidden z-index.
        hidden.extend(shown);
        Stack::new().children(hidden).into_widget()
    }

    /// Records the bounds of the panes shown in `size`; a hidden pane keeps its last ones.
    fn lay_out(
        self: Handle<Self>,
        app: &mut App,
        size: Size,
        ltr_order: ThreePaneScaffoldHorizontalOrder,
        text_direction: TextDirection,
    ) {
        let targets = pane_bounds(self.widget(app), size, ltr_order, text_direction);
        let state = app.get_mut(self);
        state.scaffold_width = size.width();
        for (index, role) in ltr_order.roles().into_iter().enumerate() {
            if let Some(target) = targets[index] {
                state.bounds[role.index()] = Some(target);
            }
        }
    }
}

/// One pane drawn at `frame`. Every pane keeps this shape in every frame, so its content keeps
/// its element, and its state, through each motion.
fn drawn_pane(
    role: ThreePaneScaffoldRole,
    pane: &Pane,
    frame: PaneFrame,
    is_shown: bool,
    offstage: bool,
) -> WidgetRef {
    let clip_behavior = if frame.clips {
        Clip::HardEdge
    } else {
        Clip::None
    };
    let content = OverflowBox::new()
        .alignment(AlignmentGeometry::CENTER)
        .min_width(frame.content_width)
        .max_width(frame.content_width)
        .child(
            Transform::translate(Offset::new(frame.content_offset, 0.0)).child(pane.child.clone()),
        );
    let clipped = ClipRect::new().clip_behavior(clip_behavior).child(content);
    // The shape clips the pane's bounds as they move, as Compose's `clip(shape)` comes after its
    // bounds animation.
    let shaped = match &pane.shape {
        Some(shape) => ClipPath::shape(shape.clone()).child(clipped).into_widget(),
        None => clipped.into_widget(),
    };
    Positioned::from_rect(
        frame.bounds,
        Offstage::new()
            .offstage(offstage)
            .child(IgnorePointer::new().ignoring(!is_shown).child(shaped)),
    )
    .key(ValueKey::new(role).into())
    .into_widget()
}

impl TickerProviderStateMixin for ThreePaneScaffoldState {
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

impl TickerProviderObject for ThreePaneScaffoldState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for ThreePaneScaffoldState {
    type Widget = ThreePaneScaffold;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let motion = settled_controller(app, self);
        let delayed_motion = settled_controller(app, self);
        let state = app.get_mut(self);
        state.motion = Some(motion);
        state.delayed_motion = Some(delayed_motion);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.motion(app).dispose(app);
        self.delayed_motion(app).dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ThreePaneScaffold) {
        let widget = self.widget(app);
        if widget.value == old_widget.value {
            return;
        }
        let current = widget.value;
        let pane_order = widget.pane_order;
        let context = self.context(app);
        let ltr_order = pane_order.to_ltr(Directionality::of(app, context));
        self.start_motion(app, &old_widget.value, &current, ltr_order);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let state = app.get(self);
        let springs: Vec<Option<Rc<dyn Listenable>>> = [&state.motion, &state.delayed_motion]
            .into_iter()
            .map(|controller| {
                let controller = controller.as_ref().expect("created in init_state");
                let animation: Rc<dyn Listenable> =
                    Rc::new(controller.erase::<AnyAnimation<f64>>());
                Some(animation)
            })
            .collect();
        ListenableBuilder::new(
            Rc::new(MergingListenable::new(springs)),
            move |_app, _context, _child| {
                LayoutBuilder::new(move |app, context, constraints| {
                    self.build_panes(app, context, constraints.constraints().biggest())
                })
                .into_widget()
            },
        )
        .into_widget()
    }
}

/// A controller for the pane spring, at rest on its end; unbounded, since the spring overshoots.
fn settled_controller(
    app: &mut App,
    vsync: Handle<ThreePaneScaffoldState>,
) -> Ref<AnimationController> {
    AnimationController::unbounded(app, 1.0, None, None, AnimationBehavior::Normal, vsync)
}

/// Compose's `ThreePaneContentMeasurePolicy` without hinges, levitated panes or pane expansion:
/// the bounds of each shown pane, left to right.
///
/// Expanded panes take their preferred widths; the leftover goes to the one with the highest
/// priority, and they all shrink in proportion when they do not fit. A reflowed pane takes the
/// bottom of its anchor's partition, at most half. Each pane's margins then move the edges it
/// shares with the scaffold, as Compose's `doMeasureAndPlace` does after measuring.
fn pane_bounds(
    scaffold: &ThreePaneScaffold,
    size: Size,
    ltr_order: ThreePaneScaffoldHorizontalOrder,
    text_direction: TextDirection,
) -> [Option<Rect>; 3] {
    let mut bounds = partition_bounds(scaffold, size, ltr_order);
    for (index, role) in ltr_order.roles().into_iter().enumerate() {
        if let (Some(rect), Some(pane)) = (bounds[index], scaffold.pane(role)) {
            bounds[index] = Some(pane.within_margins(rect, size, text_direction));
        }
    }
    bounds
}

/// The bounds of each shown pane, left to right, before margins.
fn partition_bounds(
    scaffold: &ThreePaneScaffold,
    size: Size,
    ltr_order: ThreePaneScaffoldHorizontalOrder,
) -> [Option<Rect>; 3] {
    let mut bounds = [None; 3];
    let directive = &scaffold.directive;
    let panes: Vec<(usize, ThreePaneScaffoldRole, &Pane)> = ltr_order
        .roles()
        .into_iter()
        .enumerate()
        .filter_map(|(index, role)| scaffold.pane(role).map(|pane| (index, role, pane)))
        .collect();
    let mut expanded: Vec<(usize, ThreePaneScaffoldRole, f64)> = panes
        .iter()
        .filter(|(_, role, _)| scaffold.value.get(*role) == PaneAdaptedValue::Expanded)
        .map(|(index, role, pane)| {
            let width = pane.width_for(size, directive.default_pane_preferred_width);
            (*index, *role, width)
        })
        .collect();
    let reflowed = panes
        .iter()
        .find_map(|(index, role, pane)| match scaffold.value.get(*role) {
            PaneAdaptedValue::Reflowed { reflow_under } => Some((*index, reflow_under, *pane)),
            _ => None,
        });
    if expanded.is_empty() {
        return bounds;
    }

    let spacer = directive.horizontal_partition_spacer_size;
    let allocatable_width = size.width() - (expanded.len() - 1) as f64 * spacer;
    let total_preferred_width: f64 = expanded.iter().map(|(_, _, width)| width).sum();
    if allocatable_width > total_preferred_width {
        let (_, _, width) = expanded
            .iter_mut()
            .max_by_key(|(_, role, _)| priority(*role))
            .expect("an expanded pane");
        *width += allocatable_width - total_preferred_width;
    } else if allocatable_width < total_preferred_width {
        let scale = allocatable_width / total_preferred_width;
        for (_, _, width) in &mut expanded {
            *width *= scale;
        }
    }

    let vertical_spacer = directive.vertical_partition_spacer_size;
    let mut left = 0.0;
    for (index, role, width) in expanded {
        let mut height = size.height();
        if let Some((reflowed_index, reflow_under, reflowed_pane)) = reflowed
            && reflow_under == role
        {
            let available_height = size.height() - vertical_spacer;
            let preferred_height =
                reflowed_pane.height_for(size, directive.default_pane_preferred_height);
            height = (available_height - preferred_height).max(available_height / 2.0);
            bounds[reflowed_index] = Some(Rect::from_ltwh(
                left,
                height + vertical_spacer,
                width,
                available_height - height,
            ));
        }
        bounds[index] = Some(Rect::from_ltwh(left, 0.0, width, height));
        left += width + spacer;
    }
    bounds
}

/// Compose's `ThreePaneScaffoldDefaults` pane priorities.
fn priority(role: ThreePaneScaffoldRole) -> u8 {
    match role {
        ThreePaneScaffoldRole::Primary => 10,
        ThreePaneScaffoldRole::Secondary => 5,
        ThreePaneScaffoldRole::Tertiary => 1,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use waft_embedder::{Size, TextDirection};
    use waft_foundation::ValueKey;
    use waft_painting::EdgeInsetsGeometry;
    use waft_painting::{BorderRadius, BorderSide, RoundedRectangleBorder};
    use waft_widgets::testing::{
        WidgetTester, expect, find, finds_nothing, finds_one_widget, test_widgets,
    };
    use waft_widgets::{Directionality, Padding, SizedBox};

    use super::*;
    use crate::compose::{
        PaneAdaptedValue, PanePreferredSize, SupportingPaneScaffoldRole, ThreePaneScaffoldValue,
        WindowHeightSizeClass, WindowSizeClass, WindowWidthSizeClass,
    };

    fn directive(
        width: WindowWidthSizeClass,
        height: WindowHeightSizeClass,
    ) -> PaneScaffoldDirective {
        PaneScaffoldDirective::from_window_size_class(WindowSizeClass { width, height })
    }

    fn expanded_directive() -> PaneScaffoldDirective {
        directive(
            WindowWidthSizeClass::Expanded,
            WindowHeightSizeClass::Expanded,
        )
    }

    fn pane(key: i32) -> Pane {
        Pane::new(SizedBox::new().key(ValueKey::new(key).into()))
    }

    fn pump_in(
        tester: &mut WidgetTester,
        text_direction: TextDirection,
        width: f64,
        height: f64,
        scaffold: ThreePaneScaffold,
    ) {
        tester.set_surface_size(Some(Size::new(width, height)));
        tester.pump_widget(Directionality::new(text_direction, scaffold).into_widget());
    }

    fn pump(tester: &mut WidgetTester, width: f64, height: f64, scaffold: ThreePaneScaffold) {
        pump_in(tester, TextDirection::Ltr, width, height, scaffold);
    }

    fn rect_of(tester: &WidgetTester, key: i32) -> Rect {
        tester.get_rect(find::by_key(ValueKey::new(key)).skip_offstage(false))
    }

    fn value(
        primary: PaneAdaptedValue,
        secondary: PaneAdaptedValue,
        tertiary: PaneAdaptedValue,
    ) -> ThreePaneScaffoldValue {
        ThreePaneScaffoldValue {
            primary,
            secondary,
            tertiary,
        }
    }

    fn list_and_detail() -> ThreePaneScaffoldValue {
        value(
            PaneAdaptedValue::Expanded,
            PaneAdaptedValue::Expanded,
            PaneAdaptedValue::Hidden,
        )
    }

    fn list_and_extra() -> ThreePaneScaffoldValue {
        value(
            PaneAdaptedValue::Hidden,
            PaneAdaptedValue::Expanded,
            PaneAdaptedValue::Expanded,
        )
    }

    fn list_detail_extra(value: ThreePaneScaffoldValue) -> ThreePaneScaffold {
        ThreePaneScaffold::list_detail(expanded_directive(), value, pane(1), pane(2))
            .tertiary_pane(pane(3))
    }

    #[test]
    fn a_wide_list_detail_places_the_list_beside_the_detail() {
        test_widgets(|tester| {
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    expanded_directive(),
                    list_and_detail(),
                    pane(1),
                    pane(2),
                ),
            );
            assert_eq!(rect_of(tester, 1), Rect::from_ltwh(0.0, 0.0, 360.0, 600.0));
            assert_eq!(
                rect_of(tester, 2),
                Rect::from_ltwh(384.0, 0.0, 616.0, 600.0)
            );
        });
    }

    #[test]
    fn the_panes_share_the_scaffold_s_own_width_not_the_window_s() {
        test_widgets(|tester| {
            let scaffold = ThreePaneScaffold::list_detail(
                expanded_directive(),
                list_and_detail(),
                pane(1),
                pane(2),
            );
            tester.set_surface_size(Some(Size::new(1000.0, 600.0)));
            tester.pump_widget(
                Directionality::new(
                    TextDirection::Ltr,
                    Padding::new(EdgeInsetsGeometry::only(80.0, 0.0, 0.0, 0.0)).child(scaffold),
                )
                .into_widget(),
            );
            assert_eq!(
                rect_of(tester, 2),
                Rect::from_ltwh(464.0, 0.0, 536.0, 600.0)
            );
        });
    }

    #[test]
    fn a_reflowed_pane_sits_under_its_anchor() {
        test_widgets(|tester| {
            let value = value(
                PaneAdaptedValue::Expanded,
                PaneAdaptedValue::Reflowed {
                    reflow_under: SupportingPaneScaffoldRole::MAIN,
                },
                PaneAdaptedValue::Hidden,
            );
            pump(
                tester,
                400.0,
                800.0,
                ThreePaneScaffold::supporting(
                    directive(
                        WindowWidthSizeClass::Compact,
                        WindowHeightSizeClass::Expanded,
                    ),
                    value,
                    pane(1),
                    pane(2),
                ),
            );
            assert_eq!(rect_of(tester, 1), Rect::from_ltwh(0.0, 0.0, 400.0, 388.0));
            assert_eq!(
                rect_of(tester, 2),
                Rect::from_ltwh(0.0, 412.0, 400.0, 388.0)
            );
        });
    }

    #[test]
    fn margins_move_only_the_edges_a_pane_shares_with_the_scaffold() {
        test_widgets(|tester| {
            let margins = EdgeInsetsGeometry::all(24.0);
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    expanded_directive(),
                    list_and_detail(),
                    pane(1).margins(margins.clone()),
                    pane(2).margins(margins),
                ),
            );
            assert_eq!(
                rect_of(tester, 1),
                Rect::from_ltrb(24.0, 24.0, 360.0, 576.0)
            );
            assert_eq!(
                rect_of(tester, 2),
                Rect::from_ltrb(384.0, 24.0, 976.0, 576.0)
            );
        });
    }

    #[test]
    fn directional_margins_follow_the_text_direction() {
        test_widgets(|tester| {
            pump_in(
                tester,
                TextDirection::Rtl,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    expanded_directive(),
                    list_and_detail(),
                    pane(1).margins(EdgeInsetsGeometry::directional(24.0, 0.0, 0.0, 0.0)),
                    pane(2),
                ),
            );
            assert_eq!(
                rect_of(tester, 1),
                Rect::from_ltrb(640.0, 0.0, 976.0, 600.0)
            );
        });
    }

    #[test]
    fn a_shaped_pane_is_clipped_to_its_shape() {
        test_widgets(|tester| {
            let shape =
                RoundedRectangleBorder::new(BorderSide::NONE, BorderRadius::circular(16.0).into());
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    expanded_directive(),
                    list_and_detail(),
                    pane(1).shape(shape),
                    pane(2),
                ),
            );
            expect(
                tester,
                &find::ancestor(find::by_key(ValueKey::new(1)), find::by_type::<ClipPath>()),
                finds_one_widget(),
            );
            expect(
                tester,
                &find::ancestor(find::by_key(ValueKey::new(2)), find::by_type::<ClipPath>()),
                finds_nothing(),
            );
        });
    }

    #[test]
    fn right_to_left_mirrors_the_panes() {
        test_widgets(|tester| {
            pump_in(
                tester,
                TextDirection::Rtl,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    expanded_directive(),
                    list_and_detail(),
                    pane(1),
                    pane(2),
                ),
            );
            assert_eq!(
                rect_of(tester, 1),
                Rect::from_ltwh(640.0, 0.0, 360.0, 600.0)
            );
            assert_eq!(rect_of(tester, 2), Rect::from_ltwh(0.0, 0.0, 616.0, 600.0));
        });
    }

    #[test]
    fn a_proportional_pane_takes_its_fraction_of_the_scaffold() {
        test_widgets(|tester| {
            let list = pane(1).preferred_width(PanePreferredSize::Proportion(0.25));
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    expanded_directive(),
                    list_and_detail(),
                    list,
                    pane(2),
                ),
            );
            assert_eq!(rect_of(tester, 1), Rect::from_ltwh(0.0, 0.0, 250.0, 600.0));
        });
    }

    #[test]
    fn a_tertiary_pane_takes_the_third_partition() {
        test_widgets(|tester| {
            let all = value(
                PaneAdaptedValue::Expanded,
                PaneAdaptedValue::Expanded,
                PaneAdaptedValue::Expanded,
            );
            pump(
                tester,
                1400.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    directive(WindowWidthSizeClass::Large, WindowHeightSizeClass::Expanded),
                    all,
                    pane(1),
                    pane(2),
                )
                .tertiary_pane(pane(3)),
            );
            assert_eq!(
                rect_of(tester, 3),
                Rect::from_ltwh(988.0, 0.0, 412.0, 600.0)
            );
        });
    }

    #[test]
    fn a_hidden_pane_stays_mounted_offstage() {
        test_widgets(|tester| {
            pump(tester, 1000.0, 600.0, list_detail_extra(list_and_detail()));
            assert!(find::by_key(ValueKey::new(3)).evaluate(tester).is_empty());
            assert_eq!(
                find::by_key(ValueKey::new(3))
                    .skip_offstage(false)
                    .evaluate(tester)
                    .len(),
                1
            );
        });
    }

    #[test]
    fn a_pane_that_appears_slides_in_on_the_spring_and_the_one_it_replaces_slides_out() {
        test_widgets(|tester| {
            pump(tester, 1000.0, 600.0, list_detail_extra(list_and_detail()));
            pump(tester, 1000.0, 600.0, list_detail_extra(list_and_extra()));

            // The list, of higher priority, takes the leftover width; the detail and the extra
            // pane are both drawn while they move.
            let extra_bounds = Rect::from_ltwh(640.0, 0.0, 360.0, 600.0);
            tester.pump_by(Duration::from_millis(50));
            let extra_content = rect_of(tester, 3);
            let detail_content = rect_of(tester, 2);
            assert!(
                extra_content.left > extra_bounds.left,
                "the extra pane is still sliding in from the right, at {}",
                extra_content.left
            );
            assert!(
                detail_content.left > 384.0,
                "the detail is sliding out to the right, at {}",
                detail_content.left
            );
            assert!(!find::by_key(ValueKey::new(2)).evaluate(tester).is_empty());

            tester.pump_and_settle();
            assert_eq!(rect_of(tester, 1), Rect::from_ltwh(0.0, 0.0, 616.0, 600.0));
            assert_eq!(rect_of(tester, 3), extra_bounds);
            assert!(find::by_key(ValueKey::new(2)).evaluate(tester).is_empty());
        });
    }

    #[test]
    fn a_shown_pane_moves_to_its_new_bounds_and_overshoots_on_the_way() {
        test_widgets(|tester| {
            let wide = list_and_detail();
            let one = value(
                PaneAdaptedValue::Expanded,
                PaneAdaptedValue::Hidden,
                PaneAdaptedValue::Hidden,
            );
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(expanded_directive(), wide, pane(1), pane(2)),
            );
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(expanded_directive(), one, pane(1), pane(2)),
            );

            let mut leftmost = f64::INFINITY;
            for _ in 0..60 {
                tester.pump_by(Duration::from_millis(16));
                leftmost = leftmost.min(rect_of(tester, 2).left);
            }
            assert!(
                leftmost < 0.0,
                "a damping ratio of 0.8 overshoots, leftmost {leftmost}"
            );
            tester.pump_and_settle();
            assert_eq!(rect_of(tester, 2), Rect::from_ltwh(0.0, 0.0, 1000.0, 600.0));
        });
    }
}
