//! Flutter counterpart: `cupertino/desktop_text_selection_toolbar.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Clip, Color, ColorFilter, ImageFilter, Offset, Radius};
use reveal_foundation::App;
use reveal_painting::{
    BorderRadius, BorderSide, BorderStyle, BoxShadow, EdgeInsets, EdgeInsetsGeometry,
    RoundedSuperellipseBorder, ShapeDecoration,
};
use reveal_rendering::MainAxisSize;
use reveal_widgets::{
    BackdropFilter, BuildContext, Column, Container, CustomSingleChildLayout, DecoratedBox,
    DesktopTextSelectionToolbarLayoutDelegate, IntoWidget, KeyRef, MediaQuery, Padding,
    StatelessWidget, WidgetRef,
};

use crate::colors::CupertinoDynamicColor;

const K_TOOLBAR_SCREEN_PADDING: f64 = 8.0;
const K_TOOLBAR_SATURATION_BOOST: f64 = 3.0;
const K_TOOLBAR_BLUR_SIGMA: f64 = 20.0;
const K_TOOLBAR_WIDTH: f64 = 222.0;
const K_TOOLBAR_PADDING: EdgeInsets = EdgeInsets::all(6.0);

fn k_toolbar_border_radius() -> Radius {
    Radius::circular(8.0)
}

fn k_toolbar_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow::new(
        Color::from_argb(60, 0, 0, 0),
        Offset::new(0.0, 4.0),
        10.0,
        0.5,
        reveal_embedder::BlurStyle::Normal,
    )]
}

const K_TOOLBAR_BORDER_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFB8B8B8), Color::new(0xFF5B5B5B));
const K_TOOLBAR_BACKGROUND_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xB2FFFFFF), Color::new(0xB2303030));

/// A macOS-style text selection toolbar.
///
/// Typically displays buttons for text manipulation, e.g. copying and pasting text.
///
/// Tries to position itself as closely as possible to [`anchor`](Self::anchor) while remaining
/// fully inside the viewport.
///
/// See also:
///
///  * [`crate::CupertinoAdaptiveTextSelectionToolbar`], where this is used to build the
///    toolbar for desktop platforms.
///  * [`CupertinoDesktopTextSelectionToolbarButton`](crate::CupertinoDesktopTextSelectionToolbarButton),
///    which builds a default macOS-style text selection toolbar text button.
pub struct CupertinoDesktopTextSelectionToolbar {
    pub key: Option<KeyRef>,
    /// The point at which to render the menu, if possible.
    pub anchor: Offset,
    /// The children of the toolbar, typically buttons.
    pub children: Vec<WidgetRef>,
}

impl Debug for CupertinoDesktopTextSelectionToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoDesktopTextSelectionToolbar")
            .finish_non_exhaustive()
    }
}

impl CupertinoDesktopTextSelectionToolbar {
    /// Creates an instance of [`CupertinoDesktopTextSelectionToolbar`].
    pub fn new(
        anchor: Offset,
        children: impl IntoIterator<Item = WidgetRef>,
    ) -> CupertinoDesktopTextSelectionToolbar {
        let children: Vec<WidgetRef> = children.into_iter().collect();
        debug_assert!(!children.is_empty());
        CupertinoDesktopTextSelectionToolbar {
            key: None,
            anchor,
            children,
        }
    }

    /// Dart `CupertinoDesktopTextSelectionToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoDesktopTextSelectionToolbar {
        self.key = Some(key);
        self
    }

    /// Creates a 5x5 matrix that increases saturation when used with [`ColorFilter::matrix`].
    fn matrix_with_saturation(saturation: f64) -> [f64; 20] {
        let r = 0.213 * (1.0 - saturation);
        let g = 0.715 * (1.0 - saturation);
        let b = 0.072 * (1.0 - saturation);
        [
            r + saturation,
            g,
            b,
            0.0,
            0.0,
            r,
            g + saturation,
            b,
            0.0,
            0.0,
            r,
            g,
            b + saturation,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ]
    }

    fn default_toolbar_builder(app: &mut App, context: BuildContext, child: WidgetRef) -> WidgetRef {
        let background = K_TOOLBAR_BACKGROUND_COLOR.resolve_from(app, context);
        let border = K_TOOLBAR_BORDER_COLOR.resolve_from(app, context);
        Container::new()
            .width(K_TOOLBAR_WIDTH)
            .clip_behavior(Clip::HardEdge)
            .decoration(
                ShapeDecoration::new(RoundedSuperellipseBorder::new(
                    BorderSide::NONE,
                    Some(BorderRadius::all(k_toolbar_border_radius()).into()),
                ))
                .shadows(k_toolbar_shadow()),
            )
            .child(
                BackdropFilter::filter(ImageFilter::compose(
                    ImageFilter::Color(ColorFilter::matrix(Self::matrix_with_saturation(
                        K_TOOLBAR_SATURATION_BOOST,
                    ))),
                    ImageFilter::blur(K_TOOLBAR_BLUR_SIGMA, K_TOOLBAR_BLUR_SIGMA),
                ))
                .child(
                    DecoratedBox::new(
                        ShapeDecoration::new(RoundedSuperellipseBorder::new(
                            BorderSide::new(
                                border.effective_color(),
                                1.0,
                                BorderStyle::Solid,
                                BorderSide::STROKE_ALIGN_INSIDE,
                            ),
                            Some(BorderRadius::all(k_toolbar_border_radius()).into()),
                        ))
                        .color(background),
                    )
                    .child(Padding::new(EdgeInsetsGeometry::Insets(K_TOOLBAR_PADDING)).child(child)),
                ),
            )
            .into_widget()
    }
}

impl StatelessWidget for CupertinoDesktopTextSelectionToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let padding_above =
            MediaQuery::padding_of(app, context).top + K_TOOLBAR_SCREEN_PADDING;
        let local_adjustment = Offset::new(K_TOOLBAR_SCREEN_PADDING, padding_above);
        Padding::new(EdgeInsetsGeometry::from_ltrb(
            K_TOOLBAR_SCREEN_PADDING,
            padding_above,
            K_TOOLBAR_SCREEN_PADDING,
            K_TOOLBAR_SCREEN_PADDING,
        ))
        .child(
            CustomSingleChildLayout::new(Rc::new(
                DesktopTextSelectionToolbarLayoutDelegate::new(self.anchor - local_adjustment),
            ))
            .child(Self::default_toolbar_builder(
                app,
                context,
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .children(self.children.clone())
                    .into_widget(),
            )),
        )
        .into_widget()
    }
}
