//! Flutter counterpart: `cupertino/expansion_tile.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Animatable, AnimationRef, Curve, CurveTween, Curves, Drive, Tween};
use waft_embedder::{FontWeight, Offset, TargetPlatform};
use waft_foundation::{App, Handle, OptionRetained, Ref, Task};
use waft_rendering::{BoxConstraints, MainAxisSize};
use waft_widgets::{
    BuildContext, Center, Column, ConstrainedBox, Expansible, ExpansibleController, FadeTransition,
    GlobalKey, Icon, IntoWidget, KeyRef, LayoutBuilder, Opacity, Overlay, OverlayPortal,
    OverlayPortalController, Positioned, RotationTransition, Semantics, SizedBox, State, StateData,
    StatefulWidget, Visibility, WidgetRef,
};

use crate::colors::CupertinoColors;
use crate::icons::CupertinoIcons;
use crate::list_tile::CupertinoListTile;
use crate::localizations::CupertinoLocalizations;
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
    /// See [`Widget::key`](waft_widgets::Widget::key).
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
    pub controller: Option<Ref<ExpansibleController>>,

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
    pub fn controller(mut self, controller: Ref<ExpansibleController>) -> CupertinoExpansionTile {
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
    fade_controller: Option<Ref<OverlayPortalController>>,
    /// Dart's `static final Animatable<double> _quarterTween`, created in `init_state`
    /// because an arena object needs the [`App`]; with it the `CurveTween` it is chained to.
    quarter_tween: Option<Ref<Tween<f64>>>,
    curve_tween: Option<Ref<CurveTween>>,
    tile_controller: Option<Ref<ExpansibleController>>,
    icon_turns: Option<AnimationRef<f64>>,
}

impl CupertinoExpansionTileState {
    fn tile_controller(self: Handle<Self>, app: &App) -> Handle<ExpansibleController> {
        app.get(self)
            .tile_controller
            .handle()
            .expect("created in init_state")
    }

    fn fade_controller(self: Handle<Self>, app: &App) -> Handle<OverlayPortalController> {
        app.get(self)
            .fade_controller
            .handle()
            .expect("created in init_state")
    }

    fn build_icon(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let quarter_tween = app
            .get(self)
            .quarter_tween
            .clone()
            .expect("created in init_state");
        let curve_tween = app
            .get(self)
            .curve_tween
            .clone()
            .expect("created in init_state");
        let icon_turns = animation.drive(app, quarter_tween.chain(curve_tween));
        app.get_mut(self).icon_turns = Some(icon_turns.clone());
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
        animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let localizations = <dyn CupertinoLocalizations>::of(app, context);
        let is_expanded = self.tile_controller(app).is_expanded(app);
        let on_tap_hint = if is_expanded {
            localizations.expansion_tile_expanded_tap_hint()
        } else {
            localizations.expansion_tile_collapsed_tap_hint()
        };
        let semantics_hint = match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => Some(if is_expanded {
                format!(
                    "{}\n {}",
                    localizations.collapsed_hint(),
                    localizations.expansion_tile_expanded_hint()
                )
            } else {
                format!(
                    "{}\n {}",
                    localizations.expanded_hint(),
                    localizations.expansion_tile_collapsed_hint()
                )
            }),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => None,
        };
        let trailing = self.build_icon(app, context, animation);
        let title = self.widget(app).title.clone();
        let header_key: KeyRef = app.get(self).header_key.clone().into();
        let mut semantics = Semantics::new().on_tap_hint(on_tap_hint);
        if let Some(semantics_hint) = semantics_hint {
            semantics = semantics.hint(semantics_hint);
        }
        semantics
            .child(
                CupertinoListTile::new(title)
                    .key(header_key)
                    .on_tap(Rc::new(move |app: &mut App| {
                        self.on_header_tap(app);
                        Task::ready(())
                    }))
                    .trailing(trailing)
                    .background_color_activated(CupertinoColors::TRANSPARENT),
            )
            .into_widget()
    }

    fn build_expansible(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        header: &WidgetRef,
        body: &WidgetRef,
        animation: AnimationRef<f64>,
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
        let fade_controller = app
            .get(self)
            .fade_controller
            .clone()
            .expect("created in init_state");
        LayoutBuilder::new(move |_app, _context, constraints| {
            OverlayPortal::new(fade_controller.clone(), {
                let animation = animation.clone();
                move |app, _context| {
                    self.build_fading_child(app, constraints.constraints(), animation.clone())
                }
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
        animation: AnimationRef<f64>,
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
                Visibility::new(FadeTransition::new(animation.clone()).child(child))
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
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let fade_controller = OverlayPortalController::new(app, None);
        let quarter_tween = Tween::new(app, Some(0.0), Some(0.25));
        let curve_tween = CurveTween::new(app, k_animation_curve());
        let tile_controller = match self.widget(app).controller.clone() {
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
        if old_widget.controller.handle() != self.widget(app).controller.handle() {
            if old_widget.controller.is_none() {
                self.tile_controller(app).dispose(app);
            }
            let tile_controller = match self.widget(app).controller.clone() {
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
    }

    #[allow(deprecated)]
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let tile_controller = app
            .get(self)
            .tile_controller
            .clone()
            .expect("created in init_state");
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
    use waft_embedder::{Locale, TextDirection};
    use waft_rendering::{
        AnyRenderObject, CrossAxisAlignment, RenderAnimatedOpacity, RenderAnimatedOpacityMixin,
        RenderBox, RenderFlex, RenderHandle, RenderObject, RenderOpacity,
    };
    use waft_widgets::testing::{WidgetTester, test_widgets};
    use waft_widgets::{
        DefaultWidgetsLocalizations, Directionality, GlobalKey, Localizations, SizedBox,
    };

    use super::*;
    use crate::localizations::DefaultCupertinoLocalizations;

    /// The localizations `CupertinoApp` gives a tile; its header reads its tap hints there.
    fn localized<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Localizations::new(
            Locale::new("en"),
            vec![
                DefaultWidgetsLocalizations::delegate(),
                DefaultCupertinoLocalizations::delegate(),
            ],
        )
        .child(child)
        .into_widget()
    }

    /// The header's tap: down and up, then the frame they ask for.
    fn tap_header(tester: &mut WidgetTester) {
        tester.tap(Offset::new(20.0, 20.0), 1);
        tester.pump();
    }

    /// Runs frames until the expansion animation settles.
    fn settle(tester: &mut WidgetTester) {
        for _ in 0..20 {
            tester.pump_by(Duration::from_millis(25));
        }
    }

    struct Mounted<'t> {
        tester: &'t mut WidgetTester,
        state: Handle<CupertinoExpansionTileState>,
    }

    fn mount(
        tester: &mut WidgetTester,
        configure: impl FnOnce(CupertinoExpansionTile) -> CupertinoExpansionTile,
    ) -> Mounted<'_> {
        let global_key = GlobalKey::new();
        let key: KeyRef = global_key.clone().into();
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
        tester.pump_widget(localized(Directionality::new(
            TextDirection::Ltr,
            Overlay::wrap(host),
        )));
        let state = global_key
            .current_state::<CupertinoExpansionTileState>(&tester.cell.borrow())
            .expect("the tile mounted");
        Mounted { tester, state }
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
    fn column(mounted: &Mounted<'_>) -> AnyRenderObject {
        let context = mounted.state.context(&mounted.tester.cell.borrow());
        let render_object = context
            .find_render_object(&mounted.tester.cell.borrow())
            .expect("the tile is laid out");
        find_in_subtree::<RenderFlex>(&mounted.tester.cell.borrow(), render_object)
            .expect("the tile's Column")
            .as_object()
    }

    fn body_height(mounted: &Mounted<'_>) -> f64 {
        body(mounted)
            .as_box()
            .expect("a box")
            .size(&mounted.tester.cell.borrow())
            .height()
    }

    #[test]
    fn a_tap_on_the_header_expands_and_collapses_the_tile() {
        test_widgets(|tester| {
            let mounted = mount(tester, |tile| tile);
            let controller = mounted.state.tile_controller(&mounted.tester.cell.borrow());
            assert!(!controller.is_expanded(&mounted.tester.cell.borrow()));
            assert_eq!(body_height(&mounted), 0.0);

            tap_header(mounted.tester);
            assert!(
                controller.is_expanded(&mounted.tester.cell.borrow()),
                "the tap expanded it"
            );
            settle(mounted.tester);
            assert_eq!(body_height(&mounted), 50.0);

            tap_header(mounted.tester);
            assert!(
                !controller.is_expanded(&mounted.tester.cell.borrow()),
                "the tap collapsed it"
            );
            settle(mounted.tester);
            assert_eq!(body_height(&mounted), 0.0);
        });
    }

    #[test]
    fn a_supplied_controller_drives_the_tile() {
        test_widgets(|tester| {
            let controller = tester.update(ExpansibleController::new);
            let global_key = GlobalKey::new();
            let key: KeyRef = global_key.clone().into();
            tester.pump_widget(localized(Directionality::new(
                TextDirection::Ltr,
                Overlay::wrap(
                    CupertinoExpansionTile::new(
                        SizedBox::new().width(100.0).height(20.0),
                        SizedBox::new().width(100.0).height(50.0),
                    )
                    .controller(controller.clone())
                    .transition_mode(ExpansionTileTransitionMode::Scroll)
                    .key(key),
                ),
            )));
            let state = global_key
                .current_state::<CupertinoExpansionTileState>(&tester.cell.borrow())
                .expect("the tile mounted");
            assert_eq!(
                state.tile_controller(&tester.cell.borrow()),
                controller.handle()
            );

            tester.update(|app| controller.expand(app));
            assert!(controller.is_expanded(&tester.cell.borrow()));
            settle(tester);
            let mounted = Mounted { tester, state };
            assert_eq!(body_height(&mounted), 50.0);
        });
    }

    #[test]
    fn the_chevron_turns_a_quarter_over_the_expansion() {
        test_widgets(|tester| {
            let mounted = mount(tester, |tile| tile);
            let turns = mounted
                .tester
                .cell
                .borrow()
                .get(mounted.state)
                .icon_turns
                .clone()
                .expect("built with the header");
            assert_eq!(turns.value(&mounted.tester.cell.borrow()), 0.0);

            let controller = mounted.state.tile_controller(&mounted.tester.cell.borrow());
            controller.expand(&mut mounted.tester.cell.borrow_mut());
            settle(mounted.tester);
            let turns = mounted
                .tester
                .cell
                .borrow()
                .get(mounted.state)
                .icon_turns
                .clone()
                .expect("rebuilt with the header");
            assert_eq!(turns.value(&mounted.tester.cell.borrow()), 0.25);
        });
    }

    /// The `Column`'s second child: the body, wrapped in an `Opacity` while a fading tile
    /// animates.
    fn body(mounted: &Mounted<'_>) -> AnyRenderObject {
        let mut children = Vec::new();
        column(mounted).visit_children(&mounted.tester.cell.borrow(), &mut |child| {
            children.push(child)
        });
        children[1]
    }

    /// The `FadeTransition` a fading tile paints over the content below the header, while its
    /// `Visibility` shows it.
    fn fading_copy(mounted: &mut Mounted<'_>) -> Option<RenderHandle<RenderAnimatedOpacity>> {
        let context = mounted.state.context(&mounted.tester.cell.borrow());
        let theater = {
            let mut app = mounted.tester.cell.borrow_mut();
            Overlay::of(&mut app, context, false)
                .context(&app)
                .find_render_object(&app)
                .expect("the overlay is laid out")
        };
        find_in_subtree::<RenderAnimatedOpacity>(&mounted.tester.cell.borrow(), theater)
    }

    #[test]
    fn a_fading_tile_hides_its_in_place_body_while_it_animates() {
        test_widgets(|tester| {
            let mounted = mount(tester, |tile| tile);
            assert!(
                body(&mounted)
                    .downcast::<RenderOpacity>(&mounted.tester.cell.borrow())
                    .is_none(),
                "no Opacity while the tile rests"
            );

            {
                let mut app = mounted.tester.cell.borrow_mut();
                mounted.state.tile_controller(&app).expand(&mut app);
            }
            mounted.tester.pump_by(Duration::from_millis(25));
            mounted.tester.pump_by(Duration::from_millis(25));
            let hidden = body(&mounted)
                .downcast::<RenderOpacity>(&mounted.tester.cell.borrow())
                .expect("an Opacity while a fading tile animates");
            assert_eq!(hidden.opacity(&mounted.tester.cell.borrow()), 0.0);

            settle(mounted.tester);
            assert!(
                body(&mounted)
                    .downcast::<RenderOpacity>(&mounted.tester.cell.borrow())
                    .is_none(),
                "the Opacity goes once the animation settles"
            );
        });
    }

    #[test]
    fn a_fading_tile_fades_an_extended_copy_over_the_content_below_the_header() {
        test_widgets(|tester| {
            let mut mounted = mount(tester, |tile| tile);
            assert!(
                fading_copy(&mut mounted).is_none(),
                "nothing is in the overlay before the first tap"
            );

            tap_header(mounted.tester);
            mounted.tester.pump_by(Duration::from_millis(25));
            mounted.tester.pump_by(Duration::from_millis(25));
            let copy = fading_copy(&mut mounted).expect("the overlay child fades the copy in");
            let opacity = copy
                .opacity(&mounted.tester.cell.borrow())
                .value(&mounted.tester.cell.borrow());
            assert!(
                opacity > 0.0 && opacity < 1.0,
                "the copy fades in: {opacity}"
            );
            assert_eq!(
                copy.as_box()
                    .local_to_global(&mounted.tester.cell.borrow(), Offset::ZERO, None),
                Offset::new(0.0, K_HEADER_HEIGHT),
                "the copy sits under the header"
            );
            assert_eq!(
                copy.size(&mounted.tester.cell.borrow()).height(),
                50.0,
                "the copy is fully extended while the body below is not"
            );
            assert!(
                body_height(&mounted) < 50.0,
                "the in-place body is still expanding"
            );

            settle(mounted.tester);
            assert!(
                fading_copy(&mut mounted).is_none(),
                "the Visibility hides the copy once the animation settles"
            );
        });
    }

    #[test]
    fn a_scrolling_tile_never_hides_its_body() {
        test_widgets(|tester| {
            let mounted = mount(tester, |tile| {
                tile.transition_mode(ExpansionTileTransitionMode::Scroll)
            });
            {
                let mut app = mounted.tester.cell.borrow_mut();
                mounted.state.tile_controller(&app).expand(&mut app);
            }
            mounted.tester.pump_by(Duration::from_millis(25));
            mounted.tester.pump_by(Duration::from_millis(25));
            assert!(
                body(&mounted)
                    .downcast::<RenderOpacity>(&mounted.tester.cell.borrow())
                    .is_none(),
                "a scrolling tile shows the body all the way"
            );
        });
    }
}
