# Futures

Dart's futures run by themselves; Rust's run only when polled.

- A host trait method returns a named boxed future, `pub type ClipboardSetTextFuture = Pin<Box<dyn Future<Output = ()>>>;`: `inset-embedder` cannot name `Task`.
- A Dart `async` method, and a callback type that returns a `Future`, return a `Task`. The body up to the first `await` runs in the call, the rest in `app.spawn`, and a body with nothing to await returns `Task::ready`. Where Dart drops the future, drop the `Task`; it still runs.
- A Dart method that hands back a future something else completes returns that future: a `Completer`'s as `CompleterFuture<T>`, such as a route's `popped`, and a `Future` type of its own, such as `TickerFuture`, as its port.

```
let written = clipboard.set_text(text); // before the first `await`: runs in the call
app.spawn(async move |_cx| written.await)
```
