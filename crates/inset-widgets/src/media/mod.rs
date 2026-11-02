//! Players and cameras: the host's media stack (`inset_embedder`'s `media`) as an app holds it.
//! Not in Flutter's framework: Flutter's `video_player` and `camera` plugins have controllers to
//! initialize and values copied over their method channels, where the systems share a simpler
//! model, which these follow. A [`Player`] is made from a source and loads by itself; a
//! [`CameraSession`] is a session on one camera with outputs, each the system's own. Each is a
//! listenable over the host's object, whose listeners hear its state change, and each is shown by
//! a widget that takes it, [`PlayerView`] and [`CameraPreview`], as Media3's `PlayerView` and
//! `AVPlayerLayer(player:)` take a player and CameraX's `PreviewView` a camera. An app that draws
//! the frames itself makes a reader from the platform's `video_frame_readers`, gives it to the
//! player or the session, and draws it with a `VideoFrameView`.
//!
//! A view shows its picture through a composition layer of its own (`CompositionLayerView`): the
//! framework leaves its rectangle clear, the frames of the view it is in place the layer there,
//! and the system draws the picture into a layer that is the composition layer's content, which no
//! frame of the framework's touches. A view puts its player's or session's picture on its layer as
//! it is mounted, and nowhere as it is unmounted. The invariant: a player or a session shows on
//! the layer of the view built last for it, a view puts nowhere only the picture still on its own
//! layer, and a view laid out before its picture's size is known takes no room.

mod camera;
mod player;

use std::sync::Arc;

use inset_embedder::{CompositionLayer, Size};
use inset_foundation::{App, Handle, ListenableObject};
use inset_rendering::BoxConstraints;

use crate::framework::{BuildContext, IntoWidget, State, StateData, StatefulWidget, WidgetRef};
use crate::widgets::basic::{AspectRatio, ConstrainedBox, SizedBox};
use crate::widgets::composition_layer_view::CompositionLayerView;

pub use camera::{CameraPreview, CameraSession, available_cameras};
pub use player::{Player, PlayerView};

/// What puts its picture on a composition layer it is given: a player or a camera session.
trait PictureSource: ListenableObject {
    /// Puts the picture on `layer` from now on; `None` puts it nowhere.
    fn set_layer(handle: Handle<Self>, app: &mut App, layer: Option<Arc<dyn CompositionLayer>>);

    /// The layer the picture was put on last; `None` once put nowhere, or disposed.
    fn layer(handle: Handle<Self>, app: &App) -> Option<&Arc<dyn CompositionLayer>>;

    /// The picture's size, in its own pixels, once known.
    fn picture_size(handle: Handle<Self>, app: &App) -> Option<Size>;
}

/// `source`'s picture on a composition layer of the widget's own, laid out at the picture's size
/// in logical pixels, as large as the constraints let it be at that aspect.
struct CompositionLayerPicture<S: PictureSource> {
    source: Handle<S>,
}

impl<S: PictureSource> std::fmt::Debug for CompositionLayerPicture<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositionLayerPicture")
            .finish_non_exhaustive()
    }
}

impl<S: PictureSource> StatefulWidget for CompositionLayerPicture<S> {
    type State = CompositionLayerPictureState<S>;

    fn create_state(&self) -> CompositionLayerPictureState<S> {
        CompositionLayerPictureState {
            state: StateData::new(),
            layer: None,
        }
    }
}

struct CompositionLayerPictureState<S: PictureSource> {
    state: StateData<CompositionLayerPicture<S>>,
    /// The layer the source shows on, made as the widget is mounted; `None` where the host has
    /// no compositor.
    layer: Option<Arc<dyn CompositionLayer>>,
}

impl<S: PictureSource> CompositionLayerPictureState<S> {
    fn picture_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    /// Puts `source`'s picture on the widget's layer.
    fn show(self: Handle<Self>, app: &mut App, source: Handle<S>) {
        if let Some(layer) = app.get(self).layer.clone() {
            S::set_layer(source, app, Some(layer));
        }
    }

    /// Puts `source`'s picture nowhere, if it is still on this widget's layer: a later view may
    /// have taken the source since.
    fn release(self: Handle<Self>, app: &mut App, source: Handle<S>) {
        let Some(ours) = &app.get(self).layer else {
            return;
        };
        let still_ours = S::layer(source, app).is_some_and(|shown| Arc::ptr_eq(shown, ours));
        if still_ours {
            S::set_layer(source, app, None);
        }
    }
}

impl<S: PictureSource> State for CompositionLayerPictureState<S> {
    type Widget = CompositionLayerPicture<S>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let source = self.widget(app).source;
        source.add_listener(
            app,
            inset_foundation::Listener::handle_method(self, Self::picture_changed),
        );
        let layer = app
            .platform()
            .compositor()
            .map(|compositor| compositor.make_layer(false));
        app.get_mut(self).layer = layer;
        self.show(app, source);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &CompositionLayerPicture<S>,
    ) {
        let source = self.widget(app).source;
        if source == old_widget.source {
            return;
        }
        let listener = inset_foundation::Listener::handle_method(self, Self::picture_changed);
        old_widget.source.remove_listener(app, &listener);
        source.add_listener(app, listener);
        self.release(app, old_widget.source);
        self.show(app, source);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let source = self.widget(app).source;
        self.release(app, source);
        source.remove_listener(
            app,
            &inset_foundation::Listener::handle_method(self, Self::picture_changed),
        );
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let source = self.widget(app).source;
        let size = S::picture_size(source, app);
        let layer = app.get(self).layer.clone();
        match (size, layer) {
            (Some(size), Some(layer)) => {
                at_size(size, CompositionLayerView::new(layer).into_widget())
            }
            (Some(size), None) => at_size(size, SizedBox::expand().into_widget()),
            (None, _) => SizedBox::shrink().into_widget(),
        }
    }
}

/// `child` at `size`, or as large as the constraints let it be at that aspect.
fn at_size(size: Size, child: WidgetRef) -> WidgetRef {
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return SizedBox::shrink().into_widget();
    }
    ConstrainedBox::new(BoxConstraints::loose(size))
        .child(AspectRatio::new(size.width() / size.height()).child(child))
        .into_widget()
}
