//! XAML `Page` (`dxaml/xcp/dxaml/lib/Page_Partial.cpp`): the root of an app's content, whose background covers all of it while its content stays clear of what the system draws over the window.
//!
//! WinUI's `CalculateUpdatedBounds` arranges `Content` inside the window's layout bounds, the window less the system's bars, when the page is the window's size. Here the bars are the ambient [`MediaQuery`]'s `padding`, and the keyboard its `view_insets.bottom`, as Flutter's `Scaffold` takes it with `resizeToAvoidBottomInset`. The content's `MediaQuery` has both removed, so a page or `SafeArea` inside this one does not keep the content clear of them a second time: a later editor who forwards the ambient data unchanged pads nested pages twice.

use crate::{Brush, ControlBorder};
use waft_foundation::App;
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::*;

/// XAML `Page`: content on a background, clear of the system's bars and keyboard.
#[derive(Clone, Debug, Default)]
pub struct Page {
    /// `Content`: what the page shows, inside the area the system's bars and the keyboard leave free.
    pub content: Option<WidgetRef>,

    /// `Background`: painted over the page's whole area, under the system's bars too. `None` paints nothing, as XAML's null brush.
    pub background: Option<Brush>,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl Page {
    /// Creates a page with no content and no background.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the XAML `Content`.
    pub fn content<K>(mut self, content: impl IntoWidget<K>) -> Self {
        self.content = Some(content.into_widget());
        self
    }

    /// Sets the XAML `Background`.
    pub fn background(mut self, background: Brush) -> Self {
        self.background = Some(background);
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for Page {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let page = ControlBorder::new_optional(self.background, None).border_thickness(0.0);
        match &self.content {
            Some(content) => page.child(updated_bounds(app, context, content.clone())),
            None => page,
        }
        .into_widget()
    }
}

/// `Page::CalculateUpdatedBounds`: the content inset by the system's bars and the keyboard, under a [`MediaQuery`] that no longer reports either.
fn updated_bounds(app: &mut App, context: BuildContext, content: WidgetRef) -> WidgetRef {
    let media = MediaQuery::of(app, context);
    let bars = media.padding;
    let keyboard = media.view_insets.bottom;
    let consumed = media
        .remove_padding(true, true, true, true)
        .remove_view_insets(false, false, false, true);
    MediaQuery::new(
        consumed,
        Padding::new(EdgeInsetsGeometry::from_ltrb(
            bars.left,
            bars.top,
            bars.right,
            bars.bottom + keyboard,
        ))
        .child(content),
    )
    .into_widget()
}
