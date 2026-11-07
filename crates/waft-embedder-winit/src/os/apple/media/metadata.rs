//! What a player's media says about itself: its asset's common metadata (title, artist, album
//! name, artwork), which AVFoundation loads in the background and hands to a completion handler
//! (`loadValuesAsynchronouslyForKeys:completionHandler:`), where it is read once and kept, so the
//! artwork is not copied again at each asking.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2_av_foundation::{
    AVAsynchronousKeyValueLoading, AVMetadataCommonIdentifierAlbumName,
    AVMetadataCommonIdentifierArtist, AVMetadataCommonIdentifierArtwork,
    AVMetadataCommonIdentifierTitle, AVMetadataIdentifier, AVMetadataItem, AVURLAsset,
};
use objc2_foundation::{NSArray, NSString};
use waft_embedder::{EncodedImage, MediaMetadata};

use crate::media::Changes;

/// The asset's metadata, empty until AVFoundation has loaded it.
#[derive(Clone, Default)]
pub(super) struct AssetMetadata(Arc<Mutex<MediaMetadata>>);

impl AssetMetadata {
    /// Loads `asset`'s common metadata, telling `changes` once it is read.
    pub(super) fn load(asset: &Retained<AVURLAsset>, changes: Changes) -> AssetMetadata {
        let metadata = AssetMetadata::default();
        let filled = metadata.clone();
        let loaded = asset.clone();
        let handler = RcBlock::new(move || {
            // SAFETY: the key this handler is called for has finished loading, so reading it
            // does not block; the asset is a URL asset, which AVFoundation marks sendable.
            let items = unsafe { loaded.commonMetadata() };
            *filled.lock() = media_metadata_of(&items);
            changes.notify();
        });
        let keys = NSArray::from_retained_slice(&[NSString::from_str("commonMetadata")]);
        // SAFETY: the handler reads the asset and fills a mutex, from whichever queue
        // AVFoundation calls it on.
        unsafe { asset.loadValuesAsynchronouslyForKeys_completionHandler(&keys, Some(&handler)) };
        metadata
    }

    /// The metadata read so far.
    pub(super) fn get(&self) -> MediaMetadata {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, MediaMetadata> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The title, artist, album and artwork among `items`.
pub(super) fn media_metadata_of(items: &NSArray<AVMetadataItem>) -> MediaMetadata {
    // SAFETY: constants AVFoundation exports for the life of the process.
    let (title, artist, album, artwork) = unsafe {
        (
            AVMetadataCommonIdentifierTitle,
            AVMetadataCommonIdentifierArtist,
            AVMetadataCommonIdentifierAlbumName,
            AVMetadataCommonIdentifierArtwork,
        )
    };
    MediaMetadata {
        title: title.and_then(|title| string_of(items, title)),
        artist: artist.and_then(|artist| string_of(items, artist)),
        album: album.and_then(|album| string_of(items, album)),
        artwork: artwork.and_then(|artwork| artwork_of(items, artwork)),
    }
}

/// The first item `identifier` names, where there is one.
fn item_of(
    items: &NSArray<AVMetadataItem>,
    identifier: &AVMetadataIdentifier,
) -> Option<Retained<AVMetadataItem>> {
    // SAFETY: filtering metadata items by an identifier.
    unsafe { AVMetadataItem::metadataItemsFromArray_filteredByIdentifier(items, identifier) }
        .firstObject()
}

fn string_of(items: &NSArray<AVMetadataItem>, identifier: &AVMetadataIdentifier) -> Option<String> {
    let item = item_of(items, identifier)?;
    // SAFETY: the value of an item of loaded metadata.
    unsafe { item.stringValue() }.map(|string| string.to_string())
}

/// The artwork's bytes, with the MIME type of the image format they start with.
fn artwork_of(
    items: &NSArray<AVMetadataItem>,
    identifier: &AVMetadataIdentifier,
) -> Option<EncodedImage> {
    let item = item_of(items, identifier)?;
    // SAFETY: the value of an item of loaded metadata.
    let data = unsafe { item.dataValue() }?.to_vec();
    Some(EncodedImage {
        mime_type: image_mime_type(&data).to_owned(),
        data: Arc::from(data),
    })
}

/// The MIME type of the image format whose signature `bytes` start with, as the WHATWG MIME
/// sniffing standard matches images; `application/octet-stream` for another.
fn image_mime_type(bytes: &[u8]) -> &'static str {
    const SIGNATURES: [(&[u8], &str); 4] = [
        (b"\xFF\xD8\xFF", "image/jpeg"),
        (b"\x89PNG\r\n\x1A\n", "image/png"),
        (b"GIF8", "image/gif"),
        (b"BM", "image/bmp"),
    ];
    SIGNATURES
        .iter()
        .find(|(signature, _)| bytes.starts_with(signature))
        .map_or("application/octet-stream", |(_, mime_type)| mime_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artwork_is_named_by_the_format_its_bytes_start_with() {
        assert_eq!(image_mime_type(b"\xFF\xD8\xFF\xE0rest"), "image/jpeg");
        assert_eq!(image_mime_type(b"\x89PNG\r\n\x1A\nrest"), "image/png");
        assert_eq!(image_mime_type(b"none"), "application/octet-stream");
    }
}
