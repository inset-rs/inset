//! Flutter counterpart: material_ui's `desktop_text_selection_toolbar.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Clip, Offset, Radius};
use waft_foundation::App;
use waft_painting::{BorderRadius, EdgeInsetsGeometry};
use waft_widgets::{
    BuildContext, Column, CustomSingleChildLayout, DesktopTextSelectionToolbarLayoutDelegate,
    IntoWidget, KeyRef, MainAxisSize, MediaQuery, Padding, SizedBox, StatelessWidget, WidgetRef,
    debug_check_has_media_query,
};

use crate::material::{Material, MaterialType};

// These values were measured from a screenshot of TextEdit on macOS 10.15.7 on
// a Macbook Pro.
const K_TOOLBAR_SCREEN_PADDING: f64 = 8.0;
const K_TOOLBAR_WIDTH: f64 = 222.0;

/// A Material-style desktop text selection toolbar.
///
/// Typically displays buttons for text manipulation, e.g. copying and pasting
/// text.
///
/// Tries to position its top left corner as closely as possible to [`anchor`](Self::anchor)
/// while remaining fully inside the viewport.
///
/// See also:
///
///  * `AdaptiveTextSelectionToolbar`, which builds the toolbar for the current
///    platform.
///  * `TextSelectionToolbar`, which is similar, but builds an Android-style
///    toolbar.
pub struct DesktopTextSelectionToolbar {
    pub key: Option<KeyRef>,
    /// The point where the toolbar will attempt to position itself as closely as
    /// possible.
    pub anchor: Offset,
    /// The children that will be displayed in the text selection toolbar.
    ///
    /// Typically these are buttons.
    ///
    /// Must not be empty.
    ///
    /// See also:
    ///   * `DesktopTextSelectionToolbarButton`, which builds a default
    ///     Material-style desktop text selection toolbar text button.
    pub children: Vec<WidgetRef>,
}

impl Debug for DesktopTextSelectionToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopTextSelectionToolbar")
            .field("anchor", &self.anchor)
            .finish_non_exhaustive()
    }
}

impl DesktopTextSelectionToolbar {
    /// Creates a const instance of DesktopTextSelectionToolbar.
    pub fn new(
        anchor: Offset,
        children: impl IntoIterator<Item = WidgetRef>,
    ) -> DesktopTextSelectionToolbar {
        let children: Vec<WidgetRef> = children.into_iter().collect();
        debug_assert!(!children.is_empty());
        DesktopTextSelectionToolbar {
            key: None,
            anchor,
            children,
        }
    }

    /// Dart `DesktopTextSelectionToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> DesktopTextSelectionToolbar {
        self.key = Some(key);
        self
    }

    // Builds a desktop toolbar in the Material style.
    fn default_toolbar_builder(
        _app: &mut App,
        _context: BuildContext,
        child: WidgetRef,
    ) -> WidgetRef {
        SizedBox::new()
            .width(K_TOOLBAR_WIDTH)
            .child(
                Material::new()
                    .border_radius(BorderRadius::all(Radius::circular(7.0)))
                    .clip_behavior(Clip::AntiAlias)
                    .elevation(1.0)
                    .r#type(MaterialType::Card)
                    .child(child),
            )
            .into_widget()
    }
}

impl StatelessWidget for DesktopTextSelectionToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_media_query(app, context));

        let padding_above = MediaQuery::padding_of(app, context).top + K_TOOLBAR_SCREEN_PADDING;
        let local_adjustment = Offset::new(K_TOOLBAR_SCREEN_PADDING, padding_above);

        Padding::new(EdgeInsetsGeometry::from_ltrb(
            K_TOOLBAR_SCREEN_PADDING,
            padding_above,
            K_TOOLBAR_SCREEN_PADDING,
            K_TOOLBAR_SCREEN_PADDING,
        ))
        .child(
            CustomSingleChildLayout::new(Rc::new(DesktopTextSelectionToolbarLayoutDelegate::new(
                self.anchor - local_adjustment,
            )))
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

#[cfg(test)]
mod tests {

    use waft_foundation::Listener;
    use waft_widgets::{Center, Text};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::desktop_text_selection_toolbar_button::DesktopTextSelectionToolbarButton;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// `MaterialApp(home: Center(child: child))`.
    fn pump_centered<K>(tester: &mut WidgetTester, child: impl IntoWidget<K>) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::new(&app),
                Center::new().child(child).into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    #[test]
    fn positions_itself_at_the_anchor() {
        test_widgets(|tester| {
            // An arbitrary point on the screen to position at.
            let anchor = Offset::new(30.0, 40.0);

            pump_centered(
                tester,
                DesktopTextSelectionToolbar::new(
                    anchor,
                    [DesktopTextSelectionToolbarButton::new(
                        Some(Listener::new(|_app| {})),
                        Text::new("Tap me"),
                    )
                    .into_widget()],
                ),
            );

            let buttons = tester.find_by_type::<DesktopTextSelectionToolbarButton>();
            let top_left = tester
                .global_rect(tester.render_box_of(buttons[0]))
                .top_left();
            assert_eq!(top_left, anchor);
        });
    }

    #[test]
    fn desktop_text_selection_toolbar_renders_at_zero_area() {
        test_widgets(|tester| {
            pump_centered(
                tester,
                SizedBox::shrink().child(DesktopTextSelectionToolbar::new(
                    Offset::new(10.0, 10.0),
                    [Text::new("X").into_widget()],
                )),
            );
            let toolbars = tester.find_by_type::<DesktopTextSelectionToolbar>();
            let render = tester.render_box_of(toolbars[0]);
            assert!(render.size(&tester.cell.borrow()).is_empty());
        });
    }
}
