//! Players and cameras: the host's media stack (`inset_embedder`'s `media`) as an app holds it.
//! Not in Flutter's framework: Flutter's `video_player` and `camera` plugins have controllers to
//! initialize and values copied over their method channels, where the systems share a simpler
//! model, which these follow. A [`Player`] is made from a source and loads by itself; a
//! [`CameraSession`] is a session on one camera with outputs, each the system's own. Each is a
//! listenable over the host's object, whose listeners hear its state change, and each is shown by
//! a widget that takes it, [`VideoView`] and [`CameraPreview`], as `AVPlayerLayer(player:)` and
//! `PlayerView` take a player.
//!
//! A view shows its picture through a native surface under the root of the view it is in
//! (`SurfaceView`): the framework leaves its rectangle clear, and the system draws the picture into
//! the surface's layer, which no frame of the framework's touches. A view puts its player's or
//! session's picture on its surface as it is mounted, and nowhere as it is unmounted. The
//! invariant: a player or a session shows on the surface of the view built last for it, a view
//! puts nowhere only the picture still on its own surface, and a view laid out before its
//! picture's size is known takes no room.

mod camera;
mod player;

use std::sync::Arc;

use inset_embedder::{Size, Surface, ViewId};
use inset_foundation::{App, Handle, ListenableObject};
use inset_rendering::BoxConstraints;

use crate::framework::{BuildContext, IntoWidget, State, StateData, StatefulWidget, WidgetRef};
use crate::view::View;
use crate::widgets::basic::{AspectRatio, ConstrainedBox, SizedBox};
use crate::widgets::surface_view::SurfaceView;

pub use camera::{CameraPreview, CameraSession, available_cameras};
pub use player::{Player, VideoView};

/// What puts its picture on a surface it is given: a player or a camera session.
trait PictureSource: ListenableObject {
    /// Puts the picture on `surface` from now on; `None` puts it nowhere.
    fn set_surface(handle: Handle<Self>, app: &mut App, surface: Option<Arc<dyn Surface>>);

    /// The surface the picture was put on last; `None` once put nowhere, or disposed.
    fn surface(handle: Handle<Self>, app: &App) -> Option<&Arc<dyn Surface>>;

    /// The picture's size, in its own pixels, once known.
    fn picture_size(handle: Handle<Self>, app: &App) -> Option<Size>;
}

/// `source`'s picture on a surface under the root of the view the widget is in, laid out at the
/// picture's size in logical pixels, as large as the constraints let it be at that aspect.
struct SurfacePicture<S: PictureSource> {
    source: Handle<S>,
}

impl<S: PictureSource> std::fmt::Debug for SurfacePicture<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurfacePicture").finish_non_exhaustive()
    }
}

impl<S: PictureSource> StatefulWidget for SurfacePicture<S> {
    type State = SurfacePictureState<S>;

    fn create_state(&self) -> SurfacePictureState<S> {
        SurfacePictureState {
            state: StateData::new(),
            surface: None,
        }
    }
}

struct SurfacePictureState<S: PictureSource> {
    state: StateData<SurfacePicture<S>>,
    /// The surface the source shows on, and the view it was opened for; `None` outside a view, or
    /// where the host has no compositor.
    surface: Option<(ViewId, Arc<dyn Surface>)>,
}

impl<S: PictureSource> SurfacePictureState<S> {
    fn picture_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    /// Puts the source's picture on a surface under the root of the view the widget is in now,
    /// opening one if the view changed or the source did.
    fn show_in_current_view(self: Handle<Self>, app: &mut App, source_changed: bool) {
        let context = self.context(app);
        let view = View::maybe_of(app, context).map(|view| view.id());
        let current = app.get(self).surface.as_ref().map(|(view, _)| *view);
        if view == current && !source_changed {
            return;
        }
        let source = self.widget(app).source;
        let surface = view.and_then(|view| open_surface(app, view));
        match &surface {
            Some((_, surface)) => S::set_surface(source, app, Some(Arc::clone(surface))),
            None => self.release(app, source),
        }
        app.get_mut(self).surface = surface;
    }

    /// Puts `source`'s picture nowhere, if it is still on this widget's surface: a later view
    /// may have taken the source since.
    fn release(self: Handle<Self>, app: &mut App, source: Handle<S>) {
        let Some((_, ours)) = &app.get(self).surface else {
            return;
        };
        let still_ours = S::surface(source, app).is_some_and(|shown| Arc::ptr_eq(shown, ours));
        if still_ours {
            S::set_surface(source, app, None);
        }
    }
}

impl<S: PictureSource> State for SurfacePictureState<S> {
    type Widget = SurfacePicture<S>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let source = self.widget(app).source;
        source.add_listener(
            app,
            inset_foundation::Listener::handle_method(self, Self::picture_changed),
        );
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.show_in_current_view(app, false);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SurfacePicture<S>) {
        let source = self.widget(app).source;
        if source == old_widget.source {
            return;
        }
        let listener = inset_foundation::Listener::handle_method(self, Self::picture_changed);
        old_widget.source.remove_listener(app, &listener);
        source.add_listener(app, listener);
        self.release(app, old_widget.source);
        self.show_in_current_view(app, true);
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
        let surface = app
            .get(self)
            .surface
            .as_ref()
            .map(|(_, surface)| Arc::clone(surface));
        match (size, surface) {
            (Some(size), Some(surface)) => at_size(size, SurfaceView::new(surface).into_widget()),
            (Some(size), None) => at_size(size, SizedBox::expand().into_widget()),
            (None, _) => SizedBox::shrink().into_widget(),
        }
    }
}

/// A surface under `view`'s root for a system to draw a picture into; `None` where the host has
/// no compositor.
fn open_surface(app: &App, view: ViewId) -> Option<(ViewId, Arc<dyn Surface>)> {
    let compositor = app.platform().compositor()?;
    Some((view, compositor.root(view).open_child(false)))
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
