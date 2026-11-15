# Lazy defaults

Dart's component defaults are subclasses of the theme data, `class _DrawerDefaultsM3 extends DrawerThemeData`, that keep the context and override getters. A getter runs only when the component reads it, and `widget.x ?? theme.x ?? defaults.x` stops at the first non-null. So a default the component never reaches never reads `Theme.of`, `Directionality.of` or `CupertinoTheme`, and adds no dependency.

Waft keeps that shape:

- The theme-data file declares the getters as a trait with `theme_data_getters!` (`theme_data_getters.rs`): `DrawerThemeDataGetters`, one method per field, `fn shadow_color(&self, app: &mut App) -> Option<Color>`, each answering the field of the struct its accessor returns. The theme data implements it with its own fields.
- A defaults class is a struct named after Dart's, holding the context, Dart's `super(..)` arguments as a theme-data value (the accessor returns it), and its constructor arguments as plain fields. It implements the trait and overrides only the getters Dart overrides, each computing only its own value. A `late final` field is a `OnceCell` read through a private method.
- A component holds its defaults as `Box<dyn XGetters>` (Dart's `final XThemeData defaults = useMaterial3 ? M3(context) : M2(context)`) and reads them with `or_else`, in Dart's order:

```rust
let defaults: Box<dyn DrawerThemeDataGetters> = if use_material3 {
    Box::new(DrawerDefaultsM3::new(context))
} else {
    Box::new(DrawerDefaultsM2::new(context))
};
let elevation = self.elevation.or(drawer_theme.elevation).or_else(|| defaults.elevation(app));
```

Where Dart passes the defaults on as the theme-data type (`ButtonStyleButton.defaultStyleOf` returns a `ButtonStyle`, a layer list mixes the widget's, the theme's and the default style), the receiver takes `&dyn XGetters` or `Box<dyn XGetters>`, and the theme data goes in as itself. Only where Dart itself reads every getter (`defaults.copyWith(..)`, `style.merge(defaults)`) does `copy_with(app)` make a theme-data value from the defaults.

A getter's value that is read without the `App` later (a `WidgetStateProperty` a button resolves at paint time) is read from the defaults when the property is built; that is the one place a default is asked for before Dart would.
