# inset-markdown

Markdown widgets for Inset. `MarkdownBody` lays a document out in a column, and `Markdown` lays it out in a scrolling list.

The crate takes its API ideas and its look from Flutter's `flutter_markdown` package: a style sheet worked out from the ambient text style, hooks for links, images, checkboxes, bullets and code highlighting, and the same rendering of each element. It is not a port of that package. It parses with pulldown-cmark into its own typed document (`Block` and `Inline`), which `parse` returns, and renders that document with the public functions in `render`.

The look comes from the nearest `DefaultMarkdownStyle`, or, when there is none, from `MarkdownStyleSheet::from_text_style` of the ambient `DefaultTextStyle` in the platform's brightness. A widget's own `style_sheet` replaces the inherited one whole.

Left out: text selection, which waits for a `SelectionArea`; custom syntax, since pulldown-cmark takes no plug-ins; links from bare addresses such as `www.example.com`; and footnotes, which pulldown-cmark can parse but the default options leave off. An HTML block is dropped, and inline HTML shows as its source.
