//! Flutter counterpart: `rendering/object.dart` (`RelayoutWhenSystemFontsChangeMixin`).

use std::time::Duration;

use inset_foundation::{App, Handle, Listenable, Listener};
use inset_painting::PaintingBinding;
use inset_scheduler::{FrameCallback, SchedulerBinding};

use crate::object::{RenderHandle, RenderObject};

/// Flutter's `RelayoutWhenSystemFontsChangeMixin` fields.
#[derive(Debug, Default)]
pub struct RelayoutWhenSystemFontsChangeData {
    has_pending_system_fonts_did_change_callback: bool,
}

impl RelayoutWhenSystemFontsChangeData {
    /// No pending system-fonts callback.
    pub const fn new() -> RelayoutWhenSystemFontsChangeData {
        RelayoutWhenSystemFontsChangeData {
            has_pending_system_fonts_did_change_callback: false,
        }
    }
}

/// Mixin for a [`RenderObject`] that relayouts when system fonts change.
///
/// System fonts can change when the OS installs or removes a font. Use this mixin if the
/// render object uses `TextPainter` or `Paragraph` to correctly update the text when it
/// happens.
pub trait RelayoutWhenSystemFontsChangeMixin: RenderObject {
    /// Mixin field access.
    fn relayout_when_system_fonts_change_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RelayoutWhenSystemFontsChangeData;

    /// See [`relayout_when_system_fonts_change_data`](Self::relayout_when_system_fonts_change_data).
    fn relayout_when_system_fonts_change_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RelayoutWhenSystemFontsChangeData;

    /// A callback that is called when system fonts have changed.
    ///
    /// The framework defers the invocation of the callback to the
    /// `SchedulerPhase::TransientCallbacks` phase to ensure that the render object's text
    /// layout is still valid when user interactions are in progress (which usually take
    /// place during the idle phase).
    ///
    /// By default, `markNeedsLayout` is called on the render object implementing this mixin.
    ///
    /// Subclass should override this method to clear any extra cache that depend on
    /// font-related metrics.
    fn system_fonts_did_change(self: RenderHandle<Self>, app: &mut App) {
        self.as_render_object(app).mark_needs_layout(app);
    }

    /// Flutter's `attach` override body after `super.attach`.
    fn attach_system_fonts(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(
            !self
                .relayout_when_system_fonts_change_data(app)
                .has_pending_system_fonts_did_change_callback
        );
        PaintingBinding::instance(app)
            .system_fonts(app)
            .add_listener(app, self.system_fonts_listener());
    }

    /// Flutter's `detach` override body before `super.detach`.
    fn detach_system_fonts(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(
            !self
                .relayout_when_system_fonts_change_data(app)
                .has_pending_system_fonts_did_change_callback
        );
        PaintingBinding::instance(app)
            .system_fonts(app)
            .remove_listener(app, &self.system_fonts_listener());
    }

    fn system_fonts_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), schedule_system_fonts_update::<Self>)
    }

    fn schedule_system_fonts_update(self: RenderHandle<Self>, app: &mut App) {
        if self
            .relayout_when_system_fonts_change_data(app)
            .has_pending_system_fonts_did_change_callback
        {
            return;
        }
        self.relayout_when_system_fonts_change_data_mut(app)
            .has_pending_system_fonts_did_change_callback = true;
        SchedulerBinding::schedule_frame_callback(
            app,
            FrameCallback::handle_method(self.handle(), run_system_fonts_update::<Self>),
            false,
            true,
        );
    }
}

fn schedule_system_fonts_update<T: RelayoutWhenSystemFontsChangeMixin>(
    this: Handle<T>,
    app: &mut App,
) {
    RenderHandle::from_handle(this).schedule_system_fonts_update(app);
}

fn run_system_fonts_update<T: RelayoutWhenSystemFontsChangeMixin>(
    this: Handle<T>,
    app: &mut App,
    _time_stamp: Duration,
) {
    let this = RenderHandle::from_handle(this);
    if app.is_disposed(this.handle()) {
        return;
    }
    debug_assert!(
        this.relayout_when_system_fonts_change_data(app)
            .has_pending_system_fonts_did_change_callback
    );
    this.relayout_when_system_fonts_change_data_mut(app)
        .has_pending_system_fonts_did_change_callback = false;
    if this.as_render_object(app).attached(app) {
        RelayoutWhenSystemFontsChangeMixin::system_fonts_did_change(this, app);
    }
}
