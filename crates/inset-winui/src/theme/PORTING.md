# inset-winui/src/theme
Audience: readers familiar with Flutter and Rust; the headings identify the corresponding WinUI code.

WinUI home: controls/dev/CommonStyles (`Common_themeresources_any.xaml`, `<Control>_themeresources.xaml`)
Ported against: aa3207e6

## mod.rs → accent palette and theme dictionaries

- Change: `AccentPalette` uses a supplied palette, defaulting to Windows 11 blue, instead of reading the user’s system accent color.
  Reason: os — the source obtains the accent shades from Windows, while this host supplies no system accent palette.
  Affect: `AccentPalette` does not follow system accent changes automatically; its default control colors are `dark1` (#0067C0) in light mode and `light2` (#4CC2FF) in dark mode, while `base`, `light1`, `light3`, `dark2` and `dark3` still need Windows verification.

- Change: `Theme` supports light and dark appearances but does not adapt to system high-contrast mode.
  Reason: os — WinUI obtains the user’s high-contrast colors from Windows, and this host does not expose that palette.
  Affect: `Theme` remains light or dark when the user enables system high contrast.

- Change: `ThemeResources` holds every key of the theme dictionaries as a field of one generated struct, and a `ThemeScope` sets fields where XAML adds a dictionary's entries: `any` for both themes, then `light` or `dark` as `ThemeDictionaries` do.
  Reason: language — Rust has no lookup of differently typed values by a string key, so each key is a typed field, resolved when the scope builds.
  Affect: Application code changes a resource by assigning its field in a scope's closure, such as `.any(|r| r.control_corner_radius = [6.0; 4])`, and cannot add keys WinUI does not define.

- Change: A nested `ThemeScope::new` starts from WinUI's values for its theme, not from the dictionaries of the scopes around it.
  Reason: framework — an Inset inherited widget hands its subtree resolved values, while XAML looks a key up through every ancestor's dictionaries in the element's own theme.
  Affect: Values set by an enclosing scope do not reach below a nested `ThemeScope::new`; set them on that scope too.

- Change: `ThemeScope::with_resources` gives a popup or drag preview its target's resources, the overrides around the target included, where a WinUI popup sees the application's resources and only its target's theme.
  Reason: framework — Inset's root overlay is outside the target's subtree, so the kit carries the target's resolved resources into it.
  Affect: The flyouts, menus, dialogs and drag previews a control opens show the values of the scope around that control.

## typography.rs → `XamlAutoFontFamily`

- Change: `content_control_theme_font_family` defaults to `FONT_FAMILY`, Selawik, instead of Segoe UI Variable for control text.
  Reason: os — Segoe ships with Windows only; Selawik is Microsoft's metric-compatible open substitute.
  Affect: Control text has different letter shapes and does not automatically use the size-specific letter shapes supplied by Segoe UI Variable.

## Deferred

- Verification of all accent shades remains deferred. Trigger: A palette table is read from a Windows host.
- High-contrast theme support remains deferred. Trigger: A host reports the system palette.
- A nested `ThemeScope::new` keeping the values of the scopes around it remains deferred. Trigger: An application changes theme inside a subtree whose resources it overrides.
