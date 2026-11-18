//! Flutter counterpart: material_ui's `user_accounts_drawer_header.dart`.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, Curve, CurvedAnimation, Curves,
};
use waft_embedder::{Color, Offset, Size, TextDirection};
use waft_foundation::{App, Handle, Listener, Ref, ValueKey};
use waft_painting::{
    BoxDecoration, DecorationRef, EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry,
    TextOverflow,
};
use waft_rendering::{
    BoxConstraints, ChildLayoutId, MultiChildLayoutChildren, MultiChildLayoutDelegate,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    BuildContext, Center, Column, CrossAxisAlignment, CustomMultiChildLayout, DefaultTextStyle,
    Directionality, Expanded, Icon, IntoWidget, KeyRef, LayoutId, Padding, Positioned,
    PositionedDirectional, Row, SafeArea, Semantics, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, SizedBox, Stack, State, StateData, StatefulWidget,
    StatelessWidget, Transform, WidgetRef, debug_check_has_directionality,
};

use crate::colors::Colors;
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::drawer_header::DrawerHeader;
use crate::icons::Icons;
use crate::ink_well::InkWell;
use crate::material_localizations::MaterialLocalizations;
use crate::theme::Theme;

const K_ACCOUNT_DETAILS_HEIGHT: f64 = 56.0;

fn account_name_id() -> ChildLayoutId {
    Rc::new(ValueKey::new("accountName"))
}

fn account_email_id() -> ChildLayoutId {
    Rc::new(ValueKey::new("accountEmail"))
}

fn dropdown_icon_id() -> ChildLayoutId {
    Rc::new(ValueKey::new("dropdownIcon"))
}

/// Dart's `_AccountPictures`.
struct AccountPictures {
    current_account_picture: Option<WidgetRef>,
    other_accounts_pictures: Option<Vec<WidgetRef>>,
    current_account_picture_size: Option<Size>,
    other_accounts_pictures_size: Option<Size>,
}

impl AccountPictures {
    fn new(
        current_account_picture: Option<WidgetRef>,
        other_accounts_pictures: Option<Vec<WidgetRef>>,
        current_account_picture_size: Option<Size>,
        other_accounts_pictures_size: Option<Size>,
    ) -> AccountPictures {
        AccountPictures {
            current_account_picture,
            other_accounts_pictures,
            current_account_picture_size,
            other_accounts_pictures_size,
        }
    }
}

impl fmt::Debug for AccountPictures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountPictures").finish_non_exhaustive()
    }
}

impl StatelessWidget for AccountPictures {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let other_pictures = self
            .other_accounts_pictures
            .clone()
            .unwrap_or_default()
            .into_iter()
            .take(3)
            .map(|picture| {
                Padding::new(EdgeInsetsDirectional::only(8.0, 0.0, 0.0, 0.0))
                    .child(Semantics::new().container(true).child(
                        Padding::new(EdgeInsets::from_ltrb(8.0, 0.0, 0.0, 8.0)).child(
                            SizedBox::from_size(self.other_accounts_pictures_size).child(picture),
                        ),
                    ))
                    .into_widget()
            })
            .collect::<Vec<_>>();

        let mut current_picture = SizedBox::from_size(self.current_account_picture_size);
        if let Some(current_account_picture) = &self.current_account_picture {
            current_picture = current_picture.child(current_account_picture.clone());
        }

        Stack::new()
            .children(vec![
                PositionedDirectional::new(Row::new().children(other_pictures))
                    .top(0.0)
                    .end(0.0)
                    .into_widget(),
                Positioned::new(
                    Semantics::new()
                        .explicit_child_nodes(true)
                        .child(current_picture),
                )
                .top(0.0)
                .into_widget(),
            ])
            .into_widget()
    }
}

/// Dart's `_AccountDetails`.
struct AccountDetails {
    account_name: Option<WidgetRef>,
    account_email: Option<WidgetRef>,
    on_tap: Option<Listener>,
    is_open: bool,
    arrow_color: Option<Color>,
}

impl AccountDetails {
    fn new(
        account_name: Option<WidgetRef>,
        account_email: Option<WidgetRef>,
        on_tap: Option<Listener>,
        is_open: bool,
        arrow_color: Option<Color>,
    ) -> AccountDetails {
        AccountDetails {
            account_name,
            account_email,
            on_tap,
            is_open,
            arrow_color,
        }
    }
}

impl fmt::Debug for AccountDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountDetails").finish_non_exhaustive()
    }
}

impl StatefulWidget for AccountDetails {
    type State = AccountDetailsState;

    fn create_state(&self) -> AccountDetailsState {
        AccountDetailsState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            controller: None,
            animation: None,
        }
    }
}

/// Dart's `_AccountDetailsState`.
struct AccountDetailsState {
    state: StateData<AccountDetails>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    controller: Option<Ref<AnimationController>>,
    animation: Option<Ref<CurvedAnimation>>,
}

impl AccountDetailsState {
    fn controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .controller
            .clone()
            .expect("created in init_state")
    }

    fn animation(self: Handle<Self>, app: &App) -> Ref<CurvedAnimation> {
        app.get(self)
            .animation
            .clone()
            .expect("created in init_state")
    }

    fn handle_animation_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
    }
}

impl fmt::Debug for AccountDetailsState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountDetailsState")
            .finish_non_exhaustive()
    }
}

impl SingleTickerProviderStateMixin for AccountDetailsState {
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

/// Dart's `vsync: this`.
impl TickerProviderObject for AccountDetailsState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for AccountDetailsState {
    type Widget = AccountDetails;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let is_open = self.widget(app).is_open;
        let controller = AnimationController::new(
            app,
            Some(if is_open { 1.0 } else { 0.0 }),
            Some(Duration::from_millis(200)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let animation = CurvedAnimation::new(
            app,
            controller.erase(),
            Curves::fast_out_slow_in(),
            Some(Rc::new(Curves::fast_out_slow_in().flipped()) as Rc<dyn Curve>),
        );
        animation.add_listener(
            app,
            Listener::handle_method(self, Self::handle_animation_changed),
        );
        let state = app.get_mut(self);
        state.controller = Some(controller);
        state.animation = Some(animation);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.animation(app);
        let controller = self.controller(app);
        controller.dispose(app);
        animation.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AccountDetails) {
        if old_widget.is_open == self.widget(app).is_open {
            return;
        }
        let controller = self.controller(app);
        if self.widget(app).is_open {
            controller.forward(app, None);
        } else {
            controller.reverse(app, None);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_directionality(
            app, context, None, None, None
        ));
        debug_assert!(debug_check_has_material_localizations(app, context));

        let theme = Theme::of(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let is_open = self.widget(app).is_open;
        let arrow_color = self.widget(app).arrow_color;

        let mut children: Vec<WidgetRef> = Vec::new();
        if let Some(account_name) = &self.widget(app).account_name {
            children.push(
                LayoutId::new(
                    account_name_id(),
                    Padding::new(EdgeInsets::symmetric(2.0, 0.0)).child(
                        DefaultTextStyle::new(
                            theme
                                .primary_text_theme
                                .body_large
                                .clone()
                                .expect("the primary text theme has a body large"),
                            account_name.clone(),
                        )
                        .overflow(TextOverflow::Ellipsis),
                    ),
                )
                .into_widget(),
            );
        }
        if let Some(account_email) = &self.widget(app).account_email {
            children.push(
                LayoutId::new(
                    account_email_id(),
                    Padding::new(EdgeInsets::symmetric(2.0, 0.0)).child(
                        DefaultTextStyle::new(
                            theme
                                .primary_text_theme
                                .body_medium
                                .clone()
                                .expect("the primary text theme has a body medium"),
                            account_email.clone(),
                        )
                        .overflow(TextOverflow::Ellipsis),
                    ),
                )
                .into_widget(),
            );
        }
        if let Some(on_tap) = self.widget(app).on_tap.clone() {
            let semantic_label = if is_open {
                localizations.hide_accounts_label()
            } else {
                localizations.show_accounts_label()
            };
            let on_tap_for_semantics: waft_foundation::VoidCallback = Rc::new(move |app| {
                on_tap.call(app);
            });
            let mut icon = Icon::new(Some(Icons::arrow_drop_down())).semantic_label(semantic_label);
            if let Some(arrow_color) = arrow_color {
                icon = icon.color(arrow_color);
            }
            children.push(
                LayoutId::new(
                    dropdown_icon_id(),
                    Semantics::new()
                        .container(true)
                        .button(true)
                        .on_tap(on_tap_for_semantics)
                        .child(SizedBox::square(Some(K_ACCOUNT_DETAILS_HEIGHT)).child(
                            Center::new().child(
                                Transform::rotate(self.animation(app).value(app) * PI).child(icon),
                            ),
                        )),
                )
                .into_widget(),
            );
        }

        let mut account_details = CustomMultiChildLayout::new(Rc::new(AccountDetailsLayout {
            text_direction: Directionality::of(app, context),
        }))
        .children(children)
        .into_widget();

        if let Some(on_tap) = self.widget(app).on_tap.clone() {
            account_details = InkWell::new()
                .on_tap(on_tap)
                .exclude_from_semantics(true)
                .child(account_details)
                .into_widget();
        }

        SizedBox::new()
            .height(K_ACCOUNT_DETAILS_HEIGHT)
            .child(account_details)
            .into_widget()
    }
}

/// Dart's `_AccountDetailsLayout`.
#[derive(Debug)]
struct AccountDetailsLayout {
    text_direction: TextDirection,
}

impl AccountDetailsLayout {
    fn offset_for_icon(&self, size: Size, icon_size: Size) -> Offset {
        match self.text_direction {
            TextDirection::Ltr => Offset::new(
                size.width() - icon_size.width(),
                size.height() - icon_size.height(),
            ),
            TextDirection::Rtl => Offset::new(0.0, size.height() - icon_size.height()),
        }
    }

    fn offset_for_bottom_line(
        &self,
        size: Size,
        icon_size: Size,
        bottom_line_size: Size,
    ) -> Offset {
        let y = size.height() - 0.5 * icon_size.height() - 0.5 * bottom_line_size.height();
        match self.text_direction {
            TextDirection::Ltr => Offset::new(0.0, y),
            TextDirection::Rtl => Offset::new(size.width() - bottom_line_size.width(), y),
        }
    }

    fn offset_for_name(&self, size: Size, name_size: Size, bottom_line_offset: Offset) -> Offset {
        let y = bottom_line_offset.dy() - name_size.height();
        match self.text_direction {
            TextDirection::Ltr => Offset::new(0.0, y),
            TextDirection::Rtl => Offset::new(size.width() - name_size.width(), y),
        }
    }
}

impl MultiChildLayoutDelegate for AccountDetailsLayout {
    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size) {
        let mut icon_size = None;
        if children.has_child(&dropdown_icon_id()) {
            let size_of_icon =
                children.layout_child(app, &dropdown_icon_id(), BoxConstraints::loose(size));
            children.position_child(
                app,
                &dropdown_icon_id(),
                self.offset_for_icon(size, size_of_icon),
            );
            icon_size = Some(size_of_icon);
        }

        let bottom_line = if children.has_child(&account_email_id()) {
            Some(account_email_id())
        } else if children.has_child(&account_name_id()) {
            Some(account_name_id())
        } else {
            None
        };

        if let Some(bottom_line) = bottom_line {
            let constraint_size = match icon_size {
                Some(icon_size) => Size::new(size.width() - icon_size.width(), size.height()),
                None => size,
            };
            let icon_size = icon_size.unwrap_or(Size::new(
                K_ACCOUNT_DETAILS_HEIGHT,
                K_ACCOUNT_DETAILS_HEIGHT,
            ));

            let bottom_line_size =
                children.layout_child(app, &bottom_line, BoxConstraints::loose(constraint_size));
            let bottom_line_offset = self.offset_for_bottom_line(size, icon_size, bottom_line_size);
            children.position_child(app, &bottom_line, bottom_line_offset);

            if bottom_line == account_email_id() && children.has_child(&account_name_id()) {
                let name_size = children.layout_child(
                    app,
                    &account_name_id(),
                    BoxConstraints::loose(constraint_size),
                );
                children.position_child(
                    app,
                    &account_name_id(),
                    self.offset_for_name(size, name_size, bottom_line_offset),
                );
            }
        }
    }

    fn should_relayout(&self, _old_delegate: &dyn MultiChildLayoutDelegate) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Material Design drawer header that identifies the app's user.
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// See also:
///
///  * `DrawerHeader`, for a drawer header that doesn't show user accounts.
///  * <https://material.io/design/components/navigation-drawer.html#anatomy>
pub struct UserAccountsDrawerHeader {
    pub key: Option<KeyRef>,
    /// The header's background. If decoration is null then a `BoxDecoration`
    /// with its background color set to the current theme's primaryColor is used.
    pub decoration: Option<DecorationRef>,
    /// The margin around the drawer header.
    pub margin: Option<EdgeInsetsGeometry>,
    /// A widget placed in the upper-left corner that represents the current
    /// user's account. Normally a `CircleAvatar`.
    pub current_account_picture: Option<WidgetRef>,
    /// A list of widgets that represent the current user's other accounts.
    ///
    /// Up to three of these widgets will be arranged in a row in the header's
    /// upper-right corner. Normally a list of `CircleAvatar` widgets.
    pub other_accounts_pictures: Option<Vec<WidgetRef>>,
    /// The size of the `currentAccountPicture`.
    pub current_account_picture_size: Size,
    /// The size of each widget in `otherAccountsPicturesSize`.
    pub other_accounts_pictures_size: Size,
    /// A widget that represents the user's current account name. It is
    /// displayed on the left, below the `currentAccountPicture`.
    pub account_name: Option<WidgetRef>,
    /// A widget that represents the email address of the user's current account.
    /// It is displayed on the left, below the `accountName`.
    pub account_email: Option<WidgetRef>,
    /// A callback that is called when the horizontal area which contains the
    /// `accountName` and `accountEmail` is tapped.
    pub on_details_pressed: Option<Listener>,
    /// The `Color` of the arrow icon.
    pub arrow_color: Color,
}

impl UserAccountsDrawerHeader {
    /// Creates a Material Design drawer header; Dart's optional named arguments are the setters.
    ///
    /// Requires one of its ancestors to be a `Material` widget.
    pub fn new(
        account_name: Option<WidgetRef>,
        account_email: Option<WidgetRef>,
    ) -> UserAccountsDrawerHeader {
        UserAccountsDrawerHeader {
            key: None,
            decoration: None,
            margin: Some(EdgeInsets::only(0.0, 0.0, 0.0, 8.0).into()),
            current_account_picture: None,
            other_accounts_pictures: None,
            current_account_picture_size: Size::square(72.0),
            other_accounts_pictures_size: Size::square(40.0),
            account_name,
            account_email,
            on_details_pressed: None,
            arrow_color: Colors::WHITE,
        }
    }

    /// Dart `UserAccountsDrawerHeader(key:)`.
    pub fn key(mut self, key: KeyRef) -> UserAccountsDrawerHeader {
        self.key = Some(key);
        self
    }

    /// Dart `UserAccountsDrawerHeader(decoration:)`.
    pub fn decoration(mut self, decoration: impl Into<DecorationRef>) -> UserAccountsDrawerHeader {
        self.decoration = Some(decoration.into());
        self
    }

    /// Dart `UserAccountsDrawerHeader(margin:)`.
    pub fn margin(mut self, margin: Option<EdgeInsetsGeometry>) -> UserAccountsDrawerHeader {
        self.margin = margin;
        self
    }

    /// Dart `UserAccountsDrawerHeader(currentAccountPicture:)`.
    pub fn current_account_picture<K>(
        mut self,
        current_account_picture: impl IntoWidget<K>,
    ) -> UserAccountsDrawerHeader {
        self.current_account_picture = Some(current_account_picture.into_widget());
        self
    }

    /// Dart `UserAccountsDrawerHeader(otherAccountsPictures:)`.
    pub fn other_accounts_pictures(
        mut self,
        other_accounts_pictures: impl IntoIterator<Item = WidgetRef>,
    ) -> UserAccountsDrawerHeader {
        self.other_accounts_pictures = Some(other_accounts_pictures.into_iter().collect());
        self
    }

    /// Dart `UserAccountsDrawerHeader(currentAccountPictureSize:)`.
    pub fn current_account_picture_size(
        mut self,
        current_account_picture_size: Size,
    ) -> UserAccountsDrawerHeader {
        self.current_account_picture_size = current_account_picture_size;
        self
    }

    /// Dart `UserAccountsDrawerHeader(otherAccountsPicturesSize:)`.
    pub fn other_accounts_pictures_size(
        mut self,
        other_accounts_pictures_size: Size,
    ) -> UserAccountsDrawerHeader {
        self.other_accounts_pictures_size = other_accounts_pictures_size;
        self
    }

    /// Dart `UserAccountsDrawerHeader(onDetailsPressed:)`.
    pub fn on_details_pressed(mut self, on_details_pressed: Listener) -> UserAccountsDrawerHeader {
        self.on_details_pressed = Some(on_details_pressed);
        self
    }

    /// Dart `UserAccountsDrawerHeader(arrowColor:)`.
    pub fn arrow_color(mut self, arrow_color: Color) -> UserAccountsDrawerHeader {
        self.arrow_color = arrow_color;
        self
    }
}

impl fmt::Debug for UserAccountsDrawerHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserAccountsDrawerHeader")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for UserAccountsDrawerHeader {
    type State = UserAccountsDrawerHeaderState;

    fn create_state(&self) -> UserAccountsDrawerHeaderState {
        UserAccountsDrawerHeaderState {
            state: StateData::new(),
            is_open: false,
        }
    }
}

/// Dart's `_UserAccountsDrawerHeaderState`.
pub struct UserAccountsDrawerHeaderState {
    state: StateData<UserAccountsDrawerHeader>,
    is_open: bool,
}

impl UserAccountsDrawerHeaderState {
    fn handle_details_pressed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.is_open = !state.is_open);
        let on_details_pressed = self
            .widget(app)
            .on_details_pressed
            .clone()
            .expect("the tap handler is only used when one is set");
        on_details_pressed.call(app);
    }
}

impl fmt::Debug for UserAccountsDrawerHeaderState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserAccountsDrawerHeaderState")
            .field("is_open", &self.is_open)
            .finish_non_exhaustive()
    }
}

impl State for UserAccountsDrawerHeaderState {
    type Widget = UserAccountsDrawerHeader;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));

        let theme = Theme::of(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let widget = self.widget(app);
        let decoration = widget.decoration.clone().unwrap_or_else(|| {
            BoxDecoration::new()
                .color(theme.color_scheme.primary())
                .into()
        });
        let is_open = app.get(self).is_open;
        let on_tap = widget
            .on_details_pressed
            .as_ref()
            .map(|_| Listener::handle_method(self, Self::handle_details_pressed));

        let details = AccountDetails::new(
            widget.account_name.clone(),
            widget.account_email.clone(),
            on_tap,
            is_open,
            Some(widget.arrow_color),
        );
        let pictures = AccountPictures::new(
            widget.current_account_picture.clone(),
            widget.other_accounts_pictures.clone(),
            Some(widget.current_account_picture_size),
            Some(widget.other_accounts_pictures_size),
        );

        let child = SafeArea::new(
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children(vec![
                    Expanded::new(
                        Padding::new(EdgeInsetsDirectional::only(0.0, 0.0, 16.0, 0.0))
                            .child(pictures),
                    )
                    .into_widget(),
                    details.into_widget(),
                ]),
        )
        .bottom(false);

        Semantics::new()
            .container(true)
            .label(localizations.signed_in_label())
            .child(
                DrawerHeader::new(Some(child.into_widget()))
                    .decoration(decoration)
                    .margin(widget.margin)
                    .padding(EdgeInsetsDirectional::only(16.0, 16.0, 0.0, 0.0)),
            )
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_embedder::{Size, TextDirection};
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{
        Center, Directionality, ExcludeSemantics, IntoWidget, MediaQuery, MediaQueryData, SizedBox,
        StateSetter, StatefulBuilder, Text,
    };

    use super::*;
    use crate::circle_avatar::CircleAvatar;
    use crate::color_scheme::ColorScheme;
    use crate::material::Material;
    use crate::scaffold::Scaffold as MaterialScaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// Dart's `pumpTestWidget`.
    struct Options {
        with_name: bool,
        with_email: bool,
        with_on_details_pressed_handler: bool,
        other_accounts_picture_size: Size,
        current_account_picture_size: Size,
        primary_color: Option<Color>,
        color_scheme_primary: Option<Color>,
    }

    impl Default for Options {
        fn default() -> Options {
            Options {
                with_name: true,
                with_email: true,
                with_on_details_pressed_handler: true,
                other_accounts_picture_size: Size::square(40.0),
                current_account_picture_size: Size::square(72.0),
                primary_color: None,
                color_scheme_primary: None,
            }
        }
    }

    fn avatar(name: &'static str) -> CircleAvatar {
        CircleAvatar::new().child(Text::new(name))
    }

    fn keyed_avatar(name: &'static str) -> CircleAvatar {
        avatar(name).key(key(name))
    }

    fn header(options: &Options) -> UserAccountsDrawerHeader {
        let mut header = UserAccountsDrawerHeader::new(
            options.with_name.then(|| Text::new("name").into_widget()),
            options.with_email.then(|| Text::new("email").into_widget()),
        )
        .current_account_picture_size(options.current_account_picture_size)
        .other_accounts_pictures_size(options.other_accounts_picture_size)
        .current_account_picture(ExcludeSemantics::new().child(keyed_avatar("A")))
        .other_accounts_pictures(vec![
            avatar("B").into_widget(),
            keyed_avatar("C").into_widget(),
            keyed_avatar("D").into_widget(),
            avatar("E").into_widget(),
        ]);
        if options.with_on_details_pressed_handler {
            header = header.on_details_pressed(Listener::new(|_app| {}));
        }
        header
    }

    fn pump_test_widget(tester: &mut WidgetTester, options: Options) {
        let mut builder = ThemeData::builder();
        if let Some(primary_color) = options.primary_color {
            builder = builder.primary_color(primary_color);
        }
        let mut color_scheme = ColorScheme::light();
        if let Some(color_scheme_primary) = options.color_scheme_primary {
            color_scheme = color_scheme.with_primary(color_scheme_primary);
        }
        let theme = builder
            .color_scheme(color_scheme)
            .build(&tester.cell.borrow());

        let mut data = MediaQueryData::new();
        data.padding = EdgeInsets::only(10.0, 20.0, 30.0, 40.0);
        let home = MediaQuery::new(
            data,
            Material::new()
                .child(Center::new().child(header(&options)))
                .into_widget(),
        );
        tester.pump_widget(material_app(theme, home.into_widget()));
    }

    fn key(key: &'static str) -> KeyRef {
        KeyRef::from(ValueKey::new(key))
    }

    #[test]
    fn the_header_inherits_the_color_scheme_primary() {
        test_widgets(|tester| {
            let primary_color = Color::new(0xFF00FF00);
            let color_scheme_primary = Color::new(0xFF0000FF);
            pump_test_widget(
                tester,
                Options {
                    primary_color: Some(primary_color),
                    color_scheme_primary: Some(color_scheme_primary),
                    ..Options::default()
                },
            );

            let decoration = tester
                .widget(find::by_type::<DrawerHeader>(), |header: &DrawerHeader| {
                    header.decoration.clone()
                })
                .expect("the header has a decoration");
            let decoration = decoration
                .as_any()
                .downcast_ref::<BoxDecoration>()
                .expect("a box decoration");
            let color = decoration.color.as_ref().map(|color| color.color());
            assert_ne!(color, Some(primary_color));
            assert_eq!(color, Some(color_scheme_primary));
        });
    }

    #[test]
    fn the_header_lays_out_the_pictures_and_the_account() {
        test_widgets(|tester| {
            pump_test_widget(tester, Options::default());

            for text in ["A", "B", "C", "D"] {
                assert!(
                    !find::text(text).evaluate(tester).is_empty(),
                    "{text} is shown"
                );
            }
            assert!(
                find::text("E").evaluate(tester).is_empty(),
                "only three others"
            );
            assert_eq!(find::text("name").evaluate(tester).len(), 1);
            assert_eq!(find::text("email").evaluate(tester).len(), 1);
            assert_eq!(tester.get_size(find::by_key(key("A"))), Size::square(72.0));
            assert_eq!(tester.get_size(find::by_key(key("C"))), Size::square(40.0));
            assert_eq!(
                tester
                    .get_size(find::by_type::<UserAccountsDrawerHeader>())
                    .height(),
                160.0 + 20.0 + 8.0 + 1.0
            );

            let top_left = tester.get_top_left(find::by_type::<UserAccountsDrawerHeader>());
            let top_right = tester.get_top_right(find::by_type::<UserAccountsDrawerHeader>());
            let a = tester.get_top_left(find::by_key(key("A")));
            let d = tester.get_top_right(find::by_key(key("D")));
            let c = tester.get_top_right(find::by_key(key("C")));

            assert_eq!(a.dx() - top_left.dx(), 16.0 + 10.0);
            assert_eq!(a.dy() - top_left.dy(), 16.0 + 20.0);
            assert_eq!(top_right.dx() - d.dx(), 16.0 + 30.0);
            assert_eq!(d.dy() - top_right.dy(), 16.0 + 20.0);
            assert_eq!(d.dx() - c.dx(), 40.0 + 16.0);
        });
    }

    #[test]
    fn the_picture_sizes_can_change() {
        test_widgets(|tester| {
            pump_test_widget(
                tester,
                Options {
                    current_account_picture_size: Size::square(60.0),
                    other_accounts_picture_size: Size::square(30.0),
                    ..Options::default()
                },
            );
            assert_eq!(tester.get_size(find::by_key(key("A"))), Size::square(60.0));
            assert_eq!(tester.get_size(find::by_key(key("C"))), Size::square(30.0));
        });
    }

    /// Dart's `transform.getRotation()[0]` and `[4]` of the first transform in the header: the
    /// cosine of the arrow's turn, twice.
    fn rotation(tester: &WidgetTester) -> (f32, f32) {
        tester.widget(
            find::descendant(
                find::by_type::<UserAccountsDrawerHeader>(),
                find::by_type::<Transform>(),
            )
            .first(),
            |transform: &Transform| {
                let storage = transform.transform.to_flutter_array();
                (storage[0], storage[5])
            },
        )
    }

    const RIGHT_SIDE_UP: (f32, f32) = (1.0, 1.0);
    const UPSIDE_DOWN: (f32, f32) = (-1.0, -1.0);

    #[test]
    fn the_arrow_turns_half_a_turn_and_back() {
        test_widgets(|tester| {
            pump_test_widget(tester, Options::default());

            // Icon is right side up.
            assert_eq!(rotation(tester), RIGHT_SIDE_UP);

            tester.tap(find::by_type::<Icon>());
            tester.pump();
            tester.pump_by(Duration::from_millis(10));
            assert!(tester.has_running_animations());

            tester.pump_and_settle();
            tester.pump();

            // Icon has rotated 180 degrees.
            assert_eq!(rotation(tester), UPSIDE_DOWN);

            tester.tap(find::by_type::<Icon>());
            tester.pump();
            tester.pump_by(Duration::from_millis(10));
            assert!(tester.has_running_animations());

            tester.pump_and_settle();
            tester.pump();

            // Icon has rotated 180 degrees back to the original position.
            assert_eq!(rotation(tester), RIGHT_SIDE_UP);
        });
    }

    #[test]
    fn the_arrow_keeps_turning_through_four_mid_animation_taps() {
        test_widgets(|tester| {
            pump_test_widget(tester, Options::default());
            assert_eq!(rotation(tester), RIGHT_SIDE_UP);

            // Down, up, down again and back up, each mid animation.
            for _ in 0..4 {
                tester.tap(find::by_type::<Icon>());
                tester.pump();
                tester.pump_by(Duration::from_millis(100));
                assert!(tester.has_running_animations());
            }

            tester.pump_and_settle();
            tester.pump();
            assert_eq!(rotation(tester), RIGHT_SIDE_UP);
        });
    }

    #[test]
    fn a_rebuild_does_not_turn_the_arrow() {
        test_widgets(|tester| {
            let set_state: Rc<RefCell<Option<StateSetter>>> = Rc::default();
            let captured = Rc::clone(&set_state);
            let home = StatefulBuilder::new(move |_app, _context, set_state| {
                *captured.borrow_mut() = Some(set_state);
                UserAccountsDrawerHeader::new(
                    Some(Text::new("name").into_widget()),
                    Some(Text::new("email").into_widget()),
                )
                .on_details_pressed(Listener::new(|_app| {}))
                .into_widget()
            });
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new().child(home).into_widget(),
            ));
            assert_eq!(rotation(tester), RIGHT_SIDE_UP);

            let set_state = set_state.borrow().clone().expect("the builder ran");
            set_state(&mut tester.cell.borrow_mut(), &mut |_app: &mut App| {});
            tester.pump_by(Duration::from_millis(10));
            assert!(!tester.has_running_animations());

            assert_eq!(tester.pump_and_settle(), 1);

            // Icon has not rotated.
            assert_eq!(rotation(tester), RIGHT_SIDE_UP);
        });
    }

    #[test]
    fn the_arrow_color_defaults_to_white_and_can_be_set() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        UserAccountsDrawerHeader::new(
                            Some(Text::new("name").into_widget()),
                            Some(Text::new("email").into_widget()),
                        )
                        .on_details_pressed(Listener::new(|_app| {})),
                    )
                    .into_widget(),
            ));
            let color = tester.widget(find::by_type::<Icon>(), |icon: &Icon| {
                icon.color.as_ref().map(|color| color.color())
            });
            assert_eq!(color, Some(Colors::WHITE));

            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        UserAccountsDrawerHeader::new(
                            Some(Text::new("name").into_widget()),
                            Some(Text::new("email").into_widget()),
                        )
                        .on_details_pressed(Listener::new(|_app| {}))
                        .arrow_color(Colors::RED.primary()),
                    )
                    .into_widget(),
            ));
            let color = tester.widget(find::by_type::<Icon>(), |icon: &Icon| {
                icon.color.as_ref().map(|color| color.color())
            });
            assert_eq!(color, Some(Colors::RED.primary()));
        });
    }

    /// The arguments of the null parameters cases' `buildFrame`; `margin` is always null.
    #[derive(Default)]
    struct Frame {
        current_account_picture: Option<WidgetRef>,
        other_accounts_pictures: Option<Vec<WidgetRef>>,
        account_name: Option<&'static str>,
        account_email: Option<&'static str>,
        on_details_pressed: bool,
    }

    fn pump_frame(tester: &mut WidgetTester, direction: TextDirection, frame: Frame) {
        let mut header = UserAccountsDrawerHeader::new(
            frame.account_name.map(|name| Text::new(name).into_widget()),
            frame
                .account_email
                .map(|email| Text::new(email).into_widget()),
        )
        .margin(None);
        if let Some(current_account_picture) = frame.current_account_picture {
            header = header.current_account_picture(current_account_picture);
        }
        if let Some(other_accounts_pictures) = frame.other_accounts_pictures {
            header = header.other_accounts_pictures(other_accounts_pictures);
        }
        if frame.on_details_pressed {
            header = header.on_details_pressed(Listener::new(|_app| {}));
        }
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Directionality::new(
                direction,
                Material::new().child(Center::new().child(header)),
            )
            .into_widget(),
        ));
    }

    /// Dart's `UserAccountsDrawerHeader null parameters LTR` and `RTL`: `before` is whether
    /// the start of `direction` has the smaller x.
    fn check_null_parameters(tester: &mut WidgetTester, direction: TextDirection) {
        let ltr = direction == TextDirection::Ltr;
        let before = |a: f64, b: f64| if ltr { a < b } else { a > b };
        let icon = find::by_type::<Icon>;

        pump_frame(tester, direction, Frame::default());
        assert_eq!(
            tester
                .get_size(find::by_type::<UserAccountsDrawerHeader>())
                .height(),
            160.0 + 1.0 // height + bottom edge
        );
        assert!(icon().evaluate(tester).is_empty());

        pump_frame(
            tester,
            direction,
            Frame {
                on_details_pressed: true,
                ..Frame::default()
            },
        );
        assert_eq!(icon().evaluate(tester).len(), 1);

        for (account_name, account_email) in
            [(Some("accountName"), None), (None, Some("accountEmail"))]
        {
            pump_frame(
                tester,
                direction,
                Frame {
                    account_name,
                    account_email,
                    on_details_pressed: true,
                    ..Frame::default()
                },
            );
            let line =
                tester.get_center(find::text(account_name.or(account_email).expect("a line")));
            let arrow = tester.get_center(icon());
            assert_eq!(line.dy(), arrow.dy());
            assert!(before(line.dx(), arrow.dx()));
        }

        pump_frame(
            tester,
            direction,
            Frame {
                account_name: Some("accountName"),
                account_email: Some("accountEmail"),
                on_details_pressed: true,
                ..Frame::default()
            },
        );
        let email = tester.get_center(find::text("accountEmail"));
        let arrow = tester.get_center(icon());
        assert_eq!(email.dy(), arrow.dy());
        assert!(before(email.dx(), arrow.dx()));
        assert!(
            tester.get_bottom_left(find::text("accountEmail")).dy()
                > tester.get_bottom_left(find::text("accountName")).dy()
        );
        if ltr {
            assert_eq!(
                tester.get_bottom_left(find::text("accountEmail")).dx(),
                tester.get_bottom_left(find::text("accountName")).dx()
            );
        } else {
            assert_eq!(
                tester.get_bottom_right(find::text("accountEmail")).dx(),
                tester.get_bottom_right(find::text("accountName")).dx()
            );
        }

        pump_frame(
            tester,
            direction,
            Frame {
                current_account_picture: Some(avatar("A").into_widget()),
                ..Frame::default()
            },
        );
        assert_eq!(find::text("A").evaluate(tester).len(), 1);

        pump_frame(
            tester,
            direction,
            Frame {
                other_accounts_pictures: Some(vec![avatar("A").into_widget()]),
                ..Frame::default()
            },
        );
        assert_eq!(find::text("A").evaluate(tester).len(), 1);

        pump_frame(
            tester,
            direction,
            Frame {
                current_account_picture: Some(keyed_avatar("A").into_widget()),
                account_name: Some("accountName"),
                ..Frame::default()
            },
        );
        if ltr {
            assert_eq!(
                tester.get_bottom_left(find::by_key(key("A"))).dx(),
                tester.get_bottom_left(find::text("accountName")).dx()
            );
        } else {
            assert_eq!(
                tester.get_bottom_right(find::by_key(key("A"))).dx(),
                tester.get_bottom_right(find::text("accountName")).dx()
            );
        }
        assert!(
            tester.get_bottom_left(find::text("accountName")).dy()
                > tester.get_bottom_left(find::by_key(key("A"))).dy()
        );
    }

    #[test]
    fn user_accounts_drawer_header_null_parameters_ltr() {
        test_widgets(|tester| check_null_parameters(tester, TextDirection::Ltr));
    }

    #[test]
    fn user_accounts_drawer_header_null_parameters_rtl() {
        test_widgets(|tester| check_null_parameters(tester, TextDirection::Rtl));
    }

    #[test]
    fn the_header_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                MaterialScaffold::new()
                    .body(Center::new().child(SizedBox::shrink().child(
                        UserAccountsDrawerHeader::new(
                            Some(Text::new("X").into_widget()),
                            Some(Text::new("Y").into_widget()),
                        ),
                    )))
                    .into_widget(),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<UserAccountsDrawerHeader>()),
                Size::ZERO
            );
        });
    }
}
