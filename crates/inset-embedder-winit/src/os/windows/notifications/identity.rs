//! Who the app is to the notification platform, which shows a toast only for an app it knows:
//! a packaged app by its package; an unpackaged one by the AppUserModelID the process set for
//! itself (`SetCurrentProcessExplicitAppUserModelID`), which the platform knows from a Start-menu
//! shortcut carrying it or from the key `HKCU\Software\Classes\AppUserModelId\<id>`. The embedder
//! reads that identity and registers none: an app without one has no notifications.
//!
//! The same key's `CustomActivator` names the COM class the platform activates for a click on
//! one of the app's toasts, after the app quit as well as while it runs (`activator`). The Windows
//! App SDK's `AppNotificationManager.Register` writes such a key, and the class's server, into the
//! user's registry itself; here that is the app installer's to write.

use std::ffi::c_void;
use std::mem::size_of;

use windows::UI::Notifications::{ToastNotificationManager, ToastNotifier};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::Win32::UI::Shell::GetCurrentProcessExplicitAppUserModelID;
use windows::core::{GUID, HSTRING, PCWSTR, Result};

/// The group every toast of the embedder's is in, which the history removes a toast by along
/// with its tag.
const GROUP: &str = "inset";

/// The app, as the notification platform knows it.
pub(crate) enum AppIdentity {
    /// A packaged app, known by its package.
    Package,
    /// An unpackaged app, known by the AppUserModelID it set for its process.
    Unpackaged(HSTRING),
}

impl AppIdentity {
    /// This process's identity; `None` for an unpackaged process that set no AppUserModelID.
    pub(crate) fn of_this_process() -> Option<AppIdentity> {
        if is_packaged() {
            return Some(AppIdentity::Package);
        }
        explicit_app_user_model_id().map(AppIdentity::Unpackaged)
    }

    /// The notifier that shows and schedules the app's toasts.
    pub(crate) fn notifier(&self) -> Result<ToastNotifier> {
        match self {
            AppIdentity::Package => ToastNotificationManager::CreateToastNotifier(),
            AppIdentity::Unpackaged(id) => ToastNotificationManager::CreateToastNotifierWithId(id),
        }
    }

    /// Takes the toast tagged `tag` out of the app's toasts shown and in the Action Center.
    pub(crate) fn remove_shown(&self, tag: &HSTRING) -> Result<()> {
        let history = ToastNotificationManager::History()?;
        let group = group();
        match self {
            AppIdentity::Package => history.RemoveGroupedTag(tag, &group),
            AppIdentity::Unpackaged(id) => history.RemoveGroupedTagWithId(tag, &group, id),
        }
    }

    /// The COM class the platform activates for a click on one of the app's toasts, as the app's
    /// registration names it; `None` for a packaged app, whose manifest names it, and for an app
    /// registered by its shortcut alone.
    pub(crate) fn activator(&self) -> Option<GUID> {
        let AppIdentity::Unpackaged(id) = self else {
            return None;
        };
        let key = HSTRING::from(format!(r"Software\Classes\AppUserModelId\{id}"));
        [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE]
            .into_iter()
            .find_map(|root| registry_string(root, &key, "CustomActivator"))
            .and_then(|clsid| parsed_clsid(&clsid))
    }
}

/// The group the embedder's toasts carry.
pub(crate) fn group() -> HSTRING {
    HSTRING::from(GROUP)
}

fn is_packaged() -> bool {
    let mut length = 0;
    // SAFETY: with no buffer the call only answers that the name needs one, or that the process
    // has no package.
    let answered = unsafe { GetCurrentPackageFullName(&mut length, None) };
    answered == ERROR_INSUFFICIENT_BUFFER
}

/// The AppUserModelID the process set for itself, if it set one.
fn explicit_app_user_model_id() -> Option<HSTRING> {
    // SAFETY: the call answers a string the system allocated for the caller, freed below.
    let id = unsafe { GetCurrentProcessExplicitAppUserModelID() }.ok()?;
    // SAFETY: a NUL-terminated string the call just answered.
    let text = unsafe { id.to_hstring() };
    // SAFETY: the string the call allocated, freed once.
    unsafe { CoTaskMemFree(Some(id.0 as *const c_void)) };
    Some(text)
}

/// The string value `value` of the key `key` under `root`, if it is there.
fn registry_string(root: HKEY, key: &HSTRING, value: &str) -> Option<String> {
    let value = HSTRING::from(value);
    let mut buffer = [0u16; 64];
    let mut size = (buffer.len() * size_of::<u16>()) as u32;
    // SAFETY: NUL-terminated names that outlive the call, and a buffer of `size` bytes, which
    // the call fills with a NUL-terminated string or refuses.
    let read = unsafe {
        RegGetValueW(
            root,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if read != ERROR_SUCCESS {
        return None;
    }
    let length = buffer.iter().position(|&unit| unit == 0)?;
    String::from_utf16(&buffer[..length]).ok()
}

/// A CLSID as the registry writes it, `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`.
fn parsed_clsid(text: &str) -> Option<GUID> {
    let bare = text.trim().trim_start_matches('{').trim_end_matches('}');
    GUID::try_from(bare).ok()
}
