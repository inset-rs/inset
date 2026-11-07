# Waft WinUI

WinUI controls and Fluent styling for [Waft](https://github.com/xtyxtyx/waft-ui).

The library ports control templates, theme resources and behavior from [Microsoft’s WinUI](https://github.com/microsoft/microsoft-ui-xaml). Waft handles rendering, layout and input.

## Try it

Waft **requires nightly Rust**:

```sh
rustup install nightly
cargo run -p winui_gallery
```

Run the gallery from the [Waft repository](https://github.com/xtyxtyx/waft-ui), where the kit lives.

## Use it

```toml
[dependencies]
waft-winui = "0.1.0"
```

Use controls inside an Waft `WidgetsApp`, with `ThemeScope` selecting the light or dark theme. Call `install_icon_font` once during application setup. Controls that show popups need an `Overlay` ancestor.

A `ThemeScope` can also change WinUI's theme resources for its subtree, as XAML's resource dictionaries do: `any` in both themes, then `light` or `dark`. Each resource is a field named after its XAML key.

```rust
ThemeScope::new(Theme::Light, app)
    .any(|r| r.control_corner_radius = [6.0; 4])
    .dark(|r| r.text_fill_color_primary = Color::from_argb(255, 240, 240, 240))
```

`ThemeScope::overriding(child)` changes the enclosing scope's resources for one control, as `<Control.Resources>` does.

The [gallery](https://github.com/xtyxtyx/waft-ui/tree/main/examples/winui_gallery) demonstrates application setup, font registration and control state.

## Contributing

Porting guidance, tests and resource generation: [the porting-winui skill](https://github.com/xtyxtyx/waft-ui/blob/main/.agents/skills/porting-winui/SKILL.md).

## License

MIT. Microsoft’s source and the bundled icon font retain their [third-party notices](https://github.com/xtyxtyx/waft-ui/blob/main/crates/waft-winui/THIRD_PARTY.md).
