//! Flutter counterpart: `painting/debug.dart` (`debugDisableShadows` only).

use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG_DISABLE_SHADOWS: AtomicBool = AtomicBool::new(false);

/// Whether to replace all shadows with solid color blocks.
///
/// This is useful when writing golden file tests since the rendering of shadows
/// is not guaranteed to be pixel-for-pixel identical from version to version
/// (or even from run to run).
///
/// When this is set, [`crate::BoxShadow::to_paint`] acts as if the
/// [`crate::BoxShadow::blur_style`] was [`crate::BlurStyle::Normal`] regardless of the
/// actual specified blur style. This is compensated for in `BoxDecoration` and
/// `ShapeDecoration` but may need to be explicitly considered in other
/// situations.
///
/// In Flutter this is a library-level `bool`. Assignment is
/// [`set_debug_disable_shadows`].
pub fn debug_disable_shadows() -> bool {
    DEBUG_DISABLE_SHADOWS.load(Ordering::Relaxed)
}

/// Sets [`debug_disable_shadows`].
pub fn set_debug_disable_shadows(value: bool) {
    DEBUG_DISABLE_SHADOWS.store(value, Ordering::Relaxed);
}
