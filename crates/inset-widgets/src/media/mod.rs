//! Players and cameras: the host's media stack (`inset_embedder`'s `media`) as an app holds it.
//! Not in Flutter's framework: Flutter's `video_player` and `camera` plugins have controllers to
//! initialize and values copied over their method channels, where the systems share a simpler
//! model, which these follow. A [`Player`] is made from a source and loads by itself; a
//! [`CameraSession`] is a camera, listed by [`available_cameras`] and listed again once
//! [`available_cameras_changed`] resolves, then opened ([`open_camera`]), running in one of its
//! formats with its outputs, each the system's own, given whole as it starts. Each is a
//! listenable over the host's object, whose listeners hear its state change, and each is shown by
//! a widget that takes it, [`PlayerView`] and [`CameraPreview`], as Media3's `PlayerView` and
//! `AVPlayerLayer(player:)` take a player and CameraX's `PreviewView` a camera. An app that draws
//! the frames itself makes a reader from the platform's `video_frame_readers`, gives it to the
//! player or among the session's outputs, and draws it with a `VideoFrameView`.
//!
//! A view shows its picture through a composition layer (`CompositionLayerView`): the framework
//! leaves its rectangle clear, the frames of the view it is in place the layer there, and the
//! system draws the picture into a layer that is the composition layer's content, which no frame
//! of the framework's touches. A `PlayerView` makes a layer of its own and puts its player's
//! picture there as it is mounted, and nowhere as it is unmounted; a `CameraPreview` places the
//! layer its session started with, since a camera's outputs are given as it starts. The
//! invariant: a player shows on the layer of the view built last for it, a view puts nowhere only
//! the picture still on its own layer, and a view laid out before its picture's size is known
//! takes no room.
//!
//! The system draws no subtitles: a player hands its cues to the app, as the media styles them,
//! and a [`SubtitleView`], Media3's, draws them over the picture's rectangle with the user's
//! caption style merged in, as `PlayerView` puts one over its video.
//!
//! A player's position is read, not told ([`Player::position`]): its listeners hear it jump, and
//! a widget that shows it while it plays reads it at each frame, ticking with a `Ticker` from its
//! state's ticker provider while the player's state says it plays, as a widget ticks through an
//! animation.

mod camera;
mod cue_style;
mod player;
mod subtitle_painter;
mod subtitle_view;

use inset_embedder::Size;
use inset_rendering::BoxConstraints;

use crate::framework::{IntoWidget, WidgetRef};
use crate::widgets::basic::{AspectRatio, ConstrainedBox, SizedBox};

pub use camera::{
    CameraPreview, CameraSession, available_cameras, available_cameras_changed, open_camera,
};
pub use player::{Player, PlayerView};
pub use subtitle_view::SubtitleView;

/// `child` at `size`, or as large as the constraints let it be at that aspect.
fn at_size(size: Size, child: WidgetRef) -> WidgetRef {
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return SizedBox::shrink().into_widget();
    }
    ConstrainedBox::new(BoxConstraints::loose(size))
        .child(AspectRatio::new(size.width() / size.height()).child(child))
        .into_widget()
}
