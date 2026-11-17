//! Flutter counterpart: material_ui's `navigation_drawer.dart` (and its part
//! `generated/navigation_drawer_defaults_m3.g.dart`).

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{AnimationBehavior, AnimationController, AnimationRef, AnyAnimation};
use waft_embedder::Color;
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref, ValueChanged};
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, ShapeBorderRef, TextStyle};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    BuildContext, Column, DefaultTextStyle, Expanded, IconTheme, IconThemeData, InheritedWidget,
    IntoWidget, KeyRef, ListView, Padding, Row, SafeArea, Semantics,
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, SizedBox, Stack, State,
    StateData, StatefulWidget, StatelessWidget, StatusTransitionWidget, TransitionBuilder,
    WidgetBuilder, WidgetRef, WidgetState, WidgetStates, downcast_widget,
};

use crate::colors::Colors;
use crate::drawer::Drawer;
use crate::generated::navigation_drawer_defaults_m3::NavigationDrawerDefaultsM3;
use crate::ink_decoration::Ink;
use crate::ink_well::InkWell;
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::navigation_bar::NavigationIndicator;
use crate::navigation_drawer_theme::{NavigationDrawerTheme, NavigationDrawerThemeDataGetters};

/// Material Design Navigation Drawer component.
///
/// On top of `Drawer`s, Navigation drawers offer a persistent and convenient way to switch
/// between primary destinations in an app.
///
/// The style for the icons and text are not affected by parent
/// `DefaultTextStyle`s or `IconTheme`s but rather controlled by parameters or
/// the `NavigationDrawerThemeData`.
///
/// The [`children`](Self::children) are a list of widgets to be displayed in the drawer.
/// These can be a mixture of any widgets, but there is special handling for
/// [`NavigationDrawerDestination`]s. They are treated as a group and when one is selected,
/// the [`on_destination_selected`](Self::on_destination_selected) is called with the index
/// into the group that corresponds to the selected destination.
///
/// See also:
///
///  * `Scaffold::drawer`, where one specifies a `Drawer` so that it can be
///    shown.
///  * `Scaffold::of`, to obtain the current `ScaffoldState`, which manages the
///    display and animation of the drawer.
///  * `ScaffoldState::open_drawer`, which displays its `Drawer`, if any.
///  * <https://material.io/design/components/navigation-drawer.html>
pub struct NavigationDrawer {
    pub key: Option<KeyRef>,

    /// The background color of the `Material` that holds the [`NavigationDrawer`]'s
    /// contents.
    ///
    /// If this is null, then `NavigationDrawerThemeData::background_color` is used.
    /// If that is also null, then it falls back to `ColorScheme::surface_container_low`.
    pub background_color: Option<Color>,

    /// The color used for the drop shadow to indicate elevation.
    ///
    /// If null, `NavigationDrawerThemeData::shadow_color` is used. If that
    /// is also null, the default value is `Colors::TRANSPARENT` which
    /// indicates that no drop shadow will be displayed.
    ///
    /// See `Material::shadow_color` for more details on drop shadows.
    pub shadow_color: Option<Color>,

    /// The surface tint of the `Material` that holds the [`NavigationDrawer`]'s
    /// contents.
    ///
    /// This is not recommended for use. [Material 3 spec](https://m3.material.io/styles/color/the-color-system/color-roles)
    /// introduced a set of tone-based surfaces and surface containers in its `ColorScheme`,
    /// which provide more flexibility. The intention is to eventually remove surface tint color from
    /// the framework.
    ///
    /// If this is null, then `NavigationDrawerThemeData::surface_tint_color` is used.
    /// If that is also null, the default value is `Colors::TRANSPARENT`.
    pub surface_tint_color: Option<Color>,

    /// The elevation of the [`NavigationDrawer`] itself.
    ///
    /// If null, `NavigationDrawerThemeData::elevation` is used. If that
    /// is also null, it will be 1.0.
    pub elevation: Option<f64>,

    /// The color of the [`indicator_shape`](Self::indicator_shape) when this destination is
    /// selected.
    ///
    /// If this is null, `NavigationDrawerThemeData::indicator_color` is used.
    /// If that is also null, defaults to `ColorScheme::secondary_container`.
    pub indicator_color: Option<Color>,

    /// The shape of the selected indicator.
    ///
    /// If this is null, `NavigationDrawerThemeData::indicator_shape` is used.
    /// If that is also null, defaults to `StadiumBorder`.
    pub indicator_shape: Option<ShapeBorderRef>,

    /// Defines the appearance of the items within the navigation drawer.
    ///
    /// The list contains [`NavigationDrawerDestination`] widgets and/or customized
    /// widgets like headlines and dividers.
    pub children: Vec<WidgetRef>,

    /// A widget to display at the top of the layout.
    ///
    /// Typically used for titles, navigation bars, or other header content.
    pub header: Option<WidgetRef>,

    /// A widget to display at the bottom of the layout.
    ///
    /// Typically used for actions, navigation controls, or other footer content.
    pub footer: Option<WidgetRef>,

    /// The index into destinations for the current selected
    /// [`NavigationDrawerDestination`] or null if no destination is selected.
    ///
    /// A valid [`selected_index`](Self::selected_index) satisfies
    /// 0 <= [`selected_index`](Self::selected_index) < number of
    /// [`NavigationDrawerDestination`]. For an invalid
    /// [`selected_index`](Self::selected_index) like `-1`, all destinations will appear
    /// unselected.
    pub selected_index: Option<i32>,

    /// Called when one of the [`NavigationDrawerDestination`] children is selected.
    ///
    /// This callback usually updates the int passed to
    /// [`selected_index`](Self::selected_index).
    ///
    /// Upon updating [`selected_index`](Self::selected_index), the [`NavigationDrawer`] will be
    /// rebuilt.
    pub on_destination_selected: Option<ValueChanged<i32>>,

    /// Defines the padding for [`NavigationDrawerDestination`] widgets (Drawer items).
    ///
    /// Defaults to `EdgeInsets.symmetric(horizontal: 12.0)`.
    pub tile_padding: EdgeInsetsGeometry,
}

impl NavigationDrawer {
    /// Creates a Material Design Navigation Drawer component.
    pub fn new(children: impl IntoIterator<Item = WidgetRef>) -> NavigationDrawer {
        NavigationDrawer {
            key: None,
            background_color: None,
            shadow_color: None,
            surface_tint_color: None,
            elevation: None,
            indicator_color: None,
            indicator_shape: None,
            children: children.into_iter().collect(),
            header: None,
            footer: None,
            selected_index: Some(0),
            on_destination_selected: None,
            tile_padding: EdgeInsetsGeometry::symmetric(0.0, 12.0),
        }
    }

    /// Dart `NavigationDrawer(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationDrawer {
        self.key = Some(key);
        self
    }

    /// Dart `NavigationDrawer(header:)`.
    pub fn header<K>(mut self, header: impl IntoWidget<K>) -> NavigationDrawer {
        self.header = Some(header.into_widget());
        self
    }

    /// Dart `NavigationDrawer(footer:)`.
    pub fn footer<K>(mut self, footer: impl IntoWidget<K>) -> NavigationDrawer {
        self.footer = Some(footer.into_widget());
        self
    }

    /// Dart `NavigationDrawer(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> NavigationDrawer {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `NavigationDrawer(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> NavigationDrawer {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `NavigationDrawer(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> NavigationDrawer {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `NavigationDrawer(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> NavigationDrawer {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `NavigationDrawer(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> NavigationDrawer {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `NavigationDrawer(indicatorShape:)`.
    pub fn indicator_shape(
        mut self,
        indicator_shape: impl Into<ShapeBorderRef>,
    ) -> NavigationDrawer {
        self.indicator_shape = Some(indicator_shape.into());
        self
    }

    /// Dart `NavigationDrawer(onDestinationSelected:)`.
    pub fn on_destination_selected(
        mut self,
        on_destination_selected: ValueChanged<i32>,
    ) -> NavigationDrawer {
        self.on_destination_selected = Some(on_destination_selected);
        self
    }

    /// Dart `NavigationDrawer(selectedIndex:)`; `None` selects no destination.
    pub fn selected_index(mut self, selected_index: Option<i32>) -> NavigationDrawer {
        self.selected_index = selected_index;
        self
    }

    /// Dart `NavigationDrawer(tilePadding:)`.
    pub fn tile_padding(mut self, tile_padding: impl Into<EdgeInsetsGeometry>) -> NavigationDrawer {
        self.tile_padding = tile_padding.into();
        self
    }
}

impl Debug for NavigationDrawer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationDrawer")
            .field("background_color", &self.background_color)
            .field("shadow_color", &self.shadow_color)
            .field("surface_tint_color", &self.surface_tint_color)
            .field("elevation", &self.elevation)
            .field("indicator_color", &self.indicator_color)
            .field("indicator_shape", &self.indicator_shape)
            .field("children", &self.children)
            .field("header", &self.header)
            .field("footer", &self.footer)
            .field("selected_index", &self.selected_index)
            .field("tile_padding", &self.tile_padding)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for NavigationDrawer {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let total_number_of_destinations = self
            .children
            .iter()
            .filter(|child| downcast_widget::<NavigationDrawerDestination>(&***child).is_some())
            .count() as i32;

        let mut destination_index = 0;
        let wrap_child = |child: WidgetRef, index: i32| {
            let indicator_color = self.indicator_color;
            let indicator_shape = self.indicator_shape.clone();
            let tile_padding = self.tile_padding;
            let on_destination_selected = self.on_destination_selected.clone();
            SelectableAnimatedBuilder::new(
                Some(index) == self.selected_index,
                move |_app, _context, animation| {
                    let on_destination_selected = on_destination_selected.clone();
                    NavigationDrawerDestinationInfo {
                        index,
                        total_number_of_destinations,
                        selected_animation: animation,
                        indicator_color,
                        indicator_shape: indicator_shape.clone(),
                        on_tap: Listener::new(move |app| {
                            if let Some(on_destination_selected) = &on_destination_selected {
                                on_destination_selected(app, index);
                            }
                        }),
                        child: child.clone(),
                        tile_padding,
                    }
                    .into_widget()
                },
            )
            .duration(Duration::from_millis(500))
            .into_widget()
        };

        let wrapped_children: Vec<WidgetRef> = self
            .children
            .iter()
            .map(|child| {
                if downcast_widget::<NavigationDrawerDestination>(&**child).is_none() {
                    child.clone()
                } else {
                    let index = destination_index;
                    destination_index += 1;
                    wrap_child(child.clone(), index)
                }
            })
            .collect();
        let navigation_drawer_theme = NavigationDrawerTheme::of(app, context);

        let mut drawer = Drawer::new().child(
            SafeArea::new(
                Column::new().children(
                    self.header
                        .clone()
                        .into_iter()
                        .chain([Expanded::new(
                            Material::new()
                                .r#type(MaterialType::Transparency)
                                .child(ListView::new().children(wrapped_children)),
                        )
                        .into_widget()])
                        .chain(self.footer.clone()),
                ),
            )
            .bottom(false),
        );
        drawer.background_color = self
            .background_color
            .or(navigation_drawer_theme.background_color);
        drawer.shadow_color = self.shadow_color.or(navigation_drawer_theme.shadow_color);
        drawer.surface_tint_color = self
            .surface_tint_color
            .or(navigation_drawer_theme.surface_tint_color);
        drawer.elevation = self.elevation.or(navigation_drawer_theme.elevation);
        drawer.into_widget()
    }
}

/// A Material Design [`NavigationDrawer`] destination.
///
/// Displays an icon with a label, for use in [`NavigationDrawer::children`].
pub struct NavigationDrawerDestination {
    pub key: Option<KeyRef>,

    /// The background color of the destination.
    ///
    /// If this is null, no background is set and [`NavigationDrawer::background_color`] will
    /// be visible.
    ///
    /// This is the background color of the whole rectangular area behind the drawer
    /// destination. To customize only the indicator color consider using
    /// [`NavigationDrawer::indicator_color`].
    pub background_color: Option<Color>,

    /// The `Widget` (usually an `Icon`) that's displayed for this
    /// `NavigationDestination`.
    ///
    /// The icon will use `NavigationDrawerThemeData::icon_theme`. If this is
    /// null, the default `IconThemeData` would use a size of 24.0 and
    /// `ColorScheme::on_surface_variant`.
    pub icon: WidgetRef,

    /// The optional `Widget` (usually an `Icon`) that's displayed when this
    /// `NavigationDestination` is selected.
    ///
    /// If [`selected_icon`](Self::selected_icon) is non-null, the destination will fade from
    /// [`icon`](Self::icon) to [`selected_icon`](Self::selected_icon) when this destination
    /// goes from unselected to selected.
    ///
    /// The icon will use `NavigationDrawerThemeData::icon_theme` with
    /// `WidgetState::Selected`. If this is null, the default `IconThemeData`
    /// would use a size of 24.0 and `ColorScheme::on_secondary_container`.
    pub selected_icon: Option<WidgetRef>,

    /// The text label that appears on the right of the icon
    ///
    /// The accompanying `Text` widget will use
    /// `NavigationDrawerThemeData::label_text_style`. If this are null, the default
    /// text style would use `TextTheme::label_large` with `ColorScheme::on_surface_variant`.
    pub label: WidgetRef,

    /// Indicates that this destination is selectable.
    ///
    /// Defaults to true.
    pub enabled: bool,
}

impl NavigationDrawerDestination {
    /// Creates a navigation drawer destination.
    pub fn new<I, L>(
        icon: impl IntoWidget<I>,
        label: impl IntoWidget<L>,
    ) -> NavigationDrawerDestination {
        NavigationDrawerDestination {
            key: None,
            background_color: None,
            icon: icon.into_widget(),
            selected_icon: None,
            label: label.into_widget(),
            enabled: true,
        }
    }

    /// Dart `NavigationDrawerDestination(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationDrawerDestination {
        self.key = Some(key);
        self
    }

    /// Dart `NavigationDrawerDestination(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> NavigationDrawerDestination {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `NavigationDrawerDestination(selectedIcon:)`.
    pub fn selected_icon<K>(
        mut self,
        selected_icon: impl IntoWidget<K>,
    ) -> NavigationDrawerDestination {
        self.selected_icon = Some(selected_icon.into_widget());
        self
    }

    /// Dart `NavigationDrawerDestination(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> NavigationDrawerDestination {
        self.enabled = enabled;
        self
    }
}

impl Debug for NavigationDrawerDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationDrawerDestination")
            .field("background_color", &self.background_color)
            .field("icon", &self.icon)
            .field("selected_icon", &self.selected_icon)
            .field("label", &self.label)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl StatelessWidget for NavigationDrawerDestination {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let selected_state = WidgetStates::from([WidgetState::Selected]);
        let unselected_state = WidgetStates::new();
        let disabled_state = WidgetStates::from([WidgetState::Disabled]);

        let navigation_drawer_theme = NavigationDrawerTheme::of(app, context);
        let defaults = Rc::new(NavigationDrawerDefaultsM3::new(context));

        let animation = NavigationDrawerDestinationInfo::of(app, context).selected_animation;
        let icon_animation = animation.clone();

        let enabled = self.enabled;
        let icon = self.icon.clone();
        let selected_icon = self.selected_icon.clone();
        let label = self.label.clone();
        let (icon_theme, label_theme) = (navigation_drawer_theme.clone(), navigation_drawer_theme);
        let (icon_defaults, label_defaults) = (Rc::clone(&defaults), defaults);
        let (icon_selected, icon_unselected, icon_disabled) = (
            selected_state.clone(),
            unselected_state.clone(),
            disabled_state.clone(),
        );
        NavigationDestinationBuilder {
            build_icon: Rc::new(move |app, _context| {
                let resolve = |app: &mut App, states: &WidgetStates| -> IconThemeData {
                    icon_theme
                        .icon_theme
                        .as_ref()
                        .and_then(|icon_theme| icon_theme.resolve(states))
                        .unwrap_or_else(|| {
                            icon_defaults
                                .icon_theme(app)
                                .expect("the defaults give an icon theme")
                                .resolve(states)
                                .expect("the defaults resolve every state")
                        })
                };
                let selected_icon_widget = IconTheme::merge(
                    None,
                    resolve(
                        app,
                        if enabled {
                            &icon_selected
                        } else {
                            &icon_disabled
                        },
                    ),
                    selected_icon.clone().unwrap_or_else(|| icon.clone()),
                );
                let unselected_icon_widget = IconTheme::merge(
                    None,
                    resolve(
                        app,
                        if enabled {
                            &icon_unselected
                        } else {
                            &icon_disabled
                        },
                    ),
                    icon.clone(),
                );

                if icon_animation.is_forward_or_completed(app) {
                    selected_icon_widget
                } else {
                    unselected_icon_widget
                }
            }),
            build_label: Rc::new(move |app, _context| {
                let resolve = |app: &mut App, states: &WidgetStates| -> Option<TextStyle> {
                    label_theme
                        .label_text_style
                        .as_ref()
                        .and_then(|label_text_style| label_text_style.resolve(states))
                        .or_else(|| {
                            label_defaults
                                .label_text_style(app)
                                .expect("the defaults give a label text style")
                                .resolve(states)
                        })
                };
                let effective_selected_label_text_style = resolve(
                    app,
                    if enabled {
                        &selected_state
                    } else {
                        &disabled_state
                    },
                );
                let effective_unselected_label_text_style = resolve(
                    app,
                    if enabled {
                        &unselected_state
                    } else {
                        &disabled_state
                    },
                );

                DefaultTextStyle::new(
                    if animation.is_forward_or_completed(app) {
                        effective_selected_label_text_style.expect("a selected label text style")
                    } else {
                        effective_unselected_label_text_style
                            .expect("an unselected label text style")
                    },
                    label.clone(),
                )
                .into_widget()
            }),
            enabled: self.enabled,
            background_color: self.background_color,
        }
        .into_widget()
    }
}

/// Widget that handles the semantics and layout of a navigation drawer
/// destination.
///
/// Prefer `NavigationDestination` over this widget, as it is a simpler
/// (although less customizable) way to get navigation drawer destinations.
///
/// The icon and label of this destination are built with
/// [`build_icon`](Self::build_icon) and [`build_label`](Self::build_label). They should build
/// the unselected and selected icon and label according to
/// [`NavigationDrawerDestinationInfo::selected_animation`], where an animation value of 0 is
/// unselected and 1 is selected.
///
/// See `NavigationDestination` for an example.
struct NavigationDestinationBuilder {
    /// Builds the icon for a destination in a [`NavigationDrawer`].
    ///
    /// To animate between unselected and selected, build the icon based on
    /// [`NavigationDrawerDestinationInfo::selected_animation`]. When the animation is 0,
    /// the destination is unselected, when the animation is 1, the destination is
    /// selected.
    ///
    /// The destination is considered selected as soon as the animation is
    /// increasing or completed, and it is considered unselected as soon as the
    /// animation is decreasing or dismissed.
    build_icon: WidgetBuilder,

    /// Builds the label for a destination in a [`NavigationDrawer`].
    ///
    /// To animate between unselected and selected, build the icon based on
    /// [`NavigationDrawerDestinationInfo::selected_animation`]. When the animation is
    /// 0, the destination is unselected, when the animation is 1, the destination
    /// is selected.
    ///
    /// The destination is considered selected as soon as the animation is
    /// increasing or completed, and it is considered unselected as soon as the
    /// animation is decreasing or dismissed.
    build_label: WidgetBuilder,

    /// Indicates that this destination is selectable.
    ///
    /// Defaults to true.
    enabled: bool,

    /// The background color of the destination.
    background_color: Option<Color>,
}

impl Debug for NavigationDestinationBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_NavigationDestinationBuilder")
            .field("enabled", &self.enabled)
            .field("background_color", &self.background_color)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for NavigationDestinationBuilder {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let info = NavigationDrawerDestinationInfo::of(app, context);
        let navigation_drawer_theme = NavigationDrawerTheme::of(app, context);
        let defaults = NavigationDrawerDefaultsM3::new(context);

        let indicator_shape = |app: &mut App| {
            info.indicator_shape
                .clone()
                .or_else(|| navigation_drawer_theme.indicator_shape.clone())
                .or_else(|| defaults.indicator_shape(app))
                .expect("the defaults give an indicator shape")
        };
        let indicator_size = navigation_drawer_theme
            .indicator_size
            .or_else(|| defaults.indicator_size(app))
            .expect("the defaults give an indicator size");
        let mut ink_well = InkWell::new()
            .highlight_color(Colors::TRANSPARENT)
            .custom_border(indicator_shape(app));
        if self.enabled {
            ink_well = ink_well.on_tap(info.on_tap.clone());
        }
        let indicator_color = info
            .indicator_color
            .or(navigation_drawer_theme.indicator_color)
            .or_else(|| defaults.indicator_color(app))
            .expect("the defaults give an indicator color");
        let indicator_shape = indicator_shape(app);
        let ink_well = ink_well.child(
            Stack::new().alignment(AlignmentGeometry::CENTER).children([
                NavigationIndicator::new(info.selected_animation)
                    .color(indicator_color)
                    .shape(indicator_shape)
                    .width(indicator_size.width())
                    .height(indicator_size.height())
                    .into_widget(),
                Row::new()
                    .children([
                        SizedBox::new().width(16.0).into_widget(),
                        (self.build_icon)(app, context),
                        SizedBox::new().width(12.0).into_widget(),
                        (self.build_label)(app, context),
                    ])
                    .into_widget(),
            ]),
        );

        let mut tile = SizedBox::new().child(ink_well);
        tile.height = navigation_drawer_theme
            .tile_height
            .or_else(|| defaults.tile_height(app));
        let destination = Padding::new(info.tile_padding)
            .child(NavigationDestinationSemantics {
                child: tile.into_widget(),
            })
            .into_widget();

        if let Some(background_color) = self.background_color {
            return Ink::new()
                .color(background_color)
                .child(destination)
                .into_widget();
        }
        destination
    }
}

/// Semantics widget for a navigation drawer destination.
///
/// Requires a [`NavigationDrawerDestinationInfo`] parent (normally provided by the
/// [`NavigationDrawer`] by default).
///
/// Provides localized semantic labels to the destination, for example, it will
/// read "Home, Tab 1 of 3".
///
/// Used by [`NavigationDestinationBuilder`].
#[derive(Debug)]
struct NavigationDestinationSemantics {
    /// The widget that should receive the destination semantics.
    child: WidgetRef,
}

impl StatelessWidget for NavigationDestinationSemantics {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let destination_info = NavigationDrawerDestinationInfo::of(app, context);
        let selected_animation = destination_info.selected_animation.clone();
        // The AnimationStatusBuilder will make sure that the semantics update to
        // "selected" when the animation status changes.
        StatusTransitionWidgetBuilder::new(
            selected_animation.clone(),
            move |app, _context, child| {
                let mut semantics = Semantics::new()
                    .selected(selected_animation.is_forward_or_completed(app))
                    .container(true);
                if let Some(child) = child {
                    semantics = semantics.child(child.clone());
                }
                semantics.into_widget()
            },
        )
        .child(
            Stack::new().alignment(AlignmentGeometry::CENTER).children([
                self.child.clone(),
                Semantics::new()
                    .label(localizations.tab_label(
                        destination_info.index + 1,
                        destination_info.total_number_of_destinations,
                    ))
                    .into_widget(),
            ]),
        )
        .into_widget()
    }
}

/// Widget that listens to an animation, and rebuilds when the animation changes
/// `AnimationStatus`.
///
/// This can be more efficient than just using an `AnimatedBuilder` when you
/// only need to rebuild when the `Animation::status` changes, since
/// `AnimatedBuilder` rebuilds every time the animation ticks.
struct StatusTransitionWidgetBuilder {
    animation: AnimationRef<f64>,

    /// Called every time the animation changes `AnimationStatus`.
    builder: TransitionBuilder,

    /// The child widget to pass to the [`builder`](Self::builder).
    ///
    /// If a [`builder`](Self::builder) callback's return value contains a subtree that does
    /// not depend on the animation, it's more efficient to build that subtree once
    /// instead of rebuilding it on every animation status change.
    ///
    /// Using this pre-built child is entirely optional, but can improve
    /// performance in some cases and is therefore a good practice.
    ///
    /// See: `AnimatedBuilder::child`
    child: Option<WidgetRef>,
}

impl StatusTransitionWidgetBuilder {
    /// Creates a widget that rebuilds when the given animation changes status.
    fn new(
        animation: AnimationRef<f64>,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> StatusTransitionWidgetBuilder {
        StatusTransitionWidgetBuilder {
            animation,
            builder: Rc::new(builder),
            child: None,
        }
    }

    /// Dart `_StatusTransitionWidgetBuilder(child:)`.
    fn child<K>(mut self, child: impl IntoWidget<K>) -> StatusTransitionWidgetBuilder {
        self.child = Some(child.into_widget());
        self
    }
}

impl Debug for StatusTransitionWidgetBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_StatusTransitionWidgetBuilder")
            .field("animation", &self.animation)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatusTransitionWidget for StatusTransitionWidgetBuilder {
    fn animation(&self) -> AnyAnimation<f64> {
        self.animation.handle()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        (self.builder)(app, context, self.child.as_ref())
    }
}

/// Inherited widget for passing data from the [`NavigationDrawer`] to the
/// [`NavigationDrawerDestination`] child widgets.
///
/// Useful for building navigation destinations using:
/// `NavigationDrawerDestinationInfo::of(app, context)`.
#[derive(Clone)]
struct NavigationDrawerDestinationInfo {
    /// Which destination index is this in the navigation drawer.
    ///
    /// For example:
    ///
    /// ```text
    /// NavigationDrawer::new([
    ///     Text::new("Headline").into_widget(), // This doesn't have index.
    ///     // This is destination index 0.
    ///     NavigationDrawerDestination::new(Icon::new(Some(Icons::surfing())), Text::new("Surfing")).into_widget(),
    ///     // This is destination index 1.
    ///     NavigationDrawerDestination::new(Icon::new(Some(Icons::support())), Text::new("Support")).into_widget(),
    ///     // This is destination index 2.
    ///     NavigationDrawerDestination::new(Icon::new(Some(Icons::local_hospital())), Text::new("Hospital")).into_widget(),
    /// ])
    /// ```
    ///
    /// This is required for semantics, so that each destination can have a label
    /// "Tab 1 of 3", for example.
    index: i32,

    /// How many total destinations are in this navigation drawer.
    ///
    /// This is required for semantics, so that each destination can have a label
    /// "Tab 1 of 4", for example.
    total_number_of_destinations: i32,

    /// Indicates whether or not this destination is selected, from 0 (unselected)
    /// to 1 (selected).
    selected_animation: AnimationRef<f64>,

    /// The color of the indicator.
    ///
    /// This is used by destinations to override the indicator color.
    indicator_color: Option<Color>,

    /// The shape of the indicator.
    ///
    /// This is used by destinations to override the indicator shape.
    indicator_shape: Option<ShapeBorderRef>,

    /// The callback that should be called when this destination is tapped.
    ///
    /// This is computed by calling [`NavigationDrawer::on_destination_selected`]
    /// with [`index`](Self::index) passed in.
    on_tap: Listener,

    child: WidgetRef,

    /// Defines the padding for [`NavigationDrawerDestination`] widgets (Drawer items).
    ///
    /// Defaults to `EdgeInsets.symmetric(horizontal: 12.0)`.
    tile_padding: EdgeInsetsGeometry,
}

impl NavigationDrawerDestinationInfo {
    /// Returns a non null [`NavigationDrawerDestinationInfo`].
    ///
    /// This will return an error if called with no [`NavigationDrawerDestinationInfo`]
    /// ancestor.
    ///
    /// Used by widgets that are implementing a navigation destination info to
    /// get information like the selected animation and destination number.
    fn of(app: &mut App, context: BuildContext) -> NavigationDrawerDestinationInfo {
        context
            .depend_on_inherited_widget_of_exact_type::<NavigationDrawerDestinationInfo>(app)
            .cloned()
            .expect(
                "Navigation destinations need a _NavigationDrawerDestinationInfo parent, which \
                 is usually provided by NavigationDrawer.",
            )
    }
}

impl Debug for NavigationDrawerDestinationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_NavigationDrawerDestinationInfo")
            .field("index", &self.index)
            .field(
                "total_number_of_destinations",
                &self.total_number_of_destinations,
            )
            .field("selected_animation", &self.selected_animation)
            .field("indicator_color", &self.indicator_color)
            .field("indicator_shape", &self.indicator_shape)
            .field("child", &self.child)
            .field("tile_padding", &self.tile_padding)
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for NavigationDrawerDestinationInfo {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &NavigationDrawerDestinationInfo) -> bool {
        self.index != old_widget.index
            || self.total_number_of_destinations != old_widget.total_number_of_destinations
            || self.selected_animation != old_widget.selected_animation
            || self.on_tap != old_widget.on_tap
    }
}

/// Dart's `Widget Function(BuildContext, Animation<double>)` builder of
/// [`SelectableAnimatedBuilder`].
type AnimationWidgetBuilder = Rc<dyn Fn(&mut App, BuildContext, AnimationRef<f64>) -> WidgetRef>;

// Builder widget for widgets that need to be animated from 0 (unselected) to
// 1.0 (selected).
//
// This widget creates and manages an `AnimationController` that it passes down
// to the child through the `builder` function.
//
// When `is_selected` is `true`, the animation controller will animate from
// 0 to 1 (for `duration` time).
//
// When `is_selected` is `false`, the animation controller will animate from
// 1 to 0 (for `duration` time).
//
// If `is_selected` is updated while the widget is animating, the animation will
// be reversed until it is either 0 or 1 again.
struct SelectableAnimatedBuilder {
    /// When true, the widget will animate an animation controller from 0 to 1.
    ///
    /// The animation controller is passed to the child widget through
    /// [`builder`](Self::builder).
    is_selected: bool,

    /// How long the animation controller should animate for when
    /// [`is_selected`](Self::is_selected) is updated.
    ///
    /// If the animation is currently running and [`is_selected`](Self::is_selected) is
    /// updated, only the [`duration`](Self::duration) left to finish the animation will be
    /// run.
    duration: Duration,

    /// Builds the child widget based on the current animation status.
    ///
    /// When [`is_selected`](Self::is_selected) is updated to true, this builder will be called
    /// and the animation will animate up to 1. When [`is_selected`](Self::is_selected) is
    /// updated to `false`, this will be called and the animation will animate down to 0.
    builder: AnimationWidgetBuilder,
}

impl SelectableAnimatedBuilder {
    /// Builds and maintains an `AnimationController` that will animate from 0 to
    /// 1 and back depending on when [`is_selected`](Self::is_selected) is true.
    fn new(
        is_selected: bool,
        builder: impl Fn(&mut App, BuildContext, AnimationRef<f64>) -> WidgetRef + 'static,
    ) -> SelectableAnimatedBuilder {
        SelectableAnimatedBuilder {
            is_selected,
            duration: Duration::from_millis(200),
            builder: Rc::new(builder),
        }
    }

    /// Dart `_SelectableAnimatedBuilder(duration:)`.
    fn duration(mut self, duration: Duration) -> SelectableAnimatedBuilder {
        self.duration = duration;
        self
    }
}

impl Debug for SelectableAnimatedBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_SelectableAnimatedBuilder")
            .field("is_selected", &self.is_selected)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectableAnimatedBuilder {
    type State = SelectableAnimatedBuilderState;

    fn create_state(&self) -> SelectableAnimatedBuilderState {
        SelectableAnimatedBuilderState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            controller: None,
        }
    }
}

/// State that manages the `AnimationController` that is passed to
/// [`SelectableAnimatedBuilder::builder`].
struct SelectableAnimatedBuilderState {
    state: StateData<SelectableAnimatedBuilder>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    controller: Option<Ref<AnimationController>>,
}

impl SelectableAnimatedBuilderState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .controller
            .handle()
            .expect("created in init_state")
    }
}

impl SingleTickerProviderStateMixin for SelectableAnimatedBuilderState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for SelectableAnimatedBuilderState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for SelectableAnimatedBuilderState {
    type Widget = SelectableAnimatedBuilder;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::new(
            app,
            None,
            None,
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller.clone());
        let widget = self.widget(app);
        let duration = widget.duration;
        let value = if widget.is_selected { 1.0 } else { 0.0 };
        app.get_mut(&controller).duration = Some(duration);
        controller.set_value(app, value);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &SelectableAnimatedBuilder,
    ) {
        let controller = self.controller(app);
        let widget = self.widget(app);
        let duration = widget.duration;
        let is_selected = widget.is_selected;
        if old_widget.duration != duration {
            app.get_mut(controller).duration = Some(duration);
        }
        if old_widget.is_selected != is_selected {
            if is_selected {
                controller.forward(app, None);
            } else {
                controller.reverse(app, None);
            }
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&self.widget(app).builder);
        let controller = app
            .get(self)
            .controller
            .as_ref()
            .expect("created in init_state")
            .erase();
        builder(app, context, controller)
    }
}

#[cfg(test)]
mod tests {
    //! The cases of `test/navigation_drawer_test.dart` and the widget cases of
    //! `test/navigation_drawer_theme_test.dart`.

    use std::cell::{Cell, RefCell};
    use waft_painting::ShapeBorder;

    use waft_embedder::{Offset, Radius, Rect, Size};
    use waft_painting::{
        BorderRadius, BorderRadiusGeometry, BorderSide, BoxDecoration, CircleBorder, EdgeInsets,
        RoundedRectangleBorder, ShapeDecoration, StadiumBorder,
    };
    use waft_rendering::{AnyRenderBox, AnyRenderObject};

    use waft_widgets::{
        AnyElement, Center, Container, GlobalKey, Icon, IconData, MediaQuery, MediaQueryData,
        StatefulBuilder, Text, WidgetStatePropertyAll, WidgetsBinding,
    };

    use waft_widgets::testing::{
        WidgetTester, expect, find, finds_n_widgets, finds_one_widget, test_widgets,
    };

    use super::*;
    use crate::app::MaterialApp;
    use crate::drawer_header::DrawerHeader;
    use crate::icons::Icons;
    use crate::list_tile::ListTile;
    use crate::material::RenderInkFeatures;
    use crate::navigation_drawer_theme::NavigationDrawerThemeData;
    use crate::scaffold::{Scaffold, ScaffoldState};
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m3, pump_app};

    /// Dart's `widgetSetup(tester, viewWidth, viewHeight: viewHeight)`: a 2x view of that
    /// logical size.
    fn widget_setup(tester: &WidgetTester, view_width: f64, view_height: f64) {
        tester.view().set_device_pixel_ratio(2.0);
        let dpi = tester.device_pixel_ratio();
        tester
            .view()
            .set_physical_size(Size::new(view_width * dpi, view_height * dpi));
    }

    /// Dart's `_buildWidget(scaffoldKey, child)`: a `MaterialApp` whose home is a scaffold
    /// keyed `key` with the drawer `drawer`.
    fn pump_drawer(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        key: &GlobalKey,
        drawer: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let key = key.clone();
        pump_app(tester, theme, move |app| {
            Scaffold::new()
                .key(key.into())
                .drawer(drawer(app))
                .body(Container::new())
                .into_widget()
        });
    }

    /// Dart's `scaffoldKey.currentState!.openDrawer()`.
    fn open_drawer(tester: &WidgetTester, key: &GlobalKey) {
        let mut app = tester.cell.borrow_mut();
        let state = key
            .current_state::<ScaffoldState>(&app)
            .expect("a mounted scaffold");
        state.open_drawer(&mut app);
    }

    fn destination(icon: IconData, label: &str) -> WidgetRef {
        NavigationDrawerDestination::new(Icon::new(Some(icon)), Text::new(label)).into_widget()
    }

    /// The headline and the two destinations most cases build.
    fn headline_and_two_destinations() -> Vec<WidgetRef> {
        vec![
            Text::new("Headline").into_widget(),
            destination(Icons::ac_unit(), "AC"),
            destination(Icons::access_alarm(), "Alarm"),
        ]
    }

    fn select_nothing() -> ValueChanged<i32> {
        Rc::new(|_app, _index| {})
    }

    fn found(tester: &WidgetTester, text: &str) -> usize {
        find::text(text).evaluate(tester).len()
    }

    fn text_center(tester: &WidgetTester, text: &str) -> Offset {
        let text = *find::text(text).evaluate(tester).first().expect("the text");
        tester.get_rect(text).center()
    }

    fn icon_center(tester: &WidgetTester, icon: &IconData) -> Offset {
        let icon = tester.element(find::by_icon(icon).first());
        tester.get_rect(icon).center()
    }

    fn navigation_drawer(tester: &WidgetTester) -> AnyElement {
        *find::by_type::<NavigationDrawer>()
            .evaluate(tester)
            .first()
            .expect("a NavigationDrawer")
    }

    /// Dart's `_getMaterial(tester)`: the first `Material` in the drawer.
    fn material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let material = *tester
            .descendants_of_type::<Material>(navigation_drawer(tester))
            .first()
            .expect("a Material in the drawer");
        tester.widget(material, read)
    }

    /// Dart's `_getInkWell(tester)?.customBorder`: the first `InkWell` in the drawer.
    fn ink_well_custom_border(tester: &WidgetTester) -> Option<ShapeBorderRef> {
        let ink_well = *tester
            .descendants_of_type::<InkWell>(navigation_drawer(tester))
            .first()
            .expect("an InkWell in the drawer");
        tester.widget(ink_well, |ink_well: &InkWell| {
            ink_well.ink_response.custom_border.clone()
        })
    }

    /// Dart's `_getIndicatorDecoration(tester)`: the `ShapeDecoration` of the first `Ink`
    /// under a `NavigationIndicator`, as its color and shape.
    fn indicator_decoration(tester: &WidgetTester) -> (Option<Color>, ShapeBorderRef) {
        let ink = find::by_type::<NavigationIndicator>()
            .evaluate(tester)
            .into_iter()
            .flat_map(|indicator| tester.descendants_of_type::<Ink>(indicator))
            .next()
            .expect("an Ink under a NavigationIndicator");
        tester.widget(ink, |ink: &Ink| {
            let decoration = ink
                .decoration
                .as_deref()
                .expect("the indicator's decoration");
            let decoration = decoration
                .as_any()
                .downcast_ref::<ShapeDecoration>()
                .expect("a ShapeDecoration");
            (
                decoration.color.as_deref().copied(),
                decoration.shape.clone(),
            )
        })
    }

    /// Dart's `_iconStyle(tester, icon)?.color`.
    fn icon_color(tester: &WidgetTester, icon: &IconData) -> Option<Color> {
        tester
            .glyph_style(tester.element(find::by_icon(icon).first()))
            .color
            .as_deref()
            .copied()
    }

    /// Dart's `_labelStyle(tester, label)`.
    fn label_style(tester: &WidgetTester, label: &str) -> TextStyle {
        let text = *find::text(label)
            .evaluate(tester)
            .first()
            .expect("the label");
        tester.glyph_style(text)
    }

    fn label_color(tester: &WidgetTester, label: &str) -> Option<Color> {
        label_style(tester, label).color.as_deref().copied()
    }

    fn shape_eq(a: &dyn ShapeBorder, b: &dyn ShapeBorder) -> bool {
        a == b
    }

    /// Dart has no case measuring this, but `Drawer` lays the drawer out at its `_kWidth`, 304,
    /// and the destination's indicator takes the width of its padded tile. The theme's 336 is
    /// wider than the 280 that leaves, so it is clamped to the row's own rect.
    #[test]
    fn a_destinations_indicator_covers_its_row() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            widget_setup(tester, 800.0, 1000.0);
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new([
                    destination(Icons::explore(), "Explore"),
                    destination(Icons::chat(), "Chats"),
                ])
                .on_destination_selected(select_nothing())
                .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));

            let drawer = navigation_drawer(tester);
            let drawer_rect = tester.get_rect(drawer);
            assert_eq!(drawer_rect.width(), 304.0, "Drawer._kWidth");
            let row = *tester
                .descendants_of_type::<InkWell>(drawer)
                .first()
                .expect("the destination's row");
            let row_rect = tester.get_rect(row);
            assert_eq!(row_rect.left, drawer_rect.left + 12.0, "tilePadding");
            assert_eq!(row_rect.width(), 304.0 - 2.0 * 12.0);
            assert_eq!(row_rect.height(), 56.0, "tileHeight");

            let indicator = *tester
                .descendants_of_type::<NavigationIndicator>(drawer)
                .first()
                .expect("the selected indicator");
            let indicator_ink = *tester
                .descendants_of_type::<Ink>(indicator)
                .first()
                .expect("the indicator's ink");
            assert_eq!(tester.get_rect(indicator_ink), row_rect);

            let label = *find::text("Explore")
                .evaluate(tester)
                .first()
                .expect("the label");
            assert_eq!(
                tester.get_rect(label).left,
                row_rect.left + 16.0 + 24.0 + 12.0,
                "the row's leading inset"
            );
        });
    }

    // 'Navigation drawer updates destinations when tapped'.
    #[test]
    fn navigation_drawer_updates_destinations_when_tapped() {
        test_widgets(|tester| {
            let mutated_index = Rc::new(Cell::new(-1));
            let key = GlobalKey::new();
            widget_setup(tester, 3000.0, 3000.0);
            let mutated = Rc::clone(&mutated_index);
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new(headline_and_two_destinations())
                    .on_destination_selected(Rc::new(move |_app, i| mutated.set(i)))
                    .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump();

            assert_eq!(found(tester, "Headline"), 1);
            assert_eq!(found(tester, "AC"), 1);
            assert_eq!(found(tester, "Alarm"), 1);

            tester.pump_by(Duration::from_secs(1)); // animation done

            let at = text_center(tester, "Alarm");
            tester.tap_at(at, 1);
            assert_eq!(mutated_index.get(), 1);

            let at = text_center(tester, "AC");
            tester.tap_at(at, 2);
            assert_eq!(mutated_index.get(), 0);
        });
    }

    // 'NavigationDrawer can update background color'.
    #[test]
    fn navigation_drawer_can_update_background_color() {
        test_widgets(|tester| {
            let color = Colors::YELLOW.primary();
            let key = GlobalKey::new();
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new(headline_and_two_destinations())
                    .background_color(color)
                    .on_destination_selected(select_nothing())
                    .into_widget()
            });

            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1)); // animation done

            assert_eq!(material(tester, |material| material.color), Some(color));
        });
    }

    // 'NavigationDestinationDrawer background color is customizable'.
    #[test]
    fn navigation_drawer_destination_background_color_is_customizable() {
        test_widgets(|tester| {
            let color = Colors::YELLOW.primary();
            let key = GlobalKey::new();
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new([
                    Text::new("Headline").into_widget(),
                    destination(Icons::ac_unit(), "AC"),
                    NavigationDrawerDestination::new(
                        Icon::new(Some(Icons::access_alarm())),
                        Text::new("Alarm"),
                    )
                    .background_color(color)
                    .into_widget(),
                ])
                .on_destination_selected(select_nothing())
                .into_widget()
            });

            let find_destination_ink = |tester: &WidgetTester, label: &str| -> Vec<AnyElement> {
                let text = *find::text(label)
                    .evaluate(tester)
                    .first()
                    .expect("the label");
                let destination = tester.ancestor_of_type::<NavigationDrawerDestination>(text);
                tester.descendants_of_type::<Ink>(destination)
            };

            open_drawer(tester, &key);
            tester.pump();
            tester.pump_by(Duration::from_secs(1)); // Animation done.

            // Destination with no custom background color.
            let at = text_center(tester, "AC");
            tester.tap_at(at, 1);
            tester.pump();

            // When no background color is set, only the non-visible indicator Ink is expected.
            assert_eq!(find_destination_ink(tester, "AC").len(), 1);

            // Destination with a custom background color.
            let at = icon_center(tester, &Icons::access_alarm());
            tester.tap_at(at, 2);
            tester.pump();

            // A Material is added with the custom color.
            let alarm_inks = find_destination_ink(tester, "Alarm");
            assert_eq!(alarm_inks.len(), 2);
            let destination_color = tester.widget(alarm_inks[0], |ink: &Ink| {
                let decoration = ink
                    .effective_decoration()
                    .expect("the destination's decoration");
                decoration
                    .as_any()
                    .downcast_ref::<BoxDecoration>()
                    .expect("a BoxDecoration")
                    .color
                    .as_deref()
                    .copied()
            });
            assert_eq!(destination_color, Some(color));
        });
    }

    // 'NavigationDrawer can update elevation'.
    #[test]
    fn navigation_drawer_can_update_elevation() {
        test_widgets(|tester| {
            let elevation = 42.0;
            let key = GlobalKey::new();
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new(headline_and_two_destinations())
                    .elevation(elevation)
                    .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));

            assert_eq!(material(tester, |material| material.elevation), elevation);
        });
    }

    // 'NavigationDrawer uses proper defaults when no parameters are given'.
    #[test]
    fn navigation_drawer_uses_proper_defaults_when_no_parameters_are_given() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new(headline_and_two_destinations())
                    .on_destination_selected(select_nothing())
                    .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));
            let theme = ThemeData::new(&tester.cell.borrow());
            let colors = &theme.color_scheme;

            // Test drawer Material.
            assert_eq!(
                material(tester, |material| material.color),
                Some(colors.surface_container_low())
            );
            assert_eq!(
                material(tester, |material| material.surface_tint_color),
                Some(Colors::TRANSPARENT)
            );
            assert_eq!(
                material(tester, |material| material.shadow_color),
                Some(Colors::TRANSPARENT)
            );
            assert_eq!(material(tester, |material| material.elevation), 1.0);
            // Test indicator decoration.
            let (color, shape) = indicator_decoration(tester);
            assert_eq!(color, Some(colors.secondary_container()));
            assert!(shape_eq(&*shape, &StadiumBorder::new(BorderSide::NONE)));
            // Test selected and unselected icon colors.
            assert_eq!(
                icon_color(tester, &Icons::ac_unit()),
                Some(colors.on_secondary_container())
            );
            assert_eq!(
                icon_color(tester, &Icons::access_alarm()),
                Some(colors.on_surface_variant())
            );
            // Test selected and unselected label colors.
            assert_eq!(
                label_color(tester, "AC"),
                Some(colors.on_secondary_container())
            );
            assert_eq!(
                label_color(tester, "Alarm"),
                Some(colors.on_surface_variant())
            );
            // Test that the icon and label are the correct size.
            let icon_box =
                tester.render_box_of(tester.element(find::by_icon(&Icons::ac_unit()).first()));
            assert_eq!(icon_box.size(&tester.cell.borrow()), Size::new(24.0, 24.0));
            let icon_box =
                tester.render_box_of(tester.element(find::by_icon(&Icons::access_alarm()).first()));
            assert_eq!(icon_box.size(&tester.cell.borrow()), Size::new(24.0, 24.0));
        });
    }

    /// Dart's `find.text(label)` (which skips offstage) for a destination of the drawer's
    /// `ListView`: the text is built, and its sliver child lies in the viewport, as
    /// `SliverMultiBoxAdaptorElement.debugVisitOnstageChildren` lets the finder see only the
    /// children between the scroll offset and the end of the remaining paint extent.
    fn onstage_label(tester: &WidgetTester, label: &str) -> bool {
        let Some(&text) = find::text(label).evaluate(tester).first() else {
            return false;
        };
        let list = *find::by_type::<ListView>()
            .evaluate(tester)
            .first()
            .expect("the ListView");
        let viewport = tester.get_rect(list);
        let destination = tester.ancestor_of_type::<NavigationDrawerDestination>(text);
        let item = tester.get_rect(destination);
        item.top < viewport.bottom && item.bottom > viewport.top
    }

    // 'Navigation drawer is scrollable'.
    #[test]
    fn navigation_drawer_is_scrollable() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            widget_setup(tester, 500.0, 300.0);
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new(
                    (0..100).map(|i| destination(Icons::ac_unit(), &format!("Label{i}"))),
                )
                .on_destination_selected(select_nothing())
                .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));

            for i in 0..=5 {
                assert!(onstage_label(tester, &format!("Label{i}")), "Label{i}");
            }
            for i in 6..=8 {
                assert!(!onstage_label(tester, &format!("Label{i}")), "Label{i}");
            }

            tester.drag_from(Offset::new(0.0, 200.0), Offset::new(0.0, -200.0), 1);
            tester.pump();

            for i in 0..=2 {
                assert!(!onstage_label(tester, &format!("Label{i}")), "Label{i}");
            }
            for i in 3..=8 {
                assert!(onstage_label(tester, &format!("Label{i}")), "Label{i}");
            }
            for i in 9..=10 {
                assert!(!onstage_label(tester, &format!("Label{i}")), "Label{i}");
            }
        });
    }

    // 'Safe Area test'.
    #[test]
    fn safe_area_test() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let view_height = 300.0;
            widget_setup(tester, 500.0, view_height);
            let scaffold_key = key.clone();
            #[allow(deprecated)]
            let app = MaterialApp::new().use_inherited_media_query(true).home(
                Scaffold::new()
                    .key(scaffold_key.into())
                    .drawer(
                        NavigationDrawer::new(
                            (0..10).map(|i| destination(Icons::ac_unit(), &format!("Label{i}"))),
                        )
                        .on_destination_selected(select_nothing()),
                    )
                    .body(Container::new()),
            );
            tester.pump_widget(
                MediaQuery::new(MediaQueryData::new().padding(EdgeInsets::all(20.0)), app)
                    .into_widget(),
            );
            open_drawer(tester, &key);
            tester.pump();
            tester.pump_by(Duration::from_secs(1));

            let destination_rect = |label: &str| -> Rect {
                let text = *find::text(label)
                    .evaluate(tester)
                    .first()
                    .expect("the label");
                let destination = tester.ancestor_of_type::<NavigationDrawerDestination>(text);
                tester.get_rect(destination)
            };

            // Safe area padding on the top and sides.
            assert_eq!(
                destination_rect("Label0").top_left(),
                Offset::new(20.0, 20.0)
            );

            // No Safe area padding at the bottom.
            assert_eq!(destination_rect("Label4").bottom, view_height);
        });
    }

    // 'Navigation destination updates indicator color and shape'.
    #[test]
    fn navigation_destination_updates_indicator_color_and_shape() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let color = Color::new(0xff0000ff);
            let shape = RoundedRectangleBorder::default();

            let build_navigation_drawer =
                |tester: &mut WidgetTester, indicator: Option<(Color, RoundedRectangleBorder)>| {
                    let key = key.clone();
                    pump_app(tester, m3, move |_app| {
                        let mut drawer = NavigationDrawer::new(headline_and_two_destinations())
                            .on_destination_selected(select_nothing());
                        if let Some((indicator_color, indicator_shape)) = indicator {
                            drawer = drawer
                                .indicator_color(indicator_color)
                                .indicator_shape(indicator_shape);
                        }
                        Scaffold::new()
                            .key(key.into())
                            .drawer(drawer)
                            .body(Container::new())
                            .into_widget()
                    });
                };

            build_navigation_drawer(tester, None);
            open_drawer(tester, &key);
            tester.pump_and_settle();

            let theme = ThemeData::new(&tester.cell.borrow());
            // Test default indicator color and shape.
            let (indicator_color, indicator_shape) = indicator_decoration(tester);
            assert_eq!(
                indicator_color,
                Some(theme.color_scheme.secondary_container())
            );
            assert!(shape_eq(
                &*indicator_shape,
                &StadiumBorder::new(BorderSide::NONE)
            ));
            // Test that InkWell for hover, focus and pressed use default shape.
            assert!(shape_eq(
                &*ink_well_custom_border(tester).expect("a custom border"),
                &StadiumBorder::new(BorderSide::NONE)
            ));

            build_navigation_drawer(tester, Some((color, shape)));

            // Test custom indicator color and shape.
            let (indicator_color, indicator_shape) = indicator_decoration(tester);
            assert_eq!(indicator_color, Some(color));
            assert!(shape_eq(&*indicator_shape, &shape));
            // Test that InkWell for hover, focus and pressed use custom shape.
            assert!(shape_eq(
                &*ink_well_custom_border(tester).expect("a custom border"),
                &shape
            ));
        });
    }

    // 'NavigationDrawer.tilePadding defaults to EdgeInsets.symmetric(horizontal: 12.0)'.
    #[test]
    fn navigation_drawer_tile_padding_defaults_to_symmetric_horizontal_12() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            widget_setup(tester, 3000.0, 3000.0);
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new([destination(Icons::ac_unit(), "AC")])
                    .on_destination_selected(select_nothing())
                    .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump();
            let tile_padding = tester
                .widget(navigation_drawer(tester), |drawer: &NavigationDrawer| {
                    drawer.tile_padding
                });
            assert_eq!(tile_padding, EdgeInsetsGeometry::symmetric(0.0, 12.0));
        });
    }

    // 'Destinations respect their disabled state'.
    #[test]
    fn destinations_respect_their_disabled_state() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let selected_index = Rc::new(Cell::new(0));
            widget_setup(tester, 800.0, 1000.0);
            let selected = Rc::clone(&selected_index);
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new([
                    destination(Icons::ac_unit(), "AC"),
                    destination(Icons::access_alarm(), "Alarm"),
                    NavigationDrawerDestination::new(
                        Icon::new(Some(Icons::accessible())),
                        Text::new("Accessible"),
                    )
                    .enabled(false)
                    .into_widget(),
                ])
                .on_destination_selected(Rc::new(move |_app, i| selected.set(i)))
                .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump();

            assert_eq!(found(tester, "AC"), 1);
            assert_eq!(found(tester, "Alarm"), 1);
            assert_eq!(found(tester, "Accessible"), 1);

            tester.pump_by(Duration::from_secs(1));

            assert_eq!(selected_index.get(), 0);

            let at = text_center(tester, "Alarm");
            tester.tap_at(at, 1);
            assert_eq!(selected_index.get(), 1);

            let at = text_center(tester, "Accessible");
            tester.tap_at(at, 2);
            assert_eq!(selected_index.get(), 1);

            tester.pump_and_settle();
        });
    }

    /// Stands in for Dart's `FlutterLogo`, which Waft does not have; the case only counts
    /// them.
    #[derive(Debug)]
    struct FlutterLogo;

    impl StatelessWidget for FlutterLogo {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::new().width(24.0).height(24.0).into_widget()
        }
    }

    // 'NavigationDrawer can display header and footer'.
    #[test]
    fn navigation_drawer_can_display_header_and_footer() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            widget_setup(tester, 3000.0, 3000.0);
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new(
                    (0..10).map(|i| destination(Icons::home(), &format!("Item {i}"))),
                )
                .header(DrawerHeader::new(Some(
                    Center::new()
                        .child(
                            Column::new()
                                .main_axis_alignment(waft_rendering::MainAxisAlignment::Center)
                                .spacing(8.0)
                                .children([
                                    FlutterLogo.into_widget(),
                                    Text::new("Header").into_widget(),
                                ]),
                        )
                        .into_widget(),
                )))
                .footer(
                    ListTile::new()
                        .leading(FlutterLogo)
                        .title(Text::new("Footer"))
                        .trailing(Icon::new(Some(Icons::settings())))
                        .on_tap(Listener::new(|_app| {})),
                )
                .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));

            expect(tester, &find::by_type::<DrawerHeader>(), finds_one_widget());
            assert_eq!(found(tester, "Header"), 1);
            expect(tester, &find::by_type::<FlutterLogo>(), finds_n_widgets(2));
            expect(tester, &find::by_type::<ListTile>(), finds_one_widget());
            assert_eq!(found(tester, "Footer"), 1);
            tester.element(find::by_icon(&Icons::settings()).first());
        });
    }

    // 'NavigationDrawer does not crash at zero area'.
    #[test]
    fn navigation_drawer_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(NavigationDrawer::new([destination(
                        Icons::inbox(),
                        "Inbox",
                    )])))
                    .into_widget()
            });
            let drawer = tester.render_box_of(navigation_drawer(tester));
            assert_eq!(drawer.size(&tester.cell.borrow()), Size::ZERO);
        });
    }

    /// The render object of the `Material` nearest above `element`: Dart's
    /// `Material.of(tester.element(finder)) as RenderBox`.
    fn ink_material_of(tester: &WidgetTester, element: AnyElement) -> AnyRenderObject {
        let app = tester.cell.borrow();
        let mut object = element
            .find_render_object(&app)
            .expect("the element has a render object");
        loop {
            object = object.parent(&app).expect("a Material above the element");
            if object.downcast::<RenderInkFeatures>(&app).is_some() {
                return object;
            }
        }
    }

    // Regression test for https://github.com/flutter/flutter/issues/180233
    // 'NavigationDrawer ink effects are bounded within scrollable area when footer is present'.
    #[test]
    fn navigation_drawer_ink_effects_are_bounded_within_scrollable_area_when_footer_is_present() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            widget_setup(tester, 800.0, 400.0);
            pump_drawer(tester, m3, &key, move |_app| {
                NavigationDrawer::new(
                    (0..10).map(|i| destination(Icons::home(), &format!("Item {i}"))),
                )
                .footer(Padding::new(EdgeInsetsGeometry::all(16.0)).child(Text::new("Footer")))
                .into_widget()
            });

            open_drawer(tester, &key);
            tester.pump_and_settle();

            let footer_text = *find::text("Footer")
                .evaluate(tester)
                .first()
                .expect("the footer");
            let footer = tester.ancestor_of_type::<Padding>(footer_text);
            let footer_rect = tester.get_rect(tester.render_box_of(footer));

            let first_destination = *find::by_type::<NavigationDrawerDestination>()
                .evaluate(tester)
                .first()
                .expect("a destination");
            let ink_wells = tester.descendants_of_type::<InkWell>(first_destination);
            assert_eq!(ink_wells.len(), 1);

            let ink_material: AnyRenderBox = ink_material_of(tester, ink_wells[0])
                .as_box()
                .expect("a render box");
            let ink_material_rect = tester.get_rect(ink_material);

            // Equal at f32 precision: `local_to_global` maps through valo's f32 `Matrix4`.
            assert_eq!(ink_material_rect.bottom as f32, footer_rect.top as f32);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/180359.
    // 'Indicator repaints when selectedIndex changes without pointer interaction'.
    #[test]
    fn indicator_repaints_when_selected_index_changes_without_pointer_interaction() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let selected_index = Rc::new(Cell::new(0));
            let set_selected_index: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let (scaffold_key, selected, setter) = (
                key.clone(),
                Rc::clone(&selected_index),
                Rc::clone(&set_selected_index),
            );
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    *setter.borrow_mut() = Some(set_state);
                    Scaffold::new()
                        .key(scaffold_key.clone().into())
                        .drawer(
                            NavigationDrawer::new([
                                destination(Icons::ac_unit(), "AC"),
                                destination(Icons::access_alarm(), "Alarm"),
                            ])
                            .selected_index(Some(selected.get())),
                        )
                        .body(Container::new())
                        .into_widget()
                })
                .into_widget(),
            ));

            open_drawer(tester, &key);
            tester.pump_and_settle();

            // The indicator is painted by an Ink widget, which paints on the nearest
            // ancestor Material rather than on its own render object.
            let indicator = *find::by_type::<NavigationIndicator>()
                .evaluate(tester)
                .first()
                .expect("an indicator");
            let indicator_ink = *tester
                .descendants_of_type::<Ink>(indicator)
                .first()
                .expect("the indicator's Ink");
            let ink_controller = ink_material_of(tester, indicator_ink);

            // Change the selected index without any pointer interaction and advance
            // one frame, stopping before the paint phase: the Material that hosts the
            // indicator's ink must have been marked as needing paint, otherwise it
            // keeps showing the indicator's previous frame.
            {
                let set_state = set_selected_index.borrow().clone().expect("built");
                let mut app = tester.cell.borrow_mut();
                set_state(&mut app, &mut |_app| selected_index.set(1));
            }
            pump_to_layout(tester);
            assert!(ink_controller.debug_needs_paint(&tester.cell.borrow()));
            finish_frame(tester);

            // Let the indicator animation run to completion normally.
            tester.pump_and_settle();
        });
    }

    /// The first half of Dart's `tester.pump_by(null, EnginePhase.layout)`: begins a frame at the
    /// last frame's time stamp, then builds and lays out without painting.
    fn pump_to_layout(tester: &WidgetTester) {
        let mut app = tester.cell.borrow_mut();
        let views: Vec<_> = app
            .platform()
            .views()
            .iter()
            .map(|view| view.id())
            .collect();
        waft_scheduler::SchedulerBinding::handle_begin_frame(&mut app, &views, None);
        app.drain_microtasks();
        let binding = WidgetsBinding::instance(&mut app);
        let root_element = binding.root_element(&app).expect("a root element");
        binding
            .build_owner(&app)
            .build_scope(&mut app, root_element, None);
        waft_rendering::RendererBinding::instance(&mut app)
            .root_pipeline_owner(&app)
            .flush_layout(&mut app);
    }

    /// Ends the frame [`pump_to_layout`] began, as Dart's pump ends it after its phase.
    fn finish_frame(tester: &WidgetTester) {
        let mut app = tester.cell.borrow_mut();
        let views: Vec<_> = app
            .platform()
            .views()
            .iter()
            .map(|view| view.id())
            .collect();
        waft_scheduler::SchedulerBinding::handle_draw_frame(&mut app, &views);
        app.drain_microtasks();
        drop(app);
        tester.cell.checkpoint();
    }

    // navigation_drawer_theme_test.dart

    /// The theme the theme cases give `ThemeData(navigationDrawerTheme:)`.
    fn navigation_drawer_theme() -> NavigationDrawerThemeData {
        NavigationDrawerThemeData::new()
            .background_color(Color::new(0x00000001))
            .elevation(7.0)
            .shadow_color(Color::new(0x00000002))
            .surface_tint_color(Color::new(0x00000003))
            .indicator_color(Color::new(0x00000004))
            .indicator_shape(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadius::only(
                    Radius::ZERO,
                    Radius::circular(16.0),
                    Radius::ZERO,
                    Radius::ZERO,
                )
                .into(),
            ))
            .label_text_style(Rc::new(WidgetStatePropertyAll::new(Some(
                TextStyle::new().font_size(7.0),
            ))))
            .icon_theme(Rc::new(WidgetStatePropertyAll::new(Some(
                IconThemeData::new().color(Color::new(0x00000005)),
            ))))
    }

    fn pump_themed_drawer(
        tester: &mut WidgetTester,
        key: &GlobalKey,
        theme_data_theme: NavigationDrawerThemeData,
        drawer: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        pump_drawer(
            tester,
            move |app| {
                ThemeData::builder()
                    .navigation_drawer_theme(theme_data_theme)
                    .build(app)
            },
            key,
            drawer,
        );
    }

    // 'NavigationDrawerThemeData values are used when no NavigationDrawer properties are
    // specified'.
    #[test]
    fn navigation_drawer_theme_data_values_are_used_when_no_navigation_drawer_properties_are_specified()
     {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let theme = navigation_drawer_theme();
            pump_themed_drawer(tester, &key, theme.clone(), |_app| {
                NavigationDrawer::new(headline_and_two_destinations())
                    .on_destination_selected(select_nothing())
                    .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));

            // Test drawer Material.
            assert_eq!(
                material(tester, |material| material.color),
                theme.background_color
            );
            assert_eq!(
                material(tester, |material| material.surface_tint_color),
                theme.surface_tint_color
            );
            assert_eq!(
                material(tester, |material| material.shadow_color),
                theme.shadow_color
            );
            assert_eq!(material(tester, |material| material.elevation), 7.0);
            // Test indicator decoration.
            let (indicator_color, indicator_shape) = indicator_decoration(tester);
            assert_eq!(indicator_color, theme.indicator_color);
            assert!(shape_eq(
                &*indicator_shape,
                theme.indicator_shape.as_deref().expect("a shape")
            ));
            // Test icon.
            let themed_icon_color = theme
                .icon_theme
                .as_ref()
                .and_then(|icon_theme| icon_theme.resolve(&WidgetStates::new()))
                .and_then(|icon_theme| icon_theme.color.as_deref().copied());
            assert_eq!(icon_color(tester, &Icons::ac_unit()), themed_icon_color);
            assert_eq!(
                icon_color(tester, &Icons::access_alarm()),
                themed_icon_color
            );
            // Test label.
            let themed_label_style = theme
                .label_text_style
                .as_ref()
                .and_then(|label_text_style| label_text_style.resolve(&WidgetStates::new()));
            assert_eq!(Some(label_style(tester, "AC")), themed_label_style);
            assert_eq!(Some(label_style(tester, "Alarm")), themed_label_style);
        });
    }

    // 'NavigationDrawer values take priority over NavigationDrawerThemeData values when both
    // properties are specified'.
    #[test]
    fn navigation_drawer_values_take_priority_over_navigation_drawer_theme_data_values() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let background_color = Color::new(0x00000009);
            let elevation = 14.0;
            let shadow_color = Color::new(0x00000008);
            let surface_tint_color = Color::new(0x00000007);
            let indicator_shape =
                RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(32.0));
            let indicator_color = Color::new(0x00000006);
            let shape = indicator_shape;
            pump_themed_drawer(tester, &key, navigation_drawer_theme(), move |_app| {
                NavigationDrawer::new(headline_and_two_destinations())
                    .background_color(background_color)
                    .elevation(elevation)
                    .shadow_color(shadow_color)
                    .surface_tint_color(surface_tint_color)
                    .indicator_shape(shape)
                    .indicator_color(indicator_color)
                    .on_destination_selected(select_nothing())
                    .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));

            // Test drawer Material.
            assert_eq!(
                material(tester, |material| material.color),
                Some(background_color)
            );
            assert_eq!(
                material(tester, |material| material.surface_tint_color),
                Some(surface_tint_color)
            );
            assert_eq!(
                material(tester, |material| material.shadow_color),
                Some(shadow_color)
            );
            assert_eq!(material(tester, |material| material.elevation), elevation);
            // Test indicator decoration.
            let (color, shape) = indicator_decoration(tester);
            assert_eq!(color, Some(indicator_color));
            assert!(shape_eq(&*shape, &indicator_shape));
        });
    }

    // 'Local NavigationDrawerTheme takes priority over ThemeData.navigationDrawerTheme'.
    #[test]
    fn local_navigation_drawer_theme_takes_priority_over_theme_data_navigation_drawer_theme() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let background_color = Color::new(0x00000009);
            let elevation = 7.0;
            let shadow_color = Color::new(0x00000008);
            let surface_tint_color = Color::new(0x00000007);
            let icon_color_value = Color::new(0x00000006);
            let label_style_value = TextStyle::new().font_size(7.0);
            let indicator_color = Color::new(0x00000005);
            pump_themed_drawer(tester, &key, navigation_drawer_theme(), move |_app| {
                NavigationDrawerTheme::new(
                    NavigationDrawerThemeData::new()
                        .background_color(background_color)
                        .elevation(elevation)
                        .shadow_color(shadow_color)
                        .surface_tint_color(surface_tint_color)
                        .indicator_shape(CircleBorder::default())
                        .indicator_color(indicator_color)
                        .label_text_style(Rc::new(WidgetStatePropertyAll::new(Some(
                            TextStyle::new().font_size(7.0),
                        ))))
                        .icon_theme(Rc::new(WidgetStatePropertyAll::new(Some(
                            IconThemeData::new().color(icon_color_value),
                        )))),
                    NavigationDrawer::new(headline_and_two_destinations())
                        .on_destination_selected(select_nothing()),
                )
                .into_widget()
            });
            open_drawer(tester, &key);
            tester.pump_by(Duration::from_secs(1));

            // Test drawer Material.
            assert_eq!(
                material(tester, |material| material.color),
                Some(background_color)
            );
            assert_eq!(
                material(tester, |material| material.surface_tint_color),
                Some(surface_tint_color)
            );
            assert_eq!(
                material(tester, |material| material.shadow_color),
                Some(shadow_color)
            );
            assert_eq!(material(tester, |material| material.elevation), elevation);
            // Test indicator decoration.
            let (color, shape) = indicator_decoration(tester);
            assert_eq!(color, Some(indicator_color));
            assert!(shape_eq(&*shape, &CircleBorder::default()));
            // Test icon.
            assert_eq!(
                icon_color(tester, &Icons::ac_unit()),
                Some(icon_color_value)
            );
            assert_eq!(
                icon_color(tester, &Icons::access_alarm()),
                Some(icon_color_value)
            );
            // Test label.
            assert_eq!(label_style(tester, "AC"), label_style_value);
            assert_eq!(label_style(tester, "Alarm"), label_style_value);
        });
    }

    // navigation_drawer_theme_test.dart 'NavigationDrawerThemeData copyWith, ==, hashCode,
    // basics'.
    #[test]
    fn navigation_drawer_theme_data_copy_with_equality_basics() {
        assert_eq!(
            NavigationDrawerThemeData::new(),
            NavigationDrawerThemeData::new().copy_with()
        );
    }

    // navigation_drawer_theme_test.dart 'NavigationDrawerThemeData lerp special cases'.
    #[test]
    fn navigation_drawer_theme_data_lerp_special_cases() {
        assert_eq!(NavigationDrawerThemeData::lerp(None, None, 0.0), None);
        let data = NavigationDrawerThemeData::new();
        assert_eq!(
            NavigationDrawerThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn navigation_drawer_theme_data_null_fields_by_default() {
        let theme = NavigationDrawerThemeData::new();
        assert_eq!(theme.tile_height, None);
        assert_eq!(theme.background_color, None);
        assert_eq!(theme.elevation, None);
        assert_eq!(theme.shadow_color, None);
        assert_eq!(theme.surface_tint_color, None);
        assert_eq!(theme.indicator_color, None);
        assert!(theme.indicator_shape.is_none());
        assert_eq!(theme.indicator_size, None);
        assert!(theme.label_text_style.is_none());
        assert!(theme.icon_theme.is_none());
    }

    #[test]
    fn navigation_drawer_theme_data_lerps_each_field_as_dart_does() {
        let a = NavigationDrawerThemeData::new()
            .tile_height(40.0)
            .background_color(Colors::BLACK)
            .elevation(2.0)
            .indicator_size(Size::new(100.0, 40.0));
        let b = NavigationDrawerThemeData::new()
            .tile_height(60.0)
            .background_color(Colors::WHITE)
            .elevation(6.0)
            .indicator_size(Size::new(300.0, 60.0));
        let halfway = NavigationDrawerThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both");
        assert_eq!(halfway.tile_height, Some(50.0));
        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        // Dart lerps `a.indicatorSize` with itself.
        assert_eq!(halfway.indicator_size, Some(Size::new(100.0, 40.0)));
    }

    #[test]
    fn theme_data_holds_the_navigation_drawer_theme() {
        test_widgets(|tester| {
            let app = tester.cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(
                theme.navigation_drawer_theme,
                NavigationDrawerThemeData::new()
            );

            let tall = ThemeData::builder()
                .navigation_drawer_theme(NavigationDrawerThemeData::new().tile_height(60.0))
                .build(&app);
            assert_eq!(tall.navigation_drawer_theme.tile_height, Some(60.0));
            assert_ne!(theme, tall);
            assert_eq!(
                theme
                    .copy_with()
                    .navigation_drawer_theme(tall.navigation_drawer_theme.clone())
                    .build(),
                tall
            );
            let short = ThemeData::builder()
                .navigation_drawer_theme(NavigationDrawerThemeData::new().tile_height(40.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&short, &tall, 0.5)
                    .navigation_drawer_theme
                    .tile_height,
                Some(50.0)
            );
        });
    }

    #[test]
    fn navigation_drawer_theme_of_prefers_the_closest_theme_over_theme_data() {
        test_widgets(|tester| {
            let widget_theme = NavigationDrawerThemeData::new().tile_height(30.0);
            let expected = widget_theme.clone();
            let seen = crate::drawer_theme::tests::probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .navigation_drawer_theme(NavigationDrawerThemeData::new().tile_height(70.0))
                        .build(app);
                    crate::theme::Theme::new(
                        data,
                        NavigationDrawerTheme::new(widget_theme.clone(), child),
                    )
                    .into_widget()
                },
                NavigationDrawerTheme::of,
            );
            assert_eq!(seen, expected);

            let seen = crate::drawer_theme::tests::probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .navigation_drawer_theme(NavigationDrawerThemeData::new().tile_height(70.0))
                        .build(app);
                    crate::theme::Theme::new(data, child).into_widget()
                },
                NavigationDrawerTheme::of,
            );
            assert_eq!(seen.tile_height, Some(70.0));
        });
    }
}
