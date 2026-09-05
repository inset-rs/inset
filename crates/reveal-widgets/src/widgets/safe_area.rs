//! Flutter counterpart: `widgets/safe_area.dart`.

use reveal_foundation::App;
use reveal_painting::{EdgeInsets, EdgeInsetsGeometry};

use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::{Padding, SliverPadding};
use crate::widgets::media_query::MediaQuery;

/// A widget that insets its child by sufficient padding to avoid intrusions by the operating
/// system.
///
/// For example, this will indent the child by enough to avoid the status bar at the top of
/// the screen.
///
/// It will also indent the child by the amount necessary to avoid The Notch on the iPhone X,
/// or other similar creative physical features of the display.
///
/// When a [`minimum`](Self::minimum) padding is specified, the greater of the minimum padding
/// or the safe area padding will be applied.
///
/// See also:
///
///  * `SliverSafeArea`, for insetting slivers to avoid operating system intrusions.
///  * `Padding`, for insetting widgets in general.
///  * `MediaQuery`, from which the window padding is obtained.
///  * `dart:ui.FlutterView.padding`, which reports the padding from the operating system.
#[derive(Debug)]
pub struct SafeArea {
    pub key: Option<KeyRef>,
    /// Whether to avoid system intrusions on the left.
    pub left: bool,
    /// Whether to avoid system intrusions at the top of the screen, typically the system
    /// status bar.
    pub top: bool,
    /// Whether to avoid system intrusions on the right.
    pub right: bool,
    /// Whether to avoid system intrusions on the bottom side of the screen.
    pub bottom: bool,
    /// This minimum padding to apply.
    ///
    /// The greater of the minimum insets and the media padding will be applied.
    pub minimum: EdgeInsets,
    /// Specifies whether the `SafeArea` should maintain the bottom `MediaQueryData.viewPadding`
    /// instead of the bottom `MediaQueryData.padding`, defaults to false.
    ///
    /// For example, if there is an onscreen keyboard displayed above the SafeArea, the
    /// padding can be maintained below the obstruction rather than being consumed. This can
    /// be helpful in cases where your layout contains flexible widgets, which could visibly
    /// move when opening a software keyboard due to the change in the padding value. Setting
    /// this to true will avoid the UI shift.
    pub maintain_bottom_view_padding: bool,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl SafeArea {
    /// Creates a widget that avoids operating system interfaces.
    pub fn new<K>(child: impl IntoWidget<K>) -> SafeArea {
        SafeArea {
            key: None,
            left: true,
            top: true,
            right: true,
            bottom: true,
            minimum: EdgeInsets::ZERO,
            maintain_bottom_view_padding: false,
            child: child.into_widget(),
        }
    }

    /// Dart `SafeArea(key:)`.
    pub fn key(mut self, key: KeyRef) -> SafeArea {
        self.key = Some(key);
        self
    }

    /// Dart `SafeArea(left:)`.
    pub fn left(mut self, left: bool) -> SafeArea {
        self.left = left;
        self
    }

    /// Dart `SafeArea(top:)`.
    pub fn top(mut self, top: bool) -> SafeArea {
        self.top = top;
        self
    }

    /// Dart `SafeArea(right:)`.
    pub fn right(mut self, right: bool) -> SafeArea {
        self.right = right;
        self
    }

    /// Dart `SafeArea(bottom:)`.
    pub fn bottom(mut self, bottom: bool) -> SafeArea {
        self.bottom = bottom;
        self
    }

    /// Dart `SafeArea(minimum:)`.
    pub fn minimum(mut self, minimum: EdgeInsets) -> SafeArea {
        self.minimum = minimum;
        self
    }

    /// Dart `SafeArea(maintainBottomViewPadding:)`.
    pub fn maintain_bottom_view_padding(mut self, maintain_bottom_view_padding: bool) -> SafeArea {
        self.maintain_bottom_view_padding = maintain_bottom_view_padding;
        self
    }
}

impl StatelessWidget for SafeArea {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut padding = MediaQuery::padding_of(app, context);
        // Bottom padding has been consumed - i.e. by the keyboard
        if self.maintain_bottom_view_padding {
            let view_padding = MediaQuery::view_padding_of(app, context);
            padding = padding.copy_with(None, None, None, Some(view_padding.bottom));
        }
        let inset = |avoid: bool, padding: f64, minimum: f64| {
            f64::max(if avoid { padding } else { 0.0 }, minimum)
        };
        let child = MediaQuery::remove_padding(
            app,
            context,
            self.left,
            self.top,
            self.right,
            self.bottom,
            self.child.clone(),
        )
        .into_widget();
        Padding::new(EdgeInsetsGeometry::only(
            inset(self.left, padding.left, self.minimum.left),
            inset(self.top, padding.top, self.minimum.top),
            inset(self.right, padding.right, self.minimum.right),
            inset(self.bottom, padding.bottom, self.minimum.bottom),
        ))
        .child(child)
        .into_widget()
    }
}

/// A sliver that insets another sliver by sufficient padding to avoid intrusions by the
/// operating system.
///
/// For example, this will indent the sliver by enough to avoid the status bar at the top of the
/// screen.
///
/// It will also indent the sliver by the amount necessary to avoid The Notch on the iPhone X, or
/// other similar creative physical features of the display.
///
/// When a [`minimum`](Self::minimum) padding is specified, the greater of the minimum padding or
/// the safe area padding will be applied.
///
/// See also:
///
///  * [`SafeArea`], for insetting box widgets to avoid operating system intrusions.
///  * [`SliverPadding`], for insetting slivers in general.
///  * `MediaQuery`, from which the window padding is obtained.
#[derive(Debug)]
pub struct SliverSafeArea {
    pub key: Option<KeyRef>,
    /// Whether to avoid system intrusions on the left.
    pub left: bool,
    /// Whether to avoid system intrusions at the top of the screen, typically the system status
    /// bar.
    pub top: bool,
    /// Whether to avoid system intrusions on the right.
    pub right: bool,
    /// Whether to avoid system intrusions on the bottom side of the screen.
    pub bottom: bool,
    /// This minimum padding to apply.
    ///
    /// The greater of the minimum padding and the media padding is be applied.
    pub minimum: EdgeInsets,
    /// The sliver below this sliver in the tree.
    ///
    /// The padding on the `MediaQuery` for the sliver will be suitably adjusted to zero out any
    /// sides that were avoided by this sliver.
    pub sliver: WidgetRef,
}

impl SliverSafeArea {
    /// Creates a sliver that avoids operating system interfaces.
    pub fn new<K>(sliver: impl IntoWidget<K>) -> SliverSafeArea {
        SliverSafeArea {
            key: None,
            left: true,
            top: true,
            right: true,
            bottom: true,
            minimum: EdgeInsets::ZERO,
            sliver: sliver.into_widget(),
        }
    }

    /// Dart `SliverSafeArea(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverSafeArea {
        self.key = Some(key);
        self
    }

    /// Dart `SliverSafeArea(left:)`.
    pub fn left(mut self, left: bool) -> SliverSafeArea {
        self.left = left;
        self
    }

    /// Dart `SliverSafeArea(top:)`.
    pub fn top(mut self, top: bool) -> SliverSafeArea {
        self.top = top;
        self
    }

    /// Dart `SliverSafeArea(right:)`.
    pub fn right(mut self, right: bool) -> SliverSafeArea {
        self.right = right;
        self
    }

    /// Dart `SliverSafeArea(bottom:)`.
    pub fn bottom(mut self, bottom: bool) -> SliverSafeArea {
        self.bottom = bottom;
        self
    }

    /// Dart `SliverSafeArea(minimum:)`.
    pub fn minimum(mut self, minimum: EdgeInsets) -> SliverSafeArea {
        self.minimum = minimum;
        self
    }
}

impl StatelessWidget for SliverSafeArea {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let padding = MediaQuery::padding_of(app, context);
        let inset = |avoid: bool, padding: f64, minimum: f64| {
            f64::max(if avoid { padding } else { 0.0 }, minimum)
        };
        let sliver = MediaQuery::remove_padding(
            app,
            context,
            self.left,
            self.top,
            self.right,
            self.bottom,
            self.sliver.clone(),
        )
        .into_widget();
        SliverPadding::new(EdgeInsetsGeometry::only(
            inset(self.left, padding.left, self.minimum.left),
            inset(self.top, padding.top, self.minimum.top),
            inset(self.right, padding.right, self.minimum.right),
            inset(self.bottom, padding.bottom, self.minimum.bottom),
        ))
        .sliver(sliver)
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_rendering::RenderPadding;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::media_query::MediaQueryData;

    fn padded(padding: EdgeInsets, view_padding: EdgeInsets, area: SafeArea) -> WidgetRef {
        MediaQuery::new(
            MediaQueryData::new()
                .padding(padding)
                .view_padding(view_padding),
            area,
        )
        .into_widget()
    }

    fn render_padding(harness: &Harness, app: &App) -> EdgeInsetsGeometry {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderPadding>(app)
            .expect("a RenderPadding")
            .padding(app)
    }

    #[test]
    fn a_safe_area_pads_by_the_media_padding_it_avoids_and_removes_it_below() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() = Some(MediaQuery::padding_of(app, context));
                SizedBox::shrink().into_widget()
            }
        });
        let harness = Harness::mount(
            &mut app,
            padded(
                EdgeInsets::only(10.0, 20.0, 30.0, 40.0),
                EdgeInsets::ZERO,
                SafeArea::new(probe)
                    .left(false)
                    .minimum(EdgeInsets::only(0.0, 0.0, 5.0, 45.0)),
            ),
        );
        harness.pump(&mut app);
        assert_eq!(
            render_padding(&harness, &app),
            EdgeInsetsGeometry::only(0.0, 20.0, 30.0, 45.0)
        );
        assert_eq!(
            seen.borrow().clone(),
            Some(EdgeInsets::only(10.0, 0.0, 0.0, 0.0))
        );
    }

    #[test]
    fn maintaining_the_bottom_view_padding_uses_it_instead_of_the_consumed_padding() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            padded(
                EdgeInsets::only(0.0, 0.0, 0.0, 0.0),
                EdgeInsets::only(0.0, 0.0, 0.0, 34.0),
                SafeArea::new(SizedBox::shrink()).maintain_bottom_view_padding(true),
            ),
        );
        harness.pump(&mut app);
        assert_eq!(
            render_padding(&harness, &app),
            EdgeInsetsGeometry::only(0.0, 0.0, 0.0, 34.0)
        );
    }
    /// A `SliverSafeArea` pads its sliver by the media padding it avoids, and removes that
    /// padding for its subtree.
    #[test]
    fn a_sliver_safe_area_pads_by_the_media_padding_it_avoids() {
        use reveal_embedder::TextDirection;
        use reveal_rendering::{
            ContainerRenderObjectMixin, FixedViewportOffset, RenderSliverPadding, RenderViewport,
            ViewportOffset,
        };

        use crate::widgets::basic::{Directionality, SliverToBoxAdapter};
        use crate::widgets::viewport::Viewport;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let area =
            SliverSafeArea::new(SliverToBoxAdapter::new().child(SizedBox::new().height(50.0)))
                .left(false)
                .minimum(EdgeInsets::only(0.0, 0.0, 5.0, 45.0));
        let tree = Directionality::new(
            TextDirection::Ltr,
            MediaQuery::new(
                MediaQueryData::new().padding(EdgeInsets::only(10.0, 20.0, 30.0, 40.0)),
                Viewport::new(offset.as_viewport_offset()).slivers(vec![area.into_widget()]),
            )
            .into_widget(),
        )
        .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);

        let padding = harness
            .render_root(&app)
            .child(&app)
            .expect("a child")
            .as_object()
            .downcast::<RenderViewport>(&app)
            .expect("a RenderViewport")
            .first_child(&app)
            .expect("the padding")
            .as_object()
            .downcast::<RenderSliverPadding>(&app)
            .expect("a RenderSliverPadding");
        assert_eq!(
            padding.padding(&app),
            EdgeInsetsGeometry::only(0.0, 20.0, 30.0, 45.0)
        );
    }
}
