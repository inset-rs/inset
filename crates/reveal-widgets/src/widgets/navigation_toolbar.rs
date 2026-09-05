//! Flutter counterpart: `widgets/navigation_toolbar.dart`.

use std::any::Any;
use std::rc::Rc;

use reveal_embedder::{Offset, Size, TextDirection};
use reveal_foundation::{App, ValueKey};
use reveal_rendering::{
    BoxConstraints, ChildLayoutId, MultiChildLayoutChildren, MultiChildLayoutDelegate,
};

use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::{CustomMultiChildLayout, Directionality, LayoutId};

/// [`NavigationToolbar`] is a layout helper to position 3 widgets or groups of
/// widgets along a horizontal axis that's sensible for an application's
/// navigation bar such as in Material Design and in iOS.
///
/// The [`leading`](Self::leading) and [`trailing`](Self::trailing) widgets occupy the edges of
/// the widget with reasonable size constraints while the [`middle`](Self::middle) widget
/// occupies the remaining space in either a center aligned or start aligned fashion.
///
/// Either directly use the themed app bars such as the iOS `CupertinoNavigationBar` or wrap
/// this widget with more theming specifications for your own custom app bar.
#[derive(Debug)]
pub struct NavigationToolbar {
    pub key: Option<KeyRef>,
    /// Widget to place at the start of the horizontal toolbar.
    pub leading: Option<WidgetRef>,
    /// Widget to place in the middle of the horizontal toolbar, occupying
    /// as much remaining space as possible.
    pub middle: Option<WidgetRef>,
    /// Widget to place at the end of the horizontal toolbar.
    pub trailing: Option<WidgetRef>,
    /// Whether to align the [`middle`](Self::middle) widget to the center of this widget or
    /// next to the [`leading`](Self::leading) widget when false.
    pub center_middle: bool,
    /// The spacing around the [`middle`](Self::middle) widget on horizontal axis.
    ///
    /// Defaults to [`K_MIDDLE_SPACING`](Self::K_MIDDLE_SPACING).
    pub middle_spacing: f64,
}

impl NavigationToolbar {
    /// The default spacing around the [`middle`](Self::middle) widget in dp.
    pub const K_MIDDLE_SPACING: f64 = 16.0;

    /// Creates a widget that lays out its children in a manner suitable for a
    /// toolbar; Dart's named arguments are the setters.
    pub fn new() -> NavigationToolbar {
        NavigationToolbar {
            key: None,
            leading: None,
            middle: None,
            trailing: None,
            center_middle: true,
            middle_spacing: NavigationToolbar::K_MIDDLE_SPACING,
        }
    }

    /// Dart `NavigationToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationToolbar {
        self.key = Some(key);
        self
    }

    /// Dart `NavigationToolbar(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> NavigationToolbar {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `NavigationToolbar(middle:)`.
    pub fn middle<K>(mut self, middle: impl IntoWidget<K>) -> NavigationToolbar {
        self.middle = Some(middle.into_widget());
        self
    }

    /// Dart `NavigationToolbar(trailing:)`.
    pub fn trailing<K>(mut self, trailing: impl IntoWidget<K>) -> NavigationToolbar {
        self.trailing = Some(trailing.into_widget());
        self
    }

    /// Dart `NavigationToolbar(centerMiddle:)`.
    pub fn center_middle(mut self, center_middle: bool) -> NavigationToolbar {
        self.center_middle = center_middle;
        self
    }

    /// Dart `NavigationToolbar(middleSpacing:)`.
    pub fn middle_spacing(mut self, middle_spacing: f64) -> NavigationToolbar {
        self.middle_spacing = middle_spacing;
        self
    }
}

impl Default for NavigationToolbar {
    fn default() -> NavigationToolbar {
        NavigationToolbar::new()
    }
}

impl StatelessWidget for NavigationToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        // Dart's `debugCheckHasDirectionality` is `Directionality::of`'s own panic.
        let text_direction = Directionality::of(app, context);
        let mut children = Vec::new();
        if let Some(leading) = self.leading.clone() {
            children.push(LayoutId::new(ToolbarSlot::Leading.id(), leading).into_widget());
        }
        if let Some(middle) = self.middle.clone() {
            children.push(LayoutId::new(ToolbarSlot::Middle.id(), middle).into_widget());
        }
        if let Some(trailing) = self.trailing.clone() {
            children.push(LayoutId::new(ToolbarSlot::Trailing.id(), trailing).into_widget());
        }
        CustomMultiChildLayout::new(Rc::new(ToolbarLayout {
            center_middle: self.center_middle,
            middle_spacing: self.middle_spacing,
            text_direction,
        }))
        .children(children)
        .into_widget()
    }
}

/// Dart's `_ToolbarSlot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ToolbarSlot {
    Leading,
    Middle,
    Trailing,
}

impl ToolbarSlot {
    /// The [`ChildLayoutId`] this slot is known by, Dart's enum value as the `LayoutId.id`.
    fn id(self) -> ChildLayoutId {
        Rc::new(ValueKey::new(self))
    }
}

/// Dart's `_ToolbarLayout`.
#[derive(Debug)]
struct ToolbarLayout {
    /// If false the middle widget should be start-justified within the space between the
    /// leading and trailing widgets. If true the middle widget is centered within the toolbar
    /// (not within the horizontal space between the leading and trailing widgets).
    center_middle: bool,
    /// The spacing around middle widget on horizontal axis.
    middle_spacing: f64,
    text_direction: TextDirection,
}

impl MultiChildLayoutDelegate for ToolbarLayout {
    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size) {
        let mut leading_width = 0.0;
        let mut trailing_width = 0.0;

        if children.has_child(&ToolbarSlot::Leading.id()) {
            let constraints = BoxConstraints::new()
                .max_width(size.width())
                // The height should be exactly the height of the bar.
                .min_height(size.height())
                .max_height(size.height());
            leading_width = children
                .layout_child(app, &ToolbarSlot::Leading.id(), constraints)
                .width();
            let leading_x = match self.text_direction {
                TextDirection::Rtl => size.width() - leading_width,
                TextDirection::Ltr => 0.0,
            };
            children.position_child(app, &ToolbarSlot::Leading.id(), Offset::new(leading_x, 0.0));
        }

        if children.has_child(&ToolbarSlot::Trailing.id()) {
            let constraints = BoxConstraints::loose(size);
            let trailing_size =
                children.layout_child(app, &ToolbarSlot::Trailing.id(), constraints);
            let trailing_x = match self.text_direction {
                TextDirection::Rtl => 0.0,
                TextDirection::Ltr => size.width() - trailing_size.width(),
            };
            let trailing_y = (size.height() - trailing_size.height()) / 2.0;
            trailing_width = trailing_size.width();
            children.position_child(
                app,
                &ToolbarSlot::Trailing.id(),
                Offset::new(trailing_x, trailing_y),
            );
        }

        if children.has_child(&ToolbarSlot::Middle.id()) {
            let max_width =
                (size.width() - leading_width - trailing_width - self.middle_spacing * 2.0)
                    .max(0.0);
            let constraints = BoxConstraints::loose(size).max_width(max_width);
            let middle_size = children.layout_child(app, &ToolbarSlot::Middle.id(), constraints);

            let middle_start_margin = leading_width + self.middle_spacing;
            let mut middle_start = middle_start_margin;
            let middle_y = (size.height() - middle_size.height()) / 2.0;
            // If the centered middle will not fit between the leading and trailing
            // widgets, then align its left or right edge with the adjacent boundary.
            if self.center_middle {
                middle_start = (size.width() - middle_size.width()) / 2.0;
                if middle_start + middle_size.width() > size.width() - trailing_width {
                    middle_start =
                        size.width() - trailing_width - middle_size.width() - self.middle_spacing;
                } else if middle_start < middle_start_margin {
                    middle_start = middle_start_margin;
                }
            }

            let middle_x = match self.text_direction {
                TextDirection::Rtl => size.width() - middle_size.width() - middle_start,
                TextDirection::Ltr => middle_start,
            };

            children.position_child(
                app,
                &ToolbarSlot::Middle.id(),
                Offset::new(middle_x, middle_y),
            );
        }
    }

    fn should_relayout(&self, old_delegate: &dyn MultiChildLayoutDelegate) -> bool {
        let old_delegate = old_delegate
            .as_any()
            .downcast_ref::<ToolbarLayout>()
            .expect("the delegate types matched");
        old_delegate.center_middle != self.center_middle
            || old_delegate.middle_spacing != self.middle_spacing
            || old_delegate.text_direction != self.text_direction
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use reveal_rendering::{
        AnyRenderBox, ContainerBoxParentData, ContainerRenderObjectMixin,
        MultiChildLayoutParentData, RenderCustomMultiChildLayoutBox,
    };

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    fn sized(width: f64, height: f64) -> WidgetRef {
        SizedBox {
            width: Some(width),
            height: Some(height),
            ..Default::default()
        }
        .into_widget()
    }

    /// The toolbar's children, with the size and offset the delegate gave each.
    fn placements(harness: &Harness, app: &App) -> Vec<(Size, Offset)> {
        let toolbar: reveal_rendering::RenderHandle<RenderCustomMultiChildLayoutBox> = harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderCustomMultiChildLayoutBox>(app)
            .expect("a RenderCustomMultiChildLayoutBox");
        let mut placements = Vec::new();
        let mut child: Option<AnyRenderBox> = toolbar.first_child(app);
        while let Some(current) = child {
            let parent_data = current
                .as_object()
                .parent_data_of::<MultiChildLayoutParentData>(app);
            placements.push((current.size(app), parent_data.offset()));
            child = toolbar.child_after(app, current);
        }
        placements
    }

    /// A toolbar with fixed-size children under the given direction, mounted and laid out.
    fn toolbar(
        app: &mut App,
        text_direction: TextDirection,
        center_middle: bool,
        leading: Option<(f64, f64)>,
        middle: Option<(f64, f64)>,
        trailing: Option<(f64, f64)>,
    ) -> Harness {
        let mut bar = NavigationToolbar::new().center_middle(center_middle);
        if let Some((width, height)) = leading {
            bar = bar.leading(sized(width, height));
        }
        if let Some((width, height)) = middle {
            bar = bar.middle(sized(width, height));
        }
        if let Some((width, height)) = trailing {
            bar = bar.trailing(sized(width, height));
        }
        let harness = Harness::mount(app, Directionality::new(text_direction, bar).into_widget());
        harness.pump(app);
        harness
    }

    /// The view is 300 x 200, so `size.height` is 200 and the leading widget is stretched to it.
    #[test]
    fn a_centered_middle_sits_in_the_middle_of_the_bar_in_both_directions() {
        let mut app = App::new();
        let harness = toolbar(
            &mut app,
            TextDirection::Ltr,
            true,
            Some((50.0, 20.0)),
            Some((60.0, 20.0)),
            Some((40.0, 20.0)),
        );
        assert_eq!(
            placements(&harness, &app),
            vec![
                (Size::new(50.0, 200.0), Offset::new(0.0, 0.0)),
                (Size::new(60.0, 20.0), Offset::new(120.0, 90.0)),
                (Size::new(40.0, 20.0), Offset::new(260.0, 90.0)),
            ]
        );

        let mut app = App::new();
        let harness = toolbar(
            &mut app,
            TextDirection::Rtl,
            true,
            Some((50.0, 20.0)),
            Some((60.0, 20.0)),
            Some((40.0, 20.0)),
        );
        assert_eq!(
            placements(&harness, &app),
            vec![
                (Size::new(50.0, 200.0), Offset::new(250.0, 0.0)),
                (Size::new(60.0, 20.0), Offset::new(120.0, 90.0)),
                (Size::new(40.0, 20.0), Offset::new(0.0, 90.0)),
            ]
        );
    }

    #[test]
    fn an_uncentered_middle_starts_next_to_the_leading_widget_in_both_directions() {
        let mut app = App::new();
        let harness = toolbar(
            &mut app,
            TextDirection::Ltr,
            false,
            Some((50.0, 20.0)),
            Some((60.0, 20.0)),
            Some((40.0, 20.0)),
        );
        assert_eq!(
            placements(&harness, &app)[1],
            (Size::new(60.0, 20.0), Offset::new(66.0, 90.0)),
            "leading width plus the middle spacing"
        );

        let mut app = App::new();
        let harness = toolbar(
            &mut app,
            TextDirection::Rtl,
            false,
            Some((50.0, 20.0)),
            Some((60.0, 20.0)),
            Some((40.0, 20.0)),
        );
        assert_eq!(
            placements(&harness, &app)[1],
            (Size::new(60.0, 20.0), Offset::new(174.0, 90.0)),
            "the same margin measured from the right edge"
        );
    }

    #[test]
    fn a_centered_middle_that_would_overlap_the_leading_widget_is_pushed_off_it() {
        let mut app = App::new();
        let harness = toolbar(
            &mut app,
            TextDirection::Ltr,
            true,
            Some((200.0, 20.0)),
            Some((100.0, 20.0)),
            Some((40.0, 20.0)),
        );
        assert_eq!(
            placements(&harness, &app)[1],
            (Size::new(28.0, 20.0), Offset::new(216.0, 90.0)),
            "the middle is capped at the free width and aligned with the leading edge"
        );
    }

    #[test]
    fn a_centered_middle_that_would_overlap_the_trailing_widget_is_pushed_off_it() {
        let mut app = App::new();
        let harness = toolbar(
            &mut app,
            TextDirection::Ltr,
            true,
            None,
            Some((100.0, 20.0)),
            Some((200.0, 20.0)),
        );
        assert_eq!(
            placements(&harness, &app),
            vec![
                (Size::new(68.0, 20.0), Offset::new(16.0, 90.0)),
                (Size::new(200.0, 20.0), Offset::new(100.0, 90.0)),
            ],
            "with no leading widget the middle is the first child"
        );
    }

    #[test]
    fn a_toolbar_lays_out_only_the_children_it_was_given() {
        let mut app = App::new();
        let harness = toolbar(
            &mut app,
            TextDirection::Ltr,
            true,
            None,
            Some((60.0, 20.0)),
            None,
        );
        assert_eq!(
            placements(&harness, &app),
            vec![(Size::new(60.0, 20.0), Offset::new(120.0, 90.0))]
        );
    }
}
