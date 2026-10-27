//! ColorPicker, ColorSpectrum and ColorPickerSlider: the spectrum's images and selection, the
//! keys of the spectrum and the sliders (with the colours `ColorPickerTests.cs` expects), the
//! text boxes, the More button, the focus order, and the colour the application owns.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::{Fixture, mount};
use inset_embedder::{Color, Offset, PointerChange, Rect, TextAffinity, TextRange, TextSelection};
use inset_foundation::{App, Handle, Listener};
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical, TextEditingValue, TextInput,
};
use inset_widgets::*;
use inset_winui::*;
use std::{cell::RefCell, rc::Rc};
use winui_gallery::Feature;

/// A gallery page tall enough to show every example without scrolling.
const PAGE: [u32; 2] = [1080, 1500];

fn gallery() -> Fixture {
    let mut f = Fixture::for_feature(PAGE, Feature::ColorPicker);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    f
}

fn click(f: &mut Fixture, point: Offset) {
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
}

fn send_key(f: &mut Fixture, event: KeyEvent) {
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
}

/// A press and release of `logical`.
fn key(f: &mut Fixture, logical: Key, physical: Physical) {
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
    );
    send_key(f, KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)));
    f.pump();
}

/// `logical` pressed while `modifier` is held.
fn key_with(f: &mut Fixture, modifier: (Key, Physical), logical: Key, physical: Physical) {
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(modifier.1, modifier.0, f.at)),
    );
    key(f, logical, physical);
    send_key(
        f,
        KeyEvent::Up(KeyUpEvent::new(modifier.1, modifier.0, f.at)),
    );
    f.pump();
}

const CONTROL: (Key, Physical) = (Key::CONTROL_LEFT, Physical::CONTROL_LEFT);
const SHIFT: (Key, Physical) = (Key::SHIFT_LEFT, Physical::SHIFT_LEFT);

fn tab(f: &mut Fixture) {
    key(f, Key::TAB, Physical::TAB);
}

fn arrow(f: &mut Fixture, logical: Key) {
    let physical = match logical {
        Key::ARROW_LEFT => Physical::ARROW_LEFT,
        Key::ARROW_RIGHT => Physical::ARROW_RIGHT,
        Key::ARROW_UP => Physical::ARROW_UP,
        _ => Physical::ARROW_DOWN,
    };
    key(f, logical, physical);
}

fn ctrl_arrow(f: &mut Fixture, logical: Key) {
    let physical = match logical {
        Key::ARROW_LEFT => Physical::ARROW_LEFT,
        Key::ARROW_RIGHT => Physical::ARROW_RIGHT,
        Key::ARROW_UP => Physical::ARROW_UP,
        _ => Physical::ARROW_DOWN,
    };
    key_with(f, CONTROL, logical, physical);
}

/// The input method replacing the focused text box's text with `text`, the caret at its end.
fn type_text(f: &mut Fixture, text: &str) {
    let value = TextEditingValue::new()
        .text(text)
        .selection(TextSelection::collapsed(
            text.encode_utf16().count() as i32,
            TextAffinity::Downstream,
        ))
        .composing(TextRange::EMPTY);
    let mut app = f.cell.borrow_mut();
    TextInput::instance(&mut app).update_editing_value(&mut app, value);
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

/// The global bounds of the render object of the first element whose widget is a `W`
/// satisfying `test`.
fn bounds_of<W: 'static>(f: &Fixture, test: impl Fn(&W) -> bool) -> Rect {
    let elements = f.elements();
    let app = f.cell.borrow();
    for element in elements {
        if let Some(widget) = downcast_widget::<W>(element.widget(&app).as_ref())
            && test(widget)
        {
            let render = element.find_render_object(&app).unwrap().as_box().unwrap();
            let origin = render.local_to_global(&app, Offset::ZERO, None);
            return origin & render.size(&app);
        }
    }
    panic!("missing widget")
}

/// The square the picker's `ColorSpectrum`, a box at first, lays out.
fn spectrum_bounds(f: &Fixture) -> Rect {
    bounds_of::<ColorSpectrum>(f, |spectrum| spectrum.shape == ColorSpectrumShape::Box)
}

/// The texts of the picker's text boxes, top to bottom and left to right: the hex value beside
/// the colour model, the three channels and the opacity while it shows.
fn texts(f: &Fixture) -> Vec<String> {
    let elements = f.elements();
    let app = f.cell.borrow();
    let mut boxes: Vec<_> = elements
        .into_iter()
        .filter_map(|element| {
            let editable = downcast_widget::<EditableText>(element.widget(&app).as_ref())?;
            let render = element.find_render_object(&app)?.as_box()?;
            let origin = render.local_to_global(&app, Offset::ZERO, None);
            Some((
                (origin.dy(), origin.dx()),
                editable.controller.text_value(&app).to_owned(),
            ))
        })
        .collect();
    boxes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    boxes.into_iter().map(|(_, text)| text).collect()
}

/// The text of the first label starting with `prefix`.
fn label_starting(f: &Fixture, prefix: &str) -> String {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements
        .into_iter()
        .find_map(|element| {
            let paragraph = element
                .render_object(&app)?
                .downcast::<inset_rendering::RenderParagraph>(&app)?;
            let text = paragraph.text(&app).to_plain_text(true, true);
            text.starts_with(prefix).then_some(text)
        })
        .unwrap_or_else(|| panic!("missing label {prefix}"))
}

/// The text box showing `text`: its center and its focus.
fn text_box(f: &Fixture, text: &str) -> (Offset, Handle<FocusNode>) {
    let elements = f.elements();
    let app = f.cell.borrow();
    for element in elements {
        if let Some(editable) = downcast_widget::<EditableText>(element.widget(&app).as_ref())
            && editable.controller.text_value(&app) == text
        {
            let render = element.find_render_object(&app).unwrap().as_box().unwrap();
            let size = render.size(&app);
            let center = render.local_to_global(
                &app,
                Offset::new(size.width() / 2.0, size.height() / 2.0),
                None,
            );
            return (center, editable.focus_node);
        }
    }
    panic!("missing text box {text}")
}

/// Which part of the picker has the focus: the spectrum, a slider's channel, the More button,
/// the colour model box, or a text box's text.
fn focused(f: &Fixture) -> String {
    let mut app = f.cell.borrow_mut();
    let node = primary_focus(&mut app).expect("a focused node");
    let context = node.context(&app).expect("a mounted node");
    if let Some(editable) = context.find_ancestor_widget_of_exact_type::<EditableText>(&app) {
        return format!("text {}", editable.controller.text_value(&app));
    }
    if let Some(slider) = context.find_ancestor_widget_of_exact_type::<ColorPickerSlider>(&app) {
        return format!("slider {:?}", slider.color_channel);
    }
    if context
        .find_ancestor_widget_of_exact_type::<ColorSpectrum>(&app)
        .is_some()
    {
        return "spectrum".to_owned();
    }
    if context
        .find_ancestor_widget_of_exact_type::<ComboBox>(&app)
        .is_some()
    {
        return "combo box".to_owned();
    }
    if context
        .find_ancestor_widget_of_exact_type::<ToggleButton>(&app)
        .is_some()
    {
        return "more button".to_owned();
    }
    "other".to_owned()
}

/// Whether the label `text` is on screen, a tooltip's included.
fn shows(f: &Fixture, text: &str) -> bool {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().any(|element| {
        element
            .render_object(&app)
            .and_then(|object| object.downcast::<inset_rendering::RenderParagraph>(&app))
            .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == text)
    })
}

/// The colour at `point` of the last frame.
fn pixel(f: &Fixture, point: Offset) -> [u8; 3] {
    let pixels = f.view.pixels.borrow();
    let index = (point.dy() as usize * f.view.size[0] as usize + point.dx() as usize) * 4;
    [pixels[index], pixels[index + 1], pixels[index + 2]]
}

#[test]
fn the_page_in_its_shapes_options_and_themes() {
    let mut f = gallery();
    f.capture("color_picker_box_light");
    // Four pixels in from the top left: hue 5.6, saturation 0.98; the bottom is white.
    let spectrum = spectrum_bounds(&f);
    assert_eq!(spectrum.size(), inset_embedder::Size::new(256.0, 256.0));
    let near = |a: [u8; 3], b: [u8; 3]| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 3);
    let top_left = pixel(&f, spectrum.top_left() + Offset::new(4.0, 4.0));
    assert!(near(top_left, [255, 28, 4]), "{top_left:?}");
    let bottom = pixel(&f, spectrum.bottom_left() + Offset::new(128.0, -4.0));
    assert!(bottom.iter().all(|c| *c > 240), "{bottom:?}");
    f.tap("Ring spectrum");
    f.tap("Opacity");
    f.capture("color_picker_ring_alpha_light");
    f.tap("Keep as previous color");
    f.capture("color_picker_previous_light");
    f.tap("Dark theme");
    f.capture("color_picker_ring_alpha_dark");
    f.tap("Ring spectrum");
    f.capture("color_picker_box_alpha_dark");
    f.tap("Horizontal");
    f.capture("color_picker_horizontal_dark");
    f.tap("Light theme");
    f.capture("color_picker_horizontal_light");
    f.tap("Horizontal");
    f.tap("More button");
    f.capture("color_picker_more_light");
    f.tap("More");
    f.capture("color_picker_more_open_light");
}

#[test]
fn the_hsv_entries_replace_the_rgb_ones() {
    let mut f = gallery();
    let combo = bounds_of::<ComboBox>(&f, |_| true);
    click(&mut f, combo.center());
    let hsv = f.find("HSV");
    click(&mut f, hsv);
    assert!(shows(&f, "Hue") && shows(&f, "Saturation") && !shows(&f, "Red"));
    assert_eq!(texts(&f), ["#FF2631", "357", "85", "100"]);
    f.capture("color_picker_hsv_light");
    f.tap("Dark theme");
    f.capture("color_picker_hsv_dark");
}

#[test]
fn the_standalone_spectrum_is_a_ring_of_saturation_and_value() {
    let mut f = gallery();
    let ring =
        bounds_of::<ColorSpectrum>(&f, |spectrum| spectrum.shape == ColorSpectrumShape::Ring);
    assert_eq!(ring.size(), inset_embedder::Size::new(200.0, 200.0));
    // The centre is black: value runs from the outside in.
    let centre = pixel(&f, ring.center() + Offset::new(2.0, 2.0));
    assert!(centre.iter().all(|c| *c < 20), "{centre:?}");
    // Hue 206 lies between the cyan and blue images, 0.43 of the way: near the edge just above
    // the right of the centre, saturation 0.99 and value 0.95, the cross-fade is the colour.
    let edge = pixel(&f, ring.center() + Offset::new(95.0, -2.0));
    let expected = [1u8, 138, 242];
    assert!(
        edge.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 4),
        "{edge:?}"
    );
    assert_eq!(label_starting(&f, "Color #"), "Color #FF0078D4");
    // Left of the centre, half way out: saturation half way around, value half way out, at the
    // hue of the colour.
    click(&mut f, ring.center() + Offset::new(-50.0, 0.0));
    assert_eq!(label_starting(&f, "Color #"), "Color #FF406480");
    f.capture("color_picker_spectrum_ring_light");
}

/// An application owning a picker's colour, which it may decline to change.
struct Owner {
    initial: Color,
    log: Rc<RefCell<Vec<String>>>,
    configure: Rc<dyn Fn(ColorPicker) -> ColorPicker>,
}

impl std::fmt::Debug for Owner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Owner")
    }
}

struct OwnerState {
    state: StateData<Owner>,
    color: Color,
    accept: bool,
    narrow: bool,
    more_button: bool,
}

impl StatefulWidget for Owner {
    type State = OwnerState;

    fn create_state(&self) -> OwnerState {
        OwnerState {
            state: StateData::new(),
            color: self.initial,
            accept: true,
            narrow: false,
            more_button: false,
        }
    }
}

/// `#AARRGGBB`.
fn hex(color: Color) -> String {
    format!("#{:08X}", color.to_argb32())
}

impl State for OwnerState {
    type Widget = Owner;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let log = self.widget(app).log.clone();
        let configure = self.widget(app).configure.clone();
        let state = app.get(self);
        let mut picker = configure(ColorPicker::new(state.color, move |app, args| {
            log.borrow_mut().push(format!(
                "{} -> {}",
                hex(args.old_color),
                hex(args.new_color)
            ));
            if app.get(self).accept {
                self.set_state(app, |s| s.color = args.new_color);
            }
        }));
        if state.narrow {
            picker = picker.min_saturation(50).min_value(50);
        }
        if state.more_button {
            picker = picker.is_more_button_visible(true);
        }
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children([
                picker.into_widget(),
                Row::new()
                    .children([
                        Button::text(
                            "Decline",
                            Listener::new(move |app| self.set_state(app, |s| s.accept = false)),
                        )
                        .into_widget(),
                        Button::text(
                            "White",
                            Listener::new(move |app| {
                                self.set_state(app, |s| {
                                    s.color = Color::from_argb(255, 255, 255, 255)
                                })
                            }),
                        )
                        .into_widget(),
                        Button::text(
                            "Narrow",
                            Listener::new(move |app| self.set_state(app, |s| s.narrow = true)),
                        )
                        .into_widget(),
                        Button::text(
                            "Hide entries",
                            Listener::new(move |app| self.set_state(app, |s| s.more_button = true)),
                        )
                        .into_widget(),
                    ])
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// A picker owned by `Owner`, red at first, configured by `configure`, with the log of its
/// `ColorChanged`.
fn owner(
    configure: impl Fn(ColorPicker) -> ColorPicker + 'static,
) -> (Fixture, Rc<RefCell<Vec<String>>>) {
    owner_of(Color::from_argb(255, 255, 0, 0), configure)
}

/// A picker owned by `Owner`, `initial` at first.
fn owner_of(
    initial: Color,
    configure: impl Fn(ColorPicker) -> ColorPicker + 'static,
) -> (Fixture, Rc<RefCell<Vec<String>>>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let root = log.clone();
    let configure: Rc<dyn Fn(ColorPicker) -> ColorPicker> = Rc::new(configure);
    let mut f = Fixture::with_root(
        [720, 900],
        mount(move |app| {
            winui_gallery::install_fonts(app);
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .page_route_builder(|app, settings, builder| {
                    let route = PageRouteBuilder::new(
                        app,
                        Rc::new(move |app, context, _, _| builder(app, context)),
                    )
                    .settings(app, RouteSettingsRef::Settings(settings.clone()));
                    PageRoute::as_page_route(route)
                })
                .home(ThemeScope::new(
                    Theme::Light,
                    ColoredBox::new(Color::from_argb(255, 243, 243, 243)).child(
                        Align::new()
                            .alignment(inset_painting::AlignmentGeometry::TOP_START)
                            .child(
                                Padding::new(inset_painting::EdgeInsetsGeometry::all(12.0)).child(
                                    Owner {
                                        initial,
                                        log: root,
                                        configure,
                                    },
                                ),
                            ),
                    ),
                ))
                .into_widget()
        }),
    );
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    (f, log)
}

/// The hex text box's text.
fn hex_text(f: &Fixture) -> String {
    texts(f)
        .into_iter()
        .find(|text| text.starts_with('#'))
        .expect("the hex text box")
}

/// `CanUseKeyboardToInteractWithColorSpectrum`: from red, the arrows step hue and saturation
/// with wrapping, and Ctrl steps to the next named colour; the name tooltip follows.
#[test]
fn the_spectrum_keys_reach_the_interaction_test_colours() {
    let (mut f, log) = owner(|picker| picker);
    tab(&mut f);
    assert_eq!(focused(&f), "spectrum");
    f.capture("color_picker_spectrum_focused");
    assert!(shows(&f, "Red"));
    let spectrum = spectrum_bounds(&f);
    let steps: [(bool, Key, &str, &str); 18] = [
        (false, Key::ARROW_RIGHT, "#FF0400", "Red"),
        (false, Key::ARROW_LEFT, "#FF0000", "Red"),
        (false, Key::ARROW_LEFT, "#FF0004", "Red"),
        (false, Key::ARROW_RIGHT, "#FF0000", "Red"),
        (true, Key::ARROW_RIGHT, "#FF7700", "Orange"),
        (true, Key::ARROW_LEFT, "#FF000D", "Red"),
        (true, Key::ARROW_LEFT, "#FF00AA", "Pink"),
        (true, Key::ARROW_RIGHT, "#FF000D", "Red"),
        (false, Key::ARROW_DOWN, "#FF030F", "Red"),
        (false, Key::ARROW_UP, "#FF000D", "Red"),
        (false, Key::ARROW_UP, "#FFFFFF", "White"),
        (false, Key::ARROW_DOWN, "#FF000D", "Red"),
        (true, Key::ARROW_DOWN, "#FF999E", "Rose"),
        (true, Key::ARROW_UP, "#FF2631", "Red"),
        (true, Key::ARROW_UP, "#FFF2F3", "White"),
        (true, Key::ARROW_DOWN, "#FF2934", "Red"),
        (true, Key::ARROW_DOWN, "#FF999E", "Rose"),
        (true, Key::ARROW_UP, "#FF2631", "Red"),
    ];
    for (control, logical, expected, name) in steps {
        if control {
            ctrl_arrow(&mut f, logical);
        } else {
            arrow(&mut f, logical);
        }
        assert_eq!(hex_text(&f), expected, "{logical:?} with Ctrl {control}");
        assert!(shows(&f, name), "{name} after {logical:?}");
    }
    assert_eq!(log.borrow().len(), 18);
    assert_eq!(log.borrow()[0], "#FFFF0000 -> #FFFF0400");
    // `VerifySelectionEllipsePosition(255, 38)` of a 256 spectrum: the black focus ellipse is
    // 8 to 10 left of the centre.
    let ellipse = pixel(&f, spectrum.top_left() + Offset::new(255.0 - 9.0, 38.0));
    assert!(ellipse.iter().all(|c| *c < 80), "{ellipse:?}");
}

/// `CanUseKeyboardToInteractWithSliders`: the value slider steps without wrapping, by one or to
/// the next named colour, and the opacity slider by 1 percent or to the next 10.
#[test]
fn the_slider_keys_reach_the_interaction_test_colours() {
    let (mut f, _log) = owner_of(Color::from_argb(255, 255, 38, 49), |picker| {
        picker.is_alpha_enabled(true)
    });
    tab(&mut f);
    assert_eq!(focused(&f), "spectrum");
    tab(&mut f);
    assert_eq!(focused(&f), "slider Value");
    assert!(shows(&f, "Value 100 (Red)"));
    f.capture("color_picker_slider_focused");
    for (control, logical, expected) in [
        (false, Key::ARROW_LEFT, "#FFFC2631"),
        (false, Key::ARROW_RIGHT, "#FFFF2631"),
        (false, Key::ARROW_RIGHT, "#FFFF2631"),
        (true, Key::ARROW_LEFT, "#FF660F14"),
        (true, Key::ARROW_LEFT, "#FF0F0203"),
        (true, Key::ARROW_LEFT, "#FF000000"),
        (true, Key::ARROW_RIGHT, "#FF691014"),
        (true, Key::ARROW_RIGHT, "#FFD62029"),
        (true, Key::ARROW_RIGHT, "#FFFF2631"),
    ] {
        if control {
            ctrl_arrow(&mut f, logical);
        } else {
            arrow(&mut f, logical);
        }
        assert_eq!(hex_text(&f), expected, "{logical:?} with Ctrl {control}");
    }
    tab(&mut f);
    assert_eq!(focused(&f), "slider Alpha");
    assert!(shows(&f, "100% opacity"));
    for (control, logical, expected) in [
        (false, Key::ARROW_LEFT, "#FCFF2631"),
        (false, Key::ARROW_RIGHT, "#FFFF2631"),
        (false, Key::ARROW_RIGHT, "#FFFF2631"),
        (true, Key::ARROW_LEFT, "#E6FF2631"),
        (false, Key::ARROW_RIGHT, "#E8FF2631"),
        (true, Key::ARROW_LEFT, "#E6FF2631"),
        (true, Key::ARROW_RIGHT, "#FFFF2631"),
    ] {
        if control {
            ctrl_arrow(&mut f, logical);
        } else {
            arrow(&mut f, logical);
        }
        assert_eq!(hex_text(&f), expected, "{logical:?} with Ctrl {control}");
    }
}

/// `VerifySpectrumTakesFocusOnPointerPress`, and the colour under the pointer: a press focuses
/// the spectrum without the focus ellipse, and a drag keeps picking, outside it too.
#[test]
fn a_press_on_the_spectrum_picks_the_colour_under_it() {
    let (mut f, log) = owner(|picker| picker);
    let spectrum = spectrum_bounds(&f);
    // Half way across is hue 179.5 and a quarter down saturation 0.75 at value 1.
    let point = spectrum.top_left() + Offset::new(128.0, 64.0);
    click(&mut f, point);
    assert_eq!(focused(&f), "spectrum");
    assert_eq!(hex_text(&f), "#40FEFF");
    assert_eq!(log.borrow().as_slice(), ["#FFFF0000 -> #FF40FEFF"]);
    // Dragged out past the left edge, the colour is the edge's.
    let from = spectrum.top_left() + Offset::new(128.0, 64.0);
    f.send_mouse(PointerChange::Down, from, 1);
    f.send_mouse(PointerChange::Move, from + Offset::new(-200.0, 0.0), 1);
    f.send_mouse(PointerChange::Up, from + Offset::new(-200.0, 0.0), 0);
    f.pump();
    assert_eq!(hex_text(&f), "#FF4040");
}

/// `CanUseKeyboardToInteractWithTextBoxesForRGB`, `ForAlpha`, `ForHex` and
/// `VerifyInvalidTextEntryIsReverted`.
#[test]
fn the_text_boxes_set_the_colour_and_revert_invalid_text() {
    let (mut f, _log) = owner(|picker| picker.is_alpha_enabled(true));
    assert_eq!(texts(&f), ["#FFFF0000", "255", "0", "0", "100%"]);
    let (red, _) = text_box(&f, "255");
    click(&mut f, red);
    type_text(&mut f, "128");
    assert_eq!(hex_text(&f), "#FF800000");
    // The alpha box gets its percent sign.
    let (alpha, _) = text_box(&f, "100%");
    click(&mut f, alpha);
    type_text(&mut f, "50");
    assert_eq!(texts(&f)[4], "50%");
    assert_eq!(hex_text(&f), "#80800000");
    // The hex box gets its number sign.
    let (hex, _) = text_box(&f, "#80800000");
    click(&mut f, hex);
    type_text(&mut f, "FFFF0000");
    assert_eq!(texts(&f)[1], "255");
    // Invalid text stays until the box loses the focus, then goes back.
    let (green, _) = text_box(&f, "0");
    click(&mut f, green);
    type_text(&mut f, "foo");
    assert_eq!(texts(&f)[2], "foo");
    tab(&mut f);
    assert_eq!(texts(&f)[2], "0");
    // With narrowed ranges a colour out of them is brought into them as the box loses the focus.
    f.tap("Narrow");
    let (red, _) = text_box(&f, &texts(&f)[1]);
    click(&mut f, red);
    type_text(&mut f, "0");
    tab(&mut f);
    assert_eq!(&texts(&f)[1..4], ["128", "64", "64"]);
    assert_eq!(hex_text(&f), "#FF804040");
}

/// `VerifyMoreButtonShowsAndHidesTextInputFields`.
#[test]
fn the_more_button_shows_and_hides_the_entries() {
    let (mut f, _log) = owner(|picker| picker.is_more_button_visible(true));
    assert!(texts(&f).is_empty());
    f.tap("More");
    assert_eq!(texts(&f).len(), 4);
    assert!(shows(&f, "Less"));
    f.tap("Less");
    assert!(texts(&f).is_empty());
}

/// `VerifyTabFocusOrder`: the spectrum, the sliders, the More button, the colour model box, the
/// channels and the hex box last, by `TabIndex`.
#[test]
fn tab_follows_the_template_tab_indices() {
    let (mut f, _log) = owner(|picker| picker.is_alpha_enabled(true).is_more_button_visible(true));
    f.tap("More");
    let mut order = Vec::new();
    for _ in 0..9 {
        tab(&mut f);
        order.push(focused(&f));
    }
    assert_eq!(
        order,
        [
            "spectrum",
            "slider Value",
            "slider Alpha",
            "more button",
            "combo box",
            "text 255",
            "text 0",
            "text 0",
            "text 100%",
        ]
    );
    tab(&mut f);
    assert_eq!(focused(&f), "text #FFFF0000");
    // Shift+Tab back to the spectrum shows its focus ellipse.
    for _ in 0..9 {
        key_with(&mut f, SHIFT, Key::TAB, Physical::TAB);
    }
    assert_eq!(focused(&f), "spectrum");
    f.capture("color_picker_tab_order_back");
}

/// The application owns the colour: one it sets raises no `ColorChanged`; one the picker makes
/// to bring the colour into narrowed ranges raises it after the frame; and a change the
/// application declines is undone when it next builds the picker.
#[test]
fn the_application_owns_the_colour() {
    let (mut f, log) = owner(|picker| picker);
    f.tap("White");
    assert_eq!(hex_text(&f), "#FFFFFF");
    assert!(log.borrow().is_empty());
    f.tap("Narrow");
    // White has no saturation; the minimum of 50 makes it pink.
    assert_eq!(hex_text(&f), "#FF8080");
    assert_eq!(log.borrow().as_slice(), ["#FFFFFFFF -> #FFFF8080"]);
    f.tap("Decline");
    let spectrum = spectrum_bounds(&f);
    click(&mut f, spectrum.top_left());
    assert_eq!(hex_text(&f), "#FF0000");
    assert_eq!(log.borrow().len(), 2);
    // The application builds the picker again with the colour it kept.
    f.tap("Decline");
    assert_eq!(hex_text(&f), "#FF8080");
    assert_eq!(log.borrow().len(), 2);
}

/// The picker is as tall as what it shows: with the More button its entries take no room until
/// the button opens them, and then the picker is taller by the button's row than without it.
#[test]
fn the_picker_is_as_tall_as_what_it_shows() {
    let (mut f, _log) = owner(|picker| picker);
    let picker = |f: &Fixture| bounds_of::<ColorPicker>(f, |_| true);
    let without_more_button = picker(&f).height();
    f.capture("color_picker_height_entries");
    f.tap("Hide entries");
    let collapsed = picker(&f).height();
    f.capture("color_picker_height_collapsed");
    // As tall as a picker first built with the More button.
    let (fresh, _log) = owner(|picker| picker.is_more_button_visible(true));
    assert_eq!(collapsed, picker(&fresh).height());
    assert!(
        collapsed < without_more_button,
        "{collapsed} {without_more_button}"
    );
    f.tap("More");
    let more_button = bounds_of::<ToggleButton>(&f, |_| true).height();
    assert_eq!(more_button, 32.0);
    f.capture("color_picker_height_open");
    assert_eq!(picker(&f).height(), without_more_button + more_button);
    f.tap("Less");
    assert_eq!(picker(&f).height(), collapsed);
}

/// Like `Slider`, a `ColorPickerSlider` fills the width its parent bounds.
#[test]
fn a_color_picker_slider_fills_the_width_it_is_given() {
    let size = common::size_under_center([300, 300], |_| {
        ColorPickerSlider::new(10.0, |_, _| {}).into_widget()
    });
    assert_eq!(size.width(), 300.0);
}
