//! Flutter counterpart: `widgets/default_selection_style.dart`.

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::AnyColor;
use waft_services::MouseCursorRef;

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, StatelessWidget, WidgetRef,
};
use crate::widgets::basic::Builder;
use crate::widgets::inherited_theme::InheritedTheme;

/// The default cursor and selection color (semi-transparent grey) for the current platform.
///
/// The selection color is used by `Text` widgets in a `SelectionArea` and by editable text
/// widgets that read this style.
///
/// See also:
///
///  * `DefaultTextStyle`, which provides the default text style.
#[derive(Debug)]
pub struct DefaultSelectionStyle {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// The default cursor color.
    pub cursor_color: Option<AnyColor>,
    /// The default selection color.
    pub selection_color: Option<AnyColor>,
    /// The [`MouseCursor`](waft_services::MouseCursor) for mouse pointers hovering over
    /// selectable Text widgets.
    ///
    /// If this property is `None`, `SystemMouseCursors::text` will be used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl DefaultSelectionStyle {
    /// The default color.
    pub const DEFAULT_COLOR: Color = Color::new(0x80808080);

    /// Creates a default selection style widget that specifies the selection properties for
    /// all widgets below it in the widget tree; Dart's optional named arguments are the
    /// setters.
    pub fn new<K>(child: impl IntoWidget<K>) -> DefaultSelectionStyle {
        DefaultSelectionStyle {
            key: None,
            cursor_color: None,
            selection_color: None,
            mouse_cursor: None,
            child: child.into_widget(),
        }
    }

    /// A const-constructable default selection style that provides fallback values
    /// (`None`).
    ///
    /// Returned from [`of`](Self::of) when the given [`BuildContext`] doesn't have an
    /// enclosing default selection style.
    ///
    /// This constructor creates a [`DefaultSelectionStyle`] with an invalid `child`, which
    /// means the constructed value cannot be incorporated into the tree.
    pub fn fallback() -> DefaultSelectionStyle {
        DefaultSelectionStyle::new(NullWidget)
    }

    /// Dart `DefaultSelectionStyle(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultSelectionStyle {
        self.key = Some(key);
        self
    }

    /// Dart `DefaultSelectionStyle(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: impl Into<AnyColor>) -> DefaultSelectionStyle {
        self.cursor_color = Some(cursor_color.into());
        self
    }

    /// Dart `DefaultSelectionStyle(selectionColor:)`.
    pub fn selection_color(
        mut self,
        selection_color: impl Into<AnyColor>,
    ) -> DefaultSelectionStyle {
        self.selection_color = Some(selection_color.into());
        self
    }

    /// Dart `DefaultSelectionStyle(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> DefaultSelectionStyle {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Creates a default selection style that overrides the selection styles in scope at
    /// this point in the widget tree.
    ///
    /// Any arguments that are not `None` replace the corresponding properties on the
    /// ambient default selection style. Dart's optional named arguments are the setters of
    /// the [`DefaultSelectionStyleMerge`] this returns.
    pub fn merge<K>(child: impl IntoWidget<K>) -> DefaultSelectionStyleMerge {
        DefaultSelectionStyleMerge {
            key: None,
            cursor_color: None,
            selection_color: None,
            mouse_cursor: None,
            child: child.into_widget(),
        }
    }

    /// The closest instance of this class that encloses the given context.
    ///
    /// If no such instance exists, this method will return
    /// [`fallback`](Self::fallback).
    pub fn of(app: &mut App, context: BuildContext) -> DefaultSelectionStyle {
        match context.depend_on_inherited_widget_of_exact_type::<DefaultSelectionStyle>(app) {
            Some(style) => DefaultSelectionStyle {
                key: style.key.clone(),
                cursor_color: style.cursor_color.clone(),
                selection_color: style.selection_color.clone(),
                mouse_cursor: style.mouse_cursor.clone(),
                child: style.child.clone(),
            },
            None => DefaultSelectionStyle::fallback(),
        }
    }
}

/// Dart's `mouseCursor != oldWidget.mouseCursor` over two optional cursors.
fn cursors_differ(a: &Option<MouseCursorRef>, b: &Option<MouseCursorRef>) -> bool {
    match (a, b) {
        (None, None) => false,
        (Some(a), Some(b)) => **a != **b,
        _ => true,
    }
}

impl InheritedTheme for DefaultSelectionStyle {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DefaultSelectionStyle {
            key: None,
            cursor_color: self.cursor_color.clone(),
            selection_color: self.selection_color.clone(),
            mouse_cursor: self.mouse_cursor.clone(),
            child,
        }
        .into_widget()
    }
}

impl InheritedWidget for DefaultSelectionStyle {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DefaultSelectionStyle) -> bool {
        self.cursor_color != old_widget.cursor_color
            || self.selection_color != old_widget.selection_color
            || cursors_differ(&self.mouse_cursor, &old_widget.mouse_cursor)
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Dart's `DefaultSelectionStyle.merge(..)` before its optional arguments are known: a
/// `Builder` that merges them into the ambient [`DefaultSelectionStyle`].
#[derive(Debug)]
pub struct DefaultSelectionStyleMerge {
    key: Option<KeyRef>,
    cursor_color: Option<AnyColor>,
    selection_color: Option<AnyColor>,
    mouse_cursor: Option<MouseCursorRef>,
    child: WidgetRef,
}

impl DefaultSelectionStyleMerge {
    /// Dart `DefaultSelectionStyle.merge(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultSelectionStyleMerge {
        self.key = Some(key);
        self
    }

    /// Dart `DefaultSelectionStyle.merge(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: impl Into<AnyColor>) -> DefaultSelectionStyleMerge {
        self.cursor_color = Some(cursor_color.into());
        self
    }

    /// Dart `DefaultSelectionStyle.merge(selectionColor:)`.
    pub fn selection_color(
        mut self,
        selection_color: impl Into<AnyColor>,
    ) -> DefaultSelectionStyleMerge {
        self.selection_color = Some(selection_color.into());
        self
    }

    /// Dart `DefaultSelectionStyle.merge(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> DefaultSelectionStyleMerge {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }
}

/// The kind tag of [`IntoWidget`] for a [`DefaultSelectionStyleMerge`].
pub struct DefaultSelectionStyleMergeKind;

impl IntoWidget<DefaultSelectionStyleMergeKind> for DefaultSelectionStyleMerge {
    fn into_widget(self) -> WidgetRef {
        Builder::new(move |app, context| {
            let parent = DefaultSelectionStyle::of(app, context);
            DefaultSelectionStyle {
                key: self.key.clone(),
                cursor_color: self.cursor_color.clone().or(parent.cursor_color),
                selection_color: self.selection_color.clone().or(parent.selection_color),
                mouse_cursor: self.mouse_cursor.clone().or(parent.mouse_cursor),
                child: self.child.clone(),
            }
            .into_widget()
        })
        .into_widget()
    }
}

/// Dart's `_NullWidget`: the child of [`DefaultSelectionStyle::fallback`], which must never
/// build.
#[derive(Debug)]
struct NullWidget;

impl StatelessWidget for NullWidget {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        panic!(
            "A DefaultSelectionStyle constructed with DefaultSelectionStyle.fallback cannot be \
             incorporated into the widget tree, it is meant only to provide a fallback value \
             returned by DefaultSelectionStyle.of() when no enclosing default selection style \
             is present in a BuildContext."
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_foundation::AppCell;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    fn reader(seen: &Rc<RefCell<Option<DefaultSelectionStyle>>>) -> WidgetRef {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            *seen.borrow_mut() = Some(DefaultSelectionStyle::of(app, context));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    /// Without an ancestor, `of` is the fallback; `merge` keeps what it leaves unset and
    /// replaces what it sets.
    #[test]
    fn of_falls_back_and_merge_overrides_only_what_it_sets() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen: Rc<RefCell<Option<DefaultSelectionStyle>>> = Rc::default();
        let harness = Harness::mount(&mut app, reader(&seen));
        harness.pump(&mut app);
        let fallback = seen.borrow_mut().take().expect("built");
        assert!(fallback.cursor_color.is_none() && fallback.selection_color.is_none());

        let red = Color::new(0xFFFF0000);
        let blue = Color::new(0xFF0000FF);
        harness.set_child(
            &mut app,
            DefaultSelectionStyle::new(
                DefaultSelectionStyle::merge(reader(&seen)).selection_color(blue),
            )
            .cursor_color(red)
            .selection_color(red)
            .into_widget(),
        );
        harness.pump(&mut app);
        let merged = seen.borrow_mut().take().expect("built");
        assert_eq!(merged.cursor_color, Some(AnyColor::new(red)));
        assert_eq!(merged.selection_color, Some(AnyColor::new(blue)));
    }

    /// A [`DefaultSelectionStyle`] is an [`InheritedTheme`]: a capture carries it.
    #[test]
    fn a_capture_carries_the_selection_style() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let context: Rc<RefCell<Option<BuildContext>>> = Rc::default();
        let tree = {
            let context = Rc::clone(&context);
            DefaultSelectionStyle::new(Builder::new(move |_app, builder_context| {
                *context.borrow_mut() = Some(builder_context);
                SizedBox::shrink().into_widget()
            }))
            .cursor_color(Color::new(0xFF00FF00))
        };
        let harness = Harness::mount(&mut app, tree.into_widget());
        harness.pump(&mut app);
        let from = context.borrow().expect("built");
        let seen: Rc<RefCell<Option<DefaultSelectionStyle>>> = Rc::default();
        let child = <dyn InheritedTheme>::capture_all(&app, from, reader(&seen), None);
        let route = Harness::mount(&mut app, child);
        route.pump(&mut app);
        assert_eq!(
            seen.borrow().as_ref().expect("built").cursor_color,
            Some(AnyColor::new(Color::new(0xFF00FF00)))
        );
    }
}
