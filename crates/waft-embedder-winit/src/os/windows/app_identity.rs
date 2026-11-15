//! Which app the taskbar takes the process and each window for: the process's AppUserModelID
//! (`SetCurrentProcessExplicitAppUserModelID`), set before any window shows, and a window's own,
//! with what a pin of it starts, as properties of the window (`SHGetPropertyStoreForWindow`). A
//! window with an ID of its own is grouped, shown and pinned as that app, apart from the process.
//!
//! The invariants a later editor must keep: a window's properties are set while it is still hidden,
//! so the taskbar never groups it under the process first; and they are set back to `VT_EMPTY`
//! before the window is destroyed, as Microsoft documents, or the shell leaks what they hold.

use std::mem::ManuallyDrop;

use waft_embedder::{WindowsIconResource, WindowsWindowConfig};
use windows::Win32::Foundation::{HWND, PROPERTYKEY};
use windows::Win32::Storage::EnhancedStorage::{
    PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
    PKEY_AppUserModel_RelaunchDisplayNameResource, PKEY_AppUserModel_RelaunchIconResource,
};
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::Shell::{SHStrDupW, SetCurrentProcessExplicitAppUserModelID};
use windows::core::{HSTRING, Result};

/// Every property a window's identity sets, cleared together.
const PROPERTIES: [PROPERTYKEY; 4] = [
    PKEY_AppUserModel_ID,
    PKEY_AppUserModel_RelaunchCommand,
    PKEY_AppUserModel_RelaunchDisplayNameResource,
    PKEY_AppUserModel_RelaunchIconResource,
];

/// Gives the process the AppUserModelID `id`, which every window without one of its own shows
/// under and the toasts it posts are filed under.
pub(super) fn set_process_app_user_model_id(id: &str) {
    // SAFETY: a NUL-terminated string that outlives the call.
    if let Err(error) = unsafe { SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(id)) } {
        eprintln!("waft: the process's AppUserModelID {id} was refused: {error}");
    }
}

/// Gives the hidden window `window` the identity `config` names, where it names one.
pub(super) fn label(window: HWND, config: &WindowsWindowConfig) {
    let Some(id) = &config.app_user_model_id else {
        return;
    };
    let labelled = store_of(window).and_then(|store| {
        let relaunch = config.relaunch.as_ref();
        set_string(&store, &PKEY_AppUserModel_ID, Some(id))?;
        set_string(
            &store,
            &PKEY_AppUserModel_RelaunchCommand,
            relaunch.map(|relaunch| relaunch.command.as_str()),
        )?;
        set_string(
            &store,
            &PKEY_AppUserModel_RelaunchDisplayNameResource,
            relaunch.map(|relaunch| relaunch.display_name.as_str()),
        )?;
        let icon = relaunch.and_then(|relaunch| relaunch.icon.as_ref());
        set_string(
            &store,
            &PKEY_AppUserModel_RelaunchIconResource,
            icon.map(icon_resource).as_deref(),
        )
    });
    if let Err(error) = labelled {
        eprintln!("waft: the window's AppUserModelID {id} was not set: {error}");
    }
}

/// Clears `window`'s identity, where it has one, before the window is destroyed.
pub(super) fn unlabel(window: HWND) {
    let Ok(store) = store_of(window) else {
        return;
    };
    // SAFETY: a key of the store's, read into a value the call allocates.
    let has_identity =
        unsafe { store.GetValue(&PKEY_AppUserModel_ID) }.is_ok_and(|value| !value.is_empty());
    if !has_identity {
        return;
    }
    for key in &PROPERTIES {
        let _ = set_string(&store, key, None);
    }
}

/// The icon as the shell names it: `<path>,<index>`.
fn icon_resource(icon: &WindowsIconResource) -> String {
    format!("{},{}", icon.path.display(), icon.index)
}

fn store_of(window: HWND) -> Result<IPropertyStore> {
    // SAFETY: a live window of this process's.
    unsafe { SHGetPropertyStoreForWindow(window) }
}

/// Sets `key` to `text`, or to `VT_EMPTY`, which removes it, for `None`.
fn set_string(store: &IPropertyStore, key: &PROPERTYKEY, text: Option<&str>) -> Result<()> {
    let value = match text {
        Some(text) => string_value(text)?,
        None => PROPVARIANT::default(),
    };
    // SAFETY: a key and a value that outlive the call, which copies the value.
    unsafe { store.SetValue(key, &value) }
}

/// A `VT_LPWSTR` holding `text`, allocated with the task allocator, as `PropVariantClear`, which
/// the value's drop calls, frees it. The shell takes these properties as `VT_LPWSTR` alone, not as
/// the `VT_BSTR` the `windows` crate makes from a string.
fn string_value(text: &str) -> Result<PROPVARIANT> {
    // SAFETY: a NUL-terminated string, copied into memory the caller owns.
    let copy = unsafe { SHStrDupW(&HSTRING::from(text)) }?;
    Ok(PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_LPWSTR,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 { pwszVal: copy },
            }),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn an_icon_is_named_by_its_file_and_index() {
        let icon = WindowsIconResource {
            path: PathBuf::from(r"C:\Users\me\AppData\Local\WaOS\icons\notes.ico"),
            index: 0,
        };
        assert_eq!(
            icon_resource(&icon),
            r"C:\Users\me\AppData\Local\WaOS\icons\notes.ico,0"
        );
    }

    #[test]
    fn a_string_value_is_the_wide_string_the_shell_reads() {
        let value = string_value("dev.waos.app.notes").expect("a value");
        assert_eq!(value.vt(), VT_LPWSTR);
        // SAFETY: a VT_LPWSTR's string, alive with the value.
        let text = unsafe { value.Anonymous.Anonymous.Anonymous.pwszVal.to_string() };
        assert_eq!(text.ok().as_deref(), Some("dev.waos.app.notes"));
    }
}
