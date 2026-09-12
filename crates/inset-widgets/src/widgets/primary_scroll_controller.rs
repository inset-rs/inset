//! Flutter counterpart: `widgets/primary_scroll_controller.dart`.

use std::collections::HashSet;

use inset_embedder::TargetPlatform;
use inset_foundation::App;
use inset_painting::Axis;

use crate::framework::{BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef};
use crate::widgets::scroll_configuration::ScrollConfiguration;
use crate::widgets::scroll_controller::AnyScrollController;

/// Dart's `_kMobilePlatforms`, a function because a `HashSet` has no `const` constructor.
fn k_mobile_platforms() -> HashSet<TargetPlatform> {
    HashSet::from([
        TargetPlatform::Android,
        TargetPlatform::IOS,
        TargetPlatform::Fuchsia,
    ])
}

/// Associates a `ScrollController` with a subtree.
///
/// When a `ScrollView` has `primary` set to true, the `ScrollView`
/// uses [`of`](Self::of) to inherit the [`PrimaryScrollController`] associated with its
/// subtree.
///
/// A scroll view that doesn't have a controller or the primary flag set will
/// inherit the primary scroll controller, if [`should_inherit`](Self::should_inherit) allows
/// it. By default [`should_inherit`](Self::should_inherit) is true for mobile platforms when
/// the scroll view has a scroll direction of [`Axis::Vertical`]. This automatic inheritance
/// can be configured with
/// [`automatically_inherit_for_platforms`](Self::automatically_inherit_for_platforms) and
/// [`scroll_direction`](Self::scroll_direction).
///
/// Inheriting this scroll controller can provide default behavior for scroll
/// views in a subtree. For example, the `Scaffold` uses this mechanism to
/// implement the scroll-to-top gesture on iOS.
///
/// Another default behavior handled by the [`PrimaryScrollController`] is default
/// [`crate::ScrollAction`]s. If a scroll action is not handled by an otherwise focused
/// part of the application, the scroll action will be evaluated using the scroll
/// view associated with a [`PrimaryScrollController`], for example, when executing
/// `Shortcuts` key events like page up and down.
///
/// See also:
///   * [`crate::ScrollAction`], an [`crate::Action`] that scrolls the `Scrollable` that
///     encloses the current primary focus or is attached to the primary scroll controller.
///   * `Shortcuts`, a widget that establishes a `ShortcutManager` to be used
///     by its descendants when invoking an [`crate::Action`] via a keyboard key
///     combination that maps to an [`crate::Intent`].
#[derive(Debug)]
pub struct PrimaryScrollController {
    pub key: Option<KeyRef>,

    /// The `ScrollController` associated with the subtree.
    ///
    /// See also:
    ///
    ///  * `ScrollView.controller`, which discusses the purpose of specifying a
    ///    scroll controller.
    pub controller: Option<AnyScrollController>,

    /// The [`Axis`] this controller is configured for scroll views to
    /// automatically inherit.
    ///
    /// Used in conjunction with
    /// [`automatically_inherit_for_platforms`](Self::automatically_inherit_for_platforms). If
    /// the current [`TargetPlatform`] is not included there, this is ignored.
    ///
    /// When `None`, no scroll view in any axis will automatically inherit this
    /// controller. This is dissimilar to [`PrimaryScrollController::none`]. When a
    /// primary scroll controller is inherited, `ScrollView` will insert
    /// [`PrimaryScrollController::none`] into the tree to prevent further descendant
    /// scroll views from inheriting the current primary scroll controller.
    ///
    /// For the direction in which active scrolling may be occurring, see
    /// [`inset_rendering::ScrollDirection`].
    ///
    /// Defaults to [`Axis::Vertical`].
    pub scroll_direction: Option<Axis>,

    /// The [`TargetPlatform`]s this controller is configured for scroll views to
    /// automatically inherit.
    ///
    /// Used in conjunction with [`scroll_direction`](Self::scroll_direction). If the [`Axis`]
    /// provided to [`should_inherit`](Self::should_inherit) is not
    /// [`scroll_direction`](Self::scroll_direction), this is ignored.
    ///
    /// When empty, no scroll view in any axis will automatically inherit this
    /// controller. Defaults to the mobile platforms.
    pub automatically_inherit_for_platforms: HashSet<TargetPlatform>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl PrimaryScrollController {
    /// Creates a widget that associates a `ScrollController` with a subtree.
    pub fn new<K>(
        controller: AnyScrollController,
        child: impl IntoWidget<K>,
    ) -> PrimaryScrollController {
        PrimaryScrollController {
            key: None,
            controller: Some(controller),
            scroll_direction: Some(Axis::Vertical),
            automatically_inherit_for_platforms: k_mobile_platforms(),
            child: child.into_widget(),
        }
    }

    /// Creates a subtree without an associated `ScrollController`.
    pub fn none<K>(child: impl IntoWidget<K>) -> PrimaryScrollController {
        PrimaryScrollController {
            key: None,
            controller: None,
            scroll_direction: None,
            automatically_inherit_for_platforms: HashSet::new(),
            child: child.into_widget(),
        }
    }

    /// Dart `PrimaryScrollController(key:)`.
    pub fn key(mut self, key: KeyRef) -> PrimaryScrollController {
        self.key = Some(key);
        self
    }

    /// Dart `PrimaryScrollController(scrollDirection:)`.
    pub fn scroll_direction(mut self, scroll_direction: Axis) -> PrimaryScrollController {
        self.scroll_direction = Some(scroll_direction);
        self
    }

    /// Dart `PrimaryScrollController(automaticallyInheritForPlatforms:)`.
    pub fn automatically_inherit_for_platforms(
        mut self,
        automatically_inherit_for_platforms: HashSet<TargetPlatform>,
    ) -> PrimaryScrollController {
        self.automatically_inherit_for_platforms = automatically_inherit_for_platforms;
        self
    }

    /// Returns true if this [`PrimaryScrollController`] is configured to be
    /// automatically inherited for the current [`TargetPlatform`] and the given
    /// [`Axis`].
    ///
    /// This method is typically not called directly. `ScrollView` will call this
    /// method if it has not been provided a `ScrollController` and
    /// `ScrollView.primary` is unset.
    pub fn should_inherit(app: &mut App, context: BuildContext, scroll_direction: Axis) -> bool {
        let Some(result) =
            context.find_ancestor_widget_of_exact_type::<PrimaryScrollController>(app)
        else {
            return false;
        };
        let inherit_for_platforms = result.automatically_inherit_for_platforms.clone();
        let inherited_direction = result.scroll_direction;

        let platform = ScrollConfiguration::of(app, context).get_platform(app, context);
        if inherit_for_platforms.contains(&platform) {
            return inherited_direction == Some(scroll_direction);
        }
        false
    }

    /// Returns the `ScrollController` most closely associated with the given
    /// context.
    ///
    /// Returns `None` if there is no `ScrollController` associated with the given
    /// context.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`PrimaryScrollController`] in the `context`, if there is one.
    ///
    /// See also:
    ///
    /// * [`PrimaryScrollController::of`], which is similar to this method, but
    ///   panics if no [`PrimaryScrollController`] ancestor is found.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<AnyScrollController> {
        context
            .depend_on_inherited_widget_of_exact_type::<PrimaryScrollController>(app)
            .and_then(|result| result.controller)
    }

    /// Returns the `ScrollController` most closely associated with the given
    /// context.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`PrimaryScrollController`] in the `context`.
    ///
    /// # Panics
    ///
    /// If no [`PrimaryScrollController`] ancestor is found.
    ///
    /// See also:
    ///
    /// * [`PrimaryScrollController::maybe_of`], which is similar to this method, but
    ///   returns `None` if no [`PrimaryScrollController`] ancestor is found.
    pub fn of(app: &mut App, context: BuildContext) -> AnyScrollController {
        PrimaryScrollController::maybe_of(app, context).expect(
            "PrimaryScrollController.of() was called with a context that does not contain a \
             PrimaryScrollController widget.\n\
             No PrimaryScrollController widget ancestor could be found starting from the \
             context that was passed to PrimaryScrollController.of(). This can happen because \
             you are using a widget that looks for a PrimaryScrollController ancestor, but no \
             such ancestor exists.",
        )
    }
}

impl InheritedWidget for PrimaryScrollController {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &PrimaryScrollController) -> bool {
        self.controller != old_widget.controller
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::{AnyElement, Element, IntoWidget};
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;
    use crate::widgets::scroll_controller::{ScrollController, ScrollControllerLeaf};
    use inset_foundation::AppCell;

    /// The element under the test root at `depth` levels down.
    fn descendant(harness: &Harness, app: &App, depth: usize) -> AnyElement {
        let mut element = harness.root.as_element();
        for _ in 0..=depth {
            element = element.children(app)[0];
        }
        element
    }

    fn mount(app: &mut App, widget: crate::framework::WidgetRef) -> Harness {
        let harness = Harness::mount(app, widget);
        harness.pump(app);
        harness
    }

    #[test]
    fn of_finds_the_controller_the_subtree_is_associated_with() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        let harness = mount(
            &mut app,
            PrimaryScrollController::new(controller.as_controller(), SizedBox::shrink())
                .into_widget(),
        );

        let inner = descendant(&harness, &app, 1);
        assert_eq!(
            PrimaryScrollController::of(&mut app, inner),
            controller.as_controller()
        );
        assert_eq!(
            PrimaryScrollController::maybe_of(&mut app, inner),
            Some(controller.as_controller())
        );
    }

    #[test]
    fn none_hides_the_controller_from_its_subtree() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        let harness = mount(
            &mut app,
            PrimaryScrollController::new(
                controller.as_controller(),
                PrimaryScrollController::none(SizedBox::shrink()),
            )
            .into_widget(),
        );

        let inner = descendant(&harness, &app, 2);
        assert_eq!(PrimaryScrollController::maybe_of(&mut app, inner), None);
    }

    #[test]
    fn should_inherit_follows_the_platform_and_the_axis() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        let harness = mount(
            &mut app,
            PrimaryScrollController::new(controller.as_controller(), SizedBox::shrink())
                .into_widget(),
        );

        let inner = descendant(&harness, &app, 1);
        // `App::new` runs on the host's platform; the default set covers the mobile ones.
        let mobile = k_mobile_platforms().contains(&app.platform().target_platform());
        assert_eq!(
            PrimaryScrollController::should_inherit(&mut app, inner, Axis::Vertical),
            mobile
        );
        assert!(!PrimaryScrollController::should_inherit(
            &mut app,
            inner,
            Axis::Horizontal
        ));
    }
}
