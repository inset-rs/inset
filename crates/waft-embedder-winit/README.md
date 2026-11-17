# waft-embedder-winit

Waft's desktop host: winit's event loop and native windows, with each view drawing through valo at the vsync of the screen its window is on, and held while the window is minimized or occluded. It takes an app's start closure through `WinitEmbedder::run`, which `#[waft::main]` and `waft-embedder-default` reach, and provides `WinitPlatform`, Waft's `Platform` on macOS, Windows and Linux. `src/lib.rs` says how the host is built. An application depends on `waft-embedder-default`, not this crate.

What it provides:

- Windows the app asks for, each with a view, and every view's frames, metrics, sight and focus.
- Input: the pointer, the trackpad, touches on Windows, the keyboard, text input and the system's input methods.
- Drawing: valo on the device every window draws with; composition layers, and buffers the system shares imported as textures, where the system has them.
- The system's shell: the menu bar and pop-up menus, the Dock icon's menu and the notification area, notifications, the clipboard, file dialogs, opening files and links, and quitting through the app's answer.
- The system's media: players, media assets and cameras, image codecs and video frame readers, where the system has them.
- The user's appearance, languages and local time zone.
- macOS only, through `WinitPlatform::macos`: hearing a reopen, a window that slides in, hiding the app, placing a composition layer in a `CALayer` the app holds, showing the system's layer as one, and keeping the app from being throttled while it does work the user is waiting on (`MacOs::begin_user_activity`, with a `UserActivity` that ends the activity when dropped).
