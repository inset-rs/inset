# Value tweens

Dart's tweens are objects the collector frees. Here `Tween`, `CurveTween` and `ConstantTween` are arena objects (`Tween::new(app, ..)` returns a `Handle`), and nothing frees one that is dropped. Which form a Dart tween takes depends on whether anything writes it after it is made.

- **Value**: a `static final` or `const` tween, or one a build makes and only drives. Use `waft_animation`'s `ValueTween`, `ValueCurveTween` or `ValueConstantTween`: a `Copy`/`Clone` value implementing `Animatable`, built without the `App`, so a `const` or a per-call function can hold it and a driven animation keeps its own copy.
- **Arena**: a tween whose `begin`, `end` or `curve` is written after it is driven (an implicitly animated widget's, `ExpansionTile`'s colour tweens). The state that makes it owns it and destroys it in `dispose`.

```rust
// Dart: static final Animatable<double> _fadeInTransition = Tween<double>(begin: 0.0, end: 1.0)
//           .chain(CurveTween(curve: const Interval(0.0, 0.6)));
fn fade_in_transition() -> impl Animatable<f64> + Clone + 'static {
    ValueTween::new(0.0, 1.0).chain(ValueCurveTween::new(Rc::new(Interval::new(0.0, 0.6, Curves::linear()))))
}
```

A value tween frees nothing of what `drive` / `animate` makes from it: the driven animation is an arena object. The state that drives it keeps it and destroys the one it replaces (see `ExpansionTileState::build_icon`).

A stateless build that makes an animation (Dart's `ReverseAnimation(secondaryAnimation)` or `tween.animate(animation)` in a `build` or a builder callback) frees it at once with `app.destroy_lingering`, once it has handed it on; whoever holds it across frames retains it (`App::retain_id`), as `AnimatedState`, `RenderAnimatedOpacity`, `ProxyAnimation`'s parent, `CurvedAnimation`'s parent, a driven animation's parent and `DualTransitionBuilderState` do, and releases it when it moves off it. `ReverseAnimation` does not retain its parent, so whoever makes the animation it reverses keeps that one (`ModalRoute::build_flexible_transitions`'s proxy). See `page_transitions_theme.rs`'s `snapshot_aware_delegated_transition`.

A closure that reads such an animation across frames (an `AnimatedBuilder`'s or a `LayoutBuilder`'s builder) is a holder too: it captures a `RetainedHandle` (`app.retain(animation)`) made before the build frees its own handle, reads the animation through it, and gives it back when the closure is dropped with the widget that held it. See `page_transitions_builder.rs`'s `OpenUpwardsPageTransitionState::build_for_constraints`. A build that hands every animation it drives straight to a holder can drive through a helper that frees it as it hands it on (`nav_bar.rs`'s `drive_handed_on`).

A type with no `TweenLerp` impl (a waft-painting type: waft-painting cannot name waft-animation) keeps a private value tween of its own in the same shape (`sheet.rs`'s `BorderRadiusGeometryTween`), as `implicit_animations.rs`'s and `waft-rendering`'s Flutter-named value tweens do.
