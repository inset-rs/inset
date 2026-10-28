# inset-single-instance

One instance of an Inset app: a later launch hands the files and links it was started with to the instance already running, and quits. It is opt-in, an app's own pattern rather than the platform's, as Electron's `requestSingleInstanceLock` and Tauri's single-instance plugin are.

It takes the app's id, a reverse-DNS name such as `com.example.notes`, and provides `single_instance(id)`, which answers `SingleInstance::First(FirstInstance)` for the app's only instance and `SingleInstance::Second` for a later launch, whose files and links the first instance has by then. `FirstInstance::opened()` is the stream of what later launches pass on, each an `OpenRequest` as the platform's own open requests are (`inset-embedder`'s `OpenRequest`, which `App::set_open_requested_handler` hears). A launch with nothing to open passes an empty request: the app shows itself again.

| System | How |
|---|---|
| Linux | D-Bus activation: the first instance owns the id as its name on the session bus and serves `org.freedesktop.Application` at the path the id names; a later launch calls its `Open` with `file://` URIs and links, or `Activate`. |
| Windows | A named mutex (`Local\`, per session) tells a later launch it is second, and a named pipe, per session and taking no remote client, carries its arguments and folder to the first. |
| macOS | Always first: macOS keeps one instance itself and hands a later open to the running app as an open request. |
| iOS, Android, the web | Always first: the system runs one instance. |

An app uses both streams: the platform's open requests bring the first launch's own files and links, and on macOS every later open; `opened()` brings what later launches pass on. The crate's documentation shows the two reaching the same code.

The first instance keeps its claim while its `FirstInstance` or `Opened` lives.

Used by apps; nothing in Inset depends on it.
