//! A gutter of line numbers beside a `TextBox`, placed by the box's `GetRectFromCharacterIndex`
//! and moved with it as its text scrolls or changes, the way a code editor's gutter follows its
//! text.

use std::rc::Rc;
use waft::{FrameCallback, SchedulerBinding};
use waft_foundation::{App, Handle, ListenableObject, Listener};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::*;
use waft_winui::*;

/// The gutter's width.
const GUTTER_WIDTH: f64 = 32.0;

/// `text_box` with its lines numbered at its left.
#[derive(Debug)]
pub struct LineNumbers {
    /// The numbered box; its text is its controller's.
    pub text_box: TextBox,
    /// The box's controller, which says when the text changed.
    pub controller: Handle<TextEditingController>,
}

/// Where each line's number goes.
pub struct LineNumbersState {
    state: StateData<LineNumbers>,
    key: GlobalKey,
    /// Each numbered line's top and height in the box's coordinates.
    lines: Vec<(usize, f64, f64)>,
    listener: Option<Listener>,
}

impl StatefulWidget for LineNumbers {
    type State = LineNumbersState;

    fn create_state(&self) -> LineNumbersState {
        LineNumbersState {
            state: StateData::new(),
            key: GlobalKey::new(),
            lines: Vec::new(),
            listener: None,
        }
    }
}

impl LineNumbersState {
    /// Places the numbers once the frame under way, which a scroll, an edit or the first build
    /// has asked for, has laid the box out.
    fn place_after_frame(self: Handle<Self>, app: &mut App) {
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if app.contains(self) && self.mounted(app) {
                    self.place(app);
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    /// Asks the box where each line's first character is.
    fn place(self: Handle<Self>, app: &mut App) {
        let text = self
            .widget(app)
            .controller
            .as_controller()
            .text_value(app)
            .to_string();
        let key = app.get(self).key.clone();
        let Some(text_box) = key.current_state::<TextBoxState>(app) else {
            return;
        };
        let mut lines = Vec::new();
        let mut start = 0;
        for (number, line) in text.split('\n').enumerate() {
            let length = line.encode_utf16().count();
            // The first character of the line, or the line break that ends an empty line.
            if start < text.encode_utf16().count()
                && let Some(rect) = text_box.get_rect_from_character_index(app, start as i32, false)
            {
                lines.push((number + 1, rect.top, rect.height()));
            }
            start += length + 1;
        }
        if lines != app.get(self).lines {
            self.set_state(app, |state| state.lines = lines);
        }
    }

    fn gutter(self: Handle<Self>, app: &App) -> WidgetRef {
        let numbers = app.get(self).lines.iter().map(|(number, top, height)| {
            Positioned::new(Builder::new({
                let number = *number;
                move |app, context| {
                    let r = ThemeResources::of(app, context);
                    Text::new(number.to_string())
                        .style(TextBlockStyle::Caption.text_style(&r, r.text_fill_color_tertiary))
                        .into_widget()
                }
            }))
            .top(*top)
            .height(*height)
            .right(8.0)
            .into_widget()
        });
        ClipRect::new()
            .child(Stack::new().children(numbers.collect::<Vec<_>>()))
            .into_widget()
    }
}

impl State for LineNumbersState {
    type Widget = LineNumbers;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let listener = Listener::new(move |app| self.place_after_frame(app));
        self.widget(app)
            .controller
            .add_listener(app, listener.clone());
        app.get_mut(self).listener = Some(listener);
        self.place_after_frame(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(listener) = app.get_mut(self).listener.take() {
            let controller = self.widget(app).controller;
            controller.remove_listener(app, &listener);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let key = app.get(self).key.clone();
        let text_box = self.widget(app).text_box.clone().key(Rc::new(key));
        let scrolled = NotificationListener::<dyn ScrollNotification>::new(text_box)
            .on_notification(move |app, _| {
                self.place_after_frame(app);
                false
            })
            .into_widget();
        Stack::new()
            .children([
                Padding::new(EdgeInsetsGeometry::from_ltrb(GUTTER_WIDTH, 0.0, 0.0, 0.0))
                    .child(scrolled)
                    .into_widget(),
                Positioned::new(self.gutter(app))
                    .left(0.0)
                    .top(0.0)
                    .bottom(0.0)
                    .width(GUTTER_WIDTH)
                    .into_widget(),
            ])
            .into_widget()
    }
}
