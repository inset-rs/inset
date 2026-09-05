//! Flutter counterpart: `cupertino/expansion_tile.dart`.
//!
//! The header's `Semantics` hints wait with accessibility.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{Animatable, AnyAnimation, Curve, CurveTween, Curves, Tween};
use reveal_embedder::{FontWeight, Offset};
use reveal_foundation::{App, Handle, Listener};
use reveal_rendering::{BoxConstraints, MainAxisSize};
use reveal_widgets::{
    BuildContext, Center, Column, ConstrainedBox, Expansible, ExpansibleController, FadeTransition,
    GlobalKey, Icon, IntoWidget, KeyRef, LayoutBuilder, Opacity, Overlay, OverlayPortal,
    OverlayPortalController, Positioned, RotationTransition, SizedBox, State, StateData,
    StatefulWidget, Visibility, WidgetRef,
};

use crate::colors::CupertinoColors;
use crate::icons::CupertinoIcons;
use crate::list_tile::CupertinoListTile;
use crate::theme::CupertinoTheme;

/// The curve of the animation used to expand or collapse the
/// [`CupertinoExpansionTile`].
///
/// Eyeballed from an iPhone 15 simulator running iOS 17.5.
fn k_animation_curve() -> Rc<dyn Curve> {
    Curves::ease_in_out()
}

/// The duration of the animation used to expand or collapse the
/// [`CupertinoExpansionTile`].
///
/// Eyeballed from an iPhone 15 simulator running iOS 17.5.
const K_ANIMATION_DURATION: Duration = Duration::from_millis(250);

/// The font size of the rotating trailing icon in the header of a
/// [`CupertinoExpansionTile`].
///
/// Eyeballed from an iPhone 15 simulator running iOS 17.5.
const K_ICON_FONT_SIZE: f64 = 15.0;

/// The height of the header in a [`CupertinoExpansionTile`], which is the default
/// [`CupertinoListTile`].
const K_HEADER_HEIGHT: f64 = 44.0;

/// Defines how a [`CupertinoExpansionTile`] should transition its child between
/// its collapsed state and its expanded state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpansionTileTransitionMode {
    /// Transition by fading a fully extended [`CupertinoExpansionTile::child`].
    ///
    /// When the [`CupertinoExpansionTile`] expands, the child appears fully extended
    /// and fades into view. When the [`CupertinoExpansionTile`] collapses, the child
    /// remains fully extended and fades out of view.
    Fade,

    /// Transition by scrolling [`CupertinoExpansionTile::child`] under the header.
    ///
    /// When the [`CupertinoExpansionTile`] expands, the child scrolls from under the
    /// header until it becomes fully extended. When the [`CupertinoExpansionTile`]
    /// collapses, the child scrolls under the header until it is fully collapsed.
    Scroll,
}

/// A single-line [`CupertinoListTile`] with an expansion arrow icon that expands
/// or collapses the tile to reveal or hide the [`child`](Self::child).
///
/// See also:
///
///  * `ExpansionTile`, the Material Design equivalent.
///  * `CupertinoListSection`, useful for creating an expansion tile
///    [`child`](Self::child).
///  * [`CupertinoListTile`], the header of a [`CupertinoExpansionTile`].
///  * <https://developer.apple.com/design/human-interface-guidelines/disclosure-controls/>
#[derive(Debug)]
pub struct CupertinoExpansionTile {
    /// See [`Widget::key`](reveal_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// Used to convey the central information.
    ///
    /// Usually a `Text`.
    pub title: WidgetRef,

    /// Programmatically expands and collapses the [`CupertinoExpansionTile`].
    ///
    /// In cases where control over the tile's state is needed from a
    /// callback triggered by a widget within the tile, [`ExpansibleController::of`]
    /// may be more convenient than supplying a controller.
    pub controller: Option<Handle<ExpansibleController>>,

    /// The body of the [`CupertinoExpansionTile`].
    pub child: WidgetRef,

    /// How the [`CupertinoExpansionTile`] should transition its child between its
    /// collapsed state and its expanded state.
    ///
    /// Defaults to [`ExpansionTileTransitionMode::Fade`].
    pub transition_mode: ExpansionTileTransitionMode,
}

impl CupertinoExpansionTile {
    /// Creates a single-line [`CupertinoListTile`] with an expansion arrow icon
    /// that expands or collapses the tile to reveal or hide the
    /// [`child`](Self::child); Dart's optional named arguments are the setters.
    pub fn new<T, C>(
        title: impl IntoWidget<T>,
        child: impl IntoWidget<C>,
    ) -> CupertinoExpansionTile {
        CupertinoExpansionTile {
            key: None,
            title: title.into_widget(),
            controller: None,
            child: child.into_widget(),
            transition_mode: ExpansionTileTransitionMode::Fade,
        }
    }

    /// Dart `CupertinoExpansionTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoExpansionTile {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoExpansionTile(controller:)`.
    pub fn controller(
        mut self,
        controller: Handle<ExpansibleController>,
    ) -> CupertinoExpansionTile {
        self.controller = Some(controller);
        self
    }

    /// Dart `CupertinoExpansionTile(transitionMode:)`.
    pub fn transition_mode(
        mut self,
        transition_mode: ExpansionTileTransitionMode,
    ) -> CupertinoExpansionTile {
        self.transition_mode = transition_mode;
        self
    }
}

impl StatefulWidget for CupertinoExpansionTile {
    type State = CupertinoExpansionTileState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoExpansionTileState {
        CupertinoExpansionTileState {
            state: StateData::new(),
            header_key: GlobalKey::new(),
            fade_controller: None,
            quarter_tween: None,
            curve_tween: None,
            tile_controller: None,
            icon_turns: None,
        }
    }
}

/// Dart's `_CupertinoExpansionTileState`.
pub struct CupertinoExpansionTileState {
    state: StateData<CupertinoExpansionTile>,
    header_key: GlobalKey,
    /// Dart's `final OverlayPortalController _fadeController`, created in `init_state`
    /// because an arena object needs the [`App`].
    fade_controller: Option<Handle<OverlayPortalController>>,
    /// Dart's `static final Animatable<double> _quarterTween`, created in `init_state`
    /// because an arena object needs the [`App`]; with it the `CurveTween` it is chained to.
    quarter_tween: Option<Handle<Tween<f64>>>,
    curve_tween: Option<Handle<CurveTween>>,
    tile_controller: Option<Handle<ExpansibleController>>,
    icon_turns: Option<AnyAnimation<f64>>,
}

impl CupertinoExpansionTileState {
    fn tile_controller(self: Handle<Self>, app: &App) -> Handle<ExpansibleController> {
        app.get(self)
            .tile_controller
            .expect("created in init_state")
    }

    fn fade_controller(self: Handle<Self>, app: &App) -> Handle<OverlayPortalController> {
        app.get(self)
            .fade_controller
            .expect("created in init_state")
    }

    fn build_icon(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let quarter_tween = app.get(self).quarter_tween.expect("created in init_state");
        let curve_tween = app.get(self).curve_tween.expect("created in init_state");
        let icon_turns = animation.drive(app, quarter_tween.chain(curve_tween));
        app.get_mut(self).icon_turns = Some(icon_turns);
        let font_size = CupertinoTheme::of(app, context)
            .text_theme()
            .text_style()
            .font_size;
        RotationTransition::new(icon_turns)
            .child(
                SizedBox::square(font_size).child(
                    Center::new().child(
                        Icon::new(Some(CupertinoIcons::right_chevron()))
                            .color(CupertinoColors::ACTIVE_BLUE)
                            .size(K_ICON_FONT_SIZE)
                            .font_weight(FontWeight::W900),
                    ),
                ),
            )
            .into_widget()
    }

    fn on_header_tap(self: Handle<Self>, app: &mut App) {
        let tile_controller = self.tile_controller(app);
        if tile_controller.is_expanded(app) {
            tile_controller.collapse(app);
        } else {
            tile_controller.expand(app);
        }
        self.fade_controller(app).show(app);
    }

    fn build_header(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let trailing = self.build_icon(app, context, animation);
        let title = self.widget(app).title.clone();
        let header_key: KeyRef = Rc::new(app.get(self).header_key.clone());
        CupertinoListTile::new(title)
            .key(header_key)
            .on_tap(Listener::handle_method(self, Self::on_header_tap))
            .trailing(trailing)
            .background_color_activated(CupertinoColors::TRANSPARENT)
            .into_widget()
    }

    fn build_expansible(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        header: &WidgetRef,
        body: &WidgetRef,
        animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let transition_mode = self.widget(app).transition_mode;
        let fading =
            animation.is_animating(app) && transition_mode == ExpansionTileTransitionMode::Fade;
        let body = if fading {
            Opacity::new(0.0).child(body.clone()).into_widget()
        } else {
            body.clone()
        };
        let child = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .children([header.clone(), body])
            .into_widget();
        if transition_mode == ExpansionTileTransitionMode::Scroll {
            return child;
        }
        debug_assert!(transition_mode == ExpansionTileTransitionMode::Fade);
        let fade_controller = self.fade_controller(app);
        LayoutBuilder::new(move |_app, _context, constraints| {
            OverlayPortal::new(fade_controller, move |app, _context| {
                self.build_fading_child(app, constraints, animation)
            })
            .child(child.clone())
            .into_widget()
        })
        .into_widget()
    }

    /// Dart's `overlayChildBuilder`: a fully extended copy of the child, fading in or out over
    /// the content below the header while the tile animates.
    fn build_fading_child(
        self: Handle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let header_context = app
            .get(self)
            .header_key
            .clone()
            .current_context(app)
            .expect("the header is in the tree");
        let overlay = Overlay::of(app, header_context, false)
            .context(app)
            .find_render_object(app)
            .expect("the overlay is laid out")
            .as_box()
            .expect("a box");
        let header_box = header_context
            .find_render_object(app)
            .expect("the header is laid out")
            .as_box()
            .expect("a box");
        let header_offset =
            header_box.local_to_global(app, Offset::ZERO, Some(overlay.as_object()));
        let child = self.widget(app).child.clone();
        Positioned::new(
            ConstrainedBox::new(constraints).child(
                Visibility::new(FadeTransition::new(animation).child(child))
                    .visible(animation.is_animating(app)),
            ),
        )
        .top(header_offset.dy() + K_HEADER_HEIGHT)
        .left(header_offset.dx())
        .into_widget()
    }
}

impl State for CupertinoExpansionTileState {
    type Widget = CupertinoExpansionTile;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let fade_controller = OverlayPortalController::new(app, None);
        let quarter_tween = Tween::new(app, Some(0.0), Some(0.25));
        let curve_tween = CurveTween::new(app, k_animation_curve());
        let tile_controller = match self.widget(app).controller {
            Some(controller) => controller,
            None => ExpansibleController::new(app),
        };
        let state = app.get_mut(self);
        state.fade_controller = Some(fade_controller);
        state.quarter_tween = Some(quarter_tween);
        state.curve_tween = Some(curve_tween);
        state.tile_controller = Some(tile_controller);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoExpansionTile) {
        if old_widget.controller != self.widget(app).controller {
            if old_widget.controller.is_none() {
                self.tile_controller(app).dispose(app);
            }
            let tile_controller = match self.widget(app).controller {
                Some(controller) => controller,
                None => ExpansibleController::new(app),
            };
            app.get_mut(self).tile_controller = Some(tile_controller);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if self.widget(app).controller.is_none() {
            self.tile_controller(app).dispose(app);
        }
        let state = app.get_mut(self);
        let fade_controller = state.fade_controller.take();
        let quarter_tween = state.quarter_tween.take();
        let curve_tween = state.curve_tween.take();
        state.tile_controller = None;
        state.icon_turns = None;
        if let Some(fade_controller) = fade_controller {
            app.destroy(fade_controller);
        }
        if let Some(quarter_tween) = quarter_tween {
            app.destroy(quarter_tween);
        }
        if let Some(curve_tween) = curve_tween {
            app.destroy(curve_tween);
        }
    }

    #[allow(deprecated)]
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let tile_controller = self.tile_controller(app);
        let child = self.widget(app).child.clone();
        Expansible::new(
            Rc::new(move |app, context, animation| self.build_header(app, context, animation)),
            Rc::new(move |_app, _context, _animation| child.clone()),
            tile_controller,
        )
        .duration(K_ANIMATION_DURATION)
        .curve(k_animation_curve())
        .expansible_builder(Rc::new(move |app, context, header, body, animation| {
            self.build_expansible(app, context, header, body, animation)
        }))
        .into_widget()
    }
}

impl fmt::Debug for CupertinoExpansionTileState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoExpansionTileState")
            .field("tile_controller", &self.tile_controller)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::{
        PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, TextDirection,
    };
    use reveal_gestures::GestureBinding;
    use reveal_painting::PaintingBinding;
    use reveal_rendering::{
        AnyRenderObject, CrossAxisAlignment, RenderAnimatedOpacity, RenderAnimatedOpacityMixin,
        RenderBox, RenderFlex, RenderHandle, RenderObject, RenderOpacity,
    };
    use reveal_scheduler::SchedulerBinding;
    use reveal_widgets::{Directionality, GlobalKey, SizedBox};

    use super::*;
    use crate::test_support::{build, pump};

    /// The test view is 2x: logical coordinates double into the packet.
    fn send(app: &mut App, change: PointerChange, x: f64, y: f64, at: Duration) {
        GestureBinding::instance(app).handle_pointer_data_packet(
            app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Touch,
                time_stamp: at,
                pointer_identifier: 1,
                physical_x: x * 2.0,
                physical_y: y * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
    }

    fn tap(app: &mut App, x: f64, y: f64, at: Duration) {
        send(app, PointerChange::Down, x, y, at);
        send(app, PointerChange::Up, x, y, at);
        pump(app, at);
    }

    /// Runs frames until the expansion animation settles.
    fn settle(app: &mut App, from: Duration) -> Duration {
        let mut at = from;
        for _ in 0..20 {
            at += Duration::from_millis(25);
            SchedulerBinding::handle_begin_frame(app, Some(at));
            app.drain_microtasks();
            SchedulerBinding::handle_draw_frame(app);
            app.drain_microtasks();
        }
        at
    }

    struct Mounted {
        app: App,
        state: Handle<CupertinoExpansionTileState>,
    }

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    fn mount(configure: impl FnOnce(CupertinoExpansionTile) -> CupertinoExpansionTile) -> Mounted {
        let mut app = crate::test_support::app();
        install_fonts(&mut app);
        let global_key = GlobalKey::new();
        let key: KeyRef = Rc::new(global_key.clone());
        let tile = configure(CupertinoExpansionTile::new(
            SizedBox::new().width(100.0).height(20.0),
            SizedBox::new().width(100.0).height(50.0),
        ))
        .key(key);
        // A `Column` gives the tile the loose constraints it has in a list, so the copy the
        // fading tile puts in the overlay is as tall as the child rather than as the view.
        let host = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children([tile.into_widget()]);
        build(
            &mut app,
            Directionality::new(TextDirection::Ltr, Overlay::wrap(host)).into_widget(),
        );
        let state = global_key
            .current_state::<CupertinoExpansionTileState>(&mut app)
            .expect("the tile mounted");
        Mounted { app, state }
    }

    /// The first `R` in the render subtree rooted at `render_object`, depth first.
    fn find_in_subtree<R: RenderObject>(
        app: &App,
        render_object: AnyRenderObject,
    ) -> Option<RenderHandle<R>> {
        if let Some(found) = render_object.downcast::<R>(app) {
            return Some(found);
        }
        let mut found = None;
        render_object.visit_children(app, &mut |child| {
            found = found.or_else(|| find_in_subtree::<R>(app, child));
        });
        found
    }

    /// The tile's `Column`: the tile's own render object in a scrolling tile, and below the
    /// `LayoutBuilder` and the `OverlayPortal` in a fading one.
    fn column(mounted: &Mounted) -> AnyRenderObject {
        let context = mounted.state.context(&mounted.app);
        let render_object = context
            .find_render_object(&mounted.app)
            .expect("the tile is laid out");
        find_in_subtree::<RenderFlex>(&mounted.app, render_object)
            .expect("the tile's Column")
            .as_object()
    }

    fn body_height(mounted: &Mounted) -> f64 {
        body(mounted)
            .as_box()
            .expect("a box")
            .size(&mounted.app)
            .height()
    }

    #[test]
    fn a_tap_on_the_header_expands_and_collapses_the_tile() {
        let mut mounted = mount(|tile| tile);
        let controller = mounted.state.tile_controller(&mounted.app);
        assert!(!controller.is_expanded(&mounted.app));
        assert_eq!(body_height(&mounted), 0.0);

        tap(&mut mounted.app, 20.0, 20.0, Duration::ZERO);
        assert!(controller.is_expanded(&mounted.app), "the tap expanded it");
        let at = settle(&mut mounted.app, Duration::ZERO);
        assert_eq!(body_height(&mounted), 50.0);

        tap(&mut mounted.app, 20.0, 20.0, at);
        assert!(
            !controller.is_expanded(&mounted.app),
            "the tap collapsed it"
        );
        settle(&mut mounted.app, at);
        assert_eq!(body_height(&mounted), 0.0);
    }

    #[test]
    fn a_supplied_controller_drives_the_tile() {
        let mut app = crate::test_support::app();
        install_fonts(&mut app);
        let controller = ExpansibleController::new(&mut app);
        let global_key = GlobalKey::new();
        let key: KeyRef = Rc::new(global_key.clone());
        build(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                Overlay::wrap(
                    CupertinoExpansionTile::new(
                        SizedBox::new().width(100.0).height(20.0),
                        SizedBox::new().width(100.0).height(50.0),
                    )
                    .controller(controller)
                    .transition_mode(ExpansionTileTransitionMode::Scroll)
                    .key(key),
                ),
            )
            .into_widget(),
        );
        let state = global_key
            .current_state::<CupertinoExpansionTileState>(&mut app)
            .expect("the tile mounted");
        assert_eq!(state.tile_controller(&app), controller);

        controller.expand(&mut app);
        let mounted = Mounted { app, state };
        assert!(controller.is_expanded(&mounted.app));
        let mut mounted = mounted;
        settle(&mut mounted.app, Duration::ZERO);
        assert_eq!(body_height(&mounted), 50.0);
    }

    #[test]
    fn the_chevron_turns_a_quarter_over_the_expansion() {
        let mut mounted = mount(|tile| tile);
        let turns = mounted
            .app
            .get(mounted.state)
            .icon_turns
            .expect("built with the header");
        assert_eq!(turns.value(&mounted.app), 0.0);

        let controller = mounted.state.tile_controller(&mounted.app);
        controller.expand(&mut mounted.app);
        settle(&mut mounted.app, Duration::ZERO);
        let turns = mounted
            .app
            .get(mounted.state)
            .icon_turns
            .expect("rebuilt with the header");
        assert_eq!(turns.value(&mounted.app), 0.25);
    }

    /// The `Column`'s second child: the body, wrapped in an `Opacity` while a fading tile
    /// animates.
    fn body(mounted: &Mounted) -> AnyRenderObject {
        let mut children = Vec::new();
        column(mounted).visit_children(&mounted.app, &mut |child| children.push(child));
        children[1]
    }

    /// The `FadeTransition` a fading tile paints over the content below the header, while its
    /// `Visibility` shows it.
    fn fading_copy(mounted: &mut Mounted) -> Option<RenderHandle<RenderAnimatedOpacity>> {
        let context = mounted.state.context(&mounted.app);
        let theater = Overlay::of(&mut mounted.app, context, false)
            .context(&mounted.app)
            .find_render_object(&mounted.app)
            .expect("the overlay is laid out");
        find_in_subtree::<RenderAnimatedOpacity>(&mounted.app, theater)
    }

    fn one_frame(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    #[test]
    fn a_fading_tile_hides_its_in_place_body_while_it_animates() {
        let mut mounted = mount(|tile| tile);
        assert!(
            body(&mounted)
                .downcast::<RenderOpacity>(&mounted.app)
                .is_none(),
            "no Opacity while the tile rests"
        );

        mounted
            .state
            .tile_controller(&mounted.app)
            .expand(&mut mounted.app);
        one_frame(&mut mounted.app, Duration::from_millis(25));
        one_frame(&mut mounted.app, Duration::from_millis(50));
        let hidden = body(&mounted)
            .downcast::<RenderOpacity>(&mounted.app)
            .expect("an Opacity while a fading tile animates");
        assert_eq!(hidden.opacity(&mounted.app), 0.0);

        settle(&mut mounted.app, Duration::from_millis(50));
        assert!(
            body(&mounted)
                .downcast::<RenderOpacity>(&mounted.app)
                .is_none(),
            "the Opacity goes once the animation settles"
        );
    }

    #[test]
    fn a_fading_tile_fades_an_extended_copy_over_the_content_below_the_header() {
        let mut mounted = mount(|tile| tile);
        assert!(
            fading_copy(&mut mounted).is_none(),
            "nothing is in the overlay before the first tap"
        );

        tap(&mut mounted.app, 20.0, 20.0, Duration::ZERO);
        one_frame(&mut mounted.app, Duration::from_millis(25));
        one_frame(&mut mounted.app, Duration::from_millis(50));
        let copy = fading_copy(&mut mounted).expect("the overlay child fades the copy in");
        let opacity = copy.opacity(&mounted.app).value(&mounted.app);
        assert!(
            opacity > 0.0 && opacity < 1.0,
            "the copy fades in: {opacity}"
        );
        assert_eq!(
            copy.as_box()
                .local_to_global(&mounted.app, Offset::ZERO, None),
            Offset::new(0.0, K_HEADER_HEIGHT),
            "the copy sits under the header"
        );
        assert_eq!(
            copy.size(&mounted.app).height(),
            50.0,
            "the copy is fully extended while the body below is not"
        );
        assert!(
            body_height(&mounted) < 50.0,
            "the in-place body is still expanding"
        );

        settle(&mut mounted.app, Duration::from_millis(50));
        assert!(
            fading_copy(&mut mounted).is_none(),
            "the Visibility hides the copy once the animation settles"
        );
    }

    #[test]
    fn a_scrolling_tile_never_hides_its_body() {
        let mut mounted = mount(|tile| tile.transition_mode(ExpansionTileTransitionMode::Scroll));
        mounted
            .state
            .tile_controller(&mounted.app)
            .expand(&mut mounted.app);
        one_frame(&mut mounted.app, Duration::from_millis(25));
        one_frame(&mut mounted.app, Duration::from_millis(50));
        assert!(
            body(&mounted)
                .downcast::<RenderOpacity>(&mounted.app)
                .is_none(),
            "a scrolling tile shows the body all the way"
        );
    }
}
