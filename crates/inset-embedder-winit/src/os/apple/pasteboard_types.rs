//! The framework names a form by MIME type and Apple's pasteboards by UTI, AppKit's and UIKit's
//! alike: the type of each form an item is written in, and the MIME type of each it is read in, as
//! Chrome maps the web's types on macOS. Text is the pasteboards' UTF-8 plain text (AppKit's
//! `NSPasteboardTypeString`), though the system gives it a MIME type with a charset; a URL, a
//! file's included, is read as `text/uri-list`, which the system has no type for; every other type
//! is the system's own for that MIME type, a dynamic one where it declares none, which reads back
//! as the same MIME type. A UTI with no MIME type, such as RTFD's, is not offered.

use objc2::rc::Retained;
use objc2_foundation::NSString;
use objc2_uniform_type_identifiers::UTType;

use inset_embedder::URI_LIST;

/// UTF-8 plain text: AppKit's `NSPasteboardTypeString`, UIKit's `UTTypeUTF8PlainText`.
pub(crate) const UTF8_TEXT: &str = "public.utf8-plain-text";
/// A file's URL: AppKit's `NSPasteboardTypeFileURL`.
pub(crate) const FILE_URL: &str = "public.file-url";
/// A URL: AppKit's `NSPasteboardTypeURL`.
pub(crate) const URL: &str = "public.url";

const TEXT: &str = "text/plain";

/// The UTI a form of `mime` is written as; none for a MIME type the system cannot name.
pub(crate) fn uti_of(mime: &str) -> Option<Retained<NSString>> {
    match mime {
        TEXT => Some(NSString::from_str(UTF8_TEXT)),
        URI_LIST => Some(NSString::from_str(URL)),
        _ => UTType::typeWithMIMEType(&NSString::from_str(mime)).map(|uti| uti.identifier()),
    }
}

/// The MIME type a form named `uti` is read as; none for a UTI with no MIME type.
pub(crate) fn mime_of(uti: &NSString) -> Option<String> {
    match uti.to_string().as_str() {
        UTF8_TEXT => Some(TEXT.to_owned()),
        FILE_URL | URL => Some(URI_LIST.to_owned()),
        _ => UTType::typeWithIdentifier(uti)?
            .preferredMIMEType()
            .map(|mime| mime.to_string()),
    }
}

/// Whether a form named `uti` is a URL, which is read as a line of a `text/uri-list`.
pub(crate) fn is_url(uti: &NSString) -> bool {
    let uti = uti.to_string();
    uti == FILE_URL || uti == URL
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written_and_read(mime: &str) -> Option<String> {
        mime_of(&*uti_of(mime)?)
    }

    #[test]
    fn text_is_the_pasteboards_string_and_reads_back_as_plain_text() {
        assert_eq!(uti_of("text/plain").unwrap().to_string(), UTF8_TEXT);
        assert_eq!(
            written_and_read("text/plain").as_deref(),
            Some("text/plain")
        );
    }

    #[test]
    fn a_url_is_read_as_a_uri_list() {
        assert_eq!(
            mime_of(&NSString::from_str(FILE_URL)).as_deref(),
            Some(URI_LIST)
        );
        assert_eq!(written_and_read(URI_LIST).as_deref(), Some(URI_LIST));
    }

    #[test]
    fn a_declared_type_and_an_undeclared_one_read_back_as_written() {
        assert_eq!(uti_of("image/png").unwrap().to_string(), "public.png");
        assert_eq!(written_and_read("image/png").as_deref(), Some("image/png"));
        assert_eq!(
            written_and_read("application/x-inset-test").as_deref(),
            Some("application/x-inset-test")
        );
    }

    #[test]
    fn a_type_with_no_mime_type_is_not_offered() {
        assert_eq!(mime_of(&NSString::from_str("com.apple.flat-rtfd")), None);
    }
}
