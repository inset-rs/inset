# Widget construction syntax

One shape for every widget, so a tree reads like the Dart it mirrors. Syntax is a convention, not a PORTING divergence: constructors, setters, `Option`, and `into_widget()` are never `Change` entries.

## Rule

- `Widget::new(..)` takes exactly Flutter's **required** parameters, in Dart order. A required `child` is positional (`IconTheme::new(data, child)`); an optional `child` is a setter.
- Every optional or defaulted parameter, `key` included, is a fluent setter named after the Dart parameter: `.padding(..)`, `.width_factor(1.0)`, `.key(k)`. Defaults come from `new` (or `Default` when Flutter has no required parameter, and then `new()` is `Self::default()`).
- Fields stay `pub` for reading; a field and its setter share a name. Flutter's docs live on the field; the setter carries one line: `/// Dart \`Padding(child:)\`.`
- `child` / `children` setters and positional children take `impl IntoWidget<K>` (the kind tag is inferred; a `WidgetRef` converts to itself), so `.into_widget()` appears only where a `WidgetRef` is stored (a field, a variable, `run_app`).
- Named constructors are associated functions with the same shape: `SizedBox::expand().child(x)`, `CupertinoButton::filled(child, on_pressed)`.
- Data classes whose getters are methods (`CupertinoThemeData::primary_color()`) use `with_*` setters and `copy_with()` clones; that is the only `with_` prefix.

## Shape

```rust
DecoratedBox::new(BoxDecoration::new().color(CARD))
    .child(Padding::new(EdgeInsetsGeometry::all(24.0)).child(Center::new().child(button)))
    .into_widget()
```

Struct literals stay possible (fields are `pub`) but are not written in trees, tests, or examples.
