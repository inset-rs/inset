//! Waft's scene delegate, which the `UIApplicationSceneManifest` cargo waft writes names for every
//! window scene, as Flutter's `FlutterSceneDelegate` is named in its app template: it hands the host
//! each scene's connection, with the URLs it was opened for, its moves between the foreground and
//! the background, its disconnection, and the URLs it is asked to open later. The window is the
//! host's to make (`view.rs`), as the delegate keeps nothing of a scene.
//!
//! A scene is told apart by its session's persistent identifier, which UIKit keeps for a scene it
//! disconnects and connects again, so the view the scene showed waits for it (`views.rs`).

// A class UIKit makes by name needs `unsafe` to define; this module writes it for it.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSSet, NSURL};
use objc2_ui_kit::{
    UIOpenURLContext, UIResponder, UIScene, UISceneConnectionOptions, UISceneDelegate,
    UISceneSession, UIWindowScene, UIWindowSceneDelegate,
};

use crate::host::{HostEvent, deliver};
use crate::lifecycle::SceneState;

define_class!(
    // SAFETY: UIResponder has no subclassing requirements, and `WaftSceneDelegate` has no `Drop`.
    #[unsafe(super(UIResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftSceneDelegate"]
    pub(crate) struct WaftSceneDelegate;

    unsafe impl NSObjectProtocol for WaftSceneDelegate {}

    unsafe impl UISceneDelegate for WaftSceneDelegate {
        #[unsafe(method(scene:willConnectToSession:options:))]
        fn will_connect(
            &self,
            scene: &UIScene,
            session: &UISceneSession,
            options: &UISceneConnectionOptions,
        ) {
            let Some(scene) = window_scene(scene) else {
                return;
            };
            // UIKit answers no set, where the binding says it answers one, when the scene was
            // opened for no URL.
            // SAFETY: `URLContexts`, a getter that answers a set of contexts or nil.
            let contexts: Option<Retained<NSSet<UIOpenURLContext>>> =
                unsafe { msg_send![options, URLContexts] };
            deliver(HostEvent::SceneConnected {
                scene,
                session: session.persistentIdentifier().to_string(),
                urls: contexts.as_deref().map(urls_of).unwrap_or_default(),
            });
        }

        #[unsafe(method(sceneDidDisconnect:))]
        fn did_disconnect(&self, scene: &UIScene) {
            deliver(HostEvent::SceneDisconnected {
                session: session_of(scene),
            });
        }

        #[unsafe(method(sceneWillEnterForeground:))]
        fn will_enter_foreground(&self, scene: &UIScene) {
            state_changed(scene, SceneState::Inactive);
        }

        #[unsafe(method(sceneDidBecomeActive:))]
        fn did_become_active(&self, scene: &UIScene) {
            state_changed(scene, SceneState::Active);
        }

        #[unsafe(method(sceneWillResignActive:))]
        fn will_resign_active(&self, scene: &UIScene) {
            state_changed(scene, SceneState::Inactive);
        }

        #[unsafe(method(sceneDidEnterBackground:))]
        fn did_enter_background(&self, scene: &UIScene) {
            state_changed(scene, SceneState::Background);
        }

        #[unsafe(method(scene:openURLContexts:))]
        fn open_url_contexts(&self, _scene: &UIScene, contexts: &NSSet<UIOpenURLContext>) {
            deliver(HostEvent::UrlsOpened(urls_of(contexts)));
        }
    }

    unsafe impl UIWindowSceneDelegate for WaftSceneDelegate {}
);

/// `scene` as the window scene every scene of the app's is, as its manifest configures it.
fn window_scene(scene: &UIScene) -> Option<Retained<UIWindowScene>> {
    use objc2::Message;
    scene.retain().downcast::<UIWindowScene>().ok()
}

/// The persistent identifier of `scene`'s session.
fn session_of(scene: &UIScene) -> String {
    scene.session().persistentIdentifier().to_string()
}

fn state_changed(scene: &UIScene, state: SceneState) {
    deliver(HostEvent::SceneState {
        session: session_of(scene),
        state,
    });
}

/// The URLs `contexts` carry.
fn urls_of(contexts: &NSSet<UIOpenURLContext>) -> Vec<Retained<NSURL>> {
    contexts.iter().map(|context| context.URL()).collect()
}
