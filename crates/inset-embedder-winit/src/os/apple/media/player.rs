//! A player: AVFoundation's `AVPlayer` over one `AVPlayerItem`, controlled on the main thread,
//! whose state and position are read from them when asked and whose changes AVFoundation tells by
//! key-value observing, its end-of-item, time-jumped, rate and media selection notifications, each
//! seek's completion, and the loading of its tracks, tags and subtitles. Its picture goes either
//! to an `AVPlayerLayer` shown as a composition layer's content, or to an
//! `AVPlayerItemVideoOutput` added to its item, which the reader it was given reads; its subtitles
//! go to a legible output added to its item, which hands them over as cues.
//!
//! What it does is AVFoundation's `timeControlStatus`; why it paused is the reason AVFoundation
//! gave for its rate's last fall to zero, from macOS 12 and iOS 15, or the player's own pause,
//! and none for a pause it was not told the reason of. A seek is under way from its call until
//! AVFoundation completes it, which it does on a thread of its own; a later seek replaces it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use block2::RcBlock;
use inset_embedder::{
    CompositionLayer, Cue, MediaChangeFuture, MediaMetadata, MediaPlayer, PauseReason,
    PlayerActivity, PlayerError, PlayerErrorKind, PlayerState, PlayerStatus, Players,
    SeekTolerance, Size, TimeRange, VideoFrameReader, VideoOutput, WaitReason,
};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_av_foundation::{
    AVError, AVLayerVideoGravityResizeAspect, AVPlayer, AVPlayerActionAtItemEnd, AVPlayerItem,
    AVPlayerItemDidPlayToEndTimeNotification, AVPlayerItemFailedToPlayToEndTimeErrorKey,
    AVPlayerItemFailedToPlayToEndTimeNotification, AVPlayerItemMediaSelectionDidChangeNotification,
    AVPlayerItemStatus, AVPlayerItemTimeJumpedNotification, AVPlayerLayer,
    AVPlayerTimeControlStatus, AVPlayerWaitingToMinimizeStallsReason,
    AVPlayerWaitingWhileEvaluatingBufferingRateReason, AVURLAsset, NSValueAVFoundationExtensions,
};
use objc2_core_media::{CMTime, CMTimeFlags, kCMTimePositiveInfinity, kCMTimeZero};
use objc2_foundation::{
    NSArray, NSError, NSNotification, NSString, NSURL, NSUnderlyingErrorKey, NSValue,
};

use super::cues::LegibleCues;
use super::frames::{
    AvFrameReader, FrameOutput, PlayerFrames, report_foreign_reader, report_reader_in_use,
};
use super::metadata::AssetMetadata;
use super::observing::{NotificationObservation, PropertyObservation};
use super::show_system_layer;
use super::tracks::MediaTracks;
use crate::media::{Changes, lock};

/// The timescale a time this host gives AVFoundation is in: nanoseconds.
const NANOSECONDS: i32 = 1_000_000_000;

/// AVFoundation's players.
pub(crate) struct AvPlayers {
    main_thread: MainThreadMarker,
}

impl AvPlayers {
    pub(crate) fn new(main_thread: MainThreadMarker) -> AvPlayers {
        AvPlayers { main_thread }
    }
}

impl Players for AvPlayers {
    fn open(&self, url: &str) -> Box<dyn MediaPlayer> {
        match url_of(url) {
            Ok(url) => Box::new(AvMediaPlayer::new(self.main_thread, &url)),
            Err(reason) => Box::new(FailedPlayer {
                error: PlayerError {
                    kind: PlayerErrorKind::Unsupported,
                    message: reason,
                },
                changes: Changes::default(),
            }),
        }
    }
}

/// `url` as AVFoundation takes it, or why it is none.
fn url_of(url: &str) -> Result<Retained<NSURL>, String> {
    NSURL::URLWithString(&NSString::from_str(url)).ok_or_else(|| format!("`{url}` is not a URL"))
}

/// A player over one item.
struct AvMediaPlayer {
    player: Retained<AVPlayer>,
    item: Retained<AVPlayerItem>,
    /// What tells the app the state changed: the observations below, while they last.
    changes: Changes,
    /// The rate `play` plays at.
    rate: Cell<f32>,
    /// Shared with the end-of-item notification's handler.
    end: Rc<End>,
    /// Why it paused last, as the player's own pause or AVFoundation's rate notification says;
    /// `None` once it played, until it is told why it stopped.
    pause_reason: Rc<Cell<Option<PauseReason>>>,
    /// The seek under way, which each seek's completion handler ends.
    seek: Arc<SeekUnderWay>,
    /// Why it failed partway through, as its failure to play to the end said.
    failure: Rc<RefCell<Option<PlayerError>>>,
    /// Where the picture goes now.
    destination: RefCell<Destination>,
    tracks: MediaTracks,
    metadata: AssetMetadata,
    /// The subtitles, which its item's legible output hands over.
    cues: LegibleCues,
    _properties: [PropertyObservation; 2],
    _notifications: NotificationObservation,
    _item_changes: NotificationObservation,
    _rate_changes: NotificationObservation,
}

/// What the end of the item does, and whether it was reached.
struct End {
    looping: Cell<bool>,
    reached: Cell<bool>,
}

/// Where a player's picture goes.
enum Destination {
    Nowhere,
    /// Its layer is this composition layer's content, which the player holds as a producer
    /// does.
    Layer {
        layer: Retained<AVPlayerLayer>,
        _shown_on: Arc<dyn CompositionLayer>,
    },
    /// Its item's video output, which this reader reads.
    Frames {
        reader: Arc<AvFrameReader>,
        output: Arc<PlayerFrames>,
    },
}

impl AvMediaPlayer {
    fn new(main_thread: MainThreadMarker, url: &NSURL) -> AvMediaPlayer {
        // SAFETY: an asset of the URL, and a player item of it and a player of that, made on the
        // main thread.
        let (asset, item, player) = unsafe {
            let asset = AVURLAsset::URLAssetWithURL_options(url, None);
            let item = AVPlayerItem::playerItemWithAsset(&asset, main_thread);
            let player = AVPlayer::playerWithPlayerItem(Some(&item), main_thread);
            (asset, item, player)
        };
        let changes = Changes::default();
        let end = Rc::new(End {
            looping: Cell::new(false),
            reached: Cell::new(false),
        });
        let properties = [
            PropertyObservation::new(
                &item,
                &[
                    "status",
                    "duration",
                    "presentationSize",
                    "tracks",
                    "loadedTimeRanges",
                    "seekableTimeRanges",
                ],
                changes.clone(),
            ),
            PropertyObservation::new(
                &player,
                &[
                    "timeControlStatus",
                    "reasonForWaitingToPlay",
                    "rate",
                    "volume",
                    "muted",
                ],
                changes.clone(),
            ),
        ];
        let failure = Rc::new(RefCell::new(None));
        let notifications = observe_end(
            &player,
            &item,
            Rc::clone(&end),
            Rc::clone(&failure),
            changes.clone(),
        );
        let item_changes = observe_selection_and_jumps(&item, changes.clone());
        let pause_reason = Rc::new(Cell::new(Some(PauseReason::Requested)));
        let rate_changes = observe_rate(&player, Rc::clone(&pause_reason), changes.clone());
        let cues = LegibleCues::new(changes.clone());
        // SAFETY: adding an output to the item on the main thread.
        unsafe { item.addOutput(cues.output()) };
        AvMediaPlayer {
            tracks: MediaTracks::load(&asset, &changes),
            metadata: AssetMetadata::load(&asset, changes.clone()),
            cues,
            player,
            item,
            changes,
            rate: Cell::new(1.0),
            end,
            pause_reason,
            seek: Arc::default(),
            failure,
            destination: RefCell::new(Destination::Nowhere),
            _properties: properties,
            _notifications: notifications,
            _item_changes: item_changes,
            _rate_changes: rate_changes,
        }
    }

    /// The item's status, failed too once it failed to play to its end.
    fn status(&self) -> PlayerStatus {
        // SAFETY: reading the item's status on the main thread.
        let status = unsafe { self.item.status() };
        if status == AVPlayerItemStatus::Failed {
            // SAFETY: as above.
            return PlayerStatus::Failed(error_of(unsafe { self.item.error() }.as_deref()));
        }
        if let Some(failure) = self.failure.borrow().clone() {
            return PlayerStatus::Failed(failure);
        }
        if status == AVPlayerItemStatus::ReadyToPlay {
            PlayerStatus::Ready
        } else {
            PlayerStatus::Loading
        }
    }

    /// What the player does: paused at the end once it reached it, as AVFoundation tells the end
    /// before it pauses there, and paused with no reason once it failed.
    fn activity(&self, status: &PlayerStatus) -> PlayerActivity {
        if matches!(status, PlayerStatus::Failed(_)) {
            return PlayerActivity::Paused(None);
        }
        if self.end.reached.get() {
            return PlayerActivity::Paused(Some(PauseReason::End));
        }
        // SAFETY: reading the player on the main thread.
        let time_control = unsafe { self.player.timeControlStatus() };
        if time_control == AVPlayerTimeControlStatus::Playing {
            PlayerActivity::Playing
        } else if time_control == AVPlayerTimeControlStatus::WaitingToPlayAtSpecifiedRate {
            PlayerActivity::Waiting(self.wait_reason())
        } else {
            PlayerActivity::Paused(self.pause_reason.get())
        }
    }

    /// Why AVFoundation waits to play.
    fn wait_reason(&self) -> Option<WaitReason> {
        // SAFETY: reading the player on the main thread, and constants AVFoundation exports for
        // the life of the process.
        unsafe {
            let reason = self.player.reasonForWaitingToPlay()?;
            if &*reason == AVPlayerWaitingToMinimizeStallsReason {
                Some(WaitReason::Data)
            } else if &*reason == AVPlayerWaitingWhileEvaluatingBufferingRateReason {
                Some(WaitReason::EvaluatingData)
            } else {
                None
            }
        }
    }

    /// The rate it plays at while it plays: AVFoundation's while it plays or waits to, which it
    /// may have changed, and the one `play` plays at while it is paused.
    fn playing_rate(&self, activity: PlayerActivity) -> f64 {
        match activity {
            // SAFETY: reading the rate on the main thread.
            PlayerActivity::Playing | PlayerActivity::Waiting(_) => {
                f64::from(unsafe { self.player.rate() })
            }
            PlayerActivity::Paused(_) => f64::from(self.rate.get()),
        }
    }

    /// The picture on a layer of the player's own, which is `shown_on`'s content.
    fn show_on(&self, shown_on: Arc<dyn CompositionLayer>) -> Destination {
        // SAFETY: a layer of the player, made on the main thread.
        let layer = unsafe { AVPlayerLayer::playerLayerWithPlayer(Some(&self.player)) };
        // SAFETY: a constant AVFoundation exports for the life of the process.
        if let Some(gravity) = unsafe { AVLayerVideoGravityResizeAspect } {
            unsafe { layer.setVideoGravity(gravity) };
        }
        show_system_layer(&shown_on, &layer);
        Destination::Layer {
            layer,
            _shown_on: shown_on,
        }
    }

    /// The picture to a video output added to the item, which `reader` reads; nowhere for a
    /// reader this host did not make, or one another source feeds.
    fn feed(&self, reader: Arc<dyn VideoFrameReader>) -> Destination {
        let Some(reader) = AvFrameReader::downcast_from(reader) else {
            report_foreign_reader();
            return Destination::Nowhere;
        };
        let output = Arc::new(PlayerFrames::new());
        if !reader.read(Arc::clone(&output) as Arc<dyn FrameOutput>) {
            report_reader_in_use();
            return Destination::Nowhere;
        }
        // SAFETY: adding an output to the item on the main thread.
        unsafe { self.item.addOutput(output.output()) };
        Destination::Frames { reader, output }
    }

    /// Lets go of where the picture went: its layer leaves the composition layer it was shown on,
    /// and its reader reads nothing more from it.
    fn leave_destination(&self) {
        match self.destination.replace(Destination::Nowhere) {
            Destination::Nowhere => {}
            Destination::Layer { layer, .. } => {
                // SAFETY: on the main thread, where the layer was shown.
                unsafe { layer.setPlayer(None) };
                super::in_transaction(|| layer.removeFromSuperlayer());
            }
            Destination::Frames { reader, output } => {
                // SAFETY: removing the item's output on the main thread.
                unsafe { self.item.removeOutput(output.output()) };
                reader.stop_reading(&(output as Arc<dyn FrameOutput>));
            }
        }
    }
}

impl MediaPlayer for AvMediaPlayer {
    fn play(&self) {
        if self.end.reached.replace(false) {
            // SAFETY: seeking on the main thread.
            unsafe { self.player.seekToTime(kCMTimeZero) };
        }
        self.pause_reason.set(None);
        // SAFETY: playing on the main thread.
        unsafe { self.player.setRate(self.rate.get()) };
        self.changes.notify();
    }

    fn pause(&self) {
        self.pause_reason.set(Some(PauseReason::Requested));
        // SAFETY: pausing on the main thread.
        unsafe { self.player.pause() };
        self.changes.notify();
    }

    fn seek(&self, position: Duration, tolerance: SeekTolerance) {
        self.end.reached.set(false);
        let number = self.seek.start(position);
        let seek = Arc::clone(&self.seek);
        let changes = self.changes.clone();
        let landed = RcBlock::new(move |_finished: Bool| {
            if seek.land(number) {
                changes.notify();
            }
        });
        // SAFETY: a seek on the main thread. AVFoundation may call its handler on any thread,
        // and the handler holds only what any thread may use.
        unsafe {
            self.player
                .seekToTime_toleranceBefore_toleranceAfter_completionHandler(
                    cm_time_of(position),
                    tolerance_time(tolerance.before),
                    tolerance_time(tolerance.after),
                    &landed,
                )
        };
        self.changes.notify();
    }

    fn set_rate(&self, rate: f64) {
        self.rate.set(rate as f32);
        // SAFETY: reading and changing the rate on the main thread.
        unsafe {
            if self.player.timeControlStatus() != AVPlayerTimeControlStatus::Paused {
                self.player.setRate(rate as f32);
            }
        }
    }

    fn set_volume(&self, volume: f64) {
        // SAFETY: on the main thread.
        unsafe { self.player.setVolume(volume.clamp(0.0, 1.0) as f32) };
    }

    fn set_muted(&self, muted: bool) {
        // SAFETY: on the main thread.
        unsafe { self.player.setMuted(muted) };
    }

    fn set_looping(&self, looping: bool) {
        self.end.looping.set(looping);
        let action = if looping {
            AVPlayerActionAtItemEnd::None
        } else {
            AVPlayerActionAtItemEnd::Pause
        };
        // SAFETY: on the main thread.
        unsafe { self.player.setActionAtItemEnd(action) };
    }

    fn state(&self) -> PlayerState {
        let status = self.status();
        let activity = self.activity(&status);
        // SAFETY: reading the player and its item on the main thread.
        let (volume, muted, duration, size, loaded, seekable) = unsafe {
            (
                self.player.volume(),
                self.player.isMuted(),
                self.item.duration(),
                self.item.presentationSize(),
                self.item.loadedTimeRanges(),
                self.item.seekableTimeRanges(),
            )
        };
        PlayerState {
            status,
            activity,
            seeking: self.seek.target(),
            rate: self.playing_rate(activity),
            volume: f64::from(volume),
            muted,
            duration: duration_of(duration),
            loaded: ranges_of(&loaded),
            seekable: ranges_of(&seekable),
            size: (size.width > 0.0 && size.height > 0.0)
                .then(|| Size::new(size.width, size.height)),
            tracks: self.tracks.tracks(&self.item),
        }
    }

    /// AVPlayer's `currentTime`, or the target while a seek is under way.
    fn position(&self) -> Duration {
        if let Some(target) = self.seek.target() {
            return target;
        }
        // SAFETY: reading the player's time on the main thread.
        duration_of(unsafe { self.player.currentTime() }).unwrap_or_default()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.changes.next()
    }

    fn set_video_output(&self, output: Option<VideoOutput>) {
        self.leave_destination();
        let destination = match output {
            None => Destination::Nowhere,
            Some(VideoOutput::Layer(layer)) => self.show_on(layer),
            Some(VideoOutput::Frames(reader)) => self.feed(reader),
        };
        *self.destination.borrow_mut() = destination;
    }

    fn select_audio_track(&self, id: &str) {
        self.tracks.select_audio(&self.item, id);
        self.changes.notify();
    }

    /// The cues of the text track shown before go with it.
    fn select_text_track(&self, id: Option<&str>) {
        self.tracks.select_text(&self.item, id);
        self.cues.clear();
        self.changes.notify();
    }

    fn metadata(&self) -> MediaMetadata {
        self.metadata.get()
    }

    fn cues(&self) -> Vec<Cue> {
        self.cues.cues()
    }
}

impl Drop for AvMediaPlayer {
    /// The player stops, its picture leaves where it went, and the app hears it gone.
    fn drop(&mut self) {
        // SAFETY: on the main thread, where the player is controlled.
        unsafe { self.player.pause() };
        self.leave_destination();
        self.changes.end();
    }
}

/// Hears the end of `item`: a looping player starts again, and one that does not has ended. A
/// failure to reach the end is kept in `failure`, with the error AVFoundation gave.
fn observe_end(
    player: &Retained<AVPlayer>,
    item: &AVPlayerItem,
    end: Rc<End>,
    failure: Rc<RefCell<Option<PlayerError>>>,
    changes: Changes,
) -> NotificationObservation {
    let player = player.clone();
    // SAFETY: constants AVFoundation exports for the life of the process.
    let (reached, failed) = unsafe {
        (
            AVPlayerItemDidPlayToEndTimeNotification,
            AVPlayerItemFailedToPlayToEndTimeNotification,
        )
    };
    NotificationObservation::new(item, &[reached, failed], move |notification| {
        if &*notification.name() == reached {
            if end.looping.get() {
                // SAFETY: the main queue runs the handler on the main thread.
                unsafe { player.seekToTime(kCMTimeZero) };
            } else {
                end.reached.set(true);
            }
        } else {
            *failure.borrow_mut() = Some(error_of(failure_to_end(notification).as_deref()));
        }
        changes.notify();
    })
}

/// The error a failure to play to the end carries.
fn failure_to_end(notification: &NSNotification) -> Option<Retained<NSError>> {
    // SAFETY: a constant AVFoundation exports for the life of the process.
    let key = unsafe { AVPlayerItemFailedToPlayToEndTimeErrorKey };
    notification
        .userInfo()
        .and_then(|info| info.objectForKey(key))
        .and_then(|error| error.downcast::<NSError>().ok())
}

/// Hears the player's rate change, from macOS 12 and iOS 15, and keeps in `pause_reason` why it
/// fell to zero, when it did. The notification and its keys are named by their text, which is
/// their value, so a system before them, which posts none, still starts the app.
fn observe_rate(
    player: &Retained<AVPlayer>,
    pause_reason: Rc<Cell<Option<PauseReason>>>,
    changes: Changes,
) -> NotificationObservation {
    let rate_changed = NSString::from_str("AVPlayerRateDidChangeNotification");
    let paused = player.clone();
    NotificationObservation::new(player, &[&rate_changed], move |notification| {
        // SAFETY: the main queue runs the handler on the main thread.
        if unsafe { paused.rate() } == 0.0 {
            pause_reason.set(pause_reason_of(notification));
        }
        changes.notify();
    })
}

/// Why AVFoundation's rate fell to zero, from its notification's reason.
fn pause_reason_of(notification: &NSNotification) -> Option<PauseReason> {
    let key = NSString::from_str("AVPlayerRateDidChangeReasonKey");
    let reason = notification
        .userInfo()?
        .objectForKey(&key)?
        .downcast::<NSString>()
        .ok()?
        .to_string();
    match reason.as_str() {
        "AVPlayerRateDidChangeReasonSetRateCalled" => Some(PauseReason::Requested),
        "AVPlayerRateDidChangeReasonAudioSessionInterrupted" => Some(PauseReason::Interrupted),
        "AVPlayerRateDidChangeReasonAppBackgrounded" => Some(PauseReason::Backgrounded),
        _ => None,
    }
}

/// The seek under way, shared with each seek's completion handler, which AVFoundation calls on a
/// thread of its own; a later seek replaces the target, and only its own completion ends it.
#[derive(Default)]
struct SeekUnderWay(Mutex<LatestSeek>);

#[derive(Default)]
struct LatestSeek {
    /// Counts the seeks started.
    number: u64,
    /// Where the last of them goes, until it lands.
    target: Option<Duration>,
}

impl SeekUnderWay {
    /// A seek to `target` starts, replacing any under way; answers its number.
    fn start(&self, target: Duration) -> u64 {
        let mut latest = lock(&self.0);
        latest.number += 1;
        latest.target = Some(target);
        latest.number
    }

    /// Seek `number` landed, or was given up; answers whether it was the one under way, which no
    /// longer is.
    fn land(&self, number: u64) -> bool {
        let mut latest = lock(&self.0);
        if latest.number != number {
            return false;
        }
        latest.target = None;
        true
    }

    fn target(&self) -> Option<Duration> {
        lock(&self.0).target
    }
}

/// `error` as a player's failure: its kind, and AVFoundation's words for it.
fn error_of(error: Option<&NSError>) -> PlayerError {
    match error {
        Some(error) => PlayerError {
            kind: kind_of(error),
            message: error.localizedDescription().to_string(),
        },
        None => PlayerError {
            kind: PlayerErrorKind::Other,
            message: "the system could not play the media".to_owned(),
        },
    }
}

/// The kind of `error`: by its own domain and code, or, where they say nothing more, by the error
/// it wraps, as AVFoundation wraps a connection's error.
fn kind_of(error: &NSError) -> PlayerErrorKind {
    let domain = error.domain().to_string();
    let kind = match domain.as_str() {
        "NSURLErrorDomain" => Some(url_error_kind(error.code())),
        "AVFoundationErrorDomain" => av_error_kind(AVError(error.code())),
        _ => None,
    };
    kind.or_else(|| underlying_error(error).map(|cause| kind_of(&cause)))
        .unwrap_or(PlayerErrorKind::Other)
}

/// The error `error` wraps.
fn underlying_error(error: &NSError) -> Option<Retained<NSError>> {
    // SAFETY: a constant Foundation exports for the life of the process.
    let key = unsafe { NSUnderlyingErrorKey };
    error
        .userInfo()
        .objectForKey(key)
        .and_then(|cause| cause.downcast::<NSError>().ok())
}

/// The kind of a loading system's error: refused where the system does not let the player read
/// a file or fetch a link insecurely (`NSURLErrorNoPermissionsToReadFile`,
/// `NSURLErrorAppTransportSecurityRequiresSecureConnection`), reaching the media otherwise.
fn url_error_kind(code: isize) -> PlayerErrorKind {
    const NO_PERMISSIONS_TO_READ_FILE: isize = -1102;
    const APP_TRANSPORT_SECURITY_REQUIRES_SECURE_CONNECTION: isize = -1022;
    match code {
        NO_PERMISSIONS_TO_READ_FILE | APP_TRANSPORT_SECURITY_REQUIRES_SECURE_CONNECTION => {
            PlayerErrorKind::Refused
        }
        _ => PlayerErrorKind::Network,
    }
}

/// The kind of an AVFoundation error, or `None` for one that says no more than that something
/// failed.
fn av_error_kind(error: AVError) -> Option<PlayerErrorKind> {
    match error {
        AVError::DecodeFailed | AVError::FailedToParse | AVError::UndecodableMediaData => {
            Some(PlayerErrorKind::Decode)
        }
        AVError::FileFormatNotRecognized
        | AVError::FileFailedToParse
        | AVError::DecoderNotFound
        | AVError::FormatUnsupported
        | AVError::IncompatibleAsset => Some(PlayerErrorKind::Unsupported),
        AVError::ContentIsProtected | AVError::ContentIsNotAuthorized => {
            Some(PlayerErrorKind::Protected)
        }
        AVError::FailedToLoadMediaData | AVError::ServerIncorrectlyConfigured => {
            Some(PlayerErrorKind::Network)
        }
        _ => None,
    }
}

/// `ranges` as spans of the media, leaving out any whose start or length is not a time.
fn ranges_of(ranges: &NSArray<NSValue>) -> Vec<TimeRange> {
    ranges
        .iter()
        .filter_map(|range| {
            // SAFETY: AVFoundation's time ranges are values of `CMTimeRange`.
            let range = unsafe { range.CMTimeRangeValue() };
            let start = duration_of(range.start)?;
            let length = duration_of(range.duration)?;
            Some(TimeRange {
                start,
                end: start + length,
            })
        })
        .collect()
}

/// A seek's tolerance on one side as AVFoundation takes it: infinite where it is unbounded.
fn tolerance_time(tolerance: Duration) -> CMTime {
    if tolerance == Duration::MAX {
        // SAFETY: a constant Core Media exports for the life of the process.
        unsafe { kCMTimePositiveInfinity }
    } else {
        cm_time_of(tolerance)
    }
}

/// Tells `changes` whenever a media selection group of `item` changes its selected option, and
/// whenever its time jumps, as a seek landing or a loop makes it (`AVPlayerItemTimeJumped`).
fn observe_selection_and_jumps(item: &AVPlayerItem, changes: Changes) -> NotificationObservation {
    // SAFETY: constants AVFoundation exports for the life of the process.
    let names = unsafe {
        [
            AVPlayerItemMediaSelectionDidChangeNotification,
            AVPlayerItemTimeJumpedNotification,
        ]
    };
    NotificationObservation::new(item, &names, move |_notification| changes.notify())
}

/// `time` as a duration; `None` for an invalid or indefinite time, as a live stream's duration
/// is.
fn duration_of(time: CMTime) -> Option<Duration> {
    let usable =
        time.flags.contains(CMTimeFlags::Valid) && !time.flags.contains(CMTimeFlags::Indefinite);
    if !usable || time.timescale <= 0 {
        return None;
    }
    Duration::try_from_secs_f64(time.value as f64 / f64::from(time.timescale)).ok()
}

fn cm_time_of(position: Duration) -> CMTime {
    // SAFETY: a plain value constructor.
    unsafe { CMTime::with_seconds(position.as_secs_f64(), NANOSECONDS) }
}

/// A player of a source the system cannot play: failed from the start.
struct FailedPlayer {
    error: PlayerError,
    changes: Changes,
}

impl MediaPlayer for FailedPlayer {
    fn play(&self) {}

    fn pause(&self) {}

    fn seek(&self, _position: Duration, _tolerance: SeekTolerance) {}

    fn set_rate(&self, _rate: f64) {}

    fn set_volume(&self, _volume: f64) {}

    fn set_muted(&self, _muted: bool) {}

    fn set_looping(&self, _looping: bool) {}

    fn state(&self) -> PlayerState {
        PlayerState::failed(self.error.clone())
    }

    fn position(&self) -> Duration {
        Duration::ZERO
    }

    fn changed(&self) -> MediaChangeFuture {
        self.changes.next()
    }

    fn set_video_output(&self, _output: Option<VideoOutput>) {}

    fn select_audio_track(&self, _id: &str) {}

    fn select_text_track(&self, _id: Option<&str>) {}

    fn metadata(&self) -> MediaMetadata {
        MediaMetadata::default()
    }

    fn cues(&self) -> Vec<Cue> {
        Vec::new()
    }
}

impl Drop for FailedPlayer {
    fn drop(&mut self) {
        self.changes.end();
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::PickedFile;

    use super::*;

    #[test]
    fn a_time_is_a_duration_only_while_it_is_valid_and_definite() {
        let time = |value, flags| CMTime {
            value,
            timescale: 600,
            flags,
            epoch: 0,
        };
        assert_eq!(
            duration_of(time(900, CMTimeFlags::Valid)),
            Some(Duration::from_millis(1500))
        );
        assert_eq!(duration_of(time(900, CMTimeFlags::empty())), None);
        assert_eq!(
            duration_of(time(0, CMTimeFlags::Valid | CMTimeFlags::Indefinite)),
            None
        );
    }

    /// An error of `domain` and `code`, wrapping `cause`.
    fn error(domain: &str, code: isize, cause: Option<&NSError>) -> Retained<NSError> {
        let domain = NSString::from_str(domain);
        let info = cause.map(|cause| {
            // SAFETY: a constant Foundation exports for the life of the process.
            let key = unsafe { NSUnderlyingErrorKey };
            let cause: &objc2::runtime::AnyObject = cause.as_ref();
            objc2_foundation::NSDictionary::from_slices(&[key], &[cause])
        });
        // SAFETY: a plain error, with plain values.
        unsafe { NSError::errorWithDomain_code_userInfo(&domain, code, info.as_deref()) }
    }

    #[test]
    fn a_failure_has_the_kind_of_its_error_or_of_the_error_it_wraps() {
        let offline = error("NSURLErrorDomain", -1009, None);
        assert_eq!(kind_of(&offline), PlayerErrorKind::Network);
        let unreadable = error("NSURLErrorDomain", -1102, None);
        assert_eq!(kind_of(&unreadable), PlayerErrorKind::Refused);
        let unauthorized = error(
            "AVFoundationErrorDomain",
            AVError::ContentIsNotAuthorized.0,
            None,
        );
        assert_eq!(kind_of(&unauthorized), PlayerErrorKind::Protected);
        let wrapped = error(
            "AVFoundationErrorDomain",
            AVError::Unknown.0,
            Some(&offline),
        );
        assert_eq!(
            kind_of(&wrapped),
            PlayerErrorKind::Network,
            "the cause's kind"
        );
        let unexplained = error("NSOSStatusErrorDomain", -12848, None);
        assert_eq!(kind_of(&unexplained), PlayerErrorKind::Other);
    }

    #[test]
    fn a_link_and_a_picked_files_url_are_urls_avfoundation_takes() {
        assert!(url_of("https://example.com/a.mp4").is_ok());
        let picked = PickedFile::Path("/tmp/a b/été.mov".into());
        let url = url_of(&picked.url()).unwrap();
        assert!(url.isFileURL());
        assert_eq!(
            url.path().map(|path| path.to_string()).as_deref(),
            Some("/tmp/a b/été.mov")
        );
    }
}
