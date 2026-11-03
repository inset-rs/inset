//! A camera's formats as the app knows them: each of a device's `AVCaptureDeviceFormat`s as the
//! size of its pictures, the frame rates it keeps and the sizes it takes photos at, and back from
//! such a format to the device's own. Formats that differ only in what the app cannot see, such as
//! their pixels' layout, are listed once.
//!
//! The invariant: a device's format is described the same way each time, so that a format the app
//! picked from the list finds its device format again.

use inset_embedder::{CameraFormat, Size};
use objc2::rc::Retained;
use objc2::{ClassType, sel};
use objc2_av_foundation::{AVCaptureDevice, AVCaptureDeviceFormat};
use objc2_core_media::{CMVideoDimensions, CMVideoFormatDescriptionGetDimensions};

/// The formats `device` runs in, in the system's order.
pub(super) fn formats_of(device: &AVCaptureDevice) -> Vec<CameraFormat> {
    let mut formats: Vec<CameraFormat> = Vec::new();
    // SAFETY: reading a device's formats.
    for device_format in unsafe { device.formats() }.iter() {
        let format = camera_format_of(&device_format);
        if !formats.contains(&format) {
            formats.push(format);
        }
    }
    formats
}

/// The device's own format that `format` was listed from: the first that is described as it is.
pub(super) fn device_format_for(
    device: &AVCaptureDevice,
    format: &CameraFormat,
) -> Option<Retained<AVCaptureDeviceFormat>> {
    // SAFETY: reading a device's formats.
    unsafe { device.formats() }
        .iter()
        .find(|device_format| camera_format_of(device_format) == *format)
}

/// The format `device` runs in now.
pub(super) fn active_format_of(device: &AVCaptureDevice) -> CameraFormat {
    // SAFETY: reading a device's active format.
    let active = unsafe { device.activeFormat() };
    camera_format_of(&active)
}

/// `size`, one of a format's photo sizes, as AVFoundation names a photo's dimensions.
pub(super) fn photo_dimensions_of(size: Size) -> CMVideoDimensions {
    CMVideoDimensions {
        width: size.width() as i32,
        height: size.height() as i32,
    }
}

/// `device_format` as the app knows it.
fn camera_format_of(device_format: &AVCaptureDeviceFormat) -> CameraFormat {
    let size = size_of(device_format);
    let (min_frame_rate, max_frame_rate) = frame_rates_of(device_format);
    CameraFormat {
        size,
        min_frame_rate,
        max_frame_rate,
        photo_sizes: photo_sizes_of(device_format, size),
    }
}

/// The size of the pictures `device_format` gives, in the camera's own pixels.
fn size_of(device_format: &AVCaptureDeviceFormat) -> Size {
    // SAFETY: a capture device's format describes video, whose dimensions this reads.
    let dimensions =
        unsafe { CMVideoFormatDescriptionGetDimensions(&device_format.formatDescription()) };
    size_of_dimensions(dimensions)
}

/// The slowest and the fastest frame rates `device_format` keeps, over all its ranges; none for a
/// format that lists no range.
fn frame_rates_of(device_format: &AVCaptureDeviceFormat) -> (f64, f64) {
    // SAFETY: reading a format's frame rate ranges.
    let ranges = unsafe { device_format.videoSupportedFrameRateRanges() };
    // SAFETY: reading each range's rates.
    let rates: Vec<(f64, f64)> = ranges
        .iter()
        .map(|range| unsafe { (range.minFrameRate(), range.maxFrameRate()) })
        .collect();
    let slowest = rates.iter().map(|rates| rates.0).reduce(f64::min);
    let fastest = rates.iter().map(|rates| rates.1).reduce(f64::max);
    (slowest.unwrap_or(0.0), fastest.unwrap_or(0.0))
}

/// The sizes `device_format` takes photos at: the system's list from macOS 13 and iOS 16; before,
/// the format's own size, at which Apple's documentation of `highResolutionCaptureEnabled` has a
/// photo output take its stills unless asked for more.
fn photo_sizes_of(device_format: &AVCaptureDeviceFormat, size: Size) -> Vec<Size> {
    if !AVCaptureDeviceFormat::class().responds_to(sel!(supportedMaxPhotoDimensions)) {
        return vec![size];
    }
    // SAFETY: the list's values hold `CMVideoDimensions`, as Apple documents.
    unsafe { device_format.supportedMaxPhotoDimensions() }
        .iter()
        .map(|value| size_of_dimensions(unsafe { value.get::<CMVideoDimensions>() }))
        .collect()
}

fn size_of_dimensions(dimensions: CMVideoDimensions) -> Size {
    Size::new(f64::from(dimensions.width), f64::from(dimensions.height))
}

#[cfg(test)]
mod tests {
    use super::super::camera::discovery_session;
    use super::*;

    /// Reading a camera's formats asks the user nothing; this Mac's cameras, if it has any, each
    /// list formats of a size, a frame rate range and a photo size, and each finds its device
    /// format again.
    #[test]
    fn a_cameras_formats_are_listed_without_asking_and_find_their_device_formats() {
        // SAFETY: reading the devices a discovery session found.
        let devices = unsafe { discovery_session().devices() };
        for device in devices.iter() {
            let formats = formats_of(&device);
            assert!(!formats.is_empty());
            for format in &formats {
                assert!(format.size.width() > 0.0 && format.size.height() > 0.0);
                assert!(format.min_frame_rate <= format.max_frame_rate);
                assert!(!format.photo_sizes.is_empty());
                let device_format = device_format_for(&device, format).expect("listed from it");
                assert_eq!(camera_format_of(&device_format), *format);
            }
            assert_eq!(
                formats
                    .iter()
                    .filter(|format| **format == formats[0])
                    .count(),
                1,
                "a format is listed once"
            );
        }
    }
}
