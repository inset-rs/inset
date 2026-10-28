//! Flutter counterpart: `dart:ui` `hooks.dart`.
//!
//! Dart's hooks are isolate-global functions the engine calls. Here they are
//! methods on the embedder client so a host crate need not name `App`.

use std::time::Duration;

use crate::{
    AppExitReply, AppExitResponse, AppLifecycleState, ClipboardEvent, DropData, EditCommand, Frame,
    KeyData, MenuId, OpenRequest, PointerDataPacket, TextEditingValue, TextInputAction,
    TrayIconEvent, TrayIconId, ViewFocusEvent, ViewId,
};

/// Complete frames and typed view lifecycle notifications.
///
/// The platform updates its view registry before sending a lifecycle method,
/// so the client can immediately query the affected view.
pub trait EmbedderClient {
    /// A screen refreshed at `frame.elapsed`: `views` are the views on it, in sight, that asked
    /// for a frame ([`Platform::request_frame`](crate::Platform::request_frame)) before the
    /// refresh began; one that asks during it is in the next.
    ///
    /// The host never calls it from inside another call into the client.
    fn frame(&mut self, views: &[ViewId], frame: Frame);

    /// A view arrived after launch: opened by the app, or given by the system. The views that
    /// exist at launch are in `Platform::views` when the app starts, and none of them is
    /// reported here.
    fn view_added(&mut self, id: ViewId);

    /// Flutter's window metrics event: something of `id`'s metrics changed, the screen it is on
    /// included.
    fn view_metrics_changed(&mut self, id: ViewId);

    /// A view is gone; the app keeps running.
    fn view_removed(&mut self, id: ViewId);

    /// Flutter `FlutterEngineNotifyDisplayUpdate`: a screen was added, removed or changed, and
    /// `Platform::screens` already answers the new list. A view that is on another screen now
    /// is reported through `view_metrics_changed`. The default drops it.
    fn screens_changed(&mut self) {}

    /// Flutter `PlatformDispatcher.onViewFocusChange`.
    fn view_focus_changed(&mut self, event: ViewFocusEvent) {
        let _ = event;
    }

    /// Flutter `PlatformDispatcher.onPointerDataPacket`.
    fn pointer_data_packet(&mut self, packet: PointerDataPacket);

    /// Files dragged from outside the application entered, moved over, were dropped on or
    /// left a view. No dart:ui hook; the default drops it.
    fn drop_data(&mut self, data: DropData) {
        let _ = data;
    }

    /// The user copied, cut or pasted in `view` through the platform's own command: the
    /// browser's, or a desktop's Edit menu, which a key reaches only once the framework has left
    /// it unhandled. No dart:ui hook; the default drops it.
    fn clipboard_event(&mut self, view: ViewId, event: ClipboardEvent) {
        let _ = (view, event);
    }

    /// The user chose an edit command for `view` from the system's own Edit menu, which a key
    /// reaches only once the framework has left it unhandled: undo, redo, select all or delete.
    /// No dart:ui hook; the default drops it.
    fn edit_command(&mut self, view: ViewId, command: EditCommand) {
        let _ = (view, command);
    }

    /// The user chose the item `id` of the app's menu bar (`MenuBar::set_menus`). Flutter's
    /// `Menu.selectedCallback` on the `flutter/menu` channel. The default drops it.
    fn menu_selected(&mut self, id: MenuId) {
        let _ = id;
    }

    /// The user chose the item `id` of the Dock icon's menu (`DockMenu::set_menu`). No dart:ui
    /// hook; the default drops it.
    fn dock_menu_selected(&mut self, id: MenuId) {
        let _ = id;
    }

    /// Something happened to the tray icon `icon`, as its system reports it. No dart:ui hook; the
    /// default drops it.
    fn tray_icon_event(&mut self, icon: TrayIconId, event: TrayIconEvent) {
        let _ = (icon, event);
    }

    /// The system asks the app to open files and links: from the file manager, another app or
    /// the command line, at launch or while it runs. A launch's request comes once the client
    /// exists. No dart:ui hook: Flutter's embedders hand these to their plugins. The default drops
    /// it.
    fn open_requested(&mut self, request: OpenRequest) {
        let _ = request;
    }

    /// Flutter `PlatformDispatcher.onKeyData`.
    ///
    /// Returns whether the framework handled the event; a host that shares the
    /// keyboard with other native components should not propagate a handled one.
    fn key_data(&mut self, data: KeyData) -> bool;

    /// The user asked to go back: Android's back button or back gesture, and whatever a
    /// later host maps onto the same idea.
    ///
    /// Answers whether the application went back. `false` means there was nothing left to
    /// go back to, and the host should then do what the platform does without the
    /// application, which on Android is to close the activity.
    ///
    /// Flutter sends this as `popRoute` on `SystemChannels.navigation`, and has to ask the
    /// framework in advance whether it handles back at all, because the answer cannot
    /// cross its channel in time. This call answers on the spot instead.
    fn pop_route(&mut self) -> bool {
        false
    }

    /// The user asked to close `view`: its window's close button, or the system's Close
    /// command. The host closes nothing itself; the application closes the window, through
    /// [`HostWindow::close`](crate::HostWindow::close), or keeps it open, such as until its user
    /// has chosen whether to save. Nothing waits for an answer.
    ///
    /// Flutter's engine reports this to the window's controller, whose delegate's
    /// `onWindowCloseRequested` closes it by default. The default here keeps the view open.
    fn view_close_requested(&mut self, view: ViewId) {
        let _ = view;
    }

    /// The window of `view` was minimized or restored, became occluded or shown, or moved, and
    /// its [`HostWindow`](crate::HostWindow) already answers how it is now: as Flutter's window
    /// controllers notify their listeners at the platform's word. A resize is reported as a
    /// metrics change alone (`view_metrics_changed`). The default drops it.
    fn window_changed(&mut self, view: ViewId) {
        let _ = view;
    }

    /// A request for a view ([`crate::Platform::request_view`]) got no view: the host could not
    /// make its window. The default drops it.
    fn view_request_failed(&mut self, request: crate::RequestId, error: crate::ViewError) {
        let _ = (request, error);
    }

    /// The system asked the application to quit, and the application may refuse: the user
    /// chose Quit, or is logging out. Every request comes here, a second one while the first is
    /// still being decided included, so that the application decides each; `reply` takes its
    /// answer, and the host quits on [`AppExitResponse::Exit`].
    ///
    /// The answer may take as long as the application needs, such as asking its user whether
    /// to save first, and is given inside a call into this client: this one, or a later one.
    ///
    /// Flutter sends this as `System.requestAppExit` on `SystemChannels.platform` and replies
    /// once `ServicesBinding.handleRequestAppExit`'s `Future` has completed. The default
    /// answers `Exit` at once, as `ServicesBinding`'s own answer does.
    fn request_app_exit(&mut self, reply: AppExitReply) {
        reply.answer(AppExitResponse::Exit);
    }

    /// The application entered `state`: its views were shown, hidden, focused or unfocused, or
    /// the system put it in the background or brought it back. The host sends the state it is
    /// in, and the framework fills in the states between, as Flutter's `ServicesBinding` does.
    ///
    /// Flutter sends this as the state's name on `SystemChannels.lifecycle`. The default drops
    /// it.
    fn app_lifecycle_state_changed(&mut self, state: AppLifecycleState) {
        let _ = state;
    }

    /// Flutter `PlatformDispatcher.onPlatformBrightnessChanged`: the host's light or dark
    /// preference changed, and `Platform::platform_brightness` already answers the new one.
    fn platform_brightness_changed(&mut self);

    /// Flutter `PlatformDispatcher.onLocaleChanged`: the host's locale list changed, and
    /// `Platform::locales` already answers the new one.
    fn locales_changed(&mut self);

    /// Flutter `PlatformDispatcher.onSystemFontsChanged`: the host registered faces the
    /// framework did not have when text last laid out.
    fn system_fonts_changed(&mut self) {}

    /// Another app changed what is on the clipboard, or the host cleared a copy of this app's that
    /// expired, as far as the host can tell: at the moments its system says so, or lets it check.
    /// The app's own writes are not reported, as it made them, but where the system tells them
    /// apart from no other change, as a browser does not. No dart:ui hook; the default drops it.
    fn clipboard_changed(&mut self) {}

    /// The wake asked for through [`Dispatcher::wake_at`](crate::Dispatcher::wake_at) or
    /// [`Dispatcher::wake_now`](crate::Dispatcher::wake_now).
    ///
    /// `elapsed` is the platform clock, the same one `Frame::elapsed` reports. Dart's
    /// event loop runs due `Timer`s itself; here the client advances its own clock.
    fn wake(&mut self, elapsed: Duration);

    /// Flutter `TextInputClient.updateEditingValue`. The default drops it.
    fn text_input_editing_value(&mut self, view: ViewId, value: TextEditingValue) {
        let _ = (view, value);
    }

    /// Flutter `TextInputClient.performAction`. The default drops it.
    fn text_input_action(&mut self, view: ViewId, action: TextInputAction) {
        let _ = (view, action);
    }

    /// Flutter `TextInputClient.connectionClosed`. The default drops it.
    fn text_input_closed(&mut self, view: ViewId) {
        let _ = view;
    }

    /// Flutter `SystemContextMenuClient.handleSystemHide`. The default drops it.
    fn system_context_menu_hidden(&mut self) {}

    /// Flutter `SystemContextMenuClient.handleCustomContextMenuAction`. The default drops it.
    fn custom_context_menu_action(&mut self, callback_id: &str) {
        let _ = callback_id;
    }
}
