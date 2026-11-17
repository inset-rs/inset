//! Flutter counterpart: material_ui's `button_bar.dart`.
#![allow(
    deprecated,
    reason = "Dart deprecates `ButtonBar` and its theme, which this file still implements"
)]

use std::fmt;

use waft_embedder::{Offset, Size, TextDirection};
use waft_foundation::App;
use waft_painting::{Axis, EdgeInsets, EdgeInsetsGeometry};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, ContainerBoxParentData,
    ContainerRenderObjectData, ContainerRenderObjectMixin, FlexParentData, MainAxisAlignment,
    MainAxisSize, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData, RenderFlexData,
    RenderFlexMixin, RenderHandle, RenderObject, RenderObjectData,
};
use waft_widgets::{
    BuildContext, Center, ConstrainedBox, Directionality, IntoWidget, KeyRef,
    MultiChildRenderObjectWidget, Padding, RenderObjectWidget, StatelessWidget, VerticalDirection,
    WidgetRef,
};

use crate::button_bar_theme::ButtonBarTheme;
use crate::button_theme::ButtonTheme;
use crate::button_theme::{ButtonBarLayoutBehavior, ButtonTextTheme};

/// An end-aligned row of buttons, laying out into a column if there is not enough horizontal
/// space.
///
/// [ButtonBar] has been replaced by a more efficient widget, `OverflowBar`.
///
/// Places the buttons horizontally according to `button_padding`. The children are laid out in
/// a row with `MainAxisAlignment::End`. When the directionality is LTR, the button bar's
/// children are right justified and the last child becomes the rightmost child. When it is RTL
/// the children are left justified and the last child becomes the leftmost child.
///
/// If the button bar's width exceeds the maximum width constraint on the widget, it aligns its
/// buttons in a column.
#[deprecated(note = "Use OverflowBar instead. This feature was deprecated after v3.21.0-10.0.pre.")]
pub struct ButtonBar {
    pub key: Option<KeyRef>,
    /// How the children should be placed along the horizontal axis.
    pub alignment: Option<MainAxisAlignment>,
    /// How much horizontal space is available.
    pub main_axis_size: Option<MainAxisSize>,
    /// Overrides the surrounding `ButtonBarThemeData::button_text_theme`.
    pub button_text_theme: Option<ButtonTextTheme>,
    /// Overrides the surrounding `ButtonThemeData`'s minimum width.
    pub button_min_width: Option<f64>,
    /// Overrides the surrounding `ButtonThemeData`'s height.
    pub button_height: Option<f64>,
    /// Overrides the surrounding `ButtonThemeData`'s padding.
    pub button_padding: Option<EdgeInsetsGeometry>,
    /// Overrides the surrounding `ButtonThemeData`'s aligned-dropdown setting.
    pub button_aligned_dropdown: Option<bool>,
    /// Defines whether the button bar sizes itself with a minimum size constraint or padding.
    pub layout_behavior: Option<ButtonBarLayoutBehavior>,
    /// Defines the vertical direction of the children if they overflow.
    pub overflow_direction: Option<VerticalDirection>,
    /// The spacing between buttons when they overflow.
    pub overflow_button_spacing: Option<f64>,
    /// The buttons to arrange horizontally.
    pub children: Vec<WidgetRef>,
}

impl ButtonBar {
    /// Creates a button bar; Dart's optional named arguments are the setters.
    pub fn new() -> ButtonBar {
        ButtonBar {
            key: None,
            alignment: None,
            main_axis_size: None,
            button_text_theme: None,
            button_min_width: None,
            button_height: None,
            button_padding: None,
            button_aligned_dropdown: None,
            layout_behavior: None,
            overflow_direction: None,
            overflow_button_spacing: None,
            children: Vec::new(),
        }
    }

    /// Dart `ButtonBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> ButtonBar {
        self.key = Some(key);
        self
    }

    /// Dart `ButtonBar(alignment:)`.
    pub fn alignment(mut self, alignment: MainAxisAlignment) -> ButtonBar {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `ButtonBar(mainAxisSize:)`.
    pub fn main_axis_size(mut self, main_axis_size: MainAxisSize) -> ButtonBar {
        self.main_axis_size = Some(main_axis_size);
        self
    }

    /// Dart `ButtonBar(buttonTextTheme:)`.
    pub fn button_text_theme(mut self, button_text_theme: ButtonTextTheme) -> ButtonBar {
        self.button_text_theme = Some(button_text_theme);
        self
    }

    /// Dart `ButtonBar(buttonMinWidth:)`.
    pub fn button_min_width(mut self, button_min_width: f64) -> ButtonBar {
        debug_assert!(button_min_width >= 0.0);
        self.button_min_width = Some(button_min_width);
        self
    }

    /// Dart `ButtonBar(buttonHeight:)`.
    pub fn button_height(mut self, button_height: f64) -> ButtonBar {
        debug_assert!(button_height >= 0.0);
        self.button_height = Some(button_height);
        self
    }

    /// Dart `ButtonBar(buttonPadding:)`.
    pub fn button_padding(mut self, button_padding: impl Into<EdgeInsetsGeometry>) -> ButtonBar {
        self.button_padding = Some(button_padding.into());
        self
    }

    /// Dart `ButtonBar(buttonAlignedDropdown:)`.
    pub fn button_aligned_dropdown(mut self, button_aligned_dropdown: bool) -> ButtonBar {
        self.button_aligned_dropdown = Some(button_aligned_dropdown);
        self
    }

    /// Dart `ButtonBar(layoutBehavior:)`.
    pub fn layout_behavior(mut self, layout_behavior: ButtonBarLayoutBehavior) -> ButtonBar {
        self.layout_behavior = Some(layout_behavior);
        self
    }

    /// Dart `ButtonBar(overflowDirection:)`.
    pub fn overflow_direction(mut self, overflow_direction: VerticalDirection) -> ButtonBar {
        self.overflow_direction = Some(overflow_direction);
        self
    }

    /// Dart `ButtonBar(overflowButtonSpacing:)`.
    pub fn overflow_button_spacing(mut self, overflow_button_spacing: f64) -> ButtonBar {
        debug_assert!(overflow_button_spacing >= 0.0);
        self.overflow_button_spacing = Some(overflow_button_spacing);
        self
    }

    /// Dart `ButtonBar(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> ButtonBar {
        self.children = children.into_iter().collect();
        self
    }
}

impl Default for ButtonBar {
    fn default() -> ButtonBar {
        ButtonBar::new()
    }
}

impl fmt::Debug for ButtonBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ButtonBar")
            .field("alignment", &self.alignment)
            .field("main_axis_size", &self.main_axis_size)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for ButtonBar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let parent_button_theme = ButtonTheme::of(app, context);
        let bar_theme = ButtonBarTheme::of(app, context);

        let button_theme = parent_button_theme
            .copy_with()
            .with_text_theme(
                self.button_text_theme
                    .or(bar_theme.button_text_theme)
                    .unwrap_or(ButtonTextTheme::Primary),
            )
            .with_min_width(
                self.button_min_width
                    .or(bar_theme.button_min_width)
                    .unwrap_or(64.0),
            )
            .with_height(
                self.button_height
                    .or(bar_theme.button_height)
                    .unwrap_or(36.0),
            )
            .with_padding(
                self.button_padding
                    .or(bar_theme.button_padding)
                    .unwrap_or_else(|| EdgeInsetsGeometry::symmetric(0.0, 8.0)),
            )
            .with_aligned_dropdown(
                self.button_aligned_dropdown
                    .or(bar_theme.button_aligned_dropdown)
                    .unwrap_or(false),
            )
            .with_layout_behavior(
                self.layout_behavior
                    .or(bar_theme.layout_behavior)
                    .unwrap_or(ButtonBarLayoutBehavior::Padded),
            );

        let padding_unit = button_theme.padding().horizontal() / 4.0;
        let child = ButtonTheme::from_button_theme_data(
            button_theme.clone(),
            ButtonBarRow {
                main_axis_alignment: self
                    .alignment
                    .or(bar_theme.alignment)
                    .unwrap_or(MainAxisAlignment::End),
                main_axis_size: self
                    .main_axis_size
                    .or(bar_theme.main_axis_size)
                    .unwrap_or(MainAxisSize::Max),
                overflow_direction: self
                    .overflow_direction
                    .or(bar_theme.overflow_direction)
                    .unwrap_or(VerticalDirection::Down),
                overflow_button_spacing: self.overflow_button_spacing,
                children: self
                    .children
                    .iter()
                    .map(|child| {
                        Padding::new(EdgeInsets::symmetric(0.0, padding_unit).into())
                            .child(child.clone())
                            .into_widget()
                    })
                    .collect(),
            },
        );

        match button_theme.layout_behavior {
            ButtonBarLayoutBehavior::Padded => {
                Padding::new(EdgeInsets::symmetric(2.0 * padding_unit, padding_unit).into())
                    .child(child)
                    .into_widget()
            }
            ButtonBarLayoutBehavior::Constrained => {
                ConstrainedBox::new(BoxConstraints::new().min_height(52.0))
                    .child(
                        Padding::new(EdgeInsets::symmetric(0.0, padding_unit).into())
                            .child(Center::new().child(child)),
                    )
                    .into_widget()
            }
        }
    }
}

/// Dart's `_ButtonBarRow`.
struct ButtonBarRow {
    main_axis_alignment: MainAxisAlignment,
    main_axis_size: MainAxisSize,
    overflow_direction: VerticalDirection,
    overflow_button_spacing: Option<f64>,
    children: Vec<WidgetRef>,
}

impl RenderObjectWidget for ButtonBarRow {
    type RenderObject = RenderButtonBarRow;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction =
            Directionality::maybe_of(app, context).expect("a button bar needs a Directionality");
        let render_object = RenderButtonBarRow::new(
            app,
            text_direction,
            self.main_axis_alignment,
            self.main_axis_size,
            self.overflow_direction,
            self.overflow_button_spacing,
        );
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderButtonBarRow>,
    ) {
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_main_axis_alignment(app, self.main_axis_alignment);
        render_object.set_main_axis_size(app, self.main_axis_size);
        render_object.set_vertical_direction(app, self.overflow_direction);
        render_object.set_overflow_button_spacing(app, self.overflow_button_spacing);
    }
}

impl MultiChildRenderObjectWidget for ButtonBarRow {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

impl fmt::Debug for ButtonBarRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ButtonBarRow").finish_non_exhaustive()
    }
}

/// Dart's `_RenderButtonBarRow`.
struct RenderButtonBarRow {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    flex: RenderFlexData,
    overflow_button_spacing: Option<f64>,
    has_checked_layout_width: bool,
}

impl RenderButtonBarRow {
    fn new(
        app: &mut App,
        text_direction: TextDirection,
        main_axis_alignment: MainAxisAlignment,
        main_axis_size: MainAxisSize,
        overflow_direction: VerticalDirection,
        overflow_button_spacing: Option<f64>,
    ) -> RenderHandle<RenderButtonBarRow> {
        debug_assert!(overflow_button_spacing.is_none_or(|spacing| spacing >= 0.0));
        RenderHandle::new_box(
            app,
            RenderButtonBarRow {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                flex: RenderFlexData::new()
                    .direction(Axis::Horizontal)
                    .main_axis_alignment(main_axis_alignment)
                    .main_axis_size(main_axis_size)
                    .vertical_direction(overflow_direction)
                    .text_direction(Some(text_direction)),
                overflow_button_spacing,
                has_checked_layout_width: false,
            },
        )
    }

    fn overflow_button_spacing(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).overflow_button_spacing
    }

    fn set_overflow_button_spacing(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).overflow_button_spacing == value {
            return;
        }
        self.get_mut(app).overflow_button_spacing = value;
        self.mark_needs_layout(app);
    }

    fn set_has_checked_layout_width(self: RenderHandle<Self>, app: &mut App, value: bool) {
        self.get_mut(app).has_checked_layout_width = value;
    }
}

impl ContainerRenderObjectMixin for RenderButtonBarRow {
    type ChildType = AnyRenderBox;
    type ParentDataType = FlexParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderButtonBarRow {}

impl RenderFlexMixin for RenderButtonBarRow {
    fn flex_data(self: RenderHandle<Self>, app: &App) -> &RenderFlexData {
        &self.get(app).flex
    }

    fn flex_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderFlexData {
        &mut self.get_mut(app).flex
    }
}

impl RenderObject for RenderButtonBarRow {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        self.set_has_checked_layout_width(app, false);
        RenderFlexMixin::perform_layout(self, app);
        self.set_has_checked_layout_width(app, true);
        let constraints = self.constraints(app);
        if self.size(app).width() <= constraints.max_width {
            RenderFlexMixin::perform_layout(self, app);
            return;
        }

        let child_constraints = constraints.copy_with().min_width(0.0);
        let mut current_height = 0.0;
        let mut child = match self.vertical_direction(app) {
            VerticalDirection::Down => self.first_child(app),
            VerticalDirection::Up => self.last_child(app),
        };
        while let Some(current) = child {
            current.layout(app, child_constraints, true);
            let child_size = current.size(app);
            let text_direction = self
                .text_direction(app)
                .expect("a button bar row is given a text direction");
            let offset = match text_direction {
                TextDirection::Ltr => match self.main_axis_alignment(app) {
                    MainAxisAlignment::Center => Offset::new(
                        (constraints.max_width - child_size.width()) / 2.0,
                        current_height,
                    ),
                    MainAxisAlignment::End => {
                        Offset::new(constraints.max_width - child_size.width(), current_height)
                    }
                    MainAxisAlignment::SpaceAround
                    | MainAxisAlignment::SpaceBetween
                    | MainAxisAlignment::SpaceEvenly
                    | MainAxisAlignment::Start => Offset::new(0.0, current_height),
                },
                TextDirection::Rtl => match self.main_axis_alignment(app) {
                    MainAxisAlignment::Center => Offset::new(
                        constraints.max_width / 2.0 - child_size.width() / 2.0,
                        current_height,
                    ),
                    MainAxisAlignment::End => Offset::new(0.0, current_height),
                    MainAxisAlignment::SpaceAround
                    | MainAxisAlignment::SpaceBetween
                    | MainAxisAlignment::SpaceEvenly
                    | MainAxisAlignment::Start => {
                        Offset::new(constraints.max_width - child_size.width(), current_height)
                    }
                },
            };
            current
                .as_object()
                .parent_data_of_mut::<FlexParentData>(app)
                .set_offset(offset);
            current_height += child_size.height();
            child = match self.vertical_direction(app) {
                VerticalDirection::Down => self.child_after(app, current),
                VerticalDirection::Up => self.child_before(app, current),
            };
            if let Some(spacing) = self.overflow_button_spacing(app)
                && child.is_some()
            {
                current_height += spacing;
            }
        }
        self.set_size(
            app,
            constraints.constrain(Size::new(constraints.max_width, current_height)),
        );
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
        owner: waft_foundation::Handle<waft_rendering::PipelineOwner>,
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
        RenderFlexMixin::paint(self, app, context, offset)
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        RenderFlexMixin::dispose(self, app)
    }
}

impl RenderBox for RenderButtonBarRow {
    waft_rendering::render_box_accessors!();

    fn constraints(self: RenderHandle<Self>, app: &App) -> BoxConstraints {
        let constraints = self
            .render_box_data(app)
            .constraints()
            .expect("the button bar has been given constraints");
        if self.get(app).has_checked_layout_width {
            constraints
        } else {
            constraints.copy_with().max_width(f64::INFINITY)
        }
    }

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderFlexMixin::setup_parent_data(self, app, child)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: waft_embedder::TextBaseline,
    ) -> Option<f64> {
        RenderFlexMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let size = RenderFlexMixin::compute_dry_layout(
            self,
            app,
            constraints.copy_with().max_width(f64::INFINITY),
        );
        if size.width() <= constraints.max_width {
            return RenderFlexMixin::compute_dry_layout(self, app, constraints);
        }
        let mut current_height = 0.0;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_constraints = constraints.copy_with().min_width(0.0);
            let child_size = current.get_dry_layout(app, child_constraints);
            current_height += child_size.height();
            child = self.child_after(app, current);
            if let Some(spacing) = self.overflow_button_spacing(app)
                && child.is_some()
            {
                current_height += spacing;
            }
        }
        constraints.constrain(Size::new(constraints.max_width, current_height))
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: waft_embedder::TextBaseline,
    ) -> Option<f64> {
        RenderFlexMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderFlexMixin::hit_test_children(self, app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::Size;
    use waft_foundation::{Listener, ValueKey};
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{
        Center, Directionality, IntoWidget, KeyRef, ListBody, MainAxisAlignment, MainAxisSize,
        SingleChildScrollView, SizedBox, Text, TextDirection, VerticalDirection,
    };

    use super::*;
    use crate::button_bar_theme::{ButtonBarTheme, ButtonBarThemeData};
    use crate::button_theme::ButtonThemeData;
    use crate::test_support::{material_app, probe};
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;

    /// Dart's `pumpWidget(MaterialApp(home: bar))`.
    fn pump_home(tester: &mut WidgetTester, bar: ButtonBar) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, bar.into_widget()));
    }

    /// Dart's `pumpWidget(MaterialApp(home: Center(child: bar)))`.
    fn pump_centered(tester: &mut WidgetTester, bar: ButtonBar) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, Center::new().child(bar).into_widget()));
    }

    /// Dart's `pumpWidget(MaterialApp(home: widget))`, for a theme-wrapped bar.
    fn pump_wrapped(tester: &mut WidgetTester, widget: WidgetRef) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, widget));
    }

    /// Dart's `SingleChildScrollView(child: ListBody(children: [Directionality(ltr, bar)]))`:
    /// the bar is laid out with a loose height, as it is in a column.
    fn pump_scrolling(tester: &mut WidgetTester, bar: ButtonBar) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            SingleChildScrollView::new()
                .child(ListBody::new().children(vec![
                    Directionality::new(TextDirection::Ltr, bar).into_widget(),
                ]))
                .into_widget(),
        ));
    }

    fn boxed(size: f64) -> WidgetRef {
        SizedBox::new().width(size).height(size).into_widget()
    }

    fn keyed(width: f64, height: f64, name: &'static str) -> (KeyRef, WidgetRef) {
        let key = KeyRef::from(ValueKey::new(name));
        (
            key.clone(),
            SizedBox::new()
                .key(key)
                .width(width)
                .height(height)
                .into_widget(),
        )
    }

    fn overflow_bar(
        name: &'static str,
        alignment: MainAxisAlignment,
    ) -> (KeyRef, KeyRef, ButtonBar) {
        let (first_key, first) = keyed(500.0, 50.0, name);
        let (second_key, second) = keyed(500.0, 50.0, "second");
        (
            first_key,
            second_key,
            ButtonBar::new()
                .alignment(alignment)
                .button_padding(waft_painting::EdgeInsets::ZERO)
                .children(vec![first, second]),
        )
    }

    #[test]
    fn button_bar_default_control_smoketest() {
        test_widgets(|tester| {
            tester.pump_widget(Directionality::new(TextDirection::Ltr, ButtonBar::new()));
        });
    }

    #[test]
    fn the_default_alignment_is_end() {
        test_widgets(|tester| {
            pump_home(tester, ButtonBar::new().children(vec![boxed(10.0)]));
            let child = tester.get_rect(find::by_type::<SizedBox>().first());
            assert_eq!(child.left, 782.0);
            assert_eq!(child.right, 792.0);
        });
    }

    #[test]
    fn the_bar_theme_alignment_overrides_the_default() {
        test_widgets(|tester| {
            let data = ButtonBarThemeData::new().with_alignment(MainAxisAlignment::Center);
            pump_wrapped(
                tester,
                ButtonBarTheme::new(data, ButtonBar::new().children(vec![boxed(10.0)]))
                    .into_widget(),
            );
            let child = tester.get_rect(find::by_type::<SizedBox>().first());
            assert_eq!(child.left, 395.0);
            assert_eq!(child.right, 405.0);
        });
    }

    #[test]
    fn an_alignment_overrides_the_bar_theme_and_the_default() {
        test_widgets(|tester| {
            let data = ButtonBarThemeData::new().with_alignment(MainAxisAlignment::Center);
            pump_wrapped(
                tester,
                ButtonBarTheme::new(
                    data,
                    ButtonBar::new()
                        .alignment(MainAxisAlignment::Start)
                        .children(vec![boxed(10.0)]),
                )
                .into_widget(),
            );
            let child = tester.get_rect(find::by_type::<SizedBox>().first());
            assert_eq!(child.left, 8.0);
            assert_eq!(child.right, 18.0);
        });
    }

    fn three_children(name: &'static str) -> (KeyRef, KeyRef, KeyRef, Vec<WidgetRef>) {
        let (child0, widget0) = keyed(100.0, 100.0, name);
        let (child1, widget1) = keyed(100.0, 100.0, "child1");
        let (child2, widget2) = keyed(100.0, 100.0, "child2");
        (child0, child1, child2, vec![widget0, widget1, widget2])
    }

    #[test]
    fn the_default_main_axis_size_is_max() {
        test_widgets(|tester| {
            let bar_key = KeyRef::from(ValueKey::new("row"));
            let (child0, child1, child2, children) = three_children("child0");
            pump_centered(
                tester,
                ButtonBar::new()
                    .key(bar_key.clone())
                    .button_padding(waft_painting::EdgeInsets::ZERO)
                    .children(children),
            );

            let bar = tester.get_rect(find::by_key(bar_key));
            assert_eq!(bar.width(), 800.0);
            assert_eq!(bar.height(), 100.0);

            let child0 = tester.get_rect(find::by_key(child0));
            assert_eq!(child0.size(), Size::new(100.0, 100.0));
            assert_eq!(child0.right, 800.0 - 200.0);
            let child1 = tester.get_rect(find::by_key(child1));
            assert_eq!(child1.size(), Size::new(100.0, 100.0));
            assert_eq!(child1.right, 800.0 - 100.0);
            let child2 = tester.get_rect(find::by_key(child2));
            assert_eq!(child2.size(), Size::new(100.0, 100.0));
            assert_eq!(child2.right, 800.0);
        });
    }

    #[test]
    fn the_bar_theme_main_axis_size_overrides_the_default() {
        test_widgets(|tester| {
            let bar_key = KeyRef::from(ValueKey::new("row"));
            let (child0, child1, child2, children) = three_children("child0");
            let data = ButtonBarThemeData::new().with_main_axis_size(MainAxisSize::Min);
            pump_wrapped(
                tester,
                ButtonBarTheme::new(
                    data,
                    Center::new().child(
                        ButtonBar::new()
                            .key(bar_key.clone())
                            .button_padding(waft_painting::EdgeInsets::ZERO)
                            .children(children),
                    ),
                )
                .into_widget(),
            );

            let bar = tester.get_rect(find::by_key(bar_key));
            assert_eq!(bar.width(), 300.0);
            assert_eq!(bar.height(), 100.0);

            let left = (800.0 - bar.width()) / 2.0;
            let child0 = tester.get_rect(find::by_key(child0));
            assert_eq!(child0.size(), Size::new(100.0, 100.0));
            assert_eq!(child0.left, left);
            let child1 = tester.get_rect(find::by_key(child1));
            assert_eq!(child1.size(), Size::new(100.0, 100.0));
            assert_eq!(child1.left, left + 100.0);
            let child2 = tester.get_rect(find::by_key(child2));
            assert_eq!(child2.size(), Size::new(100.0, 100.0));
            assert_eq!(child2.left, left + 200.0);
        });
    }

    #[test]
    fn a_main_axis_size_overrides_the_bar_theme_and_the_default() {
        test_widgets(|tester| {
            let bar_key = KeyRef::from(ValueKey::new("row"));
            let (child0, child1, child2, children) = three_children("child0");
            let data = ButtonBarThemeData::new().with_main_axis_size(MainAxisSize::Min);
            pump_wrapped(
                tester,
                ButtonBarTheme::new(
                    data,
                    Center::new().child(
                        ButtonBar::new()
                            .key(bar_key.clone())
                            .button_padding(waft_painting::EdgeInsets::ZERO)
                            .main_axis_size(MainAxisSize::Max)
                            .children(children),
                    ),
                )
                .into_widget(),
            );

            let bar = tester.get_rect(find::by_key(bar_key));
            assert_eq!(bar.width(), 800.0);
            assert_eq!(bar.height(), 100.0);
            let child0 = tester.get_rect(find::by_key(child0));
            assert_eq!(child0.size(), Size::new(100.0, 100.0));
            assert_eq!(child0.right, 800.0 - 200.0);
            let child1 = tester.get_rect(find::by_key(child1));
            assert_eq!(child1.size(), Size::new(100.0, 100.0));
            assert_eq!(child1.right, 800.0 - 100.0);
            let child2 = tester.get_rect(find::by_key(child2));
            assert_eq!(child2.size(), Size::new(100.0, 100.0));
            assert_eq!(child2.right, 800.0);
        });
    }

    /// The `ButtonThemeData` a child of `bar` sees, under the theme `wrap` adds.
    fn child_button_theme(
        tester: &mut WidgetTester,
        bar_theme: Option<ButtonBarThemeData>,
        bar: impl FnOnce(WidgetRef) -> ButtonBar,
    ) -> ButtonThemeData {
        probe(
            tester,
            move |app, probe| {
                let bar = bar(probe).into_widget();
                let home = match bar_theme {
                    Some(data) => ButtonBarTheme::new(data, bar).into_widget(),
                    None => bar,
                };
                material_app(ThemeData::builder().build(app), home)
            },
            ButtonTheme::of,
        )
    }

    #[test]
    fn default_button_properties_override_button_theme_properties() {
        test_widgets(|tester| {
            let button_theme =
                child_button_theme(tester, None, |child| ButtonBar::new().children(vec![child]));
            assert_eq!(button_theme.text_theme, ButtonTextTheme::Primary);
            assert_eq!(button_theme.min_width, 64.0);
            assert_eq!(button_theme.height, 36.0);
            assert_eq!(
                button_theme.padding(),
                EdgeInsets::symmetric(0.0, 8.0).into()
            );
            assert!(!button_theme.aligned_dropdown);
            assert_eq!(
                button_theme.layout_behavior,
                ButtonBarLayoutBehavior::Padded
            );
        });
    }

    #[test]
    fn button_bar_theme_button_properties_override_defaults_and_button_theme_properties() {
        test_widgets(|tester| {
            let bar_theme = ButtonBarThemeData::new()
                .with_button_text_theme(ButtonTextTheme::Primary)
                .with_button_min_width(42.0)
                .with_button_height(84.0)
                .with_button_padding(EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0))
                .with_button_aligned_dropdown(true)
                .with_layout_behavior(ButtonBarLayoutBehavior::Constrained);
            let button_theme = child_button_theme(tester, Some(bar_theme), |child| {
                ButtonBar::new().children(vec![child])
            });
            assert_eq!(button_theme.text_theme, ButtonTextTheme::Primary);
            assert_eq!(button_theme.min_width, 42.0);
            assert_eq!(button_theme.height, 84.0);
            assert_eq!(
                button_theme.padding(),
                EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0).into()
            );
            assert!(button_theme.aligned_dropdown);
            assert_eq!(
                button_theme.layout_behavior,
                ButtonBarLayoutBehavior::Constrained
            );
        });
    }

    #[test]
    fn button_bar_button_properties_override_button_bar_theme_defaults_and_button_theme_properties()
    {
        test_widgets(|tester| {
            let bar_theme = ButtonBarThemeData::new()
                .with_button_text_theme(ButtonTextTheme::Accent)
                .with_button_min_width(4242.0)
                .with_button_height(8484.0)
                .with_button_padding(EdgeInsets::from_ltrb(50.0, 60.0, 70.0, 80.0))
                .with_button_aligned_dropdown(false)
                .with_layout_behavior(ButtonBarLayoutBehavior::Padded);
            let button_theme = child_button_theme(tester, Some(bar_theme), |child| {
                ButtonBar::new()
                    .button_text_theme(ButtonTextTheme::Primary)
                    .button_min_width(42.0)
                    .button_height(84.0)
                    .button_padding(EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0))
                    .button_aligned_dropdown(true)
                    .layout_behavior(ButtonBarLayoutBehavior::Constrained)
                    .children(vec![child])
            });
            assert_eq!(button_theme.text_theme, ButtonTextTheme::Primary);
            assert_eq!(button_theme.min_width, 42.0);
            assert_eq!(button_theme.height, 84.0);
            assert_eq!(
                button_theme.padding(),
                EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0).into()
            );
            assert!(button_theme.aligned_dropdown);
            assert_eq!(
                button_theme.layout_behavior,
                ButtonBarLayoutBehavior::Constrained
            );
        });
    }

    #[test]
    fn a_constrained_layout_has_a_minimum_height_of_fifty_two() {
        test_widgets(|tester| {
            pump_scrolling(
                tester,
                ButtonBar::new()
                    .layout_behavior(ButtonBarLayoutBehavior::Constrained)
                    .children(vec![boxed(10.0)]),
            );
            let bar = tester.get_rect(find::by_type::<ButtonBar>());
            assert_eq!(bar.bottom - bar.top, 52.0);
        });
    }

    #[test]
    fn a_padded_layout_adds_its_padding() {
        test_widgets(|tester| {
            pump_scrolling(
                tester,
                ButtonBar::new()
                    .layout_behavior(ButtonBarLayoutBehavior::Padded)
                    .children(vec![boxed(10.0)]),
            );
            let bar = tester.get_rect(find::by_type::<ButtonBar>());
            assert_eq!(bar.bottom - bar.top, 26.0);
        });
    }

    #[test]
    fn the_children_wrap_when_they_overflow() {
        test_widgets(|tester| {
            let (first_key, first) = keyed(800.0, 50.0, "one");
            let (second_key, second) = keyed(800.0, 50.0, "two");
            pump_home(tester, ButtonBar::new().children(vec![first, second]));
            let first = tester.get_rect(find::by_key(first_key));
            let second = tester.get_rect(find::by_key(second_key));
            assert_eq!(first.bottom, second.top);
            assert_eq!(first.left, second.left);
        });
    }

    #[test]
    fn overflowing_children_default_to_the_end_alignment() {
        test_widgets(|tester| {
            let (first_key, second_key, bar) = overflow_bar("one", MainAxisAlignment::End);
            pump_home(tester, bar);
            let bar = tester.get_rect(find::by_type::<ButtonBar>());
            let first = tester.get_rect(find::by_key(first_key));
            let second = tester.get_rect(find::by_key(second_key));
            assert_eq!(first.bottom, second.top);
            assert_eq!(first.right, second.right);
            assert_eq!(first.right, bar.right);
        });
    }

    #[test]
    fn overflowing_children_follow_a_start_alignment() {
        test_widgets(|tester| {
            let (first_key, second_key, bar) = overflow_bar("one", MainAxisAlignment::Start);
            pump_home(tester, bar);
            let bar = tester.get_rect(find::by_type::<ButtonBar>());
            let first = tester.get_rect(find::by_key(first_key));
            let second = tester.get_rect(find::by_key(second_key));
            assert_eq!(first.bottom, second.top);
            assert_eq!(first.left, second.left);
            assert_eq!(first.left, bar.left);
        });
    }

    #[test]
    fn overflowing_children_follow_a_center_alignment() {
        test_widgets(|tester| {
            let (first_key, second_key, bar) = overflow_bar("one", MainAxisAlignment::Center);
            pump_home(tester, bar);
            let bar = tester.get_rect(find::by_type::<ButtonBar>());
            let first = tester.get_rect(find::by_key(first_key));
            let second = tester.get_rect(find::by_key(second_key));
            assert_eq!(first.bottom, second.top);
            assert_eq!(first.center().dx(), second.center().dx());
            assert_eq!(first.center().dx(), bar.center().dx());
        });
    }

    #[test]
    fn overflowing_children_of_a_spread_alignment_start_at_the_start() {
        test_widgets(|tester| {
            for alignment in [
                MainAxisAlignment::SpaceEvenly,
                MainAxisAlignment::SpaceAround,
                MainAxisAlignment::SpaceBetween,
            ] {
                let (first_key, second_key, bar) = overflow_bar("one", alignment);
                pump_home(tester, bar);
                let bar_rect = tester.get_rect(find::by_type::<ButtonBar>());
                let first = tester.get_rect(find::by_key(first_key));
                let second = tester.get_rect(find::by_key(second_key));
                assert_eq!(first.bottom, second.top);
                assert_eq!(first.left, second.left);
                assert_eq!(first.left, bar_rect.left);
            }
        });
    }

    #[test]
    fn overflowing_children_respect_the_overflow_direction() {
        test_widgets(|tester| {
            let (first_key, second_key, bar) = overflow_bar("one", MainAxisAlignment::Center);
            pump_home(tester, bar.overflow_direction(VerticalDirection::Up));
            let first = tester.get_rect(find::by_key(first_key));
            let second = tester.get_rect(find::by_key(second_key));
            assert!(second.bottom <= first.top);
        });
    }

    #[test]
    fn overflowing_children_have_no_spacing_by_default() {
        test_widgets(|tester| {
            let (first_key, second_key, bar) = overflow_bar("one", MainAxisAlignment::Center);
            pump_home(tester, bar);
            let first = tester.get_rect(find::by_key(first_key));
            let second = tester.get_rect(find::by_key(second_key));
            assert_eq!(first.bottom, second.top);
        });
    }

    #[test]
    fn overflowing_children_respect_the_overflow_button_spacing() {
        test_widgets(|tester| {
            let (first_key, second_key, bar) = overflow_bar("one", MainAxisAlignment::Center);
            pump_home(tester, bar.overflow_button_spacing(10.0));
            let first = tester.get_rect(find::by_key(first_key));
            let second = tester.get_rect(find::by_key(second_key));
            assert_eq!(first.bottom, second.top - 10.0);
        });
    }

    #[test]
    fn the_bar_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(SizedBox::shrink().child(ButtonBar::new().children(vec![
                        TextButton::new(Some(Listener::new(|_app| {})), Some(Text::new("X")))
                            .into_widget(),
                    ])))
                    .into_widget(),
            ));
            assert_eq!(tester.get_size(find::by_type::<ButtonBar>()), Size::ZERO);
        });
    }
}
