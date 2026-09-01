//! Flutter counterpart: dart:ui `painting.dart` (`Clip`).

/// Different ways to clip content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clip {
    /// No clip at all.
    None,

    /// Clip, but do not apply anti-aliasing.
    HardEdge,

    /// Clip with anti-aliasing.
    AntiAlias,

    /// Clip with anti-aliasing plus a save layer immediately after the clip.
    /// This is much slower.
    AntiAliasWithSaveLayer,
}
