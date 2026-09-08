//! Flutter counterpart: `services/process_text.dart`.

use reveal_foundation::App;

/// A data structure describing text processing actions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProcessTextAction {
    /// The action unique id.
    pub id: String,
    /// The action localized label.
    pub label: String,
}

impl ProcessTextAction {
    /// Creates text processing actions based on those returned by the engine.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> ProcessTextAction {
        ProcessTextAction {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Determines how to interact with the text processing feature.
pub trait ProcessTextService {
    /// Returns a [`Vec`] of [`ProcessTextAction`]s containing all text processing
    /// actions available.
    ///
    /// If there are no actions available, an empty list will be returned.
    fn query_text_actions(&self, app: &App) -> Vec<ProcessTextAction>;

    /// Returns a [`String`] when the text action returns a transformed text or
    /// [`None`] when the text action did not return a transformed text.
    ///
    /// The `id` parameter is the text action unique identifier returned by
    /// [`query_text_actions`](Self::query_text_actions).
    ///
    /// The `text` parameter is the text to be processed.
    ///
    /// The `read_only` parameter indicates that the transformed text, if it exists,
    /// will be used as read-only.
    fn process_text_action(
        &self,
        app: &App,
        id: &str,
        text: &str,
        read_only: bool,
    ) -> Option<String>;
}

/// The service used by default for the text processing feature.
///
/// Any widget may use this service to get a list of text processing actions
/// and send requests to activate these text actions.
///
/// This is currently only supported on Android and it requires adding the
/// following '<queries>' element to the Android manifest file:
///
/// ```text
/// <manifest ...>
///     <application ...>
///       ...
///     </application>
///     <!-- Required to query activities that can process text, see:
///          https://developer.android.com/training/package-visibility and
///          https://developer.android.com/reference/android/content/Intent#ACTION_PROCESS_TEXT.
///
///          In particular, this is used by the Flutter engine in io.flutter.plugin.text.ProcessTextPlugin. -->
///     <queries>
///         <intent>
///             <action android:name="android.intent.action.PROCESS_TEXT"/>
///             <data android:mimeType="text/plain"/>
///         </intent>
///     </queries>
/// </manifest>
/// ```
///
/// The '<queries>' element is part of the Android manifest file generated when
/// running the 'flutter create' command.
///
/// If the '<queries>' element is not found, [`query_text_actions`](ProcessTextService::query_text_actions) will return an
/// empty list of [`ProcessTextAction`].
///
/// See also:
///
///  * [`ProcessTextService`], the service that this implements.
#[derive(Clone, Debug, Default)]
pub struct DefaultProcessTextService;

impl DefaultProcessTextService {
    /// Creates the default service to interact with the platform text processing
    /// feature.
    pub fn new() -> DefaultProcessTextService {
        DefaultProcessTextService
    }
}

impl ProcessTextService for DefaultProcessTextService {
    fn query_text_actions(&self, app: &App) -> Vec<ProcessTextAction> {
        let _ = app;
        Vec::new()
    }

    /// On Android, the readOnly parameter might be used by the targeted activity, see:
    /// https://developer.android.com/reference/android/content/Intent#EXTRA_PROCESS_TEXT_READONLY.
    fn process_text_action(
        &self,
        app: &App,
        id: &str,
        text: &str,
        read_only: bool,
    ) -> Option<String> {
        let _ = (app, id, text, read_only);
        None
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;

    use super::*;

    #[test]
    fn action_equality() {
        let action = ProcessTextAction::new("id", "Translate");
        assert_eq!(action, ProcessTextAction::new("id", "Translate"));
        assert_ne!(action, ProcessTextAction::new("id", "Share"));
    }

    #[test]
    fn default_service_is_empty() {
        let cell = AppCell::new();
        let app = cell.borrow_mut();
        let service = DefaultProcessTextService::new();
        assert!(service.query_text_actions(&app).is_empty());
        assert!(
            service
                .process_text_action(&app, "id", "hello", false)
                .is_none()
        );
    }
}
