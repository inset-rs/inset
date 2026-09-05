//! Flutter counterpart: `cupertino/list_tile.dart`.

use std::fmt;
use std::rc::Rc;

use reveal_embedder::FontWeight;
use reveal_foundation::{App, Handle, Listener};
use reveal_painting::{AnyColor, EdgeInsetsGeometry, TextOverflow};
use reveal_rendering::{BoxConstraints, CrossAxisAlignment, HitTestBehavior, MainAxisAlignment};
use reveal_widgets::{
    BuildContext, Center, ColoredBox, Column, ConstrainedBox, DefaultTextStyle, Expanded,
    GestureDetector, Icon, IntoWidget, KeyRef, Padding, Row, SizedBox, State, StateData,
    StatefulWidget, StatelessWidget, WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::icons::CupertinoIcons;
use crate::theme::CupertinoTheme;

// These constants were eyeballed from iOS 14.4 Settings app for base, Notes for
// notched without leading, and Reminders app for notched with leading.
const K_LEADING_SIZE: f64 = 28.0;
const K_NOTCHED_LEADING_SIZE: f64 = 30.0;
const K_MIN_HEIGHT: f64 = K_LEADING_SIZE + 2.0 * 8.0;
const K_MIN_HEIGHT_WITH_SUBTITLE: f64 = K_LEADING_SIZE + 2.0 * 10.0;
const K_NOTCHED_MIN_HEIGHT: f64 = K_NOTCHED_LEADING_SIZE + 2.0 * 12.0;
const K_NOTCHED_MIN_HEIGHT_WITHOUT_LEADING: f64 = K_NOTCHED_LEADING_SIZE + 2.0 * 10.0;
const K_PADDING: EdgeInsetsGeometry = EdgeInsetsGeometry::directional(20.0, 0.0, 14.0, 0.0);
const K_PADDING_WITH_SUBTITLE: EdgeInsetsGeometry =
    EdgeInsetsGeometry::directional(20.0, 0.0, 14.0, 0.0);
const K_NOTCHED_PADDING: EdgeInsetsGeometry = EdgeInsetsGeometry::symmetric(0.0, 14.0);
const K_NOTCHED_PADDING_WITHOUT_LEADING: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(28.0, 10.0, 14.0, 10.0);
const K_LEADING_TO_TITLE: f64 = 16.0;
const K_NOTCHED_LEADING_TO_TITLE: f64 = 12.0;
const K_NOTCHED_TITLE_TO_SUBTITLE: f64 = 3.0;
const K_ADDITIONAL_INFO_TO_TRAILING: f64 = 6.0;
const K_NOTCHED_TITLE_WITH_SUBTITLE_FONT_SIZE: f64 = 16.0;
const K_SUBTITLE_FONT_SIZE: f64 = 12.0;
const K_NOTCHED_SUBTITLE_FONT_SIZE: f64 = 14.0;

/// Dart's `_CupertinoListTileType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CupertinoListTileType {
    Base,
    Notched,
}

/// An iOS-style list tile.
///
/// The [`CupertinoListTile`] is a Cupertino equivalent of Material `ListTile`.
/// It comes in two forms, an old-fashioned edge-to-edge variant known from iOS
/// Settings app and in a new, "Inset Grouped" form, known from either iOS Notes
/// or Reminders app. The first is constructed using [`new`](Self::new), and the
/// latter using [`notched`](Self::notched).
///
/// The [`title`](Self::title), [`subtitle`](Self::subtitle), and
/// [`additional_info`](Self::additional_info) are usually `Text` widgets. They are all
/// limited to one line so it is a responsibility of the caller to take care of text
/// wrapping.
///
/// The size of [`leading`](Self::leading) is by default constrained to match the iOS size,
/// depending of the type of list tile. This can however be overridden by providing
/// [`leading_size`](Self::leading_size). The [`trailing`](Self::trailing) widget is not
/// constrained and is therefore a responsibility of the caller to ensure reasonable size of
/// the [`trailing`](Self::trailing) widget.
///
/// The background color of the tile can be set with
/// [`background_color`](Self::background_color) for the state before tile was tapped and with
/// [`background_color_activated`](Self::background_color_activated) for the state after the
/// tile was tapped. By default, both values are set to match the default iOS appearance.
///
/// The [`padding`](Self::padding) and [`leading_to_title`](Self::leading_to_title) are by
/// default set to match iOS but can be overwritten if necessary.
///
/// The [`on_tap`](Self::on_tap) callback provides an option to react to taps anywhere inside
/// the list tile. This can be used to navigate routes and according to iOS behavior it should
/// not be used for example to toggle the `CupertinoSwitch` in the trailing widget.
///
/// See also:
///
///  * [`CupertinoListSection`](crate::CupertinoListSection), an iOS-style list that is a
///    typical container for [`CupertinoListTile`].
pub struct CupertinoListTile {
    pub key: Option<KeyRef>,
    r#type: CupertinoListTileType,
    /// A [`title`](Self::title) is used to convey the central information. Usually a `Text`.
    pub title: WidgetRef,
    /// A [`subtitle`](Self::subtitle) is used to display additional information. It is
    /// located below [`title`](Self::title). Usually a `Text` widget.
    pub subtitle: Option<WidgetRef>,
    /// Similar to [`subtitle`](Self::subtitle), an [`additional_info`](Self::additional_info)
    /// is used to display additional information. However, instead of being displayed below
    /// [`title`](Self::title), it is displayed on the right, before
    /// [`trailing`](Self::trailing). Usually a `Text` widget.
    pub additional_info: Option<WidgetRef>,
    /// A widget displayed at the start of the [`CupertinoListTile`]. This is
    /// typically an `Icon` or an `Image`.
    pub leading: Option<WidgetRef>,
    /// A widget displayed at the end of the [`CupertinoListTile`]. This is usually
    /// a right chevron icon (e.g. [`CupertinoListTileChevron`]), or an `Icon`.
    pub trailing: Option<WidgetRef>,
    /// The [`on_tap`](Self::on_tap) function is called when a user taps on
    /// [`CupertinoListTile`]. If left `None`, the [`CupertinoListTile`] will not react on
    /// taps. The tile is active only for the duration of the invocation.
    pub on_tap: Option<Listener>,
    /// The [`background_color`](Self::background_color) of the tile in normal state. Once the
    /// tile is tapped, the background color switches to
    /// [`background_color_activated`](Self::background_color_activated). It is set to match
    /// the iOS look by default.
    pub background_color: Option<AnyColor>,
    /// The [`background_color_activated`](Self::background_color_activated) is the background
    /// color of the tile after the tile was tapped. It is set to match the iOS look by
    /// default.
    pub background_color_activated: Option<AnyColor>,
    /// Padding of the content inside [`CupertinoListTile`].
    pub padding: Option<EdgeInsetsGeometry>,
    /// The [`leading_size`](Self::leading_size) is used to constrain the width and height of
    /// [`leading`](Self::leading) widget.
    pub leading_size: f64,
    /// The horizontal space between [`leading`](Self::leading) widget and
    /// [`title`](Self::title).
    pub leading_to_title: f64,
}

impl CupertinoListTile {
    /// Creates an edge-to-edge iOS-style list tile like the tiles in iOS Settings app.
    ///
    /// The `title` is used to convey the most important information of list tile. It is
    /// typically a `Text`.
    pub fn new<K>(title: impl IntoWidget<K>) -> CupertinoListTile {
        CupertinoListTile::of_type(
            title.into_widget(),
            CupertinoListTileType::Base,
            K_LEADING_SIZE,
            K_LEADING_TO_TITLE,
        )
    }

    /// Creates a notched iOS-style list tile like the tiles in iOS Notes app or
    /// Reminders app.
    ///
    /// The `title` is used to convey the most important information of list tile. It is
    /// typically a `Text`.
    pub fn notched<K>(title: impl IntoWidget<K>) -> CupertinoListTile {
        CupertinoListTile::of_type(
            title.into_widget(),
            CupertinoListTileType::Notched,
            K_NOTCHED_LEADING_SIZE,
            K_NOTCHED_LEADING_TO_TITLE,
        )
    }

    fn of_type(
        title: WidgetRef,
        r#type: CupertinoListTileType,
        leading_size: f64,
        leading_to_title: f64,
    ) -> CupertinoListTile {
        CupertinoListTile {
            key: None,
            r#type,
            title,
            subtitle: None,
            additional_info: None,
            leading: None,
            trailing: None,
            on_tap: None,
            background_color: None,
            background_color_activated: None,
            padding: None,
            leading_size,
            leading_to_title,
        }
    }

    /// Dart `CupertinoListTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoListTile {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoListTile(subtitle:)`.
    pub fn subtitle<K>(mut self, subtitle: impl IntoWidget<K>) -> CupertinoListTile {
        self.subtitle = Some(subtitle.into_widget());
        self
    }

    /// Dart `CupertinoListTile(additionalInfo:)`.
    pub fn additional_info<K>(mut self, additional_info: impl IntoWidget<K>) -> CupertinoListTile {
        self.additional_info = Some(additional_info.into_widget());
        self
    }

    /// Dart `CupertinoListTile(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> CupertinoListTile {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `CupertinoListTile(trailing:)`.
    pub fn trailing<K>(mut self, trailing: impl IntoWidget<K>) -> CupertinoListTile {
        self.trailing = Some(trailing.into_widget());
        self
    }

    /// Dart `CupertinoListTile(onTap:)`.
    pub fn on_tap(mut self, on_tap: Listener) -> CupertinoListTile {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `CupertinoListTile(backgroundColor:)`.
    pub fn background_color(mut self, background_color: impl Into<AnyColor>) -> CupertinoListTile {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `CupertinoListTile(backgroundColorActivated:)`.
    pub fn background_color_activated(
        mut self,
        background_color_activated: impl Into<AnyColor>,
    ) -> CupertinoListTile {
        self.background_color_activated = Some(background_color_activated.into());
        self
    }

    /// Dart `CupertinoListTile(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> CupertinoListTile {
        self.padding = Some(padding);
        self
    }

    /// Dart `CupertinoListTile(leadingSize:)`.
    pub fn leading_size(mut self, leading_size: f64) -> CupertinoListTile {
        self.leading_size = leading_size;
        self
    }

    /// Dart `CupertinoListTile(leadingToTitle:)`.
    pub fn leading_to_title(mut self, leading_to_title: f64) -> CupertinoListTile {
        self.leading_to_title = leading_to_title;
        self
    }
}

impl fmt::Debug for CupertinoListTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoListTile")
            .field("type", &self.r#type)
            .field("title", &self.title)
            .field("subtitle", &self.subtitle)
            .field("leading", &self.leading)
            .field("trailing", &self.trailing)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoListTile {
    type State = CupertinoListTileState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoListTileState {
        CupertinoListTileState {
            state: StateData::new(),
            tapped: false,
        }
    }
}

/// Dart's `_CupertinoListTileState`.
pub struct CupertinoListTileState {
    state: StateData<CupertinoListTile>,
    tapped: bool,
}

impl CupertinoListTileState {
    fn handle_tap(self: Handle<Self>, app: &mut App) {
        if let Some(on_tap) = self.widget(app).on_tap.clone() {
            on_tap.call(app);
        }
        if self.mounted(app) {
            self.set_state(app, |state| {
                state.tapped = false;
            });
        }
    }
}

impl State for CupertinoListTileState {
    type Widget = CupertinoListTile;
    reveal_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_style = CupertinoTheme::of(app, context).text_theme().text_style();
        let colored_style = text_style.copy_with().color(CupertinoDynamicColor::resolve(
            &CupertinoColors::SECONDARY_LABEL,
            app,
            context,
        ));

        let widget = self.widget(app);
        let (tile_type, leading_size, leading_to_title) =
            (widget.r#type, widget.leading_size, widget.leading_to_title);
        let has_leading = widget.leading.is_some();
        let has_subtitle = widget.subtitle.is_some();
        let base_type = match tile_type {
            CupertinoListTileType::Base => true,
            CupertinoListTileType::Notched => false,
        };
        let title_style = if base_type || !has_subtitle {
            text_style.clone()
        } else {
            let style = text_style.copy_with().font_weight(FontWeight::W600);
            if has_leading {
                style
            } else {
                style.font_size(K_NOTCHED_TITLE_WITH_SUBTITLE_FONT_SIZE)
            }
        };
        let title = DefaultTextStyle::new(title_style, widget.title.clone())
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis);

        let padding = widget.padding.unwrap_or(match tile_type {
            CupertinoListTileType::Base if has_subtitle => K_PADDING_WITH_SUBTITLE,
            CupertinoListTileType::Notched if has_leading => K_NOTCHED_PADDING,
            CupertinoListTileType::Base => K_PADDING,
            CupertinoListTileType::Notched => K_NOTCHED_PADDING_WITHOUT_LEADING,
        });

        // The color for default state tile is set to either what user provided or
        // null and it will resolve to the correct color provided by context. But if
        // the tile was tapped, it is set to what user provided or if null to the
        // default color that matched the iOS-style.
        let mut background_color = widget
            .background_color
            .clone()
            .unwrap_or(CupertinoColors::TRANSPARENT);
        if app.get(self).tapped {
            background_color = match self.widget(app).background_color_activated.clone() {
                Some(background_color_activated) => background_color_activated,
                None => {
                    CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_GREY4, app, context)
                }
            };
        }

        let min_height = match tile_type {
            CupertinoListTileType::Base if has_subtitle => K_MIN_HEIGHT_WITH_SUBTITLE,
            CupertinoListTileType::Notched if has_leading => K_NOTCHED_MIN_HEIGHT,
            CupertinoListTileType::Base => K_MIN_HEIGHT,
            CupertinoListTileType::Notched => K_NOTCHED_MIN_HEIGHT_WITHOUT_LEADING,
        };

        let widget = self.widget(app);
        let mut row_children: Vec<WidgetRef> = Vec::new();
        match &widget.leading {
            Some(leading) => {
                row_children.push(
                    SizedBox::square(Some(leading_size))
                        .child(Center::new().child(leading.clone()))
                        .into_widget(),
                );
                row_children.push(SizedBox::new().width(leading_to_title).into_widget());
            }
            None => row_children.push(SizedBox::new().height(leading_size).into_widget()),
        }

        let mut column_children: Vec<WidgetRef> = vec![title.into_widget()];
        if let Some(subtitle) = &widget.subtitle {
            column_children.push(
                SizedBox::new()
                    .height(K_NOTCHED_TITLE_TO_SUBTITLE)
                    .into_widget(),
            );
            column_children.push(
                DefaultTextStyle::new(
                    colored_style.copy_with().font_size(if base_type {
                        K_SUBTITLE_FONT_SIZE
                    } else {
                        K_NOTCHED_SUBTITLE_FONT_SIZE
                    }),
                    subtitle.clone(),
                )
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis)
                .into_widget(),
            );
        }
        row_children.push(
            Expanded::new(
                Column::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .children(column_children),
            )
            .into_widget(),
        );

        if let Some(additional_info) = &widget.additional_info {
            row_children.push(
                DefaultTextStyle::new(colored_style.clone(), additional_info.clone())
                    .max_lines(1)
                    .into_widget(),
            );
            if widget.trailing.is_some() {
                row_children.push(
                    SizedBox::new()
                        .width(K_ADDITIONAL_INFO_TO_TRAILING)
                        .into_widget(),
                );
            }
        }
        if let Some(trailing) = &widget.trailing {
            row_children.push(trailing.clone());
        }

        let child = ConstrainedBox::new(
            BoxConstraints::new()
                .min_width(f64::INFINITY)
                .min_height(min_height),
        )
        .child(
            ColoredBox::new(background_color)
                .child(Padding::new(padding).child(Row::new().children(row_children))),
        )
        .into_widget();

        if widget.on_tap.is_none() {
            return child;
        }

        GestureDetector::new()
            .on_tap_down(Rc::new(move |app: &mut App, _| {
                self.set_state(app, |state| {
                    state.tapped = true;
                });
            }))
            .on_tap_cancel(Listener::new(move |app| {
                self.set_state(app, |state| {
                    state.tapped = false;
                });
            }))
            .on_tap(Listener::new(move |app| self.handle_tap(app)))
            .behavior(HitTestBehavior::Opaque)
            .child(child)
            .into_widget()
    }
}

/// A typical iOS trailing widget used to denote that a [`CupertinoListTile`] is a
/// button with an action.
///
/// The [`CupertinoListTileChevron`] is meant as a convenience implementation of
/// trailing right chevron.
#[derive(Debug)]
pub struct CupertinoListTileChevron {
    pub key: Option<KeyRef>,
}

impl CupertinoListTileChevron {
    /// Creates a typical widget used to denote that a [`CupertinoListTile`] is a
    /// button with action.
    pub fn new() -> CupertinoListTileChevron {
        CupertinoListTileChevron { key: None }
    }

    /// Dart `CupertinoListTileChevron(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoListTileChevron {
        self.key = Some(key);
        self
    }
}

impl Default for CupertinoListTileChevron {
    fn default() -> CupertinoListTileChevron {
        CupertinoListTileChevron::new()
    }
}

impl StatelessWidget for CupertinoListTileChevron {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let font_size = CupertinoTheme::of(app, context)
            .text_theme()
            .text_style()
            .font_size;
        let mut icon = Icon::new(Some(CupertinoIcons::right_chevron()));
        if let Some(font_size) = font_size {
            icon = icon.size(font_size);
        }
        icon.color(CupertinoDynamicColor::resolve(
            &CupertinoColors::SYSTEM_GREY2,
            app,
            context,
        ))
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use reveal_embedder::{
        Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, Size,
        TextDirection,
    };
    use reveal_gestures::GestureBinding;
    use reveal_rendering::AnyRenderBox;
    use reveal_widgets::{Builder, Directionality, GlobalKey, downcast_widget};

    use super::*;
    use crate::test_support::{build, test_cell};
    use crate::theme::CupertinoThemeData;

    fn key_of(key: &GlobalKey) -> KeyRef {
        Rc::new(key.clone())
    }

    /// A `SizedBox` of `width` x `height`, findable through `key`.
    fn slot(key: &GlobalKey, width: f64, height: f64) -> SizedBox {
        SizedBox::new().key(key_of(key)).width(width).height(height)
    }

    fn box_of(app: &mut App, key: &GlobalKey) -> AnyRenderBox {
        key.current_context(app)
            .expect("the slot mounted")
            .find_render_object(app)
            .expect("the slot has been laid out")
            .as_box()
            .expect("a box")
    }

    fn offset_of(app: &mut App, key: &GlobalKey) -> Offset {
        box_of(app, key).local_to_global(app, Offset::ZERO, None)
    }

    /// Mounts `tile` in a column, which hands it an unbounded height, as a list section does.
    fn mount<K>(cell: &AppCell, tile: impl IntoWidget<K>) {
        build(
            cell,
            Directionality::new(
                TextDirection::Ltr,
                Column::new().children([tile.into_widget()]),
            )
            .into_widget(),
        );
    }

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

    #[test]
    fn a_notched_tile_lays_out_its_five_slots_with_the_notched_paddings() {
        let cell = test_cell();
        let (tile_key, leading, title, subtitle, info, trailing) = (
            GlobalKey::new(),
            GlobalKey::new(),
            GlobalKey::new(),
            GlobalKey::new(),
            GlobalKey::new(),
            GlobalKey::new(),
        );
        mount(
            &cell,
            CupertinoListTile::notched(slot(&title, 100.0, 20.0))
                .key(key_of(&tile_key))
                .leading(slot(&leading, 24.0, 24.0))
                .subtitle(slot(&subtitle, 80.0, 14.0))
                .additional_info(slot(&info, 40.0, 16.0))
                .trailing(slot(&trailing, 12.0, 12.0)),
        );
        let mut app = cell.borrow_mut();

        assert_eq!(
            box_of(&mut app, &tile_key).size(&app),
            Size::new(400.0, K_NOTCHED_MIN_HEIGHT),
            "the notched minimum height with a leading widget"
        );
        assert_eq!(
            offset_of(&mut app, &leading),
            Offset::new(17.0, 15.0),
            "14 of padding, then centred in a 30-point leading box"
        );
        assert_eq!(
            offset_of(&mut app, &title),
            Offset::new(56.0, 8.5),
            "14 of padding, the 30-point leading box, and 12 to the title"
        );
        assert_eq!(
            offset_of(&mut app, &subtitle),
            Offset::new(56.0, 31.5),
            "3 below the title"
        );
        assert_eq!(
            offset_of(&mut app, &info),
            Offset::new(328.0, 19.0),
            "before the trailing widget and its 6-point gap"
        );
        assert_eq!(
            offset_of(&mut app, &trailing),
            Offset::new(374.0, 21.0),
            "14 of padding from the end"
        );
    }

    #[test]
    fn a_base_tile_uses_the_edge_to_edge_padding_and_minimum_height() {
        let cell = test_cell();
        let (tile_key, title) = (GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoListTile::new(slot(&title, 100.0, 20.0)).key(key_of(&tile_key)),
        );
        let mut app = cell.borrow_mut();

        assert_eq!(
            box_of(&mut app, &tile_key).size(&app),
            Size::new(400.0, K_MIN_HEIGHT)
        );
        assert_eq!(
            offset_of(&mut app, &title).dx(),
            20.0,
            "the edge-to-edge start padding; a missing leading widget takes no room"
        );
    }

    #[test]
    fn tapping_a_tile_activates_it_and_runs_on_tap() {
        let cell = test_cell();
        let taps = Rc::new(Cell::new(0));
        let tile_key = GlobalKey::new();
        let title = GlobalKey::new();
        mount(
            &cell,
            CupertinoListTile::new(slot(&title, 100.0, 20.0))
                .key(key_of(&tile_key))
                .on_tap(Listener::new({
                    let taps = Rc::clone(&taps);
                    move |_app| taps.set(taps.get() + 1)
                })),
        );
        let mut app = cell.borrow_mut();
        let state = tile_key
            .current_state::<CupertinoListTileState>(&mut app)
            .expect("the tile mounted");

        send(&mut app, PointerChange::Down, 200.0, 20.0, Duration::ZERO);
        assert!(app.get(state).tapped);

        send(
            &mut app,
            PointerChange::Up,
            200.0,
            20.0,
            Duration::from_millis(10),
        );
        assert_eq!(taps.get(), 1);
        assert!(
            !app.get(state).tapped,
            "the listener runs to completion, so the tile deactivates at once"
        );
    }

    #[test]
    fn the_chevron_is_a_grey_right_chevron_sized_to_the_theme_text() {
        let cell = test_cell();
        let seen: Rc<RefCell<Option<(WidgetRef, AnyColor)>>> = Rc::new(RefCell::new(None));
        build(
            &cell,
            CupertinoTheme::new(
                CupertinoThemeData::new(),
                Builder::new({
                    let seen = Rc::clone(&seen);
                    move |app, context| {
                        let chevron = CupertinoListTileChevron::new().build(app, context);
                        let grey = CupertinoDynamicColor::resolve(
                            &CupertinoColors::SYSTEM_GREY2,
                            app,
                            context,
                        );
                        *seen.borrow_mut() = Some((chevron, grey));
                        SizedBox::shrink().into_widget()
                    }
                }),
            )
            .into_widget(),
        );

        let seen = seen.borrow();
        let (chevron, grey) = seen.as_ref().expect("the chevron built");
        let icon = downcast_widget::<Icon>(&**chevron).expect("an Icon");
        assert_eq!(icon.icon, Some(CupertinoIcons::right_chevron()));
        assert_eq!(icon.size, Some(17.0), "the theme text style's font size");
        assert_eq!(icon.color.as_ref(), Some(grey));
    }
}
