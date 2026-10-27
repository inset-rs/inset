---
name: porting-winui
description: >-
  Port WinUI 3 controls and Fluent styling into `crates/inset-winui` from Microsoft's
  source: templates, theme resources and behavior. Use before writing or changing any
  Rust in `crates/inset-winui` or the `winui_gallery` example.
---

# Porting WinUI

WinUI 3 (Fluent 2) on the Inset framework, ported from Microsoft's open source. Read values and behavior from the source, not from memory or screenshots. Desktop first: WinUI adapts by window size, not by device.

Ground truth: `.reference/microsoft-ui-xaml`. If that checkout is missing, clone it as `AGENTS.md` says.

## The spec

- Templates and visual states: `controls/dev/CommonStyles/<Control>_themeresources.xaml` (the Fluent styles; ignore `*_perf2026.xaml`). Older base templates: `dxaml/xcp/dxaml/themes/generic.xaml`.
- Tokens: `controls/dev/CommonStyles/Common_themeresources_any.xaml` (colours per theme, elevation brushes, durations, key splines), `CornerRadius_themeresources.xaml`, `TextBlock_themeresources.xaml`.
- Behaviour: `dxaml/xcp/dxaml/lib/<Control>_Partial.cpp`, `dxaml/xcp/core/core/elements/<Control>.cpp`; WinUI 2-era controls under `controls/dev/<Control>/`.
- Not in the repository: theme-animation timings (`RepositionThemeAnimation` and friends read the OS animation library through `uxtheme`), the accent palette and system colours, Mica's wallpaper processing, the RichEdit text engine, DirectManipulation.

## How the code is shaped

- `theme/generated.rs` holds every colour, brush, size, duration and spline of the XAML theme dictionaries in one struct, `ThemeResources`, produced by `tools/gen_resources.py`; `{ThemeResource X}` or `{StaticResource X}` in a template is its field `x`, read from `ThemeResources::of(app, context)`. It is generated rather than typed so a value cannot be mistyped or invented; a control whose resources are missing is added to `DEFAULT_CONTROLS` in the generator.
- A control file is a transcription of its `ControlTemplate`: the elements in order with their `x:Name`s in comments, each visual state group as an enum, the storyboards as the animations they declare. Its behaviour follows the C++ as closely as Inset allows.
- What the repository leaves to Windows (accent shades, system colours, theme-animation timings, Segoe UI) has a substitute and a `PORTING.md` entry saying so. Text is Selawik, Microsoft's metric-compatible open substitute for Segoe UI, bundled with the gallery.
- Widget construction follows `crates/inset-winui/docs/widget-syntax.md`.

## Development

The kit builds on the workspace's own Inset crates. The gallery is `winui_gallery`; its pages demonstrate control behavior and application-owned state. Keep gallery-specific setup in its tests and reusable GPU helpers in the unpublished `inset-winui-test-support` crate, used through dev-dependencies.

```sh
cargo test -p inset-winui -p winui_gallery
cargo clippy -p inset-winui -p winui_gallery --all-targets
WINUI_CAPTURE_DIR=$PWD/output cargo test -p winui_gallery
python3 -m unittest discover -s crates/inset-winui/tools -p 'test_*.py'
```

## Theme resources

Building the library needs no WinUI checkout or Python. Regeneration uses `.reference/microsoft-ui-xaml`:

```sh
python3 crates/inset-winui/tools/gen_resources.py
```

The script owns the ordered `DEFAULT_CONTROLS` list and defaults to `crates/inset-winui/src/theme/generated.rs`. Add a newly needed dictionary to that list rather than extending a README command. Optional positional arguments override the source checkout, output file and control subset:

```sh
python3 crates/inset-winui/tools/gen_resources.py /path/to/microsoft-ui-xaml /tmp/text-resources.rs TextBox PasswordBox
```

The default paths are relative to the repository the script is in, not the current working directory. Explicit relative paths are relative to the current working directory. Check that regeneration produces the intended resource changes and that the generator tests pass.

## Release preparation

Of the kit, only `inset-winui` is published, at the workspace's version with the rest of Inset. The gallery and test-support crate remain unpublished. The library archive includes generated resources, its icon font, licenses and porting notes.

```sh
cargo package -p inset-winui --list
cargo package -p inset-winui
```

Use `--allow-dirty` to inspect an archive of uncommitted preparation changes. Packaging verifies the crate without uploading it. Commits, pushes and publishing require an explicit user request; preparation alone does not authorize them.

## Porting a control

Bottom up: a panel or primitive a template needs is ported before the control that needs it, never stood in for by hand (`Grid` came after `ToggleSwitch`, which had to be transcribed twice). A primitive is designed for the controls that will use it, not only the first one.

1. Generate its resources and add a `ThemeResources::<control>()` accessor.
2. Transcribe the template from `<Control>_themeresources.xaml`.
3. Take behaviour from `<Control>_Partial.cpp`.
4. Add a gallery section and a GPU test, and look at the exported screenshot.
5. Write the `PORTING.md` entries.

## Porting guidance

Faithful WinUI behavior remains the aim, especially where later controls depend on it, to avoid drift over time. For small interaction differences, prefer Inset’s native mechanisms when reproducing the WinUI mechanism would add substantial complexity for little user benefit. One example is Button tap recognition: it uses Inset’s native tap recognizer, not WinUI's pointer capture.

## Rules

- No stubs, hacks or partial controls: stop and say so.
- Do not commit unless asked.
- Do not add a rule to this skill without asking.

## PORTING.md

Each source folder of `crates/inset-winui` keeps its own `PORTING.md` (`src/theme`, `src/primitives`, `src/controls`), with a section per file: `## toggle_switch.rs → ToggleSwitch`. An entry is Change / Reason / Affect, for a reader who knows Flutter and Rust but not WinUI internals; no visible Affect means identical, so no entry. An entry is three sentences: Change starts with its subject and says the idea, Reason names the kind and one fact, Affect says what a reader of the control would notice. Deferred items go under `## Deferred`, each with a trigger. The Reason is one of three, because each has its own way back:

- `os` — Windows supplies it and this host does not: a value not in the repository, or a facility of the OS. Name the substitute; the trigger is a measurement or table from a Windows machine, or a host that provides it.
- `framework` — Inset has no equivalent of the XAML mechanism. The trigger is the framework feature.
- `language` — Rust has no counterpart of the XAML or C++ shape. The Affect says what a caller writes differently.

Start Change with the concrete type, constant or variant being described, then explain the behavior in ordinary language. Keep source symbol names as references, not as substitutes for an explanation. Avoid unnamed subjects such as “dragging a switch” and session-specific shorthand. Reason explains the underlying cause, rather than merely saying an API is missing; Affect describes what the user sees or what application code must do.

For example:

- Change: `ToggleSwitch` waits for enough pointer movement to distinguish a drag from a tap before moving its knob.
  Reason: framework — Flutter recognizers first decide whether the press is a tap or a drag, while WinUI’s draggable knob starts tracking on press.
  Affect: `ToggleSwitch` keeps its knob still for the first 18 px of touch movement, and shorter movement remains a tap.

The type name identifies the code; the rest explains the difference without requiring the reader to know WinUI's internal classes or this conversation.
