//! The Markdown widgets: [`MarkdownBody`], and [`Markdown`], which scrolls.

use std::fmt;
use std::rc::Rc;

use inset_foundation::{App, Handle};
use inset_gestures::AnyGestureRecognizer;
use inset_painting::{EdgeInsets, TextSpan};
use inset_widgets::{
    AnyScrollController, BuildContext, IntoWidget, KeyRef, ListView, ScrollPhysicsRef, State,
    StateData, StatefulWidget, StatelessWidget, WidgetRef,
};
use pulldown_cmark::Options;

use crate::document::{self, Block, GITHUB_FLAVORED, Image};
use crate::render::{self, Bullet, RenderContext};
use crate::style_sheet::{DefaultMarkdownStyle, MarkdownStyleSheet};

/// Called with a tapped link's address and title.
pub type TapLinkHook = Rc<dyn Fn(&mut App, &str, &str)>;
/// Builds the widget of an image.
pub type ImageHook = Rc<dyn Fn(&mut App, &Image) -> WidgetRef>;
/// Builds a task item's checkbox from whether it is ticked.
pub type CheckboxHook = Rc<dyn Fn(&mut App, bool) -> WidgetRef>;
/// Builds a list item's bullet.
pub type BulletHook = Rc<dyn Fn(&mut App, &Bullet) -> WidgetRef>;
/// Builds the spans of a code block's text from its language.
pub type HighlightCodeHook = Rc<dyn Fn(&mut App, Option<&str>, &str) -> TextSpan>;

/// The closures that replace parts of the default rendering; an element without one renders as
/// the [`render`] module's function for it does.
#[derive(Clone, Default)]
pub struct MarkdownHooks {
    /// Without it a link is styled but does nothing when tapped.
    pub on_tap_link: Option<TapLinkHook>,
    /// Without it an image shows its alt text.
    pub image: Option<ImageHook>,
    pub checkbox: Option<CheckboxHook>,
    pub bullet: Option<BulletHook>,
    /// Its spans go inside the code block's text style, box and sideways scrolling.
    pub highlight_code: Option<HighlightCodeHook>,
}

/// What [`MarkdownBody`] and [`Markdown`] share: the text, how it is parsed, how it looks.
#[derive(Clone)]
pub struct MarkdownConfig {
    pub data: String,
    /// The extensions [`data`](Self::data) is parsed with, [`GITHUB_FLAVORED`] by default.
    pub options: Options,
    /// When given, replaces the inherited [`DefaultMarkdownStyle`] whole.
    pub style_sheet: Option<MarkdownStyleSheet>,
    /// Whether a block takes its content's width rather than the full width.
    pub fit_content: bool,
    pub hooks: MarkdownHooks,
}

impl MarkdownConfig {
    fn new(data: String, fit_content: bool) -> MarkdownConfig {
        MarkdownConfig {
            data,
            options: GITHUB_FLAVORED,
            style_sheet: None,
            fit_content,
            hooks: MarkdownHooks::default(),
        }
    }
}

/// The setters of what [`MarkdownConfig`] holds, for each widget built on it.
macro_rules! markdown_setters {
    ($name:ident) => {
        impl $name {
            /// Sets the key.
            pub fn key(mut self, key: KeyRef) -> $name {
                self.key = Some(key);
                self
            }

            /// Sets the extensions the text is parsed with.
            pub fn options(mut self, options: Options) -> $name {
                self.config.options = options;
                self
            }

            /// Sets the look, in place of the inherited [`DefaultMarkdownStyle`].
            pub fn style_sheet(mut self, style_sheet: MarkdownStyleSheet) -> $name {
                self.config.style_sheet = Some(style_sheet);
                self
            }

            /// Sets whether a block takes its content's width rather than the full width.
            pub fn fit_content(mut self, fit_content: bool) -> $name {
                self.config.fit_content = fit_content;
                self
            }

            /// Calls `on_tap_link` with a tapped link's address and title.
            pub fn on_tap_link(
                mut self,
                on_tap_link: impl Fn(&mut App, &str, &str) + 'static,
            ) -> $name {
                self.config.hooks.on_tap_link = Some(Rc::new(on_tap_link));
                self
            }

            /// Builds each image with `image`.
            pub fn image(
                mut self,
                image: impl Fn(&mut App, &Image) -> WidgetRef + 'static,
            ) -> $name {
                self.config.hooks.image = Some(Rc::new(image));
                self
            }

            /// Builds each task item's checkbox with `checkbox`.
            pub fn checkbox(
                mut self,
                checkbox: impl Fn(&mut App, bool) -> WidgetRef + 'static,
            ) -> $name {
                self.config.hooks.checkbox = Some(Rc::new(checkbox));
                self
            }

            /// Builds each list item's bullet with `bullet`.
            pub fn bullet(
                mut self,
                bullet: impl Fn(&mut App, &Bullet) -> WidgetRef + 'static,
            ) -> $name {
                self.config.hooks.bullet = Some(Rc::new(bullet));
                self
            }

            /// Spans each code block's text with `highlight_code`.
            pub fn highlight_code(
                mut self,
                highlight_code: impl Fn(&mut App, Option<&str>, &str) -> TextSpan + 'static,
            ) -> $name {
                self.config.hooks.highlight_code = Some(Rc::new(highlight_code));
                self
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("data", &self.config.data)
                    .finish_non_exhaustive()
            }
        }
    };
}

/// Markdown laid out in a column, for a parent that scrolls or sizes it.
pub struct MarkdownBody {
    pub key: Option<KeyRef>,
    /// [`fit_content`](MarkdownConfig::fit_content) is true by default.
    pub config: MarkdownConfig,
}

impl MarkdownBody {
    /// Renders the Markdown `data`.
    pub fn new(data: impl Into<String>) -> MarkdownBody {
        MarkdownBody {
            key: None,
            config: MarkdownConfig::new(data.into(), true),
        }
    }
}

markdown_setters!(MarkdownBody);

impl StatelessWidget for MarkdownBody {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        MarkdownView {
            config: self.config.clone(),
            layout: Layout::Column,
        }
        .into_widget()
    }
}

/// Markdown in a scrolling list, one block per item.
pub struct Markdown {
    pub key: Option<KeyRef>,
    /// [`fit_content`](MarkdownConfig::fit_content) is false by default.
    pub config: MarkdownConfig,
    /// Around the blocks, inside the scrolling area.
    pub padding: EdgeInsets,
    pub controller: Option<AnyScrollController>,
    pub physics: Option<ScrollPhysicsRef>,
    /// Whether the list takes its content's height rather than all the height it is given.
    pub shrink_wrap: bool,
}

impl Markdown {
    /// Renders the Markdown `data` in a scrolling list.
    pub fn new(data: impl Into<String>) -> Markdown {
        Markdown {
            key: None,
            config: MarkdownConfig::new(data.into(), false),
            padding: EdgeInsets::all(16.0),
            controller: None,
            physics: None,
            shrink_wrap: false,
        }
    }

    /// Sets the padding around the blocks.
    pub fn padding(mut self, padding: EdgeInsets) -> Markdown {
        self.padding = padding;
        self
    }

    /// Sets the controller of the list's scroll position.
    pub fn controller(mut self, controller: AnyScrollController) -> Markdown {
        self.controller = Some(controller);
        self
    }

    /// Sets how the list responds to scrolling.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> Markdown {
        self.physics = Some(physics);
        self
    }

    /// Sets whether the list takes its content's height.
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> Markdown {
        self.shrink_wrap = shrink_wrap;
        self
    }
}

markdown_setters!(Markdown);

impl StatelessWidget for Markdown {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        MarkdownView {
            config: self.config.clone(),
            layout: Layout::List {
                padding: self.padding,
                controller: self.controller,
                physics: self.physics.clone(),
                shrink_wrap: self.shrink_wrap,
            },
        }
        .into_widget()
    }
}

/// How the blocks of a document are laid out.
#[derive(Clone)]
enum Layout {
    Column,
    List {
        padding: EdgeInsets,
        controller: Option<AnyScrollController>,
        physics: Option<ScrollPhysicsRef>,
        shrink_wrap: bool,
    },
}

impl Layout {
    fn lay_out(&self, blocks: Vec<WidgetRef>, fit_content: bool) -> WidgetRef {
        match self {
            Layout::Column => render::column(blocks, fit_content),
            Layout::List {
                padding,
                controller,
                physics,
                shrink_wrap,
            } => {
                let mut list = ListView::new()
                    .padding((*padding).into())
                    .shrink_wrap(*shrink_wrap);
                if let Some(controller) = controller {
                    list = list.controller(*controller);
                }
                if let Some(physics) = physics {
                    list = list.physics(Rc::clone(physics));
                }
                list.children(blocks).into_widget()
            }
        }
    }
}

/// The widget both Markdown widgets build: it keeps the parsed document, the widgets made
/// from it and their link recognizers between builds, as flutter_markdown's state does.
struct MarkdownView {
    config: MarkdownConfig,
    layout: Layout,
}

impl fmt::Debug for MarkdownView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarkdownView")
            .field("data", &self.config.data)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MarkdownView {
    type State = MarkdownViewState;

    fn create_state(&self) -> MarkdownViewState {
        MarkdownViewState {
            state: StateData::new(),
            document: Rc::from([]),
            blocks: Vec::new(),
            recognizers: Vec::new(),
        }
    }
}

struct MarkdownViewState {
    state: StateData<MarkdownView>,
    /// Parsed from the widget's data and options whenever either changes.
    document: Rc<[Block]>,
    /// Made from the document when it, the style sheet or an inherited dependency changes, not
    /// on every build, so a rebuild in the middle of a tap keeps the link's recognizer.
    blocks: Vec<WidgetRef>,
    /// The recognizers of the links in [`blocks`](Self::blocks): disposed of when the blocks are
    /// made again, and on dispose.
    recognizers: Vec<AnyGestureRecognizer>,
}

impl MarkdownViewState {
    fn parse(self: Handle<Self>, app: &mut App) {
        let config = &self.widget(app).config;
        let document = document::parse(&config.data, config.options);
        app.get_mut(self).document = document.into();
    }

    fn dispose_recognizers(self: Handle<Self>, app: &mut App) {
        for recognizer in std::mem::take(&mut app.get_mut(self).recognizers) {
            recognizer.dispose(app);
        }
    }

    fn render(self: Handle<Self>, app: &mut App) {
        let (style_sheet, mut hooks, fit_content) = {
            let config = &self.widget(app).config;
            (
                config.style_sheet.clone(),
                config.hooks.clone(),
                config.fit_content,
            )
        };
        let style_sheet = match style_sheet {
            Some(style_sheet) => style_sheet,
            None => DefaultMarkdownStyle::of(app, self.context(app)),
        };
        if hooks.on_tap_link.is_some() {
            // flutter_markdown's link calls `widget.onTapLink`: the current widget's, at tap time.
            hooks.on_tap_link = Some(Rc::new(move |app: &mut App, url: &str, title: &str| {
                if let Some(on_tap_link) = self.widget(app).config.hooks.on_tap_link.clone() {
                    on_tap_link(app, url, title);
                }
            }));
        }
        self.dispose_recognizers(app);
        let document = Rc::clone(&app.get(self).document);
        let mut recognizers = Vec::new();
        let mut cx = RenderContext::new(&style_sheet, &hooks, fit_content, &mut recognizers);
        let blocks = render::blocks(&mut cx, app, &document, style_sheet.block_spacing);
        let state = app.get_mut(self);
        state.blocks = blocks;
        state.recognizers = recognizers;
    }
}

impl State for MarkdownViewState {
    type Widget = MarkdownView;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.parse(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.render(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MarkdownView) {
        let config = &self.widget(app).config;
        let parse =
            config.data != old_widget.config.data || config.options != old_widget.config.options;
        let restyle = config.style_sheet != old_widget.config.style_sheet;
        if parse {
            self.parse(app);
        }
        if parse || restyle {
            self.render(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_recognizers(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let (layout, fit_content) = {
            let widget = self.widget(app);
            (widget.layout.clone(), widget.config.fit_content)
        };
        let blocks = app.get(self).blocks.clone();
        layout.lay_out(blocks, fit_content)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use inset_embedder::{
        Brightness, Color, FontStyle, FontWeight, Offset, TextAlign, TextBaseline, TextDecoration,
    };
    use inset_foundation::AppCell;
    use inset_painting::{AlignmentGeometry, BoxDecoration, TextStyle};
    use inset_rendering::{
        AnyRenderAbstractViewport, AnyRenderObject, RenderBox, RenderCustomPaint,
        RenderDecoratedBox, RenderParagraph, RenderTable,
    };
    use inset_widgets::{
        Align, Builder, Directionality, GlobalKey, Padding, SizedBox, TextDirection,
    };

    use super::*;
    use crate::test_support::{
        build, descendants, find_all, global_rect_of_text, paragraph_texts, pump, render_object_of,
        span_with_text, tap_at, test_cell,
    };

    /// An agent's chat reply using every element the app shows.
    const REPLY: &str = "\
# Poster plan

Here is a **bold** idea, an *italic* one, a ~~struck~~ one and `inline code`.
See [Seede](https://seede.ai \"home\") for more.

## Steps

- first
- second
  - nested
- third

1. one
2. two

> Keep the title short.

```rust
fn main() {}
```

---

| Name | Size |
|:-----|-----:|
| Poster | A2 |
| Deck | 16:9 |
";

    /// Mounts `child` as the first item of a list in a side panel `width` wide, and returns
    /// the root render object of the item.
    fn mount_in_panel<K>(cell: &AppCell, width: f64, child: impl IntoWidget<K>) -> AnyRenderObject {
        let key = GlobalKey::new();
        let item_key: KeyRef = Rc::new(key.clone());
        let panel = SizedBox::new().width(width).height(600.0).child(
            ListView::new().children([
                Padding::new(EdgeInsets::all(8.0).into())
                    .key(item_key)
                    .child(child)
                    .into_widget(),
                SizedBox::new().height(40.0).into_widget(),
            ]),
        );
        build(
            cell,
            Directionality::new(
                TextDirection::Ltr,
                Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(panel),
            )
            .into_widget(),
        );
        render_object_of(&mut cell.borrow_mut(), &key)
    }

    /// The sheet the widgets fall back on in the test app: the empty ambient text style, light.
    fn fallback_sheet() -> MarkdownStyleSheet {
        MarkdownStyleSheet::from_text_style(TextStyle::new(), Brightness::Light)
    }

    #[test]
    fn a_reply_lays_out_in_a_narrow_side_panel() {
        for width in [320.0, 480.0] {
            let cell = test_cell();
            let item = mount_in_panel(&cell, width, MarkdownBody::new(REPLY));
            let app = cell.borrow();
            let item_box = item.as_box().expect("a box");
            assert_eq!(item_box.size(&app).width(), width);
            assert!(item_box.size(&app).height() > 200.0);

            let texts = paragraph_texts(&app, item).join("\n");
            for expected in [
                "Poster plan",
                "Steps",
                "bold",
                "inline code",
                "Seede",
                "•",
                "first",
                "nested",
                "1.",
                "2.",
                "Keep the title short.",
                "fn main() {}",
                "Poster",
                "16:9",
            ] {
                assert!(texts.contains(expected), "{expected:?} at {width}: {texts}");
            }
            for paragraph in find_all::<RenderParagraph>(&app, item) {
                assert!(
                    paragraph.size(&app).width() <= width,
                    "no text overflows the panel"
                );
            }
        }
    }

    #[test]
    fn each_element_takes_its_style_from_the_sheet() {
        let cell = test_cell();
        let item = mount_in_panel(&cell, 400.0, MarkdownBody::new(REPLY));
        let app = cell.borrow();
        let sheet = fallback_sheet();

        let (_, bold) = span_with_text(&app, item, "bold");
        assert_eq!(bold.style.unwrap().font_weight, Some(FontWeight::BOLD));
        let (_, italic) = span_with_text(&app, item, "italic");
        assert_eq!(italic.style.unwrap().font_style, Some(FontStyle::Italic));
        let (_, struck) = span_with_text(&app, item, "struck");
        assert_eq!(
            struck.style.unwrap().decoration,
            Some(TextDecoration::LINE_THROUGH)
        );
        let (_, code) = span_with_text(&app, item, "inline code");
        assert_eq!(
            code.style.unwrap().font_family.as_deref(),
            Some("monospace")
        );
        let (_, title) = span_with_text(&app, item, "Poster plan");
        let title_style = title.style.unwrap();
        assert_eq!(title_style.font_size, sheet.headings[0].text.font_size);
        assert_eq!(title_style.font_weight, Some(FontWeight::W500));
        let (_, steps) = span_with_text(&app, item, "Steps");
        assert_eq!(
            steps.style.unwrap().font_size,
            sheet.headings[1].text.font_size
        );
        let (_, link) = span_with_text(&app, item, "Seede");
        assert_eq!(link.style.unwrap().color, sheet.link.color);
        assert!(
            link.recognizer.is_none(),
            "without a hook a link has nothing to call"
        );
    }

    #[test]
    fn a_tapped_link_calls_on_tap_link_with_its_url_and_title() {
        let cell = test_cell();
        let taps: Rc<RefCell<Vec<(String, String)>>> = Rc::default();
        let sink = Rc::clone(&taps);
        let body = MarkdownBody::new(REPLY).on_tap_link(move |_app, url, title| {
            sink.borrow_mut().push((url.to_owned(), title.to_owned()))
        });
        let item = mount_in_panel(&cell, 400.0, body);
        let mut app = cell.borrow_mut();
        let link = global_rect_of_text(&mut app, item, "Seede");
        tap_at(&mut app, link.center());
        assert_eq!(
            *taps.borrow(),
            [("https://seede.ai".to_owned(), "home".to_owned())]
        );
        let text = global_rect_of_text(&mut app, item, "for more");
        tap_at(&mut app, text.center());
        assert_eq!(taps.borrow().len(), 1, "plain text is not a link");
    }

    /// Bullets come before each item, numbers count from the list's start, and a nested
    /// item is indented past its parent.
    #[test]
    fn lists_have_bullets_numbers_and_nesting() {
        let cell = test_cell();
        let item = mount_in_panel(
            &cell,
            400.0,
            MarkdownBody::new("- outer\n  - inner\n\n3. three\n4. four\n"),
        );
        let mut app = cell.borrow_mut();
        let texts = paragraph_texts(&app, item);
        assert_eq!(texts.iter().filter(|text| *text == "•").count(), 2);
        assert!(texts.contains(&"3.".to_owned()) && texts.contains(&"4.".to_owned()));
        let outer = global_rect_of_text(&mut app, item, "outer");
        let inner = global_rect_of_text(&mut app, item, "inner");
        assert!(inner.left > outer.left, "the nested item is indented");
        assert!(inner.top > outer.top);
    }

    #[test]
    fn tight_and_loose_list_items_are_one_block_spacing_apart() {
        let gap = |markdown: &str| {
            let cell = test_cell();
            let item = mount_in_panel(&cell, 400.0, MarkdownBody::new(markdown));
            let mut app = cell.borrow_mut();
            let first = global_rect_of_text(&mut app, item, "first");
            let second = global_rect_of_text(&mut app, item, "second");
            second.top - first.bottom
        };
        let sheet = fallback_sheet();
        let tight = gap("- first\n- second\n");
        let loose = gap("- first\n\n- second\n");
        assert!((tight - sheet.block_spacing).abs() < 1e-9, "{tight}");
        assert!((loose - sheet.block_spacing).abs() < 1e-9, "{loose}");
    }

    /// A bullet larger than its text sits on the text's baseline, not on its top.
    #[test]
    fn a_bullet_sits_on_the_baseline_of_its_text() {
        let cell = test_cell();
        let mut sheet = MarkdownStyleSheet::from_text_style(
            TextStyle::new().font_size(12.0),
            Brightness::Light,
        );
        sheet.list.bullet = TextStyle::new().font_size(32.0);
        let item = mount_in_panel(
            &cell,
            400.0,
            MarkdownBody::new("- item\n").style_sheet(sheet),
        );
        let mut app = cell.borrow_mut();
        let global_baseline = |app: &mut App, text: &str| {
            let (paragraph, _) = span_with_text(app, item, text);
            let paragraph = paragraph.as_box();
            let baseline = paragraph
                .get_distance_to_baseline(app, TextBaseline::Alphabetic, true)
                .expect("a paragraph has a baseline");
            let top = paragraph.local_to_global(app, Offset::ZERO, None).dy();
            (top, top + baseline)
        };
        let (bullet_top, bullet_baseline) = global_baseline(&mut app, "•");
        let (text_top, text_baseline) = global_baseline(&mut app, "item");
        assert!(bullet_top < text_top, "the larger bullet starts higher");
        assert!((bullet_baseline - text_baseline).abs() < 1e-9);
    }

    #[test]
    fn a_table_has_a_head_row_striped_body_rows_and_aligned_cells() {
        let cell = test_cell();
        let item = mount_in_panel(
            &cell,
            400.0,
            MarkdownBody::new(
                "| Name | Size |\n|------|-----:|\n| Poster | A2 |\n| Deck | 16:9 |\n| Card | A6 |\n",
            ),
        );
        let app = cell.borrow();
        let tables = find_all::<RenderTable>(&app, item);
        assert_eq!(tables.len(), 1);
        let table = tables[0];
        assert_eq!((table.columns(&app), table.rows(&app)), (2, 4));
        assert!(table.border(&app).is_some());
        assert_eq!(table.size(&app).width(), 400.0 - 16.0);
        let decorated: Vec<bool> = table
            .row_decorations(&app)
            .iter()
            .map(Option::is_some)
            .collect();
        assert_eq!(decorated, [false, false, true, false]);

        let (head, name) = span_with_text(&app, item, "Name");
        assert_eq!(name.style.unwrap().font_weight, Some(FontWeight::W600));
        assert_eq!(
            head.text_align(&app),
            TextAlign::Center,
            "an unaligned head is centred"
        );
        let (size, _) = span_with_text(&app, item, "Size");
        assert_eq!(size.text_align(&app), TextAlign::Right);
        let (poster, _) = span_with_text(&app, item, "Poster");
        assert_eq!(poster.text_align(&app), TextAlign::Start);
        let (a2, _) = span_with_text(&app, item, "A2");
        assert_eq!(a2.text_align(&app), TextAlign::Right);
    }

    /// The quote, the code block and the rule each paint their decoration, and the code
    /// scrolls sideways.
    #[test]
    fn blocks_paint_their_decorations_and_code_scrolls_sideways() {
        let cell = test_cell();
        let item = mount_in_panel(&cell, 400.0, MarkdownBody::new(REPLY));
        let app = cell.borrow();
        assert!(find_all::<RenderDecoratedBox>(&app, item).len() >= 3);
        let (code_paragraph, code) = span_with_text(&app, item, "fn main");
        assert_eq!(code.text.as_deref(), Some("fn main() {}"));
        assert!(
            is_in_viewport(&app, item, code_paragraph.as_object()),
            "a code block scrolls sideways"
        );
    }

    fn is_in_viewport(app: &App, root: AnyRenderObject, object: AnyRenderObject) -> bool {
        let viewports: Vec<AnyRenderObject> = descendants(app, root)
            .into_iter()
            .filter(|object| object.interface::<AnyRenderAbstractViewport>().is_some())
            .collect();
        let mut ancestor = object.parent(app);
        while let Some(current) = ancestor {
            if viewports.contains(&current) {
                return true;
            }
            ancestor = current.parent(app);
        }
        false
    }

    #[test]
    fn the_highlighter_spans_a_code_blocks_text() {
        let cell = test_cell();
        let red = Color::new(0xFFFF0000);
        let languages: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
        let sink = Rc::clone(&languages);
        let body = MarkdownBody::new("```rust\nfn main() {}\n```\n").highlight_code(
            move |_app, language, code| {
                sink.borrow_mut().push(language.map(str::to_owned));
                let (keyword, rest) = code.split_at(2);
                TextSpan::new().children(vec![
                    TextSpan::new()
                        .text(keyword)
                        .style(TextStyle::new().color(red))
                        .into_span(),
                    TextSpan::new().text(rest).into_span(),
                ])
            },
        );
        let item = mount_in_panel(&cell, 400.0, body);
        let app = cell.borrow();
        assert_eq!(*languages.borrow(), [Some("rust".to_owned())]);
        let (paragraph, keyword) = span_with_text(&app, item, "fn");
        assert_eq!(keyword.style.unwrap().color.as_deref(), Some(&red));
        assert_eq!(
            paragraph.text(&app).to_plain_text(true, true),
            "fn main() {}"
        );
        // `Text::rich` wraps the block's span in one carrying the ambient text style.
        let text = paragraph.text(&app);
        let wrapper = text
            .as_any()
            .downcast_ref::<TextSpan>()
            .expect("a text span");
        let block_span = wrapper.children.as_ref().expect("the block's span")[0].clone();
        let block_style = block_span.style().cloned().expect("the block's style");
        assert_eq!(block_style.font_family.as_deref(), Some("monospace"));
        assert!(is_in_viewport(&app, item, paragraph.as_object()));
    }

    #[test]
    fn a_default_markdown_style_above_sets_the_look() {
        let red = Color::new(0xFFFF0000);
        let blue = Color::new(0xFF0000FF);
        let red_sheet =
            MarkdownStyleSheet::from_text_style(TextStyle::new().color(red), Brightness::Light);
        let cell = test_cell();
        let item = mount_in_panel(
            &cell,
            400.0,
            DefaultMarkdownStyle::new(red_sheet.clone(), MarkdownBody::new("plain words")),
        );
        let (_, span) = span_with_text(&cell.borrow(), item, "plain words");
        assert_eq!(span.style.unwrap().color.as_deref(), Some(&red));

        let mut blue_sheet = red_sheet.clone();
        blue_sheet.text = TextStyle::new().color(blue);
        let cell = test_cell();
        let item = mount_in_panel(
            &cell,
            400.0,
            DefaultMarkdownStyle::new(
                red_sheet,
                MarkdownBody::new("plain words").style_sheet(blue_sheet),
            ),
        );
        let (_, span) = span_with_text(&cell.borrow(), item, "plain words");
        assert_eq!(
            span.style.unwrap().color.as_deref(),
            Some(&blue),
            "the widget's own sheet replaces the inherited one"
        );
    }

    #[test]
    fn the_hooks_build_images_checkboxes_and_bullets() {
        let cell = test_cell();
        let images: Rc<RefCell<Vec<Image>>> = Rc::default();
        let checkboxes: Rc<RefCell<Vec<bool>>> = Rc::default();
        let bullets: Rc<RefCell<Vec<Bullet>>> = Rc::default();
        let (image_sink, checkbox_sink, bullet_sink) = (
            Rc::clone(&images),
            Rc::clone(&checkboxes),
            Rc::clone(&bullets),
        );
        let body = MarkdownBody::new(
            "![a cat](cat.png \"Cat\")\n\n- [x] done\n- [ ] todo\n\n2. two\n   - under\n",
        )
        .image(move |_app, image| {
            image_sink.borrow_mut().push(image.clone());
            SizedBox::new().width(100.0).height(50.0).into_widget()
        })
        .checkbox(move |_app, checked| {
            checkbox_sink.borrow_mut().push(checked);
            SizedBox::new().width(10.0).height(10.0).into_widget()
        })
        .bullet(move |_app, bullet| {
            bullet_sink.borrow_mut().push(*bullet);
            SizedBox::new().width(10.0).height(10.0).into_widget()
        });
        mount_in_panel(&cell, 400.0, body);
        assert_eq!(
            *images.borrow(),
            [Image {
                url: "cat.png".to_owned(),
                title: "Cat".to_owned(),
                alt: "a cat".to_owned(),
            }]
        );
        assert_eq!(*checkboxes.borrow(), [true, false]);
        assert_eq!(
            *bullets.borrow(),
            [
                Bullet {
                    number: Some(2),
                    index: 0,
                    depth: 0
                },
                Bullet {
                    number: None,
                    index: 0,
                    depth: 1
                },
            ]
        );
    }

    #[test]
    fn without_hooks_an_image_shows_its_alt_text_and_a_task_its_checkbox() {
        let cell = test_cell();
        let item = mount_in_panel(
            &cell,
            400.0,
            MarkdownBody::new("![a cat](cat.png)\n\n- [x] done\n- [ ] todo\n"),
        );
        let app = cell.borrow();
        let texts = paragraph_texts(&app, item);
        assert!(texts.contains(&"a cat".to_owned()), "{texts:?}");
        assert_eq!(find_all::<RenderCustomPaint>(&app, item).len(), 2);
    }

    #[test]
    fn link_recognizers_are_disposed_when_the_text_changes_and_on_unmount() {
        let cell = test_cell();
        let text = cell
            .borrow_mut()
            .new_entity(|_cx| Some("[one](https://one.example)".to_owned()));
        let tree = Builder::new({
            let text = text.clone();
            move |app, _context| match text.read(app).clone() {
                Some(text) => MarkdownBody::new(text)
                    .on_tap_link(|_app, _url, _title| {})
                    .into_widget(),
                None => SizedBox::new().into_widget(),
            }
        });
        let item = mount_in_panel(&cell, 400.0, tree);
        let recognizer_of = |app: &App, link: &str| {
            span_with_text(app, item, link)
                .1
                .recognizer
                .expect("a link with a hook has a recognizer")
        };
        let is_gone = |app: &App, recognizer: AnyGestureRecognizer| {
            !app.contains(recognizer.id()) || app.is_disposed(recognizer.id())
        };

        let first = recognizer_of(&cell.borrow(), "one");
        text.update(&mut cell.borrow_mut(), |text, cx| {
            *text = Some("[two](https://two.example)".to_owned());
            cx.notify();
        });
        pump(&mut cell.borrow_mut(), std::time::Duration::from_millis(16));
        let second = recognizer_of(&cell.borrow(), "two");
        assert!(is_gone(&cell.borrow(), first));
        assert!(!is_gone(&cell.borrow(), second));

        text.update(&mut cell.borrow_mut(), |text, cx| {
            *text = None;
            cx.notify();
        });
        pump(&mut cell.borrow_mut(), std::time::Duration::from_millis(32));
        assert!(is_gone(&cell.borrow(), second));
    }

    /// A rebuild with the same text keeps the links' recognizers, and a tap calls the newest
    /// `on_tap_link`, as flutter_markdown's link calls `widget.onTapLink`.
    #[test]
    fn a_rebuild_with_the_same_text_keeps_the_link_and_calls_the_newest_hook() {
        let cell = test_cell();
        let generation = cell.borrow_mut().new_entity(|_cx| 1u32);
        let taps: Rc<RefCell<Vec<u32>>> = Rc::default();
        let tree = Builder::new({
            let (generation, taps) = (generation.clone(), Rc::clone(&taps));
            move |app, _context| {
                let current = *generation.read(app);
                let taps = Rc::clone(&taps);
                MarkdownBody::new("[link](https://example.com)")
                    .on_tap_link(move |_app, _url, _title| taps.borrow_mut().push(current))
                    .into_widget()
            }
        });
        let item = mount_in_panel(&cell, 400.0, tree);
        let recognizer_of = |app: &App| {
            span_with_text(app, item, "link")
                .1
                .recognizer
                .expect("a link with a hook has a recognizer")
        };
        let first = recognizer_of(&cell.borrow());

        generation.update(&mut cell.borrow_mut(), |generation, cx| {
            *generation = 2;
            cx.notify();
        });
        pump(&mut cell.borrow_mut(), std::time::Duration::from_millis(16));

        let mut app = cell.borrow_mut();
        assert_eq!(recognizer_of(&app).id(), first.id());
        let link = global_rect_of_text(&mut app, item, "link");
        tap_at(&mut app, link.center());
        assert_eq!(*taps.borrow(), [2]);
    }

    #[test]
    fn the_scrolling_widget_lists_the_blocks() {
        let cell = test_cell();
        let key = GlobalKey::new();
        let markdown_key: KeyRef = Rc::new(key.clone());
        build(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Markdown::new("# Title\n\nOne paragraph.\n\nAnother.").key(markdown_key),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let root = render_object_of(&mut app, &key);
        assert_eq!(
            paragraph_texts(&app, root),
            ["Title", "One paragraph.", "Another."]
        );
        let title = global_rect_of_text(&mut app, root, "Title");
        assert_eq!((title.left, title.top), (16.0, 16.0), "the default padding");
    }

    #[test]
    fn a_quote_paints_its_decoration_with_the_sheets_colour() {
        let cell = test_cell();
        let green = Color::new(0xFF00FF00);
        let mut sheet = fallback_sheet();
        sheet.quote.decoration = BoxDecoration::new().color(green);
        let item = mount_in_panel(
            &cell,
            400.0,
            MarkdownBody::new("> quoted").style_sheet(sheet),
        );
        let app = cell.borrow();
        let decorations: Vec<_> = find_all::<RenderDecoratedBox>(&app, item)
            .into_iter()
            .filter_map(|decorated| {
                decorated
                    .decoration(&app)
                    .as_any()
                    .downcast_ref::<BoxDecoration>()
                    .cloned()
            })
            .collect();
        assert!(decorations.contains(&BoxDecoration::new().color(green)));
    }
}
