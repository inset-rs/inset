//! Flutter counterpart: material_ui's `expansion_panel.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Curves, Interval};
use waft_embedder::Color;
use waft_foundation::{App, Handle, Listener, ValueKey};
use waft_painting::{EdgeInsets, EdgeInsetsDirectional};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    AnimatedContainer, AnimatedCrossFade, BuildContext, Column, ConstrainedBox, CrossFadeState,
    Expanded, IgnorePointer, IntoWidget, KeyRef, LimitedBox, MergeSemantics, Padding, Row,
    Semantics, SizedBox, State, StateData, StatefulWidget, WidgetRef,
};

use crate::ink_well::InkWell;

use crate::constants::{K_MIN_INTERACTIVE_DIMENSION, K_THEME_CHANGE_DURATION};
use crate::expand_icon::ExpandIcon;
use crate::material_localizations::MaterialLocalizations;
use crate::mergeable_material::{
    MaterialGap, MaterialSlice, MergeableMaterial, MergeableMaterialItem,
};

/// The collapsed height of a panel header.
const K_PANEL_HEADER_COLLAPSED_HEIGHT: f64 = K_MIN_INTERACTIVE_DIMENSION;
/// The padding around a panel header when expanded.
const K_PANEL_HEADER_EXPANDED_PADDING: EdgeInsets =
    EdgeInsets::symmetric(64.0 - K_PANEL_HEADER_COLLAPSED_HEIGHT, 0.0);
/// The padding around the expand icon.
const K_EXPAND_ICON_PADDING: EdgeInsets = EdgeInsets::all(12.0);

/// Signature for the callback that's called when an [`ExpansionPanel`] is expanded or
/// collapsed.
pub type ExpansionPanelCallback = Rc<dyn Fn(&mut App, usize, bool)>;

/// Signature for the callback that's called when the header of an [`ExpansionPanel`] needs to
/// rebuild.
pub type ExpansionPanelHeaderBuilder = Rc<dyn Fn(&mut App, BuildContext, bool) -> WidgetRef>;

/// A material expansion panel. It has a header and a body and can be either expanded or
/// collapsed. The body of the panel is only visible when it is expanded.
///
/// Expansion panels are only intended to be used as children for [`ExpansionPanelList`].
#[derive(Clone)]
pub struct ExpansionPanel {
    /// The widget builder that builds the expansion panel's header.
    pub header_builder: ExpansionPanelHeaderBuilder,
    /// The body of the expansion panel that's displayed below the header.
    pub body: WidgetRef,
    /// Whether the panel is expanded.
    pub is_expanded: bool,
    /// Whether tapping on the panel's header will expand/collapse it.
    pub can_tap_on_header: bool,
    /// Defines the background color of the panel.
    pub background_color: Option<Color>,
    /// Defines the splash color of the panel or its expand/collapse icon.
    pub splash_color: Option<Color>,
    /// Defines the highlight color of the panel or its expand/collapse icon.
    pub highlight_color: Option<Color>,
}

impl ExpansionPanel {
    /// Creates an expansion panel to be used as a child for [`ExpansionPanelList`]; Dart's
    /// optional named arguments are the setters.
    pub fn new<K>(header_builder: ExpansionPanelHeaderBuilder, body: impl IntoWidget<K>) -> Self {
        ExpansionPanel {
            header_builder,
            body: body.into_widget(),
            is_expanded: false,
            can_tap_on_header: false,
            background_color: None,
            splash_color: None,
            highlight_color: None,
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

/// An expansion panel that allows for radio-like functionality: at most one panel can be open
/// at a time.
pub struct ExpansionPanelRadio<T: Clone + PartialEq + 'static> {
    /// The panel.
    pub panel: ExpansionPanel,
    /// The value that uniquely identifies this radio panel.
    pub value: T,
}

impl<T: Clone + PartialEq + 'static> ExpansionPanelRadio<T> {
    /// Creates an expansion panel that allows for radio functionality.
    pub fn new<K>(
        value: T,
        header_builder: ExpansionPanelHeaderBuilder,
        body: impl IntoWidget<K>,
    ) -> Self {
        ExpansionPanelRadio {
            panel: ExpansionPanel::new(header_builder, body),
            value,
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

impl<T: Clone + PartialEq + 'static> fmt::Debug for ExpansionPanelRadio<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionPanelRadio")
            .field("panel", &self.panel)
            .finish_non_exhaustive()
    }
}

/// A material expansion panel list that lays out its children and animates expansions.
///
/// The `expansion_callback` is called when the expansion state changes. For normal
/// [`ExpansionPanelList`] widgets, it is the responsibility of the parent widget to rebuild the
/// list with updated values for [`ExpansionPanel::is_expanded`]. For
/// [`ExpansionPanelList::radio`] widgets, the open state is tracked internally.
pub struct ExpansionPanelList<T: Clone + PartialEq + 'static = ()> {
    pub key: Option<KeyRef>,
    /// The children of the expansion panel list.
    pub children: Vec<ExpansionPanel>,
    /// The callback that gets called whenever one of the expand/collapse buttons is pressed.
    pub expansion_callback: Option<ExpansionPanelCallback>,
    /// The duration of the expansion animation.
    pub animation_duration: Duration,
    /// The padding that surrounds the panel header when expanded.
    pub expanded_header_padding: EdgeInsets,
    /// Defines the color for the divider when a panel is collapsed.
    pub divider_color: Option<Color>,
    /// Defines the elevation of the panel while it is expanded.
    pub elevation: f64,
    /// The color of the expand icon.
    pub expand_icon_color: Option<Color>,
    /// The size of the gap between panels.
    pub material_gap_size: f64,
    allow_only_one_panel_open: bool,
    radio_values: Vec<T>,
    initial_open_panel_value: Option<T>,
}

impl ExpansionPanelList<()> {
    /// Creates an expansion panel list widget; Dart's optional named arguments are the
    /// setters.
    pub fn new() -> ExpansionPanelList<()> {
        ExpansionPanelList {
            key: None,
            children: Vec::new(),
            expansion_callback: None,
            animation_duration: K_THEME_CHANGE_DURATION,
            expanded_header_padding: K_PANEL_HEADER_EXPANDED_PADDING,
            divider_color: None,
            elevation: 2.0,
            expand_icon_color: None,
            material_gap_size: 16.0,
            allow_only_one_panel_open: false,
            radio_values: Vec::new(),
            initial_open_panel_value: None,
        }
    }

    /// Dart `ExpansionPanelList(children:)`.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = ExpansionPanel>,
    ) -> ExpansionPanelList<()> {
        self.children = children.into_iter().collect();
        self
    }
}

impl<T: Clone + PartialEq + 'static> ExpansionPanelList<T> {
    /// Creates a radio expansion panel list widget: at most one panel is open.
    pub fn radio(
        children: impl IntoIterator<Item = ExpansionPanelRadio<T>>,
    ) -> ExpansionPanelList<T> {
        let mut panels = Vec::new();
        let mut values = Vec::new();
        for child in children {
            panels.push(child.panel);
            values.push(child.value);
        }
        ExpansionPanelList {
            key: None,
            children: panels,
            expansion_callback: None,
            animation_duration: K_THEME_CHANGE_DURATION,
            expanded_header_padding: K_PANEL_HEADER_EXPANDED_PADDING,
            divider_color: None,
            elevation: 2.0,
            expand_icon_color: None,
            material_gap_size: 16.0,
            allow_only_one_panel_open: true,
            radio_values: values,
            initial_open_panel_value: None,
        }
    }

    /// Dart `ExpansionPanelList(key:)`.
    pub fn key(mut self, key: KeyRef) -> ExpansionPanelList<T> {
        self.key = Some(key);
        self
    }

    /// Dart `ExpansionPanelList(children:)`, for a radio list.
    pub fn children_radio(
        mut self,
        children: impl IntoIterator<Item = ExpansionPanelRadio<T>>,
    ) -> ExpansionPanelList<T> {
        let mut panels = Vec::new();
        let mut values = Vec::new();
        for child in children {
            panels.push(child.panel);
            values.push(child.value);
        }
        self.children = panels;
        self.radio_values = values;
        self
    }

    /// Dart `ExpansionPanelList(expansionCallback:)`.
    pub fn expansion_callback(
        mut self,
        expansion_callback: ExpansionPanelCallback,
    ) -> ExpansionPanelList<T> {
        self.expansion_callback = Some(expansion_callback);
        self
    }

    /// Dart `ExpansionPanelList(animationDuration:)`.
    pub fn animation_duration(mut self, animation_duration: Duration) -> ExpansionPanelList<T> {
        self.animation_duration = animation_duration;
        self
    }

    /// Dart `ExpansionPanelList(expandedHeaderPadding:)`.
    pub fn expanded_header_padding(mut self, padding: EdgeInsets) -> ExpansionPanelList<T> {
        self.expanded_header_padding = padding;
        self
    }

    /// Dart `ExpansionPanelList(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> ExpansionPanelList<T> {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `ExpansionPanelList(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> ExpansionPanelList<T> {
        self.elevation = elevation;
        self
    }

    /// Dart `ExpansionPanelList(expandIconColor:)`.
    pub fn expand_icon_color(mut self, expand_icon_color: Color) -> ExpansionPanelList<T> {
        self.expand_icon_color = Some(expand_icon_color);
        self
    }

    /// Dart `ExpansionPanelList(materialGapSize:)`.
    pub fn material_gap_size(mut self, material_gap_size: f64) -> ExpansionPanelList<T> {
        self.material_gap_size = material_gap_size;
        self
    }

    /// Dart `ExpansionPanelList.radio(initialOpenPanelValue:)`.
    pub fn initial_open_panel_value(mut self, value: T) -> ExpansionPanelList<T> {
        self.initial_open_panel_value = Some(value);
        self
    }
}

impl<T: Clone + PartialEq + 'static> Default for ExpansionPanelList<T> {
    fn default() -> Self {
        ExpansionPanelList {
            key: None,
            children: Vec::new(),
            expansion_callback: None,
            animation_duration: K_THEME_CHANGE_DURATION,
            expanded_header_padding: K_PANEL_HEADER_EXPANDED_PADDING,
            divider_color: None,
            elevation: 2.0,
            expand_icon_color: None,
            material_gap_size: 16.0,
            allow_only_one_panel_open: false,
            radio_values: Vec::new(),
            initial_open_panel_value: None,
        }
    }
}

impl<T: Clone + PartialEq + 'static> fmt::Debug for ExpansionPanelList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionPanelList")
            .field("children", &self.children.len())
            .field("elevation", &self.elevation)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for ExpansionPanelList<T> {
    type State = ExpansionPanelListState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ExpansionPanelListState<T> {
        ExpansionPanelListState {
            state: StateData::new(),
            current_open_panel: None,
        }
    }
}

/// Dart's `_ExpansionPanelListState`.
pub struct ExpansionPanelListState<T: Clone + PartialEq + 'static> {
    state: StateData<ExpansionPanelList<T>>,
    current_open_panel: Option<T>,
}

impl<T: Clone + PartialEq + 'static> ExpansionPanelListState<T> {
    fn is_child_expanded(self: Handle<Self>, app: &App, index: usize) -> bool {
        let list = self.widget(app);
        if list.allow_only_one_panel_open {
            app.get(self).current_open_panel.as_ref() == Some(&list.radio_values[index])
        } else {
            list.children[index].is_expanded
        }
    }

    fn handle_pressed(self: Handle<Self>, app: &mut App, is_expanded: bool, index: usize) {
        let allow_one = self.widget(app).allow_only_one_panel_open;
        if allow_one {
            let current_open = app.get(self).current_open_panel.clone();
            let pressed_value = self.widget(app).radio_values[index].clone();
            let mut close_index = None;
            for (child_index, value) in self.widget(app).radio_values.iter().enumerate() {
                if child_index != index && Some(value) == current_open.as_ref() {
                    close_index = Some(child_index);
                }
            }
            if let Some(close_index) = close_index
                && let Some(callback) = self.widget(app).expansion_callback.clone()
            {
                callback(app, close_index, false);
            }
            let value = if is_expanded {
                None
            } else {
                Some(pressed_value)
            };
            eprintln!("radio set open: {}", value.is_some());
            self.set_state(app, |state| state.current_open_panel = value);
        }
        if let Some(callback) = self.widget(app).expansion_callback.clone() {
            callback(app, index, !is_expanded);
        }
    }
}

impl<T: Clone + PartialEq + 'static> fmt::Debug for ExpansionPanelListState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionPanelListState")
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> State for ExpansionPanelListState<T> {
    type Widget = ExpansionPanelList<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).allow_only_one_panel_open {
            return;
        }
        let initial = self.widget(app).initial_open_panel_value.clone();
        if let Some(initial) = initial
            && self.widget(app).radio_values.contains(&initial)
        {
            app.get_mut(self).current_open_panel = Some(initial);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ExpansionPanelList<T>) {
        if self.widget(app).allow_only_one_panel_open {
            if !old_widget.allow_only_one_panel_open {
                let initial = self.widget(app).initial_open_panel_value.clone();
                let value = initial;
                self.set_state(app, |state| state.current_open_panel = value);
            }
        } else {
            self.set_state(app, |state| state.current_open_panel = None);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let children = self.widget(app).children.clone();
        let animation_duration = self.widget(app).animation_duration;
        let expanded_header_padding = self.widget(app).expanded_header_padding;
        let divider_color = self.widget(app).divider_color;
        let elevation = self.widget(app).elevation;
        let expand_icon_color = self.widget(app).expand_icon_color;
        let material_gap_size = self.widget(app).material_gap_size;
        let mut items = Vec::new();

        for index in 0..children.len() {
            if self.is_child_expanded(app, index)
                && index != 0
                && !self.is_child_expanded(app, index - 1)
            {
                items.push(MergeableMaterialItem::Gap(
                    MaterialGap::new(salted_key(context, index * 2 - 1)).size(material_gap_size),
                ));
            }

            let child = children[index].clone();
            let expanded = self.is_child_expanded(app, index);
            let header_widget = (child.header_builder)(app, context, expanded);

            let mut expand_icon: WidgetRef =
                Padding::new(EdgeInsetsDirectional::only(0.0, 0.0, 8.0, 0.0).into())
                    .child(
                        IgnorePointer::new()
                            .ignoring(child.can_tap_on_header)
                            .child({
                                let mut icon = ExpandIcon::new(Some(Rc::new(
                                    move |app: &mut App, is_expanded: bool| {
                                        self.handle_pressed(app, is_expanded, index)
                                    },
                                )))
                                .is_expanded(expanded)
                                .padding(K_EXPAND_ICON_PADDING);
                                if let Some(color) = expand_icon_color {
                                    icon = icon.color(color);
                                }
                                if let Some(splash_color) = child.splash_color {
                                    icon = icon.splash_color(splash_color);
                                }
                                if let Some(highlight_color) = child.highlight_color {
                                    icon = icon.highlight_color(highlight_color);
                                }
                                icon.into_widget()
                            }),
                    )
                    .into_widget();

            if !child.can_tap_on_header {
                let localizations = <dyn MaterialLocalizations>::of(app, context);
                let label = if expanded {
                    localizations.expanded_icon_tap_hint()
                } else {
                    localizations.collapsed_icon_tap_hint()
                };
                expand_icon = Semantics::new()
                    .label(label)
                    .container(true)
                    .child(expand_icon)
                    .into_widget();
            }

            let mut header = Row::new()
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
                                    BoxConstraints::new()
                                        .min_height(K_PANEL_HEADER_COLLAPSED_HEIGHT),
                                )
                                .child(header_widget),
                            ),
                    )
                    .into_widget(),
                    expand_icon.clone(),
                ])
                .into_widget();

            if child.can_tap_on_header {
                header = MergeSemantics::new()
                    .child(
                        InkWell::new()
                            .on_tap(Listener::new(move |app: &mut App| {
                                self.handle_pressed(app, expanded, index);
                            }))
                            .child(header),
                    )
                    .into_widget();
            }

            let mut slice = MaterialSlice::new(
                salted_key(context, index * 2),
                Column::new()
                    .children(vec![
                        header,
                        AnimatedCrossFade::new(
                            LimitedBox::new()
                                .max_width(0.0)
                                .child(SizedBox::new().width(f64::INFINITY).height(0.0)),
                            child.body.clone(),
                            if expanded {
                                CrossFadeState::ShowSecond
                            } else {
                                CrossFadeState::ShowFirst
                            },
                            animation_duration,
                        )
                        .first_curve(Rc::new(Interval::new(0.0, 0.6, Curves::fast_out_slow_in())))
                        .second_curve(Rc::new(Interval::new(0.4, 1.0, Curves::fast_out_slow_in())))
                        .size_curve(Curves::fast_out_slow_in())
                        .into_widget(),
                    ])
                    .into_widget(),
            );
            if let Some(background_color) = child.background_color {
                slice = slice.color(background_color);
            }
            items.push(MergeableMaterialItem::Slice(slice));

            if expanded && index != children.len() - 1 {
                items.push(MergeableMaterialItem::Gap(
                    MaterialGap::new(salted_key(context, index * 2 + 1)).size(material_gap_size),
                ));
            }
        }

        let mut material = MergeableMaterial::new()
            .has_dividers(true)
            .elevation(elevation)
            .children(items);
        if let Some(divider_color) = divider_color {
            material = material.divider_color(divider_color);
        }
        material.into_widget()
    }
}

fn salted_key(context: BuildContext, index: usize) -> KeyRef {
    KeyRef::from(ValueKey::new((context.id(), index)))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_foundation::ValueKey;
    use waft_widgets::testing::{find, test_widgets};
    use waft_widgets::{IntoWidget, SingleChildScrollView, SizedBox, Text};

    use super::*;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn header(collapsed: &'static str, expanded_text: &'static str) -> ExpansionPanelHeaderBuilder {
        Rc::new(move |_app, _context, expanded| {
            Text::new(if expanded { expanded_text } else { collapsed }).into_widget()
        })
    }

    fn panel(collapsed: &'static str) -> ExpansionPanel {
        ExpansionPanel::new(header(collapsed, "B"), SizedBox::new().height(100.0))
    }

    fn pump(tester: &mut waft_widgets::testing::WidgetTester, list: ExpansionPanelList<()>) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            SingleChildScrollView::new().child(list).into_widget(),
        ));
    }

    #[test]
    fn a_panel_reports_that_it_wants_to_expand() {
        test_widgets(|tester| {
            let captured = Rc::new(RefCell::new(None));
            let seen = Rc::clone(&captured);
            let callback: ExpansionPanelCallback = Rc::new(move |_app, index, expanded| {
                *seen.borrow_mut() = Some((index, expanded));
            });
            pump(
                tester,
                ExpansionPanelList::new()
                    .children(vec![panel("A")])
                    .expansion_callback(callback),
            );
            assert!(find::text("A").evaluate(tester).len() == 1);
            assert!(find::text("B").evaluate(tester).is_empty());
            let old_height = tester
                .get_size(find::by_type::<ExpansionPanelList>())
                .height();

            tester.tap(find::by_type::<ExpandIcon>());
            assert_eq!(*captured.borrow(), Some((0, true)));
            assert_eq!(
                tester
                    .get_size(find::by_type::<ExpansionPanelList>())
                    .height(),
                old_height,
                "the callback does not expand the panel by itself"
            );

            pump(
                tester,
                ExpansionPanelList::new()
                    .children(vec![panel("A").is_expanded(true)])
                    .expansion_callback(Rc::new(|_app, _index, _expanded| {})),
            );
            tester.pump_by(Duration::from_millis(200));
            assert!(find::text("A").evaluate(tester).is_empty());
            assert_eq!(find::text("B").evaluate(tester).len(), 1);
            assert!(
                tester
                    .get_size(find::by_type::<ExpansionPanelList>())
                    .height()
                    >= old_height + 100.0
            );
        });
    }

    fn radio_panel(
        value: u32,
        collapsed: &'static str,
        expanded: &'static str,
    ) -> ExpansionPanelRadio<u32> {
        ExpansionPanelRadio::new(
            value,
            header(collapsed, expanded),
            SizedBox::new().height(100.0),
        )
    }

    #[test]
    fn radio_mode_keeps_at_most_one_panel_open() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            let list = ExpansionPanelList::radio(vec![
                radio_panel(0, "A", "B"),
                radio_panel(1, "C", "D"),
                radio_panel(2, "E", "F"),
            ]);
            tester.pump_widget(material_app(
                theme,
                SingleChildScrollView::new().child(list).into_widget(),
            ));
            assert_eq!(find::by_type::<ExpandIcon>().evaluate(tester).len(), 3);
            assert_eq!(find::text("A").evaluate(tester).len(), 1);
            assert!(find::text("B").evaluate(tester).is_empty());

            tester.tap(find::by_type::<ExpandIcon>().at(0));
            tester.pump_by(Duration::from_millis(200));
            tester.pump_and_settle();
            assert!(find::text("A").evaluate(tester).is_empty());
            assert_eq!(find::text("B").evaluate(tester).len(), 1);
            assert_eq!(find::text("C").evaluate(tester).len(), 1);

            tester.tap(find::by_type::<ExpandIcon>().at(1));
            tester.pump_by(Duration::from_millis(200));
            tester.pump_and_settle();
            assert_eq!(find::text("A").evaluate(tester).len(), 1);
            assert!(find::text("B").evaluate(tester).is_empty());
            assert!(find::text("C").evaluate(tester).is_empty());
            assert_eq!(find::text("D").evaluate(tester).len(), 1);
        });
    }

    #[test]
    fn tapping_a_header_expands_when_can_tap_on_header_is_true() {
        test_widgets(|tester| {
            let header_key = KeyRef::from(ValueKey::new("header"));
            let header: ExpansionPanelHeaderBuilder = Rc::new(move |_app, _context, expanded| {
                Text::new(if expanded { "B" } else { "A" })
                    .key(header_key.clone())
                    .into_widget()
            });
            let list = ExpansionPanelList::radio(vec![
                ExpansionPanelRadio::new(0u32, header, SizedBox::new().height(100.0))
                    .can_tap_on_header(true),
            ]);
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                SingleChildScrollView::new().child(list).into_widget(),
            ));
            tester.tap(find::text("A"));
            tester.pump_and_settle();
            assert_eq!(find::text("B").evaluate(tester).len(), 1);
            assert!(find::text("A").evaluate(tester).is_empty());
        });
    }
}
