//! The app's toast activator: the COM class its registration names (`identity`), which the
//! platform activates with a click's arguments and what was typed, whether the app runs or the
//! platform starts it for the click, as it does with `-Embedding` or the arguments the class's
//! `LocalServer32` gives. The class's object is registered for the process's life, from a thread
//! of its own in the process's multithreaded apartment, so a click reaches it without the main
//! thread, and before the loop runs, as the platform activates a class that started the app only
//! once the app registers it.
//!
//! While the activator is registered, every click comes through it, a toast's own `Activated`
//! event goes unheard (`mod.rs`), and no click is heard twice.

use std::slice;
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::Duration;

use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_POINTER};
use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoInitializeEx, CoRegisterClassObject,
    IClassFactory, IClassFactory_Impl, REGCLS_MULTIPLEUSE,
};
use windows::Win32::UI::Notifications::{
    INotificationActivationCallback, INotificationActivationCallback_Impl,
    NOTIFICATION_USER_INPUT_DATA,
};
use windows::core::{BOOL, GUID, IUnknown, Interface, PCWSTR, Ref, Result, implement};

use super::arguments;
use crate::notifications::NotificationActivations;

/// The longest the launch waits for the activator's registration.
const PATIENCE: Duration = Duration::from_secs(5);

/// Whether the activator is registered, set once, before the loop runs.
static REGISTERED: OnceLock<bool> = OnceLock::new();

/// Registers the class `clsid` as the app's toast activator, sending each click to
/// `activations`; answers once it is registered, or failed to be.
pub(crate) fn register(clsid: GUID, activations: NotificationActivations) {
    let (registered, registering) = mpsc::channel();
    let started = std::thread::Builder::new()
        .name("inset-toast-activator".to_owned())
        .spawn(move || serve(clsid, activations, &registered));
    let is_registered = started.is_ok()
        && registering
            .recv_timeout(PATIENCE)
            .is_ok_and(|registered| registered);
    let _ = REGISTERED.set(is_registered);
}

/// Whether clicks come through the activator.
pub(crate) fn is_registered() -> bool {
    REGISTERED.get().copied().unwrap_or(false)
}

/// Registers the class's object in the multithreaded apartment, tells `registered` whether it
/// is, and keeps the thread, and with it the apartment and the registration, for the process's
/// life.
fn serve(clsid: GUID, activations: NotificationActivations, registered: &mpsc::Sender<bool>) {
    // SAFETY: COM entered on this thread, which never leaves it, so the apartment outlives the
    // registration.
    let entered = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if entered.is_err() {
        let _ = registered.send(false);
        return;
    }
    let factory: IClassFactory = ActivatorFactory { activations }.into();
    // SAFETY: the factory is a live COM object, registered for local activation and never
    // revoked, as the process serves clicks until it ends.
    let registration =
        unsafe { CoRegisterClassObject(&clsid, &factory, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE) };
    let _ = registered.send(registration.is_ok());
    if registration.is_ok() {
        loop {
            std::thread::park();
        }
    }
}

/// The class's factory, which COM asks for the activator.
#[implement(IClassFactory)]
struct ActivatorFactory {
    activations: NotificationActivations,
}

impl IClassFactory_Impl for ActivatorFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        object: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        if object.is_null() || iid.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: COM passes a writable slot for the object, checked above.
        unsafe { *object = std::ptr::null_mut() };
        if !outer.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let activator: INotificationActivationCallback = Activator {
            activations: self.activations.clone(),
        }
        .into();
        // SAFETY: COM passes the interface it wants and the slot for it, both checked above.
        unsafe { activator.query(iid, object).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

/// The activator, which hears a click.
#[implement(INotificationActivationCallback)]
struct Activator {
    activations: NotificationActivations,
}

impl INotificationActivationCallback_Impl for Activator_Impl {
    fn Activate(
        &self,
        _app_user_model_id: &PCWSTR,
        invoked_arguments: &PCWSTR,
        data: *const NOTIFICATION_USER_INPUT_DATA,
        count: u32,
    ) -> Result<()> {
        // SAFETY: the platform passes the click's arguments as a NUL-terminated string, or null.
        let invoked = unsafe { text(*invoked_arguments) };
        let typed = if data.is_null() {
            &[][..]
        } else {
            // SAFETY: the platform passes `count` pairs of what was typed, valid for the call.
            unsafe { slice::from_raw_parts(data, count as usize) }
        };
        let activation = arguments::activation(&invoked, |input| {
            typed.iter().find_map(|pair| {
                // SAFETY: each pair's key and value are NUL-terminated strings, or null.
                let (key, value) = unsafe { (text(pair.Key), text(pair.Value)) };
                (key == input).then_some(value)
            })
        });
        if let Some(activation) = activation {
            self.activations.send(activation);
        }
        Ok(())
    }
}

/// `string` as text; empty for null.
///
/// # Safety
/// `string` is null or a NUL-terminated string valid for the call.
unsafe fn text(string: PCWSTR) -> String {
    if string.is_null() {
        return String::new();
    }
    // SAFETY: a NUL-terminated string, as the caller promises.
    unsafe { string.to_string() }.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use inset_embedder::NotificationActivation;
    use windows::core::HSTRING;

    use super::*;

    #[test]
    fn a_click_through_the_activator_reaches_the_app_with_what_was_typed() {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let hearing = Arc::clone(&heard);
        let factory: IClassFactory = ActivatorFactory {
            activations: NotificationActivations::to(move |activation| {
                hearing.lock().unwrap().push(activation);
            }),
        }
        .into();
        // SAFETY: no outer object; the factory answers a new activator.
        let activator: INotificationActivationCallback =
            unsafe { factory.CreateInstance(None) }.expect("an activator");
        let app = HSTRING::from("Inset.Test");
        let invoked = HSTRING::from(arguments::of_action("message-7", "answer", Some("reply-1")));
        let (key, value) = (HSTRING::from("reply-1"), HSTRING::from("on my way"));
        let typed = [NOTIFICATION_USER_INPUT_DATA {
            Key: PCWSTR(key.as_ptr()),
            Value: PCWSTR(value.as_ptr()),
        }];
        // SAFETY: strings and pairs that outlive the calls.
        unsafe {
            activator
                .Activate(&app, &invoked, &typed)
                .expect("activated");
            activator
                .Activate(&app, &HSTRING::from("page=settings"), &[])
                .expect("activated");
        }
        assert_eq!(
            *heard.lock().unwrap(),
            [NotificationActivation {
                id: "message-7".to_owned(),
                action: Some("answer".to_owned()),
                reply: Some("on my way".to_owned()),
            }],
            "another toast's arguments tell the app nothing"
        );
    }
}
