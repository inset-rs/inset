//! `SortIndicator` (`controls/dev/SortIndicator`: `SortIndicator.xaml`, `SortIndicator_themeresources.xaml` and `SortIndicator.cpp`): the policy-free chevron a column header shows for its sort direction. It owns no sorting; its host sets the direction and positions it.

use crate::*;
use waft_foundation::App;
use waft_widgets::*;

/// `SortIndicatorDirection`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum SortIndicatorDirection {
    #[default]
    None,
    Ascending,
    Descending,
}

/// XAML `SortIndicator`: a `SortIndicatorSize` box holding the direction's chevron, hidden when there is no sort and taking no input.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SortIndicator {
    /// `Direction`.
    pub(crate) direction: SortIndicatorDirection,
}

impl SortIndicator {
    pub(crate) fn new(direction: SortIndicatorDirection) -> Self {
        Self { direction }
    }
}

impl StatelessWidget for SortIndicator {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        // `UpdateVisualState`: the direction's glyph, and `LayoutRoot.Opacity` 1 only when there is a sort.
        let (has_sort, glyph) = match self.direction {
            SortIndicatorDirection::Ascending => (true, FluentSymbol::ChevronUp),
            SortIndicatorDirection::Descending => (true, FluentSymbol::ChevronDown),
            SortIndicatorDirection::None => (false, FluentSymbol::ChevronUp),
        };
        // GlyphIcon
        let glyph_icon = FontIcon::symbol(glyph)
            .font_size(r.sort_indicator_glyph_size)
            .foreground(r.sort_indicator_foreground);
        // LayoutRoot > DirectionStateRoot
        let layout_root =
            Opacity::new(if has_sort { 1.0 } else { 0.0 }).child(Center::new().child(glyph_icon));
        IgnorePointer::new()
            .child(
                SizedBox::new()
                    .width(r.sort_indicator_size)
                    .height(r.sort_indicator_size)
                    .child(layout_root),
            )
            .into_widget()
    }
}
