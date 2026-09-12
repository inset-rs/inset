//! Flutter counterpart: `painting/box_fit.dart`.

use inset_embedder::Size;

/// How a box should be inscribed into another box.
///
/// See also:
///
///  * [`apply_box_fit`], which applies the sizing semantics of these values
///    (though not the alignment semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxFit {
    /// Fill the target box by distorting the source's aspect ratio.
    Fill,

    /// As large as possible while still containing the source entirely within
    /// the target box.
    Contain,

    /// As small as possible while still covering the entire target box.
    Cover,

    /// Make sure the full width of the source is shown, regardless of whether
    /// this means the source overflows the target box vertically.
    FitWidth,

    /// Make sure the full height of the source is shown, regardless of whether
    /// this means the source overflows the target box horizontally.
    FitHeight,

    /// Align the source within the target box (by default, centering) and
    /// discard any portions of the source that lie outside the box.
    ///
    /// The source image is not resized.
    None,

    /// Align the source within the target box (by default, centering) and, if
    /// necessary, scale the source down to ensure that the source fits within
    /// the box.
    ///
    /// This is the same as [`BoxFit::Contain`] if that would shrink the image,
    /// otherwise it is the same as [`BoxFit::None`].
    ScaleDown,
}

/// The pair of sizes returned by [`apply_box_fit`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FittedSizes {
    /// The size of the part of the input to show on the output.
    pub source: Size,
    /// The size of the part of the output on which to show the input.
    pub destination: Size,
}

impl FittedSizes {
    /// Creates an object to store a pair of sizes, as would be returned by
    /// [`apply_box_fit`].
    pub const fn new(source: Size, destination: Size) -> FittedSizes {
        FittedSizes {
            source,
            destination,
        }
    }
}

/// Apply a [`BoxFit`] value.
///
/// The arguments to this method, in addition to the [`BoxFit`] value to apply,
/// are two sizes, ostensibly the sizes of an input box and an output box.
/// Specifically, the `input_size` argument gives the size of the complete
/// source that is being fitted, and the `output_size` gives the size of the
/// rectangle into which the source is to be drawn.
///
/// This function then returns two sizes, combined into a single [`FittedSizes`]
/// object.
///
/// The [`FittedSizes::source`] size is the subpart of the `input_size` that is
/// to be shown. If the entire input source is shown, then this will equal the
/// `input_size`, but if the input source is to be cropped down, this may be
/// smaller.
///
/// The [`FittedSizes::destination`] size is the subpart of the `output_size` in
/// which to paint the (possibly cropped) source. If the
/// [`FittedSizes::destination`] size is smaller than the `output_size` then the
/// source is being letterboxed (or pillarboxed).
///
/// This method does not express an opinion regarding the alignment of the
/// source and destination sizes within the input and output rectangles.
/// Typically they are centered.
pub fn apply_box_fit(fit: BoxFit, input_size: Size, output_size: Size) -> FittedSizes {
    if input_size.height() <= 0.0
        || input_size.width() <= 0.0
        || output_size.height() <= 0.0
        || output_size.width() <= 0.0
    {
        return FittedSizes::new(Size::ZERO, Size::ZERO);
    }

    let (source_size, destination_size) = match fit {
        BoxFit::Fill => (input_size, output_size),
        BoxFit::Contain => {
            let source_size = input_size;
            let destination_size = if output_size.width() / output_size.height()
                > source_size.width() / source_size.height()
            {
                Size::new(
                    source_size.width() * output_size.height() / source_size.height(),
                    output_size.height(),
                )
            } else {
                Size::new(
                    output_size.width(),
                    source_size.height() * output_size.width() / source_size.width(),
                )
            };
            (source_size, destination_size)
        }
        BoxFit::Cover => {
            let source_size = if output_size.width() / output_size.height()
                > input_size.width() / input_size.height()
            {
                Size::new(
                    input_size.width(),
                    input_size.width() * output_size.height() / output_size.width(),
                )
            } else {
                Size::new(
                    input_size.height() * output_size.width() / output_size.height(),
                    input_size.height(),
                )
            };
            (source_size, output_size)
        }
        BoxFit::FitWidth => {
            if output_size.width() / output_size.height() > input_size.width() / input_size.height()
            {
                // Like "cover"
                (
                    Size::new(
                        input_size.width(),
                        input_size.width() * output_size.height() / output_size.width(),
                    ),
                    output_size,
                )
            } else {
                // Like "contain"
                let source_size = input_size;
                (
                    source_size,
                    Size::new(
                        output_size.width(),
                        source_size.height() * output_size.width() / source_size.width(),
                    ),
                )
            }
        }
        BoxFit::FitHeight => {
            if output_size.width() / output_size.height() > input_size.width() / input_size.height()
            {
                // Like "contain"
                let source_size = input_size;
                (
                    source_size,
                    Size::new(
                        source_size.width() * output_size.height() / source_size.height(),
                        output_size.height(),
                    ),
                )
            } else {
                // Like "cover"
                (
                    Size::new(
                        input_size.height() * output_size.width() / output_size.height(),
                        input_size.height(),
                    ),
                    output_size,
                )
            }
        }
        BoxFit::None => {
            let source_size = Size::new(
                input_size.width().min(output_size.width()),
                input_size.height().min(output_size.height()),
            );
            (source_size, source_size)
        }
        BoxFit::ScaleDown => {
            let source_size = input_size;
            let mut destination_size = input_size;
            let aspect_ratio = input_size.width() / input_size.height();
            if destination_size.height() > output_size.height() {
                destination_size =
                    Size::new(output_size.height() * aspect_ratio, output_size.height());
            }
            if destination_size.width() > output_size.width() {
                destination_size =
                    Size::new(output_size.width(), output_size.width() / aspect_ratio);
            }
            (source_size, destination_size)
        }
    };
    FittedSizes::new(source_size, destination_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_zero_and_negative_sizes(fit: BoxFit) {
        let zero = FittedSizes::new(Size::ZERO, Size::ZERO);
        assert_eq!(
            apply_box_fit(fit, Size::new(-400.0, 2000.0), Size::new(100.0, 1000.0)),
            zero
        );
        assert_eq!(
            apply_box_fit(fit, Size::new(400.0, -2000.0), Size::new(100.0, 1000.0)),
            zero
        );
        assert_eq!(
            apply_box_fit(fit, Size::new(400.0, 2000.0), Size::new(-100.0, 1000.0)),
            zero
        );
        assert_eq!(
            apply_box_fit(fit, Size::new(400.0, 2000.0), Size::new(100.0, -1000.0)),
            zero
        );
        assert_eq!(
            apply_box_fit(fit, Size::new(0.0, 2000.0), Size::new(100.0, 1000.0)),
            zero
        );
        assert_eq!(
            apply_box_fit(fit, Size::new(400.0, 0.0), Size::new(100.0, 1000.0)),
            zero
        );
        assert_eq!(
            apply_box_fit(fit, Size::new(400.0, 2000.0), Size::new(0.0, 1000.0)),
            zero
        );
        assert_eq!(
            apply_box_fit(fit, Size::new(400.0, 2000.0), Size::new(100.0, 0.0)),
            zero
        );
    }

    // box_fit_test.dart 'applyBoxFit'.
    #[test]
    fn apply_box_fit_matches_dart() {
        let result = apply_box_fit(
            BoxFit::ScaleDown,
            Size::new(100.0, 1000.0),
            Size::new(200.0, 2000.0),
        );
        assert_eq!(result.source, Size::new(100.0, 1000.0));
        assert_eq!(result.destination, Size::new(100.0, 1000.0));

        let result = apply_box_fit(
            BoxFit::ScaleDown,
            Size::new(300.0, 3000.0),
            Size::new(200.0, 2000.0),
        );
        assert_eq!(result.source, Size::new(300.0, 3000.0));
        assert_eq!(result.destination, Size::new(200.0, 2000.0));

        let result = apply_box_fit(
            BoxFit::FitWidth,
            Size::new(2000.0, 400.0),
            Size::new(1000.0, 100.0),
        );
        assert_eq!(result.source, Size::new(2000.0, 200.0));
        assert_eq!(result.destination, Size::new(1000.0, 100.0));

        let result = apply_box_fit(
            BoxFit::FitWidth,
            Size::new(2000.0, 400.0),
            Size::new(1000.0, 300.0),
        );
        assert_eq!(result.source, Size::new(2000.0, 400.0));
        assert_eq!(result.destination, Size::new(1000.0, 200.0));

        let result = apply_box_fit(
            BoxFit::FitHeight,
            Size::new(400.0, 2000.0),
            Size::new(100.0, 1000.0),
        );
        assert_eq!(result.source, Size::new(200.0, 2000.0));
        assert_eq!(result.destination, Size::new(100.0, 1000.0));

        let result = apply_box_fit(
            BoxFit::FitHeight,
            Size::new(400.0, 2000.0),
            Size::new(300.0, 1000.0),
        );
        assert_eq!(result.source, Size::new(400.0, 2000.0));
        assert_eq!(result.destination, Size::new(200.0, 1000.0));

        test_zero_and_negative_sizes(BoxFit::Fill);
        test_zero_and_negative_sizes(BoxFit::Contain);
        test_zero_and_negative_sizes(BoxFit::Cover);
        test_zero_and_negative_sizes(BoxFit::FitWidth);
        test_zero_and_negative_sizes(BoxFit::FitHeight);
        test_zero_and_negative_sizes(BoxFit::None);
        test_zero_and_negative_sizes(BoxFit::ScaleDown);
    }
}
