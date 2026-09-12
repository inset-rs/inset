//! Flutter web `text_editing.dart`: a transparent DOM field so the browser IME runs.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use inset_embedder::{
    Matrix4, Offset, Rect, Size, TextEditingValue, TextInputAction, TextInputConfiguration,
    TextInputType, TextRange, TextSelection, transform3,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{
    CompositionEvent, Document, EventTarget, FocusEvent, HtmlElement, HtmlInputElement,
    HtmlTextAreaElement, KeyboardEvent,
};

const OFF_SCREEN: i32 = -9999;

pub(crate) enum TextInputNotify {
    Value(TextEditingValue),
    Action(TextInputAction),
}

pub(crate) type TextInputListener = Rc<dyn Fn(TextInputNotify)>;

/// One active DOM editing session, Flutter's `DefaultTextEditingStrategy`.
pub(crate) struct WebTextInput {
    element: HtmlElement,
    document: Document,
    input_action: Rc<Cell<TextInputAction>>,
    newline: Rc<Cell<bool>>,
    suppress: Rc<Cell<bool>>,
    closing: Rc<Cell<bool>>,
    composing_text: Rc<RefCell<Option<String>>>,
    last_editing_state: Rc<RefCell<Option<TextEditingValue>>>,
    transform: Cell<Option<Matrix4>>,
    dpr: Cell<f64>,
    closures: Vec<Closure<dyn FnMut(web_sys::Event)>>,
    selection_change: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl WebTextInput {
    pub(crate) fn start(
        document: &Document,
        configuration: &TextInputConfiguration,
        notify: TextInputListener,
    ) -> WebTextInput {
        let element = create_element(document, configuration);
        apply_static_style(&element);
        element.set_tab_index(-1);
        apply_configuration(&element, configuration);
        let _ = document
            .body()
            .expect("document.body")
            .append_child(&element);
        let mut input = WebTextInput {
            element,
            document: document.clone(),
            input_action: Rc::new(Cell::new(configuration.input_action)),
            newline: Rc::new(Cell::new(is_newline(configuration))),
            suppress: Rc::new(Cell::new(false)),
            closing: Rc::new(Cell::new(false)),
            composing_text: Rc::new(RefCell::new(None)),
            last_editing_state: Rc::new(RefCell::new(None)),
            transform: Cell::new(None),
            dpr: Cell::new(1.0),
            closures: Vec::new(),
            selection_change: None,
        };
        input.bind(notify);
        let _ = input.element.focus();
        input
    }

    pub(crate) fn apply_configuration(&self, configuration: &TextInputConfiguration) {
        self.input_action.set(configuration.input_action);
        self.newline.set(is_newline(configuration));
        apply_configuration(&self.element, configuration);
    }

    pub(crate) fn stop(&self) {
        self.closing.set(true);
        if let Some(closure) = &self.selection_change {
            let _ = self.document.remove_event_listener_with_callback(
                "selectionchange",
                closure.as_ref().unchecked_ref(),
            );
        }
        if let Some(parent) = self.element.parent_node() {
            let _ = parent.remove_child(&self.element);
        }
    }

    pub(crate) fn set_editing_state(&self, value: &TextEditingValue) {
        *self.last_editing_state.borrow_mut() = Some(value.clone());
        if !value.selection.is_valid() {
            return;
        }
        // Setting `element.value` cancels an in-progress composition (Chrome).
        // Flutter still assigns; they do not echo during composition. We skip
        // the write so a caret or selection update cannot wipe the cluster.
        if self.composing_text.borrow().is_some() {
            return;
        }
        self.suppress.set(true);
        set_dom_value(&self.element, &value.text);
        let start = value.selection.start().max(0) as u32;
        let end = value.selection.end().max(0) as u32;
        set_dom_selection(&self.element, start, end);
        self.suppress.set(false);
    }

    pub(crate) fn set_caret_rect(&self, rect: Rect) {
        self.place(rect);
    }

    pub(crate) fn set_client_geometry(&self, _size: Size, transform: &Matrix4) {
        self.transform.set(Some(*transform));
    }

    pub(crate) fn set_dpr(&self, dpr: f64) {
        self.dpr.set(dpr);
    }

    fn place(&self, rect: Rect) {
        let origin = match self.transform.get() {
            Some(transform) => transform3(transform, Offset::new(rect.left, rect.top)),
            None => Offset::new(rect.left, rect.top),
        };
        let dpr = self.dpr.get().max(1.0);
        let style = self.element.style();
        let _ = style.set_property("left", &format!("{}px", origin.dx() / dpr));
        let _ = style.set_property("top", &format!("{}px", origin.dy() / dpr));
        let _ = style.set_property("width", &format!("{}px", (rect.width() / dpr).max(1.0)));
        let _ = style.set_property("height", &format!("{}px", (rect.height() / dpr).max(1.0)));
    }

    fn bind(&mut self, notify: TextInputListener) {
        let change = change_handler(
            self.element.clone(),
            Rc::clone(&self.composing_text),
            Rc::clone(&self.last_editing_state),
            Rc::clone(&self.suppress),
            Rc::clone(&notify),
        );
        let target = self.element.clone();
        self.listen(target.as_ref(), "input", {
            let change = Rc::clone(&change);
            move |event| change.as_ref()(event)
        });
        self.listen(target.as_ref(), "keydown", {
            let newline = Rc::clone(&self.newline);
            let action = Rc::clone(&self.input_action);
            let notify = Rc::clone(&notify);
            move |event| {
                let Some(event) = event.dyn_ref::<KeyboardEvent>() else {
                    return;
                };
                if event.key() != "Enter" {
                    return;
                }
                notify(TextInputNotify::Action(action.get()));
                if !newline.get() {
                    event.prevent_default();
                }
            }
        });
        self.listen(target.as_ref(), "compositionstart", {
            let composing_text = Rc::clone(&self.composing_text);
            move |_event| {
                *composing_text.borrow_mut() = None;
            }
        });
        self.listen(target.as_ref(), "compositionupdate", {
            let composing_text = Rc::clone(&self.composing_text);
            move |event| {
                let Some(event) = event.dyn_ref::<CompositionEvent>() else {
                    return;
                };
                *composing_text.borrow_mut() = Some(event.data().unwrap_or_default());
            }
        });
        self.listen(target.as_ref(), "compositionend", {
            let composing_text = Rc::clone(&self.composing_text);
            let change = Rc::clone(&change);
            move |event| {
                *composing_text.borrow_mut() = None;
                change.as_ref()(event);
            }
        });
        self.listen(target.as_ref(), "blur", {
            let closing = Rc::clone(&self.closing);
            let element = self.element.clone();
            move |event| {
                if closing.get() {
                    return;
                }
                let Some(event) = event.dyn_ref::<FocusEvent>() else {
                    return;
                };
                let related = event.related_target();
                if related.is_none()
                    || related
                        .and_then(|target| target.dyn_into::<web_sys::Node>().ok())
                        .is_some_and(|node| node.is_connected())
                {
                    let _ = element.focus();
                }
            }
        });
        let selection_change = Closure::<dyn FnMut(web_sys::Event)>::new({
            let change = Rc::clone(&change);
            move |event| change.as_ref()(event)
        });
        let _ = self.document.add_event_listener_with_callback(
            "selectionchange",
            selection_change.as_ref().unchecked_ref(),
        );
        self.selection_change = Some(selection_change);
    }

    fn listen(
        &mut self,
        target: &EventTarget,
        name: &str,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(handler);
        let _ = target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
        self.closures.push(closure);
    }
}

impl Drop for WebTextInput {
    fn drop(&mut self) {
        self.stop();
    }
}

fn change_handler(
    element: HtmlElement,
    composing_text: Rc<RefCell<Option<String>>>,
    last_editing_state: Rc<RefCell<Option<TextEditingValue>>>,
    suppress: Rc<Cell<bool>>,
    notify: TextInputListener,
) -> Rc<dyn Fn(web_sys::Event)> {
    Rc::new(move |_event: web_sys::Event| {
        if suppress.get() {
            return;
        }
        let composing = composing_text.borrow().clone();
        let value = determine_composition_state(read_editing_value(&element), composing.as_deref());
        if last_editing_state
            .borrow()
            .as_ref()
            .is_some_and(|last| same_dom_state(last, &value))
        {
            return;
        }
        *last_editing_state.borrow_mut() = Some(value.clone());
        notify(TextInputNotify::Value(value));
    })
}

/// Flutter `EditingState.==`: text, the selection's bounds, and the composing
/// range. Not the selection's direction — the browser cannot report one after
/// the engine writes bounds alone, so a backward selection the framework set
/// would read back as forward and be sent to it as a change.
fn same_dom_state(last: &TextEditingValue, now: &TextEditingValue) -> bool {
    last.text == now.text
        && last.selection.start() == now.selection.start()
        && last.selection.end() == now.selection.end()
        && last.composing == now.composing
}

fn is_newline(configuration: &TextInputConfiguration) -> bool {
    configuration.input_type.index == TextInputType::MULTILINE.index
        && configuration.input_action == TextInputAction::Newline
}

fn create_element(document: &Document, configuration: &TextInputConfiguration) -> HtmlElement {
    if configuration.input_type.index == TextInputType::MULTILINE.index {
        let area: HtmlTextAreaElement = document
            .create_element("textarea")
            .expect("textarea")
            .unchecked_into();
        area.unchecked_into()
    } else {
        let input: HtmlInputElement = document
            .create_element("input")
            .expect("input")
            .unchecked_into();
        input.unchecked_into()
    }
}

fn apply_configuration(element: &HtmlElement, configuration: &TextInputConfiguration) {
    if configuration.read_only {
        let _ = element.set_attribute("readonly", "readonly");
    } else {
        let _ = element.remove_attribute("readonly");
    }
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        input.set_type(input_type(configuration));
    }
    let _ = element.set_attribute("autocomplete", "off");
}

fn input_type(configuration: &TextInputConfiguration) -> &'static str {
    if configuration.obscure_text {
        return "password";
    }
    match configuration.input_type.index {
        i if i == TextInputType::EMAIL_ADDRESS.index => "email",
        i if i == TextInputType::PHONE.index => "tel",
        i if i == TextInputType::URL.index => "url",
        i if i == TextInputType::NUMBER.index => "number",
        _ => "text",
    }
}

/// Flutter `_setStaticStyleAttributes`: transparent, no pointer, IME still placed.
fn apply_static_style(element: &HtmlElement) {
    let style = element.style();
    let _ = style.set_property("forced-color-adjust", "none");
    let _ = style.set_property("white-space", "pre-wrap");
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("top", &format!("{OFF_SCREEN}px"));
    let _ = style.set_property("left", &format!("{OFF_SCREEN}px"));
    let _ = style.set_property("padding", "0");
    let _ = style.set_property("opacity", "1");
    let _ = style.set_property("color", "transparent");
    let _ = style.set_property("background", "transparent");
    let _ = style.set_property("caret-color", "transparent");
    let _ = style.set_property("outline", "none");
    let _ = style.set_property("border", "none");
    let _ = style.set_property("resize", "none");
    let _ = style.set_property("text-shadow", "none");
    let _ = style.set_property("overflow", "hidden");
    let _ = style.set_property("pointer-events", "none");
    let _ = style.set_property("transform-origin", "0 0 0");
}

/// Flutter `CompositionAwareMixin.determineCompositionState`.
fn determine_composition_state(
    editing: TextEditingValue,
    composing_text: Option<&str>,
) -> TextEditingValue {
    let Some(composing_text) = composing_text else {
        return editing.composing(TextRange::EMPTY);
    };
    let composing_base = editing.selection.extent_offset - utf16_len(composing_text);
    if composing_base < 0 {
        return editing;
    }
    editing.composing(TextRange::new(
        composing_base,
        composing_base + utf16_len(composing_text),
    ))
}

fn read_editing_value(element: &HtmlElement) -> TextEditingValue {
    let text = dom_value(element);
    let start = utf16_offset(dom_selection_start(element));
    let end = utf16_offset(dom_selection_end(element));
    let (base, extent) = if dom_selection_backward(element) {
        (end, start)
    } else {
        (start, end)
    };
    TextEditingValue::new()
        .text(text)
        .selection(TextSelection::new(base, extent))
}

fn dom_value(element: &HtmlElement) -> String {
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        input.value()
    } else if let Some(area) = element.dyn_ref::<HtmlTextAreaElement>() {
        area.value()
    } else {
        String::new()
    }
}

fn set_dom_value(element: &HtmlElement, text: &str) {
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        input.set_value(text);
    } else if let Some(area) = element.dyn_ref::<HtmlTextAreaElement>() {
        area.set_value(text);
    }
}

fn dom_selection_start(element: &HtmlElement) -> u32 {
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        input.selection_start().ok().flatten().unwrap_or(0)
    } else if let Some(area) = element.dyn_ref::<HtmlTextAreaElement>() {
        area.selection_start().ok().flatten().unwrap_or(0)
    } else {
        0
    }
}

fn dom_selection_end(element: &HtmlElement) -> u32 {
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        input.selection_end().ok().flatten().unwrap_or(0)
    } else if let Some(area) = element.dyn_ref::<HtmlTextAreaElement>() {
        area.selection_end().ok().flatten().unwrap_or(0)
    } else {
        0
    }
}

fn dom_selection_backward(element: &HtmlElement) -> bool {
    let direction = if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        input.selection_direction().ok().flatten()
    } else if let Some(area) = element.dyn_ref::<HtmlTextAreaElement>() {
        area.selection_direction().ok().flatten()
    } else {
        None
    };
    direction.as_deref() == Some("backward")
}

fn set_dom_selection(element: &HtmlElement, start: u32, end: u32) {
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        let _ = input.set_selection_range(start, end);
    } else if let Some(area) = element.dyn_ref::<HtmlTextAreaElement>() {
        let _ = area.set_selection_range(start, end);
    }
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

fn utf16_offset(js_index: u32) -> i32 {
    i32::try_from(js_index).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_range_is_extent_minus_composing_text() {
        let value = TextEditingValue::new()
            .text("hi你")
            .selection(TextSelection::new(3, 3));
        let value = determine_composition_state(value, Some("你"));
        assert_eq!(value.composing, TextRange::new(2, 3));
    }

    #[test]
    fn composition_without_text_is_empty() {
        let value = TextEditingValue::new()
            .text("hi")
            .selection(TextSelection::new(2, 2));
        let value = determine_composition_state(value, None);
        assert_eq!(value.composing, TextRange::EMPTY);
    }
}
