//! A working consumer of the WinUI kit: every control in its styles and states, in the light and dark themes.
#![feature(arbitrary_self_types)]

use std::rc::Rc;

mod catalog;
mod sections;
pub use catalog::Feature;

use waft_embedder::{Color, WindowConfig};
use waft_foundation::{App, Handle, ValueKey};
use waft_painting::{Axis, EdgeInsetsGeometry, PaintingBinding};
use waft_rendering::{CrossAxisAlignment, MainAxisSize, StackFit};
use waft_widgets::*;
use waft_winui::*;

/// Starts the gallery with the native or web host selected by Waft.
#[waft::main]
fn main(app: &mut App) {
    run(app);
}

/// The window a desktop opens for the gallery; a host that gives the app its view opens none.
fn gallery_window() -> WindowConfig {
    WindowConfig {
        title: "WinUI Gallery".to_owned(),
        size: [1080.0, 780.0],
        ..WindowConfig::default()
    }
}

/// The gallery switches directly between expanded and minimal navigation.
const GALLERY_NAVIGATION_BREAKPOINT: f64 = 800.0;

/// Registers the bundled Selawik faces under the kit's font family. The Fluent icon font is
/// [`FluentApp`]'s own, installed when it mounts.
pub fn install_fonts(app: &mut App) {
    let painting = PaintingBinding::instance(app);
    for bytes in [
        &include_bytes!("../fonts/selawk.ttf")[..],
        &include_bytes!("../fonts/selawkb.ttf")[..],
        &include_bytes!("../fonts/selawksb.ttf")[..],
        &include_bytes!("../fonts/selawkl.ttf")[..],
        &include_bytes!("../fonts/selawksl.ttf")[..],
    ] {
        painting.register_font(app, FONT_FAMILY, bytes.to_vec());
    }
}

/// The kit needs only the widgets layer: [`FluentApp`] supplies the view, media query, focus,
/// text direction, theme and page entrance, and the gallery is its one page.
pub fn run(app: &mut App) {
    run_feature(app, Feature::Button);
}

/// Starts the same gallery at a chosen feature, useful for focused host tests.
pub fn run_feature(app: &mut App, feature: Feature) {
    install_fonts(app);
    run_app_with(
        app,
        GalleryRoot {
            initial_feature: feature,
        }
        .into_widget(),
        gallery_window(),
    );
}

/// The app around the gallery: the one [`FluentApp`] the gallery's theme toggle drives.
#[derive(Debug)]
pub struct GalleryRoot {
    /// The first feature to display when this gallery mounts.
    pub initial_feature: Feature,
}

/// Whether the gallery is in its dark theme; [`FluentApp`] shows it through `theme_mode`.
pub struct GalleryRootState {
    state: StateData<GalleryRoot>,
    dark: bool,
}

impl StatefulWidget for GalleryRoot {
    type State = GalleryRootState;

    fn create_state(&self) -> GalleryRootState {
        GalleryRootState {
            state: StateData::new(),
            dark: false,
        }
    }
}

impl State for GalleryRootState {
    type Widget = GalleryRoot;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let initial_feature = self.widget(app).initial_feature;
        let dark = app.get(self).dark;
        let toggle = Rc::new(move |app: &mut App| {
            self.set_state(app, |state| state.dark = !state.dark);
        });
        FluentApp::new()
            .theme_mode(if dark {
                FluentThemeMode::Dark
            } else {
                FluentThemeMode::Light
            })
            .title("WinUI Gallery")
            .debug_show_checked_mode_banner(false)
            .home(Gallery {
                initial_feature,
                on_toggle_theme: toggle,
            })
            .into_widget()
    }
}

/// The navigation shell for the feature catalog.
pub struct Gallery {
    /// The first feature to display when this gallery mounts.
    pub initial_feature: Feature,
    /// How the gallery switches its [`FluentApp`] between the light and dark themes.
    on_toggle_theme: Rc<dyn Fn(&mut App)>,
}

impl std::fmt::Debug for Gallery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Gallery")
            .field("initial_feature", &self.initial_feature)
            .finish_non_exhaustive()
    }
}

/// Owns the theme and shared examples while keeping visited pages mounted.
pub struct GalleryState {
    /// Native state linkage.
    state: StateData<Gallery>,
    /// Activations in the Button examples.
    pub clicks: u32,
    /// The Wi-Fi example's value.
    pub wifi: bool,
    /// The airplane-mode example's value.
    pub airplane: bool,
    /// Current catalog destination.
    selected: Feature,
    /// Destinations mounted so far, in first-visit order.
    visited: Vec<Feature>,
}

impl StatefulWidget for Gallery {
    type State = GalleryState;

    fn create_state(&self) -> GalleryState {
        GalleryState {
            state: StateData::new(),
            clicks: 0,
            wifi: true,
            airplane: false,
            selected: self.initial_feature,
            visited: vec![self.initial_feature],
        }
    }
}

/// Builds text using the kit's typography, read from the resources where it is built, and
/// resolved theme color.
pub(crate) fn label(text: impl Into<String>, style: TextBlockStyle, color: Color) -> WidgetRef {
    let text = text.into();
    Builder::new(move |app, context| {
        let resources = ThemeResources::of(app, context);
        Text::new(text.clone())
            .style(style.text_style(&resources, color))
            .into_widget()
    })
    .into_widget()
}

/// Stacks example content with consistent spacing.
pub(crate) fn column(children: Vec<WidgetRef>, spacing: f64) -> WidgetRef {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(spacing)
        .children(children)
        .into_widget()
}

/// Places related example controls on one row.
pub(crate) fn row(children: Vec<WidgetRef>, spacing: f64) -> WidgetRef {
    Row::new()
        .main_axis_size(MainAxisSize::Min)
        .spacing(spacing)
        .children(children)
        .into_widget()
}

/// A preview card names the behavior before presenting its controls.
pub(crate) fn example(
    title: &str,
    description: &str,
    resources: &ThemeResources,
    content: WidgetRef,
) -> WidgetRef {
    let mut children = vec![label(
        title,
        TextBlockStyle::Subtitle,
        resources.text_fill_color_primary,
    )];
    if !description.is_empty() {
        children.push(label(
            description,
            TextBlockStyle::Body,
            resources.text_fill_color_secondary,
        ));
    }
    children.push(SizedBox::new().height(4.0).into_widget());
    children.push(content);
    SizedBox::new()
        .width(f64::INFINITY)
        .child(
            ControlBorder::new(
                Brush::Solid(resources.card_background_fill_color_default),
                Brush::Solid(resources.card_stroke_color_default),
            )
            .border_thickness(1.0)
            .corner_radius(8.0)
            .child(Padding::new(EdgeInsetsGeometry::all(20.0)).child(column(children, 12.0))),
        )
        .into_widget()
}

/// Keeps the same children mounted while arranging options vertically in narrow previews.
pub(crate) fn example_row(children: Vec<WidgetRef>, spacing: f64) -> WidgetRef {
    LayoutBuilder::new(move |_, _, constraints| {
        let required =
            children.len() as f64 * 180.0 + children.len().saturating_sub(1) as f64 * spacing;
        let horizontal = constraints.max_width >= required;
        Flex::new(if horizontal {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(spacing)
        .children(children.iter().cloned().map(|child| {
            Flexible::new(child)
                .flex(if horizontal { 1 } else { 0 })
                .into_widget()
        }))
        .into_widget()
    })
    .into_widget()
}

/// A section: its heading in the subtitle style, then its content.
pub(crate) fn section(
    title: &str,
    resources: &ThemeResources,
    content: WidgetRef,
) -> Vec<WidgetRef> {
    let text = resources.text_fill_color_primary;
    vec![label(title, TextBlockStyle::Subtitle, text), content]
}

impl GalleryState {
    /// Builds one feature without mounting unrelated examples.
    fn feature_page(
        self: Handle<Self>,
        app: &mut App,
        feature: Feature,
        resources: &ThemeResources,
        horizontal_inset: f64,
    ) -> WidgetRef {
        match feature {
            Feature::Page => sections::page::build(resources, horizontal_inset),
            _ => self.scrolled_feature_page(app, feature, resources, horizontal_inset),
        }
    }

    /// Builds one feature whose examples scroll under the shell's heading.
    fn scrolled_feature_page(
        self: Handle<Self>,
        app: &mut App,
        feature: Feature,
        resources: &ThemeResources,
        horizontal_inset: f64,
    ) -> WidgetRef {
        let examples = match feature {
            Feature::Button => sections::button::build(self, resources),
            Feature::SplitButton | Feature::ToggleSplitButton | Feature::MenuFlyout => {
                sections::command_buttons::page(feature, resources)
            }
            Feature::MenuBar => sections::menu_bar::page(resources),
            Feature::CommandBar => sections::command_bar::build(resources),
            Feature::AutoSuggestBox => sections::auto_suggest_box::build(resources),
            Feature::BreadcrumbBar => sections::breadcrumb_bar::page(resources),
            Feature::Flyout => sections::flyout::flyout(resources),
            Feature::ContentDialog => sections::content_dialog::build(resources),
            Feature::DropDownButton => sections::flyout::drop_down_button(resources),
            Feature::TextBox => sections::text_input::text_box(resources),
            Feature::PasswordBox => sections::text_input::password_box(resources),
            Feature::NumberBox => sections::number_box::build(resources),
            Feature::ComboBox => sections::combo_box::build(resources),
            Feature::ToggleSwitch => sections::toggle_switch::build(self, app, resources),
            Feature::Grid => sections::grid::build(resources),
            Feature::CheckBox => sections::check_box::build(resources),
            Feature::RadioButton => sections::radio_button::build(resources),
            Feature::ToggleButton => sections::button_family::toggle_button(resources),
            Feature::RepeatButton => sections::button_family::repeat_button(resources),
            Feature::HyperlinkButton => sections::button_family::hyperlink_button(resources),
            Feature::Slider => sections::slider::build(resources),
            Feature::ColorPicker => sections::color_picker::build(resources),
            Feature::ProgressBar => sections::progress_bar::build(resources),
            Feature::InfoBadge => sections::info_badge::build(resources),
            Feature::ProgressRing => sections::progress_ring::build(resources),
            Feature::Expander => sections::expander::build(resources),
            Feature::InfoBar => sections::info_bar::build(resources),
            Feature::SplitView => sections::split_view::build(resources),
            Feature::Acrylic => sections::acrylic::build(resources),
            Feature::ToolTip => sections::tool_tip::build(resources),
            Feature::TabView => sections::tab_view::build(resources),
            Feature::NavigationView => sections::navigation_view::build(resources),
            Feature::TreeView => sections::tree_view::build(resources),
            Feature::SelectorBar => sections::selector_bar::page(resources),
            Feature::ItemsView => sections::items_view::build(resources),
            Feature::TableView => sections::table_view::build(resources),
            Feature::Typography => sections::typography::build(resources),
            Feature::Icons => sections::icons::build(resources),
            Feature::Page => {
                unreachable!("the Page destination fills its area instead of scrolling")
            }
        };
        let mut children = vec![label(
            feature.description(),
            TextBlockStyle::Body,
            resources.text_fill_color_secondary,
        )];
        if matches!(feature, Feature::Button | Feature::ToggleSwitch) {
            let state = app.get(self);
            children.push(label(
                format!(
                    "Clicked {} times · wifi {} · airplane {}",
                    state.clicks, state.wifi, state.airplane
                ),
                TextBlockStyle::Body,
                resources.text_fill_color_secondary,
            ));
        }
        // The shell supplies the page heading; each section's first widget is that same title.
        children.extend(examples.into_iter().skip(1));
        SingleChildScrollView::new()
            .child(
                Padding::new(EdgeInsetsGeometry::from_ltrb(
                    horizontal_inset,
                    32.0,
                    horizontal_inset,
                    32.0,
                ))
                .child(column(children, 20.0)),
            )
            .into_widget()
    }
}

impl State for GalleryState {
    type Widget = Gallery;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        // The navigation's width: the view's, less the bars at its sides it keeps clear of.
        let bars = MediaQuery::padding_of(app, context);
        let width = MediaQuery::size_of(app, context).width() - bars.left - bars.right;
        let minimal = width < GALLERY_NAVIGATION_BREAKPOINT;
        // Minimal HeaderContent follows the visible toggle column and its negative margin.
        let horizontal_inset = if minimal {
            r.pane_toggle_button_width + r.navigation_view_minimal_header_margin[0]
        } else {
            r.navigation_view_header_margin[0]
        };
        let state = app.get(self);
        let (selected, visited) = (state.selected, state.visited.clone());
        let dark = r.theme == Theme::Dark;
        let resources = ThemeResources::of(app, context);
        let pages: Vec<_> = visited
            .into_iter()
            .map(|feature| {
                let active = feature == selected;
                let page = self.feature_page(app, feature, &resources, horizontal_inset);
                KeyedSubtree::new(
                    Offstage::new().offstage(!active).child(TickerMode::new(
                        active,
                        Focus::new(page)
                            .can_request_focus(false)
                            .descendants_are_focusable(active),
                    )),
                )
                .key(ValueKey::new(feature.title().to_owned()).into())
                .into_widget()
            })
            .collect();
        let mut navigation = NavigationView::new(
            Feature::ALL
                .into_iter()
                .map(|feature| {
                    NavigationViewItem::text(feature.title(), feature.title())
                        .icon(FontIcon::symbol(feature.symbol()).font_size(16.0))
                })
                .collect(),
            Some(selected.title().to_owned()),
            move |app, args| {
                if let Some(feature) = args.item.and_then(|item| {
                    Feature::ALL
                        .into_iter()
                        .find(|feature| feature.title() == item.id)
                }) {
                    self.set_state(app, |state| {
                        state.selected = feature;
                        if !state.visited.contains(&feature) {
                            state.visited.push(feature);
                        }
                    });
                }
            },
            Stack::new().fit(StackFit::Expand).children(pages),
        );
        navigation.pane_title = "WinUI Gallery".to_owned();
        navigation.open_pane_length = 240.0;
        navigation.compact_mode_threshold_width = GALLERY_NAVIGATION_BREAKPOINT;
        navigation.expanded_mode_threshold_width = GALLERY_NAVIGATION_BREAKPOINT;
        navigation.is_settings_visible = false;
        navigation.is_back_button_visible = NavigationViewBackButtonVisible::Collapsed;
        navigation.header = Some(label(
            selected.title(),
            TextBlockStyle::Title,
            resources.text_fill_color_primary,
        ));
        navigation = navigation
            .footer_menu_items(vec![
                NavigationViewItem::text(
                    "gallery-theme",
                    if dark { "Light theme" } else { "Dark theme" },
                )
                .icon(FontIcon::symbol(if dark {
                    FluentSymbol::WeatherSunny
                } else {
                    FluentSymbol::WeatherMoon
                }))
                .selects_on_invoked(false),
            ])
            .item_invoked(move |app, args| {
                if args.item.id == "gallery-theme" {
                    let toggle = Rc::clone(&self.widget(app).on_toggle_theme);
                    toggle(app);
                }
            });
        gallery_page(&resources, navigation)
    }
}

/// The page the navigation shell sits on: its content reaches under a phone's bars, and the
/// navigation keeps its toggle, pane items and content clear of them while its pane and content
/// reach the screen's edges.
fn gallery_page(resources: &ThemeResources, navigation: NavigationView) -> WidgetRef {
    waft_winui::Page::new()
        .background(Brush::Solid(resources.solid_background_fill_color_base))
        .extends_content_into_system_bars(true)
        .content(navigation)
        .into_widget()
}
