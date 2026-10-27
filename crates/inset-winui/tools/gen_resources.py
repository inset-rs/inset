#!/usr/bin/env python3
"""Generates Rust theme resources from WinUI's XAML theme dictionaries.

usage: gen_resources.py [microsoft-ui-xaml root] [output .rs] [Control ...]

With no arguments, regenerates this kit from .reference/microsoft-ui-xaml using DEFAULT_CONTROLS. Explicit paths or a control subset are optional.

Reads controls/dev/CommonStyles/Common_themeresources_any.xaml (the Fluent base
tokens) and one <Control>_themeresources.xaml per named control, preferring
CommonStyles and then controls/dev/<Control>. Missing shared aliases resolve
through Common_themeresources.xaml and the base generic.xaml dictionary.
Emits one Rust struct,
`ThemeResources`, whose fields are the keys of every dictionary read, and its `new`
constructor resolving the Light and Dark (XAML "Default") dictionaries for an accent palette.
High contrast is not emitted: its values are the OS's system colours.
"""
import argparse
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

KIT_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = KIT_ROOT.parents[1]
DEFAULT_SOURCE = REPO_ROOT / ".reference/microsoft-ui-xaml"
DEFAULT_OUTPUT = KIT_ROOT / "src/theme/generated.rs"

# Control and supporting dictionaries used by the kit, in generation order.
DEFAULT_CONTROLS = (
    'Button',
    'ToggleSwitch',
    'CheckBox',
    'RadioButton',
    'ToggleButton',
    'HyperlinkButton',
    'RepeatButton',
    'Slider',
    'ToolTip',
    'TextBlock',
    'CornerRadius',
    'SplitView',
    'TabView',
    'ScrollBar',
    'NavigationView',
    'FlyoutPresenter',
    'NavigationBackButton',
    'TextBox',
    'PasswordBox',
    'CommandBarFlyout',
    'AppBarButton',
    'AcrylicBrush',
    'ProgressBar',
    'Expander',
    'InfoBar',
    'MenuFlyout',
    'DropDownButton',
    'SplitButton',
    'RadioMenuFlyoutItem',
    'ProgressRing',
    'InfoBadge',
    'MenuBar',
    'BreadcrumbBar',
    'ContentDialog',
    'NumberBox',
    'ComboBox',
    'ColorPicker',
    'TreeView',
    'SelectorBar',
)

NS = {
    'x': 'http://schemas.microsoft.com/winfx/2006/xaml',
    '': 'http://schemas.microsoft.com/winfx/2006/xaml/presentation',
}
# MenuFlyoutPresenter::UpdateTemplateSettings looks up this key in C++, not XAML.
RUNTIME_RESOURCE_REFERENCES = {'MenuFlyout': ['FlyoutThemeTouchMinWidth']}
# Templates beside a control's dictionary that its own template uses, besides `<Control>.xaml`.
COMPANION_TEMPLATES = {'ColorPicker': ['ColorSpectrum'], 'TreeView': ['TreeViewItem']}
# Styles in another control's dictionary that a control's own styles are based on: the setters
# they pass on, and the resources those name, belong to the control.
BASE_STYLES = {'TreeView': [('ListViewItem', 'DefaultListViewItemStyle')]}

X_KEY = '{%s}Key' % NS['x']
P = '{%s}' % NS['']


def local(tag):
    return tag.split('}')[-1]


def parse_color(text):
    text = text.strip()
    named = {'Transparent': (0, 0, 0, 0), 'White': (255, 255, 255, 255), 'Black': (255, 0, 0, 0)}
    if text in named:
        return named[text]
    digits = text.lstrip('#')
    if len(digits) == 6:
        digits = 'FF' + digits
    a, r, g, b = (int(digits[i:i + 2], 16) for i in range(0, 8, 2))
    return (a, r, g, b)


def resource_references(element):
    references = []
    for node in element.iter():
        if local(node.tag) == 'StaticResource' and node.get('ResourceKey'):
            references.append(node.get('ResourceKey'))
        for value in node.attrib.values():
            references.extend(re.findall(r'\{(?:StaticResource|ThemeResource) (\w+)\}', value))
    return references


def inherited_setters(dictionary, style_key, templates):
    """Returns the setters of the style `style_key` in `dictionary` that the styles in `templates`
    based on it inherit: those they set no value of, besides `Template`."""
    style = next(element for element in dictionary.iter(P + 'Style') if element.get(X_KEY) == style_key)
    based_on = re.compile(r'\{(?:StaticResource|ThemeResource) %s\}' % re.escape(style_key))
    overridden = {'Template'}
    for template in templates:
        for derived in template.iter(P + 'Style'):
            if based_on.fullmatch(derived.get('BasedOn', '').strip()):
                overridden.update(setter.get('Property') for setter in derived.findall(P + 'Setter'))
    return [setter for setter in style.findall(P + 'Setter') if setter.get('Property') not in overridden]


def control_source(root, control):
    filename = f'{control}_themeresources.xaml'
    candidates = (root / 'controls/dev/CommonStyles' / filename,
                  root / 'controls/dev' / control / filename)
    for path in candidates:
        if path.is_file():
            return path
    nested = list((root / 'controls/dev').rglob(filename))
    if len(nested) == 1:
        return nested[0]
    raise FileNotFoundError(f'No unique resource dictionary for {control}: {candidates}, {nested}')


def read_dictionaries(path, keys=None):
    """Returns {theme: {key: (kind, value)}} for one resource file."""
    root = ET.parse(path).getroot()
    # Legacy generic.xaml contains controls we do not port. Read just the
    # requested resources and their alias dependencies when used as a fallback.
    if keys is not None:
        keys = set(keys)
        while True:
            dependencies = set()
            for element in root.iter():
                if element.get(X_KEY) in keys:
                    dependencies.update(resource_references(element))
            if dependencies <= keys:
                break
            keys.update(dependencies)
    themes = {}
    for theme_dict in root.iter(P + 'ResourceDictionary'):
        key = theme_dict.get(X_KEY)
        if key not in ('Default', 'Light'):
            continue
        entries = themes.setdefault(key, {})
        for element in theme_dict:
            name = element.get(X_KEY)
            if name is None or (keys is not None and name not in keys):
                continue
            kind = local(element.tag)
            if kind == 'StaticResource':
                entries[name] = ('alias', element.get('ResourceKey'))
            elif kind == 'SolidColorBrush':
                color = element.get('Color', '')
                match = re.match(r'\{(?:StaticResource|ThemeResource) (\w+)\}', color)
                entry = ('alias', match.group(1)) if match else ('color', parse_color(color))
                opacity = float(element.get('Opacity', '1'))
                entries[name] = ('opacity', (entry, opacity)) if opacity != 1 else entry
            elif kind == 'Color':
                entries[name] = ('color', parse_color(element.text))
            elif kind == 'AcrylicBrush':
                entries[name] = ('acrylic', {
                    'tint_color': element.get('TintColor', 'White'),
                    'tint_opacity': float(element.get('TintOpacity', '0.5')),
                    'fallback_color': element.get('FallbackColor', 'Transparent'),
                    'tint_luminosity_opacity': element.get('TintLuminosityOpacity'),
                })
            elif kind == 'LinearGradientBrush':
                stops = []
                for stop in element.iter(P + 'GradientStop'):
                    color = stop.get('Color')
                    match = re.fullmatch(r'\{(?:StaticResource|ThemeResource) (\w+)\}', color)
                    stops.append((float(stop.get('Offset')), match.group(1) if match else ('color', parse_color(color))))
                entries[name] = ('gradient', stops)
            else:
                # Sizes inside a theme dictionary that every theme agrees on
                # are emitted once, as constants.
                plain = plain_value(kind, (element.text or '').strip())
                if plain is not None:
                    entries[name] = plain
    # Values outside the theme dictionaries apply to every theme.
    shared = {}
    for element in root:
        name = element.get(X_KEY)
        if name is None or (keys is not None and name not in keys):
            continue
        plain = plain_value(local(element.tag), (element.text or '').strip())
        if plain is not None:
            shared[name] = plain
    # A size both themes define identically is a constant too.
    light, dark = themes.get('Light', {}), themes.get('Default', {})
    for name, value in light.items():
        if value[0] in ('f64', 'i32', 'thickness', 'corner', 'duration_ms', 'duration_ns', 'spline', 'enum', 'font_family') and dark.get(name) == value:
            shared.setdefault(name, value)
    return themes, shared


def plain_value(kind, text):
    if kind == 'Double':
        return ('f64', float(text))
    if kind == 'Int32':
        return ('i32', int(text))
    if kind == 'GridLength' and re.match(r'^\s*[\d.]+\s*$', text):
        # A length in pixels, emitted as its number; `Auto` and star lengths are layout, not sizes.
        return ('f64', float(text))
    if kind == 'Thickness':
        parts = [float(v) for v in text.split(',')]
        if len(parts) == 1:
            parts = parts * 4
        elif len(parts) == 2:
            parts = [parts[0], parts[1], parts[0], parts[1]]
        return ('thickness', parts)
    if kind == 'CornerRadius':
        parts = [float(v) for v in text.split(',')]
        return ('corner', parts if len(parts) == 4 else parts * 4)
    if kind == 'String' and re.match(r'\d\d:\d\d:\d\d', text):
        h, m, s = text.split(':')
        nanoseconds = round((float(s) + float(m) * 60 + float(h) * 3600) * 1_000_000_000)
        return ('duration_ms', nanoseconds // 1_000_000) if nanoseconds % 1_000_000 == 0 else ('duration_ns', nanoseconds)
    if kind == 'String' and re.match(r'^[\d., -]+$', text):
        return ('spline', [float(v) for v in text.split(',')])
    if kind == 'FontFamily':
        return ('font_family', text)
    if kind in ('BackgroundSizing', 'FontWeight'):
        # Enum-valued resources: the Rust enum shares the XAML names.
        return ('enum', (kind, text.strip()))
    return None


ACCENT = ('SystemAccentColor', 'SystemAccentColorLight1', 'SystemAccentColorLight2', 'SystemAccentColorLight3',
          'SystemAccentColorDark1', 'SystemAccentColorDark2', 'SystemAccentColorDark3')


def resolve(entries, base, name, seen=()):
    entry = entries.get(name) or base.get(name)
    if entry is None:
        # The accent palette comes from the OS at run time; it is an input of
        # `for_theme`, not a literal.
        return ('accent', name) if name in ACCENT else None
    kind, value = entry
    if kind == 'alias':
        if value in seen:
            return None
        return resolve(entries, base, value, seen + (value,))
    if kind == 'opacity':
        color, opacity = value
        if color[0] == 'alias':
            if color[1] in seen:
                return None
            color = resolve(entries, base, color[1], seen + (color[1],))
        return ('opacity', (color, opacity)) if color is not None else None
    if kind == 'gradient':
        return ('gradient', [(offset, key if isinstance(key, tuple) else resolve(entries, base, key)) for offset, key in value])
    if kind == 'acrylic':
        value = dict(value)
        for field in ('tint_color', 'fallback_color'):
            match = re.fullmatch(r'\{(?:StaticResource|ThemeResource) (\w+)\}', value[field])
            value[field] = resolve(entries, base, match.group(1), seen) if match else ('color', parse_color(value[field]))
            if value[field] is None:
                return None
        return ('acrylic', value)
    return entry


def snake(name):
    out = re.sub(r'(?<=[a-z0-9])(?=[A-Z])', '_', name).lower()
    return out


def color_literal(color):
    a, r, g, b = color
    return f'Color::from_argb({a}, {r}, {g}, {b})'


def accent_field(name):
    return {'SystemAccentColor': 'accent.base', 'SystemAccentColorLight1': 'accent.light1',
            'SystemAccentColorLight2': 'accent.light2', 'SystemAccentColorLight3': 'accent.light3',
            'SystemAccentColorDark1': 'accent.dark1', 'SystemAccentColorDark2': 'accent.dark2',
            'SystemAccentColorDark3': 'accent.dark3'}[name]


def value_literal(entry):
    kind, value = entry
    if kind == 'opacity':
        color, opacity = value
        return ('{ let color = ' + value_literal(color) + '; '
                f'color.with_values(Some(color.a * {opacity}), None, None, None, None) }}')
    return accent_field(value) if kind == 'accent' else color_literal(value)


def acrylic_literal(value):
    luminosity = value['tint_luminosity_opacity']
    luminosity = f'Some({float(luminosity)})' if luminosity is not None else 'None'
    return ('AcrylicBrushResources { '
            f'tint_color: {value_literal(value["tint_color"])}, '
            f'tint_opacity: {value["tint_opacity"]}, '
            f'tint_luminosity_opacity: {luminosity}, '
            f'fallback_color: {value_literal(value["fallback_color"])} }}')


# Fonts Windows supplies, as the Rust names of the kit's open stand-ins for them (`PORTING.md`).
FONT_STAND_INS = {'XamlAutoFontFamily': 'FONT_FAMILY'}
FONT_WEIGHTS = {'Normal': 'NORMAL', 'Bold': 'BOLD', 'SemiBold': 'W600', 'Light': 'W300', 'Medium': 'W500',
                'Black': 'W900', 'Thin': 'W100', 'ExtraLight': 'W200', 'ExtraBold': 'W800'}


def themed_value(light, dark):
    """The Rust type and the light and dark literals of a brush or colour, or None for a size,
    duration or spline, which the themes share."""
    kinds = {light[0], dark[0]}
    if kinds <= {'color', 'accent', 'opacity'}:
        return 'Color', value_literal(light), value_literal(dark)
    if kinds == {'gradient'}:
        stops = lambda g: '[' + ', '.join(f'({o}, {value_literal(c)})' for o, c in g if c) + ']'
        return '[(f64, Color); 2]', stops(light[1]), stops(dark[1])
    if kinds == {'acrylic'}:
        return 'AcrylicBrushResources', acrylic_literal(light[1]), acrylic_literal(dark[1])
    return None


def shared_value(kind, value):
    """The Rust type and literal of a value both themes share, or None and why it has none: a
    font the kit has no stand-in for, or a font weight Inset names no constant for (it names only
    the hundreds, so not SemiLight's 350)."""
    if kind == 'font_family':
        if value not in FONT_STAND_INS:
            return None, f'the font {value}, which the kit has no stand-in for'
        return "&'static str", FONT_STAND_INS[value]
    if kind == 'f64':
        return 'f64', f'{value}'
    if kind == 'i32':
        return 'i32', f'{value}'
    if kind in ('thickness', 'corner', 'spline'):
        return '[f64; 4]', f'{value}'
    if kind == 'duration_ms':
        return 'Duration', f'Duration::from_millis({value})'
    if kind == 'duration_ns':
        return 'Duration', f'Duration::from_nanos({value})'
    enum, variant = value
    # `FontWeight` in Inset is Flutter's: `FontWeight::NORMAL`, `BOLD`, `W600`...
    if enum == 'FontWeight':
        if variant not in FONT_WEIGHTS:
            return None, f'FontWeight {variant}, which Inset has no constant for'
        variant = FONT_WEIGHTS[variant]
    return enum, f'{enum}::{variant}'


class ThemeResources:
    """Every resource of the dictionaries read, in one struct whose fields are WinUI's keys, as
    WinUI merges the dictionaries into one: the brushes and colours each theme states, and the
    sizes, durations and weights both share. A key two dictionaries resolve differently keeps the
    first and is reported."""

    def __init__(self):
        self.fields = {}
        self.sections = []

    def add_dictionary(self, source, keys, themes, base_themes, shared):
        """Adds the resources `source` states for `keys` in each theme, resolved through
        `base_themes`, and the values in `shared`."""
        section = []
        for key in keys:
            light = resolve(themes.get('Light', {}), base_themes.get('Light', {}), key)
            dark = resolve(themes.get('Default', {}), base_themes.get('Default', {}), key)
            if light is None or dark is None:
                missing = ', '.join(theme for theme, value in (('Light', light), ('Default', dark)) if value is None)
                print(f'warning: {source}: unresolved resource {key} ({missing}); not emitted', file=sys.stderr)
                continue
            value = themed_value(light, dark)
            if value is None:
                if light != dark:
                    # A size, duration or spline each theme states differently has no
                    # shared value to be emitted as.
                    print(f'warning: {source}: {key} differs by theme ({light[1]} light, {dark[1]} dark); not emitted', file=sys.stderr)
                continue
            self.add(section, source, key, *value)
        for key, (kind, value) in shared.items():
            ty, literal = shared_value(kind, value)
            if ty is None:
                print(f'warning: {source}: {key} is {literal}; not emitted', file=sys.stderr)
                continue
            self.add(section, source, key, ty, literal, literal)
        self.sections.append((source, section))

    def add(self, section, source, key, ty, light, dark):
        field = snake(key)
        known = self.fields.get(field)
        if known is None:
            self.fields[field] = (key, ty, light, dark, source)
            section.append(field)
        elif known[1:4] != (ty, light, dark):
            print(f'warning: {source}: {key} differs from {known[4]}; the first is kept', file=sys.stderr)

    def emit(self, out):
        out.append('/// Every WinUI theme resource the kit uses, by its key, for one theme and accent palette: the')
        out.append("/// brushes and colours of WinUI's Light and Dark (XAML `Default`) dictionaries, and the sizes,")
        out.append('/// durations and weights both share.')
        out.append('#[derive(Clone, Debug, PartialEq)]')
        out.append('pub struct ThemeResources {')
        out.append('    /// The dictionary the brushes and colours come from.')
        out.append('    pub theme: Theme,')
        out.append('    /// The `SystemAccentColor*` shades the accent brushes come from.')
        out.append('    pub accent: AccentPalette,')
        for source, section in self.sections:
            if not section:
                continue
            out.append(f'    // `{source}`')
            for field in section:
                key, ty, _, _, _ = self.fields[field]
                out.append(f'    /// `{key}`.')
                out.append(f'    pub {field}: {ty},')
        out.append('}')
        out.append('')
        out.append('impl ThemeResources {')
        out.append("    /// WinUI's resources for `theme`, in the `accent` palette.")
        out.append('    pub fn new(theme: Theme, accent: AccentPalette) -> ThemeResources {')
        out.append('        ThemeResources {')
        out.append('            theme,')
        out.append('            accent,')
        for field, (_, _, light, dark, _) in self.fields.items():
            if light == dark:
                out.append(f'            {field}: {light},')
            else:
                out.append(f'            {field}: match theme {{ Theme::Light => {light}, Theme::Dark => {dark} }},')
        out.append('        }')
        out.append('    }')
        out.append('}')


def gather(root, controls):
    """The resources of WinUI's base dictionaries and of each of `controls`, in order, from the
    WinUI checkout at `root`."""
    styles = root / 'controls/dev/CommonStyles'
    base_themes, base_shared = read_dictionaries(styles / 'Common_themeresources_any.xaml')
    common_themes, common_shared = read_dictionaries(styles / 'Common_themeresources.xaml')
    base_shared = {**base_shared, **common_shared}
    acrylic_themes, _ = read_dictionaries(root / 'controls/dev/Materials/Acrylic/AcrylicBrush_themeresources.xaml')
    # Brush keys alias a colour rather than stating one, so the alias kind belongs
    # with the base resources as much as a literal colour does.
    common_keys = [k for k, v in base_themes['Light'].items() if v[0] in ('color', 'gradient', 'alias', 'opacity')]
    resources = ThemeResources()
    resources.add_dictionary('Common_themeresources_any.xaml', common_keys, base_themes, {}, base_shared)
    sources = [(control, control_source(root, control)) for control in controls]
    # Control dictionaries share named resources in WinUI's merged theme dictionary.
    # A sibling's Fluent definition must take precedence over generic.xaml fallback.
    control_themes = {'Light': {}, 'Default': {}}
    for _, source in sources:
        themes, _ = read_dictionaries(source)
        for theme in control_themes:
            control_themes[theme].update(themes.get(theme, {}))
    declared = set(base_shared)
    for _, source in sources:
        declared.update(read_dictionaries(source)[1])
    emitted = set(base_shared)
    for control, source in sources:
        themes, shared = read_dictionaries(source)
        templates = [ET.parse(source).getroot()]
        for name in [control, *COMPANION_TEMPLATES.get(control, [])]:
            companion = source.with_name(f'{name}.xaml')
            if companion.is_file():
                templates.append(ET.parse(companion).getroot())
        references = [key for template in templates for key in resource_references(template)]
        references.extend(RUNTIME_RESOURCE_REFERENCES.get(control, []))
        inherited_themes, inherited_shared = {'Light': {}, 'Default': {}}, {}
        for dictionary, style_key in BASE_STYLES.get(control, []):
            dictionary_source = control_source(root, dictionary)
            setters = inherited_setters(ET.parse(dictionary_source).getroot(), style_key, templates)
            inherited = [key for setter in setters for key in resource_references(setter)]
            references.extend(inherited)
            dictionary_themes, dictionary_shared = read_dictionaries(dictionary_source, inherited)
            for theme in inherited_themes:
                inherited_themes[theme].update(dictionary_themes.get(theme, {}))
            inherited_shared.update({key: value for key, value in dictionary_shared.items() if key in inherited})
        legacy_themes, legacy_shared = read_dictionaries(root / 'dxaml/xcp/dxaml/themes/generic.xaml', references)
        shared = {**{key: value for key, value in legacy_shared.items() if key not in declared}, **inherited_shared, **shared}
        shared = {key: value for key, value in shared.items() if key not in emitted}
        emitted.update(shared)
        fallback = {theme: {**legacy_themes.get(theme, {}), **common_themes.get(theme, {}), **acrylic_themes.get(theme, {}), **inherited_themes[theme], **base_themes.get(theme, {}), **control_themes.get(theme, {})}
                    for theme in ('Light', 'Default')}
        keys = list(themes.get('Light', {}).keys())
        # Expose system brushes a template uses directly, as well as its named
        # resources; the base dictionary's own tokens are already there.
        for key in references:
            if key not in keys and key not in base_themes.get('Light', {}) and (key in legacy_themes.get('Light', {}) or key in control_themes.get('Light', {}) or key in inherited_themes['Light']):
                keys.append(key)
        resources.add_dictionary(source.name, keys, themes, fallback, shared)
    return resources


def main(argv=None):
    parser = argparse.ArgumentParser(description="Generate the kit’s Rust theme resources from WinUI XAML.")
    parser.add_argument('root', nargs='?', type=Path, default=DEFAULT_SOURCE,
                        help='WinUI source checkout (default: .reference/microsoft-ui-xaml)')
    parser.add_argument('output', nargs='?', type=Path, default=DEFAULT_OUTPUT,
                        help='generated Rust file (default: this kit’s theme/generated.rs)')
    parser.add_argument('controls', nargs='*',
                        help='optional subset; defaults to the dictionaries in DEFAULT_CONTROLS')
    args = parser.parse_args(argv)
    resources = gather(args.root, args.controls or DEFAULT_CONTROLS)
    out = ['//! Generated by tools/gen_resources.py from microsoft-ui-xaml; do not edit by hand.',
           '//! Colours are ARGB literals resolved through every StaticResource alias in the',
           '//! XAML theme dictionaries; the XAML "Default" dictionary is the dark theme.',
           '#![allow(clippy::excessive_precision)]',
           'use super::{AccentPalette, AcrylicBrushResources, FONT_FAMILY, Theme};', 'use crate::BackgroundSizing;', 'use inset_embedder::{Color, FontWeight};', 'use std::time::Duration;', '']
    resources.emit(out)
    args.output.write_text('\n'.join(out) + '\n')
    subprocess.run(['rustfmt', '--edition', '2024', str(args.output)], check=True)
    print(f'wrote {args.output}')


if __name__ == '__main__':
    main()
