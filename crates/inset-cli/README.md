# inset-cli

`cargo inset`: run and package an Inset app for desktop, iOS, and the web from one project.

```sh
cargo install inset-cli
```

## Commands

```sh
cargo inset new myapp            # a crate every host can load
cargo inset doctor               # what this machine can build for, with the fix for each gap
cargo inset devices              # what `run -d` accepts

cargo inset run                  # this desktop, same as cargo run
cargo inset run -d ios           # the booted simulator, or the newest iPhone
cargo inset run -d "iPhone 16"   # a simulator by name or id
cargo inset run -d web           # build, serve locally, open the default browser (`chrome` for Chrome)
cargo inset run --release        # optimized build

cargo inset build macos          # .app              build dmg
cargo inset build windows        # folder            build msi | nsis
cargo inset build linux          # folder            build deb | appimage
cargo inset build ios            # .app for a device (`--simulator` for the simulator)
cargo inset build ipa            # .ipa around the device .app
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
#[inset::main(title = "My App", size = [420.0, 720.0])]
fn main(app: &mut App) {
    run_app(app, CupertinoApp::new().home(Home).into_widget());
}
```

`title` and `size` set the native window; both are optional.

## Metadata

```toml
[package.metadata.inset]
identifier = "com.example.myapp"   # reverse-DNS bundle id; defaults to com.example.<package>
name = "My App"                    # display name; defaults to the package name
icon = "assets/icon.png"           # defaults to assets/icon.png when it exists
resources = ["assets/**"]          # globs copied into the bundle at the same relative path

[package.metadata.inset.macos]
minimum_system_version = "11.0"

[package.metadata.inset.ios]
team = "ABCDE12345"                # narrows the signing identity when a machine has several
minimum_version = "15.0"
```

Version comes from `[package]`. A project-root `index.html` replaces the generated web page; keep a `<canvas id="inset">` and import `./app.js`.

## Signing

Signing happens when credentials are present, never by flag.

- iOS simulator: no account and no signing.
- iOS device: the Apple Development certificate in your keychain and a provisioning profile Xcode stored, found the way Flutter finds them; nothing to configure once you have signed in under Xcode Settings > Accounts and run any app on the device. `doctor` names the certificate that will be used; `ios.team` in the metadata picks one when a machine has several. `build ipa` needs a distribution profile and is uploaded with Transporter or `xcrun altool`.
- macOS: unsigned by default, which runs locally. Distribution needs a Developer ID and notarization, which Apple only grants against an account: `APPLE_SIGNING_IDENTITY` signs; `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, or an `APPLE_KEYCHAIN_PROFILE` stored with `xcrun notarytool store-credentials`, notarize and staple `build dmg`. These are cargo-packager's variables, so its docs apply.
- Windows: `certificate_thumbprint` or a custom sign command, through cargo-packager.

## Size

`build web` runs `wasm-opt -Os` and writes brotli copies. The release profile decides the rest. Measured on the gallery (wasm, then brotli):

| `[profile.release]`                  | wasm     | brotli   |
|--------------------------------------|----------|----------|
| defaults                             | 13.6 MB  | 2.48 MB  |
| `lto = "fat"`, `codegen-units = 1`   | 12.6 MB  | 2.34 MB  |
| `opt-level = "s"`                    | 8.9 MB   | 1.97 MB  |
| `opt-level = "z"`                    | 8.0 MB   | 1.85 MB  |

`panic = "abort"` changes nothing on wasm. `new` writes the LTO pair, which mostly buys speed; add `opt-level = "s"` when download size matters more than native speed.

## Tools it fetches

`wasm-bindgen` at the version the app's Cargo.lock resolved, and `wasm-opt` for release web builds, are downloaded once into `~/.cache/inset` (`INSET_CACHE_DIR` overrides). Everything else is cargo, rustup, and the Xcode command line tools.

## Not here yet

Android, an `ipa` upload step, and rebuild-on-change for `run -d web`.
