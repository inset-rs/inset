//! The pasteboard on a pasteboard of each test's own, which no other application reads. A read
//! loads through UIKit's item providers, which answer on a queue of their own, so each test waits
//! for it. A form or a file the app promised is produced on the main thread, whose queue only the
//! app's own run loop serves, and a test runs on a thread of the harness's, so those are written
//! here only to be refused.

use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::os::apple::app_futures::LocalFuture;

/// A pasteboard of the test's own, with no notices.
fn private_pasteboard() -> Pasteboard {
    Pasteboard::new(UIPasteboard::pasteboardWithUniqueName(), None)
}

/// `future`'s output, once UIKit has answered.
fn waited<T>(future: LocalFuture<T>) -> T {
    pollster::block_on(future)
}

fn data(mime: &str, bytes: &[u8]) -> (String, Content) {
    (mime.to_owned(), Content::Data(bytes.to_vec()))
}

fn item(representations: Vec<(String, Content)>) -> ClipboardItem {
    ClipboardItem { representations }
}

/// Writes `items` as `setItems:options:` takes them, or as item providers.
fn written(pasteboard: &Pasteboard, items: Vec<ClipboardItem>, as_providers: bool) {
    let written = if as_providers {
        pasteboard.write_providers(items, CopyOptions::new())
    } else {
        pasteboard.write_items(items, CopyOptions::new())
    };
    written.unwrap();
}

#[test]
fn items_are_written_and_read_in_their_types_whole_or_through_item_providers() {
    for as_providers in [false, true] {
        let pasteboard = private_pasteboard();
        let first = item(vec![
            data("text/html", b"<b>hi</b>"),
            data("text/plain", b"hi"),
        ]);
        let second = item(vec![data("image/png", b"\x89PNG")]);
        written(&pasteboard, vec![first, second], as_providers);

        let items = waited(pasteboard.items());
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].types()[..2],
            ["text/html", "text/plain"],
            "in the order written, before any form UIKit derives from them, as RTF from HTML"
        );
        assert_eq!(waited(items[0].read("text/plain")), Ok(b"hi".to_vec()));
        assert_eq!(
            waited(items[0].read("text/html")),
            Ok(b"<b>hi</b>".to_vec())
        );
        assert_eq!(waited(items[1].read("image/png")), Ok(b"\x89PNG".to_vec()));
        assert_eq!(
            waited(items[1].read("text/plain")),
            Err(ClipboardError::NotFound)
        );
    }
}

#[test]
fn a_read_after_another_write_fails() {
    let pasteboard = private_pasteboard();
    let text = |bytes: &[u8]| vec![item(vec![data("text/plain", bytes)])];
    waited(pasteboard.write(text(b"first"), CopyOptions::new())).unwrap();
    let items = waited(pasteboard.items());
    waited(pasteboard.write(text(b"second"), CopyOptions::new())).unwrap();
    assert_eq!(
        waited(items[0].read("text/plain")),
        Err(ClipboardError::Changed)
    );
}

#[test]
fn a_file_is_written_as_its_url_and_read_as_a_uri_list_whole_or_through_item_providers() {
    for as_providers in [false, true] {
        let pasteboard = private_pasteboard();
        let file = Content::File(PathBuf::from("/tmp/a b.png"));
        written(
            &pasteboard,
            vec![item(vec![("image/png".to_owned(), file)])],
            as_providers,
        );

        let items = waited(pasteboard.items());
        assert!(
            items[0]
                .types()
                .contains(&inset_embedder::URI_LIST.to_owned()),
            "{:?}",
            items[0].types()
        );
        assert_eq!(
            waited(items[0].read(inset_embedder::URI_LIST)),
            Ok(b"file:///tmp/a%20b.png\r\n".to_vec())
        );
    }
}

#[test]
fn an_item_carrying_a_file_and_another_form_is_refused() {
    let pasteboard = private_pasteboard();
    let file = (
        "image/png".to_owned(),
        Content::File(PathBuf::from("/tmp/a.png")),
    );
    let written = waited(pasteboard.write(
        vec![item(vec![file, data("text/plain", b"a.png")])],
        CopyOptions::new(),
    ));
    assert_eq!(written, Err(ClipboardError::Unsupported));
}

#[test]
fn a_promise_is_refused_off_the_main_thread_its_producer_may_not_leave() {
    let pasteboard = private_pasteboard();
    let promise = Content::Promise(std::rc::Rc::new(|| Box::pin(ready(Some(Vec::new())))));
    let written = waited(pasteboard.write(
        vec![item(vec![("text/html".to_owned(), promise)])],
        CopyOptions::new(),
    ));
    assert!(
        matches!(written, Err(ClipboardError::Failed(_))),
        "{written:?}"
    );
}

#[test]
fn another_writers_change_is_seen_once_and_the_apps_own_counts_as_seen() {
    let pasteboard = private_pasteboard();
    assert!(
        !pasteboard.has_changed(),
        "the pasteboard as it was at the start"
    );
    waited(pasteboard.write(
        vec![item(vec![data("text/plain", b"x")])],
        CopyOptions::new(),
    ))
    .unwrap();
    assert!(!pasteboard.has_changed(), "the app made it");
    pasteboard.pasteboard.setData_forPasteboardType(
        &NSData::with_bytes(b"y"),
        &NSString::from_str("public.utf8-plain-text"),
    );
    assert!(pasteboard.has_changed(), "another writer's");
    assert!(!pasteboard.has_changed());
}

#[test]
fn a_copy_that_expires_and_one_for_this_device_only_are_written() {
    let pasteboard = private_pasteboard();
    let options = CopyOptions::new()
        .this_device_only(true)
        .expires_after(Duration::from_secs(60));
    waited(pasteboard.write(vec![item(vec![data("text/plain", b"code")])], options)).unwrap();
    let items = waited(pasteboard.items());
    assert_eq!(waited(items[0].read("text/plain")), Ok(b"code".to_vec()));
}
