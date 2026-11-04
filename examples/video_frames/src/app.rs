//! The window: the video in the middle, drawn by the framework at its aspect with the player's
//! subtitles over it, and a line of status under it; a click on the video plays or pauses it. The
//! player's tracks are printed each time they change.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{
    Color, MediaTrack, MediaTrackKind, PlayerState, PlayerStatus, TextDirection, VideoOutput,
    ViewId, WindowConfig,
};
use inset_embedder_winit::WinitEmbedder;
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_painting::{
    AlignmentGeometry, BorderRadiusGeometry, BoxDecoration, EdgeInsetsGeometry, TextStyle,
};
use inset_shell::Shell;
use inset_widgets::{
    AspectRatio, Center, ClipRRect, Column, DecoratedBox, Directionality, GestureDetector,
    IntoWidget, ListenableBuilder, MainAxisSize, Padding, Player, Positioned, SizedBox, Stack,
    SubtitleView, Text, Transform, VideoFrameView, ViewRequestState, WidgetRef, Window,
    WindowController, run_widget,
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

pub(crate) fn run(presentation: Presentation) {
    WinitEmbedder
        .run(move |platform| Shell::new(platform, move |app| open_window(app, presentation)));
}

/// Asks for the window, and once its view has arrived plays the video into it; closing the
/// window quits.
fn open_window(app: &mut App, presentation: Presentation) {
    let config = WindowConfig {
        title: "Inset — a video player".to_owned(),
        size: [800.0, 560.0],
        ..WindowConfig::default()
    };
    let controller = WindowController::new(app, config);
    controller.on_close_requested(app, Rc::new(quit));
    let shown = Cell::new(false);
    let listenable: Rc<dyn Listenable> = Rc::new(controller);
    listenable.add_listener(
        app,
        Listener::new(move |app| {
            let arrived = matches!(
                controller.request(app).state(app),
                ViewRequestState::Arrived(_)
            );
            if arrived && !shown.replace(true) {
                show(app, controller, &presentation);
            }
        }),
    );
}

fn show(app: &mut App, controller: Handle<WindowController>, presentation: &Presentation) {
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
    let home = ListenableBuilder::new(listenable, move |app, _, _| {
        let subtitled = with_subtitles(video.clone(), Player::cues(player, app).to_vec());
        let state = Player::state(player, app);
        page(player, subtitled, state, &presentation)
    });
    run_widget(app, Window::new(controller, home).into_widget());
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

fn quit(app: &mut App, _view: ViewId) {
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}

/// `video` with a subtitle view of `cues` over it, the size of its picture.
fn with_subtitles(video: WidgetRef, cues: Vec<inset_embedder::Cue>) -> WidgetRef {
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
    let playing = state.playing;
    let clickable = GestureDetector::new()
        .on_tap(Listener::new(move |app| {
            if playing {
                Player::pause(player, app);
            } else {
                Player::play(player, app);
            }
        }))
        .child(turned);
    let column = Column::new().main_axis_size(MainAxisSize::Min).children([
        clickable.into_widget(),
        Padding::new(EdgeInsetsGeometry::only(0.0, 16.0, 0.0, 0.0))
            .child(Text::new(status(state)).style(TextStyle::new().font_size(14.0).color(STATUS)))
            .into_widget(),
    ]);
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
            .child(Center::new().child(column)),
    )
    .into_widget()
}

/// What the player is doing, how far it is, and what size its frames are.
fn status(state: &PlayerState) -> String {
    let doing = match &state.status {
        PlayerStatus::Loading => return "Loading…".to_owned(),
        PlayerStatus::Failed(reason) => return format!("Failed: {reason}"),
        PlayerStatus::Ready if state.playing => "Playing",
        PlayerStatus::Ready => "Paused",
    };
    let length = state.duration.map_or("live".to_owned(), clock);
    let size = state.size.map_or(String::new(), |size| {
        format!(" · {}×{}", size.width(), size.height())
    });
    format!(
        "{doing} · {} / {length}{size} · drawn by Inset · click to play or pause",
        clock(state.position)
    )
}

/// `duration` as minutes and seconds.
fn clock(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
