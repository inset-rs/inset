//! A player, as an app holds it, and the view that shows its video.

use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{
    CompositionLayer, Cue, MediaChangeFuture, MediaMetadata, MediaPlayer, PlayerState,
    SeekTolerance, VideoOutput,
};
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, ListenableObject, Listener,
};
use inset_painting::AlignmentGeometry;

use super::{SubtitleView, at_size};
use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, StatelessWidget, WidgetRef,
};
use crate::widgets::basic::{Positioned, SizedBox, Stack};
use crate::widgets::composition_layer_view::CompositionLayerView;
use crate::widgets::transitions::ListenableBuilder;

/// A player of the media at one URL, which loads by itself: the host's player, whose listeners
/// hear its state, its metadata or its cues change and its position jump, at each change its host
/// tells. A URL the system cannot play gives a player whose state is a failure.
pub struct Player {
    change_notifier: ChangeNotifierData,
    /// The host's player, until [`dispose`](Player::dispose).
    host: Option<Rc<dyn MediaPlayer>>,
    /// The host's state when it last changed.
    state: PlayerState,
    /// The host's metadata when it last changed.
    metadata: MediaMetadata,
    /// The host's cues when they last changed.
    cues: Vec<Cue>,
    /// Where the video was sent last, which a [`PlayerView`] sends it nowhere from only while
    /// it is still the view's own layer.
    video_output: Option<VideoOutput>,
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
            metadata: host.metadata(),
            cues: host.cues(),
            host: Some(host),
            video_output: None,
        });
        player.follow(app, changed);
        Some(player)
    }

    /// The player's state when it last changed.
    pub fn state(self: Handle<Self>, app: &App) -> &PlayerState {
        &app.get(self).state
    }

    /// What the media's tags say about it, when that last changed: Media3's `getMediaMetadata`.
    pub fn metadata(self: Handle<Self>, app: &App) -> &MediaMetadata {
        &app.get(self).metadata
    }

    /// The cues to show now, as the media styles them: Media3's `getCurrentCues`. A
    /// [`PlayerView`] draws them over the video in the user's caption style; an app that draws
    /// the frames itself puts a `SubtitleView` of them over its video.
    pub fn cues(self: Handle<Self>, app: &App) -> &[Cue] {
        &app.get(self).cues
    }

    /// Where the player is in the media now, read from the host's player at each call: Media3's
    /// `getCurrentPosition`. It moves on by itself while the player plays, which its listeners do
    /// not hear; they hear it jump. A widget that shows it while it plays reads it at each frame,
    /// with a ticker running while the state says it plays.
    ///
    /// # Panics
    ///
    /// Once disposed.
    pub fn position(self: Handle<Self>, app: &App) -> Duration {
        self.host(app).position()
    }

    pub fn play(self: Handle<Self>, app: &App) {
        self.host(app).play();
    }

    pub fn pause(self: Handle<Self>, app: &App) {
        self.host(app).pause();
    }

    /// Moves to `position`, or as near it as `tolerance` lets the system land for speed,
    /// playing on from there if it was playing; the state tells the seek under way until it
    /// lands.
    pub fn seek_to(self: Handle<Self>, app: &App, position: Duration, tolerance: SeekTolerance) {
        self.host(app).seek(position, tolerance);
    }

    /// How fast it plays, `1.0` the media's own speed.
    pub fn set_rate(self: Handle<Self>, app: &App, rate: f64) {
        self.host(app).set_rate(rate);
    }

    /// How loud it plays, from `0.0` to `1.0`.
    pub fn set_volume(self: Handle<Self>, app: &App, volume: f64) {
        self.host(app).set_volume(volume);
    }

    /// Whether its sound is off, apart from its volume.
    pub fn set_muted(self: Handle<Self>, app: &App, muted: bool) {
        self.host(app).set_muted(muted);
    }

    /// Whether it starts again from the beginning when it reaches the end.
    pub fn set_looping(self: Handle<Self>, app: &App, looping: bool) {
        self.host(app).set_looping(looping);
    }

    /// Plays the audio track `id`, one of the state's `tracks`, in place of the one playing.
    pub fn select_audio_track(self: Handle<Self>, app: &App, id: &str) {
        self.host(app).select_audio_track(id);
    }

    /// Shows the text track `id`, one of the state's `tracks`, or none: subtitles off.
    pub fn select_text_track(self: Handle<Self>, app: &App, id: Option<&str>) {
        self.host(app).select_text_track(id);
    }

    /// Sends the video to `output` from now on, and nowhere it went before; `None` sends it
    /// nowhere: Media3's `setVideoSurface`. A [`PlayerView`] sends it to a composition layer of
    /// its own as it is mounted, and nowhere as it is unmounted; an app that draws the frames
    /// itself sends it to a reader it made, which a `VideoFrameView` draws.
    pub fn set_video_output(self: Handle<Self>, app: &mut App, output: Option<VideoOutput>) {
        self.host(app).set_video_output(output.clone());
        app.get_mut(self).video_output = output;
    }

    /// Stops the player and lets go of the host's: its picture leaves the layer it was on.
    /// Its listeners hear nothing more.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let player = app.get_mut(self);
        player.host = None;
        player.video_output = None;
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

    /// Takes the host's state, metadata and cues and tells the listeners, since the host's
    /// change may have been a jump of the position, which nothing taken shows; answers the future
    /// of the host's next change, asked before they are read so that no change goes unheard, or
    /// `None` once the player is disposed.
    fn take_host_state(self: Handle<Self>, app: &mut App) -> Option<MediaChangeFuture> {
        let host = app.get(self).host.clone()?;
        let next = host.changed();
        let (state, metadata, cues) = (host.state(), host.metadata(), host.cues());
        let player = app.get_mut(self);
        player.state = state;
        player.metadata = metadata;
        player.cues = cues;
        self.notify_listeners(app);
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

impl Player {
    /// Puts the video on `layer` from now on; `None` sends it nowhere.
    fn set_video_layer(
        self: Handle<Self>,
        app: &mut App,
        layer: Option<Arc<dyn CompositionLayer>>,
    ) {
        self.set_video_output(app, layer.map(VideoOutput::Layer));
    }

    /// The layer the video was sent to last; `None` once sent nowhere or to a reader, or
    /// disposed.
    fn video_layer(self: Handle<Self>, app: &App) -> Option<&Arc<dyn CompositionLayer>> {
        match &app.get(self).video_output {
            Some(VideoOutput::Layer(layer)) => Some(layer),
            _ => None,
        }
    }
}

/// A player's video, Media3's `PlayerView`, laid out at its size in logical pixels, or as large as
/// the constraints let it be at that aspect; no room until the size is known. It shows the player
/// on a composition layer of its own, which the system draws the video into: the cheapest way, and
/// the only one protected and HDR video show in. The player shows on the view built for it last.
/// Over the video a [`SubtitleView`] draws the player's cues. For a video the framework draws,
/// under effects a layer cannot take, see `VideoFrameView`.
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
        PlayerPicture {
            player: self.player,
            overlay: subtitles_of(self.player),
        }
        .into_widget()
    }
}

/// `player`'s video on a composition layer of the widget's own, laid out at the video's size in
/// logical pixels, as large as the constraints let it be at that aspect, with `overlay` over the
/// video's rectangle, as Media3's content frame holds its subtitle view over the surface.
struct PlayerPicture {
    player: Handle<Player>,
    overlay: WidgetRef,
}

impl fmt::Debug for PlayerPicture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayerPicture").finish_non_exhaustive()
    }
}

impl StatefulWidget for PlayerPicture {
    type State = PlayerPictureState;

    fn create_state(&self) -> PlayerPictureState {
        PlayerPictureState {
            state: StateData::new(),
            layer: None,
        }
    }
}

struct PlayerPictureState {
    state: StateData<PlayerPicture>,
    /// The layer the player shows on, made as the widget is mounted; `None` where the host has
    /// no compositor.
    layer: Option<Arc<dyn CompositionLayer>>,
}

impl PlayerPictureState {
    fn player_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    /// Puts `player`'s video on the widget's layer.
    fn show(self: Handle<Self>, app: &mut App, player: Handle<Player>) {
        if let Some(layer) = app.get(self).layer.clone() {
            player.set_video_layer(app, Some(layer));
        }
    }

    /// Sends `player`'s video nowhere, if it is still on this widget's layer: a later view may
    /// have taken the player since.
    fn release(self: Handle<Self>, app: &mut App, player: Handle<Player>) {
        let Some(ours) = &app.get(self).layer else {
            return;
        };
        let still_ours = player
            .video_layer(app)
            .is_some_and(|shown| Arc::ptr_eq(shown, ours));
        if still_ours {
            player.set_video_layer(app, None);
        }
    }
}

impl State for PlayerPictureState {
    type Widget = PlayerPicture;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let player = self.widget(app).player;
        player.add_listener(app, Listener::handle_method(self, Self::player_changed));
        let layer = app
            .platform()
            .compositor()
            .map(|compositor| compositor.make_layer(false));
        app.get_mut(self).layer = layer;
        self.show(app, player);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &PlayerPicture) {
        let player = self.widget(app).player;
        if player == old_widget.player {
            return;
        }
        let listener = Listener::handle_method(self, Self::player_changed);
        old_widget.player.remove_listener(app, &listener);
        player.add_listener(app, listener);
        self.release(app, old_widget.player);
        self.show(app, player);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let player = self.widget(app).player;
        self.release(app, player);
        player.remove_listener(app, &Listener::handle_method(self, Self::player_changed));
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let player = self.widget(app).player;
        let Some(size) = player.state(app).size else {
            return SizedBox::shrink().into_widget();
        };
        let video = match app.get(self).layer.clone() {
            Some(layer) => CompositionLayerView::new(layer).into_widget(),
            None => SizedBox::expand().into_widget(),
        };
        let overlay = self.widget(app).overlay.clone();
        at_size(size, with_overlay(video, overlay))
    }
}

/// `video` with `overlay` filling its rectangle above it.
fn with_overlay(video: WidgetRef, overlay: WidgetRef) -> WidgetRef {
    Stack::new()
        .alignment(AlignmentGeometry::TOP_LEFT)
        .children([video, Positioned::fill(overlay).into_widget()])
        .into_widget()
}

/// A [`SubtitleView`] of `player`'s cues, built again each time they change: Media3's
/// `PlayerView` setting its subtitle view's cues at each `onCues`.
fn subtitles_of(player: Handle<Player>) -> WidgetRef {
    let listenable: Rc<dyn Listenable> = Rc::new(player);
    ListenableBuilder::new(listenable, move |app, _, _| {
        SubtitleView::new(Player::cues(player, app).to_vec()).into_widget()
    })
    .into_widget()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use inset_embedder::{
        CueContent, CueTextRun, EncodedImage, MediaTrack, MediaTrackKind, PickedFile, PlayerStatus,
        Size, TargetPlatform,
    };
    use inset_foundation::AppCell;
    use inset_test::{
        MockCompositor, MockPlayers, MockVideoFrameReaders, PlayerCall, TestPlatform, TestView,
        VideoOutputKind,
    };

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
        player.seek_to(&cell.borrow(), Duration::from_secs(3), SeekTolerance::EXACT);
        player.set_muted(&cell.borrow(), true);
        player.set_looping(&cell.borrow(), true);
        assert_eq!(
            opened[0].calls(),
            vec![
                PlayerCall::Play,
                PlayerCall::Seek(Duration::from_secs(3), SeekTolerance::EXACT),
                PlayerCall::SetMuted(true),
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
    fn a_player_reads_its_position_from_its_host_and_its_listeners_hear_it_jump() {
        let players = MockPlayers::new();
        let cell = app_with(
            &players,
            &MockCompositor::new(),
            &Rc::new(TestView::new(800.0, 600.0)),
        );
        let player = Player::new(&mut cell.borrow_mut(), "https://example.com/a.mp4").unwrap();
        let host = &players.opened()[0];
        let heard = Rc::new(std::cell::Cell::new(0));
        {
            let heard = Rc::clone(&heard);
            let listenable: Rc<dyn inset_foundation::Listenable> = Rc::new(player);
            listenable.add_listener(
                &mut cell.borrow_mut(),
                inset_foundation::Listener::new(move |_app| heard.set(heard.get() + 1)),
            );
        }
        assert_eq!(player.position(&cell.borrow()), Duration::ZERO);

        host.move_to(Duration::from_millis(1500));
        cell.checkpoint();
        assert_eq!(
            player.position(&cell.borrow()),
            Duration::from_millis(1500),
            "read from the host as it moves on"
        );
        assert_eq!(heard.get(), 0, "moving on is not told");

        host.jump_to(Duration::from_secs(7));
        cell.checkpoint();
        assert_eq!(player.position(&cell.borrow()), Duration::from_secs(7));
        assert_eq!(heard.get(), 1, "a jump is heard");
    }

    #[test]
    fn a_player_hears_its_tracks_metadata_and_cues_and_asks_its_host_to_select_tracks() {
        let players = MockPlayers::new();
        let cell = app_with(
            &players,
            &MockCompositor::new(),
            &Rc::new(TestView::new(800.0, 600.0)),
        );
        let player = Player::new(&mut cell.borrow_mut(), "https://example.com/a.m3u8").unwrap();
        let host = &players.opened()[0];
        let heard = Rc::new(std::cell::Cell::new(0));
        {
            let heard = Rc::clone(&heard);
            let listenable: Rc<dyn inset_foundation::Listenable> = Rc::new(player);
            listenable.add_listener(
                &mut cell.borrow_mut(),
                inset_foundation::Listener::new(move |_app| heard.set(heard.get() + 1)),
            );
        }

        let english = MediaTrack {
            id: "legible:0".to_owned(),
            kind: MediaTrackKind::Text,
            language: "en".to_owned(),
            label: "English".to_owned(),
            selected: false,
        };
        host.set_state(PlayerState {
            tracks: vec![english.clone()],
            ..ready(Size::new(640.0, 360.0))
        });
        cell.checkpoint();
        assert_eq!(player.state(&cell.borrow()).tracks, vec![english]);

        player.select_text_track(&cell.borrow(), Some("legible:0"));
        player.select_audio_track(&cell.borrow(), "audible:1");
        assert_eq!(
            host.calls(),
            vec![
                PlayerCall::SelectTextTrack(Some("legible:0".to_owned())),
                PlayerCall::SelectAudioTrack("audible:1".to_owned()),
            ]
        );

        let metadata = MediaMetadata {
            title: Some("Bip Bop".to_owned()),
            artwork: Some(EncodedImage {
                mime_type: "image/png".to_owned(),
                data: Arc::from([0x89, b'P', b'N', b'G'].as_slice()),
            }),
            ..MediaMetadata::default()
        };
        host.set_metadata(metadata.clone());
        cell.checkpoint();
        assert_eq!(player.metadata(&cell.borrow()), &metadata);

        let cues = vec![Cue::new(CueContent::Text(vec![CueTextRun::new("Hello")]))];
        host.set_cues(cues.clone());
        cell.checkpoint();
        assert_eq!(player.cues(&cell.borrow()), cues.as_slice());
        assert_eq!(heard.get(), 3, "each change heard once");
    }

    #[test]
    fn a_player_view_draws_its_players_cues_over_the_video() {
        let players = MockPlayers::new();
        let view = Rc::new(TestView::new(800.0, 600.0));
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::clone(&view) as inset_embedder::ViewRef)
            .with_players(Rc::new(players.clone()))
            .with_compositor(Arc::new(MockCompositor::new()))
            .with_font_source(|| Box::new(inset_embedder::SystemFontSource::platform()));
        let cell = AppCell::with_platform(Rc::new(platform));
        {
            let mut app = cell.borrow_mut();
            inset_painting::PaintingBinding::instance(&mut app).install_platform_fonts(&mut app);
        }
        let player = Player::new(&mut cell.borrow_mut(), "https://example.com/a.mp4").unwrap();
        let host = &players.opened()[0];
        mount(&cell, PlayerView::new(player).into_widget());
        host.set_state(ready(Size::new(320.0, 180.0)));
        cell.checkpoint();
        pump_every_view(&mut cell.borrow_mut(), Duration::from_millis(16));
        assert!(glyph_runs(&view).is_empty(), "no cues, no text");

        host.set_cues(vec![Cue::new(CueContent::Text(vec![CueTextRun::new(
            "Hello",
        )]))]);
        cell.checkpoint();
        pump_every_view(&mut cell.borrow_mut(), Duration::from_millis(32));
        assert!(!glyph_runs(&view).is_empty(), "the cue's text is drawn");

        host.set_cues(Vec::new());
        cell.checkpoint();
        pump_every_view(&mut cell.borrow_mut(), Duration::from_millis(48));
        assert!(glyph_runs(&view).is_empty(), "a cue gone is drawn no more");
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
        assert_eq!(host.calls().last(), Some(&PlayerCall::SetVideoOutput(None)));

        mount(&cell, PlayerView::new(player).into_widget());
        let elsewhere = inset_embedder::test_support::idle_layer();
        player.set_video_output(
            &mut cell.borrow_mut(),
            Some(VideoOutput::Layer(Arc::clone(&elsewhere))),
        );
        mount(&cell, crate::SizedBox::new().into_widget());
        assert!(
            host.video_layer()
                .is_some_and(|layer| Arc::ptr_eq(&layer, &elsewhere)),
            "a view leaves alone a layer it did not set"
        );
    }

    #[test]
    fn a_reader_the_app_made_takes_the_video_from_a_view_which_leaves_it_alone() {
        let players = MockPlayers::new();
        let readers = MockVideoFrameReaders::new();
        let view = Rc::new(TestView::new(800.0, 600.0));
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::clone(&view) as inset_embedder::ViewRef)
            .with_players(Rc::new(players.clone()))
            .with_compositor(Arc::new(MockCompositor::new()))
            .with_video_frame_readers(Rc::new(readers.clone()));
        let cell = AppCell::with_platform(Rc::new(platform));
        let player = Player::new(&mut cell.borrow_mut(), "https://example.com/a.mp4").unwrap();
        let host = &players.opened()[0];
        mount(&cell, PlayerView::new(player).into_widget());

        let reader = cell
            .borrow()
            .platform()
            .video_frame_readers()
            .expect("the platform's readers")
            .make_reader();
        assert!(Arc::ptr_eq(&reader, &readers.made()[0]));
        player.set_video_output(
            &mut cell.borrow_mut(),
            Some(VideoOutput::Frames(Arc::clone(&reader))),
        );
        assert_eq!(
            host.calls().last(),
            Some(&PlayerCall::SetVideoOutput(Some(VideoOutputKind::Frames)))
        );
        assert!(host.video_layer().is_none(), "one destination at a time");

        mount(
            &cell,
            crate::VideoFrameView::new(Arc::clone(&reader)).into_widget(),
        );
        assert!(
            host.frame_reader()
                .is_some_and(|read_by| Arc::ptr_eq(&read_by, &reader)),
            "an unmounted view leaves alone a reader it did not set"
        );
    }

    /// Runs `widget` as the app's content, and its first frame.
    fn mount(cell: &AppCell, widget: crate::WidgetRef) {
        run_app(&mut cell.borrow_mut(), widget);
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
    }

    /// The glyph runs `view`'s last scene draws.
    fn glyph_runs(view: &TestView) -> Vec<inset_embedder::valo::Op> {
        let scene = view.last_scene().expect("a frame was presented");
        inset_embedder::flattened_ops(&inset_embedder::slices(&scene, |_| None).picture_with_room())
            .into_iter()
            .filter(|op| matches!(op, inset_embedder::valo::Op::GlyphRun { .. }))
            .collect()
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
