# Widget construction syntax

One shape for every widget, so a tree reads like the XAML it mirrors. Syntax is a convention, not a PORTING divergence: constructors, setters, `Option`, and `into_widget()` are never `Change` entries.

## Rule

- `Widget::new(..)` takes exactly the control's **required** properties, in XAML order. A required `child` is positional (`ControlBorder::new(background, border)` then `.child(..)`; `ThemeScope::new(theme, child)`); an optional `child` is a setter.
- Every optional or defaulted parameter, `key` included, is a fluent setter named after the XAML property: `.padding(..)`, `.width_factor(1.0)`, `.key(k)`. Defaults come from `new` (or `Default` when the control has no required property, and then `new()` is `Self::default()`).
- Fields stay `pub` for reading; a field and its setter share a name. What the property means lives on the field; the setter carries one line naming the XAML property.
- `child` / `children` setters and positional children take `impl IntoWidget<K>` (the kind tag is inferred; a `WidgetRef` converts to itself), so `.into_widget()` appears only where a `WidgetRef` is stored (a field, a variable, `run_app`).
- Named constructors are associated functions with the same shape: `SizedBox::expand().child(x)`, `Button::text("OK", click)`.
- Event handlers are closures named after the XAML event: `click`, `toggled`.

## Shape

```rust
ToggleSwitch::new(is_on, move |app, on| state.set(app, on))
    .header(Text::new("Wi-Fi"))
    .is_enabled(false)
    .into_widget()
```

Struct literals stay possible (fields are `pub`) but are not written in trees, tests, or examples.
