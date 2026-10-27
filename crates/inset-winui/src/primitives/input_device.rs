//! XAML's `CInputManager` last-input-device record, which menus read to pick their padding states, and `CFocusManager`'s record of how the focus arrived, which decides whether the system focus visual shows.

use inset_embedder::PointerDeviceKind;
use inset_foundation::{App, Handle};
use inset_foundation::{Listenable, Listener};
use inset_gestures::{GestureBinding, PointerEvent, PointerRoute};
use inset_widgets::{AnyFocusNode, FocusManager, KeyEventResult};
use std::rc::Rc;

/// WinUI's input categories needed by menu layout on the desktop host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum InputDevice {
    /// No user input has been observed yet.
    #[default]
    None,

    /// A mouse or trackpad pointer.
    Mouse,

    /// Direct touch input uses the larger padding.
    Touch,

    /// Stylus input uses narrow padding.
    Pen,

    /// Keyboard activation uses narrow padding.
    Keyboard,
}

/// Per-App input history; listeners observe the same native routes used by FocusManager.
#[derive(Default)]
struct InputDeviceTracker {
    /// Device associated with the latest observed input.
    last: InputDevice,

    /// The focused node and how its focus arrived, recorded for the node that had the primary
    /// focus when last read.
    focus: Option<(AnyFocusNode, FocusState)>,

    /// The cause a control named for the focus it is giving a node, until the focus arrives.
    named: Option<(AnyFocusNode, FocusState)>,

    /// Prevents registering the observers more than once.
    installed: bool,
}

impl InputDevice {
    /// Starts tracking before an opening control can receive input; `FocusState::coerce_programmatic` reads `None` until then.
    pub(crate) fn initialize(app: &mut App) {
        let tracker = app.singleton::<InputDeviceTracker>();
        if app.get(tracker).installed {
            return;
        }
        app.get_mut(tracker).installed = true;
        let route = PointerRoute::new(move |app, event| {
            if matches!(event, PointerEvent::Added(_) | PointerEvent::Removed(_)) {
                return;
            }
            Self::pointer(app, event.kind());
        });
        GestureBinding::instance(app)
            .pointer_router(app)
            .add_global_route(app, route, None);
        FocusManager::instance(app).add_early_key_event_handler(
            app,
            Rc::new(move |app, _| {
                app.get_mut(tracker).last = Self::Keyboard;
                KeyEventResult::Ignored
            }),
        );
        // Records each focus change as it happens, so that later input can't stand in for its
        // cause before anything reads it.
        FocusManager::instance(app).add_listener(
            app,
            Listener::new(|app| {
                FocusState::current(app);
            }),
        );
    }

    /// Raw pointer handlers can open a menu before PointerRouter runs its global observers.
    pub(crate) fn pointer(app: &mut App, kind: PointerDeviceKind) {
        let input = match kind {
            PointerDeviceKind::Touch => Self::Touch,
            PointerDeviceKind::Stylus | PointerDeviceKind::InvertedStylus => Self::Pen,
            PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad => Self::Mouse,
            PointerDeviceKind::Unknown => return,
        };
        let tracker = app.singleton::<InputDeviceTracker>();
        app.get_mut(tracker).last = input;
    }

    /// Reads the input category when a menu is opened.
    pub(crate) fn current(app: &mut App) -> Self {
        let tracker: Handle<InputDeviceTracker> = app.singleton();
        app.get(tracker).last
    }

    /// MenuFlyoutItemBase::GetShouldBeNarrow.
    pub(crate) fn is_narrow(self) -> bool {
        matches!(self, Self::Mouse | Self::Pen | Self::Keyboard)
    }
}

/// XAML `FocusState` as `CFocusManager::CoerceFocusState` records it for the focused element: the cause of the focus change, which decides whether the system focus visual is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FocusState {
    /// Pointer input, or programmatic focus while the last input was not a keyboard; no focus visual.
    #[default]
    Pointer,

    /// Keyboard input, or programmatic focus while the last input was a keyboard; the focus visual shows.
    Keyboard,
}

impl FocusState {
    /// `CFocusManager::CoerceFocusState` for `FocusState::Programmatic`: the last input device decides, and only a keyboard yields `Keyboard`.
    pub(crate) fn coerce_programmatic(app: &mut App) -> Self {
        match InputDevice::current(app) {
            InputDevice::Keyboard => Self::Keyboard,
            InputDevice::None | InputDevice::Mouse | InputDevice::Touch | InputDevice::Pen => {
                Self::Pointer
            }
        }
    }

    /// Whether `CFocusManager` draws the system focus visual for this state (`GetRealFocusStateForFocusedElement() == FocusState::Keyboard`).
    pub(crate) fn shows_focus_visual(self) -> bool {
        self == Self::Keyboard
    }

    /// How the current focus arrived, as `CFocusManager` records it for the focused element: the
    /// cause a control named for it, else the input that moved it. A focus change is applied in a
    /// microtask right after the event that asked for it, so the first read after it, at the
    /// latest the focus manager's listener, finds that event as the last input.
    pub(crate) fn current(app: &mut App) -> Self {
        let primary = FocusManager::instance(app).primary_focus(app);
        let tracker: Handle<InputDeviceTracker> = app.singleton();
        let record = app.get(tracker);
        if let Some((node, state)) = record.focus
            && Some(node) == primary
        {
            return state;
        }
        let state = match record.named {
            Some((node, state)) if Some(node) == primary => state,
            _ => Self::coerce_programmatic(app),
        };
        let record = app.get_mut(tracker);
        record.focus = primary.map(|node| (node, state));
        if record.named.is_some_and(|(node, _)| Some(node) == primary) {
            record.named = None;
        }
        state
    }

    /// Names `state` as the cause of the focus a control is giving `node`, as XAML's
    /// `Focus(state)` does; the control then focuses `node` as it would. Focus already on `node`
    /// moves nothing to hear, so the record changes at once, and the control builds again.
    pub(crate) fn name(app: &mut App, node: AnyFocusNode, state: Self) {
        let tracker: Handle<InputDeviceTracker> = app.singleton();
        if node.has_primary_focus(app) {
            app.get_mut(tracker).focus = Some((node, state));
        } else {
            app.get_mut(tracker).named = Some((node, state));
        }
    }
}
