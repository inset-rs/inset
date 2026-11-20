# Build an app with Waft

Do each step yourself; ask the user only where a step says so.

1. **Tools:** Waft apps use nightly Rust (`#![feature(arbitrary_self_types)]`, pinned automatically by the `rust-toolchain.toml` that `cargo waft new` writes). Install the CLI with `cargo install waft-cli` and run `cargo waft doctor` to check which platforms this machine can build for.
2. **Scaffold:** `cargo waft new <name> --template winui` for a desktop, web, or cross-platform app, or `cargo waft new <name> --template cupertino` for a mobile-first iOS/Android app. The app lives in `src/lib.rs` (built as both `cdylib` and `rlib`), with a thin desktop entry in `src/main.rs`.
3. **Read the guides:** Read the Waft documentation at https://waft.rs/docs (or fetch https://waft.rs/llms-full.txt for all pages in one Markdown document):
   - `architecture` (https://waft.rs/docs/architecture.md): how an app is structured into **Logic** (plain Rust), **State** (`Entity<T>`), and **UI** (thin `StatelessWidget`s).
   - `state` (https://waft.rs/docs/state.md) and `handles` (https://waft.rs/docs/handles.md): `Entity<T>` vs `Ref<T>` and `Handle<T>`, and the borrow rule (`let val = { entity.read(app).field.clone() };` before the next `&mut App` call).
   - `kits` (https://waft.rs/docs/kits.md), `layout` (https://waft.rs/docs/layout.md), `text-input` (https://waft.rs/docs/text-input.md), `pages` (https://waft.rs/docs/pages.md), and `shortcuts` (https://waft.rs/docs/shortcuts.md).
   - Full Rust API docs are on docs.rs: `waft`, `waft-winui`, `waft-cupertino`, `waft-material`, `waft-markdown`, `waft-test`.
4. **Architecture & testing first:** Keep business rules in plain Rust structs and app state in `Entity<T>` handles (`app.new_entity(|_cx| ...)`). Write unit tests for your Logic and State layers using `AppCell::new()` and `waft-test` (`https://waft.rs/docs/testing.md`) and run `cargo test` — no display is required.
5. **Icon & metadata:** Configure `[package.metadata.waft]` in `Cargo.toml` (`identifier`, `name`, `icon = "assets/icon.png"`, `resources`, `file_associations`, `usage_descriptions`). Replace `assets/icon.png` with a clean 1024×1024 RGBA PNG.
6. **Run & package:**
   - Run locally: `cargo waft run` (desktop), `cargo waft run -d ios`, `cargo waft run -d android`, or `cargo waft run -d web`.
   - Build a distributable: `cargo waft build <target>` (`macos`, `dmg`, `windows`, `msi`, `nsis`, `linux`, `deb`, `appimage`, `ios`, `ipa`, `apk`, `web`, or `wasm`). See https://waft.rs/docs/cli.md for signing and platform details.
