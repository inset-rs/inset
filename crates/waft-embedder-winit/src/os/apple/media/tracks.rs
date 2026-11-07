//! A player's tracks, as AVFoundation lists them: the options of its asset's audible and legible
//! media selection groups, of which an item plays one at a time, a legible group allowing none
//! where it says so (`allowsEmptySelection`); and its item's video tracks. Media without a group of
//! a kind, such as an MP3 with its one sound, lists that kind's tracks from the item's tracks,
//! which play while enabled. An asset read without playing lists its tracks the same way, each
//! selected as a player of it would play it before anything is chosen: a group's default option,
//! and a track the media enables.
//!
//! A track's id is `audible:<n>` or `legible:<n>` for its group's option `n`, and `track:<id>` for
//! an item's track with that track id. The groups load in the background, and a kind lists no
//! tracks until its group has loaded or is known to be absent, so an audio or text track keeps its
//! id for the run; a stream's video track is a new item track, with a new id, at each variant
//! AVFoundation switches to.

use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2_av_foundation::{
    AVAsset, AVAssetTrack, AVMediaCharacteristic, AVMediaCharacteristicAudible,
    AVMediaCharacteristicLegible, AVMediaSelectionGroup, AVMediaSelectionOption, AVMediaType,
    AVMediaTypeAudio, AVMediaTypeClosedCaption, AVMediaTypeSubtitle, AVMediaTypeText,
    AVMediaTypeVideo, AVMetadataCommonIdentifierTitle, AVMetadataItem, AVPlayerItem,
};
use objc2_foundation::{NSArray, NSError, NSLocale, NSString};
use waft_embedder::{MediaTrack, MediaTrackKind};

use crate::answer::answer;
use crate::media::Changes;

/// The audible group's options, as a player's audio tracks.
const AUDIBLE: GroupKind = GroupKind {
    kind: MediaTrackKind::Audio,
    name: "audible",
};

/// The legible group's options, as a player's text tracks.
const LEGIBLE: GroupKind = GroupKind {
    kind: MediaTrackKind::Text,
    name: "legible",
};

/// A player's tracks: its video tracks, and the audio and text tracks it plays one of at a time.
pub(super) struct MediaTracks {
    audio: SelectableTracks,
    text: SelectableTracks,
}

impl MediaTracks {
    /// Loads `asset`'s audible and legible groups, telling `changes` as each arrives.
    pub(super) fn load(asset: &AVAsset, changes: &Changes) -> MediaTracks {
        // SAFETY: constants AVFoundation exports for the life of the process.
        let (audible, legible) =
            unsafe { (AVMediaCharacteristicAudible, AVMediaCharacteristicLegible) };
        MediaTracks {
            audio: SelectableTracks {
                of: AUDIBLE,
                group: LoadedGroup::load(asset, audible, changes.clone()),
            },
            text: SelectableTracks {
                of: LEGIBLE,
                group: LoadedGroup::load(asset, legible, changes.clone()),
            },
        }
    }

    /// The tracks of `item`: its video, then its audio, then its text.
    pub(super) fn tracks(&self, item: &AVPlayerItem) -> Vec<MediaTrack> {
        let mut tracks = item_tracks(item, MediaTrackKind::Video);
        tracks.extend(self.audio.tracks(item));
        tracks.extend(self.text.tracks(item));
        tracks
    }

    /// Plays the audio track `id` in place of the one playing.
    pub(super) fn select_audio(&self, item: &AVPlayerItem, id: &str) {
        self.audio.select(item, Some(id));
    }

    /// Shows the text track `id`, or none where the media allows it.
    pub(super) fn select_text(&self, item: &AVPlayerItem, id: Option<&str>) {
        self.text.select(item, id);
    }
}

/// The tracks of one kind a group's options are, and what their ids start with.
#[derive(Clone, Copy)]
struct GroupKind {
    kind: MediaTrackKind,
    name: &'static str,
}

impl GroupKind {
    fn option_id(self, index: usize) -> String {
        format!("{}:{index}", self.name)
    }

    /// The option `id` names; `None` for an id of anything else.
    fn option_index(self, id: &str) -> Option<usize> {
        id.strip_prefix(self.name)?.strip_prefix(':')?.parse().ok()
    }

    /// `group`'s options as tracks, `chosen` selected.
    fn option_tracks(
        self,
        group: &AVMediaSelectionGroup,
        chosen: Option<&AVMediaSelectionOption>,
    ) -> Vec<MediaTrack> {
        // SAFETY: reading the group's options, which load with it.
        let options = unsafe { group.options() };
        options
            .iter()
            .enumerate()
            .map(|(index, option)| MediaTrack {
                id: self.option_id(index),
                kind: self.kind,
                language: option_language(&option),
                // SAFETY: an option's metadata, which loads with its group.
                label: title_of(&*unsafe { option.commonMetadata() }),
                selected: chosen == Some(&*option),
            })
            .collect()
    }
}

/// The tracks of one kind an item plays one of at a time: its group's options, or the item's own
/// tracks of the kind where the media has no group.
struct SelectableTracks {
    of: GroupKind,
    group: LoadedGroup,
}

impl SelectableTracks {
    fn tracks(&self, item: &AVPlayerItem) -> Vec<MediaTrack> {
        match &*self.group.state() {
            GroupState::Loading => Vec::new(),
            GroupState::Loaded(group) => self.option_tracks(item, &group.0),
            GroupState::Absent => item_tracks(item, self.of.kind),
        }
    }

    /// Plays the track `id` in place of the one playing, or none where `id` is `None` and the
    /// media allows it. An id this player did not give is ignored.
    fn select(&self, item: &AVPlayerItem, id: Option<&str>) {
        match &*self.group.state() {
            GroupState::Loading => {}
            GroupState::Loaded(group) => self.select_option(item, &group.0, id),
            GroupState::Absent => select_item_track(item, self.of.kind, id),
        }
    }

    /// `group`'s options as tracks, the one the item plays selected.
    fn option_tracks(&self, item: &AVPlayerItem, group: &AVMediaSelectionGroup) -> Vec<MediaTrack> {
        // SAFETY: reading the item's selection on the main thread.
        let selected = unsafe {
            item.currentMediaSelection()
                .selectedMediaOptionInMediaSelectionGroup(group)
        };
        self.of.option_tracks(group, selected.as_deref())
    }

    /// Selects `group`'s option `id`, or none where `id` is `None` and the group allows it.
    fn select_option(&self, item: &AVPlayerItem, group: &AVMediaSelectionGroup, id: Option<&str>) {
        // SAFETY: reading the group and selecting in the item on the main thread.
        unsafe {
            let option = match id {
                Some(id) => {
                    let options = group.options();
                    match self.of.option_index(id) {
                        Some(index) if index < options.count() => {
                            Some(options.objectAtIndex(index))
                        }
                        _ => return,
                    }
                }
                None if group.allowsEmptySelection() => None,
                None => return,
            };
            item.selectMediaOption_inMediaSelectionGroup(option.as_deref(), group);
        }
    }
}

/// A group of one characteristic, filled in by AVFoundation from a queue of its own.
#[derive(Clone)]
struct LoadedGroup(Arc<Mutex<GroupState>>);

enum GroupState {
    Loading,
    /// The media has no group of this characteristic, or it could not be loaded.
    Absent,
    Loaded(CrossThreadGroup),
}

impl LoadedGroup {
    /// The group of `characteristic`, absent where the system names no such characteristic.
    fn load(
        asset: &AVAsset,
        characteristic: Option<&AVMediaCharacteristic>,
        changes: Changes,
    ) -> Self {
        let Some(characteristic) = characteristic else {
            return LoadedGroup(Arc::new(Mutex::new(GroupState::Absent)));
        };
        let loaded = LoadedGroup(Arc::new(Mutex::new(GroupState::Loading)));
        let filled = loaded.clone();
        let handler = RcBlock::new(
            move |group: *mut AVMediaSelectionGroup, _error: *mut NSError| {
                // SAFETY: AVFoundation hands the group, or none, for the call; retaining it keeps
                // it past.
                let group = unsafe { Retained::retain(group) };
                *filled.state() = match group {
                    Some(group) => GroupState::Loaded(CrossThreadGroup(group)),
                    None => GroupState::Absent,
                };
                changes.notify();
            },
        );
        // SAFETY: the handler only fills the shared state and tells the changes, each behind a
        // mutex.
        unsafe {
            asset.loadMediaSelectionGroupForMediaCharacteristic_completionHandler(
                characteristic,
                &handler,
            )
        };
        loaded
    }

    fn state(&self) -> MutexGuard<'_, GroupState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The group of `characteristic` once AVFoundation has loaded it; `None` where the media has none,
/// it could not be loaded, or the system names no such characteristic.
fn group_of(
    asset: &AVAsset,
    characteristic: Option<&AVMediaCharacteristic>,
) -> impl Future<Output = Option<CrossThreadGroup>> + use<> {
    let (given, loaded) = answer(None);
    if let Some(characteristic) = characteristic {
        let given = Mutex::new(Some(given));
        let handler = RcBlock::new(
            move |group: *mut AVMediaSelectionGroup, _error: *mut NSError| {
                // SAFETY: AVFoundation hands the group, or none, for the call; retaining it keeps
                // it past.
                let group = unsafe { Retained::retain(group) };
                let given = given.lock().unwrap_or_else(PoisonError::into_inner).take();
                if let Some(given) = given {
                    given.give(group.map(CrossThreadGroup));
                }
            },
        );
        // SAFETY: the handler only hands the group over, behind a mutex.
        unsafe {
            asset.loadMediaSelectionGroupForMediaCharacteristic_completionHandler(
                characteristic,
                &handler,
            )
        };
    }
    loaded
}

/// A group handed from AVFoundation's loading queue to the main thread.
struct CrossThreadGroup(Retained<AVMediaSelectionGroup>);

// SAFETY: AVFoundation's `loadMediaSelectionGroupForMediaCharacteristic:completionHandler:` hands
// the group over on a queue of its choosing for the caller to use; it is read on the main thread
// only after. The binding cannot say so.
unsafe impl Send for CrossThreadGroup {}

/// The item's tracks of `kind`, those enabled selected.
fn item_tracks(item: &AVPlayerItem, kind: MediaTrackKind) -> Vec<MediaTrack> {
    // SAFETY: reading the item's tracks on the main thread.
    let tracks = unsafe { item.tracks() };
    tracks
        .iter()
        .filter_map(|track| {
            // SAFETY: an item's track and its asset's track, read on the main thread.
            let (asset_track, enabled) = unsafe { (track.assetTrack()?, track.isEnabled()) };
            is_of_kind(&asset_track, kind).then(|| track_of(&asset_track, kind, enabled))
        })
        .collect()
}

/// `asset`'s tracks as a player of it lists them before anything is chosen: its video tracks,
/// then its audio and its text, each kind its group's options with the default one selected,
/// where it has the group, and else its tracks of the kind, those the media enables selected.
/// Once its tracks have loaded; its groups load here.
pub(super) async fn asset_tracks(asset: &AVAsset) -> Vec<MediaTrack> {
    // SAFETY: constants AVFoundation exports for the life of the process.
    let (audible, legible) =
        unsafe { (AVMediaCharacteristicAudible, AVMediaCharacteristicLegible) };
    let audible = group_of(asset, audible).await;
    let legible = group_of(asset, legible).await;
    // SAFETY: the asset's tracks, which have loaded.
    let tracks = unsafe { asset.tracks() };
    let mut listed = asset_tracks_of(&tracks, MediaTrackKind::Video);
    listed.extend(default_tracks(AUDIBLE, audible.as_ref(), &tracks));
    listed.extend(default_tracks(LEGIBLE, legible.as_ref(), &tracks));
    listed
}

/// `group`'s options with its default one selected, where there is the group; `tracks` of its
/// kind otherwise.
fn default_tracks(
    of: GroupKind,
    group: Option<&CrossThreadGroup>,
    tracks: &NSArray<AVAssetTrack>,
) -> Vec<MediaTrack> {
    match group {
        Some(group) => {
            // SAFETY: the group's default option, which loads with it.
            let chosen = unsafe { group.0.defaultOption() };
            of.option_tracks(&group.0, chosen.as_deref())
        }
        None => asset_tracks_of(tracks, of.kind),
    }
}

/// `tracks` of `kind`, those the media enables selected.
fn asset_tracks_of(tracks: &NSArray<AVAssetTrack>, kind: MediaTrackKind) -> Vec<MediaTrack> {
    tracks
        .iter()
        .filter(|track| is_of_kind(track, kind))
        // SAFETY: whether a loaded track is enabled, as its container says.
        .map(|track| track_of(&track, kind, unsafe { track.isEnabled() }))
        .collect()
}

/// `track`, of `kind`, as a track listed.
fn track_of(track: &AVAssetTrack, kind: MediaTrackKind, selected: bool) -> MediaTrack {
    MediaTrack {
        // SAFETY: a track's id and metadata, loaded with it.
        id: item_track_id(unsafe { track.trackID() }),
        kind,
        language: track_language(track),
        // SAFETY: as above.
        label: title_of(&*unsafe { track.commonMetadata() }),
        selected,
    }
}

/// Enables the item's track `id` of `kind` and disables its others of that kind; disables all of
/// them where `id` is `None`.
fn select_item_track(item: &AVPlayerItem, kind: MediaTrackKind, id: Option<&str>) {
    // SAFETY: reading and enabling the item's tracks on the main thread.
    let tracks = unsafe { item.tracks() };
    let chosen = id.and_then(item_track_number);
    for track in &tracks {
        // SAFETY: as above.
        let Some(asset_track) = (unsafe { track.assetTrack() }) else {
            continue;
        };
        if is_of_kind(&asset_track, kind) {
            // SAFETY: as above.
            let number = unsafe { asset_track.trackID() };
            unsafe { track.setEnabled(chosen == Some(number)) };
        }
    }
}

/// Whether `track` carries `kind`: closed captions and subtitles are text.
fn is_of_kind(track: &AVAssetTrack, kind: MediaTrackKind) -> bool {
    // SAFETY: a track's media type, fixed when it is made.
    let media_type = unsafe { track.mediaType() };
    media_types(kind)
        .into_iter()
        .flatten()
        .any(|of_kind| *media_type == *of_kind)
}

fn media_types(kind: MediaTrackKind) -> Vec<Option<&'static AVMediaType>> {
    // SAFETY: constants AVFoundation exports for the life of the process.
    unsafe {
        match kind {
            MediaTrackKind::Video => vec![AVMediaTypeVideo],
            MediaTrackKind::Audio => vec![AVMediaTypeAudio],
            MediaTrackKind::Text => vec![
                AVMediaTypeSubtitle,
                AVMediaTypeText,
                AVMediaTypeClosedCaption,
            ],
        }
    }
}

fn item_track_id(number: i32) -> String {
    format!("track:{number}")
}

/// The item's track an id names; `None` for an id of anything else.
fn item_track_number(id: &str) -> Option<i32> {
    id.strip_prefix("track:")?.parse().ok()
}

/// An option's BCP 47 language: its extended language tag, else its locale's language.
fn option_language(option: &AVMediaSelectionOption) -> String {
    // SAFETY: an option's language, which loads with its group.
    let (tag, locale) = unsafe { (option.extendedLanguageTag(), option.locale()) };
    tag.or_else(|| locale.map(|locale| locale.languageCode()))
        .map_or_else(String::new, |language| bcp47_of(&language))
}

/// A track's BCP 47 language: its extended language tag, else its ISO 639-2 code.
fn track_language(track: &AVAssetTrack) -> String {
    // SAFETY: a playing track's language, loaded with it.
    let (tag, code) = unsafe { (track.extendedLanguageTag(), track.languageCode()) };
    tag.or(code)
        .map_or_else(String::new, |language| bcp47_of(&language))
}

/// `language` as the system canonicalizes it to BCP 47, `eng` becoming `en`; empty for `und`,
/// the code for a language not given.
fn bcp47_of(language: &NSString) -> String {
    let canonical = NSLocale::canonicalLanguageIdentifierFromString(language).to_string();
    if canonical == "und" {
        String::new()
    } else {
        canonical
    }
}

/// The title among `metadata`, the name the media gives a track; empty where it gives none.
fn title_of(metadata: &NSArray<AVMetadataItem>) -> String {
    // SAFETY: a constant AVFoundation exports for the life of the process.
    let Some(title) = (unsafe { AVMetadataCommonIdentifierTitle }) else {
        return String::new();
    };
    // SAFETY: filtering loaded metadata items, and reading a string value of one.
    unsafe {
        AVMetadataItem::metadataItemsFromArray_filteredByIdentifier(metadata, title)
            .firstObject()
            .and_then(|item| item.stringValue())
    }
    .map_or_else(String::new, |title| title.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_names_a_groups_option_or_an_items_track_and_nothing_of_another_kind() {
        assert_eq!(AUDIBLE.option_id(2), "audible:2");
        assert_eq!(AUDIBLE.option_index("audible:2"), Some(2));
        assert_eq!(LEGIBLE.option_index("audible:2"), None);
        assert_eq!(LEGIBLE.option_index("legible:x"), None);
        assert_eq!(item_track_number(&item_track_id(7)), Some(7));
        assert_eq!(item_track_number("legible:7"), None);
    }

    #[test]
    fn a_language_is_bcp_47_and_none_given_is_empty() {
        assert_eq!(bcp47_of(&NSString::from_str("eng")), "en");
        assert_eq!(bcp47_of(&NSString::from_str("fr-CA")), "fr-CA");
        assert_eq!(bcp47_of(&NSString::from_str("und")), "");
    }
}
