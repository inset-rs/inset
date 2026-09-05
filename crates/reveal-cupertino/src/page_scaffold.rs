//! Flutter counterpart: `cupertino/page_scaffold.dart`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curves;
use reveal_embedder::{Offset, Size};
use reveal_foundation::{App, Handle};
use reveal_gestures::{GestureBinding, HitTestResult};
use reveal_painting::{AnyColor, BoxDecoration, EdgeInsetsGeometry};
use reveal_rendering::{BoxHitTestEntry, HitTestBehavior};
use reveal_widgets::{
    BuildContext, DecoratedBox, GlobalKey, InheritedWidget, IntoWidget, KeyRef, MediaQuery,
    MetaData, Padding, Positioned, PreferredSizeWidget, ScrollNotificationObserver, SizedBox,
    Stack, State, StateData, StatefulWidget, StatelessWidget, View, WidgetRef, WidgetsBinding,
    WidgetsBindingObserverObject, WidgetsBindingObserverRef, downcast_widget,
};

use crate::colors::CupertinoDynamicColor;
use crate::theme::CupertinoTheme;

/// A widget that has a preferred size and reports whether it fully obstructs
/// widgets behind it.
///
/// Used by [`CupertinoPageScaffold`] to either shift away fully obstructed content
/// or provide a padding guide to partially obstructed content.
pub trait ObstructingPreferredSizeWidget: PreferredSizeWidget {
    /// If true, this widget fully obstructs widgets behind it by the specified
    /// size.
    ///
    /// If false, this widget partially obstructs.
    fn should_fully_obstruct(&self, app: &mut App, context: BuildContext) -> bool;
}

/// An [`ObstructingPreferredSizeWidget`] held as a value: the erased widget, whose preferred
/// size and obstruction are still answered by the widget it holds. Dart types such a field
/// `ObstructingPreferredSizeWidget`.
#[derive(Clone)]
pub struct ObstructingPreferredSizeWidgetRef {
    widget: WidgetRef,
    preferred_size: fn(&WidgetRef) -> Size,
    should_fully_obstruct: fn(&WidgetRef, &mut App, BuildContext) -> bool,
}

impl ObstructingPreferredSizeWidgetRef {
    /// Erases an [`ObstructingPreferredSizeWidget`].
    pub fn new<K, W: ObstructingPreferredSizeWidget + IntoWidget<K>>(
        widget: W,
    ) -> ObstructingPreferredSizeWidgetRef {
        ObstructingPreferredSizeWidgetRef {
            widget: widget.into_widget(),
            preferred_size: |widget| {
                downcast_widget::<W>(&**widget)
                    .expect("an erased widget keeps its type")
                    .preferred_size()
            },
            should_fully_obstruct: |widget, app, context| {
                downcast_widget::<W>(&**widget)
                    .expect("an erased widget keeps its type")
                    .should_fully_obstruct(app, context)
            },
        }
    }

    /// See [`PreferredSizeWidget::preferred_size`].
    pub fn preferred_size(&self) -> Size {
        (self.preferred_size)(&self.widget)
    }

    /// See [`ObstructingPreferredSizeWidget::should_fully_obstruct`].
    pub fn should_fully_obstruct(&self, app: &mut App, context: BuildContext) -> bool {
        (self.should_fully_obstruct)(&self.widget, app, context)
    }

    /// The widget itself.
    pub fn widget(&self) -> &WidgetRef {
        &self.widget
    }
}

impl Debug for ObstructingPreferredSizeWidgetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.widget, f)
    }
}

/// The kind tag of `IntoWidget` for an [`ObstructingPreferredSizeWidgetRef`].
pub struct ObstructingPreferredSizeWidgetRefKind;

impl IntoWidget<ObstructingPreferredSizeWidgetRefKind> for ObstructingPreferredSizeWidgetRef {
    fn into_widget(self) -> WidgetRef {
        self.widget
    }
}

/// Implements a single iOS application page's layout.
///
/// The scaffold lays out the navigation bar on top and the content between or
/// behind the navigation bar.
///
/// When tapping a status bar at the top of the [`CupertinoPageScaffold`], an
/// animation will complete for the current primary `ScrollView`, scrolling to
/// the beginning. This is done using the `PrimaryScrollController` that
/// encloses the `ScrollView`.
#[derive(Debug)]
pub struct CupertinoPageScaffold {
    key: Option<KeyRef>,

    /// The navigation bar, typically a `CupertinoNavigationBar`, is drawn at the
    /// top of the screen.
    ///
    /// If translucent, the main content may slide behind it. Otherwise, the main
    /// content's top margin will be offset by its height.
    ///
    /// The scaffold assumes the navigation bar will account for the `MediaQuery`
    /// top padding, and also consume it if the navigation bar is opaque.
    ///
    /// By default the navigation bar disables text scaling to match the native iOS
    /// behavior. To override such behavior, wrap each of its components inside a
    /// `MediaQuery` with the desired `TextScaler`.
    pub navigation_bar: Option<ObstructingPreferredSizeWidgetRef>,

    /// Widget to show in the main content area.
    ///
    /// Content can slide under the [`navigation_bar`](Self::navigation_bar) when it is
    /// translucent. In that case, the child's `BuildContext`'s `MediaQuery` will have a top
    /// padding indicating the area of obstructing overlap from the navigation bar.
    pub child: WidgetRef,

    /// The color of the widget that underlies the entire scaffold.
    ///
    /// By default uses the `CupertinoTheme`'s scaffold background color when `None`.
    pub background_color: Option<AnyColor>,

    /// Whether the [`child`](Self::child) should size itself to avoid the window's bottom
    /// inset.
    ///
    /// For example, if there is an onscreen keyboard displayed above the
    /// scaffold, the body can be resized to avoid overlapping the keyboard, which
    /// prevents widgets inside the body from being obscured by the keyboard.
    ///
    /// Defaults to true.
    pub resize_to_avoid_bottom_inset: bool,
}

impl CupertinoPageScaffold {
    /// Creates a layout for pages with a navigation bar at the top.
    pub fn new<K>(child: impl IntoWidget<K>) -> CupertinoPageScaffold {
        CupertinoPageScaffold {
            key: None,
            navigation_bar: None,
            child: child.into_widget(),
            background_color: None,
            resize_to_avoid_bottom_inset: true,
        }
    }

    /// Dart `CupertinoPageScaffold(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoPageScaffold {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoPageScaffold(navigationBar:)`.
    pub fn navigation_bar(
        mut self,
        navigation_bar: ObstructingPreferredSizeWidgetRef,
    ) -> CupertinoPageScaffold {
        self.navigation_bar = Some(navigation_bar);
        self
    }

    /// Dart `CupertinoPageScaffold(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: impl Into<AnyColor>,
    ) -> CupertinoPageScaffold {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `CupertinoPageScaffold(resizeToAvoidBottomInset:)`.
    pub fn resize_to_avoid_bottom_inset(
        mut self,
        resize_to_avoid_bottom_inset: bool,
    ) -> CupertinoPageScaffold {
        self.resize_to_avoid_bottom_inset = resize_to_avoid_bottom_inset;
        self
    }
}

impl StatefulWidget for CupertinoPageScaffold {
    type State = CupertinoPageScaffoldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoPageScaffoldState {
        CupertinoPageScaffoldState {
            state: StateData::new(),
            status_bar_key: Rc::new(GlobalKey::new()),
            observer: None,
        }
    }
}

/// Dart's `_CupertinoPageScaffoldState`.
pub struct CupertinoPageScaffoldState {
    state: StateData<CupertinoPageScaffold>,
    status_bar_key: Rc<GlobalKey>,
    observer: Option<WidgetsBindingObserverRef>,
}

impl CupertinoPageScaffoldState {
    fn observer(self: Handle<Self>, app: &mut App) -> WidgetsBindingObserverRef {
        if let Some(observer) = app.get(self).observer.clone() {
            return observer;
        }
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        app.get_mut(self).observer = Some(observer.clone());
        observer
    }

    /// Called when the system tells the app to scroll the primary scroll view to the top;
    /// Dart's `WidgetsBindingObserver.handleStatusBarTap` override.
    pub fn handle_status_bar_tap(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let Some(primary_scroll_controller) =
            reveal_widgets::PrimaryScrollController::maybe_of(app, context)
        else {
            return;
        };
        // The iOS embedder used to send status bar tap events as fake touches at
        // `Offset::ZERO`, such that at most one scaffold (usually the foreground one) can
        // handle the status bar tap event, thanks to hit-testing and gesture disambiguation.
        // To keep that behavior, this widget performs an additional hit-test here to make
        // sure the status bar tap is only handled if this scaffold is hit-testable (thus in
        // the foreground).
        let status_bar_key = app.get(self).status_bar_key.clone();
        if primary_scroll_controller.has_clients(app)
            && hit_testable_at_origin(app, &status_bar_key)
        {
            primary_scroll_controller.animate_to(
                app,
                0.0,
                // Eyeballed from iOS.
                Duration::from_millis(500),
                Curves::linear_to_ease_out(),
            );
        }
    }
}

impl WidgetsBindingObserverObject for CupertinoPageScaffoldState {}

impl State for CupertinoPageScaffoldState {
    type Widget = CupertinoPageScaffold;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let observer = self.observer(app);
        WidgetsBinding::instance(app).add_observer(app, observer);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        let observer = self.observer(app);
        WidgetsBinding::instance(app).remove_observer(app, &observer);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        let observer = self.observer(app);
        WidgetsBinding::instance(app).add_observer(app, observer);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut padded_content = self.widget(app).child.clone();

        let widget_background_color = self.widget(app).background_color.clone();
        let background_color =
            CupertinoDynamicColor::maybe_resolve(widget_background_color.as_ref(), app, context)
                .unwrap_or_else(|| CupertinoTheme::of(app, context).scaffold_background_color());

        let existing_media_query = MediaQuery::of(app, context);
        let navigation_bar = self.widget(app).navigation_bar.clone();
        let resize_to_avoid_bottom_inset = self.widget(app).resize_to_avoid_bottom_inset;
        if let Some(navigation_bar) = navigation_bar.clone() {
            let top_padding =
                navigation_bar.preferred_size().height() + existing_media_query.padding.top;

            // Propagate bottom padding and include the view insets if appropriate.
            let bottom_padding = if resize_to_avoid_bottom_inset {
                existing_media_query.view_insets.bottom
            } else {
                0.0
            };

            let new_view_insets = if resize_to_avoid_bottom_inset {
                // The insets are consumed by the scaffolds and no longer exposed to the
                // descendant subtree.
                existing_media_query
                    .view_insets
                    .copy_with(None, None, None, Some(0.0))
            } else {
                existing_media_query.view_insets
            };

            let full_obstruction = navigation_bar.should_fully_obstruct(app, context);

            // If the navigation bar is opaquely obstructing, directly shift the main content
            // down. If translucent, let the main content draw behind the navigation bar but
            // hint the obstructed area.
            padded_content = if full_obstruction {
                MediaQuery::new(
                    // If the navigation bar is opaque, the top media query padding is fully
                    // consumed by the navigation bar.
                    existing_media_query
                        .remove_padding(false, true, false, false)
                        .copy_with()
                        .view_insets(new_view_insets),
                    Padding::new(EdgeInsetsGeometry::only(
                        0.0,
                        top_padding,
                        0.0,
                        bottom_padding,
                    ))
                    .child(padded_content),
                )
                .into_widget()
            } else {
                MediaQuery::new(
                    existing_media_query
                        .copy_with()
                        .padding(existing_media_query.padding.copy_with(
                            None,
                            Some(top_padding),
                            None,
                            None,
                        ))
                        .view_insets(new_view_insets),
                    Padding::new(EdgeInsetsGeometry::only(0.0, 0.0, 0.0, bottom_padding))
                        .child(padded_content),
                )
                .into_widget()
            };
        } else if resize_to_avoid_bottom_inset {
            // If there is no navigation bar, we still may need to add padding in order to
            // support `resize_to_avoid_bottom_inset`.
            padded_content = MediaQuery::new(
                existing_media_query.copy_with().view_insets(
                    existing_media_query
                        .view_insets
                        .copy_with(None, None, None, Some(0.0)),
                ),
                Padding::new(EdgeInsetsGeometry::only(
                    0.0,
                    0.0,
                    0.0,
                    existing_media_query.view_insets.bottom,
                ))
                .child(padded_content),
            )
            .into_widget();
        }

        let status_bar_key = app.get(self).status_bar_key.clone();
        let mut children = vec![
            // The main content being at the bottom is added to the stack first.
            padded_content,
        ];
        if let Some(navigation_bar) = navigation_bar {
            children.push(
                Positioned::new(MediaQuery::with_no_text_scaling(None, navigation_bar))
                    .top(0.0)
                    .left(0.0)
                    .right(0.0)
                    .into_widget(),
            );
        }
        // Add a touch handler the size of the status bar on top of all contents to handle
        // scroll to top by status bar taps.
        children.push(
            Positioned::new(HitTestableAtOrigin::new(status_bar_key))
                .top(0.0)
                .left(0.0)
                .right(0.0)
                .height(existing_media_query.padding.top)
                .into_widget(),
        );

        ScrollNotificationObserver::new(
            DecoratedBox::new(BoxDecoration::new().color(background_color.clone())).child(
                CupertinoPageScaffoldBackgroundColor::new(
                    background_color,
                    Stack::new().children(children),
                ),
            ),
        )
        .into_widget()
    }
}

impl Debug for CupertinoPageScaffoldState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CupertinoPageScaffoldState")
    }
}

/// An `InheritedWidget` indicating what the current scaffold background color is for its
/// children.
///
/// This is used by the `CupertinoNavigationBar` and the `CupertinoSliverNavigationBar`
/// widgets to paint themselves with the parent page scaffold color when no content is
/// scrolled under.
#[derive(Debug)]
pub struct CupertinoPageScaffoldBackgroundColor {
    key: Option<KeyRef>,

    /// The background color defined in [`CupertinoPageScaffold`].
    pub color: AnyColor,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl CupertinoPageScaffoldBackgroundColor {
    /// Constructs a new [`CupertinoPageScaffoldBackgroundColor`].
    pub fn new<K>(
        color: AnyColor,
        child: impl IntoWidget<K>,
    ) -> CupertinoPageScaffoldBackgroundColor {
        CupertinoPageScaffoldBackgroundColor {
            key: None,
            color,
            child: child.into_widget(),
        }
    }

    /// Dart `CupertinoPageScaffoldBackgroundColor(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoPageScaffoldBackgroundColor {
        self.key = Some(key);
        self
    }

    /// Retrieve the [`CupertinoPageScaffold`] background color from the context.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<AnyColor> {
        context
            .depend_on_inherited_widget_of_exact_type::<CupertinoPageScaffoldBackgroundColor>(app)
            .map(|scaffold_background_color| scaffold_background_color.color.clone())
    }
}

impl InheritedWidget for CupertinoPageScaffoldBackgroundColor {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &CupertinoPageScaffoldBackgroundColor) -> bool {
        self.color != old_widget.color
    }
}

/// Whether the render box of the [`HitTestableAtOrigin`] widget associated with the given
/// global `key` is hit-testable at `Offset::ZERO`.
///
/// This is used by [`CupertinoPageScaffoldState::handle_status_bar_tap`] to avoid sending
/// status bar tap events to scroll views in offscreen subtrees.
fn hit_testable_at_origin(app: &mut App, key: &GlobalKey) -> bool {
    let Some(context) = key.current_context(app) else {
        debug_assert!(
            false,
            "the BuildContext associated with the key is not mounted."
        );
        return false;
    };
    let render_object = context
        .find_render_object(app)
        .expect("a mounted MetaData has a render object")
        .as_box()
        .expect("MetaData creates a RenderMetaData");
    let view_id = View::of(app, context).id();
    let mut result = HitTestResult::new();
    GestureBinding::instance(app).hit_test_in_view(app, &mut result, Offset::ZERO, view_id);
    result.path().iter().any(|entry| {
        (entry.target() as &dyn Any)
            .downcast_ref::<BoxHitTestEntry>()
            .is_some_and(|entry| entry.target() == render_object)
    })
}

/// Dart's `_HitTestableAtOrigin`: the status-bar-sized touch target the scaffold stacks over
/// its contents.
#[derive(Debug)]
struct HitTestableAtOrigin {
    global_key: Rc<GlobalKey>,
}

impl HitTestableAtOrigin {
    fn new(global_key: Rc<GlobalKey>) -> HitTestableAtOrigin {
        HitTestableAtOrigin { global_key }
    }
}

impl StatelessWidget for HitTestableAtOrigin {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        MetaData::new()
            .key(self.global_key.clone())
            .behavior(HitTestBehavior::Translucent)
            .child(SizedBox::expand())
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::TextDirection;
    use reveal_painting::EdgeInsets;
    use reveal_rendering::{AnyRenderObject, RenderPadding};
    use reveal_widgets::{Directionality, MediaQueryData};

    use super::*;
    use crate::test_support::{app as test_app, build};

    const BAR_HEIGHT: f64 = 44.0;

    /// A navigation bar of a fixed height that reports whether it obstructs.
    #[derive(Debug)]
    struct TestNavigationBar {
        opaque: bool,
    }

    impl PreferredSizeWidget for TestNavigationBar {
        fn preferred_size(&self) -> Size {
            Size::from_height(BAR_HEIGHT)
        }
    }

    impl ObstructingPreferredSizeWidget for TestNavigationBar {
        fn should_fully_obstruct(&self, _app: &mut App, _context: BuildContext) -> bool {
            self.opaque
        }
    }

    impl reveal_widgets::StatelessWidget for TestNavigationBar {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::new().height(BAR_HEIGHT).into_widget()
        }
    }

    fn paddings(app: &App, object: AnyRenderObject) -> Vec<EdgeInsets> {
        let mut found = Vec::new();
        if let Some(padding) = object.downcast::<RenderPadding>(app) {
            found.push(padding.padding(app).resolve(Some(TextDirection::Ltr)));
        }
        object.visit_children(app, &mut |child| {
            found.extend(paddings(app, child));
        });
        found
    }

    fn mount(
        app: &mut App,
        data: MediaQueryData,
        scaffold: CupertinoPageScaffold,
    ) -> Vec<EdgeInsets> {
        let tree: WidgetRef = MediaQuery::new(data, scaffold).into_widget();
        build(
            app,
            Directionality::new(TextDirection::Ltr, tree).into_widget(),
        );
        let root = reveal_widgets::WidgetsBinding::instance(app)
            .root_element(app)
            .expect("a mounted root element")
            .find_render_object(app)
            .expect("a mounted view has a render object");
        paddings(app, root)
    }

    #[test]
    fn an_opaque_navigation_bar_shifts_the_content_down_by_its_preferred_height() {
        let mut app = test_app();
        let scaffold = CupertinoPageScaffold::new(SizedBox::expand()).navigation_bar(
            ObstructingPreferredSizeWidgetRef::new(TestNavigationBar { opaque: true }),
        );
        let paddings = mount(
            &mut app,
            MediaQueryData::new().padding(EdgeInsets::from_ltrb(0.0, 20.0, 0.0, 0.0)),
            scaffold,
        );
        assert!(
            paddings.contains(&EdgeInsets::from_ltrb(0.0, BAR_HEIGHT + 20.0, 0.0, 0.0)),
            "the content is inset by the bar's height plus the top padding: {paddings:?}"
        );
    }

    #[test]
    fn a_translucent_navigation_bar_leaves_the_content_in_place_and_hints_the_padding() {
        let mut app = test_app();
        let scaffold = CupertinoPageScaffold::new(SizedBox::expand()).navigation_bar(
            ObstructingPreferredSizeWidgetRef::new(TestNavigationBar { opaque: false }),
        );
        let paddings = mount(
            &mut app,
            MediaQueryData::new().padding(EdgeInsets::from_ltrb(0.0, 20.0, 0.0, 0.0)),
            scaffold,
        );
        assert!(
            paddings.contains(&EdgeInsets::ZERO),
            "a translucent bar does not shift the content: {paddings:?}"
        );
    }

    #[test]
    fn resize_to_avoid_bottom_inset_pads_the_content_by_the_bottom_view_inset() {
        let mut app = test_app();
        let scaffold = CupertinoPageScaffold::new(SizedBox::expand());
        let paddings = mount(
            &mut app,
            MediaQueryData::new().view_insets(EdgeInsets::from_ltrb(0.0, 0.0, 0.0, 120.0)),
            scaffold,
        );
        assert!(
            paddings.contains(&EdgeInsets::from_ltrb(0.0, 0.0, 0.0, 120.0)),
            "the keyboard inset becomes bottom padding: {paddings:?}"
        );

        let mut without = test_app();
        let scaffold =
            CupertinoPageScaffold::new(SizedBox::expand()).resize_to_avoid_bottom_inset(false);
        let paddings = mount(
            &mut without,
            MediaQueryData::new().view_insets(EdgeInsets::from_ltrb(0.0, 0.0, 0.0, 120.0)),
            scaffold,
        );
        assert!(
            paddings.is_empty(),
            "without resizeToAvoidBottomInset the content is not padded: {paddings:?}"
        );
    }
}
