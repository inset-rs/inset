//! The look of rendered Markdown, and the inherited widget that sets it for a subtree.

use std::rc::Rc;

use inset_embedder::{Brightness, Color, FontStyle, FontWeight, TextAlign, TextDecoration};
use inset_foundation::App;
use inset_painting::{
    Border, BorderRadius, BorderSide, BorderStyle, BoxDecoration, EdgeInsets, K_DEFAULT_FONT_SIZE,
    TextStyle,
};
use inset_rendering::{
    FlexColumnWidth, TableBorder, TableCellVerticalAlignment, TableColumnWidthRef,
};
use inset_widgets::{
    BuildContext, DefaultTextStyle, InheritedWidget, IntoWidget, KeyRef, MediaQuery, WidgetRef,
};

/// The light and dark values of Apple's system colors [`MarkdownStyleSheet::from_text_style`]
/// paints with, the ones flutter_markdown's Cupertino style sheet reads.
mod system_colors {
    use inset_embedder::Color;

    pub(super) const LINK: (Color, Color) = (Color::new(0xFF007AFF), Color::new(0xFF0984FF));
    pub(super) const BLUE: (Color, Color) = (Color::new(0xFF007AFF), Color::new(0xFF0A84FF));
    pub(super) const GREY_6: (Color, Color) = (Color::new(0xFFF2F2F7), Color::new(0xFF1C1C1E));
    pub(super) const GREY_4: (Color, Color) = (Color::new(0xFFD1D1D6), Color::new(0xFF3A3A3C));
    pub(super) const SEPARATOR: (Color, Color) = (Color::new(0x493C3C43), Color::new(0x99545458));
}

/// How each element of a Markdown document looks.
///
/// [`text`](Self::text) is the style of body text. Every other text style is merged onto the
/// style of the text around the element, so it may set only what differs: a link's color, a
/// heading's size.
///
/// A sheet is replaced whole, never merged field by field: build one from
/// [`from_text_style`](Self::from_text_style) or [`DefaultMarkdownStyle::of`] and change the
/// fields that should differ.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownStyleSheet {
    pub text: TextStyle,
    /// The headings from level 1 to 6.
    pub headings: [HeadingStyle; 6],
    pub link: TextStyle,
    pub inline_code: TextStyle,
    pub code_block: BlockStyle,
    pub quote: BlockStyle,
    pub list: ListStyle,
    pub table: TableStyle,
    /// Painted on a box of the full width and no height.
    pub rule: BoxDecoration,
    /// The vertical space between blocks, and between the items of a loose list.
    pub block_spacing: f64,
}

/// How a heading looks.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadingStyle {
    pub text: TextStyle,
    pub padding: EdgeInsets,
}

/// How a block drawn in a box looks: a quote or a code block.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockStyle {
    /// Merged onto the text style of the blocks inside.
    pub text: TextStyle,
    /// Between the decoration's edge and the content.
    pub padding: EdgeInsets,
    pub decoration: BoxDecoration,
}

/// How a list looks.
#[derive(Clone, Debug, PartialEq)]
pub struct ListStyle {
    /// The width of the column that holds the bullets, past their padding.
    pub indent: f64,
    /// The style of a bullet or a number.
    pub bullet: TextStyle,
    pub bullet_padding: EdgeInsets,
    /// A task item's checkbox is drawn as a glyph of this style: its color, at its font size.
    pub checkbox: TextStyle,
}

/// How a table looks.
#[derive(Clone, Debug)]
pub struct TableStyle {
    pub head: TextStyle,
    pub body: TextStyle,
    /// The alignment of a head cell whose column states none; a body cell's is the start.
    pub head_align: TextAlign,
    pub border: TableBorder,
    pub cell_padding: EdgeInsets,
    /// Painted behind every other body row, from the second.
    pub stripe: Option<BoxDecoration>,
    /// A column width that sizes to its content lets the table scroll sideways.
    pub column_width: TableColumnWidthRef,
    pub vertical_alignment: TableCellVerticalAlignment,
}

impl PartialEq for TableStyle {
    fn eq(&self, other: &TableStyle) -> bool {
        self.head == other.head
            && self.body == other.body
            && self.head_align == other.head_align
            && self.border == other.border
            && self.cell_padding == other.cell_padding
            && self.stripe == other.stripe
            && Rc::ptr_eq(&self.column_width, &other.column_width)
            && self.vertical_alignment == other.vertical_alignment
    }
}

impl MarkdownStyleSheet {
    /// The look flutter_markdown's Cupertino style sheet gives, worked out from one text style:
    /// links and checkboxes in Apple's blue, headings larger and in weight 500, quotes, code
    /// blocks and every other table row on a light grey (dark grey when `brightness` is dark).
    ///
    /// A `text` with no font size counts as [`K_DEFAULT_FONT_SIZE`], the size it is drawn at,
    /// for the sizes worked out from it.
    pub fn from_text_style(text: TextStyle, brightness: Brightness) -> MarkdownStyleSheet {
        let by_brightness = |(light, dark): (Color, Color)| match brightness {
            Brightness::Dark => dark,
            Brightness::Light => light,
        };
        let font_size = text.font_size.unwrap_or(K_DEFAULT_FONT_SIZE);
        let heading = |size_increase: Option<f64>| {
            let mut style = text.copy_with().font_weight(FontWeight::W500);
            if let Some(increase) = size_increase {
                style = style.font_size(font_size + increase);
            }
            HeadingStyle {
                text: style,
                padding: EdgeInsets::ZERO,
            }
        };
        let code = text
            .copy_with()
            .font_family("monospace")
            .font_size(font_size * 0.85);
        let grey_6 = by_brightness(system_colors::GREY_6);
        let grey_4 = by_brightness(system_colors::GREY_4);
        let side = |width: f64| {
            BorderSide::new(
                grey_4,
                width,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            )
        };
        MarkdownStyleSheet {
            text: text.clone(),
            headings: [
                heading(Some(10.0)),
                heading(Some(8.0)),
                heading(Some(6.0)),
                heading(Some(4.0)),
                heading(Some(2.0)),
                heading(None),
            ],
            link: TextStyle::new().color(by_brightness(system_colors::LINK)),
            inline_code: code.clone(),
            code_block: BlockStyle {
                text: code,
                padding: EdgeInsets::all(8.0),
                decoration: BoxDecoration::new().color(grey_6),
            },
            quote: BlockStyle {
                text: text.clone(),
                padding: EdgeInsets::all(16.0),
                decoration: BoxDecoration::new().color(grey_6).border(Border {
                    left: side(4.0),
                    top: BorderSide::NONE,
                    right: BorderSide::NONE,
                    bottom: BorderSide::NONE,
                }),
            },
            list: ListStyle {
                indent: 24.0,
                bullet: text.clone(),
                bullet_padding: EdgeInsets::only(0.0, 0.0, 4.0, 0.0),
                checkbox: text.copy_with().color(by_brightness(system_colors::BLUE)),
            },
            table: TableStyle {
                head: text.copy_with().font_weight(FontWeight::W600),
                body: text.clone(),
                head_align: TextAlign::Center,
                border: TableBorder::all(
                    by_brightness(system_colors::SEPARATOR),
                    0.0,
                    BorderStyle::Solid,
                    BorderRadius::ZERO,
                ),
                cell_padding: EdgeInsets::from_ltrb(16.0, 8.0, 16.0, 8.0),
                stripe: Some(BoxDecoration::new().color(grey_6)),
                column_width: FlexColumnWidth::default_width(),
                vertical_alignment: TableCellVerticalAlignment::Middle,
            },
            rule: BoxDecoration::new().border(Border {
                top: side(1.0),
                right: BorderSide::NONE,
                bottom: BorderSide::NONE,
                left: BorderSide::NONE,
            }),
            block_spacing: 8.0,
        }
    }

    /// The style of emphasised text inside text of `style`.
    pub(crate) fn emphasis(style: &TextStyle) -> TextStyle {
        style.copy_with().font_style(FontStyle::Italic)
    }

    /// The style of strong text inside text of `style`.
    pub(crate) fn strong(style: &TextStyle) -> TextStyle {
        style.copy_with().font_weight(FontWeight::BOLD)
    }

    /// The style of struck-through text inside text of `style`.
    pub(crate) fn strikethrough(style: &TextStyle) -> TextStyle {
        style.copy_with().decoration(TextDecoration::LINE_THROUGH)
    }
}

/// The [`MarkdownStyleSheet`] of the Markdown widgets below it that are given none of their
/// own; a kit or an app sets it, as [`DefaultTextStyle`] sets text's style.
#[derive(Debug)]
pub struct DefaultMarkdownStyle {
    pub key: Option<KeyRef>,
    pub style_sheet: MarkdownStyleSheet,
    pub child: WidgetRef,
}

impl DefaultMarkdownStyle {
    /// Sets `style_sheet` for the Markdown below `child`.
    pub fn new<K>(
        style_sheet: MarkdownStyleSheet,
        child: impl IntoWidget<K>,
    ) -> DefaultMarkdownStyle {
        DefaultMarkdownStyle {
            key: None,
            style_sheet,
            child: child.into_widget(),
        }
    }

    /// Sets the key.
    pub fn key(mut self, key: KeyRef) -> DefaultMarkdownStyle {
        self.key = Some(key);
        self
    }

    /// The style sheet of the closest [`DefaultMarkdownStyle`] above `context`, if any; the
    /// context rebuilds when it changes.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<MarkdownStyleSheet> {
        context
            .depend_on_inherited_widget_of_exact_type::<DefaultMarkdownStyle>(app)
            .map(|widget| widget.style_sheet.clone())
    }

    /// The style sheet of the closest [`DefaultMarkdownStyle`] above `context`, or else
    /// [`MarkdownStyleSheet::from_text_style`] of the ambient [`DefaultTextStyle`] in the
    /// platform's brightness; the context rebuilds when any of them changes.
    pub fn of(app: &mut App, context: BuildContext) -> MarkdownStyleSheet {
        Self::maybe_of(app, context).unwrap_or_else(|| {
            let text = DefaultTextStyle::of(app, context).style;
            let brightness = MediaQuery::platform_brightness_of(app, context);
            MarkdownStyleSheet::from_text_style(text, brightness)
        })
    }
}

impl InheritedWidget for DefaultMarkdownStyle {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DefaultMarkdownStyle) -> bool {
        self.style_sheet != old_widget.style_sheet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_text_style_sizes_the_headings_from_the_text() {
        let text = TextStyle::new().font_size(15.0);
        let sheet = MarkdownStyleSheet::from_text_style(text.clone(), Brightness::Light);
        assert_eq!(sheet.text, text);
        let sizes: Vec<Option<f64>> = sheet
            .headings
            .iter()
            .map(|heading| heading.text.font_size)
            .collect();
        assert_eq!(
            sizes,
            [
                Some(25.0),
                Some(23.0),
                Some(21.0),
                Some(19.0),
                Some(17.0),
                Some(15.0)
            ]
        );
        assert_eq!(sheet.code_block.text.font_size, Some(15.0 * 0.85));
        let without_size = MarkdownStyleSheet::from_text_style(TextStyle::new(), Brightness::Light);
        assert_eq!(
            without_size.headings[0].text.font_size,
            Some(K_DEFAULT_FONT_SIZE + 10.0)
        );
    }

    #[test]
    fn from_text_style_takes_the_colors_of_the_brightness() {
        let light = MarkdownStyleSheet::from_text_style(TextStyle::new(), Brightness::Light);
        let dark = MarkdownStyleSheet::from_text_style(TextStyle::new(), Brightness::Dark);
        assert_eq!(light.link.color.as_deref(), Some(&system_colors::LINK.0));
        assert_eq!(dark.link.color.as_deref(), Some(&system_colors::LINK.1));
        assert_ne!(light.quote, dark.quote);
        assert_ne!(light.table, dark.table);
        assert_eq!(light, light.clone());
    }
}
