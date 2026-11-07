//! What the system says changed about the machine beyond what winit reports: its screens, and the
//! user's languages. The main loop tells the app (`main_loop/app.rs`), which reads the screens as
//! the system has them by then (`screens.rs`) and the languages as the embedder read them again
//! (`locales.rs`).

/// A change the system reports, which Flutter's macOS embedder hears through the notifications
/// named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(target_os = "macos"),
    expect(dead_code, reason = "only macOS says these")
)]
pub(crate) enum SystemChange {
    /// A screen was added or removed, or changed its place, size, scale or rate: AppKit's
    /// `NSApplicationDidChangeScreenParametersNotification`.
    Screens,
    /// The user's languages or region changed: Foundation's
    /// `NSCurrentLocaleDidChangeNotification`.
    Locales,
}
