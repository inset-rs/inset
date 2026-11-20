export const helloCommands = `cargo install waft-cli
cargo waft new notes --template winui
cd notes
cargo waft run                        # this desktop
cargo waft run -d ios                 # iOS simulator
cargo waft run -d android             # Android emulator
cargo waft run -d web                 # browser
cargo waft build dmg                  # or msi, nsis, deb, appimage, ipa, apk, web, wasm`;
