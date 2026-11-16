# Ownership

Dart keeps an object while something refers to it. Here an arena object lives while a `Ref<T>` keeps it, and is freed at the checkpoint after the last `Ref` drops. Nobody frees by hand. `dispose` is Dart's method, ported as it is (stop listening, stop ticking, detach): it frees nothing, and the object stays readable while it is kept. The last drop runs no `dispose`.

`Handle<T>` is the `Copy` form that keeps nothing: the method receiver (`self: Handle<Self>`), what layout and paint walk with, what a call uses for its duration. A bare `Handle` on the stack is good until the next checkpoint; anything kept past one is a `Ref`.

## What an app writer must know

1. **Store a `Ref`.** Fields of your state, fields of your widgets, anything you keep. You never free.
2. **Call `dispose` where Flutter does**, in your state's `dispose`, for what your state created.
3. **A closure you register on an object must not capture a `Ref` to that same object.** Capture `self` (the state's bare handle) and read the field inside, or use a tear-off (`Listener::handle_method`). Breaking this leaks until `dispose`; it does not crash.
4. **After an `await` or a timer, ask `mounted(app)` before using `self` or a `BuildContext`.** Flutter's own rule.

## Constructors

A constructor is `new`. One that makes an arena object takes `&mut App` and returns a `Ref`; one that makes a plain value returns the value, and takes the `App` only to read it (a Material defaults object reads `Theme::of`, `ThemeData::new` the platform). `App::create` is the arena call those constructors make. A second, named Dart constructor keeps its name (`AnimationController.unbounded` → `AnimationController::unbounded`). A tree node's constructor returns the bare handle the tree keeps (`Retained::into_tree_node`). A base class's field bag (`ModalRouteData::new(app)`) is a plain value whose fields keep the objects it makes.

## Porting a field

- **A field is a `Ref` unless it points back at what holds it** (a parent, an owner, a state's element, a route's navigator), which is a bare `Handle`. Dart's graph already has this direction; the port writes it down.
- **Tree nodes are the tree's.** An element, a state, a render object or a layer is kept by the tree code from `Retained::into_tree_node` in its constructor to `App::release_tree_node` at the end of its Flutter lifecycle. A field that names one is a bare `Handle`; a holder Dart has beside the tree (the hit-test path, the mouse tracker) keeps an `AnyRef`.
- **Typed or erased.** A field whose type is concrete is the typed `Ref<T>`: the object made it itself, or Dart's type has no subclasses. A field where Dart's type admits subclasses and the value comes from outside (a widget parameter, a callback argument) is the erased alias: `FocusNodeRef`, `AnimationRef<T>`, `ScrollControllerRef`. `EditableText.focus_node` is a `FocusNodeRef`; a state's own focus node is a `Ref<FocusNode>`.
- **A function that keeps what it is given takes a `Ref`**; one that only uses it during the call takes the bare handle. A widget is built without the `App`, so it is handed `Ref`s.

## Reading, converting, upgrading

- **`.handle()`** is the bare handle inside a `Ref` or an `Option<Ref>` (`OptionRetained::handle`). A method is called on a `Ref` directly; `app.get` takes a `Handle`, a `&Handle` or a `&Ref`.
- **A getter returns the bare handle, or `&Ref` where a caller must clone** (to keep it, or to hand it to something that keeps it).
- **To the erased form:** `.erase()` (clone) or `.into_erased()` (move), through the erased type's `From<Handle<T>>`; `.to_any()` for an `AnyRef`. There is no other conversion.
- **From a bare handle back to a `Ref`:** `handle.upgrade(app)`, on a typed or an erased handle, `None` once nothing keeps the object.

## A listener that removes itself

Dart's `late VoidCallback listener; listener = () { …; notifier.removeListener(listener); };` names itself. A closure that held its own `Listener` strongly would keep itself alive after the notifier lets it go, so it holds a `WeakListener`:

```rust
let itself: Rc<OnceCell<WeakListener>> = Rc::default();
let listener = {
    let itself = Rc::clone(&itself);
    Listener::new(move |app| {
        if let Some(listener) = itself.get().and_then(WeakListener::upgrade) {
            entry.remove_listener(app, &listener);
        }
        // …
    })
};
let _ = itself.set(listener.downgrade());
entry.add_listener(app, listener);
```

`AnimationStatusListener::downgrade` is the same for a status listener. See `RouteEntry::dispose` in `navigator.rs`.

## Finding a leak

A debug build reports Flutter's two leak kinds:

- `App::debug_take_not_disposed`: an object that expects a `dispose` (`App::expect_dispose`) and was freed without one. Today that covers a `ChangeNotifier` from its first listener, `Ticker`, `AnimationController`, `CurvedAnimation` and `RestorationBucket`. It does not cover the gesture recognizers or the image types yet.
- `App::debug_take_not_collected`: an object disposed through `App::mark_disposed` and still kept two checkpoints later, a ring of `Ref`s or a holder that forgot it. It sees only the types whose `dispose` calls `mark_disposed` (the five above, the gesture recognizers and the image types), and it skips tree nodes (elements, states, render objects, layers), which a holder such as the hit-test path keeps legitimately after the tree releases them.

Neither report sees a type with no `dispose`, such as a `ProxyAnimation` or a tween, nor a ring made only of such objects. `waft_test::object_snapshot` and `assert_no_new_objects` bracket a cycle in a test and name what it left behind, of any type, by type and creation site. The creation site is where `App::create` ran: the caller's line for a constructor marked `#[track_caller]` (waft-animation's, the `Ticker`'s, the render objects'), and the constructor's own line for the others.
