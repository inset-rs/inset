//! Flutter counterpart: `services/undo_manager.dart`.
//!
//! [`set_undo_state`](UndoManager::set_undo_state) is empty: there is no method channel.
//! [`setChannel`](https://api.flutter.dev/flutter/services/UndoManager/setChannel.html) is omitted.

use inset_foundation::{App, Handle, HandleId};

/// The direction in which an undo action should be performed, whether undo or redo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UndoDirection {
    /// Perform an undo action.
    Undo,
    /// Perform a redo action.
    Redo,
}

/// An interface to receive events from a native UndoManager.
///
/// Dart's `mixin UndoManagerClient`.
pub trait UndoManagerClient: Sized + 'static {
    /// Requests that the client perform an undo or redo operation.
    ///
    /// Currently only used on iOS 9+ when the undo or redo methods are invoked
    /// by the platform. For example, when using three-finger swipe gestures,
    /// the iPad keyboard, or voice control.
    fn handle_platform_undo(self: Handle<Self>, app: &mut App, direction: UndoDirection);

    /// Reverts the value on the stack to the previous value.
    fn undo(self: Handle<Self>, app: &mut App);

    /// Updates the value on the stack to the next value.
    fn redo(self: Handle<Self>, app: &mut App);

    /// Will be true if there are past values on the stack.
    fn can_undo(self: Handle<Self>, app: &App) -> bool;

    /// Will be true if there are future values on the stack.
    fn can_redo(self: Handle<Self>, app: &App) -> bool;

    /// This client as the erased [`AnyUndoManagerClient`].
    fn as_undo_manager_client(self: Handle<Self>) -> AnyUndoManagerClient {
        AnyUndoManagerClient {
            id: self.id(),
            vtable: const { &UndoManagerClientVTable::of::<Self>() },
        }
    }
}

struct UndoManagerClientVTable {
    handle_platform_undo: fn(&mut App, HandleId, UndoDirection),
    undo: fn(&mut App, HandleId),
    redo: fn(&mut App, HandleId),
    can_undo: fn(&App, HandleId) -> bool,
    can_redo: fn(&App, HandleId) -> bool,
}

fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl UndoManagerClientVTable {
    const fn of<C: UndoManagerClient>() -> UndoManagerClientVTable {
        UndoManagerClientVTable {
            handle_platform_undo: |app, id, direction| {
                C::handle_platform_undo(resolve(id), app, direction)
            },
            undo: |app, id| C::undo(resolve(id), app),
            redo: |app, id| C::redo(resolve(id), app),
            can_undo: |app, id| C::can_undo(resolve(id), app),
            can_redo: |app, id| C::can_redo(resolve(id), app),
        }
    }
}

/// Erased [`UndoManagerClient`]: one identity and a static vtable.
#[derive(Clone, Copy)]
pub struct AnyUndoManagerClient {
    id: HandleId,
    vtable: &'static UndoManagerClientVTable,
}

impl AnyUndoManagerClient {
    /// See [`UndoManagerClient::handle_platform_undo`].
    pub fn handle_platform_undo(self, app: &mut App, direction: UndoDirection) {
        (self.vtable.handle_platform_undo)(app, self.id, direction);
    }

    /// See [`UndoManagerClient::undo`].
    pub fn undo(self, app: &mut App) {
        (self.vtable.undo)(app, self.id);
    }

    /// See [`UndoManagerClient::redo`].
    pub fn redo(self, app: &mut App) {
        (self.vtable.redo)(app, self.id);
    }

    /// See [`UndoManagerClient::can_undo`].
    pub fn can_undo(self, app: &App) -> bool {
        (self.vtable.can_undo)(app, self.id)
    }

    /// See [`UndoManagerClient::can_redo`].
    pub fn can_redo(self, app: &App) -> bool {
        (self.vtable.can_redo)(app, self.id)
    }
}

impl PartialEq for AnyUndoManagerClient {
    fn eq(&self, other: &AnyUndoManagerClient) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyUndoManagerClient {}

impl std::fmt::Debug for AnyUndoManagerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyUndoManagerClient")
            .field("id", &self.id)
            .finish()
    }
}

/// A low-level interface to the system's undo manager.
///
/// To receive events from the system undo manager, create an
/// [`UndoManagerClient`] and set it as the [`client`](Self::client) on [`UndoManager`].
///
/// The [`set_undo_state`](Self::set_undo_state) method can be used to update the system's undo
/// manager using the `can_undo` and `can_redo` parameters.
///
/// When the system undo or redo button is tapped, the current
/// [`UndoManagerClient`] will receive [`UndoManagerClient::handle_platform_undo`]
/// with an [`UndoDirection`] representing whether the event is "undo" or "redo".
///
/// Currently, only iOS has an UndoManagerPlugin implemented on the engine side.
/// On iOS, this can be used to listen to the keyboard undo/redo buttons and the
/// undo/redo gestures.
///
/// See also:
///
///  * [NSUndoManager](https://developer.apple.com/documentation/foundation/nsundomanager)
///
/// Dart's `UndoManager._instance` is [`instance`](Self::instance), the App's singleton.
#[derive(Default)]
pub struct UndoManager {
    current_client: Option<AnyUndoManagerClient>,
}

impl UndoManager {
    /// Dart's `UndoManager._instance`.
    pub fn instance(app: &mut App) -> Handle<UndoManager> {
        app.singleton::<UndoManager>()
    }

    /// Receive undo and redo events from the system's [`UndoManager`].
    ///
    /// Setting the [`client`](Self::client) will cause [`UndoManagerClient::handle_platform_undo`]
    /// to be called when a system undo or redo is triggered, such as by tapping
    /// the undo/redo keyboard buttons or using the 3-finger swipe gestures.
    pub fn set_client(app: &mut App, client: Option<AnyUndoManagerClient>) {
        let this = Self::instance(app);
        app.get_mut(this).current_client = client;
    }

    /// Return the current [`UndoManagerClient`].
    pub fn client(app: &mut App) -> Option<AnyUndoManagerClient> {
        let this = Self::instance(app);
        app.get(this).current_client
    }

    /// Set the current state of the system UndoManager. [`can_undo`] and [`can_redo`]
    /// control the respective "undo" and "redo" buttons of the system UndoManager.
    ///
    /// Empty: there is no method channel.
    pub fn set_undo_state(_app: &mut App, _can_undo: bool, _can_redo: bool) {}

    /// Flutter `UndoManagerClient.handleUndo`: the host asked for undo or redo.
    pub fn handle_platform_undo(app: &mut App, direction: UndoDirection) {
        let this = Self::instance(app);
        let client = app.get(this).current_client;
        debug_assert!(
            client.is_some(),
            "There must be a current UndoManagerClient."
        );
        client
            .expect("There must be a current UndoManagerClient.")
            .handle_platform_undo(app, direction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_foundation::AppCell;

    #[derive(Default)]
    struct RecordingClient {
        directions: Vec<UndoDirection>,
        undos: usize,
        redos: usize,
    }

    impl UndoManagerClient for RecordingClient {
        fn handle_platform_undo(self: Handle<Self>, app: &mut App, direction: UndoDirection) {
            app.get_mut(self).directions.push(direction);
            match direction {
                UndoDirection::Undo => self.undo(app),
                UndoDirection::Redo => self.redo(app),
            }
        }

        fn undo(self: Handle<Self>, app: &mut App) {
            app.get_mut(self).undos += 1;
        }

        fn redo(self: Handle<Self>, app: &mut App) {
            app.get_mut(self).redos += 1;
        }

        fn can_undo(self: Handle<Self>, _app: &App) -> bool {
            false
        }

        fn can_redo(self: Handle<Self>, _app: &App) -> bool {
            false
        }
    }

    #[test]
    fn set_client_receives_handle_platform_undo() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let client = app.create(RecordingClient::default());
        UndoManager::set_client(&mut app, Some(client.as_undo_manager_client()));
        assert_eq!(
            UndoManager::client(&mut app),
            Some(client.as_undo_manager_client())
        );

        UndoManager::handle_platform_undo(&mut app, UndoDirection::Undo);
        UndoManager::handle_platform_undo(&mut app, UndoDirection::Redo);
        assert_eq!(
            app.get(client).directions,
            [UndoDirection::Undo, UndoDirection::Redo]
        );
        assert_eq!(app.get(client).undos, 1);
        assert_eq!(app.get(client).redos, 1);

        UndoManager::set_client(&mut app, None);
        assert!(UndoManager::client(&mut app).is_none());
        UndoManager::set_undo_state(&mut app, true, true);
    }
}
