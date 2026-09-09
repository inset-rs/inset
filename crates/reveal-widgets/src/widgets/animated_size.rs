//! Flutter counterpart: `widgets/animated_size.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{Curve, Curves};
use reveal_embedder::Clip;
use reveal_foundation::{App, Handle, Listener};
use reveal_painting::AlignmentGeometry;
use reveal_rendering::{
    AnyRenderObject, RenderAligningShiftedBox, RenderAnimatedSize, RenderBox, RenderHandle,
    TickerProviderRef,
};
use reveal_scheduler::{Ticker, TickerCallback, TickerProviderObject};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, State,
    StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::Directionality;
use crate::widgets::ticker_provider::{
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
};

/// Animated widget that automatically transitions its size over a given
/// duration whenever the given child's size changes.
pub struct AnimatedSize {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
    /// The alignment of the child within the parent when the parent is not yet
    /// the same size as the child.
    pub alignment: AlignmentGeometry,
    /// The animation curve when transitioning this widget's size to match the
    /// child's size.
    pub curve: Rc<dyn Curve>,
    /// The duration when transitioning this widget's size to match the child's
    /// size.
    pub duration: Duration,
    /// The duration when transitioning this widget's size to match the child's
    /// size when going in reverse.
    pub reverse_duration: Option<Duration>,
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    /// Called every time an animation completes.
    pub on_end: Option<Listener>,
}

impl Debug for AnimatedSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedSize").finish_non_exhaustive()
    }
}

impl AnimatedSize {
    /// Creates a widget that animates its size to match that of its child.
    pub fn new(duration: Duration) -> AnimatedSize {
        AnimatedSize {
            key: None,
            child: None,
            alignment: AlignmentGeometry::CENTER,
            curve: Curves::linear(),
            duration,
            reverse_duration: None,
            clip_behavior: Clip::HardEdge,
            on_end: None,
        }
    }

    /// Dart `AnimatedSize(key:)`.
    pub fn key(mut self, key: KeyRef) -> AnimatedSize {
        self.key = Some(key);
        self
    }

    /// Dart `AnimatedSize(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedSize {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `AnimatedSize(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> AnimatedSize {
        self.alignment = alignment;
        self
    }

    /// Dart `AnimatedSize(curve:)`.
    pub fn curve(mut self, curve: Rc<dyn Curve>) -> AnimatedSize {
        self.curve = curve;
        self
    }

    /// Dart `AnimatedSize(reverseDuration:)`.
    pub fn reverse_duration(mut self, reverse_duration: Duration) -> AnimatedSize {
        self.reverse_duration = Some(reverse_duration);
        self
    }

    /// Dart `AnimatedSize(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> AnimatedSize {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `AnimatedSize(onEnd:)`.
    pub fn on_end(mut self, on_end: Listener) -> AnimatedSize {
        self.on_end = Some(on_end);
        self
    }
}

impl StatefulWidget for AnimatedSize {
    type State = AnimatedSizeState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> AnimatedSizeState {
        AnimatedSizeState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
        }
    }
}

/// Dart's `_AnimatedSizeState`.
pub struct AnimatedSizeState {
    state: StateData<AnimatedSize>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
}

impl Debug for AnimatedSizeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedSizeState").finish_non_exhaustive()
    }
}

impl SingleTickerProviderStateMixin for AnimatedSizeState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedSizeState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for AnimatedSizeState {
    type Widget = AnimatedSize;
    crate::state_accessors!();

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        AnimatedSizeRender {
            key: None,
            alignment: widget.alignment,
            curve: widget.curve.clone(),
            duration: widget.duration,
            reverse_duration: widget.reverse_duration,
            vsync: TickerProviderRef::new(self),
            clip_behavior: widget.clip_behavior,
            on_end: widget.on_end.clone(),
            child: widget.child.clone(),
        }
        .into_widget()
    }
}

/// Dart's `_AnimatedSize`.
struct AnimatedSizeRender {
    key: Option<KeyRef>,
    alignment: AlignmentGeometry,
    curve: Rc<dyn Curve>,
    duration: Duration,
    reverse_duration: Option<Duration>,
    vsync: TickerProviderRef,
    clip_behavior: Clip,
    on_end: Option<Listener>,
    child: Option<WidgetRef>,
}

impl Debug for AnimatedSizeRender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_AnimatedSize").finish_non_exhaustive()
    }
}

impl RenderObjectWidget for AnimatedSizeRender {
    type RenderObject = RenderAnimatedSize;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        RenderAnimatedSize::new(
            app,
            self.vsync.clone(),
            self.duration,
            self.reverse_duration,
            self.curve.clone(),
            self.alignment,
            text_direction,
            self.clip_behavior,
            self.on_end.clone(),
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderAnimatedSize>,
    ) {
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_alignment(app, self.alignment);
        render_object.set_duration(app, self.duration);
        render_object.set_reverse_duration(app, self.reverse_duration);
        render_object.set_curve(app, self.curve.clone());
        render_object.set_vsync(app, self.vsync.clone());
        render_object.set_text_direction(app, text_direction);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_on_end(app, self.on_end.clone());
    }
}

impl SingleChildRenderObjectWidget for AnimatedSizeRender {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}
