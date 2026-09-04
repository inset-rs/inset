//! Flutter counterpart: `cupertino/interface_level.dart`.

use reveal_foundation::App;
use reveal_widgets::{BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

/// Indicates the visual level for a piece of content. Equivalent to `UIUserInterfaceLevel`
/// from `UIKit`.
///
/// See also:
///
///  * `UIUserInterfaceLevel`, the UIKit equivalent: <https://developer.apple.com/documentation/uikit/uiuserinterfacelevel>.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CupertinoUserInterfaceLevelData {
    /// The level for your window's main content.
    Base,

    /// The level for content visually above [`Base`](Self::Base).
    Elevated,
}

/// Establishes a subtree in which [`CupertinoUserInterfaceLevel::of`] resolves to
/// the given visual elevation from the [`CupertinoUserInterfaceLevelData`]. This
/// can be used to apply style differences based on a widget's elevation.
///
/// Querying the current elevation status using [`CupertinoUserInterfaceLevel::of`]
/// will cause your widget to rebuild automatically whenever the
/// [`CupertinoUserInterfaceLevelData`] changes.
///
/// If no [`CupertinoUserInterfaceLevel`] is in scope then the
/// [`CupertinoUserInterfaceLevel::of`] method will panic.
/// Alternatively, [`CupertinoUserInterfaceLevel::maybe_of`] can be used, which
/// returns `None` instead of panicking if no [`CupertinoUserInterfaceLevel`] is in
/// scope.
///
/// See also:
///
///  * [`CupertinoUserInterfaceLevelData`], specifies the visual level for the content
///    in the subtree [`CupertinoUserInterfaceLevel`] established.
#[derive(Debug)]
pub struct CupertinoUserInterfaceLevel {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    data: CupertinoUserInterfaceLevelData,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl CupertinoUserInterfaceLevel {
    /// Creates a [`CupertinoUserInterfaceLevel`] to change descendant Cupertino widget's
    /// visual level.
    pub fn new<K>(
        data: CupertinoUserInterfaceLevelData,
        child: impl IntoWidget<K>,
    ) -> CupertinoUserInterfaceLevel {
        CupertinoUserInterfaceLevel {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `CupertinoUserInterfaceLevel(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoUserInterfaceLevel {
        self.key = Some(key);
        self
    }

    /// The data from the closest instance of this class that encloses the given
    /// context.
    ///
    /// You can use this function to query the user interface elevation level within
    /// the given [`BuildContext`]. When that information changes, your widget will
    /// be scheduled to be rebuilt, keeping your widget up-to-date.
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), which is similar, but will return `None` if no
    ///    [`CupertinoUserInterfaceLevel`] encloses the given context.
    pub fn of(app: &mut App, context: BuildContext) -> CupertinoUserInterfaceLevelData {
        let query =
            context.depend_on_inherited_widget_of_exact_type::<CupertinoUserInterfaceLevel>(app);
        if let Some(query) = query {
            return query.data;
        }
        panic!(
            "CupertinoUserInterfaceLevel.of() called with a context that does not contain a CupertinoUserInterfaceLevel.\n\
             No CupertinoUserInterfaceLevel ancestor could be found starting from the context that was passed \
             to CupertinoUserInterfaceLevel.of(). This can happen because you do not have a WidgetsApp or \
             MaterialApp widget (those widgets introduce a CupertinoUserInterfaceLevel), or it can happen \
             if the context you use comes from a widget above those widgets.\n\
             The context used was:\n  {context:?}"
        );
    }

    /// The data from the closest instance of this class that encloses the given
    /// context, if there is one.
    ///
    /// Returns `None` if no [`CupertinoUserInterfaceLevel`] encloses the given context.
    ///
    /// You can use this function to query the user interface elevation level within
    /// the given [`BuildContext`]. When that information changes, your widget will
    /// be scheduled to be rebuilt, keeping your widget up-to-date.
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), which is similar, but will panic if no
    ///    [`CupertinoUserInterfaceLevel`] encloses the given context.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<CupertinoUserInterfaceLevelData> {
        let query =
            context.depend_on_inherited_widget_of_exact_type::<CupertinoUserInterfaceLevel>(app);
        query.map(|query| query.data)
    }
}

impl InheritedWidget for CupertinoUserInterfaceLevel {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &CupertinoUserInterfaceLevel) -> bool {
        old_widget.data != self.data
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_widgets::{Builder, SizedBox};

    use super::*;
    use crate::test_support::{app, build};

    fn read_under<T: 'static>(
        wrap: impl FnOnce(WidgetRef) -> WidgetRef,
        read: impl Fn(&mut App, BuildContext) -> T + 'static,
    ) -> T {
        let mut app = app();
        let seen = Rc::new(RefCell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() = Some(read(app, context));
                SizedBox::shrink().into_widget()
            }
        })
        .into_widget();
        build(&mut app, wrap(probe));
        let read = seen.borrow_mut().take();
        read.expect("the probe built")
    }

    fn level(data: CupertinoUserInterfaceLevelData) -> impl FnOnce(WidgetRef) -> WidgetRef {
        move |child| CupertinoUserInterfaceLevel::new(data, child).into_widget()
    }

    #[test]
    fn of_and_maybe_of_read_the_enclosing_level() {
        let elevated = level(CupertinoUserInterfaceLevelData::Elevated);
        assert_eq!(
            read_under(elevated, CupertinoUserInterfaceLevel::of),
            CupertinoUserInterfaceLevelData::Elevated
        );
        let base = level(CupertinoUserInterfaceLevelData::Base);
        assert_eq!(
            read_under(base, CupertinoUserInterfaceLevel::maybe_of),
            Some(CupertinoUserInterfaceLevelData::Base)
        );
    }

    #[test]
    fn maybe_of_is_none_without_an_ancestor() {
        assert_eq!(
            read_under(|child| child, CupertinoUserInterfaceLevel::maybe_of),
            None
        );
    }

    #[test]
    #[should_panic(
        expected = "CupertinoUserInterfaceLevel.of() called with a context that does not contain a CupertinoUserInterfaceLevel."
    )]
    fn of_panics_without_an_ancestor() {
        read_under(|child| child, CupertinoUserInterfaceLevel::of);
    }

    #[test]
    fn a_changed_level_notifies_and_an_equal_one_does_not() {
        let child = SizedBox::shrink().into_widget();
        let base =
            CupertinoUserInterfaceLevel::new(CupertinoUserInterfaceLevelData::Base, child.clone());
        let elevated = CupertinoUserInterfaceLevel::new(
            CupertinoUserInterfaceLevelData::Elevated,
            child.clone(),
        );
        let base_again =
            CupertinoUserInterfaceLevel::new(CupertinoUserInterfaceLevelData::Base, child);
        assert!(elevated.update_should_notify(&base));
        assert!(!base_again.update_should_notify(&base));
    }
}
