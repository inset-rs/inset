# waft-cli

`cargo waft`: run and package an Waft app for desktop, iOS, Android, and the web from one project.

```sh
cargo install waft-cli
```

## Commands

```sh
cargo waft new myapp            # a crate every host can load
cargo waft doctor               # what this machine can build for, with the fix for each gap
cargo waft devices              # what `run -d` accepts

cargo waft run                  # this desktop: the built .app on macOS, the built executable elsewhere
cargo waft run -d ios           # the booted simulator, or the newest iPhone
cargo waft run -d "iPhone 16"   # a simulator by name or id
cargo waft run -d android       # the connected Android device, or the first emulator, booted
cargo waft run -d Medium_Phone  # an emulator by AVD name, or a device by serial
cargo waft run -d web           # build, serve locally, open the default browser (`chrome` for Chrome)
cargo waft run --release        # optimized build

cargo waft build macos          # .app              build dmg
cargo waft build windows        # folder            build msi | nsis
cargo waft build linux          # folder            build deb | appimage
cargo waft build ios            # .app for a device (`--simulator` for the simulator)
cargo waft build ipa            # .ipa around the device .app
cargo waft build apk            # .apk for 64-bit ARM, signed with the debug key
cargo waft build web            # static folder: index.html, app.js, app_bg.wasm, .br copies
```

`run` builds debug unless `--release`; `build` builds release unless `--debug`. Output lands under `target/waft/<target>/<profile>/`, so `cargo clean` removes it. In a workspace, pass `-p <package>`.

## Project shape

`new` starts from a widget kit: `--template cupertino` (the default, phone apps) or `--template winui` (desktop apps, from [waft-winui](https://github.com/xtyxtyx/waft-ui/tree/main/crates/waft-winui)). Both write:

```
myapp/
  Cargo.toml        lib (cdylib + rlib) and a bin, the kit dependency, [package.metadata.waft]
  src/lib.rs        the app, with an #[waft::main] function
  src/main.rs       the binary, calling the library's main, a GUI program on Windows; never edited
  assets/icon.png   one 1024 px PNG; every platform's sizes derive from it
```

The app is the library because the browser and Android load a library, not an executable. `#[waft::main]` emits the entry point for the compile target:

```rust
#[waft::main]
fn main(app: &mut App) {
    run_app(app, CupertinoApp::new().home(Home).into_widget());
}
```

On a desktop `run_app` asks for a window with the default configuration; `run_app_with` takes the window's title, size and the rest.

`main.rs` makes the binary a GUI program on Windows (`#![windows_subsystem = "windows"]`, which only a binary's own source can say), so no console window opens with the app; the entry `#[waft::main]` emits attaches it to the console of whatever started it, when that has one (`DefaultEmbedder::attach_to_parent_console`), so what it writes shows in the terminal it was run from. An entry written out without the macro calls that first. It is not the terminal a console program has, as it is not for a Flutter app on Windows either: cmd and PowerShell do not wait for a GUI program, so the prompt comes back at once and the app's output lands over it; Ctrl-C in that terminal, at the prompt too, or closing it, ends the app through the console's default handler, without Waft's quit flow. `cargo waft run` waits for the app, so its terminal stays the app's.

## Metadata

```toml
[package.metadata.waft]
identifier = "com.example.myapp"   # reverse-DNS bundle id; defaults to com.example.<package>
name = "My App"                    # display name; defaults to the package name
icon = "assets/icon.png"           # defaults to assets/icon.png when it exists
resources = ["assets/**"]          # globs copied into the bundle at the same relative path

[package.metadata.waft.usage_descriptions]  # why the app uses a device the system asks the user about first
camera = "Takes the photo on your badge."
microphone = "Records sound with your videos."

[package.metadata.waft.macos]
minimum_system_version = "11.0"
background_app = true              # LSUIElement: no Dock tile or menu bar
team = "ABCDE12345"                # narrows the signing identity when a machine has several
entitlements = "macos/app.entitlements"  # signed into the bundle: what the hardened runtime otherwise denies
helpers = [{ package = "my-app-tools", bin = "my-helper" }]  # other targets of the workspace the bundle carries, built with the app
libraries = [{ package = "my-app-tools", lib = "my_plugin" }]

[package.metadata.waft.ios]
team = "ABCDE12345"                # narrows the signing identity when a machine has several
minimum_version = "15.0"

[package.metadata.waft.android]
min_sdk = 31                       # the default and the least: Android 12, whose surface controls place composition layers
target_sdk = 34                    # the default; 35 and up, which Android lays out edge to edge, lay a Waft app out the same
version_code = 100                 # defaults to one derived from the version: 1.2.3 is 10203
permissions = ["android.permission.INTERNET"]  # what the app asks for besides Waft's own; a socket needs INTERNET
adaptive_icon = { foreground = "assets/icon-foreground.png", background = "#ffffff" }  # the launcher icon on API 26 and up, two 108 dp layers the launcher cuts to its shape; background is #rrggbb or a PNG; without it, `icon` is shrunk into the shape
dependencies = ["android/some-library-1.0.aar", "android/kotlin-stdlib-2.2.10.jar"]  # Android libraries merged into the package, as Gradle merges a module's; see Android libraries

[[package.metadata.waft.file_associations]]  # a kind of file this app defines
extensions = ["mydoc"]
name = "My document"
owner = true
role = "editor"                    # viewer (the default), editor, shell or none

[[package.metadata.waft.file_associations]]  # another app's kind of file this app opens
extensions = ["md", "markdown"]
name = "Markdown document"
identifier = "net.daringfireball.markdown"  # its macOS type identifier, when it has one
mime_type = "text/markdown"
```

The Android package name is the identifier as a Java package name: `com.example.my-app` becomes `com.example.my_app`.

On Linux the identifier is how the desktop knows the app's windows: `#[waft::main]` reads it from `Cargo.toml` as the app compiles, by the same default, and Waft gives it to each window as its Wayland app id and its X11 class (`LinuxSettings::app_id`); a deb's desktop entry is named after it, `<identifier>.desktop`, with `StartupWMClass` set to it, so the dock shows the app's name and icon for its windows. An entry written by hand gives it with `DefaultEmbedder::linux` with `LinuxSettings { app_id: Some(waft::linux_app_id!().to_owned()) }`. An AppImage's entry is named after the executable, as its tool links it, and carries the same `StartupWMClass`.

Version comes from `[package]`. A project-root `index.html` replaces the generated web page; keep a `<canvas id="waft">` and import `./app.js`.

### File associations

Each `file_associations` table lets the system open the app for files with those extensions.

- macOS: a document type; a type the app owns opens in it on double-click, another app's type is offered under Open With. `role` applies here only.
- Windows: the installer registers the extensions; the app gets the file as its first argument, which it reads itself.
- Linux: the desktop entry lists each `mime_type`; a type without one isn't associated. A type the app owns is declared to the system in shared-mime-info, `/usr/share/mime/packages/<identifier>.xml`: its MIME type, its `name` and a glob per extension, so the file manager names a file of it and opens it with the app. No registry names the MIME type of a type an app defines: give it one, `application/x-<extension>` where nothing else names it. The package has no scripts of its own: shared-mime-info and desktop-file-utils hold dpkg triggers on the folders it installs into, which update the system's databases whenever a package installs or removes a file there.
- Android: the activity is offered to view a `content:` or `file:` URI of each `mime_type`, and of any type whose path ends in one of the `extensions`. Android has no way for an app to teach the system a new extension, so a file of a type the app defines (`owner = true`) reaches apps as `application/octet-stream`, and the Files app's Downloads view may name a file by a number, with no extension to match: an app that owns a type is also offered for every `application/octet-stream` file, and tells its own files from others itself, by their name or their content.
- iOS and the web: not declared.

An app hears the files and links the system itself opens it with through `App::set_open_requested_handler`, at launch and while it runs on macOS, iOS and Android, and at launch on the web. On Windows and Linux the system gives the app only its command line: the app reads it, decides which arguments are files, and hands them to the same handler with `App::open_requested` (see `examples/app_events`). There a second launch is a new process; `waft-single-instance` hands its command line to the first.

### Helpers and libraries

A macOS app can carry other targets of its workspace in its bundle: `helpers`, binaries, in `Contents/Helpers/<bin>`, and `libraries`, `cdylib`s named by their `[lib] name`, in `Contents/Frameworks/lib<name>.dylib`. `build macos`, `build dmg` and `run` build each with the app's own profile, for the same machine, and put it in.

The bundle is signed inside out: each helper and library with the app's identity, under the hardened runtime when the identity is a certificate, and with no entitlements of its own, then the bundle with the app's entitlements. A disk image and notarization carry them as they carry the app. The app and its helpers find them by path from their own executable: `../Helpers/<bin>` and `../Frameworks/lib<name>.dylib`.

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

## Windows

`build windows`, `build msi` and `build nsis` link the app's icon into its executable as a resource (`src/windows_resources.rs`): its sizes from 16 to 256 pixels, as the compiled resource file `rc.exe` would make, written by the CLI and passed to the linker for the app's own crate alone (`cargo rustc -- -Clink-arg=…`), so neither a resource compiler nor a build script is needed. Explorer, the shortcuts and the taskbar show it, and Waft's host gives it to every window whose configuration names no icon, at the title bar's size and at the taskbar's. cargo-packager puts the same icon on the installer. `cargo waft run` builds and runs the same executable; `cargo run` builds one without the icon. Cargo does not track the resource file, only the argument naming it, so the file is named after a hash of its content: a changed icon is a new argument, and the executable links again.

## Tools it fetches

`wasm-bindgen` at the version the app's Cargo.lock resolved, and `wasm-opt` for release web builds, are downloaded once into `~/.cache/waft` (`WAFT_CACHE_DIR` overrides). Everything else is cargo, rustup, and the Xcode command line tools.

## Android

The `.apk` is built without Gradle and without any Java source of your own; prebuilt Android libraries are merged in (see [Android libraries](#android-libraries)). The Java is Waft's seven classes (`android/rs/waft/`), which `javac` and `d8` compile into the package's `classes.dex`:

- `rs.waft.WaftActivity`, the activity: Android's `NativeActivity`, which loads your library, where Waft's host starts your app's thread with the process's first activity and calls the `android_main` that `#[waft::main]` emits once for the process, each activity a view of the app, with the results of the activities your app starts, the intents that reach it while it runs and the user's answers to the permissions it asks for handed on to it, as the file dialogs, being opened and notifications need.
- `rs.waft.WaftDocumentActivity`, the same activity in a task of its own, a card in Recents, which the app starts for each view it asks for (`Platform::request_view`); the manifest declares it beside the launcher's `singleTask` activity, not exported, `standard` and `intoExisting`, Recents keeping 50 of its tasks.
- `rs.waft.WaftTask`, which describes a task's card, its label and icon, and brings a task forward or removes it.
- `rs.waft.WaftFileProvider`, a content provider at `<package>.waft.files`, not exported, through which the opener shares a file of the app's with the app that opens it.
- `rs.waft.WaftNotificationReceiver`, a broadcast receiver, not exported, which builds and shows the app's notifications, and shows a scheduled one when its alarm goes off, whether or not the app runs.
- `rs.waft.WaftTextInput`, the view the activity adds beside its own, which the system's keyboards type into a text field through.
- `rs.waft.WaftWindowInsets`, the listener the activity sets on its window, which tells your app what the system's bars and the keyboard cover of it: `MediaQuery`'s padding and view insets.

The manifest's `<queries>` names viewing `http`, `https` and `mailto` links, so the opener can say whether some app takes one; the application's meta-data `rs.waft.queried_schemes` lists the same schemes for it. It asks for `POST_NOTIFICATIONS`, which Android 13 asks the user for when the app asks (`NotificationPermission::request`), and `SCHEDULE_EXACT_ALARM`, which Android 14 leaves to the user to grant under the app's Alarms & reminders setting: without it a scheduled notification may come late. A scheduled notification does not survive a restart of the device.

An app runs on Android 12 (API 31) and later, which `min_sdk` defaults to and may not go below: Waft's host places composition layers with the surface-control calls Android 12 brought, and links them, so its library does not load on an earlier version.

You need:

- the Android SDK, with build tools and a platform, as Android Studio installs it. `ANDROID_HOME` names a different one.
- the NDK, for the linker. SDK Manager > SDK Tools > NDK, or `ANDROID_NDK_HOME`.
- a Java development kit, for `javac`, `d8` and `apksigner`. Android Studio bundles one, or set `JAVA_HOME`.
- `rustup target add aarch64-linux-android`.

`doctor` checks all four.

### Android libraries

`dependencies` lists Android libraries, each an `.aar` or a `.jar` relative to the package root, which the package carries as a Gradle build carries an app's `implementation files(…)`. They are local files: the app gathers each library and the libraries it needs itself (a library written in Kotlin needs `kotlin-stdlib`'s JAR, which a Gradle app gets from its Kotlin plugin), and the first listed has the highest priority.

What is merged, from an AAR:

- `classes.jar` and `libs/*.jar`: dexed by `d8` with Waft's classes, and on `javac`'s class path. Past 65,536 methods the dex is split, `classes2.dex` and on.
- `res/`: compiled and linked with the app's resources beneath them, the app's winning a resource both define, then the library listed first. An R class is generated for each library's package from the linked resources (`aapt2 link --extra-packages`), so a library's classes read its resources by `R`.
- `AndroidManifest.xml`: merged into Waft's by Android's merge rules ("Merge multiple manifest files" on developer.android.com) for `<uses-permission>`, `<uses-permission-sdk-23>`, `<permission>`, `<uses-feature>` (`android:required` merged by *or*), `<queries>` and its `<package>`, `<intent>` and `<provider>`, `<application>`'s attributes and its `<activity>`, `<activity-alias>`, `<service>`, `<receiver>`, `<provider>`, `<meta-data>`, `<uses-library>` and `<uses-native-library>`, with a component's `<intent-filter>`s and `<meta-data>`. An element is matched by its `android:name` (or is the one of its parent), an attribute it lacks is added, `${applicationId}` is the app's package name, a relative class name (`.Receiver`) is under the library's package, and lint's `tools:` attributes are dropped. A library's `<uses-sdk>` is checked, not merged: its `minSdkVersion` may not be above `min_sdk`, and one that targets an old level adds the permissions Android's table of implicit system permissions gives (a manifest with no `<uses-sdk>` targets API 1, so it adds `WRITE_EXTERNAL_STORAGE` and `READ_PHONE_STATE`).
- `jni/<abi>/*.so`: the package's processor's, beside the app's library under `lib/<abi>/`.
- `META-INF/com/android/build/gradle/aar-metadata.properties`: checked as Gradle checks it.

A JAR brings classes alone. Every JAR's Java resources, the files beside its classes, are packaged at the package's root with Gradle's default exclusions (`META-INF/MANIFEST.MF`, licences, signatures and the like) and its default merges (`META-INF/services/**` joined).

Ignored, as nothing reads them in a package built without Gradle: `R.txt` and `public.txt` (the R class comes from the linked resources), `proguard.txt` (nothing shrinks the classes), `lint.jar`, `annotations.zip`, `api.jar`, `aidl/`, `prefab/` and `baseline-prof.txt`.

Refused, with an error that names the library and what it holds: an AAR's `assets/` and `navigation.json`, any entry not in the AAR format, an AAR without a manifest; a manifest element outside the list above, a `tools:` merge rule marker (`tools:node="remove"`, `tools:replace`, …), a placeholder other than `${applicationId}`, an element without its `android:name`, two values for one attribute, a `minSdkVersion` above the app's; a library with native libraries but none for the package's processor; two files at one path in the package, as Gradle refuses them ("2 files found with path"); two AARs of one package; one listed twice; an AAR whose metadata asks for a newer AAR format, a later platform than the SDK's newest, or core library desugaring.

`examples/android_library` calls an AAR's class through JNI and shows what it read of the library's resources, manifest, native library and the JAR it needs.

`run -d <device>` builds for that device's processor. `build apk` builds for 64-bit ARM. After launching, the app's log follows until ctrl-c, which is `adb logcat` for its process.

The package carries a stripped copy of your library. A release apk's has no symbols (`llvm-strip --strip-all`). A debug apk's has no debug information (`llvm-strip --strip-debug`), which is most of a debug library and read by nothing on the device, but keeps the symbol table and the unwind tables, so a panic's backtrace in the log still names its functions. The library as built, with everything, stays at `target/<triple>/<profile>/lib<name>.so` (`target/aarch64-linux-android/debug/` for a debug `build apk`), and the NDK's tools read files and lines from it. `llvm-addr2line -f -C -e` it with an address in the library: a crash report's `pc`, or a backtrace's address less the start of the library's executable mapping in `/proc/<pid>/maps`. `ndk-stack -sym <folder>` symbolises a native crash's report from `adb logcat`, given a folder that holds both the library and the `.apk`: the library is loaded straight from the package, so the report names the `.apk` and an offset into it, which `ndk-stack` reads the library's name from.

Still missing, because the activity does not reach them yet: the clipboard beyond text, and haptics. Each of these needs more of the activity's Java.

## Not here yet

An `ipa` upload step, a release key for Android, and rebuild-on-change for `run -d web`.
