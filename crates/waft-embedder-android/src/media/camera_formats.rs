//! A camera's formats as the app knows them, read from what Android says of the camera before it
//! is opened: each size it streams for the GPU and the compositor at (its `PRIVATE` output
//! stream configurations), the frame rates it keeps there (its auto-exposure target ranges that
//! the size's minimum frame duration allows), and the sizes it takes photos at (its JPEG output
//! sizes, any of which Android takes beside any stream).
//!
//! The invariant: a format is described the same way each time from the same characteristics, so
//! a format the app picked from the list is found in it again.

use waft_embedder::{CameraFormat, Size};

/// `AIMAGE_FORMAT_PRIVATE`: the camera's own format for the GPU and the compositor.
pub(super) const PRIVATE: i32 = 0x22;
/// `AIMAGE_FORMAT_JPEG`.
pub(super) const JPEG: i32 = 0x100;
/// `ACAMERA_SCALER_AVAILABLE_STREAM_CONFIGURATIONS_OUTPUT`.
const OUTPUT: i32 = 0;
/// Nanoseconds in a second.
const NANOS: f64 = 1e9;

/// What Android says of a camera's streams, as its characteristics hold them.
pub(super) struct StreamCharacteristics {
    /// `SCALER_AVAILABLE_STREAM_CONFIGURATIONS`: format, width, height and whether it is an input,
    /// four values a configuration.
    pub(super) configurations: Vec<i32>,
    /// `SCALER_AVAILABLE_MIN_FRAME_DURATIONS`: format, width, height and the least nanoseconds
    /// between frames, four values a size.
    pub(super) min_frame_durations: Vec<i64>,
    /// `CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES`: the slowest and the fastest frame rate, two
    /// values a range.
    pub(super) frame_rate_ranges: Vec<i32>,
}

impl StreamCharacteristics {
    /// The formats the camera runs in, in Android's order of its sizes.
    pub(super) fn formats(&self) -> Vec<CameraFormat> {
        let photo_sizes: Vec<Size> = self.output_sizes(JPEG).map(size_of).collect();
        self.output_sizes(PRIVATE)
            .map(|size| {
                let (min_frame_rate, max_frame_rate) = self.frame_rates_at(size);
                CameraFormat {
                    size: size_of(size),
                    min_frame_rate,
                    max_frame_rate,
                    photo_sizes: photo_sizes.clone(),
                }
            })
            .collect()
    }

    /// The auto-exposure target range a session in `format` asks for: of those the format's size
    /// allows, the fastest, and of those the widest; `None` where the camera lists none.
    pub(super) fn target_frame_rates(&self, format: &CameraFormat) -> Option<[i32; 2]> {
        let size = [format.size.width() as i32, format.size.height() as i32];
        self.ranges_at(size)
            .max_by_key(|&[slowest, fastest]| (fastest, -slowest))
    }

    /// Each size the camera outputs `format` at, once, in Android's order.
    fn output_sizes(&self, format: i32) -> impl Iterator<Item = [i32; 2]> + '_ {
        let mut seen = Vec::new();
        self.configurations
            .as_chunks::<4>()
            .0
            .iter()
            .filter(move |configuration| configuration[0] == format && configuration[3] == OUTPUT)
            .map(|configuration| [configuration[1], configuration[2]])
            .filter(move |size| {
                let new = !seen.contains(size);
                seen.push(*size);
                new
            })
    }

    /// The slowest and the fastest frame rates the camera keeps at `size`: over the target ranges
    /// the size allows, or the size's fastest alone where it allows none.
    fn frame_rates_at(&self, size: [i32; 2]) -> (f64, f64) {
        let ranges: Vec<[i32; 2]> = self.ranges_at(size).collect();
        let slowest = ranges.iter().map(|range| range[0]).min();
        let fastest = ranges.iter().map(|range| range[1]).max();
        match (slowest, fastest) {
            (Some(slowest), Some(fastest)) => (f64::from(slowest), f64::from(fastest)),
            _ => {
                let fastest = self.fastest_at(size).unwrap_or(0.0);
                (fastest, fastest)
            }
        }
    }

    /// The target ranges whose fastest rate `size`'s minimum frame duration allows.
    fn ranges_at(&self, size: [i32; 2]) -> impl Iterator<Item = [i32; 2]> + '_ {
        let fastest = self.fastest_at(size).unwrap_or(f64::INFINITY);
        self.frame_rate_ranges
            .as_chunks::<2>()
            .0
            .iter()
            .map(|range| [range[0], range[1]])
            .filter(move |range| f64::from(range[1]) <= fastest + 0.5)
    }

    /// The fastest rate the camera streams `size` at for the GPU; `None` where Android gives no
    /// minimum duration for it.
    fn fastest_at(&self, size: [i32; 2]) -> Option<f64> {
        self.min_frame_durations
            .as_chunks::<4>()
            .0
            .iter()
            .find(|duration| {
                duration[0] == i64::from(PRIVATE)
                    && duration[1] == i64::from(size[0])
                    && duration[2] == i64::from(size[1])
            })
            .map(|duration| duration[3])
            .filter(|&nanos| nanos > 0)
            .map(|nanos| rounded(NANOS / nanos as f64))
    }
}

/// `rate` to a thousandth of a frame a second: Android's durations are whole nanoseconds, so a
/// size at 30 frames a second lasts 33,333,333 of them, a rate a little over 30.
fn rounded(rate: f64) -> f64 {
    (rate * 1000.0).round() / 1000.0
}

fn size_of([width, height]: [i32; 2]) -> Size {
    Size::new(f64::from(width), f64::from(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The emulator's back camera, cut down: two sizes for the GPU, one at 30 and one at 60
    /// frames a second, JPEG at two sizes, an input stream, and ranges of 15 to 30, 30 and 60.
    fn camera() -> StreamCharacteristics {
        StreamCharacteristics {
            configurations: vec![
                PRIVATE, 1920, 1080, 0, //
                PRIVATE, 640, 480, 0, //
                PRIVATE, 640, 480, 0, //
                JPEG, 1920, 1080, 0, //
                JPEG, 640, 480, 0, //
                PRIVATE, 320, 240, 1, //
            ],
            min_frame_durations: vec![
                i64::from(PRIVATE),
                1920,
                1080,
                33_333_333, //
                i64::from(PRIVATE),
                640,
                480,
                16_666_666, //
            ],
            frame_rate_ranges: vec![15, 30, 30, 30, 60, 60],
        }
    }

    #[test]
    fn each_output_size_is_a_format_with_the_rates_it_allows_and_every_photo_size() {
        let formats = camera().formats();
        assert_eq!(formats.len(), 2, "listed once each, inputs left out");
        assert_eq!(formats[0].size, Size::new(1920.0, 1080.0));
        assert_eq!(
            (formats[0].min_frame_rate, formats[0].max_frame_rate),
            (15.0, 30.0)
        );
        assert_eq!(
            (formats[1].min_frame_rate, formats[1].max_frame_rate),
            (15.0, 60.0)
        );
        assert_eq!(
            formats[1].photo_sizes,
            vec![Size::new(1920.0, 1080.0), Size::new(640.0, 480.0)]
        );
    }

    #[test]
    fn a_session_asks_for_the_fastest_widest_range_its_size_allows() {
        let camera = camera();
        let formats = camera.formats();
        assert_eq!(camera.target_frame_rates(&formats[0]), Some([15, 30]));
        assert_eq!(camera.target_frame_rates(&formats[1]), Some([60, 60]));
    }

    #[test]
    fn a_size_with_no_range_it_allows_runs_at_its_fastest() {
        let camera = StreamCharacteristics {
            frame_rate_ranges: vec![60, 60],
            ..camera()
        };
        let formats = camera.formats();
        assert_eq!(
            (formats[0].min_frame_rate, formats[0].max_frame_rate),
            (30.0, 30.0)
        );
    }
}
