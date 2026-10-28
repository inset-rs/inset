//! Flutter `widgets/constants.dart`.

/// The minimum dimension of any interactive region according to the Material
/// guidelines.
///
/// This is used to avoid small regions that are hard for the user to interact
/// with. It applies to both dimensions of a region, so a square of size
/// [`K_MIN_INTERACTIVE_DIMENSION`] x [`K_MIN_INTERACTIVE_DIMENSION`] is the smallest
/// acceptable region that should respond to gestures.
pub const K_MIN_INTERACTIVE_DIMENSION: f64 = 48.0;
