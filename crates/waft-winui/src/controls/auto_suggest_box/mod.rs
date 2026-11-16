//! AutoSuggestBox_Partial.cpp, `DefaultAutoSuggestBoxStyle` and AutoSuggestBoxHelper.cpp: a text
//! box that lists suggestions the application gives it as the user types, below or above the
//! box, and reports the query the user submits.
//!
//! The template is `LayoutRoot` holding `TextBox` in `AutoSuggestBoxTextBoxStyle` (the text box
//! with a `QueryButton` beside its `DeleteButton`) and `SuggestionsPopup`, which opens from the
//! text box's `BorderElement` into the root overlay with `SuggestionsContainer`, an acrylic border
//! around `SuggestionsList`. `KeepInteriorCornersSquare` squares the corners where the box and the
//! open list meet.
//!
//! The list keeps the selection in its own order, which is the items' reverse order while it
//! opens above the box (`IsSuggestionListVectorReversed`): an index of the list is not an index
//! of `items_source`, and `item_at` is the one place that turns one into the other.

mod placement;
mod suggestion_item;

use super::flyout::shadow::PopupShadow;
use super::text_control::TextControl;
use crate::*;
use placement::{SuggestionListLayout, SuggestionListPosition, SuggestionsPosition};
use std::{fmt, rc::Rc, time::Duration};
use suggestion_item::SuggestionItem;
use waft_embedder::{Color, Offset, Rect, TextAffinity, TextSelection};
use waft_foundation::{App, Handle, Listenable, Listener, OptionRetained, Ref, Timer};
use waft_painting::{EdgeInsetsGeometry, transform_rect};
use waft_rendering::BoxConstraints;
use waft_scheduler::FrameCallback;
use waft_services::{KeyEvent, LogicalKeyboardKey, TextEditingValue};
use waft_widgets::*;

/// `m_tpTextChangedEventTimer`'s interval: TextChanged waits this long after the last change.
const TEXT_CHANGED_EVENT_DELAY: Duration = Duration::from_millis(150);

/// XAML `AutoSuggestionBoxTextChangeReason`: what changed the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutoSuggestionBoxTextChangeReason {
    /// The user edited the text.
    UserInput,
    /// The application, or the box restoring what the user typed, set the text.
    ProgrammaticChange,
    /// The box showed the text of the suggestion the user moved to.
    SuggestionChosen,
}

/// The count of the box's text changes, which `CheckCurrent` compares an event's with.
struct TextChangedCounter {
    value: u64,
}

/// XAML `AutoSuggestBoxTextChangedEventArgs`.
#[derive(Clone, Copy, Debug)]
pub struct AutoSuggestBoxTextChangedEventArgs {
    reason: AutoSuggestionBoxTextChangeReason,
    counter: u64,
    owner: Handle<TextChangedCounter>,
}

impl AutoSuggestBoxTextChangedEventArgs {
    /// `Reason`.
    pub fn reason(&self) -> AutoSuggestionBoxTextChangeReason {
        self.reason
    }

    /// `CheckCurrent`: whether the text has not changed again since this event, so that
    /// suggestions computed for it are still wanted.
    pub fn check_current(&self, app: &App) -> bool {
        app.contains(self.owner) && app.get(self.owner).value == self.counter
    }
}

impl fmt::Debug for TextChangedCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextChangedCounter")
            .field("value", &self.value)
            .finish()
    }
}

/// An item of `items_source`, held so that event arguments can lend it while the application
/// changes the box.
struct ChosenItem<T> {
    items: Rc<[T]>,
    index: usize,
}

impl<T> Clone for ChosenItem<T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            index: self.index,
        }
    }
}

impl<T> ChosenItem<T> {
    fn get(&self) -> &T {
        &self.items[self.index]
    }
}

/// XAML `AutoSuggestBoxSuggestionChosenEventArgs`.
pub struct AutoSuggestBoxSuggestionChosenEventArgs<T> {
    selected: ChosenItem<T>,
}

impl<T> AutoSuggestBoxSuggestionChosenEventArgs<T> {
    /// `SelectedItem`: the suggestion the user moved to.
    pub fn selected_item(&self) -> &T {
        self.selected.get()
    }
}

/// XAML `AutoSuggestBoxQuerySubmittedEventArgs`.
pub struct AutoSuggestBoxQuerySubmittedEventArgs<T> {
    query_text: String,
    chosen: Option<ChosenItem<T>>,
}

impl<T> AutoSuggestBoxQuerySubmittedEventArgs<T> {
    /// `QueryText`: the text in the box.
    pub fn query_text(&self) -> &str {
        &self.query_text
    }

    /// `ChosenSuggestion`: the suggestion the user submitted with Enter or a click, or none for
    /// Enter on typed text and for the query button.
    pub fn chosen_suggestion(&self) -> Option<&T> {
        self.chosen.as_ref().map(ChosenItem::get)
    }
}

/// Reads the text of an item: `TextMemberPath`'s property.
pub type AutoSuggestBoxTextMember<T> = Rc<dyn Fn(&T) -> String>;

/// Builds an item's content: `ItemTemplate`.
pub type AutoSuggestBoxItemTemplate<T> = Rc<dyn Fn(&mut App, BuildContext, &T) -> WidgetRef>;

/// Asked to open or close the list, for an application that owns `is_suggestion_list_open`.
pub type AutoSuggestBoxOpenChangedHandler = Rc<dyn Fn(&mut App, bool)>;

/// Handles `TextChanged`.
pub type AutoSuggestBoxTextChangedHandler =
    Rc<dyn Fn(&mut App, &AutoSuggestBoxTextChangedEventArgs)>;

/// Handles `SuggestionChosen`.
pub type AutoSuggestBoxSuggestionChosenHandler<T> =
    Rc<dyn Fn(&mut App, &AutoSuggestBoxSuggestionChosenEventArgs<T>)>;

/// Handles `QuerySubmitted`.
pub type AutoSuggestBoxQuerySubmittedHandler<T> =
    Rc<dyn Fn(&mut App, &AutoSuggestBoxQuerySubmittedEventArgs<T>)>;

/// XAML `AutoSuggestBox`: a text box whose text lives in the application's controller, with the
/// suggestions the application sets in `items_source` as it hears `text_changed`.
pub struct AutoSuggestBox<T: fmt::Display + 'static> {
    /// `TextBox`'s text: `Text`.
    pub controller: TextEditingControllerRef,
    /// `ItemsSource`: the suggestions.
    pub items_source: Rc<[T]>,
    /// `TextMemberPath`: the text a chosen suggestion puts in the box; the item's `Display`
    /// when unset.
    pub text_member_path: Option<AutoSuggestBoxTextMember<T>>,
    /// `ItemTemplate`: what the list shows for an item; its `Display` when unset.
    pub item_template: Option<AutoSuggestBoxItemTemplate<T>>,
    /// `PlaceholderText`.
    pub placeholder_text: String,
    /// `Header`.
    pub header: Option<WidgetRef>,
    /// `Description`.
    pub description: Option<WidgetRef>,
    /// `QueryIcon`: the query button's content; no query button when unset. WinUI's
    /// `SymbolIcon` takes `AutoSuggestBoxIconFontSize`; the kit's icons bring their own size.
    pub query_icon: Option<WidgetRef>,
    /// `MaxSuggestionListHeight`: unbounded by default, and all the room there is when negative.
    pub max_suggestion_list_height: f64,
    /// `IsSuggestionListOpen`, when the application owns it, and where the box asks to change it.
    pub is_suggestion_list_open: Option<bool>,
    pub is_suggestion_list_open_changed: Option<AutoSuggestBoxOpenChangedHandler>,
    /// `UpdateTextOnSelect`: whether moving to a suggestion puts its text in the box.
    pub update_text_on_select: bool,
    /// `AutoMaximizeSuggestionArea`: read only while an on-screen keyboard covers the window,
    /// which no platform of the kit reports.
    pub auto_maximize_suggestion_area: bool,
    /// `TextChanged`, 150 ms after the last change of the text.
    pub text_changed: Option<AutoSuggestBoxTextChangedHandler>,
    /// `SuggestionChosen`.
    pub suggestion_chosen: Option<AutoSuggestBoxSuggestionChosenHandler<T>>,
    /// `QuerySubmitted`.
    pub query_submitted: Option<AutoSuggestBoxQuerySubmittedHandler<T>>,
    /// Identity retained across rebuilds.
    pub key: Option<KeyRef>,
}

impl<T: fmt::Display + 'static> AutoSuggestBox<T> {
    /// Creates a box editing `controller`'s text, with no suggestions.
    pub fn new(controller: TextEditingControllerRef) -> Self {
        Self {
            controller,
            items_source: Rc::from(Vec::new()),
            text_member_path: None,
            item_template: None,
            placeholder_text: String::new(),
            header: None,
            description: None,
            query_icon: None,
            max_suggestion_list_height: f64::INFINITY,
            is_suggestion_list_open: None,
            is_suggestion_list_open_changed: None,
            update_text_on_select: true,
            auto_maximize_suggestion_area: true,
            text_changed: None,
            suggestion_chosen: None,
            query_submitted: None,
            key: None,
        }
    }

    /// Sets ItemsSource.
    pub fn items_source(mut self, items: Vec<T>) -> Self {
        self.items_source = Rc::from(items);
        self
    }

    /// Sets TextMemberPath.
    pub fn text_member_path(mut self, text: impl Fn(&T) -> String + 'static) -> Self {
        self.text_member_path = Some(Rc::new(text));
        self
    }

    /// Sets ItemTemplate.
    pub fn item_template(
        mut self,
        template: impl Fn(&mut App, BuildContext, &T) -> WidgetRef + 'static,
    ) -> Self {
        self.item_template = Some(Rc::new(template));
        self
    }

    /// Sets PlaceholderText.
    pub fn placeholder_text(mut self, value: impl Into<String>) -> Self {
        self.placeholder_text = value.into();
        self
    }

    /// Sets Header.
    pub fn header<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.header = Some(value.into_widget());
        self
    }

    /// Sets Description.
    pub fn description<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.description = Some(value.into_widget());
        self
    }

    /// Sets QueryIcon.
    pub fn query_icon<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.query_icon = Some(value.into_widget());
        self
    }

    /// Sets MaxSuggestionListHeight.
    pub fn max_suggestion_list_height(mut self, value: f64) -> Self {
        self.max_suggestion_list_height = value;
        self
    }

    /// Gives the application IsSuggestionListOpen: the box shows `open` and asks for changes
    /// through `changed`. Opening it moves the focus to the box, as the property's setter does.
    pub fn is_suggestion_list_open(
        mut self,
        open: bool,
        changed: impl Fn(&mut App, bool) + 'static,
    ) -> Self {
        self.is_suggestion_list_open = Some(open);
        self.is_suggestion_list_open_changed = Some(Rc::new(changed));
        self
    }

    /// Sets UpdateTextOnSelect.
    pub fn update_text_on_select(mut self, value: bool) -> Self {
        self.update_text_on_select = value;
        self
    }

    /// Sets AutoMaximizeSuggestionArea.
    pub fn auto_maximize_suggestion_area(mut self, value: bool) -> Self {
        self.auto_maximize_suggestion_area = value;
        self
    }

    /// Sets TextChanged.
    pub fn text_changed(
        mut self,
        handler: impl Fn(&mut App, &AutoSuggestBoxTextChangedEventArgs) + 'static,
    ) -> Self {
        self.text_changed = Some(Rc::new(handler));
        self
    }

    /// Sets SuggestionChosen.
    pub fn suggestion_chosen(
        mut self,
        handler: impl Fn(&mut App, &AutoSuggestBoxSuggestionChosenEventArgs<T>) + 'static,
    ) -> Self {
        self.suggestion_chosen = Some(Rc::new(handler));
        self
    }

    /// Sets QuerySubmitted.
    pub fn query_submitted(
        mut self,
        handler: impl Fn(&mut App, &AutoSuggestBoxQuerySubmittedEventArgs<T>) + 'static,
    ) -> Self {
        self.query_submitted = Some(Rc::new(handler));
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// `TryGetSuggestionValue`: the text the box takes for `item`.
    fn text_of(&self, item: &T) -> String {
        match &self.text_member_path {
            Some(text) => text(item),
            None => item.to_string(),
        }
    }
}

impl<T: fmt::Display + 'static> Clone for AutoSuggestBox<T> {
    fn clone(&self) -> Self {
        Self {
            controller: self.controller.clone(),
            items_source: self.items_source.clone(),
            text_member_path: self.text_member_path.clone(),
            item_template: self.item_template.clone(),
            placeholder_text: self.placeholder_text.clone(),
            header: self.header.clone(),
            description: self.description.clone(),
            query_icon: self.query_icon.clone(),
            max_suggestion_list_height: self.max_suggestion_list_height,
            is_suggestion_list_open: self.is_suggestion_list_open,
            is_suggestion_list_open_changed: self.is_suggestion_list_open_changed.clone(),
            update_text_on_select: self.update_text_on_select,
            auto_maximize_suggestion_area: self.auto_maximize_suggestion_area,
            text_changed: self.text_changed.clone(),
            suggestion_chosen: self.suggestion_chosen.clone(),
            query_submitted: self.query_submitted.clone(),
            key: self.key.clone(),
        }
    }
}

impl<T: fmt::Display + 'static> fmt::Debug for AutoSuggestBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutoSuggestBox")
            .field("items", &self.items_source.len())
            .field("placeholder_text", &self.placeholder_text)
            .field(
                "max_suggestion_list_height",
                &self.max_suggestion_list_height,
            )
            .field("is_suggestion_list_open", &self.is_suggestion_list_open)
            .field("update_text_on_select", &self.update_text_on_select)
            .field(
                "auto_maximize_suggestion_area",
                &self.auto_maximize_suggestion_area,
            )
            .finish_non_exhaustive()
    }
}

/// `AutoSuggestBoxTextBoxStyle`: the TextBox template of the box's `TextBox`, with the corner
/// radius `AutoSuggestBoxHelper` gives it, its `QueryButton`, and the popup opening from its
/// `BorderElement`.
#[derive(Clone)]
pub(super) struct AutoSuggestBoxTextBoxStyle {
    /// `CornerRadius`: the box's, with the corners next to the open list square.
    pub(super) corner_radius: [f64; 4],
    /// `QueryButton`, when the box has a `QueryIcon`.
    pub(super) query_button: Option<QueryButton>,
    /// Puts `SuggestionsPopup` on `BorderElement`, which it opens from.
    pub(super) border_element: Rc<dyn Fn(WidgetRef) -> WidgetRef>,
}

/// `QueryButton`'s content and what a click on it does.
#[derive(Clone)]
pub(super) struct QueryButton {
    pub(super) icon: WidgetRef,
    pub(super) click: Listener,
}

impl fmt::Debug for AutoSuggestBoxTextBoxStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutoSuggestBoxTextBoxStyle")
            .field("corner_radius", &self.corner_radius)
            .field("query_button", &self.query_button.is_some())
            .finish_non_exhaustive()
    }
}

/// `QueryButton`'s `Width` and `Height`, and its `Margin`.
const QUERY_BUTTON_SIZE: (f64, f64) = (32.0, 28.0);
const QUERY_BUTTON_MARGIN: [f64; 4] = [2.0, 0.0, 0.0, 0.0];

impl AutoSuggestBoxTextBoxStyle {
    /// `DeleteButton`'s `Width`.
    pub(super) const DELETE_BUTTON_WIDTH: f64 = 32.0;

    /// The columns after the template's first two: `QueryButton`'s, and the margin after it.
    pub(super) fn trailing_columns(r: &ThemeResources) -> [ColumnDefinition; 2] {
        [
            ColumnDefinition::new(GridLength::AUTO),
            ColumnDefinition::new(GridLength::pixel(r.auto_suggest_box_right_button_margin)),
        ]
    }

    /// `QueryButton` in `QueryButtonStyle`: its content presenter at `AutoSuggestBoxInnerButtonMargin`
    /// within the button, with the text box's foreground but `TextControlButtonForeground` while
    /// the box is focused, and hidden while disabled.
    pub(super) fn query_button(
        &self,
        enabled: bool,
        text_box_focused: bool,
        text_box_foreground: Color,
    ) -> Option<WidgetRef> {
        let button = self.query_button.clone()?;
        let corner_radius = self.corner_radius;
        let icon = button.icon.clone();
        let button = CommonStates::new(button.click, move |app, context, states| {
            let r = ThemeResources::of(app, context);
            let (background, border, foreground) = match states.common {
                CommonState::PointerOver => (
                    r.text_control_button_background_pointer_over,
                    r.text_control_button_border_brush_pointer_over,
                    r.text_control_button_foreground_pointer_over,
                ),
                CommonState::Pressed => (
                    r.text_control_button_background_pressed,
                    r.text_control_button_border_brush_pressed,
                    r.text_control_button_foreground_pressed,
                ),
                _ if text_box_focused => (
                    r.text_control_button_background,
                    r.text_control_button_border_brush,
                    r.text_control_button_foreground,
                ),
                _ => (
                    r.text_control_button_background,
                    r.text_control_button_border_brush,
                    text_box_foreground,
                ),
            };
            let presenter = ControlBorder::new(Brush::Solid(background), Brush::Solid(border))
                .border_thickness_ltrb(r.text_control_border_theme_thickness)
                .corner_radius_corners(corner_radius)
                .child(Margin::new(
                    r.helper_button_theme_padding,
                    Center::new().child(DefaultTextStyle::new(
                        control_text_style(
                            &r,
                            r.auto_suggest_box_icon_font_size,
                            waft_embedder::FontWeight::NORMAL,
                            foreground,
                        ),
                        icon.clone(),
                    )),
                ));
            Opacity::new(if states.common == CommonState::Disabled {
                0.0
            } else {
                1.0
            })
            .child(
                SizedBox::new()
                    .width(QUERY_BUTTON_SIZE.0)
                    .height(QUERY_BUTTON_SIZE.1)
                    .child(Margin::new(
                        r.auto_suggest_box_inner_button_margin,
                        presenter,
                    )),
            )
            .into_widget()
        })
        .is_enabled(enabled)
        .is_tab_stop(false);
        Some(
            Margin::new(
                QUERY_BUTTON_MARGIN,
                Center::new().width_factor(1.0).child(button),
            )
            .into_widget(),
        )
    }
}

impl<T: fmt::Display + 'static> StatefulWidget for AutoSuggestBox<T> {
    type State = AutoSuggestBoxState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> AutoSuggestBoxState<T> {
        AutoSuggestBoxState {
            state: StateData::new(),
            text_box_key: GlobalKey::new(),
            text_focus: None,
            popup: None,
            scroll: None,
            counter: None,
            controller: None,
            text: String::new(),
            item_texts: Vec::new(),
            focused: false,
            open: false,
            pending_open: None,
            building: false,
            selected: None,
            position: SuggestionListPosition::Below,
            container_max_height: f64::INFINITY,
            opened_down: None,
            item_heights: Vec::new(),
            user_typed_text: String::new(),
            text_change_reason: None,
            text_changed_args: None,
            text_changed_timer: None,
            ignore_text_changes: false,
            ignore_selection_changes: false,
            suppress_suggestion_list_visibility: false,
        }
    }
}

/// The box's own state: the focus and popup it owns, and the source's flags.
pub struct AutoSuggestBoxState<T: fmt::Display + 'static> {
    state: StateData<AutoSuggestBox<T>>,

    /// `TextBox`, whose bounds `MaximizeSuggestionAreaWithoutInputPane` reads.
    text_box_key: GlobalKey,

    /// The text box's focus.
    text_focus: Option<Ref<FocusNode>>,

    /// `SuggestionsPopup`.
    popup: Option<Ref<OverlayPortalController>>,

    /// `SuggestionsList`'s scrolling, while the popup is open.
    scroll: Option<Ref<ScrollViewportController>>,

    /// `m_textChangedCounter`.
    counter: Option<Ref<TextChangedCounter>>,

    /// The controller whose listener is attached.
    controller: Option<TextEditingControllerRef>,

    /// The text last seen, to tell a change of the text from one of the selection.
    text: String,

    /// The items' texts, to tell a new list of suggestions from the same one passed again.
    item_texts: Vec<String>,

    /// Whether the text box has the focus.
    focused: bool,

    /// `IsSuggestionListOpen`.
    open: bool,

    /// An `is_suggestion_list_open` of the application's waiting for the frame's end.
    pending_open: Option<bool>,

    /// Whether the box is taking a new widget, when what it raises waits for the frame's end.
    building: bool,

    /// `SuggestionsList.SelectedIndex`, in the list's order.
    selected: Option<usize>,

    /// `m_suggestionListPosition`.
    position: SuggestionListPosition,

    /// `SuggestionsContainer.MaxHeight`, which `UpdateSuggestionListVisibility` reads.
    container_max_height: f64,

    /// Whether the open list was laid out below the box, for `UpdateCornerRadius`.
    opened_down: Option<bool>,

    /// The laid-out height of each container, in the list's order, for `ScrollIntoView`.
    item_heights: Vec<Option<f64>>,

    /// `m_userTypedText`.
    user_typed_text: String,

    /// `m_textChangeReason`: the reason the box gives the change it is making.
    text_change_reason: Option<AutoSuggestionBoxTextChangeReason>,

    /// `m_tpTextChangedEventArgs`: the arguments the timer raises.
    text_changed_args: Option<AutoSuggestBoxTextChangedEventArgs>,

    /// `m_tpTextChangedEventTimer`, while it runs.
    text_changed_timer: Option<Timer>,

    /// `m_ignoreTextChanges`.
    ignore_text_changes: bool,

    /// `m_ignoreSelectionChanges`.
    ignore_selection_changes: bool,

    /// `m_suppressSuggestionListVisibility`: set when the list closes, it keeps a focused box
    /// from opening the list again on its own. Nothing clears it, as nothing does in the source.
    suppress_suggestion_list_visibility: bool,
}

impl<T: fmt::Display + 'static> AutoSuggestBoxState<T> {
    fn alive(self: Handle<Self>, app: &App) -> bool {
        app.contains(self) && self.mounted(app)
    }

    fn view(self: Handle<Self>, app: &App) -> Option<waft_embedder::ViewId> {
        View::maybe_id_of(app, self.context(app))
    }

    /// Runs `work` once the frame is over, if the box is still mounted.
    fn after_frame(self: Handle<Self>, app: &mut App, work: impl Fn(&mut App) + 'static) {
        let view = self.view(app);
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if self.alive(app) {
                    work(app);
                }
            }),
        );
    }

    fn text_focus(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self)
            .text_focus
            .handle()
            .expect("an initialized AutoSuggestBox")
    }

    fn controller(self: Handle<Self>, app: &App) -> TextEditingControllerRef {
        self.widget(app).controller.clone()
    }

    fn item_count(self: Handle<Self>, app: &App) -> usize {
        self.widget(app).items_source.len()
    }

    /// The item at `index` of the list: of `items_source` from the end while the list is
    /// reversed.
    fn item_at(self: Handle<Self>, app: &App, index: usize) -> ChosenItem<T> {
        let items = self.widget(app).items_source.clone();
        let index = match app.get(self).position {
            SuggestionListPosition::Below => index,
            SuggestionListPosition::Above => items.len() - 1 - index,
        };
        ChosenItem { items, index }
    }

    // MARK: Text

    /// `OnTextBoxTextChanged`, for any change of the controller's text: counts it, restarts the
    /// TextChanged timer, and clears the selection. A change the user made is told apart by
    /// [`Self::text_box_user_input`], which the text box calls right after.
    fn text_did_change(self: Handle<Self>, app: &mut App) {
        let text = self.controller(app).text_value(app).to_owned();
        if app.get(self).text == text {
            return;
        }
        app.get_mut(self).text = text;
        let counter = app
            .get(self)
            .counter
            .handle()
            .expect("an initialized AutoSuggestBox");
        app.get_mut(counter).value += 1;
        let reason = app
            .get_mut(self)
            .text_change_reason
            .take()
            .unwrap_or(AutoSuggestionBoxTextChangeReason::ProgrammaticChange);
        app.get_mut(self).text_changed_args = Some(AutoSuggestBoxTextChangedEventArgs {
            reason,
            counter: app.get(counter).value,
            owner: counter,
        });
        self.restart_text_changed_timer(app);
        if !app.get(self).ignore_text_changes {
            self.deselect_ignoring_selection_change(app);
        }
    }

    /// The text box's own TextChanged, for an edit of the user's: the change is `UserInput`,
    /// its text what the user typed, and it opens the list when there are suggestions.
    fn text_box_user_input(self: Handle<Self>, app: &mut App, text: String) {
        if let Some(args) = &mut app.get_mut(self).text_changed_args {
            args.reason = AutoSuggestionBoxTextChangeReason::UserInput;
        }
        if !app.get(self).ignore_text_changes {
            app.get_mut(self).user_typed_text = text;
            self.update_suggestion_list_visibility(app);
        }
    }

    fn restart_text_changed_timer(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).text_changed_timer.take() {
            timer.cancel(app);
        }
        let timer = Timer::new(
            app,
            TEXT_CHANGED_EVENT_DELAY,
            Listener::new(move |app| {
                if self.alive(app) {
                    self.on_text_changed_event_timer_tick(app);
                }
            }),
        );
        app.get_mut(self).text_changed_timer = Some(timer);
    }

    /// `OnTextChangedEventTimerTick`: raises TextChanged with the last change's arguments.
    fn on_text_changed_event_timer_tick(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).text_changed_timer = None;
        let args = app.get(self).text_changed_args;
        if let (Some(args), Some(handler)) = (args, self.widget(app).text_changed.clone()) {
            handler(app, &args);
        }
    }

    /// `UpdateTextBoxText`: sets the text, with the caret at its end, for `reason`.
    fn update_text_box_text(
        self: Handle<Self>,
        app: &mut App,
        value: &str,
        reason: AutoSuggestionBoxTextChangeReason,
    ) {
        let controller = self.controller(app);
        if controller.text_value(app) == value {
            return;
        }
        app.get_mut(self).text_change_reason = Some(reason);
        let end = value.encode_utf16().count() as i32;
        controller.set_value(
            app,
            TextEditingValue::new()
                .text(value)
                .selection(TextSelection::collapsed(end, TextAffinity::Downstream)),
        );
        app.get_mut(self).text_change_reason = None;
    }

    /// Puts back the text the user typed, as a programmatic change.
    fn restore_user_typed_text(self: Handle<Self>, app: &mut App) {
        let text = app.get(self).user_typed_text.clone();
        self.update_text_box_text(
            app,
            &text,
            AutoSuggestionBoxTextChangeReason::ProgrammaticChange,
        );
    }

    // MARK: Selection

    /// `put_SelectedIndex` on `SuggestionsList`: a change raises `OnSuggestionSelectionChanged`.
    fn select(self: Handle<Self>, app: &mut App, index: Option<usize>) {
        if app.get(self).selected == index {
            return;
        }
        app.get_mut(self).selected = index;
        self.on_suggestion_selection_changed(app);
        self.set_state(app, |_| {});
    }

    /// Clears the selection without showing a suggestion or raising SuggestionChosen.
    fn deselect_ignoring_selection_change(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).ignore_selection_changes = true;
        self.select(app, None);
        app.get_mut(self).ignore_selection_changes = false;
    }

    /// `OnSuggestionSelectionChanged`: brings the selection, or the first item, into view; then,
    /// unless the box itself is clearing it, shows the selected suggestion's text and raises
    /// SuggestionChosen, ignoring the text changes that makes.
    fn on_suggestion_selection_changed(self: Handle<Self>, app: &mut App) {
        let selected = app.get(self).selected;
        if self.item_count(app) > 0 {
            self.scroll_into_view(app, selected.unwrap_or(0));
        }
        if app.get(self).ignore_selection_changes {
            return;
        }
        app.get_mut(self).ignore_text_changes = true;
        if let Some(index) = selected {
            let chosen = self.item_at(app, index);
            if self.widget(app).update_text_on_select {
                let text = self.widget(app).text_of(chosen.get());
                self.update_text_box_text(
                    app,
                    &text,
                    AutoSuggestionBoxTextChangeReason::SuggestionChosen,
                );
            }
            if let Some(handler) = self.widget(app).suggestion_chosen.clone() {
                handler(
                    app,
                    &AutoSuggestBoxSuggestionChosenEventArgs { selected: chosen },
                );
            }
        }
        app.get_mut(self).ignore_text_changes = false;
    }

    /// `OnKeyDown`'s Up and Down: moves the selection one item, from the box to the list's first
    /// or last item, and from either end back to the box and the text the user typed.
    fn move_selection(self: Handle<Self>, app: &mut App, forward: bool) {
        let count = self.item_count(app);
        if count == 0 {
            return;
        }
        let last = count - 1;
        let selected = app.get(self).selected;
        if selected == Some(0) && last == 0 {
            return;
        }
        match selected {
            None => self.select(app, Some(if forward { 0 } else { last })),
            Some(index) if (index == 0 && !forward) || (index == last && forward) => {
                self.restore_user_typed_text(app);
                self.select(app, None);
            }
            Some(index) => self.select(app, Some(if forward { index + 1 } else { index - 1 })),
        }
    }

    // MARK: Submitting

    /// `SubmitQuery`: raises QuerySubmitted with the box's text and closes the list.
    fn submit_query(self: Handle<Self>, app: &mut App, chosen: Option<ChosenItem<T>>) {
        let query_text = self.controller(app).text_value(app).to_owned();
        if let Some(handler) = self.widget(app).query_submitted.clone() {
            handler(
                app,
                &AutoSuggestBoxQuerySubmittedEventArgs { query_text, chosen },
            );
        }
        self.put_is_suggestion_list_open(app, false);
    }

    /// `ProgrammaticSubmitQuery`, for the query button: the text alone, whatever is selected.
    fn programmatic_submit_query(self: Handle<Self>, app: &mut App) {
        self.deselect_ignoring_selection_change(app);
        self.submit_query(app, None);
    }

    /// `OnListViewItemClick`: the click selects the item, which shows its text and raises
    /// SuggestionChosen, and then submits it.
    fn item_clicked(self: Handle<Self>, app: &mut App, index: usize) {
        if index >= self.item_count(app) {
            return;
        }
        let chosen = self.item_at(app, index);
        self.select(app, Some(index));
        self.submit_query(app, Some(chosen));
    }

    // MARK: Opening and closing

    /// `UpdateSuggestionListVisibility`: the list is open while there are suggestions and room
    /// for them.
    fn update_suggestion_list_visibility(self: Handle<Self>, app: &mut App) {
        let open = self.item_count(app) > 0 && app.get(self).container_max_height > 0.0;
        self.put_is_suggestion_list_open(app, open);
    }

    /// The generated `put_IsSuggestionListOpen`: a change opens or closes the list and is asked
    /// of an application that owns `is_suggestion_list_open`. While the box takes a new widget,
    /// it waits for the frame's end.
    fn put_is_suggestion_list_open(self: Handle<Self>, app: &mut App, open: bool) {
        if app.get(self).building {
            self.after_frame(app, move |app| self.put_is_suggestion_list_open(app, open));
            return;
        }
        if app.get(self).open == open {
            return;
        }
        if let Some(changed) = self.widget(app).is_suggestion_list_open_changed.clone() {
            changed(app, open);
        }
        self.set_suggestion_list_open(app, open);
    }

    /// `OnPropertyChanged2` for `IsSuggestionListOpen`: shows or hides `SuggestionsPopup`; a
    /// closing clears the selection and keeps the focused box from opening the list again.
    fn set_suggestion_list_open(self: Handle<Self>, app: &mut App, open: bool) {
        if app.get(self).open == open {
            return;
        }
        app.get_mut(self).open = open;
        let popup = app
            .get(self)
            .popup
            .handle()
            .expect("an initialized AutoSuggestBox");
        if open {
            let scroll = ScrollViewportController::new(app);
            let state = app.get_mut(self);
            state.scroll = Some(scroll);
            state.item_heights.clear();
            popup.show(app);
        } else {
            popup.hide(app);
            self.retire_scroll(app);
            let state = app.get_mut(self);
            state.opened_down = None;
            state.suppress_suggestion_list_visibility = true;
            self.deselect_ignoring_selection_change(app);
        }
        self.set_state(app, |_| {});
    }

    /// Disposes the closed list's scroll position once the list is gone.
    fn retire_scroll(self: Handle<Self>, app: &mut App) {
        let Some(scroll) = app.get_mut(self).scroll.take() else {
            return;
        };
        let view = self.view(app);
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| scroll.dispose(app)),
        );
    }

    /// An application that owns `is_suggestion_list_open` and passes another state than the
    /// box's has the box open or close once the frame is over; opening it focuses the box.
    fn settle_suggestion_list_open(self: Handle<Self>, app: &mut App) {
        let Some(open) = self.widget(app).is_suggestion_list_open else {
            return;
        };
        if open == app.get(self).open || app.get(self).pending_open == Some(open) {
            return;
        }
        app.get_mut(self).pending_open = Some(open);
        self.after_frame(app, move |app| {
            app.get_mut(self).pending_open = None;
            if self.widget(app).is_suggestion_list_open != Some(open) {
                return;
            }
            self.set_suggestion_list_open(app, open);
            if open {
                self.text_focus(app).request_focus(app, None);
            }
        });
    }

    /// `OnItemsChanged`: a new list of suggestions clears the selection, and opens or closes the
    /// list of a focused box.
    fn on_items_changed(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).selected = None;
        app.get_mut(self).item_heights.clear();
        if app.get(self).focused {
            self.update_suggestion_list_visibility(app);
        }
    }

    // MARK: Focus and keys

    fn text_focus_changed(self: Handle<Self>, app: &mut App) {
        let focused = self.text_focus(app).has_focus(app);
        if app.get(self).focused == focused {
            return;
        }
        app.get_mut(self).focused = focused;
        if focused {
            self.on_got_focus(app);
        } else {
            // `OnLostFocus`.
            self.put_is_suggestion_list_open(app, false);
        }
    }

    /// `OnGotFocus`: a box with text shows its suggestions again, unless a closing suppressed it.
    fn on_got_focus(self: Handle<Self>, app: &mut App) {
        let has_text = !self.controller(app).text_value(app).is_empty();
        if has_text && !app.get(self).suppress_suggestion_list_visibility {
            self.update_suggestion_list_visibility(app);
            app.get_mut(self).suppress_suggestion_list_visibility = false;
        }
    }

    /// `OnKeyDown`, for a key reaching the text box: Up and Down move through an open list,
    /// Escape and Tab put back the typed text and close it, and Enter submits.
    fn on_key_down(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        let open = app.get(self).open;
        if key == LogicalKeyboardKey::ARROW_UP || key == LogicalKeyboardKey::ARROW_DOWN {
            if !open {
                return KeyEventResult::Ignored;
            }
            self.move_selection(app, key == LogicalKeyboardKey::ARROW_DOWN);
            KeyEventResult::Handled
        } else if key == LogicalKeyboardKey::ESCAPE || (key == LogicalKeyboardKey::TAB && open) {
            self.restore_user_typed_text(app);
            self.put_is_suggestion_list_open(app, false);
            self.text_focus(app).request_focus(app, None);
            // Tab goes on to move the focus.
            if key == LogicalKeyboardKey::TAB {
                KeyEventResult::Ignored
            } else {
                KeyEventResult::Handled
            }
        } else if key == LogicalKeyboardKey::ENTER || key == LogicalKeyboardKey::NUMPAD_ENTER {
            let chosen = app.get(self).selected.map(|index| self.item_at(app, index));
            self.submit_query(app, chosen);
            self.put_is_suggestion_list_open(app, false);
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }

    // MARK: Scrolling

    /// `ScrollIntoView` with the default alignment: the least scrolling that shows the item.
    fn scroll_into_view(self: Handle<Self>, app: &mut App, index: usize) {
        let Some(scroll) = app.get(self).scroll.handle() else {
            return;
        };
        let metrics = scroll.metrics(app);
        if metrics.viewport_length <= 0.0 {
            return;
        }
        let estimate = ThemeResources::of(app, self.context(app)).list_view_item_min_height;
        let heights = &app.get(self).item_heights;
        let height_of = |index: usize| heights.get(index).copied().flatten().unwrap_or(estimate);
        let top: f64 = (0..index).map(height_of).sum();
        let bottom = top + height_of(index);
        let target = if top < metrics.offset {
            top
        } else if bottom > metrics.offset + metrics.viewport_length {
            bottom - metrics.viewport_length
        } else {
            return;
        };
        scroll.change_view(app, target, true);
    }

    /// `ScrollLastItemIntoView`, for a list that opens above the box.
    fn scroll_last_item_into_view(self: Handle<Self>, app: &mut App) {
        let count = self.item_count(app);
        if count > 1 {
            self.scroll_into_view(app, count - 1);
        }
    }

    fn item_measured(self: Handle<Self>, app: &mut App, index: usize, height: f64) {
        let heights = &mut app.get_mut(self).item_heights;
        if heights.len() <= index {
            heights.resize(index + 1, None);
        }
        heights[index] = Some(height);
    }

    // MARK: Template

    /// `TextBox`'s corners: `UpdateCornerRadius` squares those next to the open list.
    fn text_box_corner_radius(self: Handle<Self>, app: &App, r: &ThemeResources) -> [f64; 4] {
        let radius = r.control_corner_radius;
        match app.get(self).opened_down {
            Some(true) => [radius[0], radius[1], 0.0, 0.0],
            Some(false) => [0.0, 0.0, radius[2], radius[3]],
            None => radius,
        }
    }

    /// The whole `TextBox`, header and description included, in the root overlay.
    fn text_box_bounds(self: Handle<Self>, app: &mut App) -> Option<Rect> {
        let text_box = app
            .get(self)
            .text_box_key
            .clone()
            .current_context(app)?
            .find_render_object(app)?
            .as_box()?;
        let overlay = Overlay::of(app, self.context(app), true);
        let root = overlay.context(app).find_render_object(app)?.as_box()?;
        let transform = text_box
            .as_object()
            .get_transform_to(app, Some(root.as_object()));
        Some(transform_rect(
            &transform,
            Offset::ZERO & text_box.size(app),
        ))
    }

    /// The child of `SuggestionsPopup` in the root overlay: `SuggestionsContainer` where
    /// `UpdateSuggestionListPosition` puts it, beside `BorderElement`, the popup's anchor.
    fn suggestions_popup(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        info: OverlayChildLayoutInfo,
    ) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let border_element = transform_rect(
            &info.child_paint_transform(),
            Offset::ZERO & info.child_size(),
        );
        let text_box = self.text_box_bounds(app).unwrap_or(border_element);
        // `ContentElement`'s `Margin` is the text box's `BorderThickness`.
        let thickness = r.text_control_border_theme_thickness;
        let content_element = Rect::from_ltrb(
            border_element.left + thickness[0],
            border_element.top + thickness[1],
            border_element.right - thickness[2],
            border_element.bottom - thickness[3],
        );
        let layout = placement::arrange(
            text_box,
            content_element,
            info.overlay_size(),
            self.widget(app).max_suggestion_list_height,
        );
        self.take_layout(app, layout);
        let container = self.suggestions_container(app, context, layout.position);
        CustomSingleChildLayout::new(Rc::new(SuggestionsPosition(layout)))
            .child(TextFieldTapRegion::new(container))
            .into_widget()
    }

    /// Keeps what the layout decided: the container's height for `UpdateSuggestionListVisibility`,
    /// and the side it opens on. A change of side reverses the list, which clears its selection,
    /// and `UpdateCornerRadius` follows once the frame is over.
    fn take_layout(self: Handle<Self>, app: &mut App, layout: SuggestionListLayout) {
        app.get_mut(self).container_max_height = layout.max_height;
        let position = layout.position;
        if app.get(self).position != position {
            app.get_mut(self).position = position;
            app.get_mut(self).item_heights.clear();
            self.after_frame(app, move |app| {
                self.deselect_ignoring_selection_change(app);
                if position == SuggestionListPosition::Above {
                    self.scroll_last_item_into_view(app);
                }
            });
        }
        let down = position == SuggestionListPosition::Below;
        if app
            .get(self)
            .opened_down
            .is_some_and(|opened| opened == down)
        {
            return;
        }
        self.after_frame(app, move |app| {
            if app.get(self).open {
                self.set_state(app, |state| state.opened_down = Some(down));
            }
        });
    }

    /// `SuggestionsContainer`: `SuggestionsList` at `AutoSuggestListPadding` within the acrylic
    /// border, its corners next to the box square, with ElevationHelper's shadow.
    fn suggestions_container(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        position: SuggestionListPosition,
    ) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let Some(scroll) = app.get(self).scroll.clone() else {
            return SizedBox::shrink().into_widget();
        };
        let overlay = r.overlay_corner_radius;
        let corner_radius = match position {
            SuggestionListPosition::Below => [0.0, 0.0, overlay[2], overlay[3]],
            SuggestionListPosition::Above => [overlay[0], overlay[1], 0.0, 0.0],
        };
        let list = ListView::builder(move |app, _, index| self.suggestion(app, index as usize))
            .item_count(self.item_count(app) as i32)
            .build()
            .controller(scroll.native_controller(app))
            .shrink_wrap(true)
            .padding(EdgeInsetsGeometry::ZERO);
        let suggestions_list = Margin::new(
            r.auto_suggest_list_padding,
            ConstrainedBox::new(BoxConstraints::new().max_height(r.auto_suggest_list_max_height))
                .child(ScrollBarViewport::scroller(scroll, list)),
        );
        let border = ControlBorder::new(
            Brush::Acrylic(r.auto_suggest_box_suggestions_list_background),
            Brush::Solid(r.auto_suggest_box_suggestions_list_border_brush),
        )
        .border_thickness_ltrb(r.auto_suggest_list_border_theme_thickness)
        .corner_radius_corners(corner_radius)
        .background_sizing(BackgroundSizing::InnerBorderEdge)
        .padding(r.auto_suggest_list_margin)
        .child(suggestions_list);
        CustomPaint::new()
            .painter(PopupShadow::for_anchored_popup(r.theme, corner_radius))
            .child(border)
            .into_widget()
    }

    /// The container of the list's item at `index`, measured where the list lays it out.
    fn suggestion(self: Handle<Self>, app: &mut App, index: usize) -> Option<WidgetRef> {
        if index >= self.item_count(app) {
            return None;
        }
        let chosen = self.item_at(app, index);
        let widget = self.widget(app);
        let content = match widget.item_template.clone() {
            Some(template) => template(app, self.context(app), chosen.get()),
            None => Text::new(chosen.get().to_string())
                .soft_wrap(false)
                .into_widget(),
        };
        let item = SuggestionItem {
            content,
            selected: app.get(self).selected == Some(index),
            clicked: Listener::new(move |app| self.item_clicked(app, index)),
        };
        Some(
            SizeObserver::new(
                item,
                Rc::new(move |app, _, size| {
                    if self.alive(app) {
                        self.item_measured(app, index, size.height());
                    }
                }),
            )
            .into_widget(),
        )
    }

    /// Connects the application's controller, keeping what its text was.
    fn sync_controller(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        if app.get(self).controller.handle() == Some(controller.handle()) {
            return;
        }
        if let Some(old) = app.get_mut(self).controller.take() {
            old.remove_listener(app, &Listener::handle_method(self, Self::text_did_change));
        }
        controller.add_listener(app, Listener::handle_method(self, Self::text_did_change));
        let text = controller.text_value(app).to_owned();
        let state = app.get_mut(self);
        state.text = text;
        state.controller = Some(controller);
    }

    /// The items' texts, which tell a new list from the one passed again.
    fn item_texts(self: Handle<Self>, app: &App) -> Vec<String> {
        self.widget(app)
            .items_source
            .iter()
            .map(ToString::to_string)
            .collect()
    }
}

impl<T: fmt::Display + 'static> State for AutoSuggestBoxState<T> {
    type Widget = AutoSuggestBox<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let text_focus = FocusNode::new(app);
        text_focus.as_node().set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.on_key_down(app, event))),
        );
        text_focus.add_listener(app, Listener::handle_method(self, Self::text_focus_changed));
        let popup =
            OverlayPortalController::new(app, Some("AutoSuggestBox SuggestionsPopup".into()));
        let counter = app.create(TextChangedCounter { value: 0 });
        let item_texts = self.item_texts(app);
        let state = app.get_mut(self);
        state.text_focus = Some(text_focus);
        state.popup = Some(popup);
        state.counter = Some(counter);
        state.item_texts = item_texts;
        self.sync_controller(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &AutoSuggestBox<T>) {
        app.get_mut(self).building = true;
        self.sync_controller(app);
        let item_texts = self.item_texts(app);
        if app.get(self).item_texts != item_texts {
            app.get_mut(self).item_texts = item_texts;
            self.on_items_changed(app);
        }
        app.get_mut(self).building = false;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).text_changed_timer.take() {
            timer.cancel(app);
        }
        if let Some(controller) = app.get_mut(self).controller.take() {
            controller.remove_listener(app, &Listener::handle_method(self, Self::text_did_change));
        }
        let state = app.get_mut(self);
        let text_focus = state
            .text_focus
            .take()
            .expect("an initialized AutoSuggestBox");
        let popup = state.popup.take();
        let scroll = state.scroll.take();
        text_focus.remove_listener(
            app,
            &Listener::handle_method(self, Self::text_focus_changed),
        );
        text_focus.dispose(app);
        drop(popup);
        if let Some(scroll) = scroll {
            scroll.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        self.settle_suggestion_list_open(app);
        let widget = self.widget(app).clone();
        let r = ThemeResources::of(app, context);
        let popup = app
            .get(self)
            .popup
            .clone()
            .expect("an initialized AutoSuggestBox");
        let mut config = TextBox::new(widget.controller.clone())
            .focus_node(
                app.get(self)
                    .text_focus
                    .as_ref()
                    .expect("an initialized AutoSuggestBox")
                    .erase(),
            )
            .placeholder_text(widget.placeholder_text.clone())
            .text_changed(move |app, text| self.text_box_user_input(app, text));
        if let Some(header) = widget.header.clone() {
            config = config.header(header);
        }
        if let Some(description) = widget.description.clone() {
            config = config.description(description);
        }
        let style = AutoSuggestBoxTextBoxStyle {
            corner_radius: self.text_box_corner_radius(app, &r),
            query_button: widget.query_icon.clone().map(|icon| QueryButton {
                icon,
                click: Listener::new(move |app| self.programmatic_submit_query(app)),
            }),
            border_element: Rc::new(move |border| {
                OverlayPortal::overlay_child_layout_builder(
                    popup.clone(),
                    move |app, context, info| self.suggestions_popup(app, context, info),
                )
                .overlay_location(OverlayChildLocation::RootOverlay)
                .child(border)
                .into_widget()
            }),
        };
        KeyedSubtree::new(TextControl::new(config, None).auto_suggest_box_style(style))
            .key(app.get(self).text_box_key.clone().into())
            .into_widget()
    }
}
