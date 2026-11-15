//! Port of material_ui's `shadows.dart`.
//!
//! Dart's `const Map<int, List<BoxShadow>>` is [`k_elevation_to_shadow`]: Rust has no `const` map, so the map is a `const fn` lookup over its keys, as waft-cupertino's constants do.

use waft_embedder::{BlurStyle, Color, Offset};
use waft_painting::BoxShadow;

// Based on https://material.io/design/environment/elevation.html
// Currently, only the elevation values that are bound to one or more widgets are
// defined here.

const K_KEY_UMBRA_OPACITY: Color = Color::new(0x33000000); // alpha = 0.2
const K_KEY_PENUMBRA_OPACITY: Color = Color::new(0x24000000); // alpha = 0.14
const K_AMBIENT_SHADOW_OPACITY: Color = Color::new(0x1F000000); // alpha = 0.12

const fn box_shadow(color: Color, dy: f64, blur_radius: f64, spread_radius: f64) -> BoxShadow {
    BoxShadow {
        color,
        offset: Offset::new(0.0, dy),
        blur_radius,
        spread_radius,
        blur_style: BlurStyle::Normal,
    }
}

/// Map of elevation offsets used by Material Design to [`BoxShadow`] definitions.
///
/// The following elevations have defined shadows: 1, 2, 3, 4, 6, 8, 9, 12, 16, 24.
/// Any other elevation answers `None`, as Dart's map answers null.
///
/// Each entry has three shadows which must be combined to obtain the defined
/// effect for that elevation.
///
/// This is useful when simulating a shadow with a `BoxDecoration` or other
/// class that uses a list of [`BoxShadow`] objects.
///
/// Shadows defined by [`k_elevation_to_shadow`] use [`BlurStyle::Normal`]. To convert a
/// shadow from [`k_elevation_to_shadow`] to use a different [`BlurStyle`] (e.g. to use
/// it in a `MagnifierDecoration`), consider an expression such as the
/// following:
///
/// ```ignore
/// k_elevation_to_shadow(12).unwrap().iter().map(|shadow| shadow.copy_with(None, None, None, None, Some(BlurStyle::Outer))).collect::<Vec<_>>()
/// ```
///
/// See also:
///
///  * `Material`, which takes an arbitrary double for its elevation and generates
///    a shadow dynamically.
///  * <https://material.io/design/environment/elevation.html>
pub const fn k_elevation_to_shadow(elevation: i32) -> Option<&'static [BoxShadow]> {
    match elevation {
        // The empty list depicts no elevation.
        0 => const { Some(&[]) },
        1 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 2.0, 1.0, -1.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 1.0, 1.0, 0.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 1.0, 3.0, 0.0),
                ])
            }
        }
        2 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 3.0, 1.0, -2.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 2.0, 2.0, 0.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 1.0, 5.0, 0.0),
                ])
            }
        }
        3 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 3.0, 3.0, -2.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 3.0, 4.0, 0.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 1.0, 8.0, 0.0),
                ])
            }
        }
        4 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 2.0, 4.0, -1.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 4.0, 5.0, 0.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 1.0, 10.0, 0.0),
                ])
            }
        }
        6 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 3.0, 5.0, -1.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 6.0, 10.0, 0.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 1.0, 18.0, 0.0),
                ])
            }
        }
        8 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 5.0, 5.0, -3.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 8.0, 10.0, 1.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 3.0, 14.0, 2.0),
                ])
            }
        }
        9 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 5.0, 6.0, -3.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 9.0, 12.0, 1.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 3.0, 16.0, 2.0),
                ])
            }
        }
        12 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 7.0, 8.0, -4.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 12.0, 17.0, 2.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 5.0, 22.0, 4.0),
                ])
            }
        }
        16 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 8.0, 10.0, -5.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 16.0, 24.0, 2.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 6.0, 30.0, 5.0),
                ])
            }
        }
        24 => {
            const {
                Some(&[
                    box_shadow(K_KEY_UMBRA_OPACITY, 11.0, 15.0, -7.0),
                    box_shadow(K_KEY_PENUMBRA_OPACITY, 24.0, 38.0, 3.0),
                    box_shadow(K_AMBIENT_SHADOW_OPACITY, 9.0, 46.0, 8.0),
                ])
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevations_with_shadows_have_three_and_zero_has_none() {
        assert_eq!(k_elevation_to_shadow(0).unwrap().len(), 0);
        for elevation in [1, 2, 3, 4, 6, 8, 9, 12, 16, 24] {
            assert_eq!(k_elevation_to_shadow(elevation).unwrap().len(), 3);
        }
    }

    #[test]
    fn elevations_without_shadows_are_missing() {
        assert!(k_elevation_to_shadow(5).is_none());
        assert!(k_elevation_to_shadow(-1).is_none());
    }

    #[test]
    fn elevation_24_matches_the_material_spec() {
        let shadows = k_elevation_to_shadow(24).unwrap();
        assert_eq!(
            shadows[0],
            box_shadow(Color::new(0x33000000), 11.0, 15.0, -7.0)
        );
        assert_eq!(
            shadows[1],
            box_shadow(Color::new(0x24000000), 24.0, 38.0, 3.0)
        );
        assert_eq!(
            shadows[2],
            box_shadow(Color::new(0x1F000000), 9.0, 46.0, 8.0)
        );
    }
}
