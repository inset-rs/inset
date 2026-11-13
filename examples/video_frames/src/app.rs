//! The window: the video in the middle, drawn by the framework at its aspect with the player's
//! subtitles over it in the user's caption style, and a line of status under it, whose position
//! is read from the player at each frame while it plays, as a widget follows an animation, with a
//! ticker its state's ticker provider makes; a click on the video plays or pauses it. The
//! player's tracks are printed each time they change.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use waft_embedder::{
    Color, MediaTrack, MediaTrackKind, PauseReason, PlayerActivity, PlayerState, PlayerStatus,
    TextDirection, VideoOutput, WaitReason, WindowConfig,
};
use waft_embedder_winit::{LinuxSettings, WinitEmbedder};
use waft_foundation::{App, Handle, Listenable, Listener};
use waft_painting::{
    AlignmentGeometry, BorderRadiusGeometry, BoxDecoration, EdgeInsetsGeometry, TextStyle,
};
use waft_scheduler::{FrameCallback, Ticker};
use waft_shell::Shell;
use waft_widgets::{
    AspectRatio, BuildContext, Center, ClipRRect, Column, DecoratedBox, Directionality,
    GestureDetector, IntoWidget, ListenableBuilder, MainAxisSize, Padding, Player, Positioned,
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, SizedBox, Stack, State,
    StateData, StatefulWidget, SubtitleView, Text, Transform, VideoFrameView, WidgetRef,
    run_app_with,
};

/// The video's width in logical pixels; its height follows its aspect.
const VIDEO_WIDTH: f64 = 640.0;
/// The aspect taken until the player knows the video's.
const WIDESCREEN: f64 = 16.0 / 9.0;
const BACKGROUND: Color = Color::new(0xFF20_2830);
const STATUS: Color = Color::new(0xFFE0_E6EB);

/// What is played, and how it is shown.
#[derive(Clone)]
pub(crate) struct Presentation {
    pub(crate) url: String,
    /// How far the video is turned about its middle, clockwise.
    pub(crate) turn_degrees: f64,
    /// The radius its corners are cut round by.
    pub(crate) corner_radius: f64,
    /// The language whose text track to show, as a BCP 47 tag or its start, such as `en`;
    /// `None` to leave the player's choice.
    pub(crate) subtitles: Option<String>,
}

/// Plays the video in the window; closing the window quits, as an app's last window does.
pub(crate) fn run(presentation: Presentation) {
    WinitEmbedder::default()
        .linux(LinuxSettings {
            app_id: Some(waft_macros::linux_app_id!().to_owned()),
        })
        .run(move |platform| {
            Shell::new(platform, move |app| {
                let config = WindowConfig {
                    title: "Waft — a video player".to_owned(),
                    size: [800.0, 560.0],
                    ..WindowConfig::default()
                };
                let home = playing(app, &presentation);
                run_app_with(app, home, config);
            })
        });
}

/// Plays the video as `presentation` says: the window's content.
fn playing(app: &mut App, presentation: &Presentation) -> WidgetRef {
    let player = Player::new(app, &presentation.url).expect("macOS has players");
    Player::set_looping(player, app, true);
    let reader = app
        .platform()
        .video_frame_readers()
        .expect("macOS hands out frames")
        .make_reader();
    Player::set_video_output(player, app, Some(VideoOutput::Frames(Arc::clone(&reader))));
    let video = VideoFrameView::new(reader).into_widget();
    follow_tracks(app, player, presentation.subtitles.clone());
    Player::play(player, app);
    let presentation = presentation.clone();
    let listenable: Rc<dyn Listenable> = Rc::new(player);
    ListenableBuilder::new(listenable, move |app, _, _| {
        let subtitled = with_subtitles(video.clone(), Player::cues(player, app).to_vec());
        let state = Player::state(player, app);
        page(player, subtitled, state, &presentation)
    })
    .into_widget()
}

/// Prints the player's tracks each time they change, and shows the first text track in
/// `language` once one is listed.
fn follow_tracks(app: &mut App, player: Handle<Player>, language: Option<String>) {
    let printed = RefCell::new(Vec::new());
    let chosen = Cell::new(false);
    let listenable: Rc<dyn Listenable> = Rc::new(player);
    listenable.add_listener(
        app,
        Listener::new(move |app| {
            let tracks = Player::state(player, app).tracks.clone();
            if *printed.borrow() != tracks {
                print_tracks(&tracks);
                *printed.borrow_mut() = tracks.clone();
            }
            let Some(language) = &language else {
                return;
            };
            if chosen.get() {
                return;
            }
            if let Some(track) = text_track_in(&tracks, language) {
                Player::select_text_track(player, app, Some(&track.id));
                chosen.set(true);
            }
        }),
    );
}

fn print_tracks(tracks: &[MediaTrack]) {
    println!("tracks:");
    for track in tracks {
        let selected = if track.selected { "*" } else { " " };
        println!(
            "  {selected} {:<10} {:?} {:<6} {}",
            track.id, track.kind, track.language, track.label
        );
    }
}

/// The first text track whose language is `language` or starts with it, such as `en` for
/// `en-US`.
fn text_track_in<'a>(tracks: &'a [MediaTrack], language: &str) -> Option<&'a MediaTrack> {
    tracks.iter().find(|track| {
        track.kind == MediaTrackKind::Text
            && (track.language == language || track.language.starts_with(&format!("{language}-")))
    })
}

/// `video` with a subtitle view of `cues` over it, the size of its picture.
fn with_subtitles(video: WidgetRef, cues: Vec<waft_embedder::Cue>) -> WidgetRef {
    Stack::new()
        .alignment(AlignmentGeometry::TOP_LEFT)
        .children([
            video,
            Positioned::fill(SubtitleView::new(cues)).into_widget(),
        ])
        .into_widget()
}

/// The video at its aspect, with the status under it.
fn page(
    player: Handle<Player>,
    video: WidgetRef,
    state: &PlayerState,
    presentation: &Presentation,
) -> WidgetRef {
    let aspect = state
        .size
        .filter(|size| size.width() > 0.0 && size.height() > 0.0)
        .map_or(WIDESCREEN, |size| size.width() / size.height());
    let framed = SizedBox::new()
        .width(VIDEO_WIDTH)
        .child(AspectRatio::new(aspect).child(video));
    let rounded = ClipRRect::new()
        .border_radius(BorderRadiusGeometry::circular(presentation.corner_radius))
        .child(framed);
    let turned = Transform::rotate(presentation.turn_degrees.to_radians()).child(rounded);
    let paused = matches!(state.activity, PlayerActivity::Paused(_));
    let clickable = GestureDetector::new()
        .on_tap(Listener::new(move |app| {
            if paused {
                Player::play(player, app);
            } else {
                Player::pause(player, app);
            }
        }))
        .child(turned);
    let column = Column::new().main_axis_size(MainAxisSize::Min).children([
        clickable.into_widget(),
        Padding::new(EdgeInsetsGeometry::only(0.0, 16.0, 0.0, 0.0))
            .child(StatusLine { player })
            .into_widget(),
    ]);
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
            .child(Center::new().child(column)),
    )
    .into_widget()
}

/// What the player is doing, how far it is, and what size its frames are, its position read at
/// each frame while it plays.
#[derive(Debug)]
struct StatusLine {
    player: Handle<Player>,
}

impl StatefulWidget for StatusLine {
    type State = StatusLineState;

    fn create_state(&self) -> StatusLineState {
        StatusLineState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            ticker: None,
            shown: String::new(),
        }
    }
}

struct StatusLineState {
    state: StateData<StatusLine>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Ticks at each frame while the player plays; made in `init_state`.
    ticker: Option<Handle<Ticker>>,
    /// The line as it was built last.
    shown: String,
}

impl StatusLineState {
    fn ticker(self: Handle<Self>, app: &App) -> Handle<Ticker> {
        app.get(self).ticker.expect("made in init_state")
    }

    /// The player's state changed, or its position jumped: the line is built again.
    fn player_changed(self: Handle<Self>, app: &mut App) {
        self.tick_while_playing(app);
        self.set_state(app, |_| {});
    }

    /// Runs the ticker while the player plays, and stops it otherwise.
    fn tick_while_playing(self: Handle<Self>, app: &mut App) {
        let playing =
            Player::state(self.widget(app).player, app).activity == PlayerActivity::Playing;
        let ticker = self.ticker(app);
        if playing && !ticker.is_active(app) {
            let _ticking = ticker.start(app);
        } else if !playing && ticker.is_active(app) {
            ticker.stop(app, true);
        }
    }

    /// A frame while the player plays: the line is built again once what it shows changed.
    fn tick(self: Handle<Self>, app: &mut App) {
        if status_line(app, self.widget(app).player) != app.get(self).shown {
            self.set_state(app, |_| {});
        }
    }
}

impl SingleTickerProviderStateMixin for StatusLineState {
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

impl State for StatusLineState {
    type Widget = StatusLine;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, _elapsed| self.tick(app)),
        );
        app.get_mut(self).ticker = Some(ticker);
        let player = self.widget(app).player;
        player.add_listener(app, Listener::handle_method(self, Self::player_changed));
        self.tick_while_playing(app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let player = self.widget(app).player;
        player.remove_listener(app, &Listener::handle_method(self, Self::player_changed));
        if let Some(ticker) = app.get_mut(self).ticker.take() {
            ticker.dispose(app);
        }
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let line = status_line(app, self.widget(app).player);
        app.get_mut(self).shown = line.clone();
        Text::new(line)
            .style(TextStyle::new().font_size(14.0).color(STATUS))
            .into_widget()
    }
}

/// `player`'s status, its position read now.
fn status_line(app: &App, player: Handle<Player>) -> String {
    status(Player::state(player, app), Player::position(player, app))
}

/// What the player is doing, how far it is, at `position`, and what size its frames are.
fn status(state: &PlayerState, position: Duration) -> String {
    let doing = match &state.status {
        PlayerStatus::Loading => return "Loading…".to_owned(),
        PlayerStatus::Failed(error) => {
            return format!("Failed: {:?} ({})", error.kind, error.message);
        }
        PlayerStatus::Ready if state.seeking.is_some() => "Seeking…",
        PlayerStatus::Ready => doing(state.activity),
    };
    let length = state.duration.map_or("live".to_owned(), clock);
    let size = state.size.map_or(String::new(), |size| {
        format!(" · {}×{}", size.width(), size.height())
    });
    format!(
        "{doing} · {} / {length}{size} · drawn by Waft · click to play or pause",
        clock(position)
    )
}

/// What `activity` shows: no waiting while the system is still judging whether data comes fast
/// enough, as Apple advises.
fn doing(activity: PlayerActivity) -> &'static str {
    match activity {
        PlayerActivity::Playing | PlayerActivity::Waiting(Some(WaitReason::EvaluatingData)) => {
            "Playing"
        }
        PlayerActivity::Waiting(_) => "Waiting for the media…",
        PlayerActivity::Paused(Some(PauseReason::End)) => "Ended",
        PlayerActivity::Paused(Some(PauseReason::Interrupted)) => "Paused by the system",
        PlayerActivity::Paused(_) => "Paused",
    }
}

/// `duration` as minutes and seconds.
fn clock(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
