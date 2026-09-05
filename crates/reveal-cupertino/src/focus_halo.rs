//! Flutter `cupertino/cupertino_focus_halo.dart`.

use std::fmt::{self, Debug};

use reveal_foundation::{App, Handle};
use reveal_painting::{
    BorderRadius, BorderRadiusGeometry, BorderSide, BorderStyle, HSLColor, RoundedRectangleBorder,
    RoundedSuperellipseBorder, ShapeBorder, ShapeDecoration,
};
use reveal_rendering::DecorationPosition;
use reveal_widgets::{
    BuildContext, DecoratedBox, Focus, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    WidgetRef,
};

use crate::colors::CupertinoColors;
use crate::constants::{
    K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS, K_CUPERTINO_FOCUS_COLOR_OPACITY,
    K_CUPERTINO_FOCUS_COLOR_SATURATION,
};

/// Dart's `ShapeBorder Function({BorderRadiusGeometry borderRadius, BorderSide side})`.
type ShapeBuilder = fn(BorderRadiusGeometry, BorderSide) -> Box<dyn ShapeBorder>;

/// Applies an iOS-style focus border around its child when any of child focus nodes gain focus.
///
/// The shape of the focus halo does not automatically adapt to the child widget
/// it encloses. You are responsible for specifying a shape that correctly
/// matches the child's geometry by using the appropriate constructor, such as
/// [`CupertinoFocusHalo::with_rect`] or [`CupertinoFocusHalo::with_rrect`].
///
/// See also:
///
/// * <https://developer.apple.com/design/human-interface-guidelines/focus-and-selection/>
pub struct CupertinoFocusHalo {
    key: Option<KeyRef>,
    /// The child to draw the focused border around.
    ///
    /// Since [`CupertinoFocusHalo`] can't request focus to itself, this `child` should
    /// contain widget(s) that can request focus.
    ///
    /// The child widget is responsible for its own visual shape, for example by
    /// using an appropriate clipping.
    child: WidgetRef,
    border_radius: BorderRadiusGeometry,
    shape_builder: ShapeBuilder,
}

impl CupertinoFocusHalo {
    /// Creates a rectangular [`CupertinoFocusHalo`] around the child.
    pub fn with_rect<K>(child: impl IntoWidget<K>) -> CupertinoFocusHalo {
        CupertinoFocusHalo {
            key: None,
            child: child.into_widget(),
            border_radius: BorderRadius::ZERO.into(),
            shape_builder: rounded_rectangle_border,
        }
    }

    /// Creates a rounded rectangular [`CupertinoFocusHalo`] around the child.
    pub fn with_rrect<K>(
        border_radius: BorderRadiusGeometry,
        child: impl IntoWidget<K>,
    ) -> CupertinoFocusHalo {
        CupertinoFocusHalo {
            key: None,
            child: child.into_widget(),
            border_radius,
            shape_builder: rounded_rectangle_border,
        }
    }

    /// Creates a rounded superellipse-shaped [`CupertinoFocusHalo`] around the child.
    ///
    /// See also:
    ///
    /// * `RSuperellipse` and [`RoundedSuperellipseBorder`] for more introduction on
    ///   the rounded superellipse shape.
    pub fn with_rounded_superellipse<K>(
        border_radius: BorderRadiusGeometry,
        child: impl IntoWidget<K>,
    ) -> CupertinoFocusHalo {
        CupertinoFocusHalo {
            key: None,
            child: child.into_widget(),
            border_radius,
            shape_builder: rounded_superellipse_border,
        }
    }

    /// Dart `CupertinoFocusHalo.withRect(key:)` and the other constructors' `key`.
    pub fn key(mut self, key: KeyRef) -> CupertinoFocusHalo {
        self.key = Some(key);
        self
    }
}

impl Debug for CupertinoFocusHalo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoFocusHalo")
            .field("key", &self.key)
            .field("border_radius", &self.border_radius)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

/// Dart's `RoundedRectangleBorder.new` as a shape builder.
fn rounded_rectangle_border(
    border_radius: BorderRadiusGeometry,
    side: BorderSide,
) -> Box<dyn ShapeBorder> {
    Box::new(RoundedRectangleBorder::new(side, border_radius))
}

/// Dart's `RoundedSuperellipseBorder.new` as a shape builder.
fn rounded_superellipse_border(
    border_radius: BorderRadiusGeometry,
    side: BorderSide,
) -> Box<dyn ShapeBorder> {
    Box::new(RoundedSuperellipseBorder::new(side, Some(border_radius)))
}

impl StatefulWidget for CupertinoFocusHalo {
    type State = CupertinoFocusHaloState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoFocusHaloState {
        CupertinoFocusHaloState {
            state: StateData::new(),
            child_has_focus: false,
        }
    }
}

/// Dart's `_CupertinoFocusHaloState`.
pub struct CupertinoFocusHaloState {
    state: StateData<CupertinoFocusHalo>,
    child_has_focus: bool,
}

impl CupertinoFocusHaloState {
    fn effective_focus_outline_color() -> reveal_embedder::Color {
        HSLColor::from_color(CupertinoColors::ACTIVE_BLUE.with_values(
            Some(K_CUPERTINO_FOCUS_COLOR_OPACITY),
            None,
            None,
            None,
            None,
        ))
        .with_lightness(K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS)
        .with_saturation(K_CUPERTINO_FOCUS_COLOR_SATURATION)
        .to_color()
    }
}

impl State for CupertinoFocusHaloState {
    type Widget = CupertinoFocusHalo;
    reveal_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let side = if app.get(self).child_has_focus {
            BorderSide::new(
                Self::effective_focus_outline_color(),
                3.5,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            )
        } else {
            BorderSide::NONE
        };
        let shape = (widget.shape_builder)(widget.border_radius, side);
        Focus::new(DecoratedBox {
            key: None,
            decoration: Box::new(ShapeDecoration::new(shape)),
            position: DecorationPosition::Foreground,
            child: Some(widget.child.clone()),
        })
        .can_request_focus(false)
        .skip_traversal(true)
        .include_semantics(false)
        .on_focus_change(move |app, has_focus| {
            self.set_state(app, |state| state.child_has_focus = has_focus);
        })
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use reveal_embedder::TextDirection;
    use reveal_rendering::{AnyRenderObject, RenderDecoratedBox, RenderHandle, RenderObject};
    use reveal_widgets::{
        Directionality, FocusNode, FocusNodeLeaf, GlobalKey, SizedBox, UnfocusDisposition,
    };

    use super::*;
    use crate::test_support::{build, pump, test_cell};

    fn find<T: RenderObject>(app: &App, node: AnyRenderObject) -> Option<RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find::<T>(app, child);
            }
        });
        found
    }

    fn halo_side(app: &mut App, key: &GlobalKey) -> BorderSide {
        let root = key
            .current_context(app)
            .expect("the halo mounted")
            .find_render_object(app)
            .expect("the halo has been laid out");
        let decorated = find::<RenderDecoratedBox>(app, root).expect("the foreground decoration");
        decorated
            .decoration(app)
            .as_any()
            .downcast_ref::<ShapeDecoration>()
            .expect("a ShapeDecoration")
            .shape
            .as_any()
            .downcast_ref::<RoundedSuperellipseBorder>()
            .expect("the superellipse shape")
            .side
    }

    #[test]
    fn a_halo_outlines_its_shape_while_a_focus_node_inside_it_has_focus() {
        let cell = test_cell();
        let mut app = cell.borrow_mut();
        let key = Rc::new(GlobalKey::new());
        let node = FocusNode::new(&mut app).as_node();
        drop(app);
        build(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                CupertinoFocusHalo::with_rounded_superellipse(
                    BorderRadius::circular(10.0).into(),
                    Focus::new(SizedBox::square(Some(40.0))).focus_node(node),
                )
                .key(key.clone()),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(halo_side(&mut app, &key), BorderSide::NONE);

        node.request_focus(&mut app, None);
        pump(&mut app, std::time::Duration::ZERO);
        let side = halo_side(&mut app, &key);
        assert_eq!(side.width, 3.5);
        assert_eq!(
            side.color,
            CupertinoFocusHaloState::effective_focus_outline_color()
        );

        node.unfocus(&mut app, UnfocusDisposition::Scope);
        pump(&mut app, std::time::Duration::ZERO);
        assert_eq!(halo_side(&mut app, &key), BorderSide::NONE);
    }
}
