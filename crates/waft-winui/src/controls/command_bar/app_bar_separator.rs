//! XAML `AppBarSeparator` with the `DefaultAppBarSeparatorStyle` of `AppBarSeparator_themeresources.xaml`, and `AppBarSeparator_Partial.cpp`.
//!
//! The template is `RootGrid` > `SeparatorRectangle`, a rounded line of `AppBarSeparatorWidth` filled with `Foreground` inside the `Padding` margin. `ApplicationViewStates` picks `FullSize`, where the line stretches to the bar's height, `Compact`, where `RootGrid` is `AppBarThemeCompactHeight` tall at the top, or `Overflow`, where the line runs across the menu.

use super::element::ElementPresentation;
use crate::{Margin, ThemeResources};
use waft_embedder::Radius;
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, BorderRadius};
use waft_widgets::*;

/// XAML `AppBarSeparator`: a line between groups of commands.
#[derive(Clone, Debug, Default)]
pub struct AppBarSeparator {
    /// `DynamicOverflowOrder`: when the separator moves to the overflow, lowest first; 0 moves with the rightmost commands.
    pub dynamic_overflow_order: i32,
    pub(crate) presentation: ElementPresentation,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl AppBarSeparator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the XAML `DynamicOverflowOrder`.
    pub fn dynamic_overflow_order(mut self, order: i32) -> Self {
        self.dynamic_overflow_order = order;
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for AppBarSeparator {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let radius = Radius::circular(r.app_bar_separator_corner_radius);
        let line = |width: Option<f64>, height: Option<f64>| {
            let mut sized = SizedBox::new();
            if let Some(width) = width {
                sized = sized.width(width);
            }
            if let Some(height) = height {
                sized = sized.height(height);
            }
            ClipRRect::new()
                .border_radius(BorderRadius::all(radius).into())
                .child(sized.child(ColoredBox::new(r.app_bar_separator_foreground)))
        };
        let presentation = self.presentation;
        // ChangeVisualState
        if presentation.use_overflow_style {
            // Overflow
            return Margin::new(
                r.app_bar_overflow_separator_margin,
                line(None, Some(r.app_bar_overflow_separator_height)),
            )
            .into_widget();
        }
        let rectangle = Margin::new(
            r.app_bar_separator_margin,
            line(Some(r.app_bar_separator_width), None),
        );
        if presentation.is_compact {
            // Compact
            Align::new()
                .alignment(AlignmentGeometry::TOP_CENTER)
                .height_factor(1.0)
                .child(
                    SizedBox::new()
                        .height(r.app_bar_theme_compact_height)
                        .child(rectangle),
                )
                .into_widget()
        } else {
            // FullSize
            rectangle.into_widget()
        }
    }
}
