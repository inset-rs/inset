//! Flutter `cupertino/form_row.dart`.

use reveal_embedder::FontWeight;
use reveal_foundation::App;
use reveal_painting::{AlignmentDirectional, EdgeInsetsGeometry, TextStyle};
use reveal_rendering::MainAxisAlignment;
use reveal_widgets::{
    Align, BuildContext, Column, DefaultTextStyle, Flexible, IntoWidget, KeyRef, Padding, Row,
    StatelessWidget, WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::theme::CupertinoTheme;

// Content padding determined via SwiftUI's `Form` view in the iOS 14.2 SDK.
const DEFAULT_PADDING: EdgeInsetsGeometry = EdgeInsetsGeometry::from_steb(20.0, 6.0, 6.0, 6.0);

/// An iOS-style form row.
///
/// Creates an iOS-style split form row with a standard prefix and child widget.
/// Also provides a space for error and helper widgets that appear underneath.
///
/// The `child` parameter is required. This widget is displayed at the end of
/// the row.
///
/// The `prefix` parameter is optional and is displayed at the start of the
/// row. Standard iOS guidelines encourage passing a `Text` widget to `prefix`
/// to detail the nature of the row's `child` widget.
///
/// The `padding` parameter is used to pad the contents of the row. It defaults
/// to the standard iOS padding. If no edge insets are intended, explicitly pass
/// `EdgeInsets::ZERO` to `padding`.
///
/// The `helper` and `error` parameters are both optional widgets targeted at
/// displaying more specific information about the row. Both widgets are placed
/// underneath the `prefix` and `child`, and will expand the row's height to
/// accommodate for their presence. When a `Text` is given to `error`, it will
/// be shown in `CupertinoColors::DESTRUCTIVE_RED` coloring and medium-weighted
/// font.
///
/// See also:
///
///  * `CupertinoFormSection`, an iOS-style form section.
#[derive(Debug)]
pub struct CupertinoFormRow {
    key: Option<KeyRef>,
    /// A widget that is displayed at the start of the row.
    ///
    /// The `prefix` widget is displayed at the start of the row. Standard iOS
    /// guidelines encourage passing a `Text` widget to `prefix` to detail the
    /// nature of the row's `child` widget. If null, the `child` widget will take
    /// up all horizontal space in the row.
    prefix: Option<WidgetRef>,
    /// Content padding for the row.
    ///
    /// Defaults to the standard iOS padding for form rows. If no edge insets
    /// are intended, explicitly pass `EdgeInsets::ZERO` to `padding`.
    padding: Option<EdgeInsetsGeometry>,
    /// A widget that is displayed underneath the `prefix` and `child` widgets.
    ///
    /// The `helper` appears in primary label coloring, and is meant to inform the
    /// user about interaction with the child widget. The row becomes taller in
    /// order to display the `helper` widget underneath `prefix` and `child`. If
    /// null, the row is shorter.
    helper: Option<WidgetRef>,
    /// A widget that is displayed underneath the `prefix` and `child` widgets.
    ///
    /// The `error` widget is primarily used to inform users of input errors. When
    /// a `Text` is given to `error`, it will be shown in
    /// `CupertinoColors::DESTRUCTIVE_RED` coloring and medium-weighted font. The
    /// row becomes taller in order to display the `helper` widget underneath
    /// `prefix` and `child`. If null, the row is shorter.
    error: Option<WidgetRef>,
    /// Child widget.
    ///
    /// The child widget is displayed at the end of the row.
    child: WidgetRef,
}

impl CupertinoFormRow {
    /// Creates an iOS-style split form row with a standard prefix and child widget.
    /// Also provides a space for error and helper widgets that appear underneath.
    pub fn new<K>(child: impl IntoWidget<K>) -> CupertinoFormRow {
        CupertinoFormRow {
            key: None,
            prefix: None,
            padding: None,
            helper: None,
            error: None,
            child: child.into_widget(),
        }
    }

    /// Dart `CupertinoFormRow(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoFormRow {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoFormRow(prefix:)`.
    pub fn prefix<K>(mut self, prefix: impl IntoWidget<K>) -> CupertinoFormRow {
        self.prefix = Some(prefix.into_widget());
        self
    }

    /// Dart `CupertinoFormRow(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> CupertinoFormRow {
        self.padding = Some(padding);
        self
    }

    /// Dart `CupertinoFormRow(helper:)`.
    pub fn helper<K>(mut self, helper: impl IntoWidget<K>) -> CupertinoFormRow {
        self.helper = Some(helper.into_widget());
        self
    }

    /// Dart `CupertinoFormRow(error:)`.
    pub fn error<K>(mut self, error: impl IntoWidget<K>) -> CupertinoFormRow {
        self.error = Some(error.into_widget());
        self
    }
}

impl StatelessWidget for CupertinoFormRow {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = CupertinoTheme::of(app, context);
        let theme_text_style = theme.text_theme().text_style();
        let mut text_style = theme_text_style.copy_with();
        if let Some(color) =
            CupertinoDynamicColor::maybe_resolve(theme_text_style.color.as_ref(), app, context)
        {
            text_style = text_style.color(color);
        }

        let mut row_children = Vec::new();
        if let Some(prefix) = &self.prefix {
            row_children
                .push(DefaultTextStyle::new(text_style.clone(), prefix.clone()).into_widget());
        }
        row_children.push(
            Flexible::new(
                Align::new()
                    .alignment(AlignmentDirectional::CENTER_END.into())
                    .child(self.child.clone()),
            )
            .into_widget(),
        );

        let mut children = vec![
            Row::new()
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .children(row_children)
                .into_widget(),
        ];
        if let Some(helper) = &self.helper {
            children.push(
                Align::new()
                    .alignment(AlignmentDirectional::CENTER_START.into())
                    .child(DefaultTextStyle::new(text_style.clone(), helper.clone()))
                    .into_widget(),
            );
        }
        if let Some(error) = &self.error {
            children.push(
                Align::new()
                    .alignment(AlignmentDirectional::CENTER_START.into())
                    .child(DefaultTextStyle::new(
                        TextStyle::new()
                            .color(CupertinoColors::DESTRUCTIVE_RED)
                            .font_weight(FontWeight::W500),
                        error.clone(),
                    ))
                    .into_widget(),
            );
        }
        Padding::new(self.padding.unwrap_or(DEFAULT_PADDING))
            .child(Column::new().children(children))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_embedder::TextDirection;
    use reveal_painting::AnyColor;
    use reveal_widgets::{Builder, Directionality, SizedBox};

    use super::*;
    use crate::test_support::{build, test_cell};
    use crate::theme::CupertinoThemeData;

    /// Records the text style in force where a row places a slot, next to the theme's label
    /// colour resolved at that spot.
    fn probe(seen: &Rc<RefCell<Vec<(TextStyle, AnyColor)>>>) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            let style = DefaultTextStyle::of(app, context).style;
            let label = CupertinoDynamicColor::resolve(&CupertinoColors::LABEL, app, context);
            seen.borrow_mut().push((style, label));
            SizedBox::shrink().into_widget()
        })
    }

    #[test]
    fn a_form_row_styles_its_prefix_and_helper_with_the_theme_and_its_error_in_red() {
        let cell = test_cell();
        let app = cell.borrow();
        let seen = Rc::new(RefCell::new(Vec::new()));
        drop(app);
        build(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                CupertinoTheme::new(
                    CupertinoThemeData::new(),
                    CupertinoFormRow::new(SizedBox::shrink())
                        .prefix(probe(&seen))
                        .helper(probe(&seen))
                        .error(probe(&seen)),
                ),
            )
            .into_widget(),
        );
        let seen = seen.borrow();
        assert_eq!(seen.len(), 3, "prefix, helper, error");
        let (prefix, label) = &seen[0];
        assert_eq!(prefix.color.as_ref(), Some(label));
        let (helper, label) = &seen[1];
        assert_eq!(helper.color.as_ref(), Some(label));
        let (error, _) = &seen[2];
        assert_eq!(error.color, Some(CupertinoColors::DESTRUCTIVE_RED));
        assert_eq!(error.font_weight, Some(FontWeight::W500));
    }
}
