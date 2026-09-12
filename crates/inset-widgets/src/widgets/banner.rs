//! Flutter `widgets/banner.dart`.

use std::any::Any;
use std::cell::RefCell;
use std::f64::consts::{FRAC_1_SQRT_2, PI};
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{
    BlurStyle, Canvas, Color, FontWeight, Offset, Paint, Rect, Size, TextAlign, TextDirection,
};
use inset_foundation::{App, Handle};
use inset_painting::{BoxShadow, PaintingBinding, TextPainter, TextSpan, TextStyle};
use inset_rendering::CustomPainter;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, StatelessWidget, WidgetRef,
};
use crate::widgets::basic::{CustomPaint, Directionality};

/// Distance to bottom of banner, at a 45 degree angle inwards.
const K_OFFSET: f64 = 40.0;
/// Height of banner.
const K_HEIGHT: f64 = 12.0;
const K_BOTTOM_OFFSET: f64 = K_OFFSET + FRAC_1_SQRT_2 * K_HEIGHT;
const K_RECT: Rect = Rect::from_ltwh(-K_OFFSET, K_OFFSET - K_HEIGHT, K_OFFSET * 2.0, K_HEIGHT);
const K_COLOR: Color = Color::new(0xA0B7_1C1C);

fn k_shadow() -> BoxShadow {
    BoxShadow::new(
        Color::new(0x7F00_0000),
        Offset::ZERO,
        6.0,
        0.0,
        BlurStyle::Normal,
    )
}

fn k_text_style() -> TextStyle {
    TextStyle::new()
        .color(Color::new(0xFFFF_FFFF))
        .font_size(K_HEIGHT * 0.85)
        .font_weight(FontWeight::W900)
        .height(1.0)
}

/// Where to show a [`Banner`].
///
/// The start and end locations are relative to the ambient [`Directionality`]
/// (which can be overridden by [`Banner::layout_direction`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BannerLocation {
    /// Show the banner in the top-right corner when the ambient [`Directionality`]
    /// (or [`Banner::layout_direction`]) is [`TextDirection::Rtl`] and in the
    /// top-left corner when the ambient [`Directionality`] is
    /// [`TextDirection::Ltr`].
    TopStart,
    /// Show the banner in the top-left corner when the ambient [`Directionality`]
    /// (or [`Banner::layout_direction`]) is [`TextDirection::Rtl`] and in the
    /// top-right corner when the ambient [`Directionality`] is
    /// [`TextDirection::Ltr`].
    TopEnd,
    /// Show the banner in the bottom-right corner when the ambient
    /// [`Directionality`] (or [`Banner::layout_direction`]) is
    /// [`TextDirection::Rtl`] and in the bottom-left corner when the ambient
    /// [`Directionality`] is [`TextDirection::Ltr`].
    BottomStart,
    /// Show the banner in the bottom-left corner when the ambient
    /// [`Directionality`] (or [`Banner::layout_direction`]) is
    /// [`TextDirection::Rtl`] and in the bottom-right corner when the ambient
    /// [`Directionality`] is [`TextDirection::Ltr`].
    BottomEnd,
}

/// Paints a [`Banner`].
pub struct BannerPainter {
    /// The message to show in the banner.
    pub message: String,
    /// The directionality of the text.
    ///
    /// This value is used to disambiguate how to render bidirectional text. For
    /// example, if the message is an English phrase followed by a Hebrew phrase,
    /// in a [`TextDirection::Ltr`] context the English phrase will be on the left
    /// and the Hebrew phrase to its right, while in a [`TextDirection::Rtl`]
    /// context, the English phrase will be on the right and the Hebrew phrase on
    /// its left.
    ///
    /// See also [`layout_direction`](Self::layout_direction), which controls the
    /// interpretation of values in [`location`](Self::location).
    pub text_direction: TextDirection,
    /// Where to show the banner (e.g., the upper right corner).
    pub location: BannerLocation,
    /// The directionality of the layout.
    ///
    /// This value is used to interpret the [`location`](Self::location) of the banner.
    ///
    /// See also [`text_direction`](Self::text_direction), which controls the
    /// reading direction of the [`message`](Self::message).
    pub layout_direction: TextDirection,
    /// The color to paint behind the [`message`](Self::message).
    ///
    /// Defaults to a dark red.
    pub color: Color,
    /// The text style to use for the [`message`](Self::message).
    ///
    /// Defaults to bold, white text.
    pub text_style: TextStyle,
    /// The shadow of the banner.
    pub shadow: BoxShadow,
    /// Dart's `_prepared`, `_textPainter`, `_paintShadow` and `_paintBanner`, filled on the
    /// first paint.
    prepared: RefCell<Option<Prepared>>,
}

struct Prepared {
    text_painter: TextPainter,
    paint_shadow: Paint,
    paint_banner: Paint,
}

impl BannerPainter {
    /// Creates a banner painter.
    pub fn new(
        message: impl Into<String>,
        text_direction: TextDirection,
        location: BannerLocation,
        layout_direction: TextDirection,
    ) -> BannerPainter {
        BannerPainter {
            message: message.into(),
            text_direction,
            location,
            layout_direction,
            color: K_COLOR,
            text_style: k_text_style(),
            shadow: k_shadow(),
            prepared: RefCell::new(None),
        }
    }

    /// Dart `BannerPainter(color:)`.
    pub fn color(mut self, color: Color) -> BannerPainter {
        self.color = color;
        self
    }

    /// Dart `BannerPainter(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> BannerPainter {
        self.text_style = text_style;
        self
    }

    /// Dart `BannerPainter(shadow:)`.
    pub fn shadow(mut self, shadow: BoxShadow) -> BannerPainter {
        self.shadow = shadow;
        self
    }

    /// Release resources held by this painter.
    ///
    /// After calling this method, this object is no longer usable.
    pub fn dispose(&self) {
        self.prepared.borrow_mut().take();
    }

    fn prepare(&self) -> Prepared {
        let mut text_painter = TextPainter::new();
        text_painter.set_text(Some(Rc::new(
            TextSpan::new()
                .style(self.text_style.clone())
                .text(self.message.clone()),
        )));
        text_painter.set_text_align(TextAlign::Center);
        text_painter.set_text_direction(Some(self.text_direction));
        Prepared {
            text_painter,
            paint_shadow: self.shadow.to_paint(),
            paint_banner: Paint {
                color: self.color.into(),
                ..Paint::default()
            },
        }
    }

    fn translation_x(&self, width: f64) -> f64 {
        match (self.layout_direction, self.location) {
            (TextDirection::Rtl, BannerLocation::TopStart) => width,
            (TextDirection::Ltr, BannerLocation::TopStart) => 0.0,
            (TextDirection::Rtl, BannerLocation::TopEnd) => 0.0,
            (TextDirection::Ltr, BannerLocation::TopEnd) => width,
            (TextDirection::Rtl, BannerLocation::BottomStart) => width - K_BOTTOM_OFFSET,
            (TextDirection::Ltr, BannerLocation::BottomStart) => K_BOTTOM_OFFSET,
            (TextDirection::Rtl, BannerLocation::BottomEnd) => K_BOTTOM_OFFSET,
            (TextDirection::Ltr, BannerLocation::BottomEnd) => width - K_BOTTOM_OFFSET,
        }
    }

    fn translation_y(&self, height: f64) -> f64 {
        match self.location {
            BannerLocation::BottomStart | BannerLocation::BottomEnd => height - K_BOTTOM_OFFSET,
            BannerLocation::TopStart | BannerLocation::TopEnd => 0.0,
        }
    }

    fn rotation(&self) -> f64 {
        let turns = match (self.layout_direction, self.location) {
            (TextDirection::Rtl, BannerLocation::TopStart | BannerLocation::BottomEnd) => 1.0,
            (TextDirection::Ltr, BannerLocation::TopStart | BannerLocation::BottomEnd) => -1.0,
            (TextDirection::Rtl, BannerLocation::BottomStart | BannerLocation::TopEnd) => -1.0,
            (TextDirection::Ltr, BannerLocation::BottomStart | BannerLocation::TopEnd) => 1.0,
        };
        PI / 4.0 * turns
    }
}

impl CustomPainter for BannerPainter {
    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        let mut prepared = self.prepared.borrow_mut();
        let prepared = prepared.get_or_insert_with(|| self.prepare());
        canvas.translate(
            self.translation_x(size.width()) as f32,
            self.translation_y(size.height()) as f32,
        );
        canvas.rotate(self.rotation() as f32);
        canvas.draw_rect(K_RECT, &prepared.paint_shadow);
        canvas.draw_rect(K_RECT, &prepared.paint_banner);
        let width = K_OFFSET * 2.0;
        let fonts = PaintingBinding::instance(app).fonts(app);
        let fonts = app.get_mut(fonts);
        prepared.text_painter.layout(fonts, width, width);
        let text_height = prepared.text_painter.height();
        prepared.text_painter.paint(
            fonts,
            canvas,
            K_RECT.top_left() + Offset::new(0.0, (K_RECT.height() - text_height) / 2.0),
        );
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        let Some(old_delegate) = old_delegate.as_any().downcast_ref::<BannerPainter>() else {
            return true;
        };
        self.message != old_delegate.message
            || self.location != old_delegate.location
            || self.color != old_delegate.color
            || self.text_style != old_delegate.text_style
    }

    fn hit_test(&self, _app: &App, _position: Offset) -> Option<bool> {
        Some(false)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for BannerPainter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BannerPainter")
            .field("message", &self.message)
            .field("location", &self.location)
            .finish_non_exhaustive()
    }
}

/// Displays a diagonal message above the corner of another widget.
///
/// Useful for showing the execution mode of an app (e.g., that asserts are
/// enabled.)
///
/// See also:
///
///  * [`CheckedModeBanner`], which the `WidgetsApp` widget includes by default in
///    debug mode, to show a banner that says "DEBUG".
pub struct Banner {
    pub key: Option<KeyRef>,
    /// The widget to show behind the banner.
    pub child: Option<WidgetRef>,
    /// The message to show in the banner.
    pub message: String,
    /// The directionality of the text.
    ///
    /// This is used to disambiguate how to render bidirectional text. For
    /// example, if the message is an English phrase followed by a Hebrew phrase,
    /// in a [`TextDirection::Ltr`] context the English phrase will be on the left
    /// and the Hebrew phrase to its right, while in a [`TextDirection::Rtl`]
    /// context, the English phrase will be on the right and the Hebrew phrase on
    /// its left.
    ///
    /// Defaults to the ambient [`Directionality`], if any.
    ///
    /// See also [`layout_direction`](Self::layout_direction), which controls the
    /// interpretation of the [`location`](Self::location).
    pub text_direction: Option<TextDirection>,
    /// Where to show the banner (e.g., the upper right corner).
    pub location: BannerLocation,
    /// The directionality of the layout.
    ///
    /// This is used to resolve the [`location`](Self::location) values.
    ///
    /// Defaults to the ambient [`Directionality`], if any.
    ///
    /// See also [`text_direction`](Self::text_direction), which controls the
    /// reading direction of the [`message`](Self::message).
    pub layout_direction: Option<TextDirection>,
    /// The color of the banner.
    pub color: Color,
    /// The style of the text shown on the banner.
    pub text_style: TextStyle,
    /// The shadow of the banner.
    pub shadow: BoxShadow,
}

impl Banner {
    /// Creates a banner.
    pub fn new(message: impl Into<String>, location: BannerLocation) -> Banner {
        Banner {
            key: None,
            child: None,
            message: message.into(),
            text_direction: None,
            location,
            layout_direction: None,
            color: K_COLOR,
            text_style: k_text_style(),
            shadow: k_shadow(),
        }
    }

    /// Dart `Banner(key:)`.
    pub fn key(mut self, key: KeyRef) -> Banner {
        self.key = Some(key);
        self
    }

    /// Dart `Banner(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Banner {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `Banner(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Banner {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Banner(layoutDirection:)`.
    pub fn layout_direction(mut self, layout_direction: TextDirection) -> Banner {
        self.layout_direction = Some(layout_direction);
        self
    }

    /// Dart `Banner(color:)`.
    pub fn color(mut self, color: Color) -> Banner {
        self.color = color;
        self
    }

    /// Dart `Banner(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> Banner {
        self.text_style = text_style;
        self
    }

    /// Dart `Banner(shadow:)`.
    pub fn shadow(mut self, shadow: BoxShadow) -> Banner {
        self.shadow = shadow;
        self
    }
}

impl Debug for Banner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Banner")
            .field("message", &self.message)
            .field("location", &self.location)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Banner {
    type State = BannerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> BannerState {
        BannerState {
            state: StateData::new(),
        }
    }
}

/// Dart's `_BannerState`. The painter it builds is owned by the `CustomPaint`, and dropping
/// it releases the text painter that Dart's `_painter?.dispose()` does.
pub struct BannerState {
    state: StateData<Banner>,
}

impl State for BannerState {
    type Widget = Banner;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (text_direction, layout_direction) = {
            let widget = self.widget(app);
            (widget.text_direction, widget.layout_direction)
        };
        let text_direction = text_direction.unwrap_or_else(|| Directionality::of(app, context));
        let layout_direction = layout_direction.unwrap_or_else(|| Directionality::of(app, context));
        let widget = self.widget(app);
        let painter = BannerPainter::new(
            widget.message.clone(),
            text_direction,
            widget.location,
            layout_direction,
        )
        .color(widget.color)
        .text_style(widget.text_style.clone())
        .shadow(widget.shadow);
        let child = widget.child.clone();
        let mut custom_paint = CustomPaint::new().foreground_painter(painter);
        if let Some(child) = child {
            custom_paint = custom_paint.child(child);
        }
        custom_paint.into_widget()
    }
}

/// Displays a [`Banner`] saying "DEBUG" when running in debug mode.
/// `WidgetsApp` builds one of these by default.
///
/// Does nothing in release mode.
#[derive(Debug)]
pub struct CheckedModeBanner {
    pub key: Option<KeyRef>,
    /// The widget to show behind the banner.
    pub child: WidgetRef,
}

impl CheckedModeBanner {
    /// Creates a const debug mode banner.
    pub fn new<K>(child: impl IntoWidget<K>) -> CheckedModeBanner {
        CheckedModeBanner {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `CheckedModeBanner(key:)`.
    pub fn key(mut self, key: KeyRef) -> CheckedModeBanner {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for CheckedModeBanner {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        if cfg!(debug_assertions) {
            return Banner::new("DEBUG", BannerLocation::TopEnd)
                .text_direction(TextDirection::Ltr)
                .child(self.child.clone())
                .into_widget();
        }
        self.child.clone()
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::valo::Op;
    use inset_foundation::AppCell;
    use inset_rendering::RenderCustomPaint;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    #[test]
    fn a_banner_paints_a_rotated_shadowed_rect_with_its_message_in_the_corner() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let harness = Harness::mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                CheckedModeBanner::new(SizedBox::expand()),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let custom_paint = harness
            .render_root(&app)
            .child(&app)
            .expect("the banner's custom paint")
            .as_object()
            .downcast::<RenderCustomPaint>(&app)
            .expect("a RenderCustomPaint");
        let painter = custom_paint
            .foreground_painter(&app)
            .expect("the banner painter");
        let banner = painter
            .as_any()
            .downcast_ref::<BannerPainter>()
            .expect("a BannerPainter");
        assert_eq!(banner.message, "DEBUG");
        assert_eq!(banner.location, BannerLocation::TopEnd);
        assert_eq!(
            banner.translation_x(300.0),
            300.0,
            "top end sits at the right edge"
        );
        assert_eq!(banner.rotation(), PI / 4.0);

        let mut canvas = Canvas::new();
        painter.paint(&mut app, &mut canvas, Size::new(300.0, 200.0));
        let ops = canvas.build().ops().to_vec();
        let rects = ops
            .iter()
            .filter(|op| matches!(op, Op::DrawRect { .. } | Op::RRectBlur { .. }))
            .count();
        assert!(rects >= 2, "the shadow and the banner: {ops:?}");
        assert!(
            ops.iter().any(|op| matches!(op, Op::Transform(_))),
            "the corner rotation: {ops:?}"
        );
    }
}
