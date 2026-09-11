//! Flutter counterpart: `rendering/image.dart` (`RenderImage`).
//!
//! Sizing is the interesting part: an image with no width or height given takes its own size,
//! and one given a single dimension keeps its proportions. That is what lets a picture sit in a
//! layout without the author having to know how big the file is.
//!
//! The colour blend, the nine-patch `centerSlice`, colour inversion and the filter-quality
//! choice Dart carries are not here; see `PORTING.md`.

use reveal_embedder::{Offset, Size, TextDirection};
use reveal_foundation::App;
use reveal_painting::{Alignment, BoxFit, ImageInfo, ImagePaintStyle, ImageRepeat, paint_image};

use crate::box_::{BoxConstraints, RenderBox, RenderBoxData};
use crate::object::{AnyRenderObject, RenderHandle, RenderObject, RenderObjectData};
use crate::painting_context::PaintingContext;
use reveal_painting::ClipContext;

/// Draws an image into the box it is given. Flutter `RenderImage`.
pub struct RenderImage {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    image: Option<ImageInfo>,
    width: Option<f64>,
    height: Option<f64>,
    opacity: f64,
    fit: Option<BoxFit>,
    alignment: Alignment,
    repeat: ImageRepeat,
    match_text_direction: bool,
    text_direction: Option<TextDirection>,
}

impl RenderImage {
    /// Creates an image render object with no image yet, which lays out to nothing.
    pub fn new(app: &mut App) -> RenderHandle<RenderImage> {
        RenderHandle::new_box(
            app,
            RenderImage {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                image: None,
                width: None,
                height: None,
                opacity: 1.0,
                fit: None,
                alignment: Alignment::CENTER,
                repeat: ImageRepeat::NoRepeat,
                match_text_direction: false,
                text_direction: None,
            },
        )
    }

    /// The image being drawn, if one has arrived.
    pub fn image(self: RenderHandle<Self>, app: &App) -> Option<ImageInfo> {
        self.get(app).image.clone()
    }

    /// Sets [`image`](Self::image).
    ///
    /// Laying out again is only needed while the size follows the image, which is whenever a
    /// width or a height was left unset.
    pub fn set_image(self: RenderHandle<Self>, app: &mut App, value: Option<ImageInfo>) {
        if self.get(app).image == value {
            return;
        }
        let follows_the_image = self.get(app).width.is_none() || self.get(app).height.is_none();
        self.get_mut(app).image = value;
        self.mark_needs_paint(app);
        if follows_the_image {
            self.mark_needs_layout(app);
        }
    }

    /// The width to take, or `None` to follow the image.
    pub fn width(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).width
    }

    /// Sets [`width`](Self::width).
    pub fn set_width(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).width == value {
            return;
        }
        self.get_mut(app).width = value;
        self.mark_needs_layout(app);
    }

    /// The height to take, or `None` to follow the image.
    pub fn height(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).height
    }

    /// Sets [`height`](Self::height).
    pub fn set_height(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).height == value {
            return;
        }
        self.get_mut(app).height = value;
        self.mark_needs_layout(app);
    }

    /// How opaque the image is drawn, from zero to one.
    pub fn opacity(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).opacity
    }

    /// Sets [`opacity`](Self::opacity).
    pub fn set_opacity(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).opacity == value {
            return;
        }
        self.get_mut(app).opacity = value;
        self.mark_needs_paint(app);
    }

    /// How the image is fitted to the box.
    pub fn fit(self: RenderHandle<Self>, app: &App) -> Option<BoxFit> {
        self.get(app).fit
    }

    /// Sets [`fit`](Self::fit).
    pub fn set_fit(self: RenderHandle<Self>, app: &mut App, value: Option<BoxFit>) {
        if self.get(app).fit == value {
            return;
        }
        self.get_mut(app).fit = value;
        self.mark_needs_paint(app);
    }

    /// Where the image sits when it does not fill the box.
    pub fn alignment(self: RenderHandle<Self>, app: &App) -> Alignment {
        self.get(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    pub fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: Alignment) {
        if self.get(app).alignment == value {
            return;
        }
        self.get_mut(app).alignment = value;
        self.mark_needs_paint(app);
    }

    /// What fills the space the image does not cover.
    pub fn repeat(self: RenderHandle<Self>, app: &App) -> ImageRepeat {
        self.get(app).repeat
    }

    /// Sets [`repeat`](Self::repeat).
    pub fn set_repeat(self: RenderHandle<Self>, app: &mut App, value: ImageRepeat) {
        if self.get(app).repeat == value {
            return;
        }
        self.get_mut(app).repeat = value;
        self.mark_needs_paint(app);
    }

    /// Whether the image is mirrored in a right-to-left layout, as a directional arrow is.
    pub fn match_text_direction(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).match_text_direction
    }

    /// Sets [`match_text_direction`](Self::match_text_direction).
    pub fn set_match_text_direction(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).match_text_direction == value {
            return;
        }
        self.get_mut(app).match_text_direction = value;
        self.mark_needs_paint(app);
    }

    /// The reading direction, which only matters with
    /// [`match_text_direction`](Self::match_text_direction).
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_paint(app);
    }

    /// The size this box takes under `constraints`. Dart's `_sizeForConstraints`.
    ///
    /// A width and a height both given make the size exact. Otherwise the image's own size is
    /// constrained while its proportions are kept, which is what makes one given dimension
    /// derive the other.
    fn size_for_constraints(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> Size {
        let held = self.get(app);
        let constraints = BoxConstraints::tight_for(held.width, held.height).enforce(constraints);
        let Some(image) = &held.image else {
            return constraints.smallest();
        };
        constraints.constrain_size_and_attempt_to_preserve_aspect_ratio(image.size())
    }

    /// The style `paint_image` is called with, assembled from this object's fields.
    fn paint_style(self: RenderHandle<Self>, app: &App) -> ImagePaintStyle {
        let held = self.get(app);
        let scale = held.image.as_ref().map_or(1.0, |image| image.scale);
        ImagePaintStyle {
            scale,
            opacity: held.opacity,
            fit: held.fit,
            alignment: held.alignment,
            repeat: held.repeat,
            flip_horizontally: held.match_text_direction
                && held.text_direction == Some(TextDirection::Rtl),
        }
    }
}

impl RenderObject for RenderImage {
    crate::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        _app: &App,
        _visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        // An image has no children.
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.size_for_constraints(app, constraints);
        self.set_size(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(image) = self.get(app).image.clone() else {
            return;
        };
        let size = self.size(app);
        let style = self.paint_style(app);
        paint_image(context.canvas(), offset & size, &image.image, &style);
    }
}

impl RenderBox for RenderImage {
    crate::render_box_accessors!();

    /// Dart returns zero for every intrinsic when a dimension is unset, because an image with no
    /// constraint has no preferred size of its own to report.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.compute_max_intrinsic_width(app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        debug_assert!(height >= 0.0);
        if self.get(app).width.is_none() && self.get(app).height.is_none() {
            return 0.0;
        }
        self.size_for_constraints(app, BoxConstraints::tight_for(None, Some(height)))
            .width()
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.compute_max_intrinsic_height(app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        debug_assert!(width >= 0.0);
        if self.get(app).width.is_none() && self.get(app).height.is_none() {
            return 0.0;
        }
        self.size_for_constraints(app, BoxConstraints::tight_for(Some(width), None))
            .height()
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.size_for_constraints(app, constraints)
    }

    /// An image absorbs a hit anywhere in its box, as every painted box does.
    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::test_support::solid_image;
    use reveal_foundation::AppCell;

    use super::*;

    /// A forty-by-twenty image, so a change of proportion is unmistakable.
    fn wide_image(scale: f64) -> ImageInfo {
        ImageInfo::new(solid_image([40, 20], [255, 0, 0, 255])).scale(scale)
    }

    fn loose(width: f64, height: f64) -> BoxConstraints {
        BoxConstraints::loose(Size::new(width, height))
    }

    #[test]
    fn an_image_that_has_not_arrived_takes_no_room() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderImage::new(&mut app);
        assert_eq!(
            render.compute_dry_layout(&mut app, loose(100.0, 100.0)),
            Size::new(0.0, 0.0)
        );
    }

    #[test]
    fn an_image_with_no_size_given_takes_its_own() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderImage::new(&mut app);
        render.set_image(&mut app, Some(wide_image(1.0)));
        assert_eq!(
            render.compute_dry_layout(&mut app, loose(1000.0, 1000.0)),
            Size::new(40.0, 20.0)
        );
    }

    #[test]
    fn a_scaled_image_takes_fewer_points_than_it_has_pixels() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderImage::new(&mut app);
        // A two-times asset: the same pixels over half the points.
        render.set_image(&mut app, Some(wide_image(2.0)));
        assert_eq!(
            render.compute_dry_layout(&mut app, loose(1000.0, 1000.0)),
            Size::new(20.0, 10.0)
        );
    }

    #[test]
    fn one_given_dimension_derives_the_other_from_the_proportions() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderImage::new(&mut app);
        render.set_image(&mut app, Some(wide_image(1.0)));

        render.set_width(&mut app, Some(80.0));
        assert_eq!(
            render.compute_dry_layout(&mut app, loose(1000.0, 1000.0)),
            Size::new(80.0, 40.0)
        );

        render.set_width(&mut app, None);
        render.set_height(&mut app, Some(5.0));
        assert_eq!(
            render.compute_dry_layout(&mut app, loose(1000.0, 1000.0)),
            Size::new(10.0, 5.0)
        );
    }

    #[test]
    fn both_dimensions_given_make_the_size_exact() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderImage::new(&mut app);
        render.set_image(&mut app, Some(wide_image(1.0)));
        render.set_width(&mut app, Some(7.0));
        render.set_height(&mut app, Some(9.0));
        assert_eq!(
            render.compute_dry_layout(&mut app, loose(1000.0, 1000.0)),
            Size::new(7.0, 9.0)
        );
    }

    #[test]
    fn a_box_too_small_for_the_image_constrains_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderImage::new(&mut app);
        render.set_image(&mut app, Some(wide_image(1.0)));
        // Half the width available, so the proportions carry the height down with it.
        assert_eq!(
            render.compute_dry_layout(&mut app, loose(20.0, 1000.0)),
            Size::new(20.0, 10.0)
        );
    }

    #[test]
    fn an_image_with_no_dimension_given_reports_no_intrinsic_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderImage::new(&mut app);
        render.set_image(&mut app, Some(wide_image(1.0)));
        // Dart's rule: without a width or a height there is no preferred size to report.
        assert_eq!(render.compute_max_intrinsic_width(&mut app, 100.0), 0.0);
        assert_eq!(render.compute_max_intrinsic_height(&mut app, 100.0), 0.0);

        render.set_width(&mut app, Some(80.0));
        assert_eq!(render.compute_max_intrinsic_height(&mut app, 80.0), 40.0);
    }
}
