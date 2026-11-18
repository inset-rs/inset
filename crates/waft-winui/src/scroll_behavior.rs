//! WinUI's scrolling: a thin overlay scroll bar in the theme's scrollbar colours, and no
//! overscroll glow or stretch — a `ScrollViewer` stops at the edge and shows no indicator there.

use std::rc::Rc;

use waft_embedder::Radius;
use waft_foundation::App;
use waft_widgets::{
    BuildContext, ClampingScrollPhysics, IntoWidget, RangeMaintainingScrollPhysics, RawScrollbar,
    ScrollBehavior, ScrollPhysicsRef, ScrollableDetails, WidgetRef,
};

use crate::theme::ThemeResources;

/// How a Fluent app's scrollables behave: WinUI's scroll bar and edge.
///
/// The scrollbar is [`RawScrollbar`] at the kit's two-point thickness in the theme's
/// `ScrollBarForeground`, shown while a scroll is under way and hidden after it, as WinUI's
/// auto-hiding bar is. The physics clamp at the edge on every system, with no glow or stretch,
/// as a `ScrollViewer` does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FluentScrollBehavior;

impl FluentScrollBehavior {
    /// WinUI's scrolling for the app.
    pub const fn new() -> FluentScrollBehavior {
        FluentScrollBehavior
    }

    /// The bar's thickness: WinUI's collapsed scrollbar is 2 effective pixels across.
    const THICKNESS: f64 = 2.0;
}

impl ScrollBehavior for FluentScrollBehavior {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn build_scrollbar(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let Some(controller) = details.controller.clone() else {
            return child;
        };
        RawScrollbar::new(child)
            .controller(controller)
            .thickness(Self::THICKNESS)
            .radius(Radius::circular(Self::THICKNESS / 2.0))
            .thumb_color(resources.scroll_bar_foreground)
            .into_widget()
    }

    fn build_overscroll_indicator(
        &self,
        _app: &mut App,
        _context: BuildContext,
        child: WidgetRef,
        _details: &ScrollableDetails,
    ) -> WidgetRef {
        child
    }

    fn get_scroll_physics(&self, _app: &mut App, _context: BuildContext) -> ScrollPhysicsRef {
        Rc::new(
            ClampingScrollPhysics::new().with_parent(Rc::new(RangeMaintainingScrollPhysics::new())),
        )
    }
}
