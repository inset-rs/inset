//! Flutter `widgets/preferred_size.dart`.

use std::fmt::{self, Debug};

use reveal_embedder::Size;
use reveal_foundation::App;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef, downcast_widget,
};

/// An interface for widgets that can return the size this widget would prefer
/// if it were otherwise unconstrained.
///
/// There are a few cases, notably `AppBar` and `TabBar`, where it would be
/// undesirable for the widget to constrain its own size but where the widget
/// needs to expose a preferred or "default" size. For example a primary
/// `Scaffold` sets its app bar height to the app bar's preferred height
/// plus the height of the system status bar.
///
/// Widgets that need to know the preferred size of their child can require
/// that their child implement this interface by using this class rather
/// than `Widget` as the type of their `child` property: the field holds a
/// [`PreferredSizeWidgetRef`].
///
/// Use [`PreferredSize`] to give a preferred size to an arbitrary widget.
pub trait PreferredSizeWidget: 'static {
    /// The size this widget would prefer if it were otherwise unconstrained.
    ///
    /// In many cases it's only necessary to define one preferred dimension.
    /// For example the `Scaffold` only depends on its app bar's preferred
    /// height. In that case implementations of this method can just return
    /// `Size::from_height(my_app_bar_height)`.
    fn preferred_size(&self) -> Size;
}

/// A [`PreferredSizeWidget`] held as a value: the erased widget, whose preferred size is
/// still answered by the widget it holds. Dart types such a field `PreferredSizeWidget`.
#[derive(Clone)]
pub struct PreferredSizeWidgetRef {
    widget: WidgetRef,
    preferred_size: fn(&WidgetRef) -> Size,
}

impl PreferredSizeWidgetRef {
    /// Erases `widget`.
    pub fn new<K, W: PreferredSizeWidget + IntoWidget<K>>(widget: W) -> PreferredSizeWidgetRef {
        PreferredSizeWidgetRef {
            widget: widget.into_widget(),
            preferred_size: |widget| {
                downcast_widget::<W>(&**widget)
                    .expect("an erased widget keeps its type")
                    .preferred_size()
            },
        }
    }

    /// See [`PreferredSizeWidget::preferred_size`].
    pub fn preferred_size(&self) -> Size {
        (self.preferred_size)(&self.widget)
    }

    /// The widget itself.
    pub fn widget(&self) -> &WidgetRef {
        &self.widget
    }
}

impl Debug for PreferredSizeWidgetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.widget, f)
    }
}

/// The kind tag of [`IntoWidget`] for a [`PreferredSizeWidgetRef`].
pub struct PreferredSizeWidgetRefKind;

impl IntoWidget<PreferredSizeWidgetRefKind> for PreferredSizeWidgetRef {
    fn into_widget(self) -> WidgetRef {
        self.widget
    }
}

/// A widget with a preferred size.
///
/// This widget does not impose any constraints on its child, and it doesn't
/// affect the child's layout in any way. It just advertises a preferred size
/// which can be used by the parent.
///
/// Parents like `Scaffold` use [`PreferredSizeWidget`] to require that their
/// children implement that interface. To give a preferred size to an arbitrary
/// widget so that it can be used in a `child` property of that type, this
/// widget, [`PreferredSize`], can be used.
///
/// Widgets like `AppBar` implement a [`PreferredSizeWidget`], so that this
/// [`PreferredSize`] widget is not necessary for them.
///
/// See also:
///
///  * `AppBar.bottom` and `Scaffold.appBar`, which require preferred size widgets.
///  * [`PreferredSizeWidget`], the interface which this widget implements to expose
///    its preferred size.
///  * `AppBar` and `TabBar`, which implement PreferredSizeWidget.
#[derive(Debug)]
pub struct PreferredSize {
    key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    child: WidgetRef,
    preferred_size: Size,
}

impl PreferredSize {
    /// Creates a widget that has a preferred size that the parent can query.
    pub fn new<K>(preferred_size: Size, child: impl IntoWidget<K>) -> PreferredSize {
        PreferredSize {
            key: None,
            child: child.into_widget(),
            preferred_size,
        }
    }

    /// Dart `PreferredSize(key:)`.
    pub fn key(mut self, key: KeyRef) -> PreferredSize {
        self.key = Some(key);
        self
    }
}

impl PreferredSizeWidget for PreferredSize {
    fn preferred_size(&self) -> Size {
        self.preferred_size
    }
}

impl StatelessWidget for PreferredSize {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.child.clone()
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use reveal_rendering::RenderConstrainedBox;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    #[test]
    fn an_erased_preferred_size_widget_still_answers_its_size_and_builds_its_child() {
        let bar = PreferredSizeWidgetRef::new(PreferredSize::new(
            Size::from_height(80.0),
            SizedBox::shrink(),
        ));
        assert_eq!(bar.preferred_size(), Size::from_height(80.0));

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, bar.into_widget());
        harness.pump(&mut app);
        assert!(
            harness
                .render_root(&app)
                .child(&app)
                .expect("the child")
                .as_object()
                .downcast::<RenderConstrainedBox>(&app)
                .is_some()
        );
    }
}
