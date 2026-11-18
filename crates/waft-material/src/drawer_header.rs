//! Flutter counterpart: material_ui's `drawer_header.dart`.

use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Curve, Curves};
use waft_foundation::App;
use waft_painting::{Border, BoxDecoration, DecorationRef, EdgeInsets, EdgeInsetsGeometry};
use waft_widgets::{
    AnimatedContainer, BuildContext, Container, DefaultTextStyle, IntoWidget, KeyRef, MediaQuery,
    Semantics, StatelessWidget, WidgetRef, debug_check_has_media_query,
};

use crate::debug::debug_check_has_material;
use crate::divider::Divider;
use crate::theme::Theme;

const K_DRAWER_HEADER_HEIGHT: f64 = 160.0 + 1.0; // bottom edge

/// The top-most region of a Material Design drawer. The header's [`child`](Self::child)
/// widget, if any, is placed inside a `Container` whose [`decoration`](Self::decoration) can be
/// passed as an argument, inset by the given [`padding`](Self::padding).
///
/// Part of the Material Design `Drawer`.
///
/// Requires one of its ancestors to be a `Material` widget. This condition is
/// satisfied by putting the [`DrawerHeader`] in a `Drawer`.
///
/// See also:
///
///  * `UserAccountsDrawerHeader`, a variant of [`DrawerHeader`] that is
///    specialized for showing user accounts.
///  * <https://material.io/design/components/navigation-drawer.html>
#[derive(Debug)]
pub struct DrawerHeader {
    pub key: Option<KeyRef>,

    /// Decoration for the main drawer header `Container`; useful for applying
    /// backgrounds.
    ///
    /// This decoration will extend under the system status bar.
    ///
    /// If this is changed, it will be animated according to [`duration`](Self::duration) and
    /// [`curve`](Self::curve).
    pub decoration: Option<DecorationRef>,

    /// The padding by which to inset [`child`](Self::child).
    ///
    /// The [`DrawerHeader`] additionally offsets the child by the height of the
    /// system status bar.
    ///
    /// If the child is null, the padding has no effect.
    pub padding: EdgeInsetsGeometry,

    /// The margin around the drawer header.
    pub margin: Option<EdgeInsetsGeometry>,

    /// The duration for animations of the [`decoration`](Self::decoration).
    pub duration: Duration,

    /// The curve for animations of the [`decoration`](Self::decoration).
    pub curve: Rc<dyn Curve>,

    /// A widget to be placed inside the drawer header, inset by the [`padding`](Self::padding).
    ///
    /// This widget will be sized to the size of the header. To position the child
    /// precisely, consider using an `Align` or `Center` widget.
    pub child: Option<WidgetRef>,
}

impl DrawerHeader {
    /// Creates a Material Design drawer header; Dart's optional named arguments are the
    /// setters.
    ///
    /// Requires one of its ancestors to be a `Material` widget.
    pub fn new(child: Option<WidgetRef>) -> DrawerHeader {
        DrawerHeader {
            key: None,
            decoration: None,
            padding: EdgeInsetsGeometry::Insets(EdgeInsets::from_ltrb(16.0, 16.0, 16.0, 8.0)),
            margin: Some(EdgeInsetsGeometry::Insets(EdgeInsets::only(
                0.0, 0.0, 0.0, 8.0,
            ))),
            duration: Duration::from_millis(250),
            curve: Curves::fast_out_slow_in(),
            child,
        }
    }

    /// Dart `DrawerHeader(key:)`.
    pub fn key(mut self, key: KeyRef) -> DrawerHeader {
        self.key = Some(key);
        self
    }

    /// Dart `DrawerHeader(decoration:)`.
    pub fn decoration(mut self, decoration: impl Into<DecorationRef>) -> DrawerHeader {
        self.decoration = Some(decoration.into());
        self
    }

    /// Dart `DrawerHeader(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> DrawerHeader {
        let padding = padding.into();
        self.padding = padding;
        self
    }

    /// Dart `DrawerHeader(margin:)`.
    pub fn margin(mut self, margin: Option<EdgeInsetsGeometry>) -> DrawerHeader {
        self.margin = margin;
        self
    }

    /// Dart `DrawerHeader(duration:)`.
    pub fn duration(mut self, duration: Duration) -> DrawerHeader {
        self.duration = duration;
        self
    }

    /// Dart `DrawerHeader(curve:)`.
    pub fn curve(mut self, curve: Rc<dyn Curve>) -> DrawerHeader {
        self.curve = curve;
        self
    }
}

impl StatelessWidget for DrawerHeader {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_media_query(app, context));
        let theme = Theme::of(app, context);
        let status_bar_height = MediaQuery::padding_of(app, context).top;
        let mut container = Container::new()
            .height(status_bar_height + K_DRAWER_HEADER_HEIGHT)
            .decoration(BoxDecoration::new().border(Border {
                bottom: Divider::create_border_side(app, Some(context), None, None),
                ..Border::default()
            }));
        if let Some(margin) = self.margin {
            container = container.margin(margin);
        }

        let mut animated_container = AnimatedContainer::new(self.duration)
            .padding(
                self.padding
                    .add(EdgeInsetsGeometry::Insets(EdgeInsets::only(
                        0.0,
                        status_bar_height,
                        0.0,
                        0.0,
                    ))),
            )
            .curve(Rc::clone(&self.curve));
        animated_container.decoration = self.decoration.clone();
        if let Some(child) = &self.child {
            animated_container = animated_container.child(DefaultTextStyle::new(
                theme
                    .text_theme
                    .body_large
                    .clone()
                    .expect("the text theme has a body large style"),
                MediaQuery::remove_padding(app, context, false, true, false, false, child.clone())
                    .into_widget(),
            ));
        }
        Semantics::new()
            .header(true)
            .child(container.child(animated_container))
            .into_widget()
    }
}
