//! Flutter counterpart: `cupertino/search_field.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{
    Radius, Size, SmartDashesType, SmartQuotesType, TextInputAction, TextInputType,
};
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref, VoidCallback};
use waft_gestures::GestureTapCallback;
use waft_painting::{AnyColor, BorderRadius, BoxDecoration, EdgeInsetsGeometry, TextStyle};
use waft_widgets::{
    BuildContext, Directionality, FocusNode, FocusNodeLeaf, FocusNodeRef, Icon, IconTheme,
    IconThemeData, IntoWidget, KeyRef, MediaQuery, Opacity, Padding, RestorableListenable,
    RestorableProperty, RestorableTextEditingController, RestorationBucket, RestorationMixin,
    RestorationMixinData, ScrollNotification, ScrollNotificationCallback,
    ScrollNotificationObserver, ScrollNotificationObserverState, ScrollUpdateNotification, State,
    StateData, StatefulWidget, TextEditingControllerRef, WidgetRef,
};

use crate::button::CupertinoButton;
use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::icons::CupertinoIcons;
use crate::localizations::CupertinoLocalizations;
use crate::text_field::{CupertinoTextField, OverlayVisibilityMode};

// The fraction of the height of the search text field after which its contents
// completely fade out when resized on scroll.
//
// Eyeballed on an iPhone 15 simulator running iOS 17.5.
const K_MIN_HEIGHT_BEFORE_TOTAL_TRANSPARENCY: f64 = 4.0 / 5.0;

// The maximum icon size of the prefix icon of a focused search field before it
// is hidden in higher accessibility text scale modes.
//
// Eyeballed on an iPhone 15 simulator running iOS 17.5.
const K_MAX_PREFIX_ICON_SIZE: f64 = 30.0;

/// A [`CupertinoTextField`] that mimics the look and behavior of UIKit's
/// `UISearchTextField`.
///
/// This control defaults to showing the basic parts of a `UISearchTextField`,
/// like the 'Search' placeholder, prefix-ed Search icon, and suffix-ed
/// X-Mark icon.
///
/// To control the text that is displayed in the text field, use the
/// [`controller`](Self::controller). For example, to set the initial value of the text field, use
/// a [`controller`](Self::controller) that already contains some text.
///
/// It is recommended to pass a `ValueChanged<String>` to both [`on_changed`](Self::on_changed) and
/// [`on_submitted`](Self::on_submitted) parameters in order to be notified once the value of the
/// field changes or is submitted by the keyboard.
///
/// See also:
///
///  * <https://developer.apple.com/design/human-interface-guidelines/ios/bars/search-bars/>
#[allow(clippy::type_complexity)]
pub struct CupertinoSearchTextField {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,
    /// Controls the text being edited.
    ///
    /// Similar to [`CupertinoTextField`], to provide a prefilled text entry, pass
    /// in a `TextEditingController` with an initial value to the [`controller`](Self::controller)
    /// parameter. Defaults to creating its own `TextEditingController`.
    pub controller: Option<TextEditingControllerRef>,
    /// Invoked upon user input.
    pub on_changed: Option<Rc<dyn Fn(&mut App, String)>>,
    /// Invoked upon keyboard submission.
    pub on_submitted: Option<Rc<dyn Fn(&mut App, String)>>,
    /// Allows changing the style of the text.
    ///
    /// Defaults to the gray [`CupertinoColors::SECONDARY_LABEL`] iOS color.
    pub style: Option<TextStyle>,
    /// A hint placeholder text that appears when the text entry is empty.
    ///
    /// Defaults to 'Search' localized in each supported language.
    pub placeholder: Option<String>,
    /// Sets the style of the placeholder of the text field.
    ///
    /// Defaults to the gray [`CupertinoColors::SECONDARY_LABEL`] iOS color.
    pub placeholder_style: Option<TextStyle>,
    /// Sets the decoration for the text field.
    ///
    /// This property is automatically set using the [`background_color`](Self::background_color)
    /// and [`border_radius`](Self::border_radius) properties, which both have default values.
    /// Therefore, [`decoration`](Self::decoration) has a default value upon building the widget.
    /// It is designed to mimic the look of a `UISearchTextField`.
    pub decoration: Option<BoxDecoration>,
    /// Set the [`decoration`](Self::decoration) property's background color.
    ///
    /// Can't be set along with the [`decoration`](Self::decoration). Defaults to the translucent
    /// [`CupertinoColors::TERTIARY_SYSTEM_FILL`] iOS color.
    pub background_color: Option<AnyColor>,
    /// Sets the [`decoration`](Self::decoration) property's border radius.
    ///
    /// Can't be set along with the [`decoration`](Self::decoration). Defaults to 9 px circular
    /// corner radius.
    pub border_radius: Option<BorderRadius>,
    /// The keyboard type for this search field.
    ///
    /// Defaults to [`TextInputType::TEXT`].
    pub keyboard_type: Option<TextInputType>,
    /// Sets the padding insets for the text and placeholder.
    ///
    /// Defaults to padding that replicates the `UISearchTextField` look. The
    /// inset values were determined using the comparison tool in
    /// <https://github.com/flutter/platform_tests/>.
    pub padding: EdgeInsetsGeometry,
    /// Sets the color for the suffix and prefix icons.
    ///
    /// Defaults to [`CupertinoColors::SECONDARY_LABEL`].
    pub item_color: AnyColor,
    /// Sets the base icon size for the suffix and prefix icons.
    ///
    /// The size of the icon is scaled using the accessibility font scale
    /// settings. Defaults to `20.0`.
    pub item_size: f64,
    /// Sets the padding insets for the suffix.
    ///
    /// Defaults to padding that replicates the `UISearchTextField` suffix look.
    /// The inset values were determined using the comparison tool in
    /// <https://github.com/flutter/platform_tests/>.
    pub prefix_insets: EdgeInsetsGeometry,
    /// Sets a prefix widget.
    ///
    /// Defaults to an [`Icon`] widget with the [`CupertinoIcons::search`] icon.
    pub prefix_icon: WidgetRef,
    /// Sets the padding insets for the prefix.
    ///
    /// Defaults to padding that replicates the `UISearchTextField` prefix look.
    /// The inset values were determined using the comparison tool in
    /// <https://github.com/flutter/platform_tests/>.
    pub suffix_insets: EdgeInsetsGeometry,
    /// Sets the suffix widget's icon, an [`Icon`].
    ///
    /// Defaults to the X-Mark [`CupertinoIcons::xmark_circle_fill`]. "To change the
    /// functionality of the suffix icon, provide a custom on_suffix_tap callback
    /// and specify an intuitive suffix_icon.
    pub suffix_icon: WidgetRef,
    /// Dictates when the X-Mark (suffix) should be visible.
    ///
    /// Defaults to only on when editing.
    pub suffix_mode: OverlayVisibilityMode,
    /// Sets the X-Mark (suffix) action.
    ///
    /// Defaults to clearing the text. The suffix action is customizable
    /// so that users can override it with other functionality, that isn't
    /// necessarily clearing text.
    pub on_suffix_tap: Option<VoidCallback>,
    /// {@macro cupertino_ui.textfield.restorationId}
    pub restoration_id: Option<String>,
    /// {@macro flutter.widgets.Focus.focusNode}
    pub focus_node: Option<FocusNodeRef>,
    /// {@macro flutter.widgets.editableText.autofocus}
    pub autofocus: bool,
    /// {@macro cupertino_ui.textfield.onTap}
    pub on_tap: Option<GestureTapCallback>,
    /// Whether to enable autocorrection.
    ///
    /// Defaults to true.
    pub autocorrect: bool,
    /// Whether to allow the platform to automatically format quotes.
    ///
    /// This flag only affects iOS, where it is equivalent to `UITextSmartQuotesType`.
    ///
    /// If set to `None`, [`SmartQuotesType::Enabled`] will be used.
    ///
    /// Defaults to `None`.
    pub smart_quotes_type: Option<SmartQuotesType>,
    /// Whether to allow the platform to automatically format dashes.
    ///
    /// This flag only affects iOS versions 11 and above, where it is equivalent to
    /// `UITextSmartDashesType`.
    ///
    /// If set to `None`, [`SmartDashesType::Enabled`] will be used.
    ///
    /// Defaults to `None`.
    pub smart_dashes_type: Option<SmartDashesType>,
    /// {@macro flutter.services.TextInputConfiguration.enableIMEPersonalizedLearning}
    pub enable_ime_personalized_learning: bool,
    /// Disables the text field when false.
    ///
    /// Text fields in disabled states have a light grey background and don't
    /// respond to touch events including the [`prefix_icon`](Self::prefix_icon) and
    /// [`suffix_icon`](Self::suffix_icon) button.
    pub enabled: Option<bool>,
    /// {@macro flutter.widgets.editableText.cursorWidth}
    pub cursor_width: f64,
    /// {@macro flutter.widgets.editableText.cursorHeight}
    pub cursor_height: Option<f64>,
    /// {@macro flutter.widgets.editableText.cursorRadius}
    pub cursor_radius: Radius,
    /// {@macro flutter.widgets.editableText.cursorOpacityAnimates}
    pub cursor_opacity_animates: bool,
    /// The color to use when painting the cursor.
    pub cursor_color: Option<AnyColor>,
}

impl CupertinoSearchTextField {
    /// Creates a [`CupertinoTextField`] that mimics the look and behavior of
    /// UIKit's `UISearchTextField`.
    pub fn new() -> CupertinoSearchTextField {
        CupertinoSearchTextField {
            key: None,
            controller: None,
            on_changed: None,
            on_submitted: None,
            style: None,
            placeholder: None,
            placeholder_style: None,
            decoration: None,
            background_color: None,
            border_radius: None,
            keyboard_type: Some(TextInputType::TEXT),
            padding: EdgeInsetsGeometry::from_steb(5.5, 8.0, 5.5, 8.0),
            item_color: CupertinoColors::SECONDARY_LABEL,
            item_size: 20.0,
            prefix_insets: EdgeInsetsGeometry::from_steb(6.0, 8.0, 0.0, 8.0),
            prefix_icon: Icon::new(Some(CupertinoIcons::search())).into_widget(),
            suffix_insets: EdgeInsetsGeometry::from_steb(0.0, 8.0, 5.0, 8.0),
            suffix_icon: Icon::new(Some(CupertinoIcons::xmark_circle_fill())).into_widget(),
            suffix_mode: OverlayVisibilityMode::Editing,
            on_suffix_tap: None,
            restoration_id: None,
            focus_node: None,
            smart_quotes_type: None,
            smart_dashes_type: None,
            enable_ime_personalized_learning: true,
            autofocus: false,
            on_tap: None,
            autocorrect: true,
            enabled: None,
            cursor_width: 2.0,
            cursor_height: None,
            cursor_radius: Radius::circular(2.0),
            cursor_opacity_animates: true,
            cursor_color: None,
        }
    }

    /// Dart `CupertinoSearchTextField(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSearchTextField {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoSearchTextField(controller:)`.
    pub fn controller(mut self, controller: TextEditingControllerRef) -> CupertinoSearchTextField {
        self.controller = Some(controller);
        self
    }

    /// Dart `CupertinoSearchTextField(onChanged:)`.
    pub fn on_changed(
        mut self,
        on_changed: impl Fn(&mut App, String) + 'static,
    ) -> CupertinoSearchTextField {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `CupertinoSearchTextField(onSubmitted:)`.
    pub fn on_submitted(
        mut self,
        on_submitted: impl Fn(&mut App, String) + 'static,
    ) -> CupertinoSearchTextField {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }

    /// Dart `CupertinoSearchTextField(style:)`.
    pub fn style(mut self, style: TextStyle) -> CupertinoSearchTextField {
        self.style = Some(style);
        self
    }

    /// Dart `CupertinoSearchTextField(placeholder:)`.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> CupertinoSearchTextField {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Dart `CupertinoSearchTextField(placeholderStyle:)`.
    pub fn placeholder_style(mut self, placeholder_style: TextStyle) -> CupertinoSearchTextField {
        self.placeholder_style = Some(placeholder_style);
        self
    }

    /// Dart `CupertinoSearchTextField(decoration:)`; not with a background color or a border
    /// radius.
    pub fn decoration(mut self, decoration: BoxDecoration) -> CupertinoSearchTextField {
        self.decoration = Some(decoration);
        self
    }

    /// Dart `CupertinoSearchTextField(backgroundColor:)`; not with a decoration.
    pub fn background_color(
        mut self,
        background_color: impl Into<AnyColor>,
    ) -> CupertinoSearchTextField {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `CupertinoSearchTextField(borderRadius:)`; not with a decoration.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> CupertinoSearchTextField {
        self.border_radius = Some(border_radius);
        self
    }

    /// Dart `CupertinoSearchTextField(keyboardType:)`.
    pub fn keyboard_type(
        mut self,
        keyboard_type: Option<TextInputType>,
    ) -> CupertinoSearchTextField {
        self.keyboard_type = keyboard_type;
        self
    }

    /// Dart `CupertinoSearchTextField(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> CupertinoSearchTextField {
        self.padding = padding.into();
        self
    }

    /// Dart `CupertinoSearchTextField(itemColor:)`.
    pub fn item_color(mut self, item_color: impl Into<AnyColor>) -> CupertinoSearchTextField {
        self.item_color = item_color.into();
        self
    }

    /// Dart `CupertinoSearchTextField(itemSize:)`.
    pub fn item_size(mut self, item_size: f64) -> CupertinoSearchTextField {
        self.item_size = item_size;
        self
    }

    /// Dart `CupertinoSearchTextField(prefixInsets:)`.
    pub fn prefix_insets(
        mut self,
        prefix_insets: impl Into<EdgeInsetsGeometry>,
    ) -> CupertinoSearchTextField {
        self.prefix_insets = prefix_insets.into();
        self
    }

    /// Dart `CupertinoSearchTextField(prefixIcon:)`.
    pub fn prefix_icon<K>(mut self, prefix_icon: impl IntoWidget<K>) -> CupertinoSearchTextField {
        self.prefix_icon = prefix_icon.into_widget();
        self
    }

    /// Dart `CupertinoSearchTextField(suffixInsets:)`.
    pub fn suffix_insets(
        mut self,
        suffix_insets: impl Into<EdgeInsetsGeometry>,
    ) -> CupertinoSearchTextField {
        self.suffix_insets = suffix_insets.into();
        self
    }

    /// Dart `CupertinoSearchTextField(suffixIcon:)`.
    pub fn suffix_icon(mut self, suffix_icon: Icon) -> CupertinoSearchTextField {
        self.suffix_icon = suffix_icon.into_widget();
        self
    }

    /// Dart `CupertinoSearchTextField(suffixMode:)`.
    pub fn suffix_mode(mut self, suffix_mode: OverlayVisibilityMode) -> CupertinoSearchTextField {
        self.suffix_mode = suffix_mode;
        self
    }

    /// Dart `CupertinoSearchTextField(onSuffixTap:)`.
    pub fn on_suffix_tap(mut self, on_suffix_tap: VoidCallback) -> CupertinoSearchTextField {
        self.on_suffix_tap = Some(on_suffix_tap);
        self
    }

    /// Dart `CupertinoSearchTextField(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> CupertinoSearchTextField {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `CupertinoSearchTextField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> CupertinoSearchTextField {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CupertinoSearchTextField(smartQuotesType:)`.
    pub fn smart_quotes_type(
        mut self,
        smart_quotes_type: SmartQuotesType,
    ) -> CupertinoSearchTextField {
        self.smart_quotes_type = Some(smart_quotes_type);
        self
    }

    /// Dart `CupertinoSearchTextField(smartDashesType:)`.
    pub fn smart_dashes_type(
        mut self,
        smart_dashes_type: SmartDashesType,
    ) -> CupertinoSearchTextField {
        self.smart_dashes_type = Some(smart_dashes_type);
        self
    }

    /// Dart `CupertinoSearchTextField(enableIMEPersonalizedLearning:)`.
    pub fn enable_ime_personalized_learning(
        mut self,
        enable_ime_personalized_learning: bool,
    ) -> CupertinoSearchTextField {
        self.enable_ime_personalized_learning = enable_ime_personalized_learning;
        self
    }

    /// Dart `CupertinoSearchTextField(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> CupertinoSearchTextField {
        self.autofocus = autofocus;
        self
    }

    /// Dart `CupertinoSearchTextField(onTap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> CupertinoSearchTextField {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `CupertinoSearchTextField(autocorrect:)`.
    pub fn autocorrect(mut self, autocorrect: bool) -> CupertinoSearchTextField {
        self.autocorrect = autocorrect;
        self
    }

    /// Dart `CupertinoSearchTextField(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> CupertinoSearchTextField {
        self.enabled = Some(enabled);
        self
    }

    /// Dart `CupertinoSearchTextField(cursorWidth:)`.
    pub fn cursor_width(mut self, cursor_width: f64) -> CupertinoSearchTextField {
        self.cursor_width = cursor_width;
        self
    }

    /// Dart `CupertinoSearchTextField(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> CupertinoSearchTextField {
        self.cursor_height = Some(cursor_height);
        self
    }

    /// Dart `CupertinoSearchTextField(cursorRadius:)`.
    pub fn cursor_radius(mut self, cursor_radius: Radius) -> CupertinoSearchTextField {
        self.cursor_radius = cursor_radius;
        self
    }

    /// Dart `CupertinoSearchTextField(cursorOpacityAnimates:)`.
    pub fn cursor_opacity_animates(
        mut self,
        cursor_opacity_animates: bool,
    ) -> CupertinoSearchTextField {
        self.cursor_opacity_animates = cursor_opacity_animates;
        self
    }

    /// Dart `CupertinoSearchTextField(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: impl Into<AnyColor>) -> CupertinoSearchTextField {
        self.cursor_color = Some(cursor_color.into());
        self
    }
}

impl Default for CupertinoSearchTextField {
    fn default() -> CupertinoSearchTextField {
        CupertinoSearchTextField::new()
    }
}

impl Debug for CupertinoSearchTextField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoSearchTextField")
            .field("controller", &self.controller)
            .field("focus_node", &self.focus_node)
            .field("placeholder", &self.placeholder)
            .field("enabled", &self.enabled)
            .field("suffix_mode", &self.suffix_mode)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoSearchTextField {
    type State = CupertinoSearchTextFieldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    /// Dart's constructor asserts: a decoration is given alone, or a background color and a border
    /// radius stand for it.
    fn debug_check_constructor(&self) {
        debug_assert!(
            !(self.decoration.is_some() && self.background_color.is_some()),
            "Cannot provide both a background color and a decoration\n\
             The backgroundColor argument is just a shorthand for \
             \"decoration: BoxDecoration(color: backgroundColor)\".\n\
             To use both a backgroundColor and other decoration properties, set the color in the \
             BoxDecoration instead."
        );
        debug_assert!(
            !(self.decoration.is_some() && self.border_radius.is_some()),
            "Cannot provide both a border radius and a decoration\n\
             The borderRadius argument is just a shorthand for \
             \"decoration: BoxDecoration(borderRadius: borderRadius)\".\n\
             To use both a radius and other decoration properties, set the radius in the \
             BoxDecoration instead."
        );
    }

    fn create_state(&self) -> CupertinoSearchTextFieldState {
        CupertinoSearchTextFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            controller: None,
            focus_node: None,
            scroll_notification_observer: None,
            scroll_notification_listener: None,
            scaled_icon_size: 0.0,
            fade_extent: 0.0,
        }
    }
}

/// Dart's `_CupertinoSearchTextFieldState`.
pub struct CupertinoSearchTextFieldState {
    state: StateData<CupertinoSearchTextField>,
    restoration: RestorationMixinData,
    controller: Option<Ref<RestorableTextEditingController>>,
    focus_node: Option<Ref<FocusNode>>,
    scroll_notification_observer: Option<Handle<ScrollNotificationObserverState>>,
    /// Dart's `_handleScrollNotification` tear-off, made once so it compares equal across calls.
    scroll_notification_listener: Option<ScrollNotificationCallback>,
    scaled_icon_size: f64,
    fade_extent: f64,
}

impl CupertinoSearchTextFieldState {
    /// Default value for the border radius. Radius value was determined using the
    /// comparison tool in <https://github.com/flutter/platform_tests/>.
    const K_DEFAULT_BORDER_RADIUS: BorderRadius = BorderRadius::all(Radius::circular(9.0));

    fn effective_controller(self: Handle<Self>, app: &App) -> TextEditingControllerRef {
        if let Some(controller) = self.widget(app).controller.clone() {
            return controller;
        }
        let restorable = app
            .get(self)
            .controller
            .handle()
            .expect("a local controller exists when the widget does not supply one");
        RestorableListenable::value(restorable, app)
            .keep(app)
            .into_erased()
    }

    fn effective_focus_node(self: Handle<Self>, app: &App) -> FocusNodeRef {
        self.widget(app)
            .focus_node
            .clone()
            .or_else(|| app.get(self).focus_node.as_ref().map(Ref::erase))
            .expect("init_state creates a local focus node")
    }

    fn register_controller(self: Handle<Self>, app: &mut App) {
        let controller = app
            .get(self)
            .controller
            .handle()
            .expect("register_controller needs a local controller");
        self.register_for_restoration(app, controller.as_property(), "controller");
    }

    fn create_local_controller(
        self: Handle<Self>,
        app: &mut App,
        value: Option<waft_services::TextEditingValue>,
    ) {
        debug_assert!(app.get(self).controller.is_none());
        let controller = match value {
            None => RestorableTextEditingController::new(app),
            Some(value) => RestorableTextEditingController::from_value(app, value),
        };
        app.get_mut(self).controller = Some(controller);
        if !self.restore_pending(app) {
            self.register_controller(app);
        }
    }

    fn default_on_suffix_tap(self: Handle<Self>, app: &mut App) {
        let controller = self.effective_controller(app);
        let text_changed = !controller.text_value(app).is_empty();
        controller.clear(app);
        if let Some(on_changed) = self.widget(app).on_changed.clone()
            && text_changed
        {
            let text = controller.text_value(app).to_owned();
            on_changed(app, text);
        }
    }

    fn scroll_notification_listener(
        self: Handle<Self>,
        app: &mut App,
    ) -> ScrollNotificationCallback {
        if let Some(listener) = app.get(self).scroll_notification_listener.clone() {
            return listener;
        }
        let listener: ScrollNotificationCallback =
            Rc::new(move |app, notification| self.handle_scroll_notification(app, notification));
        app.get_mut(self).scroll_notification_listener = Some(Rc::clone(&listener));
        listener
    }

    fn handle_scroll_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn ScrollNotification,
    ) {
        if notification
            .as_any()
            .downcast_ref::<ScrollUpdateNotification>()
            .is_some()
        {
            let current_height = self
                .context(app)
                .size(app)
                .map_or(0.0, |size| size.height());
            let widget = self.widget(app);
            let max_height = app.get(self).scaled_icon_size
                + widget
                    .prefix_insets
                    .vertical()
                    .max(widget.suffix_insets.vertical());
            self.set_state(app, |state| {
                state.fade_extent = Self::calculate_scroll_opacity(current_height, max_height);
            });
        }
    }

    fn calculate_scroll_opacity(current_height: f64, max_height: f64) -> f64 {
        let threshold_height = max_height * K_MIN_HEIGHT_BEFORE_TOTAL_TRANSPARENCY;
        if current_height >= max_height {
            0.0
        } else if current_height <= threshold_height {
            1.0
        } else {
            let range = max_height - threshold_height;
            let progress = (current_height - threshold_height) / range;
            1.0 - progress
        }
    }

    // Animate the top padding so that the contents of the search field
    // move upwards when the search text field is resized on scroll.
    fn animated_insets(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        insets: EdgeInsetsGeometry,
    ) -> EdgeInsetsGeometry {
        let current_insets = insets.resolve(Some(Directionality::of(app, context)));
        let animated_insets = EdgeInsetsGeometry::lerp(
            Some(insets),
            Some(EdgeInsetsGeometry::from(current_insets.copy_with(
                None,
                Some(current_insets.top / 2.0),
                None,
                None,
            ))),
            app.get(self).fade_extent,
        );
        animated_insets.unwrap_or(insets)
    }
}

impl RestorationMixin for CupertinoSearchTextFieldState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<RestorationBucket>>,
        _initial_restore: bool,
    ) {
        if app.get(self).controller.is_some() {
            self.register_controller(app);
        }
    }
}

impl State for CupertinoSearchTextFieldState {
    type Widget = CupertinoSearchTextField;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if self.widget(app).controller.is_none() {
            self.create_local_controller(app, None);
        }
        if self.widget(app).focus_node.is_none() {
            app.get_mut(self).focus_node = Some(FocusNode::new(app));
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
        let listener = self.scroll_notification_listener(app);
        if let Some(observer) = app.get(self).scroll_notification_observer {
            observer.remove_listener(app, &listener);
        }
        let context = self.context(app);
        let observer = ScrollNotificationObserver::maybe_of(app, context);
        app.get_mut(self).scroll_notification_observer = observer;
        if let Some(observer) = observer {
            observer.add_listener(app, listener);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoSearchTextField) {
        self.did_update_restoration_id(app);
        if self.widget(app).controller.is_none() && old_widget.controller.is_some() {
            let value = old_widget
                .controller
                .as_ref()
                .map(|controller| controller.value(app).clone());
            self.create_local_controller(app, value);
        } else if self.widget(app).controller.is_some()
            && old_widget.controller.is_none()
            && let Some(controller) = app.get(self).controller.handle()
        {
            self.unregister_from_restoration(app, controller.as_property());
            RestorableProperty::dispose(controller, app);
            app.get_mut(self).controller = None;
        }
        if self.widget(app).focus_node.is_none() && old_widget.focus_node.is_some() {
            app.get_mut(self).focus_node = Some(FocusNode::new(app));
        } else if self.widget(app).focus_node.is_some()
            && old_widget.focus_node.is_none()
            && let Some(focus_node) = app.get(self).focus_node.handle()
        {
            focus_node.dispose(app);
            app.get_mut(self).focus_node = None;
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get(self).scroll_notification_observer {
            let listener = self.scroll_notification_listener(app);
            observer.remove_listener(app, &listener);
            app.get_mut(self).scroll_notification_observer = None;
        }
        if self.widget(app).focus_node.is_none()
            && let Some(focus_node) = app.get(self).focus_node.handle()
        {
            focus_node.dispose(app);
        }
        if self.widget(app).controller.is_none()
            && let Some(controller) = app.get(self).controller.handle()
        {
            RestorableProperty::dispose(controller, app);
        }
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let placeholder = match self.widget(app).placeholder.clone() {
            Some(placeholder) => placeholder,
            None => {
                <dyn CupertinoLocalizations>::of(app, context).search_text_field_placeholder_label()
            }
        };
        let default_placeholder_color =
            CupertinoDynamicColor::resolve(&CupertinoColors::SECONDARY_LABEL, app, context).color();
        let fade_extent = app.get(self).fade_extent;
        let placeholder_style = self
            .widget(app)
            .placeholder_style
            .clone()
            .unwrap_or_else(|| {
                TextStyle::new().color(default_placeholder_color.with_alpha(
                    (255.0 * (default_placeholder_color.a * (1.0 - fade_extent))).round() as i32,
                ))
            });

        // The icon size will be scaled by a factor of the accessibility text scale,
        // to follow the behavior of `UISearchTextField`.
        let item_size = self.widget(app).item_size;
        let scaled_icon_size = MediaQuery::text_scaler_of(app, context).scale(item_size);
        app.get_mut(self).scaled_icon_size = scaled_icon_size;

        // If decoration was not provided, create a decoration with the provided
        // background color and border radius.
        let widget = self.widget(app);
        let decoration = widget.decoration.clone().unwrap_or_else(|| {
            BoxDecoration::new()
                .color(
                    widget
                        .background_color
                        .clone()
                        .unwrap_or(CupertinoColors::TERTIARY_SYSTEM_FILL),
                )
                .border_radius(
                    widget
                        .border_radius
                        .unwrap_or(Self::K_DEFAULT_BORDER_RADIUS),
                )
        });

        let item_color = widget.item_color.clone();
        let icon_color = CupertinoDynamicColor::resolve(&item_color, app, context);
        let suffix_icon_theme_data = IconThemeData::new()
            .color(icon_color.clone())
            .size(scaled_icon_size);
        let focus_node = self.effective_focus_node(app);
        let prefix_icon_theme_data = IconThemeData::new().color(icon_color).size(
            if scaled_icon_size >= K_MAX_PREFIX_ICON_SIZE && focus_node.has_focus(app) {
                0.0
            } else {
                scaled_icon_size
            },
        );

        let prefix_insets = self.widget(app).prefix_insets;
        let prefix_icon = self.widget(app).prefix_icon.clone();
        let prefix = Opacity::new(1.0 - fade_extent).child(
            Padding::new(self.animated_insets(app, context, prefix_insets))
                .child(IconTheme::new(prefix_icon_theme_data, prefix_icon)),
        );

        let suffix_insets = self.widget(app).suffix_insets;
        let suffix_icon = self.widget(app).suffix_icon.clone();
        let on_suffix_tap = match self.widget(app).on_suffix_tap.clone() {
            Some(on_suffix_tap) => Listener::new(move |app| on_suffix_tap(app)),
            None => Listener::handle_method(self, Self::default_on_suffix_tap),
        };
        let suffix = Opacity::new(1.0 - fade_extent).child(
            Padding::new(self.animated_insets(app, context, suffix_insets)).child(
                CupertinoButton::new(
                    IconTheme::new(suffix_icon_theme_data, suffix_icon).into_widget(),
                    Some(on_suffix_tap),
                )
                .minimum_size(Size::new(0.0, 0.0))
                .padding(EdgeInsetsGeometry::ZERO),
            ),
        );

        let controller = self.effective_controller(app);
        let padding = self.widget(app).padding;
        let animated_padding = self.animated_insets(app, context, padding);
        let widget = self.widget(app);
        let field = CupertinoTextField::new()
            .controller(controller)
            .decoration(Some(decoration))
            .prefix(prefix)
            .suffix(suffix)
            .enabled(widget.enabled.unwrap_or(true))
            .cursor_width(widget.cursor_width)
            .cursor_radius(widget.cursor_radius)
            .cursor_opacity_animates(widget.cursor_opacity_animates)
            .suffix_mode(widget.suffix_mode)
            .placeholder(placeholder)
            .placeholder_style(Some(placeholder_style))
            .padding(animated_padding)
            .focus_node(focus_node)
            .autofocus(widget.autofocus)
            .autocorrect(Some(widget.autocorrect))
            .enable_ime_personalized_learning(widget.enable_ime_personalized_learning)
            .text_input_action(TextInputAction::Search);
        let field = match widget.style.clone() {
            Some(style) => field.style(style),
            None => field,
        };
        let field = match widget.keyboard_type {
            Some(keyboard_type) => field.keyboard_type(keyboard_type),
            None => field,
        };
        let field = match widget.on_tap.clone() {
            Some(on_tap) => field.on_tap(on_tap),
            None => field,
        };
        let field = match widget.cursor_height {
            Some(cursor_height) => field.cursor_height(cursor_height),
            None => field,
        };
        let field = match widget.cursor_color.clone() {
            Some(cursor_color) => field.cursor_color(cursor_color),
            None => field,
        };
        let field = match widget.on_changed.clone() {
            Some(on_changed) => field.on_changed(move |app, value| on_changed(app, value)),
            None => field,
        };
        let field = match widget.on_submitted.clone() {
            Some(on_submitted) => field.on_submitted(move |app, value| on_submitted(app, value)),
            None => field,
        };
        let field = match widget.smart_quotes_type {
            Some(smart_quotes_type) => field.smart_quotes_type(smart_quotes_type),
            None => field,
        };
        let field = match widget.smart_dashes_type {
            Some(smart_dashes_type) => field.smart_dashes_type(smart_dashes_type),
            None => field,
        };
        field.into_widget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use waft_embedder::{
        Brightness, Color, FontWeight, Offset, PointerChange, PointerData, PointerDataPacket,
        PointerDeviceKind, TargetPlatform, TextAffinity, TextDirection, TextSelection,
    };
    use waft_foundation::AppCell;
    use waft_gestures::GestureBinding;
    use waft_painting::PaintingBinding;
    use waft_test::{TestPlatform, TestView};
    use waft_widgets::{
        AnyElement, Center, DecoratedBox, EditableText, GlobalKey, IconData, SizedBox, Text,
        TextEditingController, TextEditingControllerLeaf, WidgetsBinding,
    };

    use crate::app::CupertinoApp;
    use crate::page_scaffold::CupertinoPageScaffold;
    use crate::test_support::{build, pump};
    use crate::theme::CupertinoThemeData;

    /// Flutter's test surface: 800 by 600 logical pixels.
    fn flutter_test_cell() -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::IOS)
            .with_view(Rc::new(TestView::with_pixel_ratio(1600.0, 1200.0, 2.0)));
        let cell = AppCell::with_platform(Rc::new(platform));
        install_fonts(&mut cell.borrow_mut());
        // The binding's root, which each `pump_app` replaces, as flutter_test's starts with one.
        build(&cell, SizedBox::new().into_widget());
        cell
    }

    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    /// Dart's `tester.pumpWidget`: makes `app_widget` the view's child and runs one frame.
    ///
    /// One frame only, as Dart's: a rebuild a state schedules after it, such as the field's when
    /// its editable first reports its scroll metrics, waits for the test's next pump.
    fn pump_app(cell: &AppCell, app_widget: CupertinoApp) {
        let mut app = cell.borrow_mut();
        let view = app.platform().views().into_iter().next().expect("one view");
        let root = waft_widgets::View::new(view, app_widget.into_widget()).into_widget();
        WidgetsBinding::instance(&mut app).attach_root_widget(&mut app, root);
        pump(&mut app, Duration::ZERO);
        drop(app);
        cell.checkpoint();
    }

    fn centered(field: CupertinoSearchTextField) -> CupertinoApp {
        CupertinoApp::new().home(Center::new().child(field))
    }

    fn root(app: &mut App) -> AnyElement {
        WidgetsBinding::instance(app)
            .root_element(app)
            .expect("the tree is attached")
    }

    /// Every element under `element`, itself first, whose widget is a `W`.
    fn elements_of<W: 'static>(app: &App, element: AnyElement) -> Vec<AnyElement> {
        let mut found = Vec::new();
        if waft_widgets::downcast_widget::<W>(&**element.widget(app)).is_some() {
            found.push(element);
        }
        element.visit_children(app, &mut |child| found.extend(elements_of::<W>(app, child)));
        found
    }

    fn widget_of<W: 'static>(app: &App, element: AnyElement) -> &W {
        waft_widgets::downcast_widget::<W>(&**element.widget(app)).expect("a widget of the type")
    }

    /// The one `W` in the tree.
    fn only<W: 'static>(app: &mut App) -> AnyElement {
        let root = root(app);
        let found = elements_of::<W>(app, root);
        assert_eq!(found.len(), 1, "one {}", std::any::type_name::<W>());
        found[0]
    }

    /// The icons showing `icon`.
    fn icons(app: &mut App, icon: IconData) -> Vec<AnyElement> {
        let root = root(app);
        elements_of::<Icon>(app, root)
            .into_iter()
            .filter(|element| widget_of::<Icon>(app, *element).icon.as_ref() == Some(&icon))
            .collect()
    }

    fn icon(app: &mut App, icon: IconData) -> AnyElement {
        let found = icons(app, icon);
        assert_eq!(found.len(), 1, "one icon");
        found[0]
    }

    /// The texts reading `text`, as `Text` or as an `EditableText`'s value.
    fn texts(app: &mut App, text: &str) -> usize {
        let root = root(app);
        let plain = elements_of::<Text>(app, root)
            .into_iter()
            .filter(|element| widget_of::<Text>(app, *element).data.as_deref() == Some(text))
            .count();
        let edited = elements_of::<EditableText>(app, root)
            .into_iter()
            .filter(|element| {
                widget_of::<EditableText>(app, *element)
                    .controller
                    .text_value(app)
                    == text
            })
            .count();
        plain + edited
    }

    /// `element`'s box in global coordinates: its top left and its size.
    fn rect_of(app: &App, element: AnyElement) -> (Offset, Size) {
        let render_box = element
            .find_render_object(app)
            .expect("a render object")
            .as_box()
            .expect("a box");
        (
            render_box.local_to_global(app, Offset::ZERO, None),
            render_box.size(app),
        )
    }

    fn top_left(app: &App, element: AnyElement) -> Offset {
        rect_of(app, element).0
    }

    fn top_right(app: &App, element: AnyElement) -> Offset {
        let (origin, size) = rect_of(app, element);
        Offset::new(origin.dx() + size.width(), origin.dy())
    }

    fn tap(app: &mut App, element: AnyElement) {
        let (origin, size) = rect_of(app, element);
        let at = Offset::new(
            origin.dx() + size.width() / 2.0,
            origin.dy() + size.height() / 2.0,
        );
        for change in [PointerChange::Down, PointerChange::Up] {
            GestureBinding::instance(app).handle_pointer_data_packet(
                app,
                PointerDataPacket::new(vec![PointerData {
                    change,
                    kind: PointerDeviceKind::Touch,
                    pointer_identifier: 1,
                    physical_x: at.dx() * 2.0,
                    physical_y: at.dy() * 2.0,
                    ..PointerData::default()
                }]),
            );
            app.drain_microtasks();
        }
        pump(app, Duration::ZERO);
    }

    fn decoration_of_field(app: &mut App) -> BoxDecoration {
        let field = only::<CupertinoSearchTextField>(app);
        let decorated = elements_of::<DecoratedBox>(app, field)[0];
        widget_of::<DecoratedBox>(app, decorated)
            .decoration
            .as_any()
            .downcast_ref::<BoxDecoration>()
            .expect("a box decoration")
            .clone()
    }

    fn controller_with(app: &mut App, text: &str) -> Ref<TextEditingController> {
        let controller = TextEditingController::new(app);
        controller.set_text(app, text);
        controller
    }

    fn inner_text_field(app: &mut App) -> &CupertinoTextField {
        let element = only::<CupertinoTextField>(app);
        widget_of::<CupertinoTextField>(app, element)
    }

    #[test]
    fn default_search_field_has_a_border_radius() {
        let cell = flutter_test_cell();
        pump_app(&cell, centered(CupertinoSearchTextField::new()));
        let mut app = cell.borrow_mut();
        assert_eq!(
            decoration_of_field(&mut app).border_radius,
            Some(BorderRadius::all(Radius::circular(9.0)).into())
        );
    }

    #[test]
    fn decoration_overrides_default_background_color() {
        let cell = flutter_test_cell();
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .decoration(BoxDecoration::new().color(Color::from_argb(1, 1, 1, 1))),
            ),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(
            decoration_of_field(&mut app)
                .color
                .map(|color| color.color()),
            Some(Color::from_argb(1, 1, 1, 1))
        );
    }

    #[test]
    fn decoration_overrides_default_border_radius() {
        let cell = flutter_test_cell();
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .decoration(BoxDecoration::new().border_radius(BorderRadius::ZERO)),
            ),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(
            decoration_of_field(&mut app).border_radius,
            Some(BorderRadius::ZERO.into())
        );
    }

    #[test]
    fn can_change_keyboard_type() {
        let cell = flutter_test_cell();
        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new().keyboard_type(Some(TextInputType::NUMBER))),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(
            inner_text_field(&mut app).keyboard_type,
            Some(TextInputType::NUMBER)
        );
    }

    #[test]
    fn can_control_text_content_via_controller() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new().controller(controller.clone().into_erased())),
        );
        let mut app = cell.borrow_mut();
        controller.set_text(&mut app, "controller text");
        pump(&mut app, Duration::ZERO);
        assert_eq!(texts(&mut app, "controller text"), 1);
        controller.set_text(&mut app, "");
        pump(&mut app, Duration::ZERO);
        assert_eq!(texts(&mut app, "controller text"), 0);
    }

    fn placeholder_color_of(app: &mut App) -> Option<Color> {
        let root = root(app);
        let placeholder = elements_of::<Text>(app, root)
            .into_iter()
            .find(|element| widget_of::<Text>(app, *element).data.as_deref() == Some("Search"))
            .expect("the placeholder");
        widget_of::<Text>(app, placeholder)
            .style
            .as_ref()
            .and_then(|style| style.color.as_ref())
            .map(AnyColor::color)
    }

    #[test]
    #[ignore = "an icon whose IconTheme gives it a size and a new colour trips TextPainter's \
                paint-time size check (text_painter.rs), which this test's change of brightness does"]
    fn placeholder_color() {
        let cell = flutter_test_cell();
        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new())
                .theme(CupertinoThemeData::new().with_brightness(Brightness::Dark)),
        );
        // `CupertinoColors.secondaryLabel.darkColor`.
        assert_eq!(
            placeholder_color_of(&mut cell.borrow_mut()),
            Some(Color::from_argb(153, 235, 235, 245))
        );

        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new())
                .theme(CupertinoThemeData::new().with_brightness(Brightness::Light)),
        );
        // `CupertinoColors.secondaryLabel.color`.
        assert_eq!(
            placeholder_color_of(&mut cell.borrow_mut()),
            Some(Color::from_argb(153, 60, 60, 67))
        );
    }

    #[test]
    fn placeholder_style_modifies_the_placeholder_and_not_the_text() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .controller(controller.clone().into_erased())
                    .placeholder("placeholder")
                    .style(
                        TextStyle::new()
                            .color(Color::new(0x00FFFFFF))
                            .font_weight(FontWeight::W300),
                    )
                    .placeholder_style(
                        TextStyle::new()
                            .color(Color::new(0xAAFFFFFF))
                            .font_weight(FontWeight::W600),
                    ),
            ),
        );
        let mut app = cell.borrow_mut();
        let root = root(&mut app);
        let placeholder = elements_of::<Text>(&app, root)
            .into_iter()
            .find(|element| {
                widget_of::<Text>(&app, *element).data.as_deref() == Some("placeholder")
            })
            .expect("the placeholder");
        let style = widget_of::<Text>(&app, placeholder)
            .style
            .clone()
            .expect("a placeholder style");
        assert_eq!(style.color.map(|c| c.color()), Some(Color::new(0xAAFFFFFF)));
        assert_eq!(style.font_weight, Some(FontWeight::W600));

        controller.set_text(&mut app, "input");
        pump(&mut app, Duration::ZERO);
        let editable = only::<EditableText>(&mut app);
        let style = widget_of::<EditableText>(&app, editable).style.clone();
        assert_eq!(style.color.map(|c| c.color()), Some(Color::new(0x00FFFFFF)));
        assert_eq!(style.font_weight, Some(FontWeight::W300));
    }

    #[test]
    fn prefix_widget_is_in_front_of_the_text() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "input");
        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new().controller(controller.into_erased())),
        );
        let mut app = cell.borrow_mut();
        let search = icon(&mut app, CupertinoIcons::search());
        let editable = only::<EditableText>(&mut app);
        let field = only::<CupertinoSearchTextField>(&mut app);
        assert_eq!(
            top_right(&app, search).dx() + 5.5,
            top_left(&app, editable).dx()
        );
        assert_eq!(
            top_left(&app, editable).dx(),
            top_left(&app, field).dx() + rect_of(&app, search).1.width() + 11.5
        );
    }

    #[test]
    fn suffix_widget_is_after_the_text() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "Hi");
        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new().controller(controller.into_erased())),
        );
        let mut app = cell.borrow_mut();
        let xmark = icon(&mut app, CupertinoIcons::xmark_circle_fill());
        let editable = only::<EditableText>(&mut app);
        let field = only::<CupertinoSearchTextField>(&mut app);
        assert_eq!(
            top_right(&app, editable).dx() + 5.5,
            top_left(&app, xmark).dx()
        );
        assert_eq!(
            top_right(&app, editable).dx(),
            top_right(&app, field).dx() - rect_of(&app, xmark).1.width() - 10.5
        );
    }

    #[test]
    fn prefix_widget_visibility() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        let prefix_icon = GlobalKey::new();
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .controller(controller.clone().into_erased())
                    .prefix_icon(
                        SizedBox::new()
                            .key(prefix_icon.clone().into())
                            .width(50.0)
                            .height(50.0),
                    ),
            ),
        );
        let mut app = cell.borrow_mut();
        assert!(icons(&mut app, CupertinoIcons::search()).is_empty());
        assert!(prefix_icon.current_context(&app).is_some());

        controller.set_text(&mut app, "text input");
        pump(&mut app, Duration::ZERO);
        assert_eq!(texts(&mut app, "text input"), 1);
        assert!(icons(&mut app, CupertinoIcons::search()).is_empty());
        assert!(prefix_icon.current_context(&app).is_some());
    }

    #[test]
    fn suffix_widget_respects_visibility_mode() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .controller(controller.clone().into_erased())
                    .suffix_mode(OverlayVisibilityMode::NotEditing),
            ),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(
            icons(&mut app, CupertinoIcons::xmark_circle_fill()).len(),
            1
        );
        controller.set_text(&mut app, "text input");
        pump(&mut app, Duration::ZERO);
        assert_eq!(texts(&mut app, "text input"), 1);
        assert!(icons(&mut app, CupertinoIcons::xmark_circle_fill()).is_empty());
    }

    #[test]
    fn default_prefix_and_suffix_insets_are_aligned() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        pump_app(
            &cell,
            CupertinoApp::new().home(Center::new().child(
                CupertinoSearchTextField::new().controller(controller.clone().into_erased()),
            )),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(icons(&mut app, CupertinoIcons::search()).len(), 1);
        assert!(icons(&mut app, CupertinoIcons::xmark_circle_fill()).is_empty());

        controller.set_text(&mut app, "text input");
        pump(&mut app, Duration::ZERO);
        let search = icon(&mut app, CupertinoIcons::search());
        let xmark = icon(&mut app, CupertinoIcons::xmark_circle_fill());
        let (search_origin, search_size) = rect_of(&app, search);
        let (xmark_origin, xmark_size) = rect_of(&app, xmark);
        // Flutter's 290 and 310 are its test font's; the field's height here is the system font's,
        // so the two icons are checked on one line rather than at Flutter's.
        assert_eq!(search_origin.dx(), 6.0);
        assert_eq!(xmark_origin.dx(), 775.0);
        assert_eq!(search_origin.dy(), xmark_origin.dy());
        assert_eq!(search_size, Size::new(20.0, 20.0));
        assert_eq!(xmark_size, Size::new(20.0, 20.0));
    }

    #[test]
    fn clear_button_shows_with_right_visibility_mode() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .controller(controller.clone().into_erased())
                    .placeholder("placeholder does not affect clear button"),
            ),
        );
        let mut app = cell.borrow_mut();
        assert!(icons(&mut app, CupertinoIcons::xmark_circle_fill()).is_empty());
        controller.set_text(&mut app, "text input");
        pump(&mut app, Duration::ZERO);
        assert_eq!(
            icons(&mut app, CupertinoIcons::xmark_circle_fill()).len(),
            1
        );
        assert_eq!(texts(&mut app, "text input"), 1);
        drop(app);

        let controller = controller_with(&mut cell.borrow_mut(), "");
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .controller(controller.clone().into_erased())
                    .placeholder("placeholder does not affect clear button")
                    .suffix_mode(OverlayVisibilityMode::NotEditing),
            ),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(
            icons(&mut app, CupertinoIcons::xmark_circle_fill()).len(),
            1
        );
        controller.set_text(&mut app, "input");
        pump(&mut app, Duration::ZERO);
        assert!(icons(&mut app, CupertinoIcons::xmark_circle_fill()).is_empty());
    }

    #[test]
    fn clear_button_removes_text() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new().controller(controller.clone().into_erased())),
        );
        let mut app = cell.borrow_mut();
        controller.set_text(&mut app, "text entry");
        pump(&mut app, Duration::ZERO);
        let xmark = icon(&mut app, CupertinoIcons::xmark_circle_fill());
        tap(&mut app, xmark);
        assert_eq!(controller.handle().text_value(&app), "");
        assert_eq!(texts(&mut app, "Search"), 1);
        assert_eq!(texts(&mut app, "text entry"), 0);
        assert!(icons(&mut app, CupertinoIcons::xmark_circle_fill()).is_empty());
    }

    #[test]
    fn tapping_clear_button_also_calls_on_changed_when_text_not_empty() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "");
        let value = Rc::new(RefCell::new(String::from("text entry")));
        let seen = Rc::clone(&value);
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .controller(controller.clone().into_erased())
                    .placeholder("placeholder")
                    .on_changed(move |_app, new_value| *seen.borrow_mut() = new_value),
            ),
        );
        let mut app = cell.borrow_mut();
        let text = value.borrow().clone();
        controller.set_text(&mut app, text);
        pump(&mut app, Duration::ZERO);
        let xmark = icon(&mut app, CupertinoIcons::xmark_circle_fill());
        tap(&mut app, xmark);
        assert!(controller.handle().text_value(&app).is_empty());
        assert_eq!(texts(&mut app, "text entry"), 0);
        assert!(value.borrow().is_empty());
    }

    #[test]
    fn rtl_puts_attachments_to_the_right_places() {
        let cell = flutter_test_cell();
        pump_app(
            &cell,
            CupertinoApp::new().home(Directionality::new(
                TextDirection::Rtl,
                Center::new().child(
                    CupertinoSearchTextField::new().suffix_mode(OverlayVisibilityMode::Always),
                ),
            )),
        );
        let mut app = cell.borrow_mut();
        let search = icon(&mut app, CupertinoIcons::search());
        let xmark = icon(&mut app, CupertinoIcons::xmark_circle_fill());
        assert_eq!(top_left(&app, search).dx(), 800.0 - 26.0);
        assert_eq!(top_right(&app, xmark).dx(), 25.0);
    }

    #[test]
    fn can_modify_prefix_and_suffix_insets() {
        let cell = flutter_test_cell();
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .suffix_mode(OverlayVisibilityMode::Always)
                    .prefix_insets(EdgeInsetsGeometry::ZERO)
                    .suffix_insets(EdgeInsetsGeometry::ZERO),
            ),
        );
        let mut app = cell.borrow_mut();
        let search = icon(&mut app, CupertinoIcons::search());
        let xmark = icon(&mut app, CupertinoIcons::xmark_circle_fill());
        assert_eq!(top_left(&app, search).dx(), 0.0);
        assert_eq!(top_right(&app, xmark).dx(), 800.0);
    }

    #[test]
    fn custom_suffix_on_tap_overrides_default_clearing_behavior() {
        let cell = flutter_test_cell();
        let controller = controller_with(&mut cell.borrow_mut(), "Text");
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .controller(controller.clone().into_erased())
                    .on_suffix_tap(Rc::new(|_app| {})),
            ),
        );
        let mut app = cell.borrow_mut();
        pump(&mut app, Duration::ZERO);
        let xmark = icon(&mut app, CupertinoIcons::xmark_circle_fill());
        tap(&mut app, xmark);
        assert!(!controller.handle().text_value(&app).is_empty());
        assert_eq!(texts(&mut app, "Text"), 1);
    }

    #[test]
    fn on_tap_is_properly_forwarded_to_the_inner_text_field() {
        let cell = flutter_test_cell();
        // on_tap can be none.
        pump_app(&cell, centered(CupertinoSearchTextField::new()));
        let taps = Rc::new(Cell::new(0));
        let counted = Rc::clone(&taps);
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .on_tap(Listener::new(move |_app| counted.set(counted.get() + 1))),
            ),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(taps.get(), 0);
        let field = only::<CupertinoTextField>(&mut app);
        tap(&mut app, field);
        assert_eq!(taps.get(), 1);
    }

    #[test]
    fn its_settings_are_forwarded_to_the_inner_text_field() {
        let cell = flutter_test_cell();
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .autocorrect(false)
                    .enabled(false)
                    .smart_quotes_type(SmartQuotesType::Disabled)
                    .smart_dashes_type(SmartDashesType::Disabled)
                    .enable_ime_personalized_learning(false)
                    .cursor_width(1.0)
                    .cursor_height(10.0)
                    .cursor_radius(Radius::circular(1.0))
                    .cursor_opacity_animates(false)
                    .cursor_color(Color::from_argb(255, 255, 0, 0)),
            ),
        );
        let mut app = cell.borrow_mut();
        let field = inner_text_field(&mut app);
        assert_eq!(field.autocorrect, Some(false));
        assert!(!field.enabled);
        assert_eq!(field.text_input_action, Some(TextInputAction::Search));
        assert_eq!(field.smart_quotes_type, Some(SmartQuotesType::Disabled));
        assert_eq!(field.smart_dashes_type, Some(SmartDashesType::Disabled));
        assert!(!field.enable_ime_personalized_learning);
        assert_eq!(field.cursor_width, 1.0);
        assert_eq!(field.cursor_height, Some(10.0));
        assert_eq!(field.cursor_radius, Radius::circular(1.0));
        assert!(!field.cursor_opacity_animates);
        assert_eq!(
            field.cursor_color.as_ref().map(AnyColor::color),
            Some(Color::from_argb(255, 255, 0, 0))
        );
    }

    #[test]
    fn autofocus_true_gives_focus_to_the_widget() {
        let cell = flutter_test_cell();
        let focus_node = FocusNode::new(&mut cell.borrow_mut());
        pump_app(
            &cell,
            centered(
                CupertinoSearchTextField::new()
                    .focus_node(focus_node.clone().into_erased())
                    .autofocus(true),
            ),
        );
        let mut app = cell.borrow_mut();
        pump(&mut app, Duration::ZERO);
        assert!(focus_node.has_focus(&mut app));
    }

    #[test]
    fn focused_search_field_hides_prefix_in_higher_accessibility_text_scale_modes() {
        let icon_size = 10.0;
        let field = |focus_node: &Ref<FocusNode>, scale_factor: f64| {
            CupertinoApp::new().home(MediaQuery::with_clamped_text_scaling(
                None,
                scale_factor,
                scale_factor,
                CupertinoPageScaffold::new(
                    Center::new().child(
                        CupertinoSearchTextField::new()
                            .placeholder("Search")
                            .focus_node(focus_node.clone().into_erased())
                            .prefix_icon(Icon::new(Some(CupertinoIcons::add())))
                            .suffix_icon(Icon::new(Some(CupertinoIcons::xmark())))
                            .suffix_mode(OverlayVisibilityMode::Always)
                            .item_size(icon_size),
                    ),
                ),
            ))
        };
        let cell = flutter_test_cell();
        let focus_node = FocusNode::new(&mut cell.borrow_mut());
        pump_app(&cell, field(&focus_node, 3.0));
        {
            let mut app = cell.borrow_mut();
            for each in [CupertinoIcons::add(), CupertinoIcons::xmark()] {
                let found = icon(&mut app, each);
                assert_eq!(rect_of(&app, found).1, Size::square(3.0 * icon_size));
            }
            focus_node.request_focus(&mut app, None);
            for frame in 0..60 {
                pump(&mut app, Duration::from_millis(16 * frame));
            }
            // The prefix icon shrinks at higher accessibility text scale modes.
            let add = icon(&mut app, CupertinoIcons::add());
            assert_eq!(rect_of(&app, add).1, Size::ZERO);
            let xmark = icon(&mut app, CupertinoIcons::xmark());
            assert_eq!(rect_of(&app, xmark).1, Size::square(3.0 * icon_size));
        }
        // Below the threshold, the prefix icon is displayed.
        pump_app(&cell, field(&focus_node, 2.9));
        let mut app = cell.borrow_mut();
        for each in [CupertinoIcons::add(), CupertinoIcons::xmark()] {
            let found = icon(&mut app, each);
            assert_eq!(rect_of(&app, found).1, Size::square(2.9 * icon_size));
        }
    }

    #[test]
    fn does_not_crash_at_zero_area() {
        let platform = TestPlatform::new()
            .on(TargetPlatform::IOS)
            .with_view(Rc::new(TestView::with_pixel_ratio(0.0, 0.0, 2.0)));
        let cell = AppCell::with_platform(Rc::new(platform));
        install_fonts(&mut cell.borrow_mut());
        build(&cell, SizedBox::new().into_widget());
        let controller = controller_with(&mut cell.borrow_mut(), "X");
        pump_app(
            &cell,
            centered(CupertinoSearchTextField::new().controller(controller.clone().into_erased())),
        );
        let mut app = cell.borrow_mut();
        let field = only::<CupertinoSearchTextField>(&mut app);
        assert_eq!(rect_of(&app, field).1, Size::ZERO);
        let erased: TextEditingControllerRef = controller.clone().into_erased();
        erased.set_selection(
            &mut app,
            TextSelection::collapsed(0, TextAffinity::Downstream),
        );
        pump(&mut app, Duration::ZERO);
    }

    /// The fade the scroll-resizing tests watch, between the icons' height with their insets and
    /// four fifths of it.
    #[test]
    fn contents_fade_out_between_the_full_height_and_four_fifths_of_it() {
        let opacity = CupertinoSearchTextFieldState::calculate_scroll_opacity;
        assert_eq!(opacity(36.0, 36.0), 0.0);
        assert_eq!(opacity(40.0, 36.0), 0.0);
        assert_eq!(opacity(36.0 * 4.0 / 5.0, 36.0), 1.0);
        assert_eq!(opacity(10.0, 36.0), 1.0);
        let halfway = 36.0 * 9.0 / 10.0;
        assert!((opacity(halfway, 36.0) - 0.5).abs() < 1e-9);
    }
}
