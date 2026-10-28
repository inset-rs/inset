//! The default rendering of a document into widgets, one public function per element, so a
//! renderer that replaces some elements can fall back on these for the rest.

use std::any::Any;
use std::fmt;
use std::rc::Rc;

use inset_embedder::valo::{FillRule, Point};
use inset_embedder::{
    Canvas, Clip, Color, Paint, PathBuilder, PlaceholderAlignment, RRect, Size, TextAlign,
    TextBaseline, rrect_radii_elliptical,
};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::{AnyGestureRecognizer, GestureRecognizerLeaf, TapGestureRecognizer};
use inset_painting::{
    Axis, EdgeInsets, InlineSpanRef, K_DEFAULT_FONT_SIZE, TextSpan, TextStyle, draw_drrect,
};
use inset_rendering::{
    CrossAxisAlignment, CustomPainter, FixedColumnWidth, FlexFit, IntrinsicColumnWidth,
    MainAxisSize,
};
use inset_widgets::{
    BuildContext, Column, Container, CustomPaint, DecoratedBox, Flexible, GestureDetector,
    IntoWidget, Padding, RawScrollbar, Row, ScrollController, ScrollControllerLeaf,
    SingleChildScrollView, SizedBox, State, StateData, StatefulWidget, Table, TableCell, TableRow,
    Text, WidgetRef, WidgetSpan,
};

use crate::document::{Alignment, Block, Cell, Inline, ListItem};
use crate::style_sheet::MarkdownStyleSheet;
use crate::widget::MarkdownHooks;

/// What a list item's bullet stands for, given to [`bullet`] and to a widget's bullet hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bullet {
    /// The item's number in an ordered list, counted from the list's start; `None` in a
    /// bulleted list.
    pub number: Option<u64>,
    /// The item's position in its list, from 0.
    pub index: usize,
    /// How many lists enclose the item's list: 0 for a list at the top.
    pub depth: usize,
}

/// What the render functions read and record while they turn one document into widgets.
pub struct RenderContext<'a> {
    style_sheet: &'a MarkdownStyleSheet,
    hooks: &'a MarkdownHooks,
    fit_content: bool,
    /// The style of body text here: the sheet's, merged with each enclosing quote's.
    text: TextStyle,
    list_depth: usize,
    /// Every recognizer made for a link. Whoever made the context disposes of them once the
    /// spans holding them are replaced.
    recognizers: &'a mut Vec<AnyGestureRecognizer>,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        style_sheet: &'a MarkdownStyleSheet,
        hooks: &'a MarkdownHooks,
        fit_content: bool,
        recognizers: &'a mut Vec<AnyGestureRecognizer>,
    ) -> RenderContext<'a> {
        RenderContext {
            style_sheet,
            hooks,
            fit_content,
            text: style_sheet.text.clone(),
            list_depth: 0,
            recognizers,
        }
    }

    /// What tapping a link does, or `None` when the widget has no link hook.
    fn link_tap(&mut self, app: &mut App, url: &str, title: &str) -> Option<LinkTap> {
        let on_tap_link = self.hooks.on_tap_link.clone()?;
        let (url, title) = (url.to_owned(), title.to_owned());
        let on_tap = Listener::new(move |app| on_tap_link(app, &url, &title));
        let recognizer = TapGestureRecognizer::new(app);
        recognizer.set_on_tap(app, Some(on_tap.clone()));
        let recognizer = recognizer.as_recognizer();
        self.recognizers.push(recognizer);
        Some(LinkTap { recognizer, on_tap })
    }
}

/// The widgets of `blocks`, `spacing` apart.
pub fn blocks(
    cx: &mut RenderContext,
    app: &mut App,
    blocks: &[Block],
    spacing: f64,
) -> Vec<WidgetRef> {
    spaced(
        blocks.iter().map(|block| self::block(cx, app, block)),
        spacing,
    )
}

/// The widget of one block.
pub fn block(cx: &mut RenderContext, app: &mut App, block: &Block) -> WidgetRef {
    match block {
        Block::Paragraph(content) => paragraph(cx, app, content),
        Block::Heading { level, content } => heading(cx, app, *level, content),
        Block::Quote(blocks) => quote(cx, app, blocks),
        Block::List { start, items, .. } => list(cx, app, *start, items),
        Block::CodeBlock { language, text } => code_block(cx, app, language.as_deref(), text),
        Block::Table {
            alignments,
            head,
            rows,
        } => table(cx, app, alignments, head, rows),
        Block::Rule => rule(cx),
    }
}

/// One text of the paragraph's spans.
pub fn paragraph(cx: &mut RenderContext, app: &mut App, content: &[Inline]) -> WidgetRef {
    let style = cx.text.clone();
    rich_text(spans(cx, app, content, &style), style, TextAlign::Start)
}

/// The heading's text in its level's style.
pub fn heading(cx: &mut RenderContext, app: &mut App, level: u8, content: &[Inline]) -> WidgetRef {
    let heading = &cx.style_sheet.headings[usize::from(level.clamp(1, 6)) - 1];
    let style = cx.text.merge(Some(&heading.text));
    padded(
        heading.padding,
        rich_text(spans(cx, app, content, &style), style, TextAlign::Start),
    )
}

/// The quote's blocks, padded inside its decoration.
pub fn quote(cx: &mut RenderContext, app: &mut App, blocks: &[Block]) -> WidgetRef {
    let sheet = cx.style_sheet;
    let quoted = cx.text.merge(Some(&sheet.quote.text));
    let outer = std::mem::replace(&mut cx.text, quoted);
    let children = self::blocks(cx, app, blocks, sheet.block_spacing);
    cx.text = outer;
    DecoratedBox::new(sheet.quote.decoration.clone())
        .child(Padding::new(sheet.quote.padding.into()).child(column(children, cx.fit_content)))
        .into_widget()
}

/// A row per item, its bullet, number or checkbox in the indent before its blocks, the items
/// [`block_spacing`](MarkdownStyleSheet::block_spacing) apart whether the list is tight or
/// loose, as flutter_markdown spaces them.
pub fn list(
    cx: &mut RenderContext,
    app: &mut App,
    start: Option<u64>,
    items: &[ListItem],
) -> WidgetRef {
    let (sheet, hooks) = (cx.style_sheet, cx.hooks);
    let spacing = sheet.block_spacing;
    let depth = cx.list_depth;
    cx.list_depth += 1;
    let mut rows = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let marker = match item.checked {
            Some(checked) => match &hooks.checkbox {
                Some(hook) => hook(app, checked),
                None => checkbox(cx, checked),
            },
            None => {
                let marker = Bullet {
                    number: start.map(|start| start + index as u64),
                    index,
                    depth,
                };
                match &hooks.bullet {
                    Some(hook) => hook(app, &marker),
                    None => bullet(cx, &marker),
                }
            }
        };
        let content = column(
            self::blocks(cx, app, &item.content, spacing),
            cx.fit_content,
        );
        rows.push(list_item(cx, marker, content));
    }
    cx.list_depth = depth;
    column(spaced(rows, spacing), cx.fit_content)
}

/// The default bullet: a dot, or the item's number and a full stop.
pub fn bullet(cx: &RenderContext, bullet: &Bullet) -> WidgetRef {
    let style = cx.text.merge(Some(&cx.style_sheet.list.bullet));
    let (text, text_align) = match bullet.number {
        None => ("•".to_owned(), TextAlign::Center),
        Some(number) => (format!("{number}."), TextAlign::Right),
    };
    Text::new(text)
        .style(style)
        .text_align(text_align)
        .into_widget()
}

/// The default checkbox: Material's checkbox glyph, ticked or empty, drawn in the middle of a
/// line of the sheet's checkbox style.
pub fn checkbox(cx: &RenderContext, checked: bool) -> WidgetRef {
    let style = cx.text.merge(Some(&cx.style_sheet.list.checkbox));
    let size = style.font_size.unwrap_or(K_DEFAULT_FONT_SIZE);
    let color = style
        .color
        .as_deref()
        .copied()
        .unwrap_or(Color::new(0xFF000000));
    let glyph = CustomPaint::new()
        .painter(CheckboxPainter { checked, color })
        .size(Size::new(size, size));
    let line = TextSpan::new().children(vec![
        WidgetSpan::new(glyph)
            .alignment(PlaceholderAlignment::Middle)
            .into_span(),
    ]);
    Text::rich(line.style(style).into_span()).into_widget()
}

/// The code in the code block's style, or as the widget's highlighter spans it, scrolling
/// sideways inside the block's decoration.
pub fn code_block(
    cx: &mut RenderContext,
    app: &mut App,
    language: Option<&str>,
    text: &str,
) -> WidgetRef {
    let block = &cx.style_sheet.code_block;
    let code = match &cx.hooks.highlight_code {
        Some(highlight) => {
            TextSpan::new().children(vec![highlight(app, language, text).into_span()])
        }
        None => TextSpan::new().text(text),
    };
    let style = cx.text.merge(Some(&block.text));
    Container::new()
        .clip_behavior(Clip::HardEdge)
        .decoration(block.decoration.clone())
        .child(SidewaysScroll {
            padding: block.padding,
            child: Text::rich(code.style(style).into_span()).into_widget(),
        })
        .into_widget()
}

/// A [`Table`] of the head row and the body rows, each cell aligned as its column; it scrolls
/// sideways when its columns size to their content.
pub fn table(
    cx: &mut RenderContext,
    app: &mut App,
    alignments: &[Alignment],
    head: &[Cell],
    rows: &[Vec<Cell>],
) -> WidgetRef {
    let table = &cx.style_sheet.table;
    let head_style = cx.text.merge(Some(&table.head));
    let body_style = cx.text.merge(Some(&table.body));
    let mut table_rows = Vec::with_capacity(rows.len() + 1);
    table_rows.push(TableRow::new().children(table_cells(
        cx,
        app,
        alignments,
        head,
        &head_style,
        table.head_align,
    )));
    for (index, row) in rows.iter().enumerate() {
        let mut table_row = TableRow::new().children(table_cells(
            cx,
            app,
            alignments,
            row,
            &body_style,
            TextAlign::Start,
        ));
        if index % 2 == 1
            && let Some(stripe) = &table.stripe
        {
            table_row = table_row.decoration(stripe.clone());
        }
        table_rows.push(table_row);
    }
    let widget = Table::new()
        .default_column_width(Rc::clone(&table.column_width))
        .default_vertical_alignment(table.vertical_alignment)
        .border(table.border)
        .children(table_rows)
        .into_widget();
    let column_width = table.column_width.as_any();
    if column_width.is::<IntrinsicColumnWidth>() || column_width.is::<FixedColumnWidth>() {
        return SidewaysScroll {
            padding: EdgeInsets::ZERO,
            child: widget,
        }
        .into_widget();
    }
    widget
}

/// A line across the full width, the sheet's rule decoration on a box of no height.
pub fn rule(cx: &RenderContext) -> WidgetRef {
    Container::new()
        .decoration(cx.style_sheet.rule.clone())
        .into_widget()
}

/// The spans of `content` in text of `style`. A link's spans carry a recognizer that calls the
/// widget's link hook; an image is the widget's image hook's widget, or its alt text.
pub fn spans(
    cx: &mut RenderContext,
    app: &mut App,
    content: &[Inline],
    style: &TextStyle,
) -> Vec<InlineSpanRef> {
    let mut spans = Vec::new();
    push_spans(cx, app, content, style, None, &mut spans);
    spans
}

/// What tapping a link's text or image does.
struct LinkTap {
    recognizer: AnyGestureRecognizer,
    on_tap: Listener,
}

fn push_spans(
    cx: &mut RenderContext,
    app: &mut App,
    content: &[Inline],
    style: &TextStyle,
    link: Option<&LinkTap>,
    spans: &mut Vec<InlineSpanRef>,
) {
    let (sheet, hooks) = (cx.style_sheet, cx.hooks);
    for inline in content {
        match inline {
            Inline::Text(text) => spans.push(text_span(text, style, link)),
            Inline::Emphasis(content) => {
                let style = MarkdownStyleSheet::emphasis(style);
                push_spans(cx, app, content, &style, link, spans);
            }
            Inline::Strong(content) => {
                let style = MarkdownStyleSheet::strong(style);
                push_spans(cx, app, content, &style, link, spans);
            }
            Inline::Strikethrough(content) => {
                let style = MarkdownStyleSheet::strikethrough(style);
                push_spans(cx, app, content, &style, link, spans);
            }
            Inline::Code(code) => {
                spans.push(text_span(
                    code,
                    &style.merge(Some(&sheet.inline_code)),
                    link,
                ));
            }
            Inline::Link {
                url,
                title,
                content,
            } => {
                if content.is_empty() {
                    continue;
                }
                let style = style.merge(Some(&sheet.link));
                let tap = cx.link_tap(app, url, title);
                push_spans(cx, app, content, &style, tap.as_ref(), spans);
            }
            Inline::Image(image) => match &hooks.image {
                Some(hook) => {
                    let mut child = hook(app, image);
                    if let Some(link) = link {
                        child = GestureDetector::new()
                            .on_tap(link.on_tap.clone())
                            .child(child)
                            .into_widget();
                    }
                    spans.push(WidgetSpan::new(child).into_span());
                }
                None if !image.alt.is_empty() => spans.push(text_span(&image.alt, style, link)),
                None => {}
            },
            Inline::LineBreak => spans.push(text_span("\n", style, link)),
        }
    }
}

fn text_span(text: &str, style: &TextStyle, link: Option<&LinkTap>) -> InlineSpanRef {
    let mut span = TextSpan::new().text(text).style(style.clone());
    if let Some(link) = link {
        span = span.recognizer(link.recognizer);
    }
    span.into_span()
}

fn rich_text(spans: Vec<InlineSpanRef>, style: TextStyle, text_align: TextAlign) -> WidgetRef {
    Text::rich(TextSpan::new().style(style).children(spans).into_span())
        .text_align(text_align)
        .into_widget()
}

fn table_cells(
    cx: &mut RenderContext,
    app: &mut App,
    alignments: &[Alignment],
    cells: &[Cell],
    style: &TextStyle,
    unaligned: TextAlign,
) -> Vec<WidgetRef> {
    let padding = cx.style_sheet.table.cell_padding;
    alignments
        .iter()
        .enumerate()
        .map(|(column, alignment)| {
            let text_align = match alignment {
                Alignment::None => unaligned,
                Alignment::Left => TextAlign::Left,
                Alignment::Center => TextAlign::Center,
                Alignment::Right => TextAlign::Right,
            };
            let content = cells.get(column).map_or(&[][..], |cell| &cell.content);
            let text = rich_text(spans(cx, app, content, style), style.clone(), text_align);
            TableCell::new(Padding::new(padding.into()).child(text)).into_widget()
        })
        .collect()
}

fn list_item(cx: &RenderContext, marker: WidgetRef, content: WidgetRef) -> WidgetRef {
    let list = &cx.style_sheet.list;
    let padding = list.bullet_padding;
    Row::new()
        .main_axis_size(if cx.fit_content {
            MainAxisSize::Min
        } else {
            MainAxisSize::Max
        })
        .cross_axis_alignment(CrossAxisAlignment::Baseline)
        .text_baseline(TextBaseline::Alphabetic)
        .children([
            SizedBox::new()
                .width(list.indent + padding.left + padding.right)
                .child(Padding::new(padding.into()).child(marker))
                .into_widget(),
            Flexible::new(content)
                .fit(if cx.fit_content {
                    FlexFit::Loose
                } else {
                    FlexFit::Tight
                })
                .into_widget(),
        ])
        .into_widget()
}

/// `children` in a column that takes their width when `fit_content`, the full width otherwise.
pub(crate) fn column(children: Vec<WidgetRef>, fit_content: bool) -> WidgetRef {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(if fit_content {
            CrossAxisAlignment::Start
        } else {
            CrossAxisAlignment::Stretch
        })
        .children(children)
        .into_widget()
}

fn spaced(children: impl IntoIterator<Item = WidgetRef>, spacing: f64) -> Vec<WidgetRef> {
    let mut spaced = Vec::new();
    for child in children {
        if !spaced.is_empty() {
            spaced.push(SizedBox::new().height(spacing).into_widget());
        }
        spaced.push(child);
    }
    spaced
}

fn padded(padding: EdgeInsets, child: WidgetRef) -> WidgetRef {
    if padding == EdgeInsets::ZERO {
        return child;
    }
    Padding::new(padding.into()).child(child).into_widget()
}

/// Material's `check_box` and `check_box_outline_blank` icons, drawn on their 24-unit grid.
#[derive(PartialEq)]
struct CheckboxPainter {
    checked: bool,
    color: Color,
}

impl CustomPainter for CheckboxPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let unit = size.width().min(size.height()) / 24.0;
        let square = |inset: f64, radius: f64| {
            RRect::from_ltrbxy(
                inset * unit,
                inset * unit,
                (24.0 - inset) * unit,
                (24.0 - inset) * unit,
                radius * unit,
                radius * unit,
            )
        };
        let paint = Paint::from_color(self.color.into());
        let outer = square(3.0, 2.0);
        if !self.checked {
            draw_drrect(canvas, outer, square(5.0, 0.0), &paint);
            return;
        }
        // The tick is a hole in the filled square.
        let mut path = PathBuilder::new();
        path.rrect_radii_elliptical(outer.outer_rect(), rrect_radii_elliptical(outer));
        let point = |x: f64, y: f64| Point::new((x * unit) as f32, (y * unit) as f32);
        path.move_to(point(10.0, 17.0));
        for (x, y) in [
            (5.0, 12.0),
            (6.41, 10.59),
            (10.0, 14.17),
            (17.59, 6.58),
            (19.0, 8.0),
        ] {
            path.line_to(point(x, y));
        }
        path.close();
        canvas.draw_path(&path.build(), FillRule::EvenOdd, &paint);
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<CheckboxPainter>() != Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `child` scrolling sideways under a [`RawScrollbar`], with a scroll controller of its own.
struct SidewaysScroll {
    padding: EdgeInsets,
    child: WidgetRef,
}

impl fmt::Debug for SidewaysScroll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SidewaysScroll")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SidewaysScroll {
    type State = SidewaysScrollState;

    fn create_state(&self) -> SidewaysScrollState {
        SidewaysScrollState {
            state: StateData::new(),
            controller: None,
        }
    }
}

struct SidewaysScrollState {
    state: StateData<SidewaysScroll>,
    /// Made in `init_state`, disposed of in `dispose`.
    controller: Option<Handle<ScrollController>>,
}

impl State for SidewaysScrollState {
    type Widget = SidewaysScroll;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = ScrollController::default(app);
        app.get_mut(self).controller = Some(controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get_mut(self).controller.take() {
            controller.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let controller = app
            .get(self)
            .controller
            .expect("made in init_state")
            .as_controller();
        let widget = self.widget(app);
        let scroll_view = SingleChildScrollView::new()
            .controller(controller)
            .scroll_direction(Axis::Horizontal)
            .padding(widget.padding.into())
            .child(widget.child.clone());
        RawScrollbar::new(scroll_view)
            .controller(controller)
            .into_widget()
    }
}
