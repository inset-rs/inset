//! Flutter `widgets/preferred_size.dart` (+ the `Size` subclass mechanism a preferred size
//! travels in).

use std::any::Any;
use std::fmt::{self, Debug};
use std::ops::Deref;
use std::rc::Rc;

use waft_embedder::Size;
use waft_foundation::App;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef, downcast_widget,
};

/// What a subclass of `Size` adds to its value.
///
/// Flutter's `_PreferredAppBarSize extends Size` travels wherever a preferred size is
/// stored, to be recognised later by an `is` check (`AppBar.preferredHeightFor`). Here the
/// engine's [`Size`] is a plain value; a subclass is an extension that an [`AnySize`] carries
/// beside the value the subclass constructor passed to `super`, as `AnyColor` carries a
/// `Color` subclass.
///
/// Unlike `ColorExtension`, there is no `==` to override: dart:ui's `Size.==` compares the
/// values only, without the runtime type, and no framework subclass overrides it.
pub trait SizeExtension: Any {
    /// The extension as `Any`, for Dart's `is` checks ([`AnySize::extension`]).
    fn as_any(&self) -> &dyn Any;
}

/// A `Size` that may be an instance of a `Size` subclass.
///
/// The framework stores this where Flutter stores a `Size` that a later reader type-checks:
/// [`PreferredSizeWidget::preferred_size`] and [`PreferredSize`]'s field. The plain value is
/// the one the subclass passed to `super`, reached by [`size`](Self::size) or by deref; the
/// subclass is recovered with [`extension`](Self::extension), Dart's `is` check.
///
/// Equality follows dart:ui's `Size.==`: the values alone, so a subclass instance equals a
/// plain size of the same width and height.
#[derive(Clone)]
pub struct AnySize {
    size: Size,
    extension: Option<Rc<dyn SizeExtension>>,
}

impl AnySize {
    /// A plain size.
    pub const fn new(size: Size) -> AnySize {
        AnySize {
            size,
            extension: None,
        }
    }

    /// A subclass instance: `size` is what it passed to `super`.
    pub fn with_shared(size: Size, extension: Rc<dyn SizeExtension>) -> AnySize {
        AnySize {
            size,
            extension: Some(extension),
        }
    }

    /// The size as the engine sees it: the subclass's `super` value.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Dart's `size is T`: the subclass instance, if this size is one.
    pub fn extension<T: SizeExtension>(&self) -> Option<&T> {
        self.extension
            .as_ref()
            .and_then(|extension| extension.as_any().downcast_ref::<T>())
    }

    /// Whether this size is a subclass instance.
    pub fn has_extension(&self) -> bool {
        self.extension.is_some()
    }
}

impl Deref for AnySize {
    type Target = Size;

    fn deref(&self) -> &Size {
        &self.size
    }
}

impl From<Size> for AnySize {
    fn from(size: Size) -> AnySize {
        AnySize::new(size)
    }
}

impl PartialEq for AnySize {
    fn eq(&self, other: &AnySize) -> bool {
        self.size == other.size
    }
}

impl PartialEq<Size> for AnySize {
    fn eq(&self, other: &Size) -> bool {
        self.size == *other
    }
}

/// A subclass prints as its `Size`: no framework subclass overrides `toString`.
impl Debug for AnySize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.size, f)
    }
}

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
    /// `Size::from_height(my_app_bar_height).into()`.
    fn preferred_size(&self) -> AnySize;
}

/// A [`PreferredSizeWidget`] held as a value: the erased widget, whose preferred size is
/// still answered by the widget it holds. Dart types such a field `PreferredSizeWidget`.
#[derive(Clone)]
pub struct PreferredSizeWidgetRef {
    widget: WidgetRef,
    preferred_size: fn(&WidgetRef) -> AnySize,
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
    pub fn preferred_size(&self) -> AnySize {
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
    preferred_size: AnySize,
}

impl PreferredSize {
    /// Creates a widget that has a preferred size that the parent can query.
    pub fn new<K>(preferred_size: impl Into<AnySize>, child: impl IntoWidget<K>) -> PreferredSize {
        PreferredSize {
            key: None,
            child: child.into_widget(),
            preferred_size: preferred_size.into(),
        }
    }

    /// Dart `PreferredSize(key:)`.
    pub fn key(mut self, key: KeyRef) -> PreferredSize {
        self.key = Some(key);
        self
    }
}

impl PreferredSizeWidget for PreferredSize {
    fn preferred_size(&self) -> AnySize {
        self.preferred_size.clone()
    }
}

impl StatelessWidget for PreferredSize {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn as_preferred_size_widget(&self) -> Option<&dyn PreferredSizeWidget> {
        Some(self)
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.child.clone()
    }
}

#[cfg(test)]
mod tests {
    use waft_foundation::AppCell;
    use waft_rendering::RenderConstrainedBox;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    #[derive(Debug, PartialEq)]
    struct TaggedSize {
        tag: u32,
    }

    impl SizeExtension for TaggedSize {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn a_preferred_size_keeps_the_subclass_it_was_given() {
        let tagged = AnySize::with_shared(Size::from_height(56.0), Rc::new(TaggedSize { tag: 7 }));
        let bar = PreferredSizeWidgetRef::new(PreferredSize::new(tagged, SizedBox::shrink()));
        assert_eq!(
            bar.preferred_size().extension::<TaggedSize>(),
            Some(&TaggedSize { tag: 7 })
        );
        assert!(
            PreferredSize::new(Size::from_height(56.0), SizedBox::shrink())
                .preferred_size()
                .extension::<TaggedSize>()
                .is_none()
        );
    }

    #[test]
    fn a_subclass_size_equals_a_plain_size_of_the_same_value_as_dart_ui_compares_sizes() {
        let tagged = AnySize::with_shared(Size::from_height(56.0), Rc::new(TaggedSize { tag: 7 }));
        assert_eq!(tagged, AnySize::new(Size::from_height(56.0)));
        assert_eq!(tagged, Size::from_height(56.0));
        assert_ne!(tagged, Size::from_height(64.0));
    }

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
