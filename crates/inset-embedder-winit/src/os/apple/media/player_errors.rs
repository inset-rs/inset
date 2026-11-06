//! AVFoundation's errors as a player's failure: the kind a player and a media asset fail with, by
//! the error's own domain and code, or the error it wraps, and AVFoundation's words for logs.

use inset_embedder::{PlayerError, PlayerErrorKind};
use objc2::rc::Retained;
use objc2_av_foundation::AVError;
use objc2_foundation::{NSError, NSUnderlyingErrorKey};

/// `error` as a player's failure: its kind, and AVFoundation's words for it.
pub(super) fn error_of(error: Option<&NSError>) -> PlayerError {
    match error {
        Some(error) => PlayerError {
            kind: kind_of(error),
            message: error.localizedDescription().to_string(),
        },
        None => PlayerError {
            kind: PlayerErrorKind::Other,
            message: "the system could not read the media".to_owned(),
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

#[cfg(test)]
mod tests {
    use objc2_foundation::NSString;

    use super::*;

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
}
