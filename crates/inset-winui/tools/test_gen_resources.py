"""Run with python3 -m unittest discover -s tools -p 'test_*.py'."""
import tempfile
import unittest
import contextlib
import io
import subprocess
import sys
from pathlib import Path

import xml.etree.ElementTree as ET

from gen_resources import DEFAULT_SOURCE, DEFAULT_OUTPUT, ThemeResources, control_source, gather, inherited_setters, plain_value, read_dictionaries, resolve


def quietly(gathering):
    """`gathering`'s result, with the generator's warnings kept off the test output."""
    with contextlib.redirect_stderr(io.StringIO()):
        return gathering()


def section(resources, source):
    """The fields `source` added to `resources`, in order."""
    return next(fields for name, fields in resources.sections if name == source)


def value(resources, field):
    """The Rust type and the light and dark literals `resources` holds for `field`."""
    return resources.fields[field][1:4]


class ResourceGenerationTests(unittest.TestCase):
    def test_default_control_list_reproduces_checked_in_resources(self):
        if not DEFAULT_SOURCE.exists():
            self.skipTest('local WinUI source checkout unavailable')
        generator = Path(__file__).with_name('gen_resources.py').resolve()
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'resources.rs'
            subprocess.run(
                [sys.executable, str(generator), str(DEFAULT_SOURCE), str(output)],
                cwd=directory, check=True, capture_output=True, text=True,
            )
            self.assertEqual(output.read_bytes(), DEFAULT_OUTPUT.read_bytes())

    def test_password_box_uses_sibling_fluent_text_box_brushes(self):
        if not DEFAULT_SOURCE.exists():
            self.skipTest('local WinUI source checkout unavailable')
        # PasswordBox first, so the brushes it resolves through TextBox's dictionary are its own.
        resources = quietly(lambda: gather(DEFAULT_SOURCE, ['PasswordBox', 'TextBox']))
        password = section(resources, 'PasswordBox_themeresources.xaml')
        thickness = lambda literal: ('[f64; 4]', literal, literal)
        self.assertEqual(value(resources, 'text_control_border_theme_thickness'), thickness('[1.0, 1.0, 1.0, 1.0]'))
        self.assertEqual(value(resources, 'text_control_border_theme_thickness_focused'), thickness('[1.0, 1.0, 1.0, 2.0]'))
        self.assertEqual(value(resources, 'text_control_theme_padding'), thickness('[10.0, 5.0, 6.0, 6.0]'))
        self.assertIn('text_control_border_brush', password)
        self.assertIn('text_control_border_brush_focused', password)
        self.assertEqual(value(resources, 'text_control_border_brush')[0], '[(f64, Color); 2]')
        self.assertEqual(value(resources, 'text_control_background'),
                         ('Color', 'Color::from_argb(179, 255, 255, 255)', 'Color::from_argb(15, 255, 255, 255)'))
        self.assertIn('text_control_background', password)

    def test_control_source_prefers_common_styles_then_control_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            local = root / 'controls/dev/SplitView/SplitView_themeresources.xaml'
            common = root / 'controls/dev/CommonStyles/SplitView_themeresources.xaml'
            local.parent.mkdir(parents=True)
            common.parent.mkdir(parents=True)
            local.touch()
            self.assertEqual(control_source(root, 'SplitView'), local)
            common.touch()
            self.assertEqual(control_source(root, 'SplitView'), common)
            with self.assertRaises(FileNotFoundError):
                control_source(root, 'Missing')

    def test_legacy_filter_reads_alias_dependencies_without_unrelated_resources(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'resources.xaml'
            path.write_text('''<ResourceDictionary
                xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
                <ResourceDictionary.ThemeDictionaries>
                  <ResourceDictionary x:Key="Light">
                    <StaticResource x:Key="Overlay" ResourceKey="Brush" />
                    <SolidColorBrush x:Key="Brush" Color="{StaticResource Color}" />
                    <Color x:Key="Color">#99FFFFFF</Color>
                    <Color x:Key="UnrelatedUnsupportedColor">NotAColor</Color>
                  </ResourceDictionary>
                  <ResourceDictionary x:Key="HighContrast">
                    <Color x:Key="Color">NotAColor</Color>
                  </ResourceDictionary>
                </ResourceDictionary.ThemeDictionaries>
                </ResourceDictionary>''')
            themes, _ = read_dictionaries(path, ['Overlay'])
            self.assertEqual(set(themes['Light']), {'Overlay', 'Brush', 'Color'})
            self.assertEqual(resolve({}, themes['Light'], 'Overlay'), ('color', (153, 255, 255, 255)))
            self.assertEqual(resolve({'Color': ('color', (255, 0, 0, 0))}, themes['Light'], 'Overlay'),
                             ('color', (255, 0, 0, 0)))

    def test_animation_pre_duration_keeps_submillisecond_precision(self):
        self.assertEqual(plain_value('String', '00:00:00.19999'), ('duration_ns', 199990000))
        self.assertEqual(plain_value('String', '00:00:00.2'), ('duration_ms', 200))

    def test_a_grid_length_in_pixels_is_a_size_and_auto_or_star_is_not(self):
        self.assertEqual(plain_value('GridLength', '8'), ('f64', 8.0))
        self.assertIsNone(plain_value('GridLength', 'Auto'))
        self.assertIsNone(plain_value('GridLength', '2*'))

    def test_an_int32_is_a_count(self):
        self.assertEqual(plain_value('Int32', '15'), ('i32', 15))

    def test_acrylic_alias_preserves_recipe_and_source_fallback(self):
        source = DEFAULT_SOURCE
        if not source.exists():
            self.skipTest('local WinUI source checkout unavailable')
        themes, _ = read_dictionaries(source / 'controls/dev/Materials/Acrylic/AcrylicBrush_themeresources.xaml')
        aliases = {'ToolTipBackgroundBrush': ('alias', 'AcrylicInAppFillColorDefaultBrush')}
        dark = resolve(aliases, themes['Default'], 'ToolTipBackgroundBrush')
        self.assertEqual(dark[0], 'acrylic')
        self.assertEqual(dark[1]['fallback_color'], ('color', (255, 44, 44, 44)))
        self.assertEqual(dark[1]['tint_opacity'], 0.15)
        self.assertEqual(float(dark[1]['tint_luminosity_opacity']), 0.96)

    def test_gradient_can_mix_literal_and_resource_colors(self):
        entries = {'Border': ('gradient', [(1.0, ('color', (0, 0, 0, 0))), (1.0, 'Stroke')])}
        base = {'Stroke': ('color', (255, 1, 2, 3))}
        self.assertEqual(resolve(entries, base, 'Border'),
                         ('gradient', [(1.0, ('color', (0, 0, 0, 0))), (1.0, ('color', (255, 1, 2, 3)))]))

    def test_color_picker_reads_the_spectrum_template_beside_it(self):
        source = DEFAULT_SOURCE
        if not source.exists():
            self.skipTest('local WinUI source checkout unavailable')
        resources = quietly(lambda: gather(source, ['ColorPicker']))
        picker = section(resources, 'ColorPicker_themeresources.xaml')
        # `ColorSpectrum.xaml` strokes the selection ellipse with these.
        self.assertIn('system_control_background_chrome_white_brush', picker)
        self.assertIn('system_control_background_chrome_black_high_brush', picker)

    def test_a_derived_style_inherits_only_the_setters_it_does_not_set(self):
        namespaces = 'xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"'
        base = ET.fromstring(f'''<ResourceDictionary {namespaces}>
            <Style x:Key="BaseStyle" TargetType="ListViewItem">
              <Setter Property="Foreground" Value="{{ThemeResource BaseForeground}}" />
              <Setter Property="MinHeight" Value="{{ThemeResource BaseMinHeight}}" />
              <Setter Property="Template"><Setter.Value><ControlTemplate /></Setter.Value></Setter>
            </Style>
            </ResourceDictionary>''')
        derived = ET.fromstring(f'''<ResourceDictionary {namespaces}>
            <Style x:Key="DerivedStyle" TargetType="TreeViewItem" BasedOn="{{StaticResource BaseStyle}}">
              <Setter Property="MinHeight" Value="{{ThemeResource DerivedMinHeight}}" />
            </Style>
            </ResourceDictionary>''')
        setters = inherited_setters(base, 'BaseStyle', [derived])
        self.assertEqual([setter.get('Property') for setter in setters], ['Foreground'])

    def test_tree_view_takes_the_list_view_item_setters_its_item_style_inherits(self):
        source = DEFAULT_SOURCE
        if not source.exists():
            self.skipTest('local WinUI source checkout unavailable')
        resources = quietly(lambda: gather(source, ['TreeView']))
        tree = section(resources, 'TreeView_themeresources.xaml')
        # `DefaultListViewItemStyle`'s `Foreground`, `MinWidth` and focus visual brushes reach
        # `TreeViewItem`; its `Background` and `MinHeight` the item style sets itself.
        self.assertIn('list_view_item_foreground', tree)
        self.assertIn('list_view_item_focus_visual_primary_brush', tree)
        self.assertEqual(value(resources, 'list_view_item_min_width'), ('f64', '88.0', '88.0'))
        self.assertNotIn('list_view_item_background', tree)
        self.assertNotIn('list_view_item_min_height', resources.fields)
        # `TreeViewItem.xaml`, beside the dictionary, draws the drag count on these.
        self.assertIn('system_control_background_accent_brush', tree)

    def test_control_source_finds_companion_control_in_parent_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            companion = root / 'controls/dev/NavigationView/NavigationBackButton_themeresources.xaml'
            companion.parent.mkdir(parents=True)
            companion.touch()
            self.assertEqual(control_source(root, 'NavigationBackButton'), companion)
            duplicate = root / 'controls/dev/Other/NavigationBackButton_themeresources.xaml'
            duplicate.parent.mkdir(parents=True)
            duplicate.touch()
            with self.assertRaises(FileNotFoundError):
                control_source(root, 'NavigationBackButton')

    def test_legacy_filter_preserves_requested_shared_dimensions(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'resources.xaml'
            path.write_text('''<ResourceDictionary
                xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
                <x:Double x:Key="FlyoutThemeMinWidth">96</x:Double>
                <x:Double x:Key="UnrelatedWidth">400</x:Double>
                </ResourceDictionary>''')
            _, shared = read_dictionaries(path, ['FlyoutThemeMinWidth'])
            self.assertEqual(shared, {'FlyoutThemeMinWidth': ('f64', 96.0)})

    def test_a_size_each_theme_states_differently_is_reported(self):
        # A scalar both themes agree on becomes a constant; one they disagree on has nowhere to
        # go, and must be reported rather than dropped in silence.
        errors = io.StringIO()
        themes = {
            'Light': {'IconHeight': ('f64', 9.0)},
            'Default': {'IconHeight': ('f64', 8.0)},
        }
        with contextlib.redirect_stderr(errors):
            ThemeResources().add_dictionary('Test.xaml', ['IconHeight'], themes, {}, {})
        self.assertIn('Test.xaml: IconHeight differs by theme (9.0 light, 8.0 dark)', errors.getvalue())

    def test_brush_aliases_of_the_common_dictionary_reach_the_resources(self):
        source = DEFAULT_SOURCE
        if not source.exists():
            self.skipTest('local WinUI source checkout unavailable')
        resources = quietly(lambda: gather(source, ['Button']))
        common = section(resources, 'Common_themeresources_any.xaml')
        # SystemFillColorAttentionBrush states its colour through a ThemeResource reference.
        self.assertIn('system_fill_color_attention_brush', common)
        self.assertEqual(value(resources, 'system_fill_color_attention_brush'), ('Color', 'accent.base', 'accent.light2'))

    def test_brush_opacity_survives_aliases_and_multiplies_existing_alpha(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'opacity.xaml'
            source.write_text('''<ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
              <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key="Light">
                  <SolidColorBrush x:Key="AccentHover" Color="{ThemeResource SystemAccentColorDark1}" Opacity="0.9" />
                  <SolidColorBrush x:Key="Translucent" Color="#80010203" Opacity="0.5" />
                  <StaticResource x:Key="ControlHover" ResourceKey="AccentHover" />
                </ResourceDictionary>
              </ResourceDictionary.ThemeDictionaries>
            </ResourceDictionary>''')
            themes, _ = read_dictionaries(source)
        entries = themes['Light']
        hover = resolve(entries, {}, 'ControlHover')
        self.assertEqual(hover, ('opacity', (('accent', 'SystemAccentColorDark1'), 0.9)))
        alpha = resolve(entries, {}, 'Translucent')
        self.assertEqual(alpha, ('opacity', (('color', (128, 1, 2, 3)), 0.5)))
        resources = ThemeResources()
        resources.add_dictionary('opacity.xaml', ['ControlHover', 'Translucent'], {'Light': entries, 'Default': entries}, {}, {})
        out = []
        resources.emit(out)
        generated = '\n'.join(out)
        self.assertIn('pub control_hover: Color', generated)
        self.assertIn('color = accent.dark1;', generated)
        self.assertIn('Some(color.a * 0.9)', generated)
        self.assertIn('color = Color::from_argb(128, 1, 2, 3);', generated)
        self.assertIn('Some(color.a * 0.5)', generated)

    def test_unresolved_resource_names_are_reported(self):
        errors = io.StringIO()
        with contextlib.redirect_stderr(errors):
            ThemeResources().add_dictionary('Test.xaml', ['MissingBrush'], {}, {}, {})
        self.assertIn('Test.xaml: unresolved resource MissingBrush (Light, Default)', errors.getvalue())


if __name__ == '__main__':
    unittest.main()
