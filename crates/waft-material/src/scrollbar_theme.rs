//! Flutter counterpart: material_ui's `scrollbar_theme.dart`.

use waft_embedder::{Color, Radius, lerp_double};
use waft_foundation::App;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::theme::Theme;

/// Defines default property values for descendant `Scrollbar` widgets.
///
/// Descendant widgets obtain the current [`ScrollbarThemeData`] object with
/// [`ScrollbarTheme::of`]. Instances of [`ScrollbarThemeData`] can be
/// customized with [`ScrollbarThemeData::copy_with`].
///
/// Typically the [`ScrollbarThemeData`] of a [`ScrollbarTheme`] is specified as
/// part of the overall `Theme` with `ThemeData::scrollbar_theme`.
///
/// All [`ScrollbarThemeData`] properties are `None` by default. When `None`, the
/// `Scrollbar` computes its own default values, typically based on the overall
/// theme's `ThemeData::color_scheme`.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct ScrollbarThemeData {
    /// Overrides the default value of `Scrollbar::thumb_visibility` in all
    /// descendant `Scrollbar` widgets.
    pub thumb_visibility: Option<WidgetStatePropertyRef<Option<bool>>>,

    /// Overrides the default value of `Scrollbar::thickness` in all
    /// descendant `Scrollbar` widgets.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Hovered` on web and desktop platforms.
    pub thickness: Option<WidgetStatePropertyRef<Option<f64>>>,

    /// Overrides the default value of `Scrollbar::track_visibility` in all
    /// descendant `Scrollbar` widgets.
    pub track_visibility: Option<WidgetStatePropertyRef<Option<bool>>>,

    /// Overrides the default value of `Scrollbar::interactive` in all
    /// descendant `Scrollbar` widgets.
    pub interactive: Option<bool>,

    /// Overrides the default value of `Scrollbar::radius` in all
    /// descendant widgets.
    pub radius: Option<Radius>,

    /// Overrides the default [`Color`] of the `Scrollbar` thumb in all descendant
    /// `Scrollbar` widgets.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Dragged`.
    ///  * `WidgetState::Hovered` on web and desktop platforms.
    pub thumb_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default [`Color`] of the `Scrollbar` track when
    /// [`track_visibility`](Self::track_visibility) is true in all descendant `Scrollbar`
    /// widgets.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Hovered` on web and desktop platforms.
    pub track_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default [`Color`] of the `Scrollbar` track border when
    /// [`track_visibility`](Self::track_visibility) is true in all descendant `Scrollbar`
    /// widgets.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Hovered` on web and desktop platforms.
    pub track_border_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default value of the `ScrollbarPainter::cross_axis_margin`
    /// property in all descendant `Scrollbar` widgets.
    ///
    /// See also:
    ///
    ///  * `ScrollbarPainter::cross_axis_margin`, which sets the distance from the
    ///    scrollbar's side to the nearest edge in logical pixels.
    pub cross_axis_margin: Option<f64>,

    /// Overrides the default value of the `ScrollbarPainter::main_axis_margin`
    /// property in all descendant `Scrollbar` widgets.
    ///
    /// See also:
    ///
    ///  * `ScrollbarPainter::main_axis_margin`, which sets the distance from the
    ///    scrollbar's start and end to the edge of the viewport in logical pixels.
    pub main_axis_margin: Option<f64>,

    /// Overrides the default value of the `ScrollbarPainter::min_length`
    /// property in all descendant `Scrollbar` widgets.
    ///
    /// See also:
    ///
    ///  * `ScrollbarPainter::min_length`, which sets the preferred smallest size
    ///    the scrollbar can shrink to when the total scrollable extent is large,
    ///    the current visible viewport is small, and the viewport is not
    ///    overscrolled.
    pub min_thumb_length: Option<f64>,
}

impl ScrollbarThemeData {
    /// Creates a theme that can be used for `ThemeData::scrollbar_theme`; Dart's optional
    /// named arguments are the fluent setters.
    pub fn new() -> ScrollbarThemeData {
        ScrollbarThemeData::default()
    }

    /// Dart `ScrollbarThemeData(thumbVisibility:)`.
    pub fn thumb_visibility(
        mut self,
        thumb_visibility: WidgetStatePropertyRef<Option<bool>>,
    ) -> ScrollbarThemeData {
        self.thumb_visibility = Some(thumb_visibility);
        self
    }

    /// Dart `ScrollbarThemeData(thickness:)`.
    pub fn thickness(
        mut self,
        thickness: WidgetStatePropertyRef<Option<f64>>,
    ) -> ScrollbarThemeData {
        self.thickness = Some(thickness);
        self
    }

    /// Dart `ScrollbarThemeData(trackVisibility:)`.
    pub fn track_visibility(
        mut self,
        track_visibility: WidgetStatePropertyRef<Option<bool>>,
    ) -> ScrollbarThemeData {
        self.track_visibility = Some(track_visibility);
        self
    }

    /// Dart `ScrollbarThemeData(radius:)`.
    pub fn radius(mut self, radius: Radius) -> ScrollbarThemeData {
        self.radius = Some(radius);
        self
    }

    /// Dart `ScrollbarThemeData(thumbColor:)`.
    pub fn thumb_color(
        mut self,
        thumb_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> ScrollbarThemeData {
        self.thumb_color = Some(thumb_color);
        self
    }

    /// Dart `ScrollbarThemeData(trackColor:)`.
    pub fn track_color(
        mut self,
        track_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> ScrollbarThemeData {
        self.track_color = Some(track_color);
        self
    }

    /// Dart `ScrollbarThemeData(trackBorderColor:)`.
    pub fn track_border_color(
        mut self,
        track_border_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> ScrollbarThemeData {
        self.track_border_color = Some(track_border_color);
        self
    }

    /// Dart `ScrollbarThemeData(crossAxisMargin:)`.
    pub fn cross_axis_margin(mut self, cross_axis_margin: f64) -> ScrollbarThemeData {
        self.cross_axis_margin = Some(cross_axis_margin);
        self
    }

    /// Dart `ScrollbarThemeData(mainAxisMargin:)`.
    pub fn main_axis_margin(mut self, main_axis_margin: f64) -> ScrollbarThemeData {
        self.main_axis_margin = Some(main_axis_margin);
        self
    }

    /// Dart `ScrollbarThemeData(minThumbLength:)`.
    pub fn min_thumb_length(mut self, min_thumb_length: f64) -> ScrollbarThemeData {
        self.min_thumb_length = Some(min_thumb_length);
        self
    }

    /// Dart `ScrollbarThemeData(interactive:)`.
    pub fn interactive(mut self, interactive: bool) -> ScrollbarThemeData {
        self.interactive = Some(interactive);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().interactive(false)`).
    pub fn copy_with(&self) -> ScrollbarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two Scrollbar themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&ScrollbarThemeData>,
        b: Option<&ScrollbarThemeData>,
        t: f64,
    ) -> ScrollbarThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        ScrollbarThemeData {
            thumb_visibility: <dyn WidgetStateProperty<Option<bool>>>::lerp_optional(
                a.and_then(|a| a.thumb_visibility.clone()),
                b.and_then(|b| b.thumb_visibility.clone()),
                t,
                lerp_bool,
            ),
            thickness: <dyn WidgetStateProperty<Option<f64>>>::lerp_optional(
                a.and_then(|a| a.thickness.clone()),
                b.and_then(|b| b.thickness.clone()),
                t,
                lerp_double,
            ),
            track_visibility: <dyn WidgetStateProperty<Option<bool>>>::lerp_optional(
                a.and_then(|a| a.track_visibility.clone()),
                b.and_then(|b| b.track_visibility.clone()),
                t,
                lerp_bool,
            ),
            interactive: lerp_bool(
                a.and_then(|a| a.interactive),
                b.and_then(|b| b.interactive),
                t,
            ),
            radius: Radius::lerp(a.and_then(|a| a.radius), b.and_then(|b| b.radius), t),
            thumb_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.thumb_color.clone()),
                b.and_then(|b| b.thumb_color.clone()),
                t,
                Color::lerp,
            ),
            track_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.track_color.clone()),
                b.and_then(|b| b.track_color.clone()),
                t,
                Color::lerp,
            ),
            track_border_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.track_border_color.clone()),
                b.and_then(|b| b.track_border_color.clone()),
                t,
                Color::lerp,
            ),
            cross_axis_margin: lerp_double(
                a.and_then(|a| a.cross_axis_margin),
                b.and_then(|b| b.cross_axis_margin),
                t,
            ),
            main_axis_margin: lerp_double(
                a.and_then(|a| a.main_axis_margin),
                b.and_then(|b| b.main_axis_margin),
                t,
            ),
            min_thumb_length: lerp_double(
                a.and_then(|a| a.min_thumb_length),
                b.and_then(|b| b.min_thumb_length),
                t,
            ),
        }
    }
}

impl PartialEq for ScrollbarThemeData {
    fn eq(&self, other: &ScrollbarThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.thumb_visibility == self.thumb_visibility
            && other.thickness == self.thickness
            && other.track_visibility == self.track_visibility
            && other.interactive == self.interactive
            && other.radius == self.radius
            && other.thumb_color == self.thumb_color
            && other.track_color == self.track_color
            && other.track_border_color == self.track_border_color
            && other.cross_axis_margin == self.cross_axis_margin
            && other.main_axis_margin == self.main_axis_margin
            && other.min_thumb_length == self.min_thumb_length
    }
}

fn lerp_bool(a: Option<bool>, b: Option<bool>, t: f64) -> Option<bool> {
    if t < 0.5 { a } else { b }
}

/// Applies a scrollbar theme to descendant `Scrollbar` widgets.
///
/// Descendant widgets obtain the current theme's [`ScrollbarThemeData`] using
/// [`ScrollbarTheme::of`]. When a widget uses [`ScrollbarTheme::of`], it is
/// automatically rebuilt if the theme later changes.
///
/// A scrollbar theme can be specified as part of the overall Material theme
/// using `ThemeData::scrollbar_theme`.
///
/// See also:
///
///  * [`ScrollbarThemeData`], which describes the configuration of a
///    scrollbar theme.
#[derive(Debug)]
pub struct ScrollbarTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant `Scrollbar` widgets.
    pub data: ScrollbarThemeData,
    pub child: WidgetRef,
}

impl ScrollbarTheme {
    /// Constructs a scrollbar theme that configures all descendant `Scrollbar`
    /// widgets.
    pub fn new<K>(data: ScrollbarThemeData, child: impl IntoWidget<K>) -> ScrollbarTheme {
        ScrollbarTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ScrollbarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ScrollbarTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration [`data`](Self::data) from the closest [`ScrollbarTheme`]
    /// ancestor. If there is no ancestor, it returns `ThemeData::scrollbar_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ScrollbarTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ScrollbarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<ScrollbarTheme>(app) {
            Some(scrollbar_theme) => scrollbar_theme.data.clone(),
            None => Theme::of(app, context).scrollbar_theme.clone(),
        }
    }
}

impl InheritedTheme for ScrollbarTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ScrollbarTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ScrollbarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ScrollbarTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use waft_embedder::{Offset, Rect, TargetPlatform};
    use waft_widgets::{ScrollController, ScrollControllerLeaf, WidgetState, WidgetStates};

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::scrollbar::Scrollbar;
    use crate::scrollbar::tests::{
        DESKTOP, K_DEFAULT_IDLE_THUMB_COLOR, K_DEFAULT_THUMB_RADIUS, assert_thumb_rrect,
        assert_track, hover_at, move_by, no_scrollbar, painted, scroll_view, start_gesture,
        tester_on,
    };
    use crate::test_support::{Drawn, WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    // The const represents the starting position of the scrollbar thumb for
    // the below tests. The thumb is 90 pixels long, and 8 pixels wide, with a 2
    // pixel margin to the right edge of the viewport.
    const K_MATERIAL_DESIGN_INITIAL_THUMB_RECT: Rect = Rect::from_ltrb(790.0, 0.0, 798.0, 90.0);
    const K_DEFAULT_DRAG_THUMB_COLOR: u32 = 0x99000000;

    /// Dart's `Colors.red`, `Colors.blue`, `Colors.green`, `Colors.yellow` and
    /// `Colors.black`.
    const RED: Color = Color::new(0xfff44336);
    const BLUE: Color = Color::new(0xff2196f3);
    const GREEN: Color = Color::new(0xff4caf50);
    const YELLOW: Color = Color::new(0xffffeb3b);
    const BLACK: Color = Color::new(0xff000000);

    const DESKTOP_AND_FUCHSIA: [TargetPlatform; 4] =
        [DESKTOP[0], DESKTOP[1], DESKTOP[2], TargetPlatform::Fuchsia];

    #[test]
    fn scrollbar_theme_data_copy_with_equality_basics() {
        assert_eq!(
            ScrollbarThemeData::new(),
            ScrollbarThemeData::new().copy_with()
        );
    }

    #[test]
    fn scrollbar_theme_data_lerp_special_cases() {
        assert_eq!(
            ScrollbarThemeData::lerp(None, None, 0.0),
            ScrollbarThemeData::new()
        );
        let data = ScrollbarThemeData::new()
            .thickness(<dyn WidgetStateProperty<Option<f64>>>::all(Some(3.0)));
        // Dart's `identical`: the same property objects come back.
        let lerped = ScrollbarThemeData::lerp(Some(&data), Some(&data), 0.5);
        assert!(Rc::ptr_eq(
            lerped.thickness.as_ref().expect("lerped"),
            data.thickness.as_ref().expect("given")
        ));
    }

    /// `MaterialApp(theme: theme, home: ScrollConfiguration(NoScrollbarBehavior,
    /// Scrollbar(thumbVisibility: true, controller:, child: SingleChildScrollView(...))))`,
    /// with `wrap` around the scrollbar, its theme animated as `MaterialApp` animates it.
    fn pump_frame(
        tester: &mut WidgetTester,
        theme: ThemeData,
        scrollbar: impl FnOnce(Scrollbar) -> Scrollbar,
        wrap: impl FnOnce(WidgetRef) -> WidgetRef,
    ) {
        let controller = ScrollController::default(&mut tester.cell.borrow_mut()).as_controller();
        let scrollbar = scrollbar(
            Scrollbar::new(scroll_view(Some(controller), 4000.0, 4000.0))
                .thumb_visibility(true)
                .controller(controller),
        );
        tester.pump_widget(material_app(
            theme,
            no_scrollbar(wrap(scrollbar.into_widget())),
        ));
    }

    fn hovered_track_visibility() -> WidgetStatePropertyRef<Option<bool>> {
        <dyn WidgetStateProperty<Option<bool>>>::resolve_with(|states: &WidgetStates| {
            Some(states.contains(&WidgetState::Hovered))
        })
    }

    /// Drags the thumb from (797, 45), asserting `drag_color` while it is held, by 10
    /// pixels down.
    fn drag_thumb(tester: &mut WidgetTester, rect: Rect, radius: Radius, drag_color: u32) {
        const SCROLL_AMOUNT: f64 = 10.0;
        let mut drag_scrollbar_gesture = start_gesture(tester, Offset::new(797.0, 45.0));
        tester.pump_and_settle();

        // Drag color
        assert_thumb_rrect(tester, rect, radius, drag_color);

        move_by(
            tester,
            &mut drag_scrollbar_gesture,
            Offset::new(0.0, SCROLL_AMOUNT),
        );
        tester.pump_and_settle();
        tester.up(&drag_scrollbar_gesture);
        tester.pump_and_settle();
    }

    #[test]
    fn passing_no_scrollbar_theme_returns_defaults() {
        for platform in DESKTOP_AND_FUCHSIA {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .scrollbar_theme(
                        ScrollbarThemeData::new().track_visibility(hovered_track_visibility()),
                    )
                    .build(&tester.cell.borrow());
                pump_frame(&mut tester, theme, |scrollbar| scrollbar, |child| child);
                tester.pump_and_settle();
                // Idle scrollbar behavior
                assert_thumb_rrect(
                    &tester,
                    K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                // Drag scrollbar behavior
                drag_thumb(
                    &mut tester,
                    K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_DRAG_THUMB_COLOR,
                );

                // Hover scrollbar behavior
                hover_at(&mut tester, Offset::new(794.0, 5.0));
                tester.pump_and_settle();

                let drawn = painted(&tester);
                assert_track(
                    &tester,
                    &drawn,
                    Rect::from_ltrb(784.0, 0.0, 800.0, 600.0),
                    0x08000000,
                    (Offset::new(784.0, 0.0), Offset::new(784.0, 600.0)),
                    0x1a000000,
                );
                // Scrollbar thumb is larger; hover color
                assert_thumb_rrect(
                    &tester,
                    Rect::from_ltrb(786.0, 10.0, 798.0, 100.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0x80000000,
                );
            });
        }
    }

    #[test]
    fn scrollbar_uses_values_from_scrollbar_theme() {
        for platform in DESKTOP_AND_FUCHSIA {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let theme = ThemeData::builder()
                    .scrollbar_theme(scrollbar_theme())
                    .build(&tester.cell.borrow());
                pump_frame(&mut tester, theme, |scrollbar| scrollbar, |child| child);
                tester.pump_and_settle();
                // Idle scrollbar behavior
                assert_thumb_rrect(
                    &tester,
                    Rect::from_ltrb(785.0, 10.0, 795.0, 97.0),
                    Radius::circular(6.0),
                    GREEN.to_argb32(),
                );

                // Drag scrollbar behavior
                drag_thumb(
                    &mut tester,
                    Rect::from_ltrb(785.0, 10.0, 795.0, 97.0),
                    Radius::circular(6.0),
                    RED.to_argb32(),
                );

                // Hover scrollbar behavior
                hover_at(&mut tester, Offset::new(794.0, 15.0));
                tester.pump_and_settle();

                let drawn = painted(&tester);
                assert_track(
                    &tester,
                    &drawn,
                    Rect::from_ltrb(770.0, 0.0, 800.0, 600.0),
                    BLACK.to_argb32(),
                    (Offset::new(770.0, 0.0), Offset::new(770.0, 600.0)),
                    YELLOW.to_argb32(),
                );
                // Scrollbar thumb is larger; hover color
                assert_thumb_rrect(
                    &tester,
                    Rect::from_ltrb(775.0, 20.0, 795.0, 107.0),
                    Radius::circular(6.0),
                    BLUE.to_argb32(),
                );
            });
        }
    }

    #[test]
    fn scrollbar_uses_values_from_scrollbar_theme_if_exists_instead_of_values_from_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = ThemeData::builder()
                .scrollbar_theme(scrollbar_theme())
                .build(&tester.cell.borrow());
            pump_frame(
                &mut tester,
                theme,
                |scrollbar| scrollbar,
                |child| {
                    ScrollbarTheme::new(
                        scrollbar_theme()
                            .copy_with()
                            .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                                Color::new(0xFF000000),
                            ))),
                        child,
                    )
                    .into_widget()
                },
            );
            tester.pump_and_settle();
            // Idle scrollbar behavior
            assert_thumb_rrect(
                &tester,
                Rect::from_ltrb(785.0, 10.0, 795.0, 97.0),
                Radius::circular(6.0),
                0xFF000000,
            );
        });
    }

    #[test]
    fn scrollbar_theme_can_disable_gestures() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Fuchsia);
            let theme = ThemeData::builder()
                .use_material3(false)
                .scrollbar_theme(ScrollbarThemeData::new().interactive(false))
                .build(&tester.cell.borrow());
            let controller =
                ScrollController::default(&mut tester.cell.borrow_mut()).as_controller();
            tester.pump_widget(material_app(
                theme,
                Scrollbar::new(scroll_view(Some(controller), 4000.0, 4000.0))
                    .thumb_visibility(true)
                    .controller(controller)
                    .into_widget(),
            ));
            tester.pump_and_settle();
            // Idle scrollbar behavior
            assert_thumb_rrect(
                &tester,
                K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                K_DEFAULT_THUMB_RADIUS,
                K_DEFAULT_IDLE_THUMB_COLOR,
            );

            // Try to drag scrollbar.
            drag_thumb(
                &mut tester,
                K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                K_DEFAULT_THUMB_RADIUS,
                K_DEFAULT_IDLE_THUMB_COLOR,
            );
            // Expect no change
            assert_thumb_rrect(
                &tester,
                K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                K_DEFAULT_THUMB_RADIUS,
                K_DEFAULT_IDLE_THUMB_COLOR,
            );
        });
    }

    #[test]
    fn scrollbar_interactive_takes_priority_over_scrollbar_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Fuchsia);
            let theme = ThemeData::builder()
                .use_material3(false)
                .scrollbar_theme(ScrollbarThemeData::new().interactive(false))
                .build(&tester.cell.borrow());
            let controller =
                ScrollController::default(&mut tester.cell.borrow_mut()).as_controller();
            tester.pump_widget(material_app(
                theme,
                Scrollbar::new(scroll_view(Some(controller), 4000.0, 4000.0))
                    .interactive(true)
                    .thumb_visibility(true)
                    .controller(controller)
                    .into_widget(),
            ));
            tester.pump_and_settle();
            // Idle scrollbar behavior
            assert_thumb_rrect(
                &tester,
                K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                K_DEFAULT_THUMB_RADIUS,
                K_DEFAULT_IDLE_THUMB_COLOR,
            );

            // Drag scrollbar.
            drag_thumb(
                &mut tester,
                K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                K_DEFAULT_THUMB_RADIUS,
                K_DEFAULT_DRAG_THUMB_COLOR,
            );
            // Gestures handled by Scrollbar.
            assert_thumb_rrect(
                &tester,
                Rect::from_ltrb(790.0, 10.0, 798.0, 100.0),
                K_DEFAULT_THUMB_RADIUS,
                K_DEFAULT_IDLE_THUMB_COLOR,
            );
        });
    }

    #[test]
    fn scrollbar_widget_properties_take_priority_over_theme() {
        for platform in DESKTOP_AND_FUCHSIA {
            with_main_thread_stack(|| {
                const THICKNESS: f64 = 4.0;
                const EDGE_MARGIN: f64 = 2.0;
                const RADIUS: Radius = Radius::circular(3.0);
                let mut tester = tester_on(platform);
                let theme = ThemeData::builder()
                    .color_scheme(ColorScheme::light())
                    .scrollbar_theme(
                        ScrollbarThemeData::new().track_visibility(hovered_track_visibility()),
                    )
                    .build(&tester.cell.borrow());
                pump_frame(
                    &mut tester,
                    theme,
                    |scrollbar| scrollbar.thickness(THICKNESS).radius(RADIUS),
                    |child| child,
                );

                tester.pump_and_settle();
                // Idle scrollbar behavior
                let thumb = Rect::from_ltrb(800.0 - THICKNESS - EDGE_MARGIN, 0.0, 798.0, 90.0);
                assert_thumb_rrect(
                    &tester,
                    thumb,
                    Radius::circular(3.0),
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                // Drag scrollbar behavior.
                drag_thumb(
                    &mut tester,
                    thumb,
                    Radius::circular(3.0),
                    K_DEFAULT_DRAG_THUMB_COLOR,
                );

                // Hover scrollbar behavior.
                hover_at(&mut tester, Offset::new(794.0, 5.0));
                tester.pump_and_settle();

                let drawn = painted(&tester);
                assert_track(
                    &tester,
                    &drawn,
                    Rect::from_ltrb(792.0, 0.0, 800.0, 600.0),
                    0x08000000,
                    (Offset::new(792.0, 0.0), Offset::new(792.0, 600.0)),
                    0x1a000000,
                );
                // Hover color
                assert_thumb_rrect(
                    &tester,
                    Rect::from_ltrb(800.0 - THICKNESS - EDGE_MARGIN, 10.0, 798.0, 100.0),
                    Radius::circular(3.0),
                    0x80000000,
                );
            });
        }
    }

    #[test]
    fn theme_data_color_scheme_is_used_when_no_scrollbar_theme_is_set() {
        for platform in DESKTOP_AND_FUCHSIA {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let build_frame = |tester: &mut WidgetTester, color_scheme: ColorScheme| {
                    let theme = ThemeData::builder()
                        .color_scheme(color_scheme)
                        .build(&tester.cell.borrow())
                        .copy_with()
                        .scrollbar_theme(
                            ScrollbarThemeData::new().track_visibility(hovered_track_visibility()),
                        )
                        .build();
                    pump_frame(tester, theme, |scrollbar| scrollbar, |child| child);
                };

                // Scrollbar defaults for light themes:
                // - coloring based on ColorScheme.onSurface
                build_frame(&mut tester, ColorScheme::light());
                tester.pump_and_settle();
                // Idle scrollbar behavior
                assert_thumb_rrect(
                    &tester,
                    K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                // Drag scrollbar behavior
                drag_thumb(
                    &mut tester,
                    K_MATERIAL_DESIGN_INITIAL_THUMB_RECT,
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_DRAG_THUMB_COLOR,
                );

                // Hover scrollbar behavior
                let mut hover_gesture = hover_at(&mut tester, Offset::new(794.0, 5.0));
                tester.pump_and_settle();

                let drawn = painted(&tester);
                assert_track(
                    &tester,
                    &drawn,
                    Rect::from_ltrb(784.0, 0.0, 800.0, 600.0),
                    0x08000000,
                    (Offset::new(784.0, 0.0), Offset::new(784.0, 600.0)),
                    0x1a000000,
                );
                // Scrollbar thumb is larger; hover color
                assert_thumb_rrect(
                    &tester,
                    Rect::from_ltrb(786.0, 10.0, 798.0, 100.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0x80000000,
                );

                tester.move_to(&mut hover_gesture, Offset::ZERO, false);

                // Scrollbar defaults for dark themes:
                // - coloring slightly different based on ColorScheme.onSurface
                build_frame(&mut tester, ColorScheme::dark());
                tester.pump_and_settle(); // Theme change animation

                // Idle scrollbar behavior
                assert_thumb_rrect(
                    &tester,
                    Rect::from_ltrb(790.0, 10.0, 798.0, 100.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0x4dffffff,
                );

                // Drag scrollbar behavior
                drag_thumb(
                    &mut tester,
                    Rect::from_ltrb(790.0, 10.0, 798.0, 100.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0xbfffffff,
                );

                // Hover scrollbar behavior
                tester.move_to(&mut hover_gesture, Offset::new(794.0, 5.0), false);
                tester.pump_and_settle();

                let drawn = painted(&tester);
                assert_track(
                    &tester,
                    &drawn,
                    Rect::from_ltrb(784.0, 0.0, 800.0, 600.0),
                    0x0dffffff,
                    (Offset::new(784.0, 0.0), Offset::new(784.0, 600.0)),
                    0x40ffffff,
                );
                // Scrollbar thumb is larger; hover color
                assert_thumb_rrect(
                    &tester,
                    Rect::from_ltrb(786.0, 20.0, 798.0, 110.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0xa6ffffff,
                );
            });
        }
    }

    #[test]
    fn scrollbar_theme_data_track_visibility_test() {
        for platform in DESKTOP_AND_FUCHSIA {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow())
                    .copy_with()
                    .scrollbar_theme(scrollbar_theme().track_visibility(<dyn WidgetStateProperty<
                        Option<bool>,
                    >>::resolve_with(
                        |_states: &WidgetStates| Some(true),
                    )))
                    .build();
                pump_frame(&mut tester, theme, |scrollbar| scrollbar, |child| child);
                tester.pump_and_settle();

                let drawn = painted(&tester);
                assert!(
                    matches!(drawn.as_slice(), [
                        Drawn::Rect(0x08000000, _),
                        Drawn::Path(0x1a000000, _),
                        Drawn::Path(thumb, _),
                    ] if *thumb == GREEN.to_argb32()),
                    "{drawn:?}"
                );
            });
        }
    }

    /// `scrollbar_theme_test.dart`'s `_scrollbarTheme()` with its defaults.
    fn scrollbar_theme() -> ScrollbarThemeData {
        ScrollbarThemeData::new()
            .thickness(<dyn WidgetStateProperty<Option<f64>>>::resolve_with(
                get_thickness,
            ))
            .track_visibility(hovered_track_visibility())
            .radius(Radius::circular(6.0))
            .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                get_thumb_color,
            ))
            .track_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                get_track_color,
            ))
            .track_border_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                get_track_border_color,
            ))
            .cross_axis_margin(5.0)
            .main_axis_margin(10.0)
            .min_thumb_length(50.0)
    }

    fn get_thickness(states: &WidgetStates) -> Option<f64> {
        if states.contains(&WidgetState::Hovered) {
            return Some(20.0);
        }
        Some(10.0)
    }

    fn get_thumb_color(states: &WidgetStates) -> Option<Color> {
        if states.contains(&WidgetState::Dragged) {
            return Some(RED);
        }
        if states.contains(&WidgetState::Hovered) {
            return Some(BLUE);
        }
        Some(GREEN)
    }

    fn get_track_color(states: &WidgetStates) -> Option<Color> {
        if states.contains(&WidgetState::Hovered) {
            return Some(BLACK);
        }
        None
    }

    fn get_track_border_color(states: &WidgetStates) -> Option<Color> {
        if states.contains(&WidgetState::Hovered) {
            return Some(YELLOW);
        }
        None
    }
}
