//! Which view an activity was, saved for the activity Android makes in its place.
//!
//! Android destroys an activity without finishing it to make it again, after a change of
//! configuration the manifest does not absorb or under the developer option "Don't keep
//! activities", and hands the new one the state the old one saved (`onSaveInstanceState`). The
//! view outlives the old activity, without a window, so the new one attaches to it rather than
//! adding a view, as Flutter's engine outlives a configuration change and the new activity attaches
//! to it (`FlutterActivityAndFragmentDelegate.onDetach`, `detachFromActivityForConfigChanges`).
//!
//! What an activity saves is its view's id with a value made fresh in each process
//! (`this_process.rs`). A card Android keeps in Recents after the process died brings back the
//! state an earlier process saved, whose view ids are no view of this one; the value tells those
//! apart, and such an activity adds a view. The invariant a later editor will break: a token is
//! this process's only where the value is, never by the id alone.

use waft_embedder::ViewId;

use crate::this_process::this_process;

/// The bytes an activity saves for `view`: this process's value, then the view's id.
pub(crate) fn token(view: ViewId) -> [u8; 16] {
    let mut token = [0; 16];
    token[..8].copy_from_slice(&this_process().to_le_bytes());
    token[8..].copy_from_slice(&view.0.to_le_bytes());
    token
}

/// The view an activity saved `saved`, the bytes of its token, for: none where the token is
/// another process's, or not a token at all.
pub(crate) fn view_of(saved: &[u8]) -> Option<ViewId> {
    let (process, view) = saved.split_at_checked(8)?;
    let process = u64::from_le_bytes(process.try_into().ok()?);
    let view = u64::from_le_bytes(view.try_into().ok()?);
    (process == this_process()).then_some(ViewId(view))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_of_this_process_names_its_view() {
        assert_eq!(view_of(&token(ViewId(7))), Some(ViewId(7)));
    }

    #[test]
    fn a_token_of_another_process_names_no_view() {
        let mut token = token(ViewId(7));
        token[0] ^= 1;
        assert_eq!(view_of(&token), None);
    }

    #[test]
    fn bytes_that_are_no_token_name_no_view() {
        assert_eq!(view_of(&[]), None);
        assert_eq!(view_of(&token(ViewId(7))[..15]), None);
    }
}
