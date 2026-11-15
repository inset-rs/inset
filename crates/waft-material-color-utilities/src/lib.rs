//! Port of Dart's `material_color_utilities` 0.13.0 (Google LLC, Apache-2.0): the colour
//! science behind Material Design's dynamic colour.
//!
//! One module per Dart file, in modules named for the package's directories. The crate root
//! re-exports what the package's `material_color_utilities.dart` exports, so a caller names
//! `Hct` or `MaterialDynamicColors` as a Dart caller does.

mod dart_math;
#[cfg(test)]
mod test_support;

pub mod blend;
pub mod contrast;
pub mod dislike;
pub mod dynamiccolor;
pub mod hct;
pub mod palettes;
pub mod quantize;
pub mod scheme;
pub mod score;
pub mod temperature;
pub mod utils;

pub use blend::blend::*;
pub use contrast::contrast::*;
pub use dislike::dislike_analyzer::*;
pub use dynamiccolor::dynamic_color::*;
pub use dynamiccolor::dynamic_scheme::*;
pub use dynamiccolor::material_dynamic_colors::*;
pub use dynamiccolor::variant::*;
pub use hct::cam16::*;
pub use hct::hct::*;
pub use hct::viewing_conditions::*;
#[allow(deprecated)]
pub use palettes::core_palette::*;
pub use palettes::tonal_palette::*;
pub use quantize::quantizer::*;
pub use quantize::quantizer_celebi::*;
pub use quantize::quantizer_map::*;
pub use quantize::quantizer_wsmeans::*;
// Wu's `Box` stays at `quantize::quantizer_wu`: it would shadow the prelude's `Box` in a crate
// that glob-imports this one.
pub use quantize::quantizer_wu::{CreateBoxesResult, Direction, MaximizeResult, QuantizerWu};
#[allow(deprecated)]
pub use scheme::scheme::*;
pub use scheme::scheme_content::*;
pub use scheme::scheme_expressive::*;
pub use scheme::scheme_fidelity::*;
pub use scheme::scheme_fruit_salad::*;
pub use scheme::scheme_monochrome::*;
pub use scheme::scheme_neutral::*;
pub use scheme::scheme_rainbow::*;
pub use scheme::scheme_tonal_spot::*;
pub use scheme::scheme_vibrant::*;
pub use score::score::*;
pub use temperature::temperature_cache::*;

pub use utils::color_utils::*;
pub use utils::math_utils::*;
pub use utils::string_utils::*;
