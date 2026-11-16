# Value tweens

Dart's `Tween`, `CurveTween` and `ConstantTween` are objects; here they are arena objects (`Tween::new(app, ..)` returns a `Ref`). A tween nobody writes to after it is made can be a value instead, which saves an arena object: `waft_animation`'s `ValueTween`, `ValueCurveTween` and `ValueConstantTween`, `Clone` values implementing `Animatable`, built without the `App`, so a `const` or a per-call function can hold one and a driven animation keeps its own copy.

- **Value**: a `static final` or `const` tween, or one a build makes and only drives.
- **Arena**: a tween whose `begin`, `end` or `curve` is written after it is driven (an implicitly animated widget's, `ExpansionTile`'s colour tweens).

```rust
// Dart: static final Animatable<double> _fadeInTransition = Tween<double>(begin: 0.0, end: 1.0)
//           .chain(CurveTween(curve: const Interval(0.0, 0.6)));
fn fade_in_transition() -> impl Animatable<f64> + Clone + 'static {
    ValueTween::new(0.0, 1.0).chain(ValueCurveTween::new(Rc::new(Interval::new(0.0, 0.6, Curves::linear()))))
}
```

What `drive` / `animate` makes from either form is an arena object, an `AnimationRef`, held like any other ([ownership.md](ownership.md)).

A type with no `TweenLerp` impl (a waft-painting type: waft-painting cannot name waft-animation) keeps a private value tween of its own in the same shape (`sheet.rs`'s `BorderRadiusGeometryTween`), as `implicit_animations.rs`'s and `waft-rendering`'s Flutter-named value tweens do.
