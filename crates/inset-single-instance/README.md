# inset-single-instance

One instance of an Inset app: a later launch hands its command line to the instance already running, and quits. It is opt-in, an app's own pattern rather than the platform's, as Electron's `requestSingleInstanceLock` and Tauri's single-instance plugin are.

It takes the app's id, a reverse-DNS name such as `com.example.notes`, and provides `single_instance(id)`, which answers `SingleInstance::First(FirstInstance)` for the app's only instance and `SingleInstance::Second` for a later launch, whose command line the first instance has by then. `FirstInstance::launches()` is the stream of later launches, each a `Launch` of its arguments, as the strings the system gave, unread, and the folder it was started in. Which arguments are files is the app's to say, as for its own launch: it hands the files it decides on to `App::open_requested`, where the platform's own open requests arrive too. A launch with no arguments asks the app to show itself again.

| System | How |
|---|---|
| Linux | The session bus: the first instance owns the id as its name and serves Inset's `rs.inset.SingleInstance` at the path the id names; a later launch calls its `Launch` with its arguments and folder as bytes. Not `org.freedesktop.Application`, whose `Open` takes URIs and so would make every argument a file. |
| Windows | A named mutex (`Local\`, per session) tells a later launch it is second, and a named pipe, per session and taking no remote client, carries its arguments and folder to the first. |
| macOS | Always first: macOS keeps one instance itself and hands a later open to the running app as an open request. |
| iOS, Android, the web | Always first: the system runs one instance. |

An app reads its own launch's command line, hears later launches' through `launches()`, and hears the platform's open requests, which bring the files the system itself hands over, and on macOS every later open. The crate's documentation shows all three reaching the same code.

The first instance keeps its claim while its `FirstInstance` or `LaterLaunches` lives.

Used by apps; nothing in Inset depends on it.
