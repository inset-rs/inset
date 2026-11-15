//! Flutter counterpart: `widgets/inherited_theme.dart`.

use std::any::TypeId;
use std::collections::HashSet;
use std::fmt::{self, Debug};

use waft_foundation::App;

use crate::framework::{BuildContext, IntoWidget, StatelessWidget, WidgetRef};

/// An [`InheritedWidget`](crate::InheritedWidget) that defines visual properties like colors
/// and text styles, which the `child`'s subtree depends on.
///
/// The [`wrap`](Self::wrap) method is used by [`capture_all`](<dyn InheritedTheme>::capture_all)
/// and [`CapturedThemes::wrap`] to construct a widget that will wrap a child in all of the
/// inherited themes which are present in a specified part of the widget tree.
///
/// A widget that's shown in a different context from the one it's built in, like the contents
/// of a new route or an overlay, will be able to see the ancestor inherited themes of the
/// context it was built in.
///
/// An implementor is an inherited widget whose
/// [`as_inherited_theme`](crate::InheritedWidget::as_inherited_theme) answers with itself.
pub trait InheritedTheme: Debug + 'static {
    /// Return a copy of this inherited theme with the specified `child`.
    ///
    /// This implementation for `TooltipTheme` is typical:
    ///
    /// ```text
    /// fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
    ///     TooltipTheme::new(self.data.clone(), child).into_widget()
    /// }
    /// ```
    fn wrap(&self, app: &mut App, context: BuildContext, child: WidgetRef) -> WidgetRef;
}

impl dyn InheritedTheme {
    /// Returns a widget that will `wrap` `child` in all of the inherited themes which are
    /// present between `context` and the specified `to` [`BuildContext`].
    ///
    /// The `to` context must be an ancestor of `context`. If `to` is not specified, all
    /// inherited themes up to the root of the widget tree are captured.
    ///
    /// After calling this method, the themes present between `context` and `to` are frozen
    /// for the provided `child`. If the themes (or their theme data) change in the original
    /// subtree, those changes will not be visible to the wrapped `child` - unless this method
    /// is called again to re-wrap the child.
    pub fn capture_all(
        app: &App,
        context: BuildContext,
        child: WidgetRef,
        to: Option<BuildContext>,
    ) -> WidgetRef {
        <dyn InheritedTheme>::capture(app, context, to).wrap(child)
    }

    /// Returns a [`CapturedThemes`] object that includes all the [`InheritedTheme`]s between
    /// the given `from` and `to` [`BuildContext`]s.
    ///
    /// The `to` context must be an ancestor of the `from` context. If `to` is `None`, all
    /// ancestor inherited themes of `from` up to the root of the widget tree are captured.
    ///
    /// After calling this method, the themes present between `from` and `to` are frozen in
    /// the returned [`CapturedThemes`] object. If the themes (or their theme data) change in
    /// the original subtree, those changes will not be applied to the themes captured in the
    /// [`CapturedThemes`] object - unless this method is called again to re-capture the
    /// updated themes.
    ///
    /// To wrap a [`Widget`](crate::Widget) in the captured themes, call
    /// [`CapturedThemes::wrap`].
    ///
    /// This method can be expensive if there are many widgets between `from` and `to` (it
    /// walks the element tree between those nodes).
    pub fn capture(app: &App, from: BuildContext, to: Option<BuildContext>) -> CapturedThemes {
        if Some(from) == to {
            // Nothing to capture.
            return CapturedThemes { themes: Vec::new() };
        }

        let mut themes = Vec::new();
        let mut theme_types = HashSet::<TypeId>::new();
        let mut debug_did_find_ancestor = to.is_none();
        from.visit_ancestor_elements(app, &mut |ancestor| {
            if Some(ancestor) == to {
                debug_did_find_ancestor = true;
                return false;
            }
            let widget = ancestor.widget(app);
            if ancestor.is_inherited_element() && widget.as_inherited_theme().is_some() {
                let theme_type = widget.widget_type();
                // Only remember the first theme of any type. This assumes
                // that inherited themes completely shadow ancestors of the
                // same type.
                if theme_types.insert(theme_type) {
                    themes.push(widget.clone());
                }
            }
            true
        });
        debug_assert!(
            debug_did_find_ancestor,
            "The provided `to` context must be an ancestor of the `from` context."
        );
        CapturedThemes { themes }
    }
}

/// Stores a list of captured [`InheritedTheme`]s that can be wrapped around a child
/// [`Widget`](crate::Widget).
///
/// Used as return type by [`capture`](<dyn InheritedTheme>::capture).
pub struct CapturedThemes {
    /// The captured themes, nearest first; each answers
    /// [`as_inherited_theme`](crate::Widget::as_inherited_theme).
    themes: Vec<WidgetRef>,
}

impl CapturedThemes {
    /// Wraps a `child` [`Widget`](crate::Widget) in the [`InheritedTheme`]s captured in this
    /// object.
    pub fn wrap(&self, child: WidgetRef) -> WidgetRef {
        CaptureAll {
            themes: self.themes.clone(),
            child,
        }
        .into_widget()
    }
}

impl Debug for CapturedThemes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapturedThemes")
            .field("themes", &self.themes)
            .finish()
    }
}

/// Dart's `_CaptureAll`.
#[derive(Debug)]
struct CaptureAll {
    themes: Vec<WidgetRef>,
    child: WidgetRef,
}

impl StatelessWidget for CaptureAll {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut wrapped_child = self.child.clone();
        for theme in &self.themes {
            wrapped_child = theme
                .as_inherited_theme()
                .expect("a captured theme is an inherited theme")
                .wrap(app, context, wrapped_child);
        }
        wrapped_child
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use waft_embedder::Color;
    use waft_foundation::AppCell;
    use waft_painting::TextStyle;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::icon_theme::IconTheme;
    use crate::widgets::icon_theme_data::IconThemeData;
    use crate::widgets::text::DefaultTextStyle;

    const FONT_SIZE: f64 = 32.0;
    const ICON_SIZE: f64 = 48.0;
    const TEXT_COLOR: Color = Color::new(0xFF00FF00);
    const ICON_COLOR: Color = Color::new(0xFF0000FF);

    /// What a probe saw: the default text style's font size and the icon theme's size.
    type Seen = Rc<RefCell<Option<(Option<f64>, Option<f64>)>>>;

    fn probe(seen: &Seen) -> WidgetRef {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            let text = DefaultTextStyle::of(app, context).style.font_size;
            let icon = IconTheme::of(app, context).size;
            *seen.borrow_mut() = Some((text, icon));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    /// `inherited_theme_test.dart`'s tree: outer themes that must not count, inner themes that
    /// do, and a builder under them that reports its context.
    fn themed(context: &Rc<Cell<Option<BuildContext>>>, child: WidgetRef) -> WidgetRef {
        let context = Rc::clone(context);
        DefaultTextStyle::new(
            TextStyle::new().font_size(ICON_SIZE).color(ICON_COLOR),
            IconTheme::new(
                IconThemeData::new().size(FONT_SIZE).color(TEXT_COLOR),
                DefaultTextStyle::new(
                    TextStyle::new().font_size(FONT_SIZE).color(TEXT_COLOR),
                    IconTheme::new(
                        IconThemeData::new().size(ICON_SIZE).color(ICON_COLOR),
                        Builder::new(move |_app, builder_context| {
                            context.set(Some(builder_context));
                            child.clone()
                        }),
                    ),
                ),
            ),
        )
        .into_widget()
    }

    /// `inherited_theme_test.dart`: "InheritedTheme.captureAll()", with a second tree standing
    /// in for the pushed route: only a captured child sees the closest themes.
    #[test]
    fn capture_all_carries_the_closest_themes_into_another_tree() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let home: Seen = Rc::default();
        let harness = Harness::mount(&mut app, themed(&context, probe(&home)));
        harness.pump(&mut app);
        assert_eq!(*home.borrow(), Some((Some(FONT_SIZE), Some(ICON_SIZE))));

        let plain: Seen = Rc::default();
        let route = Harness::mount(&mut app, probe(&plain));
        route.pump(&mut app);
        let (text, icon) = plain.borrow().expect("the probe built");
        assert_ne!(
            text,
            Some(FONT_SIZE),
            "an uncaptured child does not inherit the style"
        );
        assert_ne!(
            icon,
            Some(ICON_SIZE),
            "an uncaptured child does not inherit the icon theme"
        );

        let captured: Seen = Rc::default();
        let context = context.get().expect("the builder ran");
        let child = <dyn InheritedTheme>::capture_all(&app, context, probe(&captured), None);
        let route = Harness::mount(&mut app, child);
        route.pump(&mut app);
        assert_eq!(*captured.borrow(), Some((Some(FONT_SIZE), Some(ICON_SIZE))));
    }

    /// A capture up to an ancestor `to` leaves out the themes above it.
    #[test]
    fn capture_stops_at_the_to_context() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let outer: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let inner: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let inner_builder = {
            let inner = Rc::clone(&inner);
            Builder::new(move |_app, context| {
                inner.set(Some(context));
                SizedBox::shrink().into_widget()
            })
            .into_widget()
        };
        let tree = {
            let outer = Rc::clone(&outer);
            IconTheme::new(
                IconThemeData::new().size(ICON_SIZE),
                Builder::new(move |_app, context| {
                    outer.set(Some(context));
                    DefaultTextStyle::new(
                        TextStyle::new().font_size(FONT_SIZE),
                        inner_builder.clone(),
                    )
                    .into_widget()
                }),
            )
        };
        let harness = Harness::mount(&mut app, tree.into_widget());
        harness.pump(&mut app);

        let captured: Seen = Rc::default();
        let from = inner.get().expect("the inner builder ran");
        let to = outer.get().expect("the outer builder ran");
        let child = <dyn InheritedTheme>::capture(&app, from, Some(to)).wrap(probe(&captured));
        let route = Harness::mount(&mut app, child);
        route.pump(&mut app);
        let (text, icon) = captured.borrow().expect("the probe built");
        assert_eq!(text, Some(FONT_SIZE), "the style below `to` is captured");
        assert_ne!(icon, Some(ICON_SIZE), "the icon theme above `to` is not");
        assert!(
            <dyn InheritedTheme>::capture(&app, from, Some(from))
                .themes
                .is_empty(),
            "a capture from a context to itself is empty"
        );
    }

    /// `DefaultTextStyle::merge` merges into the ambient style and keeps what it leaves unset.
    #[test]
    fn default_text_style_merge_merges_into_the_ambient_style() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen: Rc<RefCell<Option<DefaultTextStyle>>> = Rc::default();
        let reader = {
            let seen = Rc::clone(&seen);
            Builder::new(move |app, context| {
                *seen.borrow_mut() = Some(DefaultTextStyle::of(app, context));
                SizedBox::shrink().into_widget()
            })
        };
        let harness = Harness::mount(
            &mut app,
            DefaultTextStyle::new(
                TextStyle::new().font_size(FONT_SIZE).color(TEXT_COLOR),
                DefaultTextStyle::merge(reader)
                    .style(TextStyle::new().color(ICON_COLOR))
                    .max_lines(2),
            )
            .max_lines(5)
            .soft_wrap(false)
            .into_widget(),
        );
        harness.pump(&mut app);
        let merged = seen.borrow().clone().expect("the reader built");
        assert_eq!(merged.style.font_size, Some(FONT_SIZE));
        assert_eq!(
            merged.style.color.map(|color| color.color()),
            Some(ICON_COLOR)
        );
        assert_eq!(merged.max_lines, Some(2));
        assert!(
            !merged.soft_wrap,
            "an unset argument keeps the ambient value"
        );
    }
}
