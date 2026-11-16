//! Flutter counterpart: material_ui's `tooltip_visibility.dart`.

use waft_foundation::App;
use waft_widgets::{BuildContext, InheritedWidget, IntoWidget, KeyRef, StatelessWidget, WidgetRef};

#[derive(Debug)]
struct TooltipVisibilityScope {
    child: WidgetRef,
    visible: bool,
}

impl InheritedWidget for TooltipVisibilityScope {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &TooltipVisibilityScope) -> bool {
        old.visible != self.visible
    }
}

/// Overrides the visibility of descendant `Tooltip` widgets.
///
/// If disabled, the descendant `Tooltip` widgets will not display a tooltip
/// when tapped, long-pressed, hovered by the mouse, or when
/// `ensure_tooltip_visible` is called. This only visually disables tooltips but
/// continues to provide any semantic information that is provided.
#[derive(Debug)]
pub struct TooltipVisibility {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    ///
    /// The entire app can be wrapped in this widget to globally control `Tooltip`
    /// visibility.
    pub child: WidgetRef,

    /// Determines the visibility of `Tooltip` widgets that inherit from this widget.
    pub visible: bool,
}

impl TooltipVisibility {
    /// Creates a widget that configures the visibility of `Tooltip`.
    pub fn new<K>(visible: bool, child: impl IntoWidget<K>) -> TooltipVisibility {
        TooltipVisibility {
            key: None,
            child: child.into_widget(),
            visible,
        }
    }

    /// Dart `TooltipVisibility(key:)`.
    pub fn key(mut self, key: KeyRef) -> TooltipVisibility {
        self.key = Some(key);
        self
    }

    /// The [`visible`](Self::visible) of the closest instance of this class that encloses the
    /// given context. Defaults to `true` if none are found.
    pub fn of(app: &mut App, context: BuildContext) -> bool {
        context
            .depend_on_inherited_widget_of_exact_type::<TooltipVisibilityScope>(app)
            .is_none_or(|visibility| visibility.visible)
    }
}

impl StatelessWidget for TooltipVisibility {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        TooltipVisibilityScope {
            child: self.child.clone(),
            visible: self.visible,
        }
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use waft_embedder::{Offset, PointerChange};
    use waft_widgets::{Center, GestureDetector, GlobalKey, MouseRegion, RawTooltip, SizedBox};

    use waft_widgets::testing::{
        TestPointer, WidgetTester, expect, find, finds_nothing, finds_one_widget, test_widgets,
    };

    use super::*;
    use crate::tooltip::tests::{
        TOOLTIP_TEXT, found, m3, pump_app, test_gesture_tap, tooltip_center,
    };
    use crate::tooltip::{Tooltip, TooltipState};
    use waft_widgets::TooltipTriggerMode;

    fn sized_100() -> SizedBox {
        SizedBox::new().width(100.0).height(100.0)
    }

    /// Dart's mouse `TestGesture` added at the origin and wiggled before the tree is pumped.
    fn mouse_at_origin(tester: &mut WidgetTester) -> TestPointer {
        let mut gesture = TestPointer::mouse(1, 0);
        tester.add_pointer(&mut gesture, Offset::ZERO);
        tester.move_to(&mut gesture, Offset::new(1.0, 1.0), false);
        tester.pump();
        tester.move_to(&mut gesture, Offset::ZERO, false);
        gesture
    }

    #[test]
    fn tooltip_does_not_build_mouse_region_when_mouse_is_detected_and_in_tooltip_visibility_with_visibility_false()
     {
        test_widgets(|tester| {
            let gesture = mouse_at_origin(tester);
            pump_app(tester, m3, |_app| {
                TooltipVisibility::new(
                    false,
                    Tooltip::new().message(TOOLTIP_TEXT).child(sized_100()),
                )
                .into_widget()
            });

            let raw_tooltips = find::by_type::<RawTooltip>().evaluate(tester);
            let mouse_regions_under_raw_tooltips = find::by_type::<MouseRegion>()
                .evaluate(tester)
                .into_iter()
                .filter(|mouse_region| {
                    tester
                        .ancestors(*mouse_region)
                        .iter()
                        .any(|ancestor| raw_tooltips.contains(ancestor))
                })
                .count();
            assert_eq!(mouse_regions_under_raw_tooltips, 0);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    /// The hover cases: `waitDuration` zero, under a `TooltipVisibility` of `visible`.
    fn hover(tester: &mut WidgetTester, visible: bool) -> TestPointer {
        let mut gesture = mouse_at_origin(tester);
        pump_app(tester, m3, |_app| {
            Center::new()
                .child(TooltipVisibility::new(
                    visible,
                    Tooltip::new()
                        .message(TOOLTIP_TEXT)
                        .wait_duration(Duration::ZERO)
                        .child(sized_100()),
                ))
                .into_widget()
        });

        tester.move_to(&mut gesture, Offset::ZERO, false);
        tester.pump();
        let at = tooltip_center(tester);
        tester.move_to(&mut gesture, at, false);
        tester.pump();
        // Wait for it to appear.
        tester.pump();
        gesture
    }

    #[test]
    fn tooltip_does_not_show_when_hovered_when_in_tooltip_visibility_with_visible_false() {
        test_widgets(|tester| {
            let gesture = hover(tester, false);
            assert!(!found(tester, TOOLTIP_TEXT));
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn tooltip_shows_when_hovered_when_in_tooltip_visibility_with_visible_true() {
        test_widgets(|tester| {
            let mut gesture = hover(tester, true);
            expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());

            // Wait for it to disappear.
            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();
            tester.send(&gesture, PointerChange::Remove);
            assert!(!found(tester, TOOLTIP_TEXT));
        });
    }

    /// Dart's `setWidgetForTooltipMode`.
    fn set_widget_for_tooltip_mode(
        tester: &mut WidgetTester,
        trigger_mode: TooltipTriggerMode,
        visibility: bool,
    ) {
        pump_app(tester, m3, |_app| {
            TooltipVisibility::new(
                visibility,
                Tooltip::new()
                    .message(TOOLTIP_TEXT)
                    .trigger_mode(trigger_mode)
                    .child(sized_100()),
            )
            .into_widget()
        });
    }

    #[test]
    fn tooltip_does_not_build_gesture_detector_when_in_tooltip_visibility_with_visibility_false() {
        test_widgets(|tester| {
            set_widget_for_tooltip_mode(tester, TooltipTriggerMode::Tap, false);

            expect(tester, &find::by_type::<GestureDetector>(), finds_nothing());
        });
    }

    #[test]
    fn tooltip_triggers_on_tap_when_trigger_mode_is_tap_and_in_tooltip_visibility_with_visible_true()
     {
        test_widgets(|tester| {
            set_widget_for_tooltip_mode(tester, TooltipTriggerMode::Tap, true);

            assert!(!found(tester, TOOLTIP_TEXT));

            let at = tooltip_center(tester);
            test_gesture_tap(tester, at);
            expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());
        });
    }

    /// The manual cases: `ensureTooltipVisible` under a `TooltipVisibility` of `visible`.
    fn ensure_visible_under(tester: &mut WidgetTester, visible: bool) {
        let key = GlobalKey::new();
        let tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .child(sized_100());
        pump_app(tester, m3, |_app| {
            TooltipVisibility::new(visible, tooltip).into_widget()
        });

        {
            let mut app = tester.cell.borrow_mut();
            if let Some(state) = key.current_state::<TooltipState>(&app) {
                state.ensure_tooltip_visible(&mut app);
            }
        }
        tester.pump();
    }

    #[test]
    fn tooltip_does_not_trigger_manually_when_in_tooltip_visibility_with_visible_false() {
        test_widgets(|tester| {
            ensure_visible_under(tester, false);
            assert!(!found(tester, TOOLTIP_TEXT));
        });
    }

    #[test]
    fn tooltip_triggers_manually_when_in_tooltip_visibility_with_visible_true() {
        test_widgets(|tester| {
            ensure_visible_under(tester, true);
            expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());
        });
    }
}
