//! Flutter counterpart: `services/keyboard_inserted_content.dart`.

/// A class representing rich content (such as a PNG image) inserted via the
/// system input method.
///
/// The following data is represented in this class:
///  - MIME Type
///  - Bytes
///  - URI
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyboardInsertedContent {
    /// The mime type of the inserted content.
    pub mime_type: String,
    /// The URI (location) of the inserted content, usually a "content://" URI.
    pub uri: String,
    /// The bytedata of the inserted content.
    pub data: Option<Vec<u8>>,
}

impl KeyboardInsertedContent {
    /// Creates an object to represent content that is inserted from the virtual
    /// keyboard.
    ///
    /// The mime type and URI will always be provided, but the bytedata may be null.
    pub fn new(mime_type: impl Into<String>, uri: impl Into<String>) -> KeyboardInsertedContent {
        KeyboardInsertedContent {
            mime_type: mime_type.into(),
            uri: uri.into(),
            data: None,
        }
    }

    /// Dart `KeyboardInsertedContent(data:)`.
    pub fn data(mut self, data: Vec<u8>) -> KeyboardInsertedContent {
        self.data = Some(data);
        self
    }

    /// Convenience getter to check if bytedata is available for the inserted content.
    pub fn has_data(&self) -> bool {
        self.data.as_ref().is_some_and(|data| !data.is_empty())
    }
}

impl std::fmt::Display for KeyboardInsertedContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KeyboardInsertedContent({}, {}, {:?})",
            self.mime_type, self.uri, self.data
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_and_has_data() {
        let empty = KeyboardInsertedContent::new("image/png", "content://item");
        let with_bytes =
            KeyboardInsertedContent::new("image/png", "content://item").data(vec![1, 2]);
        assert_eq!(
            empty,
            KeyboardInsertedContent::new("image/png", "content://item")
        );
        assert_ne!(empty, with_bytes);
        assert!(!empty.has_data());
        assert!(with_bytes.has_data());
        assert!(
            !KeyboardInsertedContent::new("image/png", "content://item")
                .data(Vec::new())
                .has_data()
        );
    }
}
