//! ContentDialog_themeresources.xaml and ContentDialog_Partial.cpp: a modal dialog over the
//! window with a title, content and up to three command buttons. `show_async` pushes it onto the
//! root navigator as a dialog route whose barrier is the smoke layer; a command button, Escape,
//! Enter on the default button or `hide` closes it through the cancelable Closing event.
//!
//! The invariant a later editor must keep: a showing ends only in `finish_closing`, once the
//! route has animated out or its page is gone, so the Closed event and the result always come
//! after the close transition, and exactly once per `show_async`.

use super::flyout::shadow::PopupShadow;
use crate::*;
use inset_animation::{AnimationStatus, AnimationStatusListener, AnyAnimation, Cubic, Curve};
use inset_embedder::{FontWeight, TextDirection};
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Completer, CompleterFuture, Handle, HandleId,
    ListenableObject, Listener,
};
use inset_painting::BorderRadius;
use inset_rendering::{BoxConstraints, CrossAxisAlignment, MainAxisSize};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{cell::Cell, collections::HashMap, fmt, rc::Rc, time::Duration};

/// XAML `ContentDialogButton`: one of the three command buttons, or none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContentDialogButton {
    #[default]
    None,
    Primary,
    Secondary,
    Close,
}

/// XAML `ContentDialogResult`: the command that closed the dialog. The close button, Escape and
/// `hide` close it with `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContentDialogResult {
    #[default]
    None,
    Primary,
    Secondary,
}

/// `ContentDialogButtonClickDeferral`: what `ContentDialogButtonClickEventArgs::get_deferral`
/// gives.
pub type ContentDialogButtonClickDeferral = Deferral;

/// `ContentDialogClosingDeferral`: what `ContentDialogClosingEventArgs::get_deferral` gives.
pub type ContentDialogClosingDeferral = Deferral;

/// Arguments of a command button's click, raised before the dialog closes. A handler that
/// decides later keeps them and takes a deferral.
pub struct ContentDialogButtonClickEventArgs {
    cancel: Cell<bool>,
    deferrals: Rc<DeferralManager>,
    generation: u64,
}

impl ContentDialogButtonClickEventArgs {
    /// `Cancel`: whether the dialog stays open.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Sets Cancel.
    pub fn set_cancel(&self, cancel: bool) {
        self.cancel.set(cancel);
    }

    /// `GetDeferral`: the dialog reads Cancel, and closes, once the deferral is complete.
    pub fn get_deferral(&self) -> Result<ContentDialogButtonClickDeferral, DeferralCompleted> {
        self.deferrals.get_deferral(self.generation)
    }
}

impl fmt::Debug for ContentDialogButtonClickEventArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContentDialogButtonClickEventArgs")
            .field("cancel", &self.cancel.get())
            .finish_non_exhaustive()
    }
}

/// Arguments of the Closing event. A handler that decides later keeps them and takes a
/// deferral.
pub struct ContentDialogClosingEventArgs {
    result: ContentDialogResult,
    cancel: Cell<bool>,
    deferrals: Rc<DeferralManager>,
    generation: u64,
}

impl ContentDialogClosingEventArgs {
    /// `Result`: the result the dialog is closing with.
    pub fn result(&self) -> ContentDialogResult {
        self.result
    }

    /// `Cancel`: whether the dialog stays open.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Sets Cancel.
    pub fn set_cancel(&self, cancel: bool) {
        self.cancel.set(cancel);
    }

    /// `GetDeferral`: the dialog reads Cancel, and closes, once the deferral is complete.
    pub fn get_deferral(&self) -> Result<ContentDialogClosingDeferral, DeferralCompleted> {
        self.deferrals.get_deferral(self.generation)
    }
}

impl fmt::Debug for ContentDialogClosingEventArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContentDialogClosingEventArgs")
            .field("result", &self.result)
            .field("cancel", &self.cancel.get())
            .finish_non_exhaustive()
    }
}

/// Arguments of the Closed event.
#[derive(Clone, Copy, Debug)]
pub struct ContentDialogClosedEventArgs {
    /// The result the dialog closed with.
    pub result: ContentDialogResult,
}

/// Receives a command button's cancelable click.
pub type ContentDialogButtonClickHandler =
    Rc<dyn Fn(&mut App, &Rc<ContentDialogButtonClickEventArgs>)>;

/// Receives the cancelable Closing event.
pub type ContentDialogClosingHandler = Rc<dyn Fn(&mut App, &Rc<ContentDialogClosingEventArgs>)>;

/// Receives the Closed event.
pub type ContentDialogClosedHandler = Rc<dyn Fn(&mut App, ContentDialogClosedEventArgs)>;

/// Why `show_async` refused to show a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentDialogError {
    /// `ERROR_CONTENTDIALOG_MULTIPLE_OPEN`: this dialog is still showing, or another is open in
    /// the same window.
    MultipleOpen,
}

impl fmt::Display for ContentDialogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MultipleOpen => f.write_str("only one ContentDialog can be open at a time"),
        }
    }
}

impl std::error::Error for ContentDialogError {}

/// `ContentDialogMetadata`: the dialog open in each window, keyed by its root navigator.
#[derive(Default)]
struct ContentDialogMetadata {
    open: HashMap<HandleId, Handle<ContentDialog>>,
}

/// A persistent dialog object: set its properties, show it with `show_async` from a context
/// beneath a `Navigator`, and dispose it when its owner is disposed.
pub struct ContentDialog {
    /// `Title`: shown above the content in the subtitle size, on at most two lines. Without one,
    /// the title row and its margin collapse.
    pub title: Option<WidgetRef>,

    /// `Content`: shown below the title in the body size, wrapping.
    pub content: Option<WidgetRef>,

    /// `PrimaryButtonText`: the leftmost button's label; empty hides the button.
    pub primary_button_text: String,

    /// `SecondaryButtonText`: the middle button's label; empty hides the button.
    pub secondary_button_text: String,

    /// `CloseButtonText`: the rightmost button's label, which Escape also invokes; empty hides
    /// the button.
    pub close_button_text: String,

    /// `IsPrimaryButtonEnabled`.
    pub is_primary_button_enabled: bool,

    /// `IsSecondaryButtonEnabled`.
    pub is_secondary_button_enabled: bool,

    /// `DefaultButton`: drawn in the accent style and invoked by Enter.
    pub default_button: ContentDialogButton,

    /// `FullSizeDesired`: whether the dialog takes the window's height, up to its maximum.
    pub full_size_desired: bool,

    /// `PrimaryButtonClick`: can cancel the close the primary button asks for.
    pub primary_button_click: Option<ContentDialogButtonClickHandler>,

    /// `SecondaryButtonClick`: can cancel the close the secondary button asks for.
    pub secondary_button_click: Option<ContentDialogButtonClickHandler>,

    /// `CloseButtonClick`: can cancel the close the close button or Escape asks for.
    pub close_button_click: Option<ContentDialogButtonClickHandler>,

    /// `Opened`: called after the dialog's first frame and its initial focus.
    pub opened: Option<Listener>,

    /// `Closing`: can cancel any close but one the dialog's route was removed for.
    pub closing: Option<ContentDialogClosingHandler>,

    /// `Closed`: called after the close transition, before the result is delivered.
    pub closed: Option<ContentDialogClosedHandler>,

    /// Rebuilds the shown presenter when a property changes.
    notifier: ChangeNotifierData,

    /// The current showing, from `show_async` until the dialog has finished closing.
    showing: Option<Showing>,

    /// `m_spButtonClickDeferralManager`: a new one for each showing, so a click deferral that a
    /// closed showing left pending cannot hold back the next showing's clicks.
    button_click_deferrals: Rc<DeferralManager>,

    /// `m_spClosingDeferralManager`.
    closing_deferrals: Rc<DeferralManager>,

    /// Set by `dispose`; the object is destroyed once no showing needs it.
    disposed: bool,
}

/// One showing of a dialog.
struct Showing {
    route: AnyRoute,
    navigator: Handle<NavigatorState>,

    /// `m_tpCurrentAsyncOperation`: completed with the result once the dialog has closed.
    completion: Completer<ContentDialogResult>,

    /// The result the dialog closes with, once a close is under way.
    result: ContentDialogResult,

    /// `m_hideInProgress`: a close was accepted and the route is leaving.
    hide_in_progress: bool,

    /// `m_skipClosingEventOnHide`: true until the dialog has opened, and when the route was
    /// removed by something other than the dialog.
    skip_closing_event: bool,

    /// The theme and direction of the context that showed the dialog, which its route, in the
    /// root overlay, does not inherit.
    resources: Rc<ThemeResources>,
    direction: TextDirection,
}

impl fmt::Debug for ContentDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContentDialog")
            .field("primary_button_text", &self.primary_button_text)
            .field("secondary_button_text", &self.secondary_button_text)
            .field("close_button_text", &self.close_button_text)
            .field("default_button", &self.default_button)
            .field("showing", &self.showing.is_some())
            .finish_non_exhaustive()
    }
}

impl ChangeNotifier for ContentDialog {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}

impl ContentDialog {
    /// Creates a dialog with no title, no content and no buttons.
    pub fn new(app: &mut App) -> Handle<Self> {
        app.create(Self {
            title: None,
            content: None,
            primary_button_text: String::new(),
            secondary_button_text: String::new(),
            close_button_text: String::new(),
            is_primary_button_enabled: true,
            is_secondary_button_enabled: true,
            default_button: ContentDialogButton::None,
            full_size_desired: false,
            primary_button_click: None,
            secondary_button_click: None,
            close_button_click: None,
            opened: None,
            closing: None,
            closed: None,
            notifier: ChangeNotifierData::new(),
            showing: None,
            button_click_deferrals: DeferralManager::new(),
            closing_deferrals: DeferralManager::new(),
            disposed: false,
        })
    }

    /// Sets Title.
    pub fn title<K>(self: Handle<Self>, app: &mut App, title: impl IntoWidget<K>) -> Handle<Self> {
        self.change(app, |dialog| dialog.title = Some(title.into_widget()))
    }

    /// Sets Content.
    pub fn content<K>(
        self: Handle<Self>,
        app: &mut App,
        content: impl IntoWidget<K>,
    ) -> Handle<Self> {
        self.change(app, |dialog| dialog.content = Some(content.into_widget()))
    }

    /// Sets PrimaryButtonText.
    pub fn primary_button_text(
        self: Handle<Self>,
        app: &mut App,
        text: impl Into<String>,
    ) -> Handle<Self> {
        self.change(app, |dialog| dialog.primary_button_text = text.into())
    }

    /// Sets SecondaryButtonText.
    pub fn secondary_button_text(
        self: Handle<Self>,
        app: &mut App,
        text: impl Into<String>,
    ) -> Handle<Self> {
        self.change(app, |dialog| dialog.secondary_button_text = text.into())
    }

    /// Sets CloseButtonText.
    pub fn close_button_text(
        self: Handle<Self>,
        app: &mut App,
        text: impl Into<String>,
    ) -> Handle<Self> {
        self.change(app, |dialog| dialog.close_button_text = text.into())
    }

    /// Sets IsPrimaryButtonEnabled.
    pub fn is_primary_button_enabled(
        self: Handle<Self>,
        app: &mut App,
        value: bool,
    ) -> Handle<Self> {
        self.change(app, |dialog| dialog.is_primary_button_enabled = value)
    }

    /// Sets IsSecondaryButtonEnabled.
    pub fn is_secondary_button_enabled(
        self: Handle<Self>,
        app: &mut App,
        value: bool,
    ) -> Handle<Self> {
        self.change(app, |dialog| dialog.is_secondary_button_enabled = value)
    }

    /// Sets DefaultButton.
    pub fn default_button(
        self: Handle<Self>,
        app: &mut App,
        button: ContentDialogButton,
    ) -> Handle<Self> {
        self.change(app, |dialog| dialog.default_button = button)
    }

    /// Sets FullSizeDesired.
    pub fn full_size_desired(self: Handle<Self>, app: &mut App, value: bool) -> Handle<Self> {
        self.change(app, |dialog| dialog.full_size_desired = value)
    }

    /// Sets the PrimaryButtonClick handler.
    pub fn primary_button_click(
        self: Handle<Self>,
        app: &mut App,
        handler: impl Fn(&mut App, &Rc<ContentDialogButtonClickEventArgs>) + 'static,
    ) -> Handle<Self> {
        app.get_mut(self).primary_button_click = Some(Rc::new(handler));
        self
    }

    /// Sets the SecondaryButtonClick handler.
    pub fn secondary_button_click(
        self: Handle<Self>,
        app: &mut App,
        handler: impl Fn(&mut App, &Rc<ContentDialogButtonClickEventArgs>) + 'static,
    ) -> Handle<Self> {
        app.get_mut(self).secondary_button_click = Some(Rc::new(handler));
        self
    }

    /// Sets the CloseButtonClick handler.
    pub fn close_button_click(
        self: Handle<Self>,
        app: &mut App,
        handler: impl Fn(&mut App, &Rc<ContentDialogButtonClickEventArgs>) + 'static,
    ) -> Handle<Self> {
        app.get_mut(self).close_button_click = Some(Rc::new(handler));
        self
    }

    /// Sets the Opened handler.
    pub fn opened(self: Handle<Self>, app: &mut App, handler: Listener) -> Handle<Self> {
        app.get_mut(self).opened = Some(handler);
        self
    }

    /// Sets the Closing handler.
    pub fn closing(
        self: Handle<Self>,
        app: &mut App,
        handler: impl Fn(&mut App, &Rc<ContentDialogClosingEventArgs>) + 'static,
    ) -> Handle<Self> {
        app.get_mut(self).closing = Some(Rc::new(handler));
        self
    }

    /// Sets the Closed handler.
    pub fn closed(
        self: Handle<Self>,
        app: &mut App,
        handler: impl Fn(&mut App, ContentDialogClosedEventArgs) + 'static,
    ) -> Handle<Self> {
        app.get_mut(self).closed = Some(Rc::new(handler));
        self
    }

    /// `ShowAsync`: shows the dialog over the window of `context`'s root navigator. The future
    /// completes with the result once the dialog has closed.
    pub fn show_async(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> Result<CompleterFuture<ContentDialogResult>, ContentDialogError> {
        assert!(
            !app.get(self).disposed,
            "cannot show a disposed ContentDialog"
        );
        let navigator = Navigator::of(app, context, true);
        let metadata = app.singleton::<ContentDialogMetadata>();
        if app.get(self).showing.is_some() || app.get(metadata).open.contains_key(&navigator.id()) {
            return Err(ContentDialogError::MultipleOpen);
        }
        let resources = ThemeResources::of(app, context);
        let direction = Directionality::maybe_of(app, context).unwrap_or(TextDirection::Ltr);
        let dialog_route = self.dialog_route(app, &resources);
        let route = dialog_route.as_route();
        let completion = Completer::new();
        let result = completion.future();
        app.get_mut(self).showing = Some(Showing {
            route,
            navigator,
            completion,
            result: ContentDialogResult::None,
            hide_in_progress: false,
            skip_closing_event: true,
            resources,
            direction,
        });
        app.get_mut(metadata).open.insert(navigator.id(), self);
        navigator
            .push(app, route)
            .then(app, move |app, _| self.route_popped(app, route));
        self.finish_closing_when_dismissed(app, dialog_route);
        Ok(result)
    }

    /// `Hide`: closes the dialog with `None`, through the Closing event.
    pub fn hide(self: Handle<Self>, app: &mut App) {
        self.hide_internal(app, ContentDialogResult::None);
    }

    /// Releases the dialog. A shown dialog closes first, without its Closing and Closed events,
    /// and its result is `None`.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed {
            return;
        }
        app.get_mut(self).disposed = true;
        if app.get(self).showing.is_none() {
            self.notifier_dispose(app);
            app.destroy(self);
            return;
        }
        self.hide_internal(app, ContentDialogResult::None);
    }

    /// Applies `change` to the dialog's properties and rebuilds a shown presenter.
    fn change(self: Handle<Self>, app: &mut App, change: impl FnOnce(&mut Self)) -> Handle<Self> {
        change(app.get_mut(self));
        self.notify_listeners(app);
        self
    }

    /// The route the dialog is shown in, with the smoke layer of `resources`.
    fn dialog_route(
        self: Handle<Self>,
        app: &mut App,
        resources: &ThemeResources,
    ) -> Handle<ContentDialogRoute> {
        let smoke = resources.content_dialog_smoke_fill;
        ContentDialogRoute::new(app, self, smoke)
    }

    /// Ends the showing once `route` has animated out, even when its page never mounted, as
    /// when the dialog is hidden in the frame it was shown in. The page's removal ends it
    /// otherwise, whichever comes first.
    fn finish_closing_when_dismissed(
        self: Handle<Self>,
        app: &mut App,
        route: Handle<ContentDialogRoute>,
    ) {
        let Some(animation) = TransitionRoute::animation(route, app) else {
            return;
        };
        let shown = route.as_route();
        animation.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                if status == AnimationStatus::Dismissed {
                    self.finish_closing(app, shown);
                }
            }),
        );
    }

    /// `OnCommandButtonClicked`: raises the button's cancelable click and, once the click's
    /// deferrals are complete and no handler canceled, closes with the button's result. A click
    /// while an earlier click's deferrals are outstanding is ignored.
    fn command_button_clicked(self: Handle<Self>, app: &mut App, button: ContentDialogButton) {
        if !app.contains(self) || !self.is_accepting_commands(app) {
            return;
        }
        let dialog = app.get(self);
        let (handler, result) = match button {
            ContentDialogButton::Primary => (
                dialog.primary_button_click.clone(),
                ContentDialogResult::Primary,
            ),
            ContentDialogButton::Secondary => (
                dialog.secondary_button_click.clone(),
                ContentDialogResult::Secondary,
            ),
            ContentDialogButton::Close => {
                (dialog.close_button_click.clone(), ContentDialogResult::None)
            }
            ContentDialogButton::None => return,
        };
        let deferrals = Rc::clone(&dialog.button_click_deferrals);
        let Some(generation) = deferrals.prepare() else {
            return;
        };
        let args = Rc::new(ContentDialogButtonClickEventArgs {
            cancel: Cell::new(false),
            deferrals: Rc::clone(&deferrals),
            generation,
        });
        if let Some(handler) = handler {
            handler(app, &args);
        }
        deferrals.continue_with(app, move |app| {
            if app.contains(self) && !args.cancel() {
                self.hide_internal(app, result);
            }
        });
    }

    /// `ExecuteCloseAction`: Escape and the system back request press the close button when
    /// there is one, and otherwise close with `None`.
    fn execute_close_action(self: Handle<Self>, app: &mut App) {
        if app.get(self).close_button_text.is_empty() {
            self.hide_internal(app, ContentDialogResult::None);
        } else {
            self.command_button_clicked(app, ContentDialogButton::Close);
        }
    }

    /// Enter presses the default button when it is enabled; false when there is none to press.
    fn invoke_default_button(self: Handle<Self>, app: &mut App) -> bool {
        let dialog = app.get(self);
        let enabled = match dialog.default_button {
            ContentDialogButton::None => return false,
            ContentDialogButton::Primary => dialog.is_primary_button_enabled,
            ContentDialogButton::Secondary => dialog.is_secondary_button_enabled,
            ContentDialogButton::Close => true,
        };
        if enabled {
            let button = dialog.default_button;
            self.command_button_clicked(app, button);
        }
        enabled
    }

    /// `HideInternal`: raises the cancelable Closing event, unless it is skipped, and once its
    /// deferrals are complete closes the route with `result`.
    fn hide_internal(self: Handle<Self>, app: &mut App, result: ContentDialogResult) {
        if !self.is_accepting_commands(app) {
            return;
        }
        let deferrals = Rc::clone(&app.get(self).closing_deferrals);
        let generation = deferrals
            .prepare()
            .expect("a close in progress stops new ones");
        let dialog = app.get_mut(self);
        let disposed = dialog.disposed;
        let showing = dialog.showing.as_mut().expect("a showing dialog");
        showing.hide_in_progress = true;
        let fire_closing = !showing.skip_closing_event && !disposed;
        let args = fire_closing.then(|| {
            Rc::new(ContentDialogClosingEventArgs {
                result,
                cancel: Cell::new(false),
                deferrals: Rc::clone(&deferrals),
                generation,
            })
        });
        if let Some(args) = &args
            && let Some(closing) = app.get(self).closing.clone()
        {
            closing(app, args);
        }
        deferrals.continue_with(app, move |app| {
            if app.contains(self) {
                let canceled = args.is_some_and(|args| args.cancel());
                self.hide_after_deferral(app, canceled, result);
            }
        });
    }

    /// `HideAfterDeferralWorker`: a canceled close keeps the dialog open while its route is still
    /// shown; otherwise the window's open-dialog slot is freed and the route leaves with
    /// `result`.
    fn hide_after_deferral(
        self: Handle<Self>,
        app: &mut App,
        canceled: bool,
        result: ContentDialogResult,
    ) {
        let disposed = app.get(self).disposed;
        let Some(showing) = app.get(self).showing.as_ref() else {
            return;
        };
        let (route, navigator) = (showing.route, showing.navigator);
        let keep_open = canceled && !disposed && route.is_active(app);
        let showing = app
            .get_mut(self)
            .showing
            .as_mut()
            .expect("a showing dialog");
        if keep_open {
            showing.hide_in_progress = false;
            return;
        }
        showing.result = result;
        self.leave_metadata(app, navigator);
        if route.is_current(app) {
            navigator.pop(app, None);
        } else if route.is_active(app) {
            navigator.remove_route(app, route, None);
        }
    }

    /// Whether a showing can still be closed: it is shown and no close is under way.
    fn is_accepting_commands(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .showing
            .as_ref()
            .is_some_and(|showing| !showing.hide_in_progress)
    }

    /// `OnPopupOpened`: from now on a close raises Closing.
    fn finish_opening(self: Handle<Self>, app: &mut App, route: AnyRoute) {
        let Some(showing) = self.showing_of(app, route) else {
            return;
        };
        showing.skip_closing_event = false;
        if let Some(opened) = app.get(self).opened.clone() {
            opened.call(app);
        }
    }

    /// The route left the navigator. A route removed by something other than the dialog closes
    /// it with `None`, without Closing, as when an app closes the dialog's popup itself.
    fn route_popped(self: Handle<Self>, app: &mut App, route: AnyRoute) {
        let Some(showing) = self.showing_of(app, route) else {
            return;
        };
        if showing.hide_in_progress {
            return;
        }
        showing.hide_in_progress = true;
        showing.skip_closing_event = true;
        showing.result = ContentDialogResult::None;
        let navigator = showing.navigator;
        self.leave_metadata(app, navigator);
    }

    /// `OnFinishedClosing`: the route's page is gone, so the showing ends: Closed is raised and
    /// the result delivered.
    fn finish_closing(self: Handle<Self>, app: &mut App, route: AnyRoute) {
        if self.showing_of(app, route).is_none() {
            return;
        }
        let showing = app.get_mut(self).showing.take().expect("a showing dialog");
        self.leave_metadata(app, showing.navigator);
        let result = showing.result;
        if app.get(self).disposed {
            showing.completion.complete(app, result);
            self.notifier_dispose(app);
            app.destroy(self);
            return;
        }
        if let Some(closed) = app.get(self).closed.clone() {
            closed(app, ContentDialogClosedEventArgs { result });
        }
        let dialog = app.get_mut(self);
        dialog.button_click_deferrals.disconnect();
        dialog.button_click_deferrals = DeferralManager::new();
        showing.completion.complete(app, result);
    }

    /// The current showing, when it is the one shown in `route`.
    fn showing_of(self: Handle<Self>, app: &mut App, route: AnyRoute) -> Option<&mut Showing> {
        if !app.contains(self) {
            return None;
        }
        app.get_mut(self)
            .showing
            .as_mut()
            .filter(|showing| showing.route == route)
    }

    /// Frees the window's open-dialog slot, when this dialog holds it.
    fn leave_metadata(self: Handle<Self>, app: &mut App, navigator: Handle<NavigatorState>) {
        let metadata = app.singleton::<ContentDialogMetadata>();
        let open = &mut app.get_mut(metadata).open;
        if open.get(&navigator.id()) == Some(&self) {
            open.remove(&navigator.id());
        }
    }

    fn notifier_dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).notifier.dispose();
    }
}

/// `ButtonsVisibilityStates`: which command buttons have text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names, reason = "the template's state names")]
enum ButtonsVisibilityState {
    AllVisible,
    NoneVisible,
    PrimaryVisible,
    SecondaryVisible,
    CloseVisible,
    PrimaryAndSecondaryVisible,
    PrimaryAndCloseVisible,
    SecondaryAndCloseVisible,
}

impl ButtonsVisibilityState {
    /// The state `ChangeVisualState` goes to for the button texts that are set.
    fn of(has_primary: bool, has_secondary: bool, has_close: bool) -> ButtonsVisibilityState {
        use ButtonsVisibilityState::*;
        match (has_primary, has_secondary, has_close) {
            (true, true, true) => AllVisible,
            (false, false, false) => NoneVisible,
            (true, false, false) => PrimaryVisible,
            (false, true, false) => SecondaryVisible,
            (false, false, true) => CloseVisible,
            (true, true, false) => PrimaryAndSecondaryVisible,
            (true, false, true) => PrimaryAndCloseVisible,
            (false, true, true) => SecondaryAndCloseVisible,
        }
    }

    /// The command space's columns and the column of each visible button: the template's,
    /// with this state's setters applied. `None` for `NoneVisible`, which collapses the command
    /// space.
    fn layout(self, r: &ThemeResources) -> Option<CommandLayout> {
        use ButtonsVisibilityState::*;
        let spacing = GridLength::pixel(r.content_dialog_button_spacing);
        // `PrimaryColumn`, `FirstSpacer`, `SecondaryColumn`, `SecondSpacer`, `CloseColumn`.
        let mut columns = [
            GridLength::STAR,
            GridLength::pixel(0.0),
            GridLength::pixel(0.0),
            spacing,
            GridLength::STAR,
        ];
        // `PrimaryButton` and `SecondaryButton` sit in the first column, `CloseButton` in the
        // last, unless a setter moves them.
        let (primary, secondary, close) = match self {
            NoneVisible => return None,
            AllVisible => {
                columns[1] = spacing;
                columns[2] = GridLength::STAR;
                (Some(0), Some(2), Some(4))
            }
            PrimaryVisible => (Some(4), None, None),
            SecondaryVisible => (None, Some(4), None),
            CloseVisible => (None, None, Some(4)),
            PrimaryAndSecondaryVisible => (Some(0), Some(4), None),
            PrimaryAndCloseVisible => (Some(0), None, Some(4)),
            SecondaryAndCloseVisible => (None, Some(0), Some(4)),
        };
        Some(CommandLayout {
            columns,
            primary,
            secondary,
            close,
        })
    }
}

/// The command space's five columns, and the column each visible command button takes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CommandLayout {
    columns: [GridLength; 5],
    primary: Option<usize>,
    secondary: Option<usize>,
    close: Option<usize>,
}

/// `DefaultButtonStates`: the button drawn in the accent style. While the focus is on a command
/// button, only the default button itself keeps the accent, when it is the one focused.
fn accented_button(
    default: ContentDialogButton,
    focused: Option<ContentDialogButton>,
) -> ContentDialogButton {
    match focused {
        Some(focused) if focused != default => ContentDialogButton::None,
        _ => default,
    }
}

/// `ContentDialogOpenCloseThemeTransition`'s `s_OpenScaleDuration`: opening shrinks the dialog
/// from its scaled factor over this long, which is also how long the route takes to open.
const OPEN_SCALE_DURATION: Duration = Duration::from_millis(250);

/// `s_CloseScaleDuration`: closing grows the dialog back to its scaled factor over this long,
/// which is also how long the route takes to close.
const CLOSE_SCALE_DURATION: Duration = Duration::from_millis(167);

/// `s_OpacityChangeDuration`: the dialog and its smoke layer fade in, and out, over this long.
const OPACITY_CHANGE_DURATION: Duration = Duration::from_millis(83);

/// The transition's `scaledFactor`: the dialog's scale as it starts to open and ends closing.
const SCALED_FACTOR: f64 = 1.05;

/// `ContentDialogOpenCloseThemeTransition`, which a dialog shown in a popup plays as it opens
/// and closes: the opacity of the dialog and its smoke layer, and the dialog's scale, at `value`
/// of its route's animation going in the direction `status` says. The opacity changes linearly,
/// the scale along cubic-bezier (0, 0, 0, 1).
fn open_close_transition(status: AnimationStatus, value: f64) -> (f64, f64) {
    let easing = Cubic::new(0.0, 0.0, 0.0, 1.0);
    let fade = OPACITY_CHANGE_DURATION.as_secs_f64();
    let progress = |elapsed: f64, duration: f64| (elapsed / duration).clamp(0.0, 1.0);
    if matches!(
        status,
        AnimationStatus::Reverse | AnimationStatus::Dismissed
    ) {
        let close = CLOSE_SCALE_DURATION.as_secs_f64();
        let elapsed = (1.0 - value) * close;
        let opacity = 1.0 - progress(elapsed, fade);
        let scale = 1.0 + (SCALED_FACTOR - 1.0) * easing.transform(progress(elapsed, close));
        (opacity, scale)
    } else {
        let open = OPEN_SCALE_DURATION.as_secs_f64();
        let elapsed = value * open;
        let opacity = progress(elapsed, fade);
        let scale =
            SCALED_FACTOR - (SCALED_FACTOR - 1.0) * easing.transform(progress(elapsed, open));
        (opacity, scale)
    }
}

/// The route a dialog is shown in: a `RawDialogRoute` whose page is the presenter and whose
/// barrier is the smoke layer, opening and closing in the time `ContentDialogOpenCloseThemeTransition`
/// takes, and keeping the keyboard focus inside the dialog.
struct ContentDialogRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    raw_dialog_route: RawDialogRouteData,
}

impl ContentDialogRoute {
    /// A route showing `dialog` over a `smoke` layer.
    fn new(app: &mut App, dialog: Handle<ContentDialog>, smoke: Color) -> Handle<Self> {
        let page_builder: RoutePageBuilder = Rc::new(move |_, _, animation, _| {
            ContentDialogPresenter { dialog, animation }.into_widget()
        });
        let mut raw_dialog_route = RawDialogRouteData::new(page_builder);
        raw_dialog_route.barrier_dismissible = false;
        raw_dialog_route.barrier_color = Some(smoke);
        raw_dialog_route.transition_duration = OPEN_SCALE_DURATION;
        raw_dialog_route.transition_builder = Some(Rc::new(|_, _, _, _, child| child));
        let mut modal_route = ModalRouteData::new(app);
        modal_route.traversal_edge_behavior = Some(TraversalEdgeBehavior::ClosedLoop);
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        app.create(ContentDialogRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            raw_dialog_route,
        })
    }
}

impl Route for ContentDialogRoute {
    inset_widgets::modal_route_overrides!();
}

impl OverlayRoute for ContentDialogRoute {
    inset_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for ContentDialogRoute {
    inset_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for ContentDialogRoute {
    inset_widgets::raw_dialog_route_transition_route_overrides!();

    fn reverse_transition_duration(self: Handle<Self>, _app: &App) -> Duration {
        CLOSE_SCALE_DURATION
    }
}

impl LocalHistoryRoute for ContentDialogRoute {
    inset_widgets::local_history_route_accessors!();
}

impl ModalRoute for ContentDialogRoute {
    inset_widgets::raw_dialog_route_modal_route_overrides!();
}

impl PopupRoute for ContentDialogRoute {}

impl RawDialogRouteLeaf for ContentDialogRoute {
    inset_widgets::raw_dialog_route_accessors!();

    /// `SmokeLayerBackground`: blocks input to the page beneath and fades with the dialog.
    fn build_modal_barrier(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = TransitionRoute::animation(self, app).expect("an installed route");
        let smoke = self
            .raw_dialog_route_data(app)
            .barrier_color
            .expect("the smoke fill");
        ListenableBuilder::new(Rc::new(animation), move |app, _, _| {
            let (opacity, _) = open_close_transition(animation.status(app), animation.value(app));
            ModalBarrier::new()
                .color(smoke.with_values(Some(smoke.a * opacity), None, None, None, None))
                .dismissible(false)
                .into_widget()
        })
        .into_widget()
    }
}

/// The route's page: `LayoutRoot` of the template, the dialog centered in the window.
#[derive(Debug)]
struct ContentDialogPresenter {
    dialog: Handle<ContentDialog>,

    /// The route's animation, which drives the showing and hiding storyboards.
    animation: AnyAnimation<f64>,
}

/// The focus nodes the template's behavior needs, and the showing the presenter belongs to.
struct ContentDialogPresenterState {
    state: StateData<ContentDialogPresenter>,

    /// The route of the showing this presenter was built for.
    route: Option<AnyRoute>,

    /// Encloses `Content`, whose first focusable element takes the initial focus.
    content_focus: Option<Handle<FocusNode>>,

    /// The primary, secondary and close buttons' focus, which decides the accented button.
    button_focus: Vec<(ContentDialogButton, Handle<FocusNode>)>,

    /// Rebuilds the presenter for property and focus changes.
    rebuild: Option<Listener>,
}

impl StatefulWidget for ContentDialogPresenter {
    type State = ContentDialogPresenterState;

    fn create_state(&self) -> Self::State {
        ContentDialogPresenterState {
            state: StateData::new(),
            route: None,
            content_focus: None,
            button_focus: Vec::new(),
            rebuild: None,
        }
    }
}

impl State for ContentDialogPresenterState {
    type Widget = ContentDialogPresenter;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let dialog = self.widget(app).dialog;
        let route = app
            .get(dialog)
            .showing
            .as_ref()
            .map(|showing| showing.route);
        let rebuild = Listener::new(move |app| {
            if app.contains(self) && self.mounted(app) {
                self.set_state(app, |_| {});
            }
        });
        dialog.add_listener(app, rebuild.clone());
        let content_focus = FocusNode::new(app);
        let button_focus: Vec<_> = [
            ContentDialogButton::Primary,
            ContentDialogButton::Secondary,
            ContentDialogButton::Close,
        ]
        .into_iter()
        .map(|button| {
            let node = FocusNode::new(app);
            node.as_node().add_listener(app, rebuild.clone());
            (button, node)
        })
        .collect();
        let state = app.get_mut(self);
        state.route = route;
        state.content_focus = Some(content_focus);
        state.button_focus = button_focus;
        state.rebuild = Some(rebuild);
        self.open_after_first_frame(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let dialog = self.widget(app).dialog;
        let state = app.get_mut(self);
        let rebuild = state.rebuild.take().expect("an initialized presenter");
        let nodes: Vec<_> = state
            .button_focus
            .drain(..)
            .map(|(_, node)| node)
            .chain(state.content_focus.take())
            .collect();
        let route = state.route;
        if app.contains(dialog) {
            dialog.remove_listener(app, &rebuild);
        }
        for node in nodes {
            node.as_node().remove_listener(app, &rebuild);
            node.as_node().dispose(app);
            app.destroy(node);
        }
        if let Some(route) = route {
            after_next_frame(
                app,
                // Disposed, the presenter is in no view any more.
                None,
                FrameCallback::new(move |app, _| dialog.finish_closing(app, route)),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let dialog = self.widget(app).dialog;
        let Some(showing) = app.get(dialog).showing.as_ref() else {
            return SizedBox::shrink().into_widget();
        };
        let (resources, direction) = (showing.resources.clone(), showing.direction);
        let background = self.background_element(app, &resources);
        let animation = self.widget(app).animation;
        let animated = ListenableBuilder::new(Rc::new(animation), move |app, _, child| {
            let status = animation.status(app);
            let (opacity, scale) = open_close_transition(status, animation.value(app));
            // The template makes the dialog unreachable while its closing transition plays;
            // the scale is `BackgroundElement`'s `ScaleTransform`.
            IgnorePointer::new()
                .ignoring(status == AnimationStatus::Reverse)
                .child(
                    Opacity::new(opacity).child(
                        Transform::scale(Some(scale), None, None)
                            .child(child.cloned().expect("the dialog's background element")),
                    ),
                )
                .into_widget()
        })
        .child(background);
        // `Container` and its `LayoutRoot`: the dialog centered over the window, where the route's
        // barrier is `SmokeLayerBackground`.
        let layout_root = Focus::new(Center::new().child(animated))
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| {
                layout_root_key(app, dialog, event)
            }));
        let back_request = PopScope::new(layout_root)
            .can_pop(false)
            .on_pop_invoked_with_result(Rc::new(move |app, did_pop, _| {
                if !did_pop {
                    dialog.execute_close_action(app);
                }
            }));
        ThemeScope::with_resources(resources, Directionality::new(direction, back_request))
            .into_widget()
    }
}

/// `ProcessLayoutRootKey`: Escape runs the close action; Enter presses the default button.
fn layout_root_key(
    app: &mut App,
    dialog: Handle<ContentDialog>,
    event: &KeyEvent,
) -> KeyEventResult {
    if !matches!(event, KeyEvent::Down(_)) || !app.contains(dialog) {
        return KeyEventResult::Ignored;
    }
    match event.logical_key() {
        LogicalKeyboardKey::ESCAPE => {
            dialog.execute_close_action(app);
            KeyEventResult::Handled
        }
        LogicalKeyboardKey::ENTER | LogicalKeyboardKey::NUMPAD_ENTER
            if dialog.invoke_default_button(app) =>
        {
            KeyEventResult::Handled
        }
        _ => KeyEventResult::Ignored,
    }
}

impl ContentDialogPresenterState {
    /// `BackgroundElement`: the bordered, rounded, shadowed dialog between its minimum and
    /// maximum sizes, as wide as its content asks.
    fn background_element(
        self: Handle<Self>,
        app: &mut App,
        resources: &ThemeResources,
    ) -> WidgetRef {
        let dialog = app.get(self.widget(app).dialog);
        let full_size = dialog.full_size_desired;
        let r = &resources;
        let dialog_space = self.dialog_space(app, resources);
        let border = ControlBorder::new(
            Brush::Solid(r.content_dialog_background),
            Brush::Solid(r.content_dialog_border_brush),
        )
        .border_thickness_ltrb(r.content_dialog_border_width)
        .corner_radius(r.overlay_corner_radius[0])
        .background_sizing(BackgroundSizing::InnerBorderEdge)
        .child(
            // `DialogSpace` rounds its own corners with `OverlayCornerRadius`.
            ClipRRect::new()
                .border_radius(BorderRadius::circular(r.overlay_corner_radius[0]).into())
                .child(dialog_space),
        );
        let shadowed = CustomPaint::new()
            .painter(PopupShadow::for_dialog(resources, resources.theme))
            .child(border);
        let sized = IntrinsicWidth::new().child(shadowed);
        // FullDialogSizing stretches the dialog to the window's height, up to its maximum.
        let sized = if full_size {
            SizedBox::new()
                .height(f64::INFINITY)
                .child(sized)
                .into_widget()
        } else {
            sized.into_widget()
        };
        ConstrainedBox::new(
            BoxConstraints::new()
                .min_width(r.content_dialog_min_width)
                .max_width(r.content_dialog_max_width)
                .min_height(r.content_dialog_min_height)
                .max_height(r.content_dialog_max_height),
        )
        .child(sized)
        .into_widget()
    }

    /// `DialogSpace`: the content area over the command space. The content area takes what the
    /// buttons leave, clipped, as its scroll viewer with scrolling disabled does; in full size it
    /// takes all of it.
    fn dialog_space(self: Handle<Self>, app: &mut App, resources: &ThemeResources) -> WidgetRef {
        let full_size = app.get(self.widget(app).dialog).full_size_desired;
        let content_area = ClipRect::new().child(self.content_area(app, resources));
        let fit = if full_size {
            FlexFit::Tight
        } else {
            FlexFit::Loose
        };
        let mut children = vec![Flexible::new(content_area).fit(fit).into_widget()];
        if let Some(command_space) = self.command_space(app, resources) {
            children.push(command_space);
        }
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children(children)
            .into_widget()
    }

    /// The grid inside `ContentScrollViewer`: `Title` above `Content`, on the top overlay, over
    /// the separator.
    fn content_area(self: Handle<Self>, app: &mut App, resources: &ThemeResources) -> WidgetRef {
        let dialog = app.get(self.widget(app).dialog);
        let r = &resources;
        let foreground = r.content_dialog_foreground;
        let mut children = Vec::new();
        if let Some(title) = dialog.title.clone() {
            // `Title`: left aligned, where `Content` stretches.
            children.push(
                Align::new()
                    .alignment(AlignmentDirectional::TOP_START.into())
                    .child(Margin::new(
                        r.content_dialog_title_margin,
                        DefaultTextStyle::new(
                            control_text_style(resources, 20.0, FontWeight::W600, foreground),
                            title,
                        )
                        .max_lines(2)
                        .soft_wrap(true),
                    ))
                    .into_widget(),
            );
        }
        if let Some(content) = dialog.content.clone() {
            let focus = app
                .get(self)
                .content_focus
                .expect("an initialized presenter");
            children.push(
                Focus::new(DefaultTextStyle::new(
                    control_text_style(
                        resources,
                        r.control_content_theme_font_size,
                        FontWeight::NORMAL,
                        foreground,
                    ),
                    content,
                ))
                .focus_node(focus.as_node())
                .can_request_focus(false)
                .skip_traversal(true)
                .into_widget(),
            );
        }
        ControlBorder::new(
            Brush::Solid(r.content_dialog_top_overlay),
            Brush::Solid(r.content_dialog_separator_border_brush),
        )
        .border_thickness_ltrb(r.content_dialog_separator_thickness)
        .padding(r.content_dialog_padding)
        .child(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children(children),
        )
        .into_widget()
    }

    /// `CommandSpace`: the buttons with text, in the columns their visibility state gives them;
    /// nothing when no button has text.
    fn command_space(
        self: Handle<Self>,
        app: &mut App,
        resources: &ThemeResources,
    ) -> Option<WidgetRef> {
        let dialog_handle = self.widget(app).dialog;
        let dialog = app.get(dialog_handle);
        let layout = ButtonsVisibilityState::of(
            !dialog.primary_button_text.is_empty(),
            !dialog.secondary_button_text.is_empty(),
            !dialog.close_button_text.is_empty(),
        )
        .layout(resources)?;
        let accented = accented_button(dialog.default_button, self.focused_button(app));
        let dialog = app.get(dialog_handle);
        // `PrimaryButton`, `SecondaryButton` and `CloseButton`.
        let buttons = [
            (
                ContentDialogButton::Primary,
                layout.primary,
                dialog.primary_button_text.clone(),
                dialog.is_primary_button_enabled,
            ),
            (
                ContentDialogButton::Secondary,
                layout.secondary,
                dialog.secondary_button_text.clone(),
                dialog.is_secondary_button_enabled,
            ),
            (
                ContentDialogButton::Close,
                layout.close,
                dialog.close_button_text.clone(),
                true,
            ),
        ];
        let cells: Vec<WidgetRef> = buttons
            .into_iter()
            .filter_map(|(button, column, text, enabled)| {
                let column = column?;
                let style = if button == accented {
                    ButtonStyle::Accent
                } else {
                    ButtonStyle::Default
                };
                let node = self.focus_of(app, button);
                let click =
                    Listener::new(move |app| dialog_handle.command_button_clicked(app, button));
                let button = Button::text(text, click)
                    .style(style)
                    .is_enabled(enabled)
                    .focus_node(node.as_node());
                Some(GridCell::new(button).column(column).into_widget())
            })
            .collect();
        let background = resources.content_dialog_background;
        Some(
            Grid::new()
                .column_definitions(layout.columns.map(ColumnDefinition::new))
                .padding(resources.content_dialog_padding)
                .background(Brush::Solid(background))
                .children(cells)
                .into_widget(),
        )
    }

    /// The command button that has the focus, if one has.
    fn focused_button(self: Handle<Self>, app: &mut App) -> Option<ContentDialogButton> {
        let buttons = app.get(self).button_focus.clone();
        buttons
            .into_iter()
            .find(|(_, node)| node.as_node().has_focus(app))
            .map(|(button, _)| button)
    }

    fn focus_of(self: Handle<Self>, app: &App, button: ContentDialogButton) -> Handle<FocusNode> {
        app.get(self)
            .button_focus
            .iter()
            .find(|(candidate, _)| *candidate == button)
            .map(|(_, node)| *node)
            .expect("a command button")
    }

    /// `OnLayoutRootLoaded` and `OnPopupOpened`: once the dialog is laid out, its initial focus is
    /// set and it has opened.
    fn open_after_first_frame(self: Handle<Self>, app: &mut App) {
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || !self.mounted(app) {
                    return;
                }
                let dialog = self.widget(app).dialog;
                let Some(route) = app.get(self).route else {
                    return;
                };
                self.set_initial_focus(app);
                dialog.finish_opening(app, route);
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    /// `SetInitialFocusElement`: the content's first focusable element, else the default button,
    /// else the first command button that can take the focus.
    fn set_initial_focus(self: Handle<Self>, app: &mut App) {
        let content = app
            .get(self)
            .content_focus
            .expect("an initialized presenter");
        let first_in_content = content
            .as_node()
            .traversal_descendants(app)
            .into_iter()
            .find(|node| node.can_request_focus(app));
        if let Some(node) = first_in_content {
            node.request_focus(app, None);
            return;
        }
        let default = app.get(self.widget(app).dialog).default_button;
        let candidates = std::iter::once(default).chain([
            ContentDialogButton::Primary,
            ContentDialogButton::Secondary,
            ContentDialogButton::Close,
        ]);
        for button in candidates {
            if button == ContentDialogButton::None {
                continue;
            }
            let node = self.focus_of(app, button).as_node();
            if node.context(app).is_some() && node.can_request_focus(app) {
                node.request_focus(app, None);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_set_of_button_texts_picks_its_columns() {
        let r = ThemeResources::new(Theme::Light, AccentPalette::default());
        let layout = |primary, secondary, close| {
            ButtonsVisibilityState::of(primary, secondary, close).layout(&r)
        };
        let all = layout(true, true, true).unwrap();
        assert_eq!(
            (all.primary, all.secondary, all.close),
            (Some(0), Some(2), Some(4))
        );
        assert_eq!(
            all.columns[1],
            GridLength::pixel(r.content_dialog_button_spacing)
        );
        let secondary_and_close = layout(false, true, true).unwrap();
        assert_eq!(
            (secondary_and_close.secondary, secondary_and_close.close),
            (Some(0), Some(4))
        );
        let primary_alone = layout(true, false, false).unwrap();
        assert_eq!(primary_alone.primary, Some(4));
        assert_eq!(
            ButtonsVisibilityState::of(false, false, false),
            ButtonsVisibilityState::NoneVisible
        );
        assert_eq!(layout(false, false, false), None);
    }

    #[test]
    fn focusing_another_command_button_takes_the_accent_from_the_default() {
        use ContentDialogButton::{Close, Primary};
        assert_eq!(accented_button(Primary, None), Primary);
        assert_eq!(accented_button(Primary, Some(Primary)), Primary);
        assert_eq!(
            accented_button(Primary, Some(Close)),
            ContentDialogButton::None
        );
        assert_eq!(
            accented_button(ContentDialogButton::None, Some(Close)),
            ContentDialogButton::None
        );
    }

    #[test]
    fn opening_and_closing_fade_in_83_ms_and_scale_over_their_own_durations() {
        let fraction = |part: Duration, whole: Duration| part.as_secs_f64() / whole.as_secs_f64();
        let (opacity, scale) = open_close_transition(AnimationStatus::Forward, 0.0);
        assert_eq!((opacity, scale), (0.0, SCALED_FACTOR));
        let faded_in = fraction(OPACITY_CHANGE_DURATION, OPEN_SCALE_DURATION);
        let (opacity, scale) = open_close_transition(AnimationStatus::Forward, faded_in);
        assert!((opacity - 1.0).abs() < 1e-9 && scale > 1.0);
        assert_eq!(
            open_close_transition(AnimationStatus::Completed, 1.0),
            (1.0, 1.0)
        );
        let faded_out = 1.0 - fraction(OPACITY_CHANGE_DURATION, CLOSE_SCALE_DURATION);
        let (opacity, scale) = open_close_transition(AnimationStatus::Reverse, faded_out);
        assert!(opacity.abs() < 1e-9 && scale > 1.0 && scale < SCALED_FACTOR);
        let (_, scale) = open_close_transition(AnimationStatus::Dismissed, 0.0);
        assert!((scale - SCALED_FACTOR).abs() < 1e-9);
    }
}
