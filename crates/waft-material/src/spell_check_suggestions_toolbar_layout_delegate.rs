//! Flutter counterpart: material_ui's `spell_check_suggestions_toolbar_layout_delegate.dart`.

use std::any::Any;

use waft_embedder::{Offset, Size};
use waft_rendering::{BoxConstraints, SingleChildLayoutDelegate};
use waft_widgets::TextSelectionToolbarLayoutDelegate;

/// Positions the toolbar below [`anchor`](Self::anchor) or adjusts it higher to fit above
/// the bottom view insets, if applicable.
///
/// See also:
///
///   * `SpellCheckSuggestionsToolbar`, which uses this to position itself.
#[derive(Debug)]
pub struct SpellCheckSuggestionsToolbarLayoutDelegate {
    /// The focal point below which the toolbar attempts to position itself.
    ///
    /// Should be provided in local coordinates.
    pub anchor: Offset,
}

impl SpellCheckSuggestionsToolbarLayoutDelegate {
    /// Creates an instance of [`SpellCheckSuggestionsToolbarLayoutDelegate`].
    pub fn new(anchor: Offset) -> SpellCheckSuggestionsToolbarLayoutDelegate {
        SpellCheckSuggestionsToolbarLayoutDelegate { anchor }
    }
}

impl SingleChildLayoutDelegate for SpellCheckSuggestionsToolbarLayoutDelegate {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        constraints.loosen()
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        Offset::new(
            TextSelectionToolbarLayoutDelegate::center_on(
                self.anchor.dx(),
                child_size.width(),
                size.width(),
            ),
            // Positions child (of childSize) just enough upwards to fit within size
            // if it otherwise does not fit below the anchor.
            if self.anchor.dy() + child_size.height() > size.height() {
                size.height() - child_size.height()
            } else {
                self.anchor.dy()
            },
        )
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<SpellCheckSuggestionsToolbarLayoutDelegate>()
            .is_none_or(|old| self.anchor != old.anchor)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Color, TextDirection};
    use waft_widgets::{
        Container, CustomSingleChildLayout, Directionality, IntoWidget, MediaQuery, MediaQueryData,
        StateSetter, StatefulBuilder,
    };

    use super::*;
    use crate::test_support::{WidgetTester, with_main_thread_stack};

    #[test]
    fn positions_itself_at_anchor_above_if_it_fits_and_shifts_up_when_not() {
        with_main_thread_stack(|| {
            let mut tester = WidgetTester::on(waft_embedder::TargetPlatform::Android);
            let set_state: Rc<RefCell<Option<StateSetter>>> = Rc::new(RefCell::new(None));
            const TOOLBAR_OVERLAP: f64 = 100.0;
            const HEIGHT: f64 = 500.0;
            let anchor_y = Rc::new(Cell::new(200.0));

            let builder = {
                let set_state = Rc::clone(&set_state);
                let anchor_y = Rc::clone(&anchor_y);
                StatefulBuilder::new(move |_app, _context, setter| {
                    *set_state.borrow_mut() = Some(setter);
                    CustomSingleChildLayout::new(Rc::new(
                        SpellCheckSuggestionsToolbarLayoutDelegate::new(Offset::new(
                            50.0,
                            anchor_y.get(),
                        )),
                    ))
                    .child(
                        Container::new()
                            .width(200.0)
                            .height(HEIGHT)
                            .color(Color::new(0xffff0000)),
                    )
                    .into_widget()
                })
            };
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new(),
                    Directionality::new(TextDirection::Ltr, builder),
                )
                .into_widget(),
            );

            let toolbar_y = |tester: &WidgetTester| {
                let container = tester.find_by_type::<Container>()[0];
                tester.global_rect(tester.render_box_of(container)).top
            };

            // When the toolbar doesn't fit below anchor, it positions itself such that
            // it can just fit.
            // Total height available is 600.
            assert_eq!(toolbar_y(&tester), TOOLBAR_OVERLAP);

            // When it does fit below anchor, it positions itself there.
            let setter = set_state.borrow().clone().expect("the builder built");
            setter(&mut tester.cell.borrow_mut(), &mut |_app| {
                anchor_y.set(anchor_y.get() - TOOLBAR_OVERLAP);
            });
            tester.pump(Duration::ZERO);
            assert_eq!(toolbar_y(&tester), anchor_y.get());
        });
    }
}
