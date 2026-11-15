//! Media read without playing it: AVFoundation's `AVURLAsset`, opened on the URL a player takes.
//! Its length, its tracks and its tags load in the background
//! (`loadValuesAsynchronouslyForKeys:completionHandler:`), and are read on the main thread once
//! AVFoundation says they have: its tracks as a player lists them (`tracks`), its tags as a
//! player's metadata (`metadata`); its pictures come from an image generator (`asset_frames`).
//!
//! The invariant: nothing here reads a value of the asset before AVFoundation has loaded it, so no
//! read blocks the main thread; and each answer is given once, a failure where AVFoundation could
//! not load what it asks.

use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use block2::RcBlock;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2_av_foundation::{AVAsynchronousKeyValueLoading, AVKeyValueStatus, AVURLAsset};
use objc2_foundation::{NSArray, NSError, NSString};
use waft_embedder::{
    MediaAsset, MediaAssetFuture, MediaAssets, MediaMetadata, MediaTrack, PlayerError,
    PlayerErrorKind, SeekTolerance, Size, VideoFrame,
};

use super::asset_frames::frame_at;
use super::media_times::duration_of;
use super::metadata::media_metadata_of;
use super::player::url_of;
use super::player_errors::error_of;
use super::tracks::asset_tracks;
use crate::answer::answer;

/// AVFoundation's reading of media.
pub(crate) struct AvMediaAssets;

impl MediaAssets for AvMediaAssets {
    fn open(&self, url: &str) -> Box<dyn MediaAsset> {
        match url_of(url) {
            // SAFETY: an asset of the URL, which AVFoundation reads from any thread.
            Ok(url) => Box::new(AvMediaAsset(unsafe {
                AVURLAsset::URLAssetWithURL_options(&url, None)
            })),
            Err(reason) => Box::new(UnreadableAsset(PlayerError {
                kind: PlayerErrorKind::Unsupported,
                message: reason,
            })),
        }
    }
}

/// An asset AVFoundation reads.
struct AvMediaAsset(Retained<AVURLAsset>);

impl MediaAsset for AvMediaAsset {
    /// None for a live stream, whose length AVFoundation calls indefinite.
    fn duration(&self) -> MediaAssetFuture<Option<Duration>> {
        let asset = self.0.clone();
        let loading = loaded(&asset, "duration");
        Box::pin(async move {
            loading.await?;
            // SAFETY: the asset's length, which has loaded.
            Ok(duration_of(unsafe { asset.duration() }))
        })
    }

    fn tracks(&self) -> MediaAssetFuture<Vec<MediaTrack>> {
        let asset = self.0.clone();
        let loading = loaded(&asset, "tracks");
        Box::pin(async move {
            loading.await?;
            Ok(asset_tracks(&asset).await)
        })
    }

    fn metadata(&self) -> MediaAssetFuture<MediaMetadata> {
        let asset = self.0.clone();
        let loading = loaded(&asset, "commonMetadata");
        Box::pin(async move {
            loading.await?;
            // SAFETY: the asset's common metadata, which has loaded.
            Ok(media_metadata_of(&*unsafe { asset.commonMetadata() }))
        })
    }

    fn frame_at(
        &self,
        time: Duration,
        tolerance: SeekTolerance,
        max_size: Option<Size>,
    ) -> MediaAssetFuture<Arc<dyn VideoFrame>> {
        frame_at(&self.0, time, tolerance, max_size)
    }
}

/// Resolves once AVFoundation has loaded `asset`'s value of `key`, or failed to, with why.
fn loaded(
    asset: &Retained<AVURLAsset>,
    key: &'static str,
) -> impl Future<Output = Result<(), PlayerError>> + use<> {
    let (given, answered) = answer(());
    let given = Mutex::new(Some(given));
    let handler = RcBlock::new(move || {
        let given = given.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(given) = given {
            given.give(());
        }
    });
    let keys = NSArray::from_retained_slice(&[NSString::from_str(key)]);
    // SAFETY: the handler only hands the answer over, behind a mutex, from whichever queue
    // AVFoundation calls it on.
    unsafe { asset.loadValuesAsynchronouslyForKeys_completionHandler(&keys, Some(&handler)) };
    let asset = asset.clone();
    async move {
        answered.await;
        status_of(&asset, key)
    }
}

/// Whether `asset`'s value of `key` loaded, or why not.
fn status_of(asset: &AVURLAsset, key: &str) -> Result<(), PlayerError> {
    let key = NSString::from_str(key);
    let mut error: Option<Retained<NSError>> = None;
    // SAFETY: `statusOfValueForKey:error:` of `AVAsynchronousKeyValueLoading`, which the binding
    // leaves out, with the error it fills where loading failed.
    let status: AVKeyValueStatus =
        unsafe { msg_send![asset, statusOfValueForKey: &*key, error: &mut error] };
    if status == AVKeyValueStatus::Loaded {
        Ok(())
    } else {
        Err(error_of(error.as_deref()))
    }
}

/// An asset of a URL AVFoundation cannot take: every answer fails, with why.
struct UnreadableAsset(PlayerError);

impl UnreadableAsset {
    fn failed<T: 'static>(&self) -> MediaAssetFuture<T> {
        Box::pin(std::future::ready(Err(self.0.clone())))
    }
}

impl MediaAsset for UnreadableAsset {
    fn duration(&self) -> MediaAssetFuture<Option<Duration>> {
        self.failed()
    }

    fn tracks(&self) -> MediaAssetFuture<Vec<MediaTrack>> {
        self.failed()
    }

    fn metadata(&self) -> MediaAssetFuture<MediaMetadata> {
        self.failed()
    }

    fn frame_at(
        &self,
        _time: Duration,
        _tolerance: SeekTolerance,
        _max_size: Option<Size>,
    ) -> MediaAssetFuture<Arc<dyn VideoFrame>> {
        self.failed()
    }
}
