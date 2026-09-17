//! Flutter counterpart: `packages/flutter/lib/src/widgets`.
//!
//! The widget framework: `framework.dart` (widgets, elements, state, the build owner).
//! The binding, `runApp`, and the widgets themselves follow.
#![feature(arbitrary_self_types)]

mod binding;
mod framework;
#[cfg(test)]
mod test_harness;
mod view;
mod widgets;
mod window;

pub use binding::*;
pub use framework::*;
// Flutter's `basic.dart` re-exports the animation and painting libraries whole, and these
// from `rendering.dart`, so a widget author never names those layers for a `Row`, a curve
// or a border. The animation mixins stay behind: they are Rust's spelling of Dart's, and
// beside `Animation` they would make its methods ambiguous.
pub use inset_animation::{
    AlwaysStoppedAnimation, Animatable, Animation, AnimationBehavior, AnimationController,
    AnimationMax, AnimationMean, AnimationMin, AnimationStatus, AnimationStatusListener,
    AnimationStyle, AnimationWithParent, AnyAnimation, CallbackAnimatable, ChainedEvaluation,
    ColorTween, CompoundAnimation, ConstantTween, Cubic, Curve, CurveTween, CurvedAnimation,
    Curves, ElasticInCurve, ElasticInOutCurve, ElasticOutCurve, FlippedCurve, FlippedTweenSequence,
    IntTween, Interval, ProxyAnimation, RectTween, ReverseAnimation, ReverseTween, SawTooth,
    SizeTween, Split, StepTween, ThreePointCubic, Threshold, TrainHoppingAnimation, Tween,
    TweenLerp, TweenSequence, TweenSequenceItem, k_always_complete_animation,
    k_always_dismissed_animation,
};
pub use inset_painting::*;
pub use inset_rendering::{
    BoxConstraints, CrossAxisAlignment, FlexFit, MainAxisAlignment, MainAxisSize, StackFit,
};
// The widget, not dart:ui's `Image` that painting carries: Flutter's `widgets.dart` makes
// the same choice, leaving the other as `ui.Image`.
pub use widgets::image::Image;

/// Paths the crate's macros name. Not an API.
#[doc(hidden)]
pub mod __private {
    pub use inset_foundation as foundation;
}
pub use view::*;
pub use widgets::actions::*;
pub use widgets::animated_size::*;
pub use widgets::annotated_region::*;
pub use widgets::app::*;
pub use widgets::app_lifecycle_listener::*;
pub use widgets::drop_listener::*;
pub use widgets::autofill::*;
pub use widgets::automatic_keep_alive::*;
pub use widgets::banner::*;
pub use widgets::basic::*;
pub use widgets::container::*;
pub use widgets::context_menu_button_item::*;
pub use widgets::default_text_editing_shortcuts::*;
pub use widgets::desktop_text_selection_toolbar_layout_delegate::*;
pub use widgets::drag_target::*;
pub use widgets::draggable_scrollable_sheet::*;
pub use widgets::editable_text::*;
pub use widgets::expansible::*;
pub use widgets::feedback::*;
pub use widgets::focus_manager::*;
pub use widgets::focus_scope::*;
pub use widgets::focus_traversal::*;
pub use widgets::gesture_detector::*;
pub use widgets::heroes::*;
pub use widgets::icon::*;
pub use widgets::icon_data::*;
pub use widgets::icon_theme::*;
pub use widgets::icon_theme_data::*;
pub use widgets::image::*;
pub use widgets::implicit_animations::*;
pub use widgets::inherited_notifier::*;
pub use widgets::layout_builder::*;
pub use widgets::localizations::*;
pub use widgets::magnifier::*;
pub use widgets::media_query::*;
pub use widgets::modal_barrier::*;
pub use widgets::navigation_toolbar::*;
pub use widgets::navigator::*;
pub use widgets::navigator_pop_handler::*;
pub use widgets::notification_listener::*;
pub use widgets::overlay::*;
pub use widgets::page_storage::*;
pub use widgets::pages::*;
pub use widgets::pop_scope::*;
pub use widgets::preferred_size::*;
pub use widgets::primary_scroll_controller::*;
pub use widgets::radio_group::*;
pub use widgets::restoration::*;
pub use widgets::restoration_properties::*;
pub use widgets::routes::*;
pub use widgets::safe_area::*;
pub use widgets::scroll_activity::*;
pub use widgets::scroll_configuration::*;
pub use widgets::scroll_context::*;
pub use widgets::scroll_controller::*;
pub use widgets::scroll_delegate::*;
pub use widgets::scroll_metrics::*;
pub use widgets::scroll_notification::*;
pub use widgets::scroll_notification_observer::*;
pub use widgets::scroll_physics::*;
pub use widgets::scroll_position::*;
pub use widgets::scroll_position_with_single_context::*;
pub use widgets::scroll_simulation::*;
pub use widgets::scroll_view::*;
pub use widgets::scrollable::*;
pub use widgets::scrollable_helpers::*;
pub use widgets::scrollbar::*;
pub use widgets::shared_app_data::*;
pub use widgets::shortcuts::*;
pub use widgets::single_child_scroll_view::*;
pub use widgets::sliver::*;
pub use widgets::sliver_persistent_header::*;
pub use widgets::spell_check::*;
pub use widgets::standard_component_type::*;
pub use widgets::status_transitions::*;
pub use widgets::system_context_menu::*;
pub use widgets::tap_region::*;
pub use widgets::text::*;
pub use widgets::text_editing_intents::*;
pub use widgets::text_selection::*;
pub use widgets::text_selection_toolbar_anchors::*;
pub use widgets::text_selection_toolbar_layout_delegate::*;
pub use widgets::ticker_provider::*;
pub use widgets::title::*;
pub use widgets::transitions::*;
pub use widgets::undo_history::*;
pub use widgets::value_listenable_builder::*;
pub use widgets::viewport::*;
pub use widgets::visibility::*;
pub use widgets::widget_state::*;
pub use window::*;
