//! Flutter counterpart: material_ui's `mergeable_material.dart`.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationStatus, CurvedAnimation, Curves,
};
use waft_embedder::{
    Canvas, Color, Offset, Path, PathBuilder, Radius, Size, TextBaseline, rrect_radii_elliptical,
    valo,
};
use waft_foundation::{App, Handle, Listener, Ref, UniqueKey};
use waft_painting::{
    Axis, AxisDirection, Border, BorderRadius, BorderRadiusGeometry, BorderSide, BoxDecoration,
    ClipContext,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, ContainerBoxParentData,
    ContainerParentDataMixin, ContainerRenderObjectData, ContainerRenderObjectMixin, RenderBox,
    RenderBoxContainerDefaultsMixin, RenderBoxData, RenderHandle, RenderListBodyData,
    RenderListBodyMixin, RenderObject, RenderObjectData,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    AnimatedContainer, BuildContext, Container, IntoWidget, KeyRef, ListBody,
    MultiChildRenderObjectWidget, RenderObjectWidget, SizedBox, State, StateData, StatefulWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, WidgetRef,
    get_axis_direction_from_axis_reverse_and_directionality,
};

use crate::colors::Colors;
use crate::divider::Divider;
use crate::material::{Material, MaterialType, k_material_edges};
use crate::slider_parts::draw_shadow;
use crate::theme::{K_THEME_ANIMATION_DURATION, Theme};

/// The base type for [`MaterialSlice`] and [`MaterialGap`].
///
/// All [`MergeableMaterialItem`] objects need a key.
#[derive(Clone, Debug)]
pub enum MergeableMaterialItem {
    /// A slice of `Material`.
    Slice(MaterialSlice),
    /// A gap between slices.
    Gap(MaterialGap),
}

impl MergeableMaterialItem {
    /// The key that matches this item from frame to frame.
    pub fn key(&self) -> &KeyRef {
        match self {
            MergeableMaterialItem::Slice(slice) => &slice.key,
            MergeableMaterialItem::Gap(gap) => &gap.key,
        }
    }

    fn is_gap(&self) -> bool {
        matches!(self, MergeableMaterialItem::Gap(_))
    }
}

impl From<MaterialSlice> for MergeableMaterialItem {
    fn from(slice: MaterialSlice) -> MergeableMaterialItem {
        MergeableMaterialItem::Slice(slice)
    }
}

impl From<MaterialGap> for MergeableMaterialItem {
    fn from(gap: MaterialGap) -> MergeableMaterialItem {
        MergeableMaterialItem::Gap(gap)
    }
}

/// A class that can be used as a child to [`MergeableMaterial`]. It is a slice of `Material`
/// that animates merging with other slices.
///
/// All [`MaterialSlice`] objects need a key.
#[derive(Clone)]
pub struct MaterialSlice {
    /// The key for this item of the list.
    pub key: KeyRef,
    /// The contents of this slice.
    pub child: WidgetRef,
    /// Defines the color for the slice.
    ///
    /// By default, the value of `color` is `ThemeData::card_color`.
    pub color: Option<Color>,
}

impl MaterialSlice {
    /// Creates a slice of `Material` that's mergeable within a [`MergeableMaterial`]; Dart's
    /// optional named arguments are the setters.
    pub fn new<K>(key: impl Into<KeyRef>, child: impl IntoWidget<K>) -> MaterialSlice {
        MaterialSlice {
            key: key.into(),
            child: child.into_widget(),
            color: None,
        }
    }

    /// Dart `MaterialSlice(color:)`.
    pub fn color(mut self, color: Color) -> MaterialSlice {
        self.color = Some(color);
        self
    }
}

impl fmt::Debug for MaterialSlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialSlice")
            .field("key", &self.key)
            .field("color", &self.color)
            .finish_non_exhaustive()
    }
}

/// A class that represents a gap within [`MergeableMaterial`].
///
/// All [`MaterialGap`] objects need a key.
#[derive(Clone, Debug)]
pub struct MaterialGap {
    /// The key for this item of the list.
    pub key: KeyRef,
    /// The main axis extent of this gap.
    pub size: f64,
}

impl MaterialGap {
    /// Creates a Material gap with a given size; Dart's optional named arguments are the
    /// setters.
    pub fn new(key: impl Into<KeyRef>) -> MaterialGap {
        MaterialGap {
            key: key.into(),
            size: 16.0,
        }
    }

    /// Dart `MaterialGap(size:)`.
    pub fn size(mut self, size: f64) -> MaterialGap {
        self.size = size;
        self
    }
}

/// Displays a list of [`MergeableMaterialItem`] children. The list contains [`MaterialSlice`]
/// items whose boundaries are either "merged" with adjacent items or separated by a
/// [`MaterialGap`]. The children are distributed along the given `main_axis` in the same way as
/// the children of a `ListBody`. When the list of children changes, gaps are automatically
/// animated open or closed as needed.
///
/// To enable this widget to correlate its list of children with the previous one, each child
/// must specify a key.
///
/// When a new gap is added to the list of children the adjacent items are animated apart.
/// Similarly when a gap is removed the adjacent items are brought back together.
///
/// When a new slice is added or removed, the app is responsible for animating the transition of
/// the slices, while the gaps will be animated automatically.
pub struct MergeableMaterial {
    pub key: Option<KeyRef>,
    /// The children of the mergeable material.
    pub children: Vec<MergeableMaterialItem>,
    /// The main layout axis.
    pub main_axis: Axis,
    /// The z-coordinate at which to place all the `Material` slices.
    pub elevation: f64,
    /// Whether connected pieces of `MaterialSlice` have dividers between them.
    pub has_dividers: bool,
    /// Defines the color used for dividers if `has_dividers` is true.
    pub divider_color: Option<Color>,
}

impl MergeableMaterial {
    /// Creates a mergeable Material list of items; Dart's optional named arguments are the
    /// setters.
    pub fn new() -> MergeableMaterial {
        MergeableMaterial {
            key: None,
            children: Vec::new(),
            main_axis: Axis::Vertical,
            elevation: 2.0,
            has_dividers: false,
            divider_color: None,
        }
    }

    /// Dart `MergeableMaterial(key:)`.
    pub fn key(mut self, key: KeyRef) -> MergeableMaterial {
        self.key = Some(key);
        self
    }

    /// Dart `MergeableMaterial(children:)`.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = MergeableMaterialItem>,
    ) -> MergeableMaterial {
        self.children = children.into_iter().collect();
        self
    }

    /// Dart `MergeableMaterial(mainAxis:)`.
    pub fn main_axis(mut self, main_axis: Axis) -> MergeableMaterial {
        self.main_axis = main_axis;
        self
    }

    /// Dart `MergeableMaterial(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> MergeableMaterial {
        self.elevation = elevation;
        self
    }

    /// Dart `MergeableMaterial(hasDividers:)`.
    pub fn has_dividers(mut self, has_dividers: bool) -> MergeableMaterial {
        self.has_dividers = has_dividers;
        self
    }

    /// Dart `MergeableMaterial(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> MergeableMaterial {
        self.divider_color = Some(divider_color);
        self
    }
}

impl Default for MergeableMaterial {
    fn default() -> MergeableMaterial {
        MergeableMaterial::new()
    }
}

impl fmt::Debug for MergeableMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MergeableMaterial")
            .field("main_axis", &self.main_axis)
            .field("elevation", &self.elevation)
            .finish_non_exhaustive()
    }
}

/// Dart's `_AnimationTuple`.
struct AnimationTuple {
    controller: Ref<AnimationController>,
    start_animation: Ref<CurvedAnimation>,
    end_animation: Ref<CurvedAnimation>,
    gap_animation: Ref<CurvedAnimation>,
    gap_start: f64,
}

impl AnimationTuple {
    fn dispose(self, app: &mut App) {
        self.controller.dispose(app);
        self.start_animation.dispose(app);
        self.end_animation.dispose(app);
        self.gap_animation.dispose(app);
    }
}

impl StatefulWidget for MergeableMaterial {
    type State = MergeableMaterialState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MergeableMaterialState {
        MergeableMaterialState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            children: Vec::new(),
            animation_tuples: HashMap::new(),
        }
    }
}

/// Dart's `_MergeableMaterialState`.
pub struct MergeableMaterialState {
    state: StateData<MergeableMaterial>,
    ticker_provider: TickerProviderStateMixinData,
    children: Vec<MergeableMaterialItem>,
    animation_tuples: HashMap<KeyRef, AnimationTuple>,
}

impl MergeableMaterialState {
    fn handle_tick(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
    }

    fn gap_tuple<'a>(self: Handle<Self>, app: &'a App, key: &KeyRef) -> &'a AnimationTuple {
        app.get(self)
            .animation_tuples
            .get(key)
            .expect("a gap has its animation tuple")
    }

    fn controller(self: Handle<Self>, app: &App, key: &KeyRef) -> Ref<AnimationController> {
        self.gap_tuple(app, key).controller.clone()
    }

    fn init_gap(self: Handle<Self>, app: &mut App, gap: &MaterialGap) {
        let controller = AnimationController::new(
            app,
            None,
            Some(K_THEME_ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let curve = || Curves::fast_out_slow_in();
        let start_animation = CurvedAnimation::new(app, controller.erase(), curve(), None);
        let end_animation = CurvedAnimation::new(app, controller.erase(), curve(), None);
        let gap_animation = CurvedAnimation::new(app, controller.erase(), curve(), None);
        controller.add_listener(app, Listener::handle_method(self, Self::handle_tick));
        app.get_mut(self).animation_tuples.insert(
            gap.key.clone(),
            AnimationTuple {
                controller,
                start_animation,
                end_animation,
                gap_animation,
                gap_start: 0.0,
            },
        );
    }

    fn dispose_tuple(self: Handle<Self>, app: &mut App, key: &KeyRef) {
        if let Some(tuple) = app.get_mut(self).animation_tuples.remove(key) {
            tuple.dispose(app);
        }
    }

    /// Dart's `_debugHasConsecutiveGaps`, which reads the widget's children whatever it is given.
    fn debug_has_consecutive_gaps(self: Handle<Self>, app: &App) -> bool {
        self.widget(app)
            .children
            .windows(2)
            .any(|pair| pair[0].is_gap() && pair[1].is_gap())
    }

    /// Dart's `_debugGapsAreValid`: no two gaps in a row, and neither end is a gap.
    fn debug_gaps_are_valid(
        self: Handle<Self>,
        app: &App,
        children: &[MergeableMaterialItem],
    ) -> bool {
        if self.debug_has_consecutive_gaps(app) {
            return false;
        }
        if let (Some(first), Some(last)) = (children.first(), children.last())
            && (first.is_gap() || last.is_gap())
        {
            return false;
        }
        true
    }

    fn insert_child(self: Handle<Self>, app: &mut App, index: usize, child: MergeableMaterialItem) {
        if let MergeableMaterialItem::Gap(gap) = &child {
            self.init_gap(app, gap);
        }
        app.get_mut(self).children.insert(index, child);
    }

    fn remove_child(self: Handle<Self>, app: &mut App, index: usize) -> MergeableMaterialItem {
        let child = app.get_mut(self).children.remove(index);
        if let MergeableMaterialItem::Gap(gap) = &child {
            self.dispose_tuple(app, &gap.key);
        }
        child
    }

    fn is_closing_gap(self: Handle<Self>, app: &App, index: usize) -> bool {
        let children = &app.get(self).children;
        if index < children.len().saturating_sub(1)
            && let MergeableMaterialItem::Gap(gap) = &children[index]
        {
            return self.controller(app, &gap.key).status(app) == AnimationStatus::Reverse;
        }
        false
    }

    fn remove_empty_gaps(self: Handle<Self>, app: &mut App) {
        for j in (0..app.get(self).children.len()).rev() {
            let dismissed = match &app.get(self).children[j] {
                MergeableMaterialItem::Gap(gap) => {
                    self.controller(app, &gap.key).status(app).is_dismissed()
                }
                MergeableMaterialItem::Slice(_) => false,
            };
            if dismissed {
                self.remove_child(app, j);
            }
        }
    }

    fn border_radius(
        self: Handle<Self>,
        app: &App,
        index: usize,
        start: bool,
        end: bool,
    ) -> BorderRadiusGeometry {
        let children = &app.get(self).children;
        let card_radius = k_material_edges(MaterialType::Card)
            .expect("the card material has a radius")
            .top_left;
        let mut start_radius = Radius::ZERO;
        let mut end_radius = Radius::ZERO;
        if index > 0
            && let MergeableMaterialItem::Gap(gap) = &children[index - 1]
        {
            let value = self.gap_tuple(app, &gap.key).start_animation.value(app);
            start_radius =
                Radius::lerp(Some(Radius::ZERO), Some(card_radius), value).expect("a radius lerps");
        }
        if index + 1 < children.len().saturating_sub(1)
            && let MergeableMaterialItem::Gap(gap) = &children[index + 1]
        {
            let value = self.gap_tuple(app, &gap.key).end_animation.value(app);
            end_radius =
                Radius::lerp(Some(Radius::ZERO), Some(card_radius), value).expect("a radius lerps");
        }

        let start_radius = if start { card_radius } else { start_radius };
        let end_radius = if end { card_radius } else { end_radius };
        match self.widget(app).main_axis {
            Axis::Vertical => BorderRadiusGeometry::vertical(start_radius, end_radius),
            Axis::Horizontal => BorderRadiusGeometry::BorderRadius(BorderRadius::horizontal(
                start_radius,
                end_radius,
            )),
        }
    }

    fn gap_size(self: Handle<Self>, app: &App, index: usize) -> f64 {
        let MergeableMaterialItem::Gap(gap) = &app.get(self).children[index] else {
            panic!("a gap is a gap");
        };
        let tuple = self.gap_tuple(app, &gap.key);
        waft_embedder::lerp_double(
            Some(tuple.gap_start),
            Some(gap.size),
            tuple.gap_animation.value(app),
        )
        .expect("a gap size lerps")
    }

    fn will_need_divider(self: Handle<Self>, app: &App, index: i64) -> bool {
        if index < 0 || index as usize >= app.get(self).children.len() {
            return false;
        }
        matches!(
            app.get(self).children[index as usize],
            MergeableMaterialItem::Slice(_)
        ) || self.is_closing_gap(app, index as usize)
    }
}

impl fmt::Debug for MergeableMaterialState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MergeableMaterialState")
            .field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}

impl TickerProviderStateMixin for MergeableMaterialState {
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
impl TickerProviderObject for MergeableMaterialState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for MergeableMaterialState {
    type Widget = MergeableMaterial;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let children = self.widget(app).children.clone();
        app.get_mut(self).children = children.clone();
        for child in &children {
            if let MergeableMaterialItem::Gap(gap) = child {
                self.init_gap(app, gap);
                // Gaps are initially full-sized.
                self.controller(app, &gap.key).set_value(app, 1.0);
            }
        }
        debug_assert!(self.debug_gaps_are_valid(app, &children));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MergeableMaterial) {
        let old_keys: HashSet<KeyRef> = old_widget
            .children
            .iter()
            .map(|child| child.key().clone())
            .collect();
        let new_children = self.widget(app).children.clone();
        let new_keys: HashSet<KeyRef> = new_children
            .iter()
            .map(|child| child.key().clone())
            .collect();
        let new_only: HashSet<KeyRef> = new_keys.difference(&old_keys).cloned().collect();
        let old_only: HashSet<KeyRef> = old_keys.difference(&new_keys).cloned().collect();

        debug_assert!(self.debug_gaps_are_valid(app, &new_children));

        self.remove_empty_gaps(app);

        let mut i = 0usize;
        let mut j = 0usize;
        while i < new_children.len() && j < app.get(self).children.len() {
            if new_only.contains(new_children[i].key())
                || old_only.contains(app.get(self).children[j].key())
            {
                let start_new = i;
                let start_old = j;

                while i < new_children.len() && new_only.contains(new_children[i].key()) {
                    i += 1;
                }
                while j < app.get(self).children.len()
                    && (old_only.contains(app.get(self).children[j].key())
                        || self.is_closing_gap(app, j))
                {
                    j += 1;
                }
                let new_length = i - start_new;
                let old_length = j - start_old;

                if new_length > 0 {
                    if old_length > 1
                        || (old_length == 1
                            && matches!(
                                app.get(self).children[start_old],
                                MergeableMaterialItem::Slice(_)
                            ))
                    {
                        if new_length == 1
                            && matches!(new_children[start_new], MergeableMaterialItem::Gap(_))
                        {
                            let mut gap_size_sum = 0.0;
                            while start_old < j {
                                if let MergeableMaterialItem::Gap(gap) =
                                    &app.get(self).children[start_old]
                                {
                                    gap_size_sum += gap.size;
                                }
                                self.remove_child(app, start_old);
                                j -= 1;
                            }
                            self.insert_child(app, start_old, new_children[start_new].clone());
                            let gap_key = new_children[start_new].key().clone();
                            if let Some(tuple) =
                                app.get_mut(self).animation_tuples.get_mut(&gap_key)
                            {
                                tuple.gap_start = gap_size_sum;
                            }
                            self.controller(app, &gap_key).forward(app, None);
                            j += 1;
                        } else {
                            for _ in 0..old_length {
                                self.remove_child(app, start_old);
                            }
                            for k in 0..new_length {
                                self.insert_child(
                                    app,
                                    start_old + k,
                                    new_children[start_new + k].clone(),
                                );
                            }
                            j = j + new_length - old_length;
                        }
                    } else if old_length == 1 {
                        let same_gap = new_length == 1
                            && matches!(new_children[start_new], MergeableMaterialItem::Gap(_))
                            && app.get(self).children[start_old].key()
                                == new_children[start_new].key();
                        if same_gap {
                            let key = new_children[start_new].key().clone();
                            self.controller(app, &key).forward(app, None);
                        } else {
                            let gap_size = self.gap_size(app, start_old);
                            self.remove_child(app, start_old);
                            for k in 0..new_length {
                                self.insert_child(
                                    app,
                                    start_old + k,
                                    new_children[start_new + k].clone(),
                                );
                            }
                            j = j + new_length - 1;
                            let mut gap_size_sum = 0.0;
                            for child in &new_children[start_new..i] {
                                if let MergeableMaterialItem::Gap(gap) = child {
                                    gap_size_sum += gap.size;
                                }
                            }
                            for child in &new_children[start_new..i] {
                                if let MergeableMaterialItem::Gap(gap) = child {
                                    let key = gap.key.clone();
                                    if let Some(tuple) =
                                        app.get_mut(self).animation_tuples.get_mut(&key)
                                    {
                                        tuple.gap_start = gap_size * gap.size / gap_size_sum;
                                    }
                                    let controller = self.controller(app, &key);
                                    controller.set_value(app, 0.0);
                                    controller.forward(app, None);
                                }
                            }
                        }
                    } else {
                        for k in 0..new_length {
                            let new_child = new_children[start_new + k].clone();
                            self.insert_child(app, start_old + k, new_child.clone());
                            if let MergeableMaterialItem::Gap(gap) = &new_child {
                                self.controller(app, &gap.key).forward(app, None);
                            }
                        }
                        j += new_length;
                    }
                } else if old_length > 1
                    || (old_length == 1
                        && matches!(
                            app.get(self).children[start_old],
                            MergeableMaterialItem::Slice(_)
                        ))
                {
                    let mut gap_size_sum = 0.0;
                    while start_old < j {
                        if let MergeableMaterialItem::Gap(gap) = &app.get(self).children[start_old]
                        {
                            gap_size_sum += gap.size;
                        }
                        self.remove_child(app, start_old);
                        j -= 1;
                    }
                    if gap_size_sum != 0.0 {
                        let gap =
                            MaterialGap::new(KeyRef::from(UniqueKey::new())).size(gap_size_sum);
                        self.insert_child(app, start_old, MergeableMaterialItem::Gap(gap));
                        let key = app.get(self).children[start_old].key().clone();
                        if let Some(tuple) = app.get_mut(self).animation_tuples.get_mut(&key) {
                            tuple.gap_start = 0.0;
                        }
                        let controller = self.controller(app, &key);
                        controller.set_value(app, 1.0);
                        controller.reverse(app, None);
                        j += 1;
                    }
                } else if old_length == 1 {
                    let MergeableMaterialItem::Gap(gap) = &app.get(self).children[start_old] else {
                        panic!("the old child is a gap");
                    };
                    let key = gap.key.clone();
                    if let Some(tuple) = app.get_mut(self).animation_tuples.get_mut(&key) {
                        tuple.gap_start = 0.0;
                    }
                    self.controller(app, &key).reverse(app, None);
                }
            } else if app.get(self).children[j].is_gap() == new_children[i].is_gap() {
                // Same kind: their places have been swapped.
                app.get_mut(self).children[j] = new_children[i].clone();
                i += 1;
                j += 1;
            } else {
                // A closing gap, which is skipped.
                debug_assert!(app.get(self).children[j].is_gap());
                j += 1;
            }
        }

        while j < app.get(self).children.len() {
            self.remove_child(app, j);
        }
        while i < new_children.len() {
            let new_child = new_children[i].clone();
            self.insert_child(app, j, new_child.clone());
            if let MergeableMaterialItem::Gap(gap) = &new_child {
                self.controller(app, &gap.key).forward(app, None);
            }
            i += 1;
            j += 1;
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let keys: Vec<KeyRef> = app.get(self).animation_tuples.keys().cloned().collect();
        for key in keys {
            self.dispose_tuple(app, &key);
        }
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        self.remove_empty_gaps(app);

        let theme = Theme::of(app, context);
        let main_axis = self.widget(app).main_axis;
        let elevation = self.widget(app).elevation;
        let has_dividers = self.widget(app).has_dividers;
        let divider_color = self.widget(app).divider_color;
        let mut widgets: Vec<WidgetRef> = Vec::new();
        let mut slices: Vec<WidgetRef> = Vec::new();

        for i in 0..app.get(self).children.len() {
            if app.get(self).children[i].is_gap() {
                debug_assert!(!slices.is_empty());
                widgets.push(
                    ListBody::new()
                        .main_axis(main_axis)
                        .children(std::mem::take(&mut slices))
                        .into_widget(),
                );
                let gap_size = self.gap_size(app, i);
                widgets.push(match main_axis {
                    Axis::Horizontal => SizedBox::new().width(gap_size).into_widget(),
                    Axis::Vertical => SizedBox::new().height(gap_size).into_widget(),
                });
            } else {
                let (slice_child, slice_color, slice_key) = match &app.get(self).children[i] {
                    MergeableMaterialItem::Slice(slice) => {
                        (slice.child.clone(), slice.color, slice.key.clone())
                    }
                    MergeableMaterialItem::Gap(_) => unreachable!(),
                };
                let mut child = slice_child;

                if has_dividers {
                    let has_top_divider = self.will_need_divider(app, i as i64 - 1);
                    let has_bottom_divider = self.will_need_divider(app, i as i64 + 1);
                    let divider =
                        Divider::create_border_side(app, Some(context), divider_color, Some(0.5));
                    let side = |shown: bool| if shown { divider } else { BorderSide::NONE };
                    let border = if i == 0 {
                        Border {
                            bottom: side(has_bottom_divider),
                            ..Border::from_border_side(BorderSide::NONE)
                        }
                    } else if i == app.get(self).children.len() - 1 {
                        Border {
                            top: side(has_top_divider),
                            ..Border::from_border_side(BorderSide::NONE)
                        }
                    } else {
                        Border {
                            top: side(has_top_divider),
                            bottom: side(has_bottom_divider),
                            ..Border::from_border_side(BorderSide::NONE)
                        }
                    };

                    child = AnimatedContainer::new(K_THEME_ANIMATION_DURATION)
                        .key(KeyRef::Global(waft_widgets::GlobalKey::wrapping(Rc::new(
                            MergeableMaterialSliceKey(slice_key),
                        ))))
                        .decoration(BoxDecoration::new().border(border))
                        .curve(Curves::fast_out_slow_in())
                        .child(child)
                        .into_widget();
                }

                let color = slice_color.unwrap_or(theme.card_color);
                slices.push(
                    Container::new()
                        .decoration(BoxDecoration::new().color(color).border_radius(
                            self.border_radius(
                                app,
                                i,
                                i == 0,
                                i == app.get(self).children.len() - 1,
                            ),
                        ))
                        .child(
                            Material::new()
                                .r#type(MaterialType::Transparency)
                                .child(child),
                        )
                        .into_widget(),
                );
            }
        }
        if !slices.is_empty() {
            widgets.push(
                ListBody::new()
                    .main_axis(main_axis)
                    .children(slices)
                    .into_widget(),
            );
        }

        MergeableMaterialListBody {
            key: None,
            main_axis,
            elevation,
            children: widgets,
        }
        .into_widget()
    }
}

/// Dart's `_MergeableMaterialSliceKey`: a global key that compares by the slice's local key.
#[derive(Debug)]
struct MergeableMaterialSliceKey(KeyRef);

impl waft_foundation::Key for MergeableMaterialSliceKey {
    fn eq_key(&self, other: &dyn waft_foundation::Key) -> bool {
        (other as &dyn std::any::Any)
            .downcast_ref::<MergeableMaterialSliceKey>()
            .is_some_and(|other| other.0 == self.0)
    }

    fn hash_key(&self, mut state: &mut dyn std::hash::Hasher) {
        std::any::TypeId::of::<Self>().hash(&mut state);
        self.0.hash(&mut state);
    }
}

/// Dart's `_MergeableMaterialListBody`.
struct MergeableMaterialListBody {
    key: Option<KeyRef>,
    main_axis: Axis,
    elevation: f64,
    children: Vec<WidgetRef>,
}

impl RenderObjectWidget for MergeableMaterialListBody {
    type RenderObject = RenderMergeableMaterialListBody;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let axis_direction = get_axis_direction_from_axis_reverse_and_directionality(
            app,
            context,
            self.main_axis,
            false,
        );
        let render_object = RenderMergeableMaterialListBody::new(app, axis_direction);
        render_object.set_elevation(app, self.elevation);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderMergeableMaterialListBody>,
    ) {
        let axis_direction = get_axis_direction_from_axis_reverse_and_directionality(
            app,
            context,
            self.main_axis,
            false,
        );
        render_object.set_axis_direction(app, axis_direction);
        render_object.set_elevation(app, self.elevation);
    }
}

impl MultiChildRenderObjectWidget for MergeableMaterialListBody {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

impl fmt::Debug for MergeableMaterialListBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MergeableMaterialListBody")
            .field("main_axis", &self.main_axis)
            .field("elevation", &self.elevation)
            .finish_non_exhaustive()
    }
}

/// Dart's `_RenderMergeableMaterialListBody`.
struct RenderMergeableMaterialListBody {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    list_body: RenderListBodyData,
    elevation: f64,
}

impl RenderMergeableMaterialListBody {
    fn new(
        app: &mut App,
        axis_direction: AxisDirection,
    ) -> RenderHandle<RenderMergeableMaterialListBody> {
        RenderHandle::new_box(
            app,
            RenderMergeableMaterialListBody {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                list_body: RenderListBodyData::new().axis_direction(axis_direction),
                elevation: 0.0,
            },
        )
    }

    fn elevation(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).elevation
    }

    fn set_elevation(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).elevation == value {
            return;
        }
        self.get_mut(app).elevation = value;
        self.mark_needs_paint(app);
    }

    fn paint_shadows(
        self: RenderHandle<Self>,
        app: &App,
        canvas: &mut Canvas,
        rect: waft_embedder::Rect,
    ) {
        let elevation = self.elevation(app);
        if elevation != 0.0 {
            let rrect = k_material_edges(MaterialType::Card)
                .expect("the card material has a radius")
                .to_rrect(rect);
            let mut path = PathBuilder::new();
            path.rrect_radii_elliptical(valo::Rect::from(rect), rrect_radii_elliptical(rrect));
            draw_shadow(
                app,
                canvas,
                self.as_object().as_box().expect("a list body is a box"),
                &Path::new(path.build()),
                Colors::BLACK,
                elevation,
                // The occluding object is not (necessarily) opaque.
                true,
            );
        }
    }
}

impl ContainerRenderObjectMixin for RenderMergeableMaterialListBody {
    type ChildType = AnyRenderBox;
    type ParentDataType = waft_rendering::ListBodyParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderMergeableMaterialListBody {}

impl RenderListBodyMixin for RenderMergeableMaterialListBody {
    fn list_body_data(self: RenderHandle<Self>, app: &App) -> &RenderListBodyData {
        &self.get(app).list_body
    }

    fn list_body_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderListBodyData {
        &mut self.get_mut(app).list_body
    }
}

impl RenderObject for RenderMergeableMaterialListBody {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderListBodyMixin::perform_layout(self, app)
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: Handle<waft_rendering::PipelineOwner>,
    ) {
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
        context: &mut waft_rendering::PaintingContext,
        offset: Offset,
    ) {
        let mut child = self.first_child(app);
        let mut index = 0;
        while let Some(current) = child {
            let parent_data = current
                .as_object()
                .parent_data_of::<waft_rendering::ListBodyParentData>(app);
            let child_offset = parent_data.box_parent_data().offset;
            let size = current.size(app);
            let rect = waft_embedder::Rect::from_ltwh(
                child_offset.dx() + offset.dx(),
                child_offset.dy() + offset.dy(),
                size.width(),
                size.height(),
            );
            if index % 2 == 0 {
                self.paint_shadows(app, context.canvas(), rect);
            }
            child = parent_data.next_sibling();
            index += 1;
        }
        RenderListBodyMixin::paint(self, app, context, offset)
    }
}

impl RenderBox for RenderMergeableMaterialListBody {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderListBodyMixin::setup_parent_data(self, app, child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderListBodyMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderListBodyMixin::compute_dry_layout(self, app, constraints)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderListBodyMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderListBodyMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderListBodyMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderListBodyMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderListBodyMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderListBodyMixin::hit_test_children(self, app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use waft_embedder::{Rect, Size};
    use waft_foundation::ValueKey;
    use waft_painting::{Border, BorderRadius, BorderRadiusGeometry, BorderSide, BoxDecoration};
    use waft_rendering::AnyRenderBox;
    use waft_rendering::testing::{PaintCall, assert_paints, paints};
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{
        AnimatedContainer, Container, DecoratedBox, IntoWidget, SingleChildScrollView, SizedBox,
        WidgetRef,
    };

    use super::{MaterialGap, MaterialSlice, MergeableMaterial, MergeableMaterialItem};
    use crate::app::MaterialApp;
    use crate::colors::Colors;
    use crate::material::{MaterialType, k_material_edges};
    use crate::scaffold::Scaffold;
    use crate::test_support::PaintInspection;
    use crate::theme_data::ThemeData;

    #[derive(Clone, Copy)]
    enum RadiusType {
        Sharp,
        Shifting,
        Round,
    }

    fn card_radius() -> waft_embedder::Radius {
        k_material_edges(MaterialType::Card)
            .expect("the card material has a radius")
            .top_left
    }

    /// Dart's `matches`: the corners at the top and at the bottom of `border_radius` are each
    /// sharp, on their way between sharp and the card's radius, or the card's radius.
    fn matches(border_radius: Option<BorderRadius>, top: RadiusType, bottom: RadiusType) {
        let border_radius = border_radius.expect("the slice has a border radius");
        matches_corners(border_radius.top_left, border_radius.top_right, top);
        matches_corners(
            border_radius.bottom_left,
            border_radius.bottom_right,
            bottom,
        );
    }

    /// One of the two `switch`es of Dart's `matches`, on the two corners of one end.
    fn matches_corners(
        left: waft_embedder::Radius,
        right: waft_embedder::Radius,
        radius_type: RadiusType,
    ) {
        let card_radius = card_radius();
        match radius_type {
            RadiusType::Sharp => {
                assert_eq!(left, waft_embedder::Radius::ZERO);
                assert_eq!(right, waft_embedder::Radius::ZERO);
            }
            RadiusType::Shifting => {
                for corner in [left, right] {
                    assert!(corner.x > 0.0, "{corner:?} is shifting");
                    assert!(corner.x < card_radius.x, "{corner:?} is shifting");
                    assert!(corner.y > 0.0, "{corner:?} is shifting");
                    assert!(corner.y < card_radius.y, "{corner:?} is shifting");
                }
            }
            RadiusType::Round => {
                assert_eq!(left, card_radius);
                assert_eq!(right, card_radius);
            }
        }
    }

    /// Dart's `getBorderRadius`: the border radius of the decoration of the `index`th
    /// `Container`, which depends on the exact structure Material and MergeableMaterial build.
    fn get_border_radius(tester: &WidgetTester, index: usize) -> Option<BorderRadius> {
        let containers = find::by_type::<Container>().evaluate(tester);
        tester.widget(containers[index], |container: &Container| {
            box_decoration(container.decoration.as_ref())
                .border_radius
                .map(|border_radius| match border_radius {
                    BorderRadiusGeometry::BorderRadius(border_radius) => border_radius,
                    _ => panic!("the radius is a BorderRadius"),
                })
        })
    }

    /// Dart's `isDivider`.
    fn is_divider(decoration: &BoxDecoration, top: bool, bottom: bool) -> bool {
        let side = BorderSide::new(
            waft_embedder::Color::new(0x1F000000),
            0.5,
            waft_painting::BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_INSIDE,
        );
        let side_if = |shown: bool| if shown { side } else { BorderSide::NONE };
        *decoration
            == BoxDecoration::new().border(Border {
                top: side_if(top),
                bottom: side_if(bottom),
                ..Border::from_border_side(BorderSide::NONE)
            })
    }

    /// Dart's `decoration! as BoxDecoration`.
    fn box_decoration(decoration: Option<&waft_painting::DecorationRef>) -> BoxDecoration {
        decoration
            .expect("the widget has a decoration")
            .as_any()
            .downcast_ref::<BoxDecoration>()
            .expect("the decoration is a BoxDecoration")
            .clone()
    }

    /// The decorations of every `AnimatedContainer`, in tree order.
    fn animated_container_decorations(tester: &WidgetTester) -> Vec<BoxDecoration> {
        find::by_type::<AnimatedContainer>()
            .evaluate(tester)
            .into_iter()
            .map(|container| {
                tester.widget(container, |container: &AnimatedContainer| {
                    box_decoration(container.decoration.as_ref())
                })
            })
            .collect()
    }

    /// Dart's `MaterialSlice(key: ValueKey<String>(key), child: SizedBox(width: 100.0, height: 100.0))`.
    fn slice(key: &'static str) -> MergeableMaterialItem {
        MaterialSlice::new(
            ValueKey::new(key),
            SizedBox::new().width(100.0).height(100.0),
        )
        .into()
    }

    /// Dart's `MaterialGap(key: ValueKey<String>(key))`.
    fn gap(key: &'static str) -> MergeableMaterialItem {
        MaterialGap::new(ValueKey::new(key)).into()
    }

    /// Dart's `Scaffold(body: SingleChildScrollView(child: material))`.
    fn scaffold(material: MergeableMaterial) -> Scaffold {
        Scaffold::new().body(SingleChildScrollView::new().child(material))
    }

    /// Dart's `MaterialApp(home: Scaffold(body: SingleChildScrollView(child: material)))`.
    fn app(material: MergeableMaterial) -> WidgetRef {
        MaterialApp::new().home(scaffold(material)).into_widget()
    }

    /// Dart's `MaterialApp(theme: theme, home: Scaffold(body: SingleChildScrollView(child:
    /// material)))`.
    fn themed_app(theme: ThemeData, material: MergeableMaterial) -> WidgetRef {
        MaterialApp::new()
            .theme(theme)
            .home(scaffold(material))
            .into_widget()
    }

    /// Dart's `ThemeData(useMaterial3: false)`.
    fn material2_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    /// Dart's `tester.renderObject(find.byType(MergeableMaterial))` as a `RenderBox`.
    fn mergeable_material_box(tester: &WidgetTester) -> AnyRenderBox {
        tester
            .render_object(find::by_type::<MergeableMaterial>())
            .as_box()
            .expect("MergeableMaterial renders a box")
    }

    /// Dart's `box.size.height`.
    fn height(tester: &WidgetTester, render_box: AnyRenderBox) -> f64 {
        tester.get_size(render_box).height()
    }

    #[test]
    fn mergeable_material_empty() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new()));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 0.0);
        });
    }

    #[test]
    fn mergeable_material_update_slice() {
        test_widgets(|tester| {
            let sized_slice = |height: f64| -> MergeableMaterialItem {
                MaterialSlice::new(
                    ValueKey::new("A"),
                    SizedBox::new().width(100.0).height(height),
                )
                .into()
            };
            tester.pump_widget(app(MergeableMaterial::new().children([sized_slice(100.0)])));

            let mut render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 100.0);

            tester.pump_widget(app(MergeableMaterial::new().children([sized_slice(200.0)])));

            render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 200.0);
        });
    }

    #[test]
    fn mergeable_material_swap_slices() {
        test_widgets(|tester| {
            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("B")])
            ));

            let mut render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );

            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("B"), slice("A")])
            ));

            render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 200.0);

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_paints_shadows() {
        test_widgets(|tester| {
            let theme = material2_theme(tester);
            tester.pump_widget(themed_app(
                theme,
                MergeableMaterial::new().children([slice("A")]),
            ));

            let rrect = k_material_edges(MaterialType::Card)
                .expect("the card material has a radius")
                .to_rrect(Rect::from_ltrb(0.0, 0.0, 800.0, 100.0));
            let ops =
                tester.painted_ops(tester.render_object(find::by_type::<MergeableMaterial>()));
            assert_paints(
                &ops,
                &[
                    PaintCall::shadow_with_elevation(2.0, None),
                    PaintCall::RRect {
                        rrect: Some(rrect),
                        color: Some(Colors::WHITE),
                        style: None,
                        has_mask_filter: Some(false),
                    },
                ],
            );
        });
    }

    #[test]
    fn mergeable_material_skips_shadow_for_zero_elevation() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new()
                .elevation(0.0)
                .children([slice("A")])));

            let ops =
                tester.painted_ops(tester.render_object(find::by_type::<MergeableMaterial>()));
            assert!(
                paints(&ops, &[PaintCall::shadow_with_elevation(0.0, None)]).is_err(),
                "MergeableMaterial paints no shadow at zero elevation"
            );
        });
    }

    #[test]
    fn mergeable_material_merge_gap() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
            ])));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );

            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("B")])
            ));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_separate_slices() {
        test_widgets(|tester| {
            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("B")])
            ));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
            ])));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_separate_merge_separate() {
        test_widgets(|tester| {
            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("B")])
            ));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
            ])));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );

            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("B")])
            ));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
            ])));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_insert_slice() {
        test_widgets(|tester| {
            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("C")])
            ));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                slice("B"),
                slice("C"),
            ])));

            assert_eq!(height(tester, render_box), 300.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Sharp,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_remove_slice() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                slice("B"),
                slice("C"),
            ])));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 300.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Sharp,
                RadiusType::Round,
            );

            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("C")])
            ));

            tester.pump();
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_insert_chunk() {
        test_widgets(|tester| {
            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("C")])
            ));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
                gap("y"),
                slice("C"),
            ])));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Round,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_remove_chunk() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
                gap("y"),
                slice("C"),
            ])));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Round,
                RadiusType::Round,
            );

            tester.pump_widget(app(
                MergeableMaterial::new().children([slice("A"), slice("C")])
            ));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 200.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Sharp,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Sharp,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_replace_gap_with_chunk() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("C"),
            ])));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("y"),
                slice("B"),
                gap("z"),
                slice("C"),
            ])));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Round,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_replace_chunk_with_gap() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
                gap("y"),
                slice("C"),
            ])));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 2),
                RadiusType::Round,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("z"),
                slice("C"),
            ])));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 332.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_insert_and_separate_slice() {
        test_widgets(|tester| {
            tester.pump_widget(app(MergeableMaterial::new().children([slice("A")])));

            let render_box = mergeable_material_box(tester);
            assert_eq!(height(tester, render_box), 100.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );

            tester.pump_widget(app(MergeableMaterial::new().children([
                slice("A"),
                gap("x"),
                slice("B"),
            ])));

            tester.pump_by(Duration::from_millis(100));
            assert!(height(tester, render_box) < 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Shifting,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Shifting,
                RadiusType::Round,
            );

            tester.pump_by(Duration::from_millis(100));
            assert_eq!(height(tester, render_box), 216.0);

            matches(
                get_border_radius(tester, 0),
                RadiusType::Round,
                RadiusType::Round,
            );
            matches(
                get_border_radius(tester, 1),
                RadiusType::Round,
                RadiusType::Round,
            );
        });
    }

    #[test]
    fn mergeable_material_dividers() {
        test_widgets(|tester| {
            let theme = material2_theme(tester);
            tester.pump_widget(themed_app(
                theme.clone(),
                MergeableMaterial::new().has_dividers(true).children([
                    slice("A"),
                    slice("B"),
                    slice("C"),
                    slice("D"),
                ]),
            ));

            let mut boxes = animated_container_decorations(tester);

            let mut offset = 0;

            assert!(is_divider(&boxes[offset], false, true));
            assert!(is_divider(&boxes[offset + 1], true, true));
            assert!(is_divider(&boxes[offset + 2], true, true));
            assert!(is_divider(&boxes[offset + 3], true, false));

            tester.pump_widget(themed_app(
                theme,
                MergeableMaterial::new().has_dividers(true).children([
                    slice("A"),
                    slice("B"),
                    gap("x"),
                    slice("C"),
                    slice("D"),
                ]),
            ));

            // Wait for dividers to shrink.
            tester.pump_by(Duration::from_millis(200));

            boxes = animated_container_decorations(tester);

            offset = 0;

            assert!(is_divider(&boxes[offset], false, true));
            assert!(is_divider(&boxes[offset + 1], true, false));
            assert!(is_divider(&boxes[offset + 2], false, true));
            assert!(is_divider(&boxes[offset + 3], true, false));
        });
    }

    #[test]
    fn mergeable_material_respects_divider_color() {
        test_widgets(|tester| {
            let divider_color = Colors::RED.primary();
            tester.pump_widget(app(MergeableMaterial::new()
                .has_dividers(true)
                .divider_color(divider_color)
                .children([slice("A"), slice("B")])));

            let decoration = tester.widget(
                find::by_type::<DecoratedBox>().last(),
                |decorated_box: &DecoratedBox| box_decoration(Some(&decorated_box.decoration)),
            );
            // Since we are getting the last DecoratedBox, it will have a Border.top.
            assert_eq!(
                decoration
                    .border
                    .expect("the decoration has a border")
                    .top()
                    .color,
                divider_color
            );
        });
    }

    #[test]
    fn mergeable_material_respects_material_slice_color() {
        test_widgets(|tester| {
            let theme_card_color = Colors::RED.primary();
            let material_slice_color = Colors::GREEN.primary();

            let theme = ThemeData::builder()
                .card_color(theme_card_color)
                .build(&tester.cell.borrow());
            tester.pump_widget(themed_app(
                theme,
                MergeableMaterial::new().children([
                    MaterialSlice::new(
                        ValueKey::new("A"),
                        SizedBox::new().height(100.0).width(100.0),
                    )
                    .color(material_slice_color)
                    .into(),
                    gap("B"),
                    MaterialSlice::new(
                        ValueKey::new("C"),
                        SizedBox::new().height(100.0).width(100.0),
                    )
                    .into(),
                ]),
            ));

            let container_color = |finder: waft_widgets::testing::Finder| {
                tester.widget(finder, |container: &Container| {
                    box_decoration(container.decoration.as_ref())
                        .color
                        .map(|color| color.color())
                })
            };
            assert_eq!(
                container_color(find::by_type::<Container>().first()),
                Some(material_slice_color)
            );
            assert_eq!(
                container_color(find::by_type::<Container>().last()),
                Some(theme_card_color)
            );
        });
    }

    #[test]
    fn mergeable_material_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.view().set_physical_size(Size::ZERO);
            tester.pump_widget(app(MergeableMaterial::new()));
            assert_eq!(
                tester.get_size(find::by_type::<MergeableMaterial>()),
                Size::ZERO
            );
        });
    }
}
