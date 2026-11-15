//! A leak check around a piece of a test, after GPUI's `leak_detector_snapshot` and
//! `assert_no_new_leaks`: [`object_snapshot`] counts the objects an `App` holds by type and
//! creation site, [`assert_no_new_objects`] fails naming each type and creation site that holds
//! more objects than it did. A count, not a set of ids: a rebuild replaces an object made in
//! `build` with a new one made at the same site, which is no leak. Debug builds only, since a
//! release arena records no types or creation sites.

use std::collections::BTreeMap;
use std::panic::Location;

use waft_foundation::App;

/// Where an object was made: its type and its creation site.
type Site = (&'static str, &'static Location<'static>);

/// The objects an `App` held at one moment, counted by type and creation site; see
/// [`object_snapshot`].
#[derive(Debug, Clone)]
pub struct ObjectSnapshot {
    counts: BTreeMap<Site, usize>,
}

/// Counts the objects `app` holds now, kept or waiting for the next checkpoint.
pub fn object_snapshot(app: &App) -> ObjectSnapshot {
    let mut counts = BTreeMap::new();
    for object in app.debug_live_objects() {
        *counts
            .entry((object.type_name, object.created_at))
            .or_insert(0) += 1;
    }
    ObjectSnapshot { counts }
}

/// Fails if `app` holds more objects of some type, made at some site, than it did at
/// `snapshot`, listing each such type and creation site with how many more. Run it after a
/// checkpoint, so that what was let go is freed.
///
/// # Panics
///
/// If objects made since the snapshot are still held beyond what it held.
#[track_caller]
pub fn assert_no_new_objects(app: &App, snapshot: &ObjectSnapshot) {
    let now = object_snapshot(app);
    let mut message = String::new();
    for (&(type_name, created_at), &count) in &now.counts {
        let before = snapshot
            .counts
            .get(&(type_name, created_at))
            .copied()
            .unwrap_or(0);
        if count > before {
            message.push_str(&format!(
                "\n  +{} `{type_name}` created at {created_at}",
                count - before
            ));
        }
    }
    if !message.is_empty() {
        panic!("objects made since the snapshot are still held:{message}");
    }
}

#[cfg(test)]
mod tests {
    use waft_foundation::AppCell;

    use super::*;

    #[test]
    fn an_object_made_after_the_snapshot_and_still_held_is_named() {
        let cell = AppCell::new();
        let snapshot = object_snapshot(&cell.borrow());
        let kept = cell.borrow_mut().create(7u32);
        let dropped = cell.borrow_mut().create(8u64);
        drop(dropped);
        cell.checkpoint();
        let message = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_no_new_objects(&cell.borrow(), &snapshot);
        }))
        .expect_err("the kept object fails the check");
        let message = message
            .downcast_ref::<String>()
            .expect("a formatted message");
        assert!(message.contains("+1 `u32`"), "{message}");
        assert!(!message.contains("`u64`"), "{message}");
        assert!(message.contains("objects.rs"), "{message}");
        drop(kept);
        cell.checkpoint();
        assert_no_new_objects(&cell.borrow(), &snapshot);
    }

    #[test]
    fn an_object_replaced_by_one_made_at_the_same_site_is_no_leak() {
        let cell = AppCell::new();
        let make = |cell: &AppCell| cell.borrow_mut().create(1u32);
        let first = make(&cell);
        cell.checkpoint();
        let snapshot = object_snapshot(&cell.borrow());
        let second = make(&cell);
        drop(first);
        cell.checkpoint();
        assert_no_new_objects(&cell.borrow(), &snapshot);
        drop(second);
    }
}
