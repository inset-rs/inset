//! A player, as an app holds it, and the view that shows its video.

use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{
    CompositionLayer, MediaChangeFuture, MediaPlayer, PlayerState, Size, VideoFrameReader,
};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle};

use super::{CompositionLayerPicture, PictureSource};
use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};

/// A player of the media at one URL, which loads by itself: the host's player, whose listeners
/// hear its state change. A URL the system cannot play gives a player whose state is a failure.
pub struct Player {
    change_notifier: ChangeNotifierData,
    /// The host's player, until [`dispose`](Player::dispose).
    host: Option<Rc<dyn MediaPlayer>>,
    /// The host's state when it last changed.
    state: PlayerState,
    /// The layer the video was put on last, which a [`PlayerView`] puts it nowhere from only
    /// while it is still its own.
    video_layer: Option<Arc<dyn CompositionLayer>>,
}

impl Player {
    /// A player of the media at `url`, which starts loading at once: a link, or a picked file's
    /// `PickedFile::url`. `None` where the platform has no players.
    pub fn new(app: &mut App, url: &str) -> Option<Handle<Player>> {
        let platform = app.platform();
        let host: Rc<dyn MediaPlayer> = Rc::from(platform.players()?.open(url));
        let changed = host.changed();
        let player = app.create(Player {
            change_notifier: ChangeNotifierData::new(),
            state: host.state(),
            host: Some(host),
            video_layer: None,
        });
        player.follow(app, changed);
        Some(player)
    }

    /// The player's state when it last changed.
    pub fn state(self: Handle<Self>, app: &App) -> &PlayerState {
        &app.get(self).state
    }

    pub fn play(self: Handle<Self>, app: &App) {
        self.host(app).play();
    }

    pub fn pause(self: Handle<Self>, app: &App) {
        self.host(app).pause();
    }

    /// Moves to `position`, playing on from there if it was playing.
    pub fn seek_to(self: Handle<Self>, app: &App, position: Duration) {
        self.host(app).seek(position);
    }

    /// How fast it plays, `1.0` the media's own speed.
    pub fn set_rate(self: Handle<Self>, app: &App, rate: f64) {
        self.host(app).set_rate(rate);
    }

    /// How loud it plays, from `0.0` to `1.0`.
    pub fn set_volume(self: Handle<Self>, app: &App, volume: f64) {
        self.host(app).set_volume(volume);
    }

    /// Whether it starts again from the beginning when it reaches the end.
    pub fn set_looping(self: Handle<Self>, app: &App, looping: bool) {
        self.host(app).set_looping(looping);
    }

    /// Puts the video on `layer` from now on, and on no layer or frame reader it went to before;
    /// `None` puts it nowhere: Media3's `setVideoSurface`, which [`PlayerView`] calls with a
    /// composition layer of its own as it is mounted, and with `None` as it is unmounted.
    pub fn set_video_layer(
        self: Handle<Self>,
        app: &mut App,
        layer: Option<Arc<dyn CompositionLayer>>,
    ) {
        self.host(app).set_video_layer(layer.clone());
        app.get_mut(self).video_layer = layer;
    }

    /// Hands the video to a frame reader from now on, and to no layer.
    pub fn read_frames(self: Handle<Self>, app: &mut App) -> Arc<dyn VideoFrameReader> {
        app.get_mut(self).video_layer = None;
        self.host(app).read_frames()
    }

    /// Stops the player and lets go of the host's: its picture leaves the layer it was on.
    /// Its listeners hear nothing more.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let player = app.get_mut(self);
        player.host = None;
        player.video_layer = None;
        player.change_notifier.dispose();
    }

    /// The host's player.
    ///
    /// # Panics
    ///
    /// Once disposed.
    fn host(self: Handle<Self>, app: &App) -> &dyn MediaPlayer {
        app.get(self)
            .host
            .as_deref()
            .expect("a Player used after dispose")
    }

    /// Takes the host's state at each of its changes, from the one `changed` resolves at, until
    /// the player is disposed.
    fn follow(self: Handle<Self>, app: &mut App, changed: MediaChangeFuture) {
        let _following = app.spawn(async move |cx| {
            let mut changed = changed;
            loop {
                changed.await;
                match cx.update(|app| self.take_host_state(app)) {
                    Some(next) => changed = next,
                    None => return,
                }
            }
        });
    }

    /// Takes the host's state, telling the listeners if it changed; answers the future of the
    /// host's next change, asked before the state is read so that no change goes unheard, or
    /// `None` once the player is disposed.
    fn take_host_state(self: Handle<Self>, app: &mut App) -> Option<MediaChangeFuture> {
        let host = app.get(self).host.clone()?;
        let next = host.changed();
        let state = host.state();
        if state != app.get(self).state {
            app.get_mut(self).state = state;
            self.notify_listeners(app);
        }
        Some(next)
    }
}

impl ChangeNotifier for Player {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Player")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl PictureSource for Player {
    fn set_layer(handle: Handle<Self>, app: &mut App, layer: Option<Arc<dyn CompositionLayer>>) {
        handle.set_video_layer(app, layer);
    }

    fn layer(handle: Handle<Self>, app: &App) -> Option<&Arc<dyn CompositionLayer>> {
        app.get(handle).video_layer.as_ref()
    }

    fn picture_size(handle: Handle<Self>, app: &App) -> Option<Size> {
        handle.state(app).size
    }
}

/// A player's video, Media3's `PlayerView`, laid out at its size in logical pixels, or as large as
/// the constraints let it be at that aspect; no room until the size is known. It shows the player
/// on a composition layer of its own, which the system draws the video into: the cheapest way, and
/// the only one protected and HDR video show in. The player shows on the view built for it last.
/// For a video the framework draws, under effects a layer cannot take, see `VideoFrameView`.
#[derive(Clone)]
pub struct PlayerView {
    pub key: Option<KeyRef>,
    pub player: Handle<Player>,
}

impl PlayerView {
    pub fn new(player: Handle<Player>) -> PlayerView {
        PlayerView { key: None, player }
    }

    pub fn key(mut self, key: KeyRef) -> PlayerView {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for PlayerView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayerView").finish_non_exhaustive()
    }
}

impl StatelessWidget for PlayerView {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        CompositionLayerPicture {
            source: self.player,
        }
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use inset_embedder::{PickedFile, PlayerStatus, TargetPlatform};
    use inset_foundation::AppCell;
    use inset_test::{MockCompositor, MockPlayers, PlayerCall, TestPlatform, TestView};

    use super::*;
    use crate::binding::run_app;
    use crate::test_harness::pump_every_view;

    fn app_with(
        players: &MockPlayers,
        compositor: &MockCompositor,
        view: &Rc<TestView>,
    ) -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::clone(view) as inset_embedder::ViewRef)
            .with_players(Rc::new(players.clone()))
            .with_compositor(Arc::new(compositor.clone()));
        AppCell::with_platform(Rc::new(platform))
    }

    fn ready(size: Size) -> PlayerState {
        PlayerState {
            status: PlayerStatus::Ready,
            size: Some(size),
            duration: Some(Duration::from_secs(10)),
            ..PlayerState::loading()
        }
    }

    #[test]
    fn a_player_opens_its_source_and_its_listeners_hear_its_state_change() {
        let players = MockPlayers::new();
        let cell = app_with(
            &players,
            &MockCompositor::new(),
            &Rc::new(TestView::new(800.0, 600.0)),
        );
        let picked = PickedFile::Path(PathBuf::from("/movies/a b.mov"));
        let player = Player::new(&mut cell.borrow_mut(), &picked.url()).unwrap();
        let opened = players.opened();
        assert_eq!(opened[0].url(), "file:///movies/a%20b.mov");
        assert_eq!(player.state(&cell.borrow()).status, PlayerStatus::Loading);

        let heard = Rc::new(std::cell::Cell::new(0));
        {
            let heard = Rc::clone(&heard);
            let listenable: Rc<dyn inset_foundation::Listenable> = Rc::new(player);
            listenable.add_listener(
                &mut cell.borrow_mut(),
                inset_foundation::Listener::new(move |_app| heard.set(heard.get() + 1)),
            );
        }
        opened[0].set_state(ready(Size::new(640.0, 360.0)));
        cell.checkpoint();
        assert_eq!(heard.get(), 1);
        assert_eq!(
            player.state(&cell.borrow()).size,
            Some(Size::new(640.0, 360.0))
        );

        player.play(&cell.borrow());
        player.seek_to(&cell.borrow(), Duration::from_secs(3));
        player.set_looping(&cell.borrow(), true);
        assert_eq!(
            opened[0].calls(),
            vec![
                PlayerCall::Play,
                PlayerCall::Seek(Duration::from_secs(3)),
                PlayerCall::SetLooping(true)
            ]
        );

        player.dispose(&mut cell.borrow_mut());
        assert!(
            opened[0].is_dropped(),
            "disposing lets go of the host's player"
        );
        opened[0].set_state(ready(Size::new(1.0, 1.0)));
        cell.checkpoint();
        assert_eq!(
            heard.get(),
            1,
            "a disposed player's listeners hear nothing more"
        );
    }

    #[test]
    fn a_platform_without_players_gives_no_player() {
        let cell = AppCell::with_platform(Rc::new(TestPlatform::new()));
        let player = Player::new(&mut cell.borrow_mut(), "https://example.com/a.mp4");
        assert!(player.is_none());
    }

    #[test]
    fn a_player_view_shows_its_player_on_a_layer_it_places_at_the_videos_size() {
        let players = MockPlayers::new();
        let compositor = MockCompositor::new();
        let view = Rc::new(TestView::new(800.0, 600.0));
        let cell = app_with(&players, &compositor, &view);
        let player = Player::new(&mut cell.borrow_mut(), "https://example.com/a.mp4").unwrap();
        run_app(
            &mut cell.borrow_mut(),
            crate::Center::new()
                .child(PlayerView::new(player))
                .into_widget(),
        );
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);

        let made = compositor.made();
        assert_eq!(made.len(), 1, "one layer, of the view's own");
        assert!(!made[0].opaque);
        let video_layer = players.opened()[0]
            .video_layer()
            .expect("put on the view's layer");
        assert!(Arc::ptr_eq(&video_layer, &made[0].layer));
        assert_eq!(
            placements(&view).len(),
            0,
            "nothing placed until the size is known"
        );

        players.opened()[0].set_state(ready(Size::new(320.0, 180.0)));
        cell.checkpoint();
        pump_every_view(&mut cell.borrow_mut(), Duration::from_millis(16));
        let placements = placements(&view);
        assert_eq!(placements.len(), 1);
        let (placed, placement) = &placements[0];
        assert!(Arc::ptr_eq(placed, &made[0].layer));
        assert_eq!(
            placement.bounds(),
            inset_embedder::Rect::from_ltwh(240.0, 210.0, 320.0, 180.0)
        );
    }

    #[test]
    fn a_player_view_puts_its_player_nowhere_as_it_goes_unless_another_took_it_since() {
        let players = MockPlayers::new();
        let view = Rc::new(TestView::new(800.0, 600.0));
        let cell = app_with(&players, &MockCompositor::new(), &view);
        let player = Player::new(&mut cell.borrow_mut(), "https://example.com/a.mp4").unwrap();
        let host = &players.opened()[0];
        mount(&cell, PlayerView::new(player).into_widget());
        assert!(host.video_layer().is_some());
        mount(&cell, crate::SizedBox::new().into_widget());
        assert!(
            host.video_layer().is_none(),
            "unmounted, it puts the video nowhere"
        );
        assert_eq!(host.calls().last(), Some(&PlayerCall::SetVideoLayer(false)));

        mount(&cell, PlayerView::new(player).into_widget());
        let elsewhere = inset_embedder::test_support::idle_layer();
        player.set_video_layer(&mut cell.borrow_mut(), Some(Arc::clone(&elsewhere)));
        mount(&cell, crate::SizedBox::new().into_widget());
        assert!(
            host.video_layer()
                .is_some_and(|layer| Arc::ptr_eq(&layer, &elsewhere)),
            "a view leaves alone a layer it did not set"
        );
    }

    /// Runs `widget` as the app's content, and its first frame.
    fn mount(cell: &AppCell, widget: crate::WidgetRef) {
        run_app(&mut cell.borrow_mut(), widget);
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
    }

    /// The composition layers `view`'s last scene places, with their placements.
    fn placements(
        view: &TestView,
    ) -> Vec<(
        Arc<dyn inset_embedder::CompositionLayer>,
        inset_embedder::Placement,
    )> {
        let scene = view.last_scene().expect("a frame was presented");
        inset_embedder::slices(&scene, |_| None)
            .composition_layers()
            .map(|(layer, placement)| (Arc::clone(layer), placement.clone()))
            .collect()
    }
}
