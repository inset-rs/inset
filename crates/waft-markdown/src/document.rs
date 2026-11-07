//! The typed document [`parse`] builds from pulldown-cmark's events, and the renderer reads.

pub use pulldown_cmark::Alignment;
use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

/// The extensions a document is parsed with unless a widget asks for others: GitHub's tables,
/// strikethrough and task lists.
pub const GITHUB_FLAVORED: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS);

/// A block of a document.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    /// `level` is 1 to 6.
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Quote(Vec<Block>),
    /// `start` is the first number of an ordered list, `None` for a bulleted one. A tight list
    /// is one whose items are not separated by blank lines.
    List {
        start: Option<u64>,
        tight: bool,
        items: Vec<ListItem>,
    },
    /// `language` is the first word of a fenced block's info string; `text` has no final
    /// line break.
    CodeBlock {
        language: Option<String>,
        text: String,
    },
    /// Every row, `head` included, has one cell per entry of `alignments`.
    Table {
        alignments: Vec<Alignment>,
        head: Vec<Cell>,
        rows: Vec<Vec<Cell>>,
    },
    Rule,
}

/// A run of text inside a block.
#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    /// Plain text; a soft line break in the source is a space here.
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    /// An e-mail autolink's `url` starts with `mailto:`.
    Link {
        url: String,
        title: String,
        content: Vec<Inline>,
    },
    Image(Image),
    /// A hard line break.
    LineBreak,
}

/// An image, as the document wrote it.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    /// The address, unparsed: often a relative path.
    pub url: String,
    pub title: String,
    /// The plain text of the image's description.
    pub alt: String,
}

/// An item of a [`Block::List`].
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    /// Whether a task item is ticked; `None` for an item that is not a task.
    pub checked: Option<bool>,
    pub content: Vec<Block>,
}

/// A cell of a [`Block::Table`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub content: Vec<Inline>,
}

/// The blocks of `markdown`, parsed by pulldown-cmark with `options`.
///
/// An HTML block is left out, and inline HTML stays as its source text.
pub fn parse(markdown: &str, options: Options) -> Vec<Block> {
    let mut builder = DocumentBuilder::default();
    for event in Parser::new_ext(markdown, options) {
        builder.event(event);
    }
    builder.finish()
}

/// A container being built while its end has not been seen.
enum Frame {
    Blocks {
        blocks: Vec<Block>,
        container: BlockContainer,
    },
    List {
        start: Option<u64>,
        tight: bool,
        items: Vec<ListItem>,
    },
    Inlines {
        inlines: Vec<Inline>,
        container: InlineContainer,
    },
    CodeBlock {
        language: Option<String>,
        text: String,
    },
    Table {
        alignments: Vec<Alignment>,
        head: Vec<Cell>,
        rows: Vec<Vec<Cell>>,
        row: Vec<Cell>,
    },
    /// Content that is not part of the document, such as an HTML or metadata block.
    Dropped,
}

enum BlockContainer {
    Quote,
    Item {
        checked: Option<bool>,
    },
    /// A container with no block of its own, such as a footnote definition: its blocks join
    /// the enclosing container's.
    Transparent,
}

enum InlineContainer {
    Paragraph,
    /// The text of a tight list item, which pulldown-cmark gives no paragraph of its own.
    ImplicitParagraph,
    Heading(u8),
    Emphasis,
    Strong,
    Strikethrough,
    Link {
        url: String,
        title: String,
    },
    Image {
        url: String,
        title: String,
    },
    Cell,
    /// Inline content with no node of its own, such as a superscript: it joins the enclosing
    /// run.
    Transparent,
}

#[derive(Default)]
struct DocumentBuilder {
    document: Vec<Block>,
    stack: Vec<Frame>,
}

impl DocumentBuilder {
    fn event(&mut self, event: Event<'_>) {
        if matches!(self.stack.last(), Some(Frame::Dropped)) {
            match event {
                Event::Start(_) => self.stack.push(Frame::Dropped),
                Event::End(_) => {
                    self.stack.pop();
                }
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.push_inline(Inline::Code(code.into_string())),
            Event::InlineHtml(html) => self.push_text(&html),
            Event::InlineMath(math) | Event::DisplayMath(math) => self.push_text(&math),
            Event::FootnoteReference(label) => self.push_text(&format!("[{label}]")),
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.push_inline(Inline::LineBreak),
            Event::Rule => self.push_block(Block::Rule),
            Event::TaskListMarker(checked) => self.check_item(checked),
            Event::Html(_) => {}
        }
    }

    fn finish(mut self) -> Vec<Block> {
        self.close_implicit_paragraph();
        self.document
    }

    fn start(&mut self, tag: Tag<'_>) {
        if is_block(&tag) {
            self.close_implicit_paragraph();
        }
        let frame = match tag {
            Tag::Paragraph => {
                self.loosen_list();
                inlines(InlineContainer::Paragraph)
            }
            Tag::Heading { level, .. } => inlines(InlineContainer::Heading(level as u8)),
            Tag::BlockQuote(_) => blocks(BlockContainer::Quote),
            Tag::CodeBlock(kind) => Frame::CodeBlock {
                language: match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().map(str::to_owned)
                    }
                    CodeBlockKind::Indented => None,
                },
                text: String::new(),
            },
            Tag::HtmlBlock | Tag::MetadataBlock(_) => Frame::Dropped,
            Tag::List(start) => Frame::List {
                start,
                tight: true,
                items: Vec::new(),
            },
            Tag::Item => blocks(BlockContainer::Item { checked: None }),
            Tag::Table(alignments) => Frame::Table {
                alignments,
                head: Vec::new(),
                rows: Vec::new(),
                row: Vec::new(),
            },
            Tag::TableHead | Tag::TableRow => return,
            Tag::TableCell => inlines(InlineContainer::Cell),
            Tag::FootnoteDefinition(_) | Tag::DefinitionList | Tag::DefinitionListDefinition => {
                blocks(BlockContainer::Transparent)
            }
            Tag::DefinitionListTitle => inlines(InlineContainer::Paragraph),
            Tag::Emphasis => self.inline_frame(InlineContainer::Emphasis),
            Tag::Strong => self.inline_frame(InlineContainer::Strong),
            Tag::Strikethrough => self.inline_frame(InlineContainer::Strikethrough),
            Tag::Superscript | Tag::Subscript => self.inline_frame(InlineContainer::Transparent),
            Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            } => self.inline_frame(InlineContainer::Link {
                url: match link_type {
                    LinkType::Email => format!("mailto:{dest_url}"),
                    _ => dest_url.into_string(),
                },
                title: title.into_string(),
            }),
            Tag::Image {
                dest_url, title, ..
            } => self.inline_frame(InlineContainer::Image {
                url: dest_url.into_string(),
                title: title.into_string(),
            }),
        };
        self.stack.push(frame);
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::TableHead => {
                if let Some(Frame::Table {
                    alignments,
                    head,
                    row,
                    ..
                }) = self.stack.last_mut()
                {
                    *head = padded_row(std::mem::take(row), alignments.len());
                }
                return;
            }
            TagEnd::TableRow => {
                if let Some(Frame::Table {
                    alignments,
                    rows,
                    row,
                    ..
                }) = self.stack.last_mut()
                {
                    rows.push(padded_row(std::mem::take(row), alignments.len()));
                }
                return;
            }
            _ => {}
        }
        // An implicit paragraph has no end event of its own: whatever ends, it ends first.
        self.close_implicit_paragraph();
        let Some(frame) = self.stack.pop() else {
            return;
        };
        match frame {
            Frame::Blocks {
                blocks: content,
                container,
            } => match container {
                BlockContainer::Quote => self.push_block(Block::Quote(content)),
                BlockContainer::Item { checked } => {
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(ListItem { checked, content });
                    }
                }
                BlockContainer::Transparent => {
                    for block in content {
                        self.push_block(block);
                    }
                }
            },
            Frame::List {
                start,
                tight,
                items,
            } => self.push_block(Block::List {
                start,
                tight,
                items,
            }),
            Frame::Inlines {
                inlines: content,
                container,
            } => self.close_inlines(content, container),
            Frame::CodeBlock { language, mut text } => {
                if text.ends_with('\n') {
                    text.pop();
                }
                self.push_block(Block::CodeBlock { language, text });
            }
            Frame::Table {
                alignments,
                head,
                rows,
                ..
            } => self.push_block(Block::Table {
                alignments,
                head,
                rows,
            }),
            Frame::Dropped => {}
        }
    }

    fn close_inlines(&mut self, content: Vec<Inline>, container: InlineContainer) {
        match container {
            InlineContainer::Paragraph | InlineContainer::ImplicitParagraph => {
                self.push_block(Block::Paragraph(content));
            }
            InlineContainer::Heading(level) => self.push_block(Block::Heading { level, content }),
            InlineContainer::Emphasis => self.push_inline(Inline::Emphasis(content)),
            InlineContainer::Strong => self.push_inline(Inline::Strong(content)),
            InlineContainer::Strikethrough => self.push_inline(Inline::Strikethrough(content)),
            InlineContainer::Link { url, title } => self.push_inline(Inline::Link {
                url,
                title,
                content,
            }),
            InlineContainer::Image { url, title } => self.push_inline(Inline::Image(Image {
                url,
                title,
                alt: plain_text(&content),
            })),
            InlineContainer::Cell => {
                if let Some(Frame::Table { row, .. }) = self.stack.last_mut() {
                    row.push(Cell { content });
                }
            }
            InlineContainer::Transparent => {
                for inline in content {
                    self.push_inline(inline);
                }
            }
        }
    }

    fn text(&mut self, text: &str) {
        match self.stack.last_mut() {
            Some(Frame::CodeBlock { text: code, .. }) => code.push_str(text),
            _ => self.push_text(text),
        }
    }

    /// Adds `text` to the open run, joining a text just before it.
    fn push_text(&mut self, text: &str) {
        let inlines = self.open_inlines();
        if let Some(Inline::Text(last)) = inlines.last_mut() {
            last.push_str(text);
        } else {
            inlines.push(Inline::Text(text.to_owned()));
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        self.open_inlines().push(inline);
    }

    /// The open run of inline content, opening a tight list item's paragraph when a block
    /// container is on top.
    fn open_inlines(&mut self) -> &mut Vec<Inline> {
        if !matches!(self.stack.last(), Some(Frame::Inlines { .. })) {
            self.stack.push(inlines(InlineContainer::ImplicitParagraph));
        }
        match self.stack.last_mut() {
            Some(Frame::Inlines { inlines, .. }) => inlines,
            _ => unreachable!("an inline frame was just pushed"),
        }
    }

    /// A frame for inline content that opens inside a run, opening the run first if need be.
    fn inline_frame(&mut self, container: InlineContainer) -> Frame {
        self.open_inlines();
        inlines(container)
    }

    fn push_block(&mut self, block: Block) {
        self.close_implicit_paragraph();
        match self.stack.last_mut() {
            Some(Frame::Blocks { blocks, .. }) => blocks.push(block),
            None => self.document.push(block),
            Some(_) => {}
        }
    }

    fn close_implicit_paragraph(&mut self) {
        if let Some(Frame::Inlines {
            container: InlineContainer::ImplicitParagraph,
            ..
        }) = self.stack.last()
            && let Some(Frame::Inlines { inlines, .. }) = self.stack.pop()
        {
            self.push_block(Block::Paragraph(inlines));
        }
    }

    /// pulldown-cmark wraps an item's text in a paragraph only in a loose list.
    fn loosen_list(&mut self) {
        let depth = self.stack.len();
        if depth >= 2
            && let [
                Frame::List { tight, .. },
                Frame::Blocks {
                    container: BlockContainer::Item { .. },
                    ..
                },
            ] = &mut self.stack[depth - 2..]
        {
            *tight = false;
        }
    }

    fn check_item(&mut self, is_checked: bool) {
        let item = self.stack.iter_mut().rev().find_map(|frame| match frame {
            Frame::Blocks {
                container: BlockContainer::Item { checked },
                ..
            } => Some(checked),
            _ => None,
        });
        if let Some(checked) = item {
            *checked = Some(is_checked);
        }
    }
}

fn blocks(container: BlockContainer) -> Frame {
    Frame::Blocks {
        blocks: Vec::new(),
        container,
    }
}

fn inlines(container: InlineContainer) -> Frame {
    Frame::Inlines {
        inlines: Vec::new(),
        container,
    }
}

fn is_block(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Table(_)
            | Tag::MetadataBlock(_)
    )
}

/// A row with exactly `columns` cells, as GitHub's tables fill a short row with empty cells and
/// drop a long row's extra ones.
fn padded_row(mut row: Vec<Cell>, columns: usize) -> Vec<Cell> {
    row.resize_with(columns, Cell::default);
    row
}

/// The text of `inlines` without their formatting.
fn plain_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(run) | Inline::Code(run) => text.push_str(run),
            Inline::Emphasis(content)
            | Inline::Strong(content)
            | Inline::Strikethrough(content)
            | Inline::Link { content, .. } => text.push_str(&plain_text(content)),
            Inline::Image(image) => text.push_str(&image.alt),
            Inline::LineBreak => text.push('\n'),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(text: &str) -> Inline {
        Inline::Text(text.to_owned())
    }

    fn paragraph(text: &str) -> Block {
        Block::Paragraph(vec![self::text(text)])
    }

    fn item(checked: Option<bool>, content: Vec<Block>) -> ListItem {
        ListItem { checked, content }
    }

    #[test]
    fn headings_and_paragraphs_carry_their_inline_formatting() {
        let document = parse(
            "# Title\n\n### Small\n\nSome *em*, **strong**, ~~gone~~ and `code`.\n\n> quoted\n\n---\n",
            GITHUB_FLAVORED,
        );
        assert_eq!(
            document,
            [
                Block::Heading {
                    level: 1,
                    content: vec![text("Title")],
                },
                Block::Heading {
                    level: 3,
                    content: vec![text("Small")],
                },
                Block::Paragraph(vec![
                    text("Some "),
                    Inline::Emphasis(vec![text("em")]),
                    text(", "),
                    Inline::Strong(vec![text("strong")]),
                    text(", "),
                    Inline::Strikethrough(vec![text("gone")]),
                    text(" and "),
                    Inline::Code("code".to_owned()),
                    text("."),
                ]),
                Block::Quote(vec![paragraph("quoted")]),
                Block::Rule,
            ]
        );
    }

    #[test]
    fn a_soft_break_is_a_space_and_a_hard_break_is_a_line_break() {
        assert_eq!(parse("one\ntwo", GITHUB_FLAVORED), [paragraph("one two")]);
        assert_eq!(
            parse("one  \ntwo", GITHUB_FLAVORED),
            [Block::Paragraph(vec![
                text("one"),
                Inline::LineBreak,
                text("two")
            ])]
        );
    }

    #[test]
    fn a_tight_list_nests_and_a_loose_one_keeps_its_paragraphs() {
        assert_eq!(
            parse("- outer\n  - inner\n- next\n", GITHUB_FLAVORED),
            [Block::List {
                start: None,
                tight: true,
                items: vec![
                    item(
                        None,
                        vec![
                            paragraph("outer"),
                            Block::List {
                                start: None,
                                tight: true,
                                items: vec![item(None, vec![paragraph("inner")])],
                            },
                        ]
                    ),
                    item(None, vec![paragraph("next")]),
                ],
            }]
        );
        assert_eq!(
            parse("3. three\n\n4. four\n\n   more\n", GITHUB_FLAVORED),
            [Block::List {
                start: Some(3),
                tight: false,
                items: vec![
                    item(None, vec![paragraph("three")]),
                    item(None, vec![paragraph("four"), paragraph("more")]),
                ],
            }]
        );
    }

    #[test]
    fn task_items_carry_whether_they_are_ticked() {
        let [Block::List { items, .. }] =
            &parse("- [x] done\n- [ ] todo\n- plain\n", GITHUB_FLAVORED)[..]
        else {
            panic!("one list");
        };
        assert_eq!(
            items,
            &[
                item(Some(true), vec![paragraph("done")]),
                item(Some(false), vec![paragraph("todo")]),
                item(None, vec![paragraph("plain")]),
            ]
        );
        let [Block::List { items, tight, .. }] =
            &parse("- [x] done\n\n- [ ] todo\n", GITHUB_FLAVORED)[..]
        else {
            panic!("one list");
        };
        assert!(!tight);
        assert_eq!(items[0], item(Some(true), vec![paragraph("done")]));
    }

    #[test]
    fn a_table_keeps_its_alignments_and_pads_short_rows() {
        let cell = |content: &str| Cell {
            content: vec![text(content)],
        };
        assert_eq!(
            parse(
                "| a | b | c |\n|:--|:-:|--:|\n| 1 | **2** |\n",
                GITHUB_FLAVORED
            ),
            [Block::Table {
                alignments: vec![Alignment::Left, Alignment::Center, Alignment::Right],
                head: vec![cell("a"), cell("b"), cell("c")],
                rows: vec![vec![
                    cell("1"),
                    Cell {
                        content: vec![Inline::Strong(vec![text("2")])],
                    },
                    Cell::default(),
                ]],
            }]
        );
    }

    #[test]
    fn a_code_block_keeps_its_language_and_drops_its_last_line_break() {
        assert_eq!(
            parse(
                "```rust title\nfn main() {}\n```\n\n    indented\n",
                GITHUB_FLAVORED
            ),
            [
                Block::CodeBlock {
                    language: Some("rust".to_owned()),
                    text: "fn main() {}".to_owned(),
                },
                Block::CodeBlock {
                    language: None,
                    text: "indented".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn links_and_images_carry_their_address_title_and_text() {
        assert_eq!(
            parse(
                "[Seede *home*](https://seede.ai \"home\") ![a **cat**](cat.png \"Cat\") <me@seede.ai>",
                GITHUB_FLAVORED,
            ),
            [Block::Paragraph(vec![
                Inline::Link {
                    url: "https://seede.ai".to_owned(),
                    title: "home".to_owned(),
                    content: vec![text("Seede "), Inline::Emphasis(vec![text("home")])],
                },
                text(" "),
                Inline::Image(Image {
                    url: "cat.png".to_owned(),
                    title: "Cat".to_owned(),
                    alt: "a cat".to_owned(),
                }),
                text(" "),
                Inline::Link {
                    url: "mailto:me@seede.ai".to_owned(),
                    title: String::new(),
                    content: vec![text("me@seede.ai")],
                },
            ])]
        );
    }

    #[test]
    fn an_html_block_is_left_out() {
        assert_eq!(
            parse(
                "<div>\nhidden\n</div>\n\nshown <b>bold</b>\n",
                GITHUB_FLAVORED
            ),
            [paragraph("shown <b>bold</b>")]
        );
    }
}
