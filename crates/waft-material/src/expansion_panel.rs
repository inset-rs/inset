//! Flutter counterpart: material_ui's `expansion_panel.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Curves, Interval};
use waft_embedder::Color;
use waft_foundation::{AnyEq, App, Handle, HandleId, Listener, ValueKey};
use waft_painting::{EdgeInsets, EdgeInsetsDirectional};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    AnimatedContainer, AnimatedCrossFade, BuildContext, Column, ConstrainedBox, CrossFadeState,
    Expanded, IgnorePointer, IntoWidget, KeyRef, LimitedBox, MergeSemantics, Padding, Row,
    Semantics, SizedBox, State, StateData, StatefulWidget, WidgetRef,
};

use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::expand_icon::ExpandIcon;
use crate::ink_well::InkWell;
use crate::material_localizations::MaterialLocalizations;
use crate::mergeable_material::{
    MaterialGap, MaterialSlice, MergeableMaterial, MergeableMaterialItem,
};
use crate::shadows::k_elevation_to_shadow;
use crate::theme::K_THEME_ANIMATION_DURATION;

const K_PANEL_HEADER_COLLAPSED_HEIGHT: f64 = K_MIN_INTERACTIVE_DIMENSION;
const K_PANEL_HEADER_EXPANDED_DEFAULT_PADDING: EdgeInsets =
    EdgeInsets::symmetric(64.0 - K_PANEL_HEADER_COLLAPSED_HEIGHT, 0.0);
const K_EXPAND_ICON_PADDING: EdgeInsets = EdgeInsets::all(12.0);

/// Signature for the callback that's called when an [`ExpansionPanel`] is expanded or
/// collapsed.
///
/// The position of the panel within an [`ExpansionPanelList`] is given by `panel_index`.
pub type ExpansionPanelCallback = Rc<dyn Fn(&mut App, usize, bool)>;

/// Signature for the callback that's called when the header of the [`ExpansionPanel`] needs to
/// rebuild.
pub type ExpansionPanelHeaderBuilder = Rc<dyn Fn(&mut App, BuildContext, bool) -> WidgetRef>;

/// A material expansion panel. It has a header and a body and can be either expanded or
/// collapsed. The body of the panel is only visible when it is expanded.
///
/// Expansion panels are only intended to be used as children for [`ExpansionPanelList`].
#[derive(Clone)]
pub struct ExpansionPanel {
    /// The widget builder that builds the expansion panels' header.
    pub header_builder: ExpansionPanelHeaderBuilder,
    /// The body of the expansion panel that's displayed below the header.
    ///
    /// This widget is visible only when the panel is expanded.
    pub body: WidgetRef,
    /// Whether the panel is expanded.
    pub is_expanded: bool,
    /// Defines the splash color of the panel if [`can_tap_on_header`](Self::can_tap_on_header)
    /// is true, or the splash color of the expand/collapse IconButton if it is false.
    pub splash_color: Option<Color>,
    /// Defines the highlight color of the panel if
    /// [`can_tap_on_header`](Self::can_tap_on_header) is true, or the highlight color of the
    /// expand/collapse IconButton if it is false.
    pub highlight_color: Option<Color>,
    /// Whether tapping on the panel's header will expand/collapse it.
    pub can_tap_on_header: bool,
    /// Defines the background color of the panel.
    ///
    /// Defaults to `ThemeData::card_color`.
    pub background_color: Option<Color>,
}

impl ExpansionPanel {
    /// Creates an expansion panel to be used as a child for [`ExpansionPanelList`]; Dart's
    /// optional named arguments are the setters.
    pub fn new<K>(header_builder: ExpansionPanelHeaderBuilder, body: impl IntoWidget<K>) -> Self {
        ExpansionPanel {
            header_builder,
            body: body.into_widget(),
            is_expanded: false,
            splash_color: None,
            highlight_color: None,
            can_tap_on_header: false,
            background_color: None,
        }
    }

    /// Dart `ExpansionPanel(isExpanded:)`.
    pub fn is_expanded(mut self, is_expanded: bool) -> Self {
        self.is_expanded = is_expanded;
        self
    }

    /// Dart `ExpansionPanel(canTapOnHeader:)`.
    pub fn can_tap_on_header(mut self, can_tap_on_header: bool) -> Self {
        self.can_tap_on_header = can_tap_on_header;
        self
    }

    /// Dart `ExpansionPanel(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `ExpansionPanel(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> Self {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `ExpansionPanel(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> Self {
        self.highlight_color = Some(highlight_color);
        self
    }
}

impl fmt::Debug for ExpansionPanel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionPanel")
            .field("is_expanded", &self.is_expanded)
            .field("can_tap_on_header", &self.can_tap_on_header)
            .finish_non_exhaustive()
    }
}

/// Dart's `==` on two radio values: the values', not the shared pointers'.
fn same_value(a: &Rc<dyn AnyEq>, b: &Rc<dyn AnyEq>) -> bool {
    a == b
}

/// An expansion panel that allows for radio-like functionality. This means that at any given
/// time, at most, one [`ExpansionPanelRadio`] can remain expanded.
///
/// A unique identifier [`value`](Self::value) must be assigned to each panel. This identifier
/// allows the [`ExpansionPanelList`] to determine which [`ExpansionPanelRadio`] instance should
/// be expanded.
///
/// See [`ExpansionPanelList::radio`] for a sample implementation.
#[derive(Clone)]
pub struct ExpansionPanelRadio {
    /// Dart's `extends ExpansionPanel`: the fields it inherits.
    pub panel: ExpansionPanel,
    /// The value that uniquely identifies a radio panel so that the currently selected radio
    /// panel can be identified.
    pub value: Rc<dyn AnyEq>,
}

impl ExpansionPanelRadio {
    /// An expansion panel that allows for radio functionality; Dart's optional named arguments
    /// are the setters.
    pub fn new<K>(
        value: impl AnyEq,
        header_builder: ExpansionPanelHeaderBuilder,
        body: impl IntoWidget<K>,
    ) -> Self {
        ExpansionPanelRadio {
            panel: ExpansionPanel::new(header_builder, body),
            value: Rc::new(value),
        }
    }

    /// Dart `ExpansionPanelRadio(canTapOnHeader:)`.
    pub fn can_tap_on_header(mut self, can_tap_on_header: bool) -> Self {
        self.panel = self.panel.can_tap_on_header(can_tap_on_header);
        self
    }

    /// Dart `ExpansionPanelRadio(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> Self {
        self.panel = self.panel.background_color(background_color);
        self
    }

    /// Dart `ExpansionPanelRadio(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> Self {
        self.panel = self.panel.splash_color(splash_color);
        self
    }

    /// Dart `ExpansionPanelRadio(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> Self {
        self.panel = self.panel.highlight_color(highlight_color);
        self
    }
}

impl fmt::Debug for ExpansionPanelRadio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionPanelRadio")
            .field("panel", &self.panel)
            .finish_non_exhaustive()
    }
}

/// A material expansion panel list that lays out its children and animates expansions.
///
/// The [`expansion_callback`](Self::expansion_callback) is called when the expansion state
/// changes. For normal [`ExpansionPanelList`] widgets, it is the responsibility of the parent
/// widget to rebuild the [`ExpansionPanelList`] with updated values for
/// [`ExpansionPanel::is_expanded`]. For [`ExpansionPanelList::radio`] widgets, the open state is
/// tracked internally and the callback is invoked both for the previously open panel, which is
/// closing, and the previously closed panel, which is opening.
pub struct ExpansionPanelList {
    pub key: Option<KeyRef>,
    /// The children of the expansion panel list. They are laid out in a similar fashion to
    /// `ListBody`.
    pub children: Vec<ExpansionPanel>,
    /// The callback that gets called whenever one of the expand/collapse buttons is pressed.
    /// The arguments passed to the callback are the index of the pressed panel and whether the
    /// panel is currently expanded or not.
    pub expansion_callback: Option<ExpansionPanelCallback>,
    /// The duration of the expansion animation.
    pub animation_duration: Duration,
    // Whether multiple panels can be open simultaneously.
    allow_only_one_panel_open: bool,
    // Dart's `ExpansionPanelRadio.value` of each child of a radio list, in order; empty
    // otherwise.
    radio_values: Vec<Rc<dyn AnyEq>>,
    /// The value of the panel that initially begins open. (This value is only used when
    /// initializing with the [`ExpansionPanelList::radio`] constructor.)
    pub initial_open_panel_value: Option<Rc<dyn AnyEq>>,
    /// The padding that surrounds the panel header when expanded.
    ///
    /// By default, 16px of space is added to the header vertically (above and below) during
    /// expansion.
    pub expanded_header_padding: EdgeInsets,
    /// Defines color for the divider when [`ExpansionPanel::is_expanded`] is false.
    ///
    /// If `divider_color` is `None`, then `DividerThemeData::color` is used. If that is `None`,
    /// then `ThemeData::divider_color` is used.
    pub divider_color: Option<Color>,
    /// Defines elevation for the [`ExpansionPanel`] while it's expanded.
    ///
    /// By default, the value of elevation is 2.
    pub elevation: f64,
    /// The color of the icon. Defaults to `Colors::BLACK54` when the theme's brightness is
    /// light and to `Colors::WHITE60` when it is dark.
    pub expand_icon_color: Option<Color>,
    /// Defines the [`MaterialGap::size`] of the gap between panels when expanded.
    ///
    /// Defaults to 16.0.
    pub material_gap_size: f64,
}

impl ExpansionPanelList {
    /// Creates an expansion panel list widget. The `expansion_callback` is triggered when an
    /// expansion panel expand/collapse button is pushed; Dart's optional named arguments are
    /// the setters.
    pub fn new() -> ExpansionPanelList {
        ExpansionPanelList {
            key: None,
            children: Vec::new(),
            expansion_callback: None,
            animation_duration: K_THEME_ANIMATION_DURATION,
            allow_only_one_panel_open: false,
            radio_values: Vec::new(),
            initial_open_panel_value: None,
            expanded_header_padding: K_PANEL_HEADER_EXPANDED_DEFAULT_PADDING,
            divider_color: None,
            elevation: 2.0,
            expand_icon_color: None,
            material_gap_size: 16.0,
        }
    }

    /// Creates a radio expansion panel list widget.
    ///
    /// This widget allows for at most one panel in the list to be open. The expansion panel
    /// callback is triggered when an expansion panel expand/collapse button is pushed. The
    /// `children` are radio panels, each with a unique value.
    pub fn radio(children: impl IntoIterator<Item = ExpansionPanelRadio>) -> ExpansionPanelList {
        let (children, radio_values) = children
            .into_iter()
            .map(|radio| (radio.panel, radio.value))
            .unzip();
        ExpansionPanelList {
            children,
            allow_only_one_panel_open: true,
            radio_values,
            ..ExpansionPanelList::new()
        }
    }

    /// Dart `ExpansionPanelList(key:)`.
    pub fn key(mut self, key: KeyRef) -> ExpansionPanelList {
        self.key = Some(key);
        self
    }

    /// Dart `ExpansionPanelList(children:)`.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = ExpansionPanel>,
    ) -> ExpansionPanelList {
        debug_assert!(
            !self.allow_only_one_panel_open,
            "a radio list takes its children in ExpansionPanelList::radio"
        );
        self.children = children.into_iter().collect();
        self
    }

    /// Dart `ExpansionPanelList(expansionCallback:)`.
    pub fn expansion_callback(
        mut self,
        expansion_callback: ExpansionPanelCallback,
    ) -> ExpansionPanelList {
        self.expansion_callback = Some(expansion_callback);
        self
    }

    /// Dart `ExpansionPanelList(animationDuration:)`.
    pub fn animation_duration(mut self, animation_duration: Duration) -> ExpansionPanelList {
        self.animation_duration = animation_duration;
        self
    }

    /// Dart `ExpansionPanelList.radio(initialOpenPanelValue:)`.
    pub fn initial_open_panel_value(mut self, value: impl AnyEq) -> ExpansionPanelList {
        self.initial_open_panel_value = Some(Rc::new(value));
        self
    }

    /// Dart `ExpansionPanelList(expandedHeaderPadding:)`.
    pub fn expanded_header_padding(
        mut self,
        expanded_header_padding: EdgeInsets,
    ) -> ExpansionPanelList {
        self.expanded_header_padding = expanded_header_padding;
        self
    }

    /// Dart `ExpansionPanelList(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> ExpansionPanelList {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `ExpansionPanelList(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> ExpansionPanelList {
        self.elevation = elevation;
        self
    }

    /// Dart `ExpansionPanelList(expandIconColor:)`.
    pub fn expand_icon_color(mut self, expand_icon_color: Color) -> ExpansionPanelList {
        self.expand_icon_color = Some(expand_icon_color);
        self
    }

    /// Dart `ExpansionPanelList(materialGapSize:)`.
    pub fn material_gap_size(mut self, material_gap_size: f64) -> ExpansionPanelList {
        self.material_gap_size = material_gap_size;
        self
    }
}

impl Default for ExpansionPanelList {
    fn default() -> ExpansionPanelList {
        ExpansionPanelList::new()
    }
}

impl fmt::Debug for ExpansionPanelList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionPanelList")
            .field("children", &self.children.len())
            .field("elevation", &self.elevation)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ExpansionPanelList {
    type State = ExpansionPanelListState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ExpansionPanelListState {
        ExpansionPanelListState {
            state: StateData::new(),
            current_open_panel: None,
        }
    }
}

/// Dart's `_ExpansionPanelListState`.
pub struct ExpansionPanelListState {
    state: StateData<ExpansionPanelList>,
    // Dart keeps the open `ExpansionPanelRadio`; its value is all that is read.
    current_open_panel: Option<Rc<dyn AnyEq>>,
}

impl ExpansionPanelListState {
    fn all_identifiers_unique(self: Handle<Self>, app: &App) -> bool {
        let list = self.widget(app);
        let mut unique: Vec<&Rc<dyn AnyEq>> = Vec::new();
        for value in &list.radio_values {
            if !unique.iter().any(|seen| same_value(seen, value)) {
                unique.push(value);
            }
        }
        unique.len() == list.children.len()
    }

    fn is_child_expanded(self: Handle<Self>, app: &App, index: usize) -> bool {
        let list = self.widget(app);
        if list.allow_only_one_panel_open {
            let radio_value = &list.radio_values[index];
            return app
                .get(self)
                .current_open_panel
                .as_ref()
                .is_some_and(|open| same_value(open, radio_value));
        }
        list.children[index].is_expanded
    }

    fn handle_pressed(self: Handle<Self>, app: &mut App, is_expanded: bool, index: usize) {
        if self.widget(app).allow_only_one_panel_open {
            let pressed_child = self.widget(app).radio_values[index].clone();

            // If another ExpansionPanelRadio was already open, apply its expansionCallback (if
            // any) to false, because it's closing.
            for child_index in 0..self.widget(app).children.len() {
                let callback = self.widget(app).expansion_callback.clone();
                let child = &self.widget(app).radio_values[child_index];
                let is_open = app
                    .get(self)
                    .current_open_panel
                    .as_ref()
                    .is_some_and(|open| same_value(open, child));
                if let Some(callback) = callback
                    && child_index != index
                    && is_open
                {
                    callback(app, child_index, false);
                }
            }

            self.set_state(app, |state| {
                state.current_open_panel = if is_expanded {
                    None
                } else {
                    Some(pressed_child)
                };
            });
        }
        // !is_expanded is passed because, when handle_pressed runs, the state of the panel to
        // expand is not yet expanded.
        if let Some(callback) = self.widget(app).expansion_callback.clone() {
            callback(app, index, !is_expanded);
        }
    }

    fn search_panel_by_value(
        self: Handle<Self>,
        app: &App,
        value: Option<&Rc<dyn AnyEq>>,
    ) -> Option<Rc<dyn AnyEq>> {
        let value = value?;
        self.widget(app)
            .radio_values
            .iter()
            .find(|panel| same_value(panel, value))
            .cloned()
    }

    fn header(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
        child: &ExpansionPanel,
    ) -> WidgetRef {
        let list = self.widget(app);
        let animation_duration = list.animation_duration;
        let expanded_header_padding = list.expanded_header_padding;
        let expand_icon_color = list.expand_icon_color;
        let expanded = self.is_child_expanded(app, index);
        let header_widget = (child.header_builder)(app, context, expanded);

        let mut expand_icon =
            ExpandIcon::new(Some(Rc::new(move |app: &mut App, is_expanded: bool| {
                self.handle_pressed(app, is_expanded, index)
            })))
            .is_expanded(expanded)
            .padding(K_EXPAND_ICON_PADDING);
        if let Some(color) = expand_icon_color {
            expand_icon = expand_icon.color(color);
        }
        if let Some(splash_color) = child.splash_color {
            expand_icon = expand_icon.splash_color(splash_color);
        }
        if let Some(highlight_color) = child.highlight_color {
            expand_icon = expand_icon.highlight_color(highlight_color);
        }
        let mut expand_icon_padded =
            Padding::new(EdgeInsetsDirectional::only(0.0, 0.0, 8.0, 0.0).into())
                .child(
                    IgnorePointer::new()
                        .ignoring(child.can_tap_on_header)
                        .child(expand_icon),
                )
                .into_widget();

        if !child.can_tap_on_header {
            let localizations = <dyn MaterialLocalizations>::of(app, context);
            expand_icon_padded = Semantics::new()
                .label(if expanded {
                    localizations.expanded_icon_tap_hint()
                } else {
                    localizations.collapsed_icon_tap_hint()
                })
                .container(true)
                .child(expand_icon_padded)
                .into_widget();
        }
        let header = Row::new()
            .children(vec![
                Expanded::new(
                    AnimatedContainer::new(animation_duration)
                        .curve(Curves::fast_out_slow_in())
                        .margin(if expanded {
                            expanded_header_padding.into()
                        } else {
                            EdgeInsets::ZERO.into()
                        })
                        .child(
                            ConstrainedBox::new(
                                BoxConstraints::new().min_height(K_PANEL_HEADER_COLLAPSED_HEIGHT),
                            )
                            .child(header_widget),
                        ),
                )
                .into_widget(),
                expand_icon_padded,
            ])
            .into_widget();
        if !child.can_tap_on_header {
            return header;
        }
        let mut ink_well = InkWell::new()
            .on_tap(Listener::new(move |app: &mut App| {
                let expanded = self.is_child_expanded(app, index);
                self.handle_pressed(app, expanded, index);
            }))
            .child(header);
        if let Some(splash_color) = child.splash_color {
            ink_well = ink_well.splash_color(splash_color);
        }
        if let Some(highlight_color) = child.highlight_color {
            ink_well = ink_well.highlight_color(highlight_color);
        }
        MergeSemantics::new().child(ink_well).into_widget()
    }

    fn body(self: Handle<Self>, app: &App, index: usize, child: &ExpansionPanel) -> WidgetRef {
        AnimatedCrossFade::new(
            LimitedBox::new()
                .max_width(0.0)
                .child(SizedBox::new().width(f64::INFINITY).height(0.0)),
            child.body.clone(),
            if self.is_child_expanded(app, index) {
                CrossFadeState::ShowSecond
            } else {
                CrossFadeState::ShowFirst
            },
            self.widget(app).animation_duration,
        )
        .first_curve(Rc::new(Interval::new(0.0, 0.6, Curves::fast_out_slow_in())))
        .second_curve(Rc::new(Interval::new(0.4, 1.0, Curves::fast_out_slow_in())))
        .size_curve(Curves::fast_out_slow_in())
        .into_widget()
    }
}

impl fmt::Debug for ExpansionPanelListState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionPanelListState")
            .finish_non_exhaustive()
    }
}

impl State for ExpansionPanelListState {
    type Widget = ExpansionPanelList;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if self.widget(app).allow_only_one_panel_open {
            debug_assert!(
                self.all_identifiers_unique(app),
                "All ExpansionPanelRadio identifier values must be unique."
            );
            let initial_open_panel_value = self.widget(app).initial_open_panel_value.clone();
            if initial_open_panel_value.is_some() {
                app.get_mut(self).current_open_panel =
                    self.search_panel_by_value(app, initial_open_panel_value.as_ref());
            }
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ExpansionPanelList) {
        if self.widget(app).allow_only_one_panel_open {
            debug_assert!(
                self.all_identifiers_unique(app),
                "All ExpansionPanelRadio identifier values must be unique."
            );
            // If the previous widget was non-radio ExpansionPanelList, initialize the open panel
            // to widget.initial_open_panel_value.
            if !old_widget.allow_only_one_panel_open {
                let initial_open_panel_value = self.widget(app).initial_open_panel_value.clone();
                app.get_mut(self).current_open_panel =
                    self.search_panel_by_value(app, initial_open_panel_value.as_ref());
            }
        } else {
            app.get_mut(self).current_open_panel = None;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let elevation = self.widget(app).elevation;
        debug_assert!(
            elevation.fract() == 0.0 && k_elevation_to_shadow(elevation as i32).is_some(),
            "Invalid value for elevation. See the kElevationToShadow constant for possible \
             elevation values."
        );

        let children = self.widget(app).children.clone();
        let material_gap_size = self.widget(app).material_gap_size;
        let mut items: Vec<MergeableMaterialItem> = Vec::new();

        for (index, child) in children.iter().enumerate() {
            if self.is_child_expanded(app, index)
                && index != 0
                && !self.is_child_expanded(app, index - 1)
            {
                items.push(
                    MaterialGap::new(salted_key(context, index * 2 - 1))
                        .size(material_gap_size)
                        .into(),
                );
            }

            let header = self.header(app, context, index, child);
            let body = self.body(app, index, child);
            let mut slice = MaterialSlice::new(
                salted_key(context, index * 2),
                Column::new().children(vec![header, body]),
            );
            if let Some(background_color) = child.background_color {
                slice = slice.color(background_color);
            }
            items.push(slice.into());

            if self.is_child_expanded(app, index) && index != children.len() - 1 {
                items.push(
                    MaterialGap::new(salted_key(context, index * 2 + 1))
                        .size(material_gap_size)
                        .into(),
                );
            }
        }

        let mut material = MergeableMaterial::new()
            .has_dividers(true)
            .elevation(elevation)
            .children(items);
        if let Some(divider_color) = self.widget(app).divider_color {
            material = material.divider_color(divider_color);
        }
        material.into_widget()
    }
}

/// Dart's `_SaltedKey<BuildContext, int>`: a local key that is this list's own, never equal to
/// a key the app gives.
fn salted_key(context: BuildContext, value: usize) -> KeyRef {
    #[derive(Debug, PartialEq, Hash)]
    struct SaltedKey(HandleId, usize);
    KeyRef::from(ValueKey::new(SaltedKey(context.id(), value)))
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Color, Rect};
    use waft_foundation::{App, Handle, Listener, ValueKey};
    use waft_painting::{AlignmentGeometry, BoxDecoration, DecorationRef, EdgeInsets};
    use waft_rendering::AnyRenderBox;
    use waft_widgets::testing::{
        Finder, WidgetTester, assert_f32_equals, expect, find, finds_n_widgets, finds_nothing,
        finds_one_widget, test_widgets,
    };
    use waft_widgets::{
        AnimatedContainer, AnimatedSize, BuildContext, Column, Container, DecoratedBox,
        IgnorePointer, IntoWidget, KeyRef, LayoutBuilder, ListView, Placeholder,
        SingleChildScrollView, SizedBox, State, StateData, StateSetter, StatefulBuilder,
        StatefulWidget, Text, WidgetRef, downcast_widget,
    };

    use super::{
        ExpansionPanel, ExpansionPanelCallback, ExpansionPanelHeaderBuilder, ExpansionPanelList,
        ExpansionPanelRadio,
    };
    use crate::app::MaterialApp;
    use crate::colors::Colors;
    use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
    use crate::expand_icon::ExpandIcon;
    use crate::floating_action_button::FloatingActionButton;
    use crate::icon_button::IconButton;
    use crate::ink_well::InkWell;
    use crate::list_tile::ListTile;
    use crate::mergeable_material::{MergeableMaterial, MergeableMaterialItem};
    use crate::scaffold::Scaffold;
    use crate::theme::K_THEME_ANIMATION_DURATION;

    /// Dart's `SimpleExpansionPanelListTestWidget`: two panels, each toggled by the list's
    /// callback.
    #[derive(Debug)]
    struct SimpleExpansionPanelListTestWidget {
        first_panel_key: Option<KeyRef>,
        second_panel_key: Option<KeyRef>,
        can_tap_on_header: bool,
        divider_color: Option<Color>,
        elevation: f64,
        /// If `None`, the default `ExpansionPanelList`'s expanded header padding value is
        /// applied via [`default_expanded_header_padding`](Self::default_expanded_header_padding).
        expanded_header_padding: Option<EdgeInsets>,
    }

    impl SimpleExpansionPanelListTestWidget {
        fn new() -> SimpleExpansionPanelListTestWidget {
            SimpleExpansionPanelListTestWidget {
                first_panel_key: None,
                second_panel_key: None,
                can_tap_on_header: false,
                divider_color: None,
                elevation: 2.0,
                expanded_header_padding: None,
            }
        }

        fn first_panel_key(mut self, key: KeyRef) -> SimpleExpansionPanelListTestWidget {
            self.first_panel_key = Some(key);
            self
        }

        fn second_panel_key(mut self, key: KeyRef) -> SimpleExpansionPanelListTestWidget {
            self.second_panel_key = Some(key);
            self
        }

        fn can_tap_on_header(
            mut self,
            can_tap_on_header: bool,
        ) -> SimpleExpansionPanelListTestWidget {
            self.can_tap_on_header = can_tap_on_header;
            self
        }

        fn divider_color(mut self, divider_color: Color) -> SimpleExpansionPanelListTestWidget {
            self.divider_color = Some(divider_color);
            self
        }

        fn elevation(mut self, elevation: f64) -> SimpleExpansionPanelListTestWidget {
            self.elevation = elevation;
            self
        }

        fn expanded_header_padding(
            mut self,
            expanded_header_padding: EdgeInsets,
        ) -> SimpleExpansionPanelListTestWidget {
            self.expanded_header_padding = Some(expanded_header_padding);
            self
        }

        /// Mirrors the default expanded header padding as its source constants are private.
        fn default_expanded_header_padding() -> EdgeInsets {
            ExpansionPanelList::new().expanded_header_padding
        }
    }

    struct SimpleExpansionPanelListTestWidgetState {
        state: StateData<SimpleExpansionPanelListTestWidget>,
        extended_state: [bool; 2],
    }

    impl StatefulWidget for SimpleExpansionPanelListTestWidget {
        type State = SimpleExpansionPanelListTestWidgetState;

        fn create_state(&self) -> SimpleExpansionPanelListTestWidgetState {
            SimpleExpansionPanelListTestWidgetState {
                state: StateData::new(),
                extended_state: [false, false],
            }
        }
    }

    impl State for SimpleExpansionPanelListTestWidgetState {
        type Widget = SimpleExpansionPanelListTestWidget;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let widget = self.widget(app);
            let first_panel_key = widget.first_panel_key.clone();
            let second_panel_key = widget.second_panel_key.clone();
            let can_tap_on_header = widget.can_tap_on_header;
            let divider_color = widget.divider_color;
            let elevation = widget.elevation;
            let expanded_header_padding = widget.expanded_header_padding.unwrap_or_else(
                SimpleExpansionPanelListTestWidget::default_expanded_header_padding,
            );
            let extended_state = app.get(self).extended_state;
            let mut list = ExpansionPanelList::new()
                .expanded_header_padding(expanded_header_padding)
                .expansion_callback(Rc::new(move |app: &mut App, index: usize, _is_expanded| {
                    self.set_state(app, |state| {
                        state.extended_state[index] = !state.extended_state[index];
                    });
                }))
                .elevation(elevation)
                .children([
                    ExpansionPanel::new(
                        text_header("A", "B", first_panel_key),
                        SizedBox::new().height(100.0),
                    )
                    .can_tap_on_header(can_tap_on_header)
                    .is_expanded(extended_state[0]),
                    ExpansionPanel::new(
                        text_header("C", "D", second_panel_key),
                        SizedBox::new().height(100.0),
                    )
                    .can_tap_on_header(can_tap_on_header)
                    .is_expanded(extended_state[1]),
                ]);
            list.divider_color = divider_color;
            list.into_widget()
        }
    }

    impl std::fmt::Debug for SimpleExpansionPanelListTestWidgetState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SimpleExpansionPanelListTestWidgetState")
                .field("extended_state", &self.extended_state)
                .finish_non_exhaustive()
        }
    }

    /// Dart's `(context, isExpanded) => Text(isExpanded ? expanded : collapsed, key: key)`.
    fn text_header(
        collapsed: &'static str,
        expanded: &'static str,
        key: Option<KeyRef>,
    ) -> ExpansionPanelHeaderBuilder {
        Rc::new(move |_app, _context, is_expanded| {
            let text = Text::new(if is_expanded { expanded } else { collapsed });
            match &key {
                Some(key) => text.key(key.clone()).into_widget(),
                None => text.into_widget(),
            }
        })
    }

    /// Dart's `(context, isExpanded) => widget`, the same whether expanded or not.
    fn fixed_header(widget: impl Fn() -> WidgetRef + 'static) -> ExpansionPanelHeaderBuilder {
        Rc::new(move |_app, _context, _is_expanded| widget())
    }

    /// Dart's `ExpansionPanel(headerBuilder: Text(isExpanded ? expanded : collapsed), body:
    /// SizedBox(height: 100.0))`.
    fn panel(collapsed: &'static str, expanded: &'static str) -> ExpansionPanel {
        ExpansionPanel::new(
            text_header(collapsed, expanded, None),
            SizedBox::new().height(100.0),
        )
    }

    /// Dart's `ExpansionPanelRadio(headerBuilder: Text(isExpanded ? expanded : collapsed),
    /// body: SizedBox(height: 100.0), value: value)`.
    fn radio_panel(
        value: i32,
        collapsed: &'static str,
        expanded: &'static str,
    ) -> ExpansionPanelRadio {
        ExpansionPanelRadio::new(
            value,
            text_header(collapsed, expanded, None),
            SizedBox::new().height(100.0),
        )
    }

    /// The three radio panels A/B, C/D and E/F of value 0, 1 and 2.
    fn demo_items_radio() -> Vec<ExpansionPanelRadio> {
        vec![
            radio_panel(0, "A", "B"),
            radio_panel(1, "C", "D"),
            radio_panel(2, "E", "F"),
        ]
    }

    fn key(name: &'static str) -> KeyRef {
        KeyRef::from(ValueKey::new(name))
    }

    /// Dart's `MaterialApp(home: SingleChildScrollView(child: child))`.
    fn scroll_app<K>(child: impl IntoWidget<K>) -> WidgetRef {
        MaterialApp::new()
            .home(SingleChildScrollView::new().child(child))
            .into_widget()
    }

    /// An expansion callback that records each call.
    fn recording_callback(history: &Rc<RefCell<Vec<(usize, bool)>>>) -> ExpansionPanelCallback {
        let history = Rc::clone(history);
        Rc::new(move |_app, index, is_expanded| history.borrow_mut().push((index, is_expanded)))
    }

    /// Dart's `tester.renderObject(find.byType(ExpansionPanelList))` as a `RenderBox`.
    fn expansion_panel_list_box(tester: &WidgetTester) -> AnyRenderBox {
        tester
            .render_object(find::by_type::<ExpansionPanelList>())
            .as_box()
            .expect("ExpansionPanelList renders a box")
    }

    /// Dart's `box.size.height`.
    fn height(tester: &WidgetTester, render_box: AnyRenderBox) -> f64 {
        tester.get_size(render_box).height()
    }

    /// Dart's `find.ancestor(of: find.byKey(key), matching: find.byType(AnimatedContainer)).first`.
    fn header_container(key: &KeyRef) -> Finder {
        find::ancestor(
            find::by_key(key.clone()),
            find::by_type::<AnimatedContainer>(),
        )
        .first()
    }

    /// Dart's `decoration as BoxDecoration`.
    fn box_decoration(decoration: &DecorationRef) -> BoxDecoration {
        decoration
            .as_any()
            .downcast_ref::<BoxDecoration>()
            .expect("the decoration is a BoxDecoration")
            .clone()
    }

    /// Dart's `tester.widget(find.byType(DecoratedBox).last)`'s `decoration.border!.top.color`.
    fn last_decorated_box_top_border_color(tester: &WidgetTester) -> Color {
        let decoration = tester.widget(
            find::by_type::<DecoratedBox>().last(),
            |decorated_box: &DecoratedBox| box_decoration(&decorated_box.decoration),
        );
        decoration
            .border
            .expect("the decoration has a border")
            .top()
            .color
    }

    /// Dart's `tester.widget<MergeableMaterial>(find.byType(MergeableMaterial)).children`.
    fn mergeable_material_children(tester: &WidgetTester) -> Vec<MergeableMaterialItem> {
        tester.widget(
            find::by_type::<MergeableMaterial>(),
            |material: &MergeableMaterial| material.children.clone(),
        )
    }

    /// Dart's `(item as MaterialSlice).color`.
    fn slice_color(item: &MergeableMaterialItem) -> Option<Color> {
        match item {
            MergeableMaterialItem::Slice(slice) => slice.color,
            MergeableMaterialItem::Gap(_) => panic!("the item is a MaterialSlice"),
        }
    }

    #[test]
    fn expansion_panel_list_test() {
        test_widgets(|tester| {
            let captured_index = Rc::new(Cell::new(None));
            let captured_is_expanded = Rc::new(Cell::new(None));
            let callback = {
                let captured_index = Rc::clone(&captured_index);
                let captured_is_expanded = Rc::clone(&captured_is_expanded);
                move || -> ExpansionPanelCallback {
                    let captured_index = Rc::clone(&captured_index);
                    let captured_is_expanded = Rc::clone(&captured_is_expanded);
                    Rc::new(move |_app, index, is_expanded| {
                        captured_index.set(Some(index));
                        captured_is_expanded.set(Some(is_expanded));
                    })
                }
            };

            tester.pump_widget(scroll_app(
                ExpansionPanelList::new()
                    .expansion_callback(callback())
                    .children([panel("A", "B")]),
            ));

            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            let mut render_box = expansion_panel_list_box(tester);
            let old_height = height(tester, render_box);
            expect(tester, &find::by_type::<ExpandIcon>(), finds_one_widget());
            tester.tap(find::by_type::<ExpandIcon>());
            assert_eq!(captured_index.get(), Some(0));
            assert_eq!(captured_is_expanded.get(), Some(true));
            render_box = expansion_panel_list_box(tester);
            assert_eq!(height(tester, render_box), old_height);

            // Now, expand the child panel.
            tester.pump_widget(scroll_app(
                ExpansionPanelList::new()
                    .expansion_callback(callback())
                    // this is the addition
                    .children([panel("A", "B").is_expanded(true)]),
            ));
            tester.pump_by(Duration::from_millis(200));

            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            render_box = expansion_panel_list_box(tester);
            // 100 + some margin
            assert!(height(tester, render_box) - old_height >= 100.0);
        });
    }

    #[test]
    fn multiple_panel_list_test() {
        test_widgets(|tester| {
            tester.pump_widget(MaterialApp::new().home(ListView::new().children(vec![
                    ExpansionPanelList::new()
                        .children([panel("A", "B").is_expanded(true)])
                        .into_widget(),
                    ExpansionPanelList::new()
                        .children([panel("C", "D").is_expanded(true)])
                        .into_widget(),
                ])));
            tester.pump_by(Duration::from_millis(200));

            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            expect(tester, &find::text("C"), finds_nothing());
            expect(tester, &find::text("D"), finds_one_widget());
        });
    }

    #[test]
    fn open_close_animations() {
        test_widgets(|tester| {
            const K_SIZE_ANIMATION_DURATION: Duration = Duration::from_millis(1000);
            // The MaterialGaps animate in using kThemeAnimationDuration (hardcoded), which
            // should be less than our test size animation length. So we can assume that they
            // appear immediately. Here we just verify that our assumption is true.
            assert!(K_THEME_ANIMATION_DURATION < K_SIZE_ANIMATION_DURATION / 2);

            let build = |a: bool, b: bool, c: bool| -> WidgetRef {
                let placeholder_header =
                    || fixed_header(|| Placeholder::new().fallback_height(12.0).into_widget());
                MaterialApp::new()
                    .home(Column::new().children(vec![
                        ExpansionPanelList::new()
                            .animation_duration(K_SIZE_ANIMATION_DURATION)
                            .children([
                                ExpansionPanel::new(
                                    placeholder_header(),
                                    SizedBox::new()
                                        .height(100.0)
                                        .child(Placeholder::new().fallback_height(12.0)),
                                )
                                .is_expanded(a),
                                ExpansionPanel::new(
                                    placeholder_header(),
                                    SizedBox::new().height(100.0).child(Placeholder::new()),
                                )
                                .is_expanded(b),
                                ExpansionPanel::new(
                                    placeholder_header(),
                                    SizedBox::new().height(100.0).child(Placeholder::new()),
                                )
                                .is_expanded(c),
                            ])
                            .into_widget(),
                    ]))
                    .into_widget()
            };
            let animated_size = |index: usize| find::by_type::<AnimatedSize>().at(index);

            tester.pump_widget(build(false, false, false));
            assert_eq!(find::by_type::<AnimatedSize>().evaluate(tester).len(), 3);
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(1)),
                Rect::from_ltwh(0.0, 97.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(2)),
                Rect::from_ltwh(0.0, 146.0, 800.0, 0.0)
            );

            tester.pump_by(Duration::from_millis(200));
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(1)),
                Rect::from_ltwh(0.0, 97.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(2)),
                Rect::from_ltwh(0.0, 146.0, 800.0, 0.0)
            );

            tester.pump_widget(build(false, true, false));
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(1)),
                Rect::from_ltwh(0.0, 97.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(2)),
                Rect::from_ltwh(0.0, 146.0, 800.0, 0.0)
            );

            tester.pump_by(K_SIZE_ANIMATION_DURATION / 2);
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            let rect1 = tester.get_rect(animated_size(1));
            assert_eq!(rect1.left, 0.0);
            // 16.0 material gap, plus 12.0 top and bottom margins added to the header
            assert!(
                rect1.top > 113.0 && rect1.top < 113.0 + 16.0 + 24.0,
                "{rect1:?}"
            );
            assert_eq!(rect1.width(), 800.0);
            assert!(rect1.height() > 0.0 && rect1.height() < 100.0, "{rect1:?}");
            let rect2 = tester.get_rect(animated_size(2));
            // the 16.0 comes from the MaterialGap being introduced, the 48.0 is the header height.
            // Dart's exact `expect(rect2, ...)`: positions go through valo's `f32` matrices.
            assert_f32_equals(
                rect2,
                Rect::from_ltwh(0.0, rect1.bottom + 16.0 + 48.0, 800.0, 0.0),
            );

            tester.pump_widget(build(false, false, false));
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            assert_eq!(tester.get_rect(animated_size(1)), rect1);
            assert_eq!(tester.get_rect(animated_size(2)), rect2);

            tester.pump_widget(build(false, false, true));
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            assert_eq!(tester.get_rect(animated_size(1)), rect1);
            assert_eq!(tester.get_rect(animated_size(2)), rect2);

            // a few no-op pumps to make sure there's nothing fishy going on
            tester.pump();
            tester.pump();
            tester.pump();
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            assert_eq!(tester.get_rect(animated_size(1)), rect1);
            assert_eq!(tester.get_rect(animated_size(2)), rect2);

            tester.pump_and_settle();
            assert_eq!(
                tester.get_rect(animated_size(0)),
                Rect::from_ltwh(0.0, 48.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(1)),
                Rect::from_ltwh(0.0, 48.0 + 1.0 + 48.0, 800.0, 0.0)
            );
            assert_eq!(
                tester.get_rect(animated_size(2)),
                Rect::from_ltwh(
                    0.0,
                    48.0 + 1.0 + 48.0 + 16.0 + 16.0 + 48.0 + 16.0,
                    800.0,
                    100.0
                )
            );
        });
    }

    #[test]
    fn radio_mode_has_max_of_one_panel_open_at_a_time() {
        test_widgets(|tester| {
            let expansion_list_radio = ExpansionPanelList::radio(demo_items_radio());

            tester.pump_widget(scroll_app(expansion_list_radio));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            let mut render_box = expansion_panel_list_box(tester);
            let mut old_height = height(tester, render_box);

            expect(tester, &find::by_type::<ExpandIcon>(), finds_n_widgets(3));

            tester.tap(find::by_type::<ExpandIcon>().at(0));

            render_box = expansion_panel_list_box(tester);
            assert_eq!(height(tester, render_box), old_height);

            tester.pump_by(Duration::from_millis(200));
            tester.pump_and_settle();

            // Now the first panel is open
            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            render_box = expansion_panel_list_box(tester);
            // 100 + some margin
            assert!(height(tester, render_box) - old_height >= 100.0);

            tester.tap(find::by_type::<ExpandIcon>().at(1));

            render_box = expansion_panel_list_box(tester);
            old_height = height(tester, render_box);

            tester.pump_by(Duration::from_millis(200));

            // Now the first panel is closed and the second should be opened
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_nothing());
            expect(tester, &find::text("D"), finds_one_widget());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            assert!(height(tester, render_box) >= old_height);

            // Dart's `demoItemsRadio.removeAt(0)` mutates the list the mounted widget holds,
            // which nothing rebuilds from; a Rust widget's children cannot be changed in place.
            tester.pump_and_settle();

            // Now the first panel should be opened
            expect(tester, &find::text("C"), finds_nothing());
            expect(tester, &find::text("D"), finds_one_widget());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            let demo_items = [panel("A", "B"), panel("C", "D"), panel("E", "F")];

            let expansion_list = ExpansionPanelList::new().children(demo_items);

            tester.pump_widget(scroll_app(expansion_list));

            // We've reinitialized with a regular expansion panel so they should all be closed
            // again
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());
        });
    }

    #[test]
    fn radio_mode_calls_expansion_callback_once_if_other_panels_closed() {
        test_widgets(|tester| {
            let callback_history = Rc::new(RefCell::new(Vec::new()));
            let expansion_list_radio = ExpansionPanelList::radio(demo_items_radio())
                .expansion_callback(recording_callback(&callback_history));

            tester.pump_widget(scroll_app(expansion_list_radio));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            // Open one panel
            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_and_settle();

            // Callback is invoked once with appropriate arguments
            assert_eq!(callback_history.borrow().len(), 1);
            assert_eq!(callback_history.borrow().last(), Some(&(1, true)));

            // Close the same panel
            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_and_settle();

            // Callback is invoked once with appropriate arguments
            assert_eq!(callback_history.borrow().len(), 2);
            assert_eq!(callback_history.borrow().last(), Some(&(1, false)));
        });
    }

    #[test]
    fn radio_mode_calls_expansion_callback_twice_if_other_panel_open_prior() {
        test_widgets(|tester| {
            let callback_history = Rc::new(RefCell::new(Vec::new()));
            let expansion_list_radio = ExpansionPanelList::radio(demo_items_radio())
                .expansion_callback(recording_callback(&callback_history));

            tester.pump_widget(scroll_app(expansion_list_radio));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            // Open one panel
            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_and_settle();

            // Callback is invoked once with appropriate arguments
            assert_eq!(callback_history.borrow().len(), 1);
            let mut callback_results =
                callback_history.borrow()[callback_history.borrow().len() - 1];
            assert_eq!(callback_results, (1, true));

            // Close a different panel
            tester.tap(find::by_type::<ExpandIcon>().at(2));
            tester.pump_and_settle();

            // Callback is invoked the first time with correct arguments
            assert_eq!(callback_history.borrow().len(), 3);
            callback_results = callback_history.borrow()[callback_history.borrow().len() - 2];
            assert_eq!(callback_results, (1, false));

            // Callback is invoked the second time with correct arguments
            callback_results = callback_history.borrow()[callback_history.borrow().len() - 1];
            assert_eq!(callback_results, (2, true));
        });
    }

    #[test]
    fn expansion_panel_list_radio_callback_displays_true_or_false_based_on_the_visibility_of_a_list_item()
     {
        test_widgets(|tester| {
            let last_expanded = Rc::new(Cell::new(None));
            let top_element_expanded = Rc::new(Cell::new(false));
            let bottom_element_expanded = Rc::new(Cell::new(false));

            let demo_items_radio = vec![
                // topElement
                radio_panel(0, "A", "B"),
                // bottomElement
                radio_panel(1, "C", "D"),
            ];

            let expansion_list_radio = ExpansionPanelList::radio(demo_items_radio)
                .expansion_callback({
                    let last_expanded = Rc::clone(&last_expanded);
                    let top_element_expanded = Rc::clone(&top_element_expanded);
                    let bottom_element_expanded = Rc::clone(&bottom_element_expanded);
                    Rc::new(move |_app, index, is_expanded| {
                        last_expanded.set(Some(index));
                        if index == 0 {
                            top_element_expanded.set(is_expanded);
                            bottom_element_expanded.set(false);
                        } else {
                            top_element_expanded.set(false);
                            bottom_element_expanded.set(is_expanded);
                        }
                    })
                });

            tester.pump_widget(scroll_app(expansion_list_radio));

            // Initializes with all panels closed.
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_type::<ExpandIcon>().at(0));
            tester.pump_by(Duration::from_millis(200));
            tester.pump_and_settle();

            // Now the first panel is open.
            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            assert_eq!(last_expanded.get(), Some(0));
            assert!(top_element_expanded.get());

            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_by(Duration::from_millis(200));
            tester.pump_and_settle();

            // Open the other panel and ensure the first is now closed.
            assert_eq!(last_expanded.get(), Some(1));
            assert!(bottom_element_expanded.get());
            assert!(!top_element_expanded.get());
            expect(tester, &find::text("D"), finds_one_widget());
            expect(tester, &find::text("A"), finds_one_widget());

            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_by(Duration::from_millis(200));
            tester.pump_and_settle();

            // Close the item that was expanded should now be false.
            assert_eq!(last_expanded.get(), Some(1));
            assert!(!bottom_element_expanded.get());

            // All panels should be closed.
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
        });
    }

    #[test]
    fn did_update_widget_accounts_for_toggling_between_expansion_panel_list_and_expansion_pane_list_radio()
     {
        test_widgets(|tester| {
            let is_radio_list = Rc::new(Cell::new(false));
            let panel_expansion_state = Rc::new(RefCell::new([false, false, false]));

            let build_radio_expansion_panel_list =
                || ExpansionPanelList::radio(demo_items_radio()).initial_open_panel_value(2);

            let build_expansion_panel_list = {
                let panel_expansion_state = Rc::clone(&panel_expansion_state);
                move |set_state: StateSetter| {
                    let expansion_state = *panel_expansion_state.borrow();
                    let toggled = Rc::clone(&panel_expansion_state);
                    ExpansionPanelList::new()
                        .expansion_callback(Rc::new(move |app: &mut App, index: usize, _| {
                            set_state(app, &mut |_app| {
                                let mut state = toggled.borrow_mut();
                                state[index] = !state[index];
                            });
                        }))
                        .children([
                            panel("A", "B").is_expanded(expansion_state[0]),
                            panel("C", "D").is_expanded(expansion_state[1]),
                            panel("E", "F").is_expanded(expansion_state[2]),
                        ])
                }
            };

            tester.pump_widget(StatefulBuilder::new({
                let is_radio_list = Rc::clone(&is_radio_list);
                move |_app, _context, set_state| {
                    let list = if is_radio_list.get() {
                        build_radio_expansion_panel_list()
                    } else {
                        build_expansion_panel_list(set_state.clone())
                    };
                    let is_radio_list = Rc::clone(&is_radio_list);
                    MaterialApp::new()
                        .home(
                            Scaffold::new()
                                .body(SingleChildScrollView::new().child(list))
                                .floating_action_button(FloatingActionButton::new(Some(
                                    Listener::new(move |app: &mut App| {
                                        set_state(app, &mut |_app| {
                                            is_radio_list.set(!is_radio_list.get());
                                        });
                                    }),
                                ))),
                        )
                        .into_widget()
                }
            }));

            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            tester.tap(find::by_type::<ExpandIcon>().at(0));
            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_and_settle();

            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            expect(tester, &find::text("C"), finds_nothing());
            expect(tester, &find::text("D"), finds_one_widget());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            // ExpansionPanelList --> ExpansionPanelList.radio
            tester.tap(find::by_type::<FloatingActionButton>());
            tester.pump_and_settle();

            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_nothing());
            expect(tester, &find::text("F"), finds_one_widget());

            // ExpansionPanelList.radio --> ExpansionPanelList
            tester.tap(find::by_type::<FloatingActionButton>());
            tester.pump_and_settle();

            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            expect(tester, &find::text("C"), finds_nothing());
            expect(tester, &find::text("D"), finds_one_widget());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());
        });
    }

    #[test]
    fn no_duplicate_global_keys_at_layout_build_time() {
        // Regression test for https://github.com/flutter/flutter/issues/13780
        test_widgets(|tester| {
            tester.pump_widget(StatefulBuilder::new(|_app, _context, set_state| {
                MaterialApp::new()
                    // Wrapping with LayoutBuilder or other widgets that augment layout/build
                    // order should not create duplicate keys
                    .home(
                        LayoutBuilder::new(move |_app, _context, _constraints| {
                            let set_state = set_state.clone();
                            SingleChildScrollView::new()
                                .child(
                                    ExpansionPanelList::radio(demo_items_radio())
                                        .expansion_callback(Rc::new(
                                            move |app: &mut App, _index, is_expanded| {
                                                if !is_expanded {
                                                    // setState invocation required to trigger
                                                    // _ExpansionPanelListState.didUpdateWidget, which
                                                    // causes duplicate keys to be generated in the
                                                    // regression
                                                    set_state(app, &mut |_app| {});
                                                }
                                            },
                                        )),
                                )
                                .into_widget()
                        })
                        .into_widget(),
                    )
                    .into_widget()
            }));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
            expect(tester, &find::text("E"), finds_one_widget());
            expect(tester, &find::text("F"), finds_nothing());

            // Open a panel
            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_and_settle();

            let panel_expansion_state = Rc::new(RefCell::new([false, false]));

            tester.pump_widget(StatefulBuilder::new(move |_app, _context, set_state| {
                let panel_expansion_state = Rc::clone(&panel_expansion_state);
                MaterialApp::new()
                    .home(
                        Scaffold::new().body(
                            // Wrapping with LayoutBuilder or other widgets that augment
                            // layout/build order should not create duplicate keys
                            LayoutBuilder::new(move |_app, _context, _constraints| {
                                let set_state = set_state.clone();
                                let expansion_state = *panel_expansion_state.borrow();
                                let toggled = Rc::clone(&panel_expansion_state);
                                SingleChildScrollView::new()
                                .child(
                                    ExpansionPanelList::new()
                                        .expansion_callback(Rc::new(
                                            move |app: &mut App, index: usize, is_expanded: bool| {
                                                // setState invocation required to trigger
                                                // _ExpansionPanelListState.didUpdateWidget,
                                                // which causes duplicate keys to be generated
                                                // in the regression
                                                set_state(app, &mut |_app| {
                                                    toggled.borrow_mut()[index] = !is_expanded;
                                                });
                                            },
                                        ))
                                        .children([
                                            panel("A", "B").is_expanded(expansion_state[0]),
                                            panel("C", "D").is_expanded(expansion_state[1]),
                                        ]),
                                )
                                .into_widget()
                            })
                            .into_widget(),
                        ),
                    )
                    .into_widget()
            }));

            // initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            // open a panel
            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_and_settle();
        });
    }

    #[test]
    fn ensure_can_tap_on_header_is_false_by_default() {
        let expansion_panel = ExpansionPanel::new(
            fixed_header(|| Text::new("Demo").into_widget()),
            SizedBox::new().height(100.0),
        );

        assert!(!expansion_panel.can_tap_on_header);
    }

    #[test]
    fn toggle_expansion_panel_radio_when_tapping_header_and_can_tap_on_header_is_true() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");
            let second_panel_key = key("secondPanelKey");

            let demo_items_radio = [
                ExpansionPanelRadio::new(
                    0,
                    text_header("A", "B", Some(first_panel_key.clone())),
                    SizedBox::new().height(100.0),
                )
                .can_tap_on_header(true),
                ExpansionPanelRadio::new(
                    1,
                    text_header("C", "D", Some(second_panel_key.clone())),
                    SizedBox::new().height(100.0),
                )
                .can_tap_on_header(true),
            ];

            let expansion_list_radio = ExpansionPanelList::radio(demo_items_radio);

            tester.pump_widget(scroll_app(expansion_list_radio));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(first_panel_key));
            tester.pump_and_settle();

            // Now the first panel is open
            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(second_panel_key));
            tester.pump_and_settle();

            // Now the second panel is open
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_nothing());
            expect(tester, &find::text("D"), finds_one_widget());
        });
    }

    #[test]
    fn toggle_expansion_panel_when_tapping_header_and_can_tap_on_header_is_true() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");
            let second_panel_key = key("secondPanelKey");

            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new()
                    .first_panel_key(first_panel_key.clone())
                    .second_panel_key(second_panel_key.clone())
                    .can_tap_on_header(true),
            ));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(first_panel_key.clone()));
            tester.pump_and_settle();

            // The first panel is open
            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(first_panel_key));
            tester.pump_and_settle();

            // The first panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(second_panel_key.clone()));
            tester.pump_and_settle();

            // The second panel is open
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_nothing());
            expect(tester, &find::text("D"), finds_one_widget());

            tester.tap(find::by_key(second_panel_key));
            tester.pump_and_settle();

            // The second panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
        });
    }

    #[test]
    fn do_not_toggle_expansion_panel_when_tapping_header_and_can_tap_on_header_is_false() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");
            let second_panel_key = key("secondPanelKey");

            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new()
                    .first_panel_key(first_panel_key.clone())
                    .second_panel_key(second_panel_key.clone()),
            ));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(first_panel_key));
            tester.pump_and_settle();

            // The first panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(second_panel_key));
            tester.pump_and_settle();

            // The second panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
        });
    }

    #[test]
    fn do_not_toggle_expansion_panel_radio_when_tapping_header_and_can_tap_on_header_is_false() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");
            let second_panel_key = key("secondPanelKey");

            let demo_items_radio = [
                ExpansionPanelRadio::new(
                    0,
                    text_header("A", "B", Some(first_panel_key.clone())),
                    SizedBox::new().height(100.0),
                ),
                ExpansionPanelRadio::new(
                    1,
                    text_header("C", "D", Some(second_panel_key.clone())),
                    SizedBox::new().height(100.0),
                ),
            ];

            let expansion_list_radio = ExpansionPanelList::radio(demo_items_radio);

            tester.pump_widget(scroll_app(expansion_list_radio));

            // Initializes with all panels closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(first_panel_key));
            tester.pump_and_settle();

            // The first panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());

            tester.tap(find::by_key(second_panel_key));
            tester.pump_and_settle();

            // The second panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());
            expect(tester, &find::text("C"), finds_one_widget());
            expect(tester, &find::text("D"), finds_nothing());
        });
    }

    #[test]
    fn correct_default_header_padding() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");

            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new()
                    .first_panel_key(first_panel_key.clone())
                    .can_tap_on_header(true),
            ));

            // The panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());

            // No padding applied to closed header
            let mut size = tester.get_size(header_container(&first_panel_key));
            assert_eq!(size.height(), 48.0); // _kPanelHeaderCollapsedHeight
            assert_eq!(size.width(), 744.0);

            // Now, expand the child panel.
            tester.tap(find::by_key(first_panel_key.clone()));
            tester.pump_and_settle();

            // The panel is expanded
            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());

            // Padding is added to expanded header
            size = tester.get_size(header_container(&first_panel_key));
            // _kPanelHeaderCollapsedHeight + 24.0 (double default padding)
            assert_eq!(size.height(), 80.0);
            assert_eq!(size.width(), 744.0);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/5848.
    #[test]
    fn the_animated_container_and_icon_button_have_the_same_height_of_48px() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");

            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new()
                    .first_panel_key(first_panel_key.clone())
                    .can_tap_on_header(true),
            ));

            // The panel is closed.
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());

            // No padding applied to closed header.
            let box_of_container = tester.get_size(header_container(&first_panel_key));
            let box_of_icon_button = tester.get_size(find::by_type::<IconButton>().first());
            assert_eq!(box_of_container.height(), box_of_icon_button.height());
            // Header should have 48px height according to Material 2 Design spec.
            assert_eq!(box_of_container.height(), 48.0);
        });
    }

    #[test]
    fn the_animated_container_s_height_is_at_least_k_min_interactive_dimension() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");

            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new()
                    .first_panel_key(first_panel_key.clone())
                    .can_tap_on_header(true),
            ));

            // The panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());

            // No padding applied to closed header
            let size = tester.get_size(header_container(&first_panel_key));
            assert!(size.height() >= K_MIN_INTERACTIVE_DIMENSION);
        });
    }

    #[test]
    fn correct_custom_header_padding() {
        test_widgets(|tester| {
            let first_panel_key = key("firstPanelKey");

            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new()
                    .first_panel_key(first_panel_key.clone())
                    .can_tap_on_header(true)
                    .expanded_header_padding(EdgeInsets::symmetric(40.0, 0.0)),
            ));

            // The panel is closed
            expect(tester, &find::text("A"), finds_one_widget());
            expect(tester, &find::text("B"), finds_nothing());

            // No padding applied to closed header
            let mut size = tester.get_size(header_container(&first_panel_key));
            assert_eq!(size.height(), 48.0); // _kPanelHeaderCollapsedHeight
            assert_eq!(size.width(), 744.0);

            // Now, expand the child panel.
            tester.tap(find::by_key(first_panel_key.clone()));
            tester.pump_and_settle();

            // The panel is expanded
            expect(tester, &find::text("A"), finds_nothing());
            expect(tester, &find::text("B"), finds_one_widget());

            // Padding is added to expanded header
            size = tester.get_size(header_container(&first_panel_key));
            // _kPanelHeaderCollapsedHeight + 80.0 (double padding)
            assert_eq!(size.height(), 128.0);
            assert_eq!(size.width(), 744.0);
        });
    }

    #[test]
    fn expansion_panel_list_respects_divider_color() {
        test_widgets(|tester| {
            let divider_color = Colors::RED.primary();
            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new().divider_color(divider_color),
            ));

            // For the last DecoratedBox, we will have a Border.top with the provided
            // dividerColor.
            assert_eq!(last_decorated_box_top_border_color(tester), divider_color);
        });
    }

    #[test]
    fn expansion_panel_list_radio_respects_divider_color() {
        test_widgets(|tester| {
            let divider_color = Colors::RED.primary();
            tester.pump_widget(scroll_app(
                ExpansionPanelList::radio([
                    ExpansionPanelRadio::new(
                        0,
                        text_header("A", "B", Some(key("firstKey"))),
                        SizedBox::new().height(100.0),
                    ),
                    ExpansionPanelRadio::new(
                        1,
                        text_header("C", "D", Some(key("secondKey"))),
                        SizedBox::new().height(100.0),
                    ),
                ])
                .divider_color(divider_color),
            ));

            // For the last DecoratedBox, we will have a Border.top with the provided
            // dividerColor.
            assert_eq!(last_decorated_box_top_border_color(tester), divider_color);
        });
    }

    #[test]
    fn expansion_panel_list_respects_expand_icon_color() {
        test_widgets(|tester| {
            let expand_icon_color = Colors::BLUE.primary();
            tester.pump_widget(scroll_app(
                ExpansionPanelList::new()
                    .expand_icon_color(expand_icon_color)
                    .children([ExpansionPanel::new(
                        fixed_header(|| SizedBox::shrink().into_widget()),
                        SizedBox::shrink(),
                    )
                    .can_tap_on_header(true)]),
            ));

            let color = tester.widget(find::by_type::<ExpandIcon>(), |expand_icon: &ExpandIcon| {
                expand_icon.color
            });

            assert_eq!(color, Some(expand_icon_color));
        });
    }

    #[test]
    fn expansion_panel_list_radio_respects_expand_icon_color() {
        test_widgets(|tester| {
            let expand_icon_color = Colors::BLUE.primary();
            tester.pump_widget(scroll_app(
                ExpansionPanelList::radio([ExpansionPanelRadio::new(
                    true,
                    fixed_header(|| SizedBox::shrink().into_widget()),
                    SizedBox::shrink(),
                )
                .can_tap_on_header(true)])
                .expand_icon_color(expand_icon_color),
            ));

            let color = tester.widget(find::by_type::<ExpandIcon>(), |expand_icon: &ExpandIcon| {
                expand_icon.color
            });

            assert_eq!(color, Some(expand_icon_color));
        });
    }

    #[test]
    fn elevation_is_propagated_properly_to_mergeable_material() {
        test_widgets(|tester| {
            const ELEVATION: f64 = 8.0;
            let mergeable_material_elevation = |tester: &WidgetTester| {
                tester.widget(
                    find::by_type::<MergeableMaterial>(),
                    |material: &MergeableMaterial| material.elevation,
                )
            };

            // Test for ExpansionPanelList.
            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new().elevation(ELEVATION),
            ));

            assert_eq!(mergeable_material_elevation(tester), ELEVATION);

            // Test for ExpansionPanelList.radio.
            tester.pump_widget(scroll_app(
                ExpansionPanelList::radio([
                    ExpansionPanelRadio::new(
                        0,
                        text_header("A", "B", Some(key("firstKey"))),
                        SizedBox::new().height(100.0),
                    ),
                    ExpansionPanelRadio::new(
                        1,
                        text_header("C", "D", Some(key("secondKey"))),
                        SizedBox::new().height(100.0),
                    ),
                ])
                .elevation(ELEVATION),
            ));

            assert_eq!(mergeable_material_elevation(tester), ELEVATION);
        });
    }

    #[test]
    #[should_panic(
        expected = "Invalid value for elevation. See the kElevationToShadow constant for possible \
                    elevation values."
    )]
    fn using_a_value_non_defined_value_throws_assertion_error() {
        test_widgets(|tester| {
            // It should throw an AssertionError since, 19 is not defined in kElevationToShadow.
            tester.pump_widget(scroll_app(
                SimpleExpansionPanelListTestWidget::new().elevation(19.0),
            ));
        });
    }

    #[test]
    fn expansion_panel_panel_color_test() {
        test_widgets(|tester| {
            let first_panel_color = Colors::RED.primary();
            let second_panel_color = Colors::BROWN.primary();

            tester.pump_widget(scroll_app(
                ExpansionPanelList::new()
                    .expansion_callback(Rc::new(|_app, _index, _is_expanded| {}))
                    .children([
                        ExpansionPanel::new(
                            fixed_header(|| Text::new("A").into_widget()),
                            SizedBox::new().height(100.0),
                        )
                        .background_color(first_panel_color),
                        ExpansionPanel::new(
                            fixed_header(|| Text::new("B").into_widget()),
                            SizedBox::new().height(100.0),
                        )
                        .background_color(second_panel_color),
                    ]),
            ));

            let children = mergeable_material_children(tester);

            assert_eq!(
                slice_color(children.first().unwrap()),
                Some(first_panel_color)
            );
            assert_eq!(
                slice_color(children.last().unwrap()),
                Some(second_panel_color)
            );
        });
    }

    #[test]
    fn expansion_panel_radio_background_color_test() {
        test_widgets(|tester| {
            let first_panel_color = Colors::RED.primary();
            let second_panel_color = Colors::BROWN.primary();

            tester.pump_widget(scroll_app(ExpansionPanelList::radio([
                ExpansionPanelRadio::new(
                    0,
                    fixed_header(|| Text::new("A").into_widget()),
                    SizedBox::new().height(100.0),
                )
                .background_color(first_panel_color),
                ExpansionPanelRadio::new(
                    1,
                    fixed_header(|| Text::new("B").into_widget()),
                    SizedBox::new().height(100.0),
                )
                .background_color(second_panel_color),
            ])));

            let children = mergeable_material_children(tester);

            assert_eq!(
                slice_color(children.first().unwrap()),
                Some(first_panel_color)
            );
            assert_eq!(
                slice_color(children.last().unwrap()),
                Some(second_panel_color)
            );
        });
    }

    #[test]
    fn expansion_panel_list_material_gap_size_defaults_to_16_0() {
        test_widgets(|tester| {
            tester.pump_widget(scroll_app(
                ExpansionPanelList::new().children([ExpansionPanel::new(
                    fixed_header(|| SizedBox::shrink().into_widget()),
                    SizedBox::shrink(),
                )
                .can_tap_on_header(true)]),
            ));

            let material_gap_size = tester.widget(
                find::by_type::<ExpansionPanelList>(),
                |expansion_panel_list: &ExpansionPanelList| expansion_panel_list.material_gap_size,
            );
            assert_eq!(material_gap_size, 16.0);
        });
    }

    #[test]
    fn expansion_panel_list_respects_material_gap_size() {
        test_widgets(|tester| {
            let build_widget_for_test = |material_gap_size: f64| {
                scroll_app(
                    ExpansionPanelList::new()
                        .material_gap_size(material_gap_size)
                        .children([
                            ExpansionPanel::new(
                                fixed_header(|| SizedBox::shrink().into_widget()),
                                SizedBox::shrink(),
                            )
                            .is_expanded(true)
                            .can_tap_on_header(true),
                            ExpansionPanel::new(
                                fixed_header(|| SizedBox::shrink().into_widget()),
                                SizedBox::shrink(),
                            )
                            .can_tap_on_header(true),
                        ]),
                )
            };
            let expect_one_gap_of = |tester: &WidgetTester, size: f64| {
                let children = mergeable_material_children(tester);
                assert_eq!(children.len(), 3);
                let gaps: Vec<f64> = children
                    .iter()
                    .filter_map(|item| match item {
                        MergeableMaterialItem::Gap(gap) => Some(gap.size),
                        MergeableMaterialItem::Slice(_) => None,
                    })
                    .collect();
                assert_eq!(gaps.len(), 1);
                assert_eq!(children.len() - gaps.len(), 2);
                for gap_size in gaps {
                    assert_eq!(gap_size, size);
                }
            };

            tester.pump_widget(build_widget_for_test(0.0));
            tester.pump_and_settle();
            expect_one_gap_of(tester, 0.0);

            tester.pump_widget(build_widget_for_test(20.0));
            tester.pump_and_settle();
            expect_one_gap_of(tester, 20.0);

            // Dart's default `materialGapSize = 16`.
            tester.pump_widget(build_widget_for_test(16.0));
            tester.pump_and_settle();
            expect_one_gap_of(tester, 16.0);
        });
    }

    #[test]
    fn ensure_icon_button_splash_color_and_highlight_color_are_correctly_set_when_can_tap_on_header_is_false()
     {
        test_widgets(|tester| {
            let expected_splash_color = Colors::GREEN.primary();
            let expected_highlight_color = Colors::YELLOW.primary();

            tester.pump_widget(scroll_app(
                ExpansionPanelList::new().children([ExpansionPanel::new(
                    fixed_header(|| ListTile::new().title(Text::new("Panel 1")).into_widget()),
                    ListTile::new().title(Text::new("Content for Panel 1")),
                )
                .splash_color(expected_splash_color)
                .highlight_color(expected_highlight_color)]),
            ));

            tester.tap(find::text("Panel 1"));
            tester.pump_and_settle();

            let (splash_color, highlight_color) = tester.widget(
                find::by_type::<IconButton>().first(),
                |icon_button: &IconButton| (icon_button.splash_color, icon_button.highlight_color),
            );
            assert_eq!(splash_color, Some(expected_splash_color));
            assert_eq!(highlight_color, Some(expected_highlight_color));
        });
    }

    #[test]
    fn ensure_ink_well_splash_color_and_highlight_color_are_correctly_set_when_can_tap_on_header_is_true()
     {
        test_widgets(|tester| {
            let expected_splash_color = Colors::GREEN.primary();
            let expected_highlight_color = Colors::YELLOW.primary();

            tester.pump_widget(scroll_app(
                ExpansionPanelList::new().children([ExpansionPanel::new(
                    fixed_header(|| {
                        Container::new()
                            .padding(EdgeInsets::all(16.0).into())
                            .alignment(AlignmentGeometry::CENTER_LEFT)
                            .child(Text::new("Panel 1"))
                            .into_widget()
                    }),
                    ListTile::new().title(Text::new("Content for Panel 1")),
                )
                .can_tap_on_header(true)
                .splash_color(expected_splash_color)
                .highlight_color(expected_highlight_color)]),
            ));

            tester.pump_and_settle();

            let ink_well_finder = find::descendant(
                find::by_type::<ExpansionPanelList>(),
                find::by_widget_predicate("an InkWell with an onTap", |_app, widget| {
                    downcast_widget::<InkWell>(widget)
                        .is_some_and(|ink_well| ink_well.ink_response.on_tap.is_some())
                }),
            );

            let (splash_color, highlight_color) =
                tester.widget(ink_well_finder.first(), |ink_well: &InkWell| {
                    (
                        ink_well.ink_response.splash_color,
                        ink_well.ink_response.highlight_color,
                    )
                });
            assert_eq!(splash_color, Some(expected_splash_color));
            assert_eq!(highlight_color, Some(expected_highlight_color));
        });
    }

    #[test]
    fn expand_icon_ignores_pointer_tap_events_when_can_tap_on_header_is_true() {
        test_widgets(|tester| {
            let build_widget = |can_tap_on_header: bool| {
                scroll_app(
                    ExpansionPanelList::new().children([ExpansionPanel::new(
                        fixed_header(|| ListTile::new().title(Text::new("Panel")).into_widget()),
                        ListTile::new().title(Text::new("Content for Panel")),
                    )
                    .can_tap_on_header(can_tap_on_header)]),
                )
            };

            tester.pump_widget(build_widget(false));

            let ignore_pointer_finder = find::descendant(
                find::by_type::<ExpansionPanelList>(),
                find::by_type::<IgnorePointer>(),
            )
            .first();

            let ignoring = |tester: &WidgetTester, finder: &Finder| {
                tester.widget(finder, |ignore_pointer: &IgnorePointer| {
                    ignore_pointer.ignoring
                })
            };
            assert!(!ignoring(tester, &ignore_pointer_finder));

            tester.pump_widget(build_widget(true));
            tester.pump_and_settle();

            assert!(ignoring(tester, &ignore_pointer_finder));
        });
    }
}
