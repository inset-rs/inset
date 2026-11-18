//! Flutter counterpart: material_ui's `grid_tile_bar.dart`.

use std::rc::Rc;

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::{BoxDecoration, EdgeInsetsDirectional, TextOverflow};
use waft_widgets::{
    BuildContext, Column, Container, CrossAxisAlignment, DefaultTextStyle, Expanded, IconTheme,
    IconThemeData, IntoWidget, KeyRef, MainAxisAlignment, Padding, Row, StatelessWidget, WidgetRef,
};

use crate::colors::Colors;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

/// A header used in a Material Design [`GridTile`](crate::GridTile).
///
/// Typically used to add a one or two line header or footer on a
/// [`GridTile`](crate::GridTile).
///
/// For a one-line header, include a [`title`](Self::title) widget. To add a second line, also
/// include a [`subtitle`](Self::subtitle) widget. Use [`leading`](Self::leading) or
/// [`trailing`](Self::trailing) to add an icon.
///
/// See also:
///
///  * [`GridTile`](crate::GridTile)
///  * <https://material.io/design/components/image-lists.html#anatomy>
#[derive(Debug)]
pub struct GridTileBar {
    pub key: Option<KeyRef>,
    /// The color to paint behind the child widgets.
    ///
    /// Defaults to transparent.
    pub background_color: Option<Color>,
    /// A widget to display before the title.
    ///
    /// Typically an `Icon` or an `IconButton` widget.
    pub leading: Option<WidgetRef>,
    /// The primary content of the list item.
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,
    /// Additional content displayed below the title.
    ///
    /// Typically a `Text` widget.
    pub subtitle: Option<WidgetRef>,
    /// A widget to display after the title.
    ///
    /// Typically an `Icon` or an `IconButton` widget.
    pub trailing: Option<WidgetRef>,
}

impl GridTileBar {
    /// Creates a grid tile bar; Dart's optional named arguments are the setters.
    pub fn new() -> GridTileBar {
        GridTileBar {
            key: None,
            background_color: None,
            leading: None,
            title: None,
            subtitle: None,
            trailing: None,
        }
    }

    /// Dart `GridTileBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> GridTileBar {
        self.key = Some(key);
        self
    }

    /// Dart `GridTileBar(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> GridTileBar {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `GridTileBar(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> GridTileBar {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `GridTileBar(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> GridTileBar {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `GridTileBar(subtitle:)`.
    pub fn subtitle<K>(mut self, subtitle: impl IntoWidget<K>) -> GridTileBar {
        self.subtitle = Some(subtitle.into_widget());
        self
    }

    /// Dart `GridTileBar(trailing:)`.
    pub fn trailing<K>(mut self, trailing: impl IntoWidget<K>) -> GridTileBar {
        self.trailing = Some(trailing.into_widget());
        self
    }
}

impl Default for GridTileBar {
    fn default() -> GridTileBar {
        GridTileBar::new()
    }
}

impl StatelessWidget for GridTileBar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let dark_theme = Rc::new(ThemeData::dark(app, None));
        let padding = EdgeInsetsDirectional::only(
            if self.leading.is_some() { 8.0 } else { 16.0 },
            0.0,
            if self.trailing.is_some() { 8.0 } else { 16.0 },
            0.0,
        );
        let height = if self.title.is_some() && self.subtitle.is_some() {
            68.0
        } else {
            48.0
        };

        let mut children: Vec<WidgetRef> = Vec::new();
        if let Some(leading) = &self.leading {
            children.push(
                Padding::new(EdgeInsetsDirectional::only(0.0, 0.0, 8.0, 0.0))
                    .child(leading.clone())
                    .into_widget(),
            );
        }

        if self.title.is_some() && self.subtitle.is_some() {
            let title = DefaultTextStyle::new(
                dark_theme
                    .text_theme
                    .title_medium
                    .clone()
                    .expect("a dark theme has a title medium"),
                self.title.clone().expect("a title"),
            )
            .soft_wrap(false)
            .overflow(TextOverflow::Ellipsis);
            let subtitle = DefaultTextStyle::new(
                dark_theme
                    .text_theme
                    .body_small
                    .clone()
                    .expect("a dark theme has a body small"),
                self.subtitle.clone().expect("a subtitle"),
            )
            .soft_wrap(false)
            .overflow(TextOverflow::Ellipsis);
            children.push(
                Expanded::new(
                    Column::new()
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .children(vec![title.into_widget(), subtitle.into_widget()]),
                )
                .into_widget(),
            );
        } else if self.title.is_some() || self.subtitle.is_some() {
            let text = self
                .title
                .clone()
                .or_else(|| self.subtitle.clone())
                .expect("a title or a subtitle");
            children.push(
                Expanded::new(
                    DefaultTextStyle::new(
                        dark_theme
                            .text_theme
                            .title_medium
                            .clone()
                            .expect("a dark theme has a title medium"),
                        text,
                    )
                    .soft_wrap(false)
                    .overflow(TextOverflow::Ellipsis),
                )
                .into_widget(),
            );
        }

        if let Some(trailing) = &self.trailing {
            children.push(
                Padding::new(EdgeInsetsDirectional::only(8.0, 0.0, 0.0, 0.0))
                    .child(trailing.clone())
                    .into_widget(),
            );
        }

        let mut container = Container::new().padding(padding).height(height);
        if let Some(background_color) = self.background_color {
            container = container.decoration(BoxDecoration::new().color(background_color));
        }
        container
            .child(Theme::new(
                Rc::clone(&dark_theme),
                IconTheme::merge(
                    None,
                    IconThemeData::new().color(Colors::WHITE),
                    Row::new().children(children),
                ),
            ))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use waft_widgets::testing::{find, test_widgets};
    use waft_widgets::{Center, SizedBox, Text};

    use super::*;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn pump(tester: &mut waft_widgets::testing::WidgetTester, bar: GridTileBar) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, Center::new().child(bar).into_widget()));
    }

    fn height(tester: &waft_widgets::testing::WidgetTester) -> f64 {
        tester.get_size(find::by_type::<GridTileBar>()).height()
    }

    #[test]
    fn grid_tile_bar_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(SizedBox::shrink().child(GridTileBar::new().title(Text::new("X"))))
                    .into_widget(),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<GridTileBar>()),
                waft_embedder::Size::ZERO
            );
        });
    }

    #[test]
    fn a_bar_with_a_title_and_a_subtitle_is_sixty_eight_tall() {
        test_widgets(|tester| {
            pump(
                tester,
                GridTileBar::new()
                    .title(Text::new("Title"))
                    .subtitle(Text::new("Subtitle")),
            );
            assert_eq!(height(tester), 68.0);
        });
    }

    #[test]
    fn a_bar_with_one_line_is_forty_eight_tall() {
        test_widgets(|tester| {
            pump(tester, GridTileBar::new().title(Text::new("Title")));
            assert_eq!(height(tester), 48.0);
        });
    }

    #[test]
    fn a_leading_and_a_trailing_pad_the_bar_by_eight() {
        test_widgets(|tester| {
            pump(
                tester,
                GridTileBar::new()
                    .leading(SizedBox::square(Some(10.0)))
                    .trailing(SizedBox::square(Some(10.0))),
            );
            let bar = tester.get_top_left(find::by_type::<GridTileBar>());
            let leading = tester.get_top_left(find::by_type::<SizedBox>().first());
            assert_eq!(leading.dx(), bar.dx() + 8.0);
        });
    }

    #[test]
    fn a_title_without_a_leading_starts_sixteen_in() {
        test_widgets(|tester| {
            pump(tester, GridTileBar::new().title(Text::new("Title")));
            let bar = tester.get_top_left(find::by_type::<GridTileBar>());
            let title = tester.get_top_left(find::by_type::<Text>());
            assert_eq!(title.dx(), bar.dx() + 16.0);
        });
    }
}
