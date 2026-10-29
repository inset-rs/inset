//! Hearing AVFoundation say something changed: key-value observing of an object's properties, and
//! notifications about an object, each telling a player's or a session's [`Changes`] and nothing
//! more, from whatever thread AVFoundation says so on. The app reads the state itself when it
//! hears.

use std::ffi::c_void;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::Message;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{
    NSDictionary, NSKeyValueChangeKey, NSKeyValueObservingOptions, NSNotification,
    NSNotificationCenter, NSNotificationName, NSObjectNSKeyValueObserverRegistration,
    NSOperationQueue, NSString,
};

use super::abort_on_panic;
use crate::media::Changes;

define_class!(
    /// Tells its changes whenever a property it observes changes.
    //
    // SAFETY: NSObject has no subclassing requirements, and `ChangeObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "InsetMediaChangeObserver"]
    #[ivars = Changes]
    struct ChangeObserver;

    unsafe impl NSObjectProtocol for ChangeObserver {}

    // SAFETY: the method has the signature `NSKeyValueObserving` declares.
    impl ChangeObserver {
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            _key_path: Option<&NSString>,
            _object: Option<&AnyObject>,
            _change: Option<&NSDictionary<NSKeyValueChangeKey, AnyObject>>,
            _context: *mut c_void,
        ) {
            abort_on_panic(|| self.ivars().notify());
        }
    }
);

/// Properties of one object observed, until dropped.
pub(super) struct PropertyObservation {
    object: Retained<NSObject>,
    observer: Retained<ChangeObserver>,
    key_paths: Vec<Retained<NSString>>,
}

impl PropertyObservation {
    /// Tells `changes` whenever one of `key_paths` of `object` changes.
    pub(super) fn new(
        object: &NSObject,
        key_paths: &[&str],
        changes: Changes,
    ) -> PropertyObservation {
        let observer = ChangeObserver::alloc().set_ivars(changes);
        // SAFETY: plain NSObject initialisation of an allocated instance.
        let observer: Retained<ChangeObserver> = unsafe { msg_send![super(observer), init] };
        let key_paths: Vec<Retained<NSString>> = key_paths
            .iter()
            .map(|path| NSString::from_str(path))
            .collect();
        for key_path in &key_paths {
            // SAFETY: the observer is removed in `Drop`, before it is let go, and the object is
            // kept until then.
            unsafe {
                object.addObserver_forKeyPath_options_context(
                    &observer,
                    key_path,
                    NSKeyValueObservingOptions::empty(),
                    std::ptr::null_mut(),
                );
            }
        }
        PropertyObservation {
            object: object.retain(),
            observer,
            key_paths,
        }
    }
}

impl Drop for PropertyObservation {
    fn drop(&mut self) {
        for key_path in &self.key_paths {
            // SAFETY: added in `new` for this observer and key path.
            unsafe {
                self.object
                    .removeObserver_forKeyPath(&self.observer, key_path)
            };
        }
    }
}

/// Notifications about one object heard on the main queue, until dropped.
pub(super) struct NotificationObservation {
    tokens: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

impl NotificationObservation {
    /// Calls `heard` on the main queue with each notification named one of `names` about
    /// `object`.
    pub(super) fn new(
        object: &AnyObject,
        names: &[&NSNotificationName],
        heard: impl Fn(&NSNotification) + 'static,
    ) -> NotificationObservation {
        let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
            // SAFETY: the notification center hands a live notification for the call.
            abort_on_panic(|| heard(unsafe { notification.as_ref() }));
        });
        let center = NSNotificationCenter::defaultCenter();
        let main_queue = NSOperationQueue::mainQueue();
        let tokens = names
            .iter()
            .map(|name| {
                // SAFETY: the block lives as long as the observation, which `Drop` removes; the
                // main queue serves it on the main thread.
                unsafe {
                    center.addObserverForName_object_queue_usingBlock(
                        Some(name),
                        Some(object),
                        Some(&main_queue),
                        &block,
                    )
                }
            })
            .collect();
        NotificationObservation { tokens }
    }
}

impl Drop for NotificationObservation {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for token in &self.tokens {
            // SAFETY: a token `addObserverForName:object:queue:usingBlock:` answered.
            unsafe { center.removeObserver(token.as_ref()) };
        }
    }
}
