//! A camera's characteristics as the NDK hands them out: an `ACameraMetadata` the camera manager
//! made for the caller, read entry by entry, each a run of values of the tag's one type.
//!
//! The invariant: the metadata is the caller's copy, freed when this drops, and an entry's values
//! are copied out before it is.

// Reading the NDK's metadata is `unsafe` to call; this module writes it for its callers.
#![allow(unsafe_code)]

use std::ptr::NonNull;

use ndk_sys::{
    ACameraMetadata, ACameraMetadata_const_entry, ACameraMetadata_free,
    ACameraMetadata_getConstEntry, acamera_metadata_tag, camera_status_t,
};

/// A camera's characteristics, freed when dropped.
pub(super) struct CameraMetadata(NonNull<ACameraMetadata>);

// SAFETY: the metadata is the caller's own copy, read only, and the NDK's reads of it take no
// lock and change nothing.
unsafe impl Send for CameraMetadata {}
unsafe impl Sync for CameraMetadata {}

impl CameraMetadata {
    /// The metadata at `raw`, which the camera manager made for the caller to free.
    ///
    /// # Safety
    ///
    /// `raw` is an `ACameraMetadata` nothing else frees.
    pub(super) unsafe fn from_owned(raw: NonNull<ACameraMetadata>) -> CameraMetadata {
        CameraMetadata(raw)
    }

    /// The values of `tag`, a tag of 32-bit integers; empty where the camera gives none.
    pub(super) fn i32s(&self, tag: acamera_metadata_tag) -> Vec<i32> {
        self.entry(tag)
            .map(|entry| {
                // SAFETY: a tag of 32-bit integers holds `count` of them, live while the
                // metadata is, and copied out here.
                unsafe { std::slice::from_raw_parts(entry.data.i32_, entry.count as usize) }
                    .to_vec()
            })
            .unwrap_or_default()
    }

    /// The values of `tag`, a tag of 64-bit integers; empty where the camera gives none.
    pub(super) fn i64s(&self, tag: acamera_metadata_tag) -> Vec<i64> {
        self.entry(tag)
            .map(|entry| {
                // SAFETY: as for `i32s`, of 64-bit integers.
                unsafe { std::slice::from_raw_parts(entry.data.i64_, entry.count as usize) }
                    .to_vec()
            })
            .unwrap_or_default()
    }

    /// The values of `tag`, a tag of bytes; empty where the camera gives none.
    pub(super) fn u8s(&self, tag: acamera_metadata_tag) -> Vec<u8> {
        self.entry(tag)
            .map(|entry| {
                // SAFETY: as for `i32s`, of bytes.
                unsafe { std::slice::from_raw_parts(entry.data.u8_, entry.count as usize) }.to_vec()
            })
            .unwrap_or_default()
    }

    /// The entry of `tag`, with at least one value; `None` where the camera gives none.
    fn entry(&self, tag: acamera_metadata_tag) -> Option<ACameraMetadata_const_entry> {
        let mut entry = std::mem::MaybeUninit::<ACameraMetadata_const_entry>::uninit();
        // SAFETY: live metadata, and a place for the entry, which the call fills where it answers
        // `ACAMERA_OK`.
        let status =
            unsafe { ACameraMetadata_getConstEntry(self.0.as_ptr(), tag.0, entry.as_mut_ptr()) };
        if status != camera_status_t::ACAMERA_OK {
            return None;
        }
        // SAFETY: filled, as the status says.
        let entry = unsafe { entry.assume_init() };
        (entry.count > 0).then_some(entry)
    }
}

impl Drop for CameraMetadata {
    fn drop(&mut self) {
        // SAFETY: the caller's copy, freed once.
        unsafe { ACameraMetadata_free(self.0.as_ptr()) };
    }
}
