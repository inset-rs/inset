//! A player: AVFoundation's `AVPlayer` over one `AVPlayerItem`, controlled on the main thread,
//! whose state is read from them when asked and whose changes AVFoundation tells by key-value
//! observing, a periodic time observer and its end-of-item notification. Its picture goes either
//! to an `AVPlayerLayer` hung in a surface, or to an `AVPlayerItemVideoOutput` a frame reader
//! reads.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use block2::RcBlock;
use inset_embedder::{
    FrameReader, MediaChangeFuture, MediaPlayer, PlayerState, PlayerStatus, Players, Size, Surface,
};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_av_foundation::{
    AVLayerVideoGravityResizeAspect, AVPlayer, AVPlayerActionAtItemEnd, AVPlayerItem,
    AVPlayerItemDidPlayToEndTimeNotification, AVPlayerItemFailedToPlayToEndTimeNotification,
    AVPlayerItemStatus, AVPlayerLayer, AVPlayerTimeControlStatus,
};
use objc2_core_media::{CMTime, CMTimeFlags, kCMTimeZero};
use objc2_foundation::{NSString, NSURL};

use super::frames::PlayerFrames;
use super::observing::{NotificationObservation, PropertyObservation};
use super::show_layer_on;
use crate::media::Changes;

/// How often a playing player tells its position: every half second, as Flutter's
/// `video_player` polls it.
const POSITION_INTERVAL_SECONDS: f64 = 0.5;
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
                reason,
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
    /// Where the picture goes now.
    destination: RefCell<Destination>,
    _properties: [PropertyObservation; 2],
    _notifications: NotificationObservation,
    /// The periodic time observer's token, which `Drop` removes.
    time_observer: Retained<AnyObject>,
}

/// What the end of the item does, and whether it was reached.
struct End {
    looping: Cell<bool>,
    reached: Cell<bool>,
}

/// Where a player's picture goes.
enum Destination {
    Nowhere,
    /// Its layer hangs in this surface, which the player holds as a producer does.
    Surface {
        layer: Retained<AVPlayerLayer>,
        _surface: Arc<dyn Surface>,
    },
    Frames(Arc<PlayerFrames>),
}

impl AvMediaPlayer {
    fn new(main_thread: MainThreadMarker, url: &NSURL) -> AvMediaPlayer {
        // SAFETY: a player item and a player of it, made on the main thread.
        let item = unsafe { AVPlayerItem::playerItemWithURL(url, main_thread) };
        let player = unsafe { AVPlayer::playerWithPlayerItem(Some(&item), main_thread) };
        let changes = Changes::default();
        let end = Rc::new(End {
            looping: Cell::new(false),
            reached: Cell::new(false),
        });
        let properties = [
            PropertyObservation::new(
                &item,
                &["status", "duration", "presentationSize"],
                changes.clone(),
            ),
            PropertyObservation::new(&player, &["timeControlStatus", "rate"], changes.clone()),
        ];
        let notifications = observe_end(&player, &item, Rc::clone(&end), changes.clone());
        let time_observer = observe_time(&player, changes.clone());
        AvMediaPlayer {
            player,
            item,
            changes,
            rate: Cell::new(1.0),
            end,
            destination: RefCell::new(Destination::Nowhere),
            _properties: properties,
            _notifications: notifications,
            time_observer,
        }
    }

    fn status(&self) -> PlayerStatus {
        // SAFETY: reading the item's status on the main thread.
        let status = unsafe { self.item.status() };
        if status == AVPlayerItemStatus::ReadyToPlay {
            PlayerStatus::Ready
        } else if status == AVPlayerItemStatus::Failed {
            // SAFETY: as above.
            let reason = unsafe { self.item.error() }
                .map(|error| error.localizedDescription().to_string())
                .unwrap_or_else(|| "the system could not play the media".to_owned());
            PlayerStatus::Failed(reason)
        } else {
            PlayerStatus::Loading
        }
    }

    /// Lets go of where the picture went: its layer leaves its surface, and its frame reader
    /// reads nothing more.
    fn leave_destination(&self) {
        match self.destination.replace(Destination::Nowhere) {
            Destination::Nowhere => {}
            Destination::Surface { layer, .. } => {
                // SAFETY: on the main thread, where the layer was hung.
                unsafe { layer.setPlayer(None) };
                super::in_transaction(|| layer.removeFromSuperlayer());
            }
            // SAFETY: removing the item's output on the main thread.
            Destination::Frames(frames) => unsafe { self.item.removeOutput(frames.output()) },
        }
    }
}

impl MediaPlayer for AvMediaPlayer {
    fn play(&self) {
        if self.end.reached.replace(false) {
            // SAFETY: seeking on the main thread.
            unsafe { self.player.seekToTime(kCMTimeZero) };
        }
        // SAFETY: playing on the main thread.
        unsafe { self.player.setRate(self.rate.get()) };
        self.changes.notify();
    }

    fn pause(&self) {
        // SAFETY: pausing on the main thread.
        unsafe { self.player.pause() };
    }

    fn seek(&self, position: Duration) {
        self.end.reached.set(false);
        let time = cm_time_of(position);
        // SAFETY: an exact seek on the main thread.
        unsafe {
            self.player
                .seekToTime_toleranceBefore_toleranceAfter(time, kCMTimeZero, kCMTimeZero)
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
        // SAFETY: reading the player and its item on the main thread.
        let (playing, position, duration, size) = unsafe {
            (
                self.player.timeControlStatus() != AVPlayerTimeControlStatus::Paused,
                self.player.currentTime(),
                self.item.duration(),
                self.item.presentationSize(),
            )
        };
        let ended = self.end.reached.get();
        PlayerState {
            status: self.status(),
            // AVFoundation tells the end before it pauses there.
            playing: playing && !ended,
            position: duration_of(position).unwrap_or_default(),
            duration: duration_of(duration),
            size: (size.width > 0.0 && size.height > 0.0)
                .then(|| Size::new(size.width, size.height)),
            ended,
        }
    }

    fn changed(&self) -> MediaChangeFuture {
        self.changes.next()
    }

    fn show_on(&self, surface: Arc<dyn Surface>) {
        self.leave_destination();
        // SAFETY: a layer of the player, made on the main thread.
        let layer = unsafe { AVPlayerLayer::playerLayerWithPlayer(Some(&self.player)) };
        // SAFETY: a constant AVFoundation exports for the life of the process.
        if let Some(gravity) = unsafe { AVLayerVideoGravityResizeAspect } {
            unsafe { layer.setVideoGravity(gravity) };
        }
        show_layer_on(&surface, &layer);
        *self.destination.borrow_mut() = Destination::Surface {
            layer,
            _surface: surface,
        };
    }

    fn read_frames(&self) -> Arc<dyn FrameReader> {
        self.leave_destination();
        let frames = Arc::new(PlayerFrames::new());
        // SAFETY: adding an output to the item on the main thread.
        unsafe { self.item.addOutput(frames.output()) };
        *self.destination.borrow_mut() = Destination::Frames(Arc::clone(&frames));
        frames
    }
}

impl Drop for AvMediaPlayer {
    /// The player stops, its picture leaves where it went, and the app hears it gone.
    fn drop(&mut self) {
        // SAFETY: on the main thread, where the player is controlled.
        unsafe {
            self.player.pause();
            self.player.removeTimeObserver(&self.time_observer);
        }
        self.leave_destination();
        self.changes.end();
    }
}

/// Hears the end of `item`: a looping player starts again, and one that does not has ended. A
/// failure to reach the end is a change of the item's error, which the state reads.
fn observe_end(
    player: &Retained<AVPlayer>,
    item: &AVPlayerItem,
    end: Rc<End>,
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
        }
        changes.notify();
    })
}

/// Tells `changes` every half second while `player` plays, and whenever its time jumps, it
/// starts or it stops; answers the observer's token.
fn observe_time(player: &AVPlayer, changes: Changes) -> Retained<AnyObject> {
    let block = RcBlock::new(move |_time: CMTime| changes.notify());
    // SAFETY: the main queue (`None`) calls the block, which only tells the changes; the
    // observer is removed in the player's `Drop`.
    unsafe {
        player.addPeriodicTimeObserverForInterval_queue_usingBlock(
            CMTime::with_seconds(POSITION_INTERVAL_SECONDS, NANOSECONDS),
            None,
            &block,
        )
    }
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
    reason: String,
    changes: Changes,
}

impl MediaPlayer for FailedPlayer {
    fn play(&self) {}

    fn pause(&self) {}

    fn seek(&self, _position: Duration) {}

    fn set_rate(&self, _rate: f64) {}

    fn set_volume(&self, _volume: f64) {}

    fn set_looping(&self, _looping: bool) {}

    fn state(&self) -> PlayerState {
        PlayerState::failed(self.reason.clone())
    }

    fn changed(&self) -> MediaChangeFuture {
        self.changes.next()
    }

    fn show_on(&self, _surface: Arc<dyn Surface>) {}

    fn read_frames(&self) -> Arc<dyn FrameReader> {
        Arc::new(super::frames::NoFrames)
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
