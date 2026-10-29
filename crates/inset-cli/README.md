# inset-cli

`cargo inset`: run and package an Inset app for desktop, iOS, Android, and the web from one project.

```sh
cargo install inset-cli
```

## Commands

```sh
cargo inset new myapp            # a crate every host can load
cargo inset doctor               # what this machine can build for, with the fix for each gap
cargo inset devices              # what `run -d` accepts

cargo inset run                  # this desktop: on macOS the built .app, elsewhere cargo run
cargo inset run -d ios           # the booted simulator, or the newest iPhone
cargo inset run -d "iPhone 16"   # a simulator by name or id
cargo inset run -d android       # the connected Android device, or the first emulator, booted
cargo inset run -d Medium_Phone  # an emulator by AVD name, or a device by serial
cargo inset run -d web           # build, serve locally, open the default browser (`chrome` for Chrome)
cargo inset run --release        # optimized build

cargo inset build macos          # .app              build dmg
cargo inset build windows        # folder            build msi | nsis
cargo inset build linux          # folder            build deb | appimage
cargo inset build ios            # .app for a device (`--simulator` for the simulator)
cargo inset build ipa            # .ipa around the device .app
cargo inset build apk            # .apk for 64-bit ARM, signed with the debug key
cargo inset build web            # static folder: index.html, app.js, app_bg.wasm, .br copies
```

`run` builds debug unless `--release`; `build` builds release unless `--debug`. Output lands under `target/inset/<target>/<profile>/`, so `cargo clean` removes it. In a workspace, pass `-p <package>`.

## Project shape

`new` starts from a widget kit: `--template cupertino` (the default, phone apps) or `--template winui` (desktop apps, from [inset-winui](https://github.com/inset-rs/inset-winui)). Both write:

```
myapp/
  Cargo.toml        lib (cdylib + rlib) and a bin, the kit dependency, [package.metadata.inset]
  src/lib.rs        the app, with an #[inset::main] function
  src/main.rs       one line calling the library's main; never edited
  assets/icon.png   one 1024 px PNG; every platform's sizes derive from it
```

The app is the library because the browser and Android load a library, not an executable. `#[inset::main]` emits the entry point for the compile target:

```rust
#[inset::main]
fn main(app: &mut App) {
    run_app(app, CupertinoApp::new().home(Home).into_widget());
}
```

On a desktop `run_app` asks for a window with the default configuration; `run_app_with` takes the window's title, size and the rest.

## Metadata

```toml
[package.metadata.inset]
identifier = "com.example.myapp"   # reverse-DNS bundle id; defaults to com.example.<package>
name = "My App"                    # display name; defaults to the package name
icon = "assets/icon.png"           # defaults to assets/icon.png when it exists
resources = ["assets/**"]          # globs copied into the bundle at the same relative path

[package.metadata.inset.usage_descriptions]  # why the app uses a device the system asks the user about first
camera = "Takes the photo on your badge."
microphone = "Records sound with your videos."

[package.metadata.inset.macos]
minimum_system_version = "11.0"
background_app = true              # LSUIElement: no Dock tile or menu bar
team = "ABCDE12345"                # narrows the signing identity when a machine has several
entitlements = "macos/app.entitlements"  # signed into the bundle: what the hardened runtime otherwise denies
helpers = [{ package = "my-app-tools", bin = "my-helper" }]  # other targets of the workspace the bundle carries, built with the app
libraries = [{ package = "my-app-tools", lib = "my_plugin" }]

[package.metadata.inset.ios]
team = "ABCDE12345"                # narrows the signing identity when a machine has several
minimum_version = "15.0"

[package.metadata.inset.android]
min_sdk = 26                       # the default: Android 8.0, the first whose clip says when it was set
target_sdk = 34                    # 35 and up draw edge to edge: the safe area goes, until the host reads insets
version_code = 100                 # defaults to one derived from the version: 1.2.3 is 10203

[[package.metadata.inset.file_associations]]  # a kind of file this app defines
extensions = ["mydoc"]
name = "My document"
owner = true
role = "editor"                    # viewer (the default), editor, shell or none

[[package.metadata.inset.file_associations]]  # another app's kind of file this app opens
extensions = ["md", "markdown"]
name = "Markdown document"
identifier = "net.daringfireball.markdown"  # its macOS type identifier, when it has one
mime_type = "text/markdown"
```

The Android package name is the identifier as a Java package name: `com.example.my-app` becomes `com.example.my_app`.

Version comes from `[package]`. A project-root `index.html` replaces the generated web page; keep a `<canvas id="inset">` and import `./app.js`.

### File associations

Each `file_associations` table lets the system open the app for files with those extensions.

- macOS: a document type; a type the app owns opens in it on double-click, another app's type is offered under Open With. `role` applies here only.
- Windows: the installer registers the extensions; the app gets the file as its first argument, which it reads itself.
- Linux: the desktop entry lists each `mime_type`; a type without one isn't associated.
- iOS, Android and the web: not declared.

An app hears the files and links the system itself opens it with through `App::set_open_requested_handler`, at launch and while it runs on macOS, iOS and Android, and at launch on the web. On Windows and Linux the system gives the app only its command line: the app reads it, decides which arguments are files, and hands them to the same handler with `App::open_requested` (see `examples/app_events`). There a second launch is a new process; `inset-single-instance` hands its command line to the first.

### Helpers and libraries

A macOS app can carry other targets of its workspace in its bundle: `helpers`, binaries, in `Contents/Helpers/<bin>`, and `libraries`, `cdylib`s named by their `[lib] name`, in `Contents/Frameworks/lib<name>.dylib`. `build macos` and `run` build each with the app's own profile, for the same machine, and put it in.

cargo-packager has no place for a helper, and signs a bundle as it makes it, so for an app that carries anything the CLI signs the bundle itself, inside out: each helper and library with the app's identity and the hardened runtime, and no entitlements of their own, then the bundle with the app's entitlements. Without an identity nothing is signed, and each keeps the ad hoc signature the linker gave it, as the app's executable does. The app and its helpers find them by path from their own executable: `../Helpers/<bin>` and `../Frameworks/lib<name>.dylib`.

### Usage descriptions

macOS and iOS ask the user before an app opens the camera or the microphone, showing the app's own sentence for why, and end an app that opens one without such a sentence in its `Info.plist`. Each key of `usage_descriptions` declares one device and gives its sentence:

- `camera`: `NSCameraUsageDescription`.
- `microphone`: `NSMicrophoneUsageDescription`.

On macOS the bundle is also signed with the hardened runtime's entitlement for each declared device, `com.apple.security.device.camera` and `com.apple.security.device.audio-input`, merged into the app's own `entitlements`, whose value wins where it names one: a signed bundle runs with the hardened runtime, which refuses a device its entitlements leave out. Android and the web: not declared yet.

## Signing

Signing happens when credentials are present, never by flag.

- iOS simulator: no account and no signing.
- iOS device: the Apple Development certificate in your keychain and a provisioning profile Xcode stored, found the way Flutter finds them; nothing to configure once you have signed in under Xcode Settings > Accounts and run any app on the device. `doctor` names the certificate that will be used; `ios.team` in the metadata picks one when a machine has several. `build ipa` needs a distribution profile and is uploaded with Transporter or `xcrun altool`.
- macOS: the `.app` is signed with `APPLE_SIGNING_IDENTITY`, an identity in the keychain by its name, part of it, or its hash, `-` for ad hoc; else with the keychain's Apple Development certificate when there is one, so permission grants such as Accessibility survive a rebuild; else ad hoc. A certificate's signature carries the hardened runtime. Distribution needs a Developer ID and notarization, which Apple only grants against an account: with `APPLE_SIGNING_IDENTITY` naming the Developer ID, `build macos` notarizes and staples the `.app`, and `build dmg` the `.app` and the disk image, when the environment holds one of these sets:
  - `APPLE_KEYCHAIN_PROFILE`, a profile stored with `xcrun notarytool store-credentials`;
  - `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password) and `APPLE_TEAM_ID`;
  - an App Store Connect key: `APPLE_API_KEY` (its id), `APPLE_API_ISSUER` and `APPLE_API_KEY_PATH`.

  `run` is never notarized.
- Windows: `certificate_thumbprint` or a custom sign command, through cargo-packager.
- Android: the debug key in `~/.android/debug.keystore`, made with `keytool` when the machine has none, for `run` and `build apk` alike. A release key is not here yet.

## Size

`build web` runs `wasm-opt -Os` and writes brotli copies. The release profile decides the rest. Measured on the gallery (wasm, then brotli):

| `[profile.release]`                  | wasm     | brotli   |
|--------------------------------------|----------|----------|
| defaults                             | 13.6 MB  | 2.48 MB  |
| `lto = "fat"`, `codegen-units = 1`   | 12.6 MB  | 2.34 MB  |
| `opt-level = "s"`                    | 8.9 MB   | 1.97 MB  |
| `opt-level = "z"`                    | 8.0 MB   | 1.85 MB  |

`panic = "abort"` changes nothing on wasm. `new` writes the LTO pair, which mostly buys speed; add `opt-level = "s"` when download size matters more than native speed.

## macOS

The CLI puts the `.app` together itself, as it does an iOS app: the executable in `Contents/MacOS`, the `Info.plist`, the icon (an `.icns` made by `iconutil`) and the resources in `Contents/Resources`, signed with `codesign` as [Signing](#signing) says. `build dmg` makes the disk image around it with `hdiutil`: the app beside a link to Applications in a 600 by 400 window, laid out by a `.DS_Store` the CLI writes, so no Finder window opens while it builds, and the app's icon as the volume's.

## Tools it fetches

`wasm-bindgen` at the version the app's Cargo.lock resolved, and `wasm-opt` for release web builds, are downloaded once into `~/.cache/inset` (`INSET_CACHE_DIR` overrides). Everything else is cargo, rustup, and the Xcode command line tools.

## Android

The `.apk` is built without Gradle and without any Java of your own. The Java is Inset's three classes (`android/rs/inset/`), which `javac` and `d8` compile into the package's `classes.dex`:

- `rs.inset.InsetActivity`, the activity: Android's `NativeActivity`, which loads your library and calls the `android_main` that `#[inset::main]` emits, with the results of the activities your app starts, the intents that reach it while it runs and the user's answers to the permissions it asks for handed on to it, as the file dialogs, being opened and notifications need.
- `rs.inset.InsetFileProvider`, a content provider at `<package>.inset.files`, not exported, through which the opener shares a file of the app's with the app that opens it.
- `rs.inset.InsetNotificationReceiver`, a broadcast receiver, not exported, which builds and shows the app's notifications, and shows a scheduled one when its alarm goes off, whether or not the app runs.

The manifest's `<queries>` names viewing `http`, `https` and `mailto` links, so the opener can say whether some app takes one; the application's meta-data `rs.inset.queried_schemes` lists the same schemes for it. It asks for `POST_NOTIFICATIONS`, which Android 13 asks the user for when the app asks (`NotificationPermission::request`), and `SCHEDULE_EXACT_ALARM`, which Android 14 leaves to the user to grant under the app's Alarms & reminders setting: without it a scheduled notification may come late. A scheduled notification does not survive a restart of the device.

You need:

- the Android SDK, with build tools and a platform, as Android Studio installs it. `ANDROID_HOME` names a different one.
- the NDK, for the linker. SDK Manager > SDK Tools > NDK, or `ANDROID_NDK_HOME`.
- a Java development kit, for `javac`, `d8` and `apksigner`. Android Studio bundles one, or set `JAVA_HOME`.
- `rustup target add aarch64-linux-android`.

`doctor` checks all four.

`run -d <device>` builds for that device's processor. `build apk` builds for 64-bit ARM. After launching, the app's log follows until ctrl-c, which is `adb logcat` for its process.

Still missing, because the activity does not reach them yet: the keyboard, the clipboard beyond text, and haptics. Text fields draw and scroll, but nothing can be typed into them yet. Each of these needs more of the activity's Java.

File associations are not declared either: the host does not read the intent the activity starts with, so an intent filter would list the app under Open with and then drop the file.

## Not here yet

An `ipa` upload step, a release key for Android, rebuild-on-change for `run -d web`, a MIME type definition for a Linux app's own file associations, and a disk image or notarization for a macOS app that carries helpers or libraries.
