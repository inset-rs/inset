//! Flutter counterpart: `widgets/text_selection_toolbar_anchors.dart`.

use reveal_embedder::{Offset, Rect, clamp_double};
use reveal_foundation::App;
use reveal_rendering::{AnyRenderBox, TextSelectionPoint};

/// The position information for a text selection toolbar.
///
/// Typically, a menu will attempt to position itself at [`primary_anchor`], and
/// if that's not possible, then it will use [`secondary_anchor`] instead, if it
/// exists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionToolbarAnchors {
    /// The location that the toolbar should attempt to position itself at.
    ///
    /// If the toolbar doesn't fit at this location, use [`secondary_anchor`] if it
    /// exists.
    pub primary_anchor: Offset,
    /// The fallback position that should be used if [`primary_anchor`] doesn't work.
    pub secondary_anchor: Option<Offset>,
}

impl TextSelectionToolbarAnchors {
    /// Creates an instance of [`TextSelectionToolbarAnchors`] directly from the
    /// anchor points.
    pub const fn new(primary_anchor: Offset) -> TextSelectionToolbarAnchors {
        TextSelectionToolbarAnchors {
            primary_anchor,
            secondary_anchor: None,
        }
    }

    /// Dart `TextSelectionToolbarAnchors(secondaryAnchor:)`.
    pub const fn secondary_anchor(
        mut self,
        secondary_anchor: Offset,
    ) -> TextSelectionToolbarAnchors {
        self.secondary_anchor = Some(secondary_anchor);
        self
    }

    /// Creates an instance of [`TextSelectionToolbarAnchors`] for some selection.
    pub fn from_selection(
        app: &App,
        render_box: AnyRenderBox,
        start_glyph_height: f64,
        end_glyph_height: f64,
        selection_endpoints: &[TextSelectionPoint],
    ) -> TextSelectionToolbarAnchors {
        let selection_rect = Self::get_selection_rect(
            app,
            render_box,
            start_glyph_height,
            end_glyph_height,
            selection_endpoints,
        );
        if selection_rect == Rect::ZERO {
            return TextSelectionToolbarAnchors::new(Offset::ZERO);
        }
        let editing_region = editing_region(app, render_box);
        TextSelectionToolbarAnchors::new(Offset::new(
            selection_rect.left + selection_rect.width() / 2.0,
            clamp_double(
                selection_rect.top,
                editing_region.top,
                editing_region.bottom,
            ),
        ))
        .secondary_anchor(Offset::new(
            selection_rect.left + selection_rect.width() / 2.0,
            clamp_double(
                selection_rect.bottom,
                editing_region.top,
                editing_region.bottom,
            ),
        ))
    }

    /// Returns the [`Rect`] covering the given selection in the given `RenderBox`
    /// in global coordinates.
    pub fn get_selection_rect(
        app: &App,
        render_box: AnyRenderBox,
        start_glyph_height: f64,
        end_glyph_height: f64,
        selection_endpoints: &[TextSelectionPoint],
    ) -> Rect {
        let editing_region = editing_region(app, render_box);
        if editing_region.left.is_nan()
            || editing_region.top.is_nan()
            || editing_region.right.is_nan()
            || editing_region.bottom.is_nan()
        {
            return Rect::ZERO;
        }
        if selection_endpoints.is_empty() {
            return Rect::ZERO;
        }
        let is_multiline = selection_endpoints.last().unwrap().point.dy()
            - selection_endpoints.first().unwrap().point.dy()
            > end_glyph_height / 2.0;
        Rect::from_ltrb(
            if is_multiline {
                editing_region.left
            } else {
                editing_region.left + selection_endpoints.first().unwrap().point.dx()
            },
            editing_region.top + selection_endpoints.first().unwrap().point.dy()
                - start_glyph_height,
            if is_multiline {
                editing_region.right
            } else {
                editing_region.left + selection_endpoints.last().unwrap().point.dx()
            },
            editing_region.top + selection_endpoints.last().unwrap().point.dy(),
        )
    }
}

fn editing_region(app: &App, render_box: AnyRenderBox) -> Rect {
    Rect::from_points(
        render_box.local_to_global(app, Offset::ZERO, None),
        render_box.local_to_global(app, render_box.size(app).bottom_right(Offset::ZERO), None),
    )
}
