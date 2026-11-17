# AnyEq

Dart passes an arbitrary `Object` where it only compares it with `==` and hashes it: a hero's tag, an expansion panel radio's value. In Rust that position is `Rc<dyn AnyEq>` (`waft_foundation::AnyEq`):

```rust
let tag: Rc<dyn AnyEq> = Rc::new("photo");
```

`AnyEq` is blanket-implemented for every `PartialEq + Hash + Debug + 'static` value. It compares as Dart's `==` does: an equal value of the same type only, never a value of another type. Its `Hash` mixes in the type, as Dart's `hashCode` reaches `runtimeType`, so a `HashSet` or an `IndexMap` keyed by it behaves as Dart's does.

Use it, not a new trait, whenever a parameter or field is Dart's `Object` used only this way.

Keys are not this. A key is its own family of types (`Key`, `LocalKey`, `GlobalKey`) and keeps its own trait.

The value must be `'static`. A value that borrows arena data is kept as a `Ref`, or copied.
