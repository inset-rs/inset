//! Flutter counterpart: `widgets/text.dart`, plus the public `RichText` from `basic.dart`.
//!
//! [`DefaultTextStyle`] and [`DefaultTextHeightBehavior`] are plain inherited widgets: the
//! `InheritedTheme` capture (`wrap`, `captureAll`) waits. [`Text`] builds only the
//! [`RichText`] branch: selection (`SelectionContainer`, `_SelectableTextContainer`,
//! `selectionColor`),
//! `Locale`, `StrutStyle`, and `Semantics` wait. [`RichText`] is a leaf here: `WidgetSpan`
//! children and the `SelectionRegistrar` wait.

use reveal_embedder::{FontWeight, TextAlign, TextDirection, TextHeightBehavior};
use reveal_foundation::App;
use reveal_painting::{
    InlineSpanRef, TextOverflow, TextScaler, TextSpan, TextStyle, TextWidthBasis,
};
use reveal_rendering::{AnyRenderObject, RenderBox, RenderHandle, RenderParagraph};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, LeafRenderObjectWidget, RenderObjectWidget,
    StatelessWidget, WidgetRef,
};
use crate::view::View;
use crate::widgets::basic::Directionality;
use crate::widgets::media_query::MediaQuery;

// ---------------------------------------------------------------------------------------------
// DefaultTextStyle

/// The text style to apply to descendant [`Text`] widgets which don't have an explicit style.
///
/// A `MediaQuery` ancestor of a [`Text`] widget may still override the `TextStyle.height`,
/// `TextStyle.letterSpacing`, and `TextStyle.wordSpacing` of the [`TextStyle`] set by this
/// [`DefaultTextStyle`] widget through its `MediaQueryData.lineHeightScaleFactorOverride`,
/// `MediaQueryData.letterSpacingOverride`, and `MediaQueryData.wordSpacingOverride` members.
///
/// See also:
///
///  * `AnimatedDefaultTextStyle`, which animates changes in the text style smoothly over a
///    given duration.
///  * `DefaultTextStyleTransition`, which takes a provided `Animation` to animate changes in
///    text style smoothly over time.
///
/// Dart's named constructor arguments are the fluent setters
/// (`DefaultTextStyle::new(style, child).max_lines(2)`).
#[derive(Clone, Debug)]
pub struct DefaultTextStyle {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// The text style to apply.
    pub style: TextStyle,
    /// How each line of text in the Text widget should be aligned horizontally.
    pub text_align: Option<TextAlign>,
    /// Whether the text should break at soft line breaks.
    ///
    /// If false, the glyphs in the text will be positioned as if there was unlimited
    /// horizontal space.
    ///
    /// This also decides the `overflow` property's behavior. If this is true or null, the
    /// glyph causing overflow, and those that follow, will not be rendered.
    pub soft_wrap: bool,
    /// How visual overflow should be handled.
    ///
    /// If `soft_wrap` is true or null, the glyph causing overflow, and those that follow,
    /// will not be rendered. Otherwise, it will be shown with the given overflow option.
    pub overflow: TextOverflow,
    /// An optional maximum number of lines for the text to span, wrapping if necessary. If
    /// the text exceeds the given number of lines, it will be truncated according to
    /// `overflow`.
    ///
    /// If this is 1, text will not wrap. Otherwise, text will be wrapped at the edge of the
    /// box.
    ///
    /// If this is non-null, it will override even explicit null values of `Text.max_lines`.
    pub max_lines: Option<i32>,
    /// The strategy to use when calculating the width of the Text.
    ///
    /// See [`TextWidthBasis`] for possible values and their implications.
    pub text_width_basis: TextWidthBasis,
    /// Defines how to apply `TextStyle.height` over and under text.
    pub text_height_behavior: Option<TextHeightBehavior>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl DefaultTextStyle {
    /// Creates a default text style for the given subtree.
    ///
    /// Consider using `DefaultTextStyle.merge` to inherit styling information from the
    /// current default text style for a given [`BuildContext`].
    ///
    /// The `max_lines` property may be `None` (and indeed defaults to `None`), but if it is
    /// not `None`, it must be greater than zero.
    pub fn new<K>(style: TextStyle, child: impl IntoWidget<K>) -> DefaultTextStyle {
        DefaultTextStyle {
            key: None,
            style,
            text_align: None,
            soft_wrap: true,
            overflow: TextOverflow::Clip,
            max_lines: None,
            text_width_basis: TextWidthBasis::Parent,
            text_height_behavior: None,
            child: child.into_widget(),
        }
    }

    /// A default text style that provides fallback values.
    ///
    /// Returned from [`of`](Self::of) when the given [`BuildContext`] doesn't have an
    /// enclosing default text style.
    ///
    /// This constructor creates a [`DefaultTextStyle`] with an invalid `child`, which means
    /// the constructed value cannot be incorporated into the tree.
    pub fn fallback() -> DefaultTextStyle {
        DefaultTextStyle::new(TextStyle::new(), NullWidget)
    }

    /// Dart `DefaultTextStyle(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultTextStyle {
        self.key = Some(key);
        self
    }

    /// Dart `DefaultTextStyle(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> DefaultTextStyle {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `DefaultTextStyle(softWrap:)`.
    pub fn soft_wrap(mut self, soft_wrap: bool) -> DefaultTextStyle {
        self.soft_wrap = soft_wrap;
        self
    }

    /// Dart `DefaultTextStyle(overflow:)`.
    pub fn overflow(mut self, overflow: TextOverflow) -> DefaultTextStyle {
        self.overflow = overflow;
        self
    }

    /// Dart `DefaultTextStyle(maxLines:)`. Must be greater than zero.
    pub fn max_lines(mut self, max_lines: i32) -> DefaultTextStyle {
        debug_assert!(max_lines > 0);
        self.max_lines = Some(max_lines);
        self
    }

    /// Dart `DefaultTextStyle(textWidthBasis:)`.
    pub fn text_width_basis(mut self, text_width_basis: TextWidthBasis) -> DefaultTextStyle {
        self.text_width_basis = text_width_basis;
        self
    }

    /// Dart `DefaultTextStyle(textHeightBehavior:)`.
    pub fn text_height_behavior(
        mut self,
        text_height_behavior: TextHeightBehavior,
    ) -> DefaultTextStyle {
        self.text_height_behavior = Some(text_height_behavior);
        self
    }

    /// The closest instance of this class that encloses the given context.
    ///
    /// If no such instance exists, returns an instance created by
    /// [`fallback`](Self::fallback), which contains fallback values.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let style = DefaultTextStyle::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> DefaultTextStyle {
        context
            .depend_on_inherited_widget_of_exact_type::<DefaultTextStyle>(app)
            .cloned()
            .unwrap_or_else(DefaultTextStyle::fallback)
    }
}

impl InheritedWidget for DefaultTextStyle {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DefaultTextStyle) -> bool {
        self.style != old_widget.style
            || self.text_align != old_widget.text_align
            || self.soft_wrap != old_widget.soft_wrap
            || self.overflow != old_widget.overflow
            || self.max_lines != old_widget.max_lines
            || self.text_width_basis != old_widget.text_width_basis
            || self.text_height_behavior != old_widget.text_height_behavior
    }
}

/// Dart's `_NullWidget`: the child of [`DefaultTextStyle::fallback`], which must never build.
#[derive(Debug)]
struct NullWidget;

impl StatelessWidget for NullWidget {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        panic!(
            "A DefaultTextStyle constructed with DefaultTextStyle::fallback cannot be \
             incorporated into the widget tree, it is meant only to provide a fallback value \
             returned by DefaultTextStyle::of() when no enclosing default text style is \
             present in a BuildContext."
        );
    }
}

// ---------------------------------------------------------------------------------------------
// DefaultTextHeightBehavior

/// The [`TextHeightBehavior`] that will apply to descendant [`Text`] and `EditableText`
/// widgets which have not explicitly set `Text.text_height_behavior`.
///
/// If there is a [`DefaultTextStyle`] with a non-null `DefaultTextStyle.text_height_behavior`
/// below this widget, the `DefaultTextStyle.text_height_behavior` will be used over this
/// widget's [`TextHeightBehavior`].
///
/// See also:
///
///  * [`DefaultTextStyle`], which defines a [`TextStyle`] to apply to descendant [`Text`]
///    widgets.
#[derive(Debug)]
pub struct DefaultTextHeightBehavior {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// Defines how to apply `TextStyle.height` over and under text.
    pub text_height_behavior: TextHeightBehavior,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl DefaultTextHeightBehavior {
    /// Creates a default text height behavior for the given subtree.
    pub fn new<K>(
        text_height_behavior: TextHeightBehavior,
        child: impl IntoWidget<K>,
    ) -> DefaultTextHeightBehavior {
        DefaultTextHeightBehavior {
            key: None,
            text_height_behavior,
            child: child.into_widget(),
        }
    }

    /// Dart `DefaultTextHeightBehavior(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultTextHeightBehavior {
        self.key = Some(key);
        self
    }

    /// The closest instance of [`DefaultTextHeightBehavior`] that encloses the given
    /// context, or `None` if none is found.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`DefaultTextHeightBehavior`] in the `context`, if there is one.
    ///
    /// See also:
    ///
    /// * [`of`](Self::of), which is similar to this method, but asserts if no
    ///   [`DefaultTextHeightBehavior`] ancestor is found.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<TextHeightBehavior> {
        context
            .depend_on_inherited_widget_of_exact_type::<DefaultTextHeightBehavior>(app)
            .map(|widget| widget.text_height_behavior)
    }

    /// The closest instance of [`DefaultTextHeightBehavior`] that encloses the given
    /// context.
    ///
    /// If no such instance exists, this method panics.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`DefaultTextHeightBehavior`] in the `context`.
    ///
    /// See also:
    ///
    /// * [`maybe_of`](Self::maybe_of), which is similar to this method, but returns `None`
    ///   if no [`DefaultTextHeightBehavior`] ancestor is found.
    pub fn of(app: &mut App, context: BuildContext) -> TextHeightBehavior {
        let behavior = Self::maybe_of(app, context);
        behavior.expect(
            "DefaultTextHeightBehavior::of() was called with a context that does not contain \
             a DefaultTextHeightBehavior widget.\n\
             No DefaultTextHeightBehavior widget ancestor could be found starting from the \
             context that was passed to DefaultTextHeightBehavior::of(). This can happen \
             because you are using a widget that looks for a DefaultTextHeightBehavior \
             ancestor, but no such ancestor exists.",
        )
    }
}

impl InheritedWidget for DefaultTextHeightBehavior {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DefaultTextHeightBehavior) -> bool {
        self.text_height_behavior != old_widget.text_height_behavior
    }
}

// ---------------------------------------------------------------------------------------------
// Text

/// A run of text with a single style.
///
/// The [`Text`] widget displays a string of text with single style. The string might break
/// across multiple lines or might all be displayed on the same line depending on the layout
/// constraints.
///
/// The `style` argument is optional. When omitted, the text will use the style from the
/// closest enclosing [`DefaultTextStyle`]. If the given style's `TextStyle.inherit` property
/// is true (the default), the given style will be merged with the closest enclosing
/// [`DefaultTextStyle`]. This merging behavior is useful, for example, to make the text bold
/// while using the default font family and size.
///
/// This example shows how to display text using the [`Text`] widget with the `overflow` set
/// to [`TextOverflow::Ellipsis`].
///
/// ```text
/// Text::new("Hello, how are you?").overflow(TextOverflow::Ellipsis)
/// ```
///
/// Setting `max_lines` to `1` is not equivalent to disabling soft wrapping with `soft_wrap`.
/// This is apparent when using [`TextOverflow::Fade`]: with `max_lines(1)` a second line
/// overflows and the fade appears in a horizontal direction at the bottom; with
/// `soft_wrap(false)` the single line overflows and the fade appears in a vertical direction
/// at the right.
///
/// Using the [`rich`](Self::rich) constructor, the [`Text`] widget can display a paragraph
/// with differently styled [`TextSpan`]s.
///
/// ## Interactivity
///
/// To make [`Text`] react to touch events, wrap it in a `GestureDetector` widget with a
/// `GestureDetector.on_tap` handler.
///
/// To make sections of the text interactive, use [`RichText`] and specify a
/// `TapGestureRecognizer` as the `TextSpan.recognizer` of the relevant part of the text.
///
/// ## Selection
///
/// [`Text`] is not selectable by default. To make a [`Text`] selectable, one can wrap a
/// subtree with a `SelectionArea` widget.
///
/// See also:
///
///  * [`RichText`], which gives you more control over the text styles.
///  * [`DefaultTextStyle`], which sets default styles for [`Text`] widgets.
///
/// Dart's named constructor arguments are the fluent setters
/// (`Text::new("Hi").max_lines(1)`).
#[derive(Debug)]
pub struct Text {
    /// See [`StatelessWidget::key`].
    pub key: Option<KeyRef>,
    /// The text to display.
    ///
    /// This will be `None` if a `text_span` is provided instead.
    pub data: Option<String>,
    /// The text to display as an [`InlineSpan`](reveal_painting::InlineSpan).
    ///
    /// This will be `None` if `data` is provided instead.
    pub text_span: Option<InlineSpanRef>,
    /// If non-null, the style to use for this text.
    ///
    /// If the style's "inherit" property is true, the style will be merged with the closest
    /// enclosing [`DefaultTextStyle`]. Otherwise, the style will replace the closest
    /// enclosing [`DefaultTextStyle`].
    pub style: Option<TextStyle>,
    /// How the text should be aligned horizontally.
    pub text_align: Option<TextAlign>,
    /// The directionality of the text.
    ///
    /// This decides how `text_align` values like [`TextAlign::Start`] and [`TextAlign::End`]
    /// are interpreted.
    ///
    /// This is also used to disambiguate how to render bidirectional text. For example, if
    /// the `data` is an English phrase followed by a Hebrew phrase, in a
    /// [`TextDirection::Ltr`] context the English phrase will be on the left and the Hebrew
    /// phrase to its right, while in a [`TextDirection::Rtl`] context, the English phrase
    /// will be on the right and the Hebrew phrase on its left.
    ///
    /// Defaults to the ambient `Directionality`, if any.
    pub text_direction: Option<TextDirection>,
    /// Whether the text should break at soft line breaks.
    ///
    /// If false, the glyphs in the text will be positioned as if there was unlimited
    /// horizontal space.
    pub soft_wrap: Option<bool>,
    /// How visual overflow should be handled.
    ///
    /// If this is `None`, `TextStyle.overflow` will be used, otherwise the value from the
    /// nearest [`DefaultTextStyle`] ancestor will be used.
    pub overflow: Option<TextOverflow>,
    /// The font scaling strategy to use when laying out and rendering the text.
    pub text_scaler: Option<TextScaler>,
    /// An optional maximum number of lines for the text to span, wrapping if necessary. If
    /// the text exceeds the given number of lines, it will be truncated according to
    /// `overflow`.
    ///
    /// If this is 1, text will not wrap. Otherwise, text will be wrapped at the edge of the
    /// box.
    ///
    /// If this is `None`, but there is an ambient [`DefaultTextStyle`] that specifies an
    /// explicit number for its `DefaultTextStyle.max_lines`, then the [`DefaultTextStyle`]
    /// value will take precedence. You can use a [`RichText`] widget directly to entirely
    /// override the [`DefaultTextStyle`].
    pub max_lines: Option<i32>,
    /// Defines how to measure the width of the rendered text.
    pub text_width_basis: Option<TextWidthBasis>,
    /// Defines how to apply `TextStyle.height` over and under text.
    pub text_height_behavior: Option<TextHeightBehavior>,
}

impl Text {
    /// Creates a text widget.
    ///
    /// If the `style` argument is `None`, the text will use the style from the closest
    /// enclosing [`DefaultTextStyle`].
    ///
    /// The `overflow` property's behavior is affected by the `soft_wrap` argument. If the
    /// `soft_wrap` is true or `None`, the glyph causing overflow, and those that follow, will
    /// not be rendered. Otherwise, it will be shown with the given overflow option.
    pub fn new(data: impl Into<String>) -> Text {
        Text::unstyled(Some(data.into()), None)
    }

    /// Creates a text widget with an [`InlineSpan`](reveal_painting::InlineSpan).
    ///
    /// The following subclasses of `InlineSpan` may be used to build rich text:
    ///
    /// * [`TextSpan`]s define text and children `InlineSpan`s.
    /// * `WidgetSpan`s define embedded inline widgets.
    ///
    /// See [`RichText`] which provides a lower-level way to draw text.
    pub fn rich(text_span: InlineSpanRef) -> Text {
        Text::unstyled(None, Some(text_span))
    }

    /// The fields both constructors leave unset.
    fn unstyled(data: Option<String>, text_span: Option<InlineSpanRef>) -> Text {
        Text {
            key: None,
            data,
            text_span,
            style: None,
            text_align: None,
            text_direction: None,
            soft_wrap: None,
            overflow: None,
            text_scaler: None,
            max_lines: None,
            text_width_basis: None,
            text_height_behavior: None,
        }
    }

    /// Dart `Text(key:)`.
    pub fn key(mut self, key: KeyRef) -> Text {
        self.key = Some(key);
        self
    }

    /// Dart `Text(style:)`.
    pub fn style(mut self, style: TextStyle) -> Text {
        self.style = Some(style);
        self
    }

    /// Dart `Text(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> Text {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `Text(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Text {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Text(softWrap:)`.
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Text {
        self.soft_wrap = Some(soft_wrap);
        self
    }

    /// Dart `Text(overflow:)`.
    pub fn overflow(mut self, overflow: TextOverflow) -> Text {
        self.overflow = Some(overflow);
        self
    }

    /// Dart `Text(textScaler:)`.
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> Text {
        self.text_scaler = Some(text_scaler);
        self
    }

    /// Dart `Text(maxLines:)`.
    pub fn max_lines(mut self, max_lines: i32) -> Text {
        self.max_lines = Some(max_lines);
        self
    }

    /// Dart `Text(textWidthBasis:)`.
    pub fn text_width_basis(mut self, text_width_basis: TextWidthBasis) -> Text {
        self.text_width_basis = Some(text_width_basis);
        self
    }

    /// Dart `Text(textHeightBehavior:)`.
    pub fn text_height_behavior(mut self, text_height_behavior: TextHeightBehavior) -> Text {
        self.text_height_behavior = Some(text_height_behavior);
        self
    }
}

impl StatelessWidget for Text {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let default_text_style = DefaultTextStyle::of(app, context);
        let mut effective_text_style = self.style.clone();
        if self.style.as_ref().is_none_or(|style| style.inherit) {
            effective_text_style = Some(default_text_style.style.merge(self.style.as_ref()));
        }
        if MediaQuery::bold_text_of(app, context) {
            let bold = TextStyle::new().font_weight(FontWeight::BOLD);
            effective_text_style = Some(
                effective_text_style
                    .expect("the style is set above")
                    .merge(Some(&bold)),
            );
        }
        // Flutter leaves `MediaQueryData.paragraphSpacingOverride` to its own text components.
        let line_height_scale_factor =
            MediaQuery::maybe_line_height_scale_factor_override_of(app, context);
        let letter_spacing = MediaQuery::maybe_letter_spacing_override_of(app, context);
        let word_spacing = MediaQuery::maybe_word_spacing_override_of(app, context);
        let effective_text_span = apply_text_spacing_overrides(
            line_height_scale_factor,
            letter_spacing,
            word_spacing,
            TextSpan {
                style: effective_text_style.clone(),
                text: self.data.clone(),
                children: self
                    .text_span
                    .as_ref()
                    .map(|text_span| vec![text_span.clone()]),
            },
        );
        let text_scaler = self
            .text_scaler
            .clone()
            .unwrap_or_else(|| MediaQuery::text_scaler_of(app, context));
        let text_height_behavior = self
            .text_height_behavior
            .or(default_text_style.text_height_behavior)
            .or_else(|| DefaultTextHeightBehavior::maybe_of(app, context));
        let mut rich_text = RichText::new(effective_text_span.into_span())
            .text_align(
                self.text_align
                    .or(default_text_style.text_align)
                    .unwrap_or(TextAlign::Start),
            )
            .soft_wrap(self.soft_wrap.unwrap_or(default_text_style.soft_wrap))
            .overflow(
                self.overflow
                    .or(effective_text_style
                        .as_ref()
                        .and_then(|style| style.overflow))
                    .unwrap_or(default_text_style.overflow),
            )
            .text_scaler(text_scaler)
            .text_width_basis(
                self.text_width_basis
                    .unwrap_or(default_text_style.text_width_basis),
            );
        // RichText uses Directionality.of to obtain a default if this is None.
        if let Some(text_direction) = self.text_direction {
            rich_text = rich_text.text_direction(text_direction);
        }
        if let Some(max_lines) = self.max_lines.or(default_text_style.max_lines) {
            rich_text = rich_text.max_lines(max_lines);
        }
        if let Some(text_height_behavior) = text_height_behavior {
            rich_text = rich_text.text_height_behavior(text_height_behavior);
        }
        rich_text.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// RichText (basic.dart)

/// Dart's `_OverridingTextStyleTextSpanUtils.applyTextSpacingOverrides`: overrides the text
/// styles of a [`TextSpan`] tree with the `MediaQuery` spacing overrides.
fn apply_text_spacing_overrides(
    line_height_scale_factor: Option<f64>,
    letter_spacing: Option<f64>,
    word_spacing: Option<f64>,
    text_span: TextSpan,
) -> TextSpan {
    if line_height_scale_factor.is_none() && letter_spacing.is_none() && word_spacing.is_none() {
        return text_span;
    }
    let mut override_text_style = TextStyle::new();
    if let Some(height) = line_height_scale_factor {
        override_text_style = override_text_style.height(height);
    }
    if let Some(letter_spacing) = letter_spacing {
        override_text_style = override_text_style.letter_spacing(letter_spacing);
    }
    if let Some(word_spacing) = word_spacing {
        override_text_style = override_text_style.word_spacing(word_spacing);
    }
    apply_text_style_overrides(&override_text_style, &text_span)
}

/// Dart's `_OverridingTextStyleTextSpanUtils._applyTextStyleOverrides`.
fn apply_text_style_overrides(override_text_style: &TextStyle, text_span: &TextSpan) -> TextSpan {
    TextSpan {
        text: text_span.text.clone(),
        children: text_span.children.as_ref().map(|children| {
            children
                .iter()
                .map(|child| match child.as_any().downcast_ref::<TextSpan>() {
                    Some(child) => {
                        apply_text_style_overrides(override_text_style, child).into_span()
                    }
                    None => child.clone(),
                })
                .collect()
        }),
        style: Some(match &text_span.style {
            Some(style) => style.merge(Some(override_text_style)),
            None => override_text_style.clone(),
        }),
    }
}

/// A paragraph of rich text.
///
/// The [`RichText`] widget displays text that uses multiple different styles. The text to
/// display is described using a tree of [`TextSpan`] objects, each of which has an
/// associated style that is used for that subtree. The text might break across multiple
/// lines or might all be displayed on the same line depending on the layout constraints.
///
/// Text displayed in a [`RichText`] widget must be explicitly styled. When picking which
/// style to use, consider using [`DefaultTextStyle::of`] the current [`BuildContext`] to
/// provide defaults. For more details on how to style text in a [`RichText`] widget, see the
/// documentation for [`TextStyle`].
///
/// Consider using the [`Text`] widget to integrate with the [`DefaultTextStyle`]
/// automatically. When all the text uses the same style, the default constructor is less
/// verbose. The [`Text::rich`] constructor allows you to style multiple spans with the
/// default text style while still allowing specified styles per span.
///
/// ```text
/// RichText::new(
///     TextSpan::new()
///         .text("Hello ")
///         .style(DefaultTextStyle::of(app, context).style)
///         .children(vec![
///             TextSpan::new().text("bold").style(TextStyle::new().font_weight(FontWeight::BOLD)).into_span(),
///             TextSpan::new().text(" world!").into_span(),
///         ])
///         .into_span(),
/// )
/// ```
///
/// See also:
///
///  * [`TextStyle`], which discusses how to style text.
///  * [`TextSpan`], which is used to describe the text in a paragraph.
///  * [`Text`], which automatically applies the ambient styles described by a
///    [`DefaultTextStyle`] to a single string.
///  * [`Text::rich`], a text widget that provides similar functionality as [`RichText`].
///    [`Text::rich`] will inherit [`TextStyle`] from [`DefaultTextStyle`].
///
/// Dart's named constructor arguments are the fluent setters
/// (`RichText::new(span).text_align(TextAlign::Center)`).
#[derive(Debug)]
pub struct RichText {
    /// See [`RenderObjectWidget::key`].
    pub key: Option<KeyRef>,
    /// The text to display in this widget.
    pub text: InlineSpanRef,
    /// How the text should be aligned horizontally.
    pub text_align: TextAlign,
    /// The directionality of the text.
    ///
    /// This decides how `text_align` values like [`TextAlign::Start`] and [`TextAlign::End`]
    /// are interpreted.
    ///
    /// This is also used to disambiguate how to render bidirectional text. For example, if
    /// the `text` is an English phrase followed by a Hebrew phrase, in a
    /// [`TextDirection::Ltr`] context the English phrase will be on the left and the Hebrew
    /// phrase to its right, while in a [`TextDirection::Rtl`] context, the English phrase
    /// will be on the right and the Hebrew phrase on its left.
    ///
    /// Defaults to the ambient `Directionality`, if any. If there is no ambient
    /// `Directionality`, then this must not be `None`.
    pub text_direction: Option<TextDirection>,
    /// Whether the text should break at soft line breaks.
    ///
    /// If false, the glyphs in the text will be positioned as if there was unlimited
    /// horizontal space.
    pub soft_wrap: bool,
    /// How visual overflow should be handled.
    pub overflow: TextOverflow,
    /// The font scaling strategy to use when laying out and rendering the text.
    pub text_scaler: TextScaler,
    /// An optional maximum number of lines for the text to span, wrapping if necessary. If
    /// the text exceeds the given number of lines, it will be truncated according to
    /// `overflow`.
    ///
    /// If this is 1, text will not wrap. Otherwise, text will be wrapped at the edge of the
    /// box.
    pub max_lines: Option<i32>,
    /// Defines how to measure the width of the rendered text.
    pub text_width_basis: TextWidthBasis,
    /// Defines how to apply `TextStyle.height` over and under text.
    pub text_height_behavior: Option<TextHeightBehavior>,
}

impl RichText {
    /// Creates a paragraph of rich text.
    ///
    /// The `max_lines` property may be `None` (and indeed defaults to `None`), but if it is
    /// not `None`, it must be greater than zero.
    ///
    /// The `text_direction`, if `None`, defaults to the ambient `Directionality`, which in
    /// that case must not be null.
    pub fn new(text: InlineSpanRef) -> RichText {
        RichText {
            key: None,
            text,
            text_align: TextAlign::Start,
            text_direction: None,
            soft_wrap: true,
            overflow: TextOverflow::Clip,
            text_scaler: TextScaler::NO_SCALING,
            max_lines: None,
            text_width_basis: TextWidthBasis::Parent,
            text_height_behavior: None,
        }
    }

    /// Dart `RichText(key:)`.
    pub fn key(mut self, key: KeyRef) -> RichText {
        self.key = Some(key);
        self
    }

    /// Dart `RichText(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> RichText {
        self.text_align = text_align;
        self
    }

    /// Dart `RichText(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> RichText {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `RichText(softWrap:)`.
    pub fn soft_wrap(mut self, soft_wrap: bool) -> RichText {
        self.soft_wrap = soft_wrap;
        self
    }

    /// Dart `RichText(overflow:)`.
    pub fn overflow(mut self, overflow: TextOverflow) -> RichText {
        self.overflow = overflow;
        self
    }

    /// Dart `RichText(textScaler:)`.
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> RichText {
        self.text_scaler = text_scaler;
        self
    }

    /// Dart `RichText(maxLines:)`. Must be greater than zero.
    pub fn max_lines(mut self, max_lines: i32) -> RichText {
        debug_assert!(max_lines > 0);
        self.max_lines = Some(max_lines);
        self
    }

    /// Dart `RichText(textWidthBasis:)`.
    pub fn text_width_basis(mut self, text_width_basis: TextWidthBasis) -> RichText {
        self.text_width_basis = text_width_basis;
        self
    }

    /// Dart `RichText(textHeightBehavior:)`.
    pub fn text_height_behavior(mut self, text_height_behavior: TextHeightBehavior) -> RichText {
        self.text_height_behavior = Some(text_height_behavior);
        self
    }

    /// Dart's `_getDevicePixelRatio`.
    fn device_pixel_ratio(app: &mut App, context: BuildContext) -> f64 {
        MediaQuery::maybe_device_pixel_ratio_of(app, context)
            .or_else(|| View::maybe_of(app, context).map(|view| view.metrics().device_pixel_ratio))
            .unwrap_or(1.0)
    }
}

impl RenderObjectWidget for RichText {
    type RenderObject = RenderParagraph;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = self
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context));
        let device_pixel_ratio = Self::device_pixel_ratio(app, context);
        let render_object = RenderParagraph::new(app, self.text.clone(), text_direction, None);
        render_object.set_text_align(app, self.text_align);
        render_object.set_soft_wrap(app, self.soft_wrap);
        render_object.set_overflow(app, self.overflow);
        render_object.set_text_scaler(app, self.text_scaler.clone());
        render_object.set_max_lines(app, self.max_lines);
        render_object.set_text_width_basis(app, self.text_width_basis);
        render_object.set_text_height_behavior(app, self.text_height_behavior);
        render_object.set_device_pixel_ratio(app, device_pixel_ratio);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderParagraph>,
    ) {
        let text_direction = self
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context));
        render_object.set_text(app, self.text.clone());
        render_object.set_text_align(app, self.text_align);
        render_object.set_text_direction(app, text_direction);
        render_object.set_soft_wrap(app, self.soft_wrap);
        render_object.set_overflow(app, self.overflow);
        render_object.set_text_scaler(app, self.text_scaler.clone());
        render_object.set_max_lines(app, self.max_lines);
        render_object.set_text_width_basis(app, self.text_width_basis);
        render_object.set_text_height_behavior(app, self.text_height_behavior);
        let device_pixel_ratio = Self::device_pixel_ratio(app, context);
        render_object.set_device_pixel_ratio(app, device_pixel_ratio);
    }
}

impl LeafRenderObjectWidget for RichText {}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use reveal_embedder::{Color, TextLeadingDistribution};
    use reveal_foundation::ValueKey;
    use reveal_painting::PaintingBinding;
    use reveal_rendering::RenderObjectWithChildMixin;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::media_query::MediaQueryData;

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    /// Dart tests wrap text in a `Directionality`, which `RichText` requires.
    fn ltr(child: WidgetRef) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    /// Mounts `child` with fonts installed and pumps the first frame.
    fn mount(app: &mut App, child: WidgetRef) -> Harness {
        install_fonts(app);
        let harness = Harness::mount(app, ltr(child));
        harness.pump(app);
        harness
    }

    /// The `RenderParagraph` directly under the test root.
    fn paragraph_under_root(harness: &Harness, app: &App) -> RenderHandle<RenderParagraph> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderParagraph>(app)
            .expect("a RenderParagraph")
    }

    fn style_of(paragraph: RenderHandle<RenderParagraph>, app: &App) -> TextStyle {
        paragraph
            .text(app)
            .style()
            .cloned()
            .expect("Text always styles its span")
    }

    fn plain_text_of(paragraph: RenderHandle<RenderParagraph>, app: &App) -> String {
        paragraph.text(app).to_plain_text(true, true)
    }

    #[test]
    fn a_text_lays_out_a_paragraph_with_the_fallback_style() {
        let mut app = App::new();
        let harness = mount(&mut app, Text::new("Hello").into_widget());
        let paragraph = paragraph_under_root(&harness, &app);
        let size = paragraph.size(&app);
        assert!(size.width() > 0.0 && size.height() > 0.0);
        assert_eq!(plain_text_of(paragraph, &app), "Hello");
        assert_eq!(
            style_of(paragraph, &app),
            TextStyle::new(),
            "DefaultTextStyle::fallback"
        );
        assert_eq!(paragraph.text_direction(&app), TextDirection::Ltr);
        assert_eq!(paragraph.text_align(&app), TextAlign::Start);
        assert_eq!(paragraph.max_lines(&app), None);
        assert_eq!(paragraph.overflow(&app), TextOverflow::Clip);
    }

    #[test]
    fn a_rich_text_span_becomes_the_child_of_the_styled_span() {
        let mut app = App::new();
        let span = TextSpan::new().text("Hello").into_span();
        let harness = mount(&mut app, Text::rich(span).into_widget());
        let paragraph = paragraph_under_root(&harness, &app);
        assert_eq!(plain_text_of(paragraph, &app), "Hello");
        assert!(paragraph.size(&app).width() > 0.0);
    }

    #[test]
    fn a_default_text_style_styles_the_text_and_notifies_on_change() {
        let mut app = App::new();
        // One Text instance, reused like a `const` widget, so only the default style decides
        // rebuilds.
        let text: WidgetRef = Text::new("Hello").into_widget();
        let styled = |font_size: f64| {
            DefaultTextStyle::new(TextStyle::new().font_size(font_size), text.clone()).into_widget()
        };
        let harness = mount(&mut app, styled(12.0));
        let paragraph = paragraph_under_root(&harness, &app);
        assert_eq!(style_of(paragraph, &app).font_size, Some(12.0));
        let small = paragraph.size(&app);

        harness.set_child(&mut app, ltr(styled(24.0)));
        harness.pump(&mut app);
        assert_eq!(
            paragraph_under_root(&harness, &app),
            paragraph,
            "the same paragraph is restyled"
        );
        assert_eq!(style_of(paragraph, &app).font_size, Some(24.0));
        assert!(paragraph.size(&app).height() > small.height());
    }

    #[test]
    fn an_explicit_style_merges_into_the_default_unless_it_does_not_inherit() {
        let mut app = App::new();
        let red = Color::from_argb(255, 255, 0, 0);
        let under_default = |style: TextStyle| {
            DefaultTextStyle::new(
                TextStyle::new().font_size(12.0),
                Text::new("Hello").style(style),
            )
            .into_widget()
        };
        let harness = mount(&mut app, under_default(TextStyle::new().color(red)));
        let paragraph = paragraph_under_root(&harness, &app);
        let merged = style_of(paragraph, &app);
        assert_eq!(merged.font_size, Some(12.0));
        assert_eq!(merged.color, Some(red.into()));

        harness.set_child(
            &mut app,
            ltr(under_default(
                TextStyle::new().inherit(false).font_size(30.0),
            )),
        );
        harness.pump(&mut app);
        let replaced = style_of(paragraph, &app);
        assert_eq!(replaced.font_size, Some(30.0));
        assert_eq!(
            replaced.color, None,
            "a non-inheriting style replaces the default"
        );
    }

    #[test]
    fn a_new_string_updates_the_same_paragraph() {
        let mut app = App::new();
        let harness = mount(&mut app, Text::new("Hello").into_widget());
        let paragraph = paragraph_under_root(&harness, &app);
        let short = paragraph.size(&app);

        harness.set_child(&mut app, ltr(Text::new("Hello there, world").into_widget()));
        harness.pump(&mut app);
        assert_eq!(paragraph_under_root(&harness, &app), paragraph);
        assert_eq!(plain_text_of(paragraph, &app), "Hello there, world");
        assert!(paragraph.size(&app).width() > short.width());
    }

    #[test]
    fn max_lines_and_overflow_reach_the_render_object() {
        let mut app = App::new();
        let harness = mount(
            &mut app,
            Text::new("Hello")
                .max_lines(2)
                .overflow(TextOverflow::Ellipsis)
                .soft_wrap(false)
                .text_direction(TextDirection::Rtl)
                .text_align(TextAlign::End)
                .into_widget(),
        );
        let paragraph = paragraph_under_root(&harness, &app);
        assert_eq!(paragraph.max_lines(&app), Some(2));
        assert_eq!(paragraph.overflow(&app), TextOverflow::Ellipsis);
        assert!(!paragraph.soft_wrap(&app));
        assert_eq!(paragraph.text_direction(&app), TextDirection::Rtl);
        assert_eq!(paragraph.text_align(&app), TextAlign::End);
    }

    #[test]
    fn a_default_text_style_supplies_what_the_text_leaves_unset() {
        let mut app = App::new();
        let under_default = |text: Text| {
            DefaultTextStyle::new(TextStyle::new(), text)
                .max_lines(1)
                .overflow(TextOverflow::Fade)
                .text_align(TextAlign::Center)
                .soft_wrap(false)
                .into_widget()
        };
        let harness = mount(&mut app, under_default(Text::new("Hello")));
        let paragraph = paragraph_under_root(&harness, &app);
        assert_eq!(paragraph.max_lines(&app), Some(1));
        assert_eq!(paragraph.overflow(&app), TextOverflow::Fade);
        assert_eq!(paragraph.text_align(&app), TextAlign::Center);
        assert!(!paragraph.soft_wrap(&app));

        harness.set_child(
            &mut app,
            ltr(under_default(Text::new("Hello").max_lines(3))),
        );
        harness.pump(&mut app);
        assert_eq!(
            paragraph.max_lines(&app),
            Some(3),
            "the Text's own value wins"
        );
    }

    #[test]
    fn the_style_overflow_beats_the_default_text_style_overflow() {
        let mut app = App::new();
        let harness = mount(
            &mut app,
            DefaultTextStyle::new(
                TextStyle::new(),
                Text::new("Hello").style(TextStyle::new().overflow(TextOverflow::Ellipsis)),
            )
            .overflow(TextOverflow::Fade)
            .into_widget(),
        );
        let paragraph = paragraph_under_root(&harness, &app);
        assert_eq!(paragraph.overflow(&app), TextOverflow::Ellipsis);
    }

    #[test]
    fn a_default_text_height_behavior_reaches_the_paragraph_and_notifies_on_change() {
        let mut app = App::new();
        let text: WidgetRef = Text::new("Hello").into_widget();
        let under_behavior = |leading_distribution: TextLeadingDistribution| {
            DefaultTextHeightBehavior::new(
                TextHeightBehavior {
                    leading_distribution,
                    ..TextHeightBehavior::default()
                },
                text.clone(),
            )
            .into_widget()
        };
        let harness = mount(&mut app, under_behavior(TextLeadingDistribution::Even));
        let paragraph = paragraph_under_root(&harness, &app);
        let behavior = paragraph.text_height_behavior(&app).expect("inherited");
        assert_eq!(behavior.leading_distribution, TextLeadingDistribution::Even);

        harness.set_child(
            &mut app,
            ltr(under_behavior(TextLeadingDistribution::Proportional)),
        );
        harness.pump(&mut app);
        let behavior = paragraph.text_height_behavior(&app).expect("inherited");
        assert_eq!(
            behavior.leading_distribution,
            TextLeadingDistribution::Proportional
        );
    }

    #[test]
    fn a_rich_text_forwards_its_configuration_in_place() {
        let mut app = App::new();
        let rich = |text: &str| {
            RichText::new(TextSpan::new().text(text).into_span())
                .text_align(TextAlign::Center)
                .text_direction(TextDirection::Rtl)
                .max_lines(1)
                .into_widget()
        };
        let harness = mount(&mut app, rich("Hello"));
        let paragraph = paragraph_under_root(&harness, &app);
        assert_eq!(paragraph.text_align(&app), TextAlign::Center);
        assert_eq!(paragraph.text_direction(&app), TextDirection::Rtl);
        assert_eq!(paragraph.max_lines(&app), Some(1));

        harness.set_child(&mut app, ltr(rich("Goodbye")));
        harness.pump(&mut app);
        assert_eq!(paragraph_under_root(&harness, &app), paragraph);
        assert_eq!(plain_text_of(paragraph, &app), "Goodbye");
    }

    #[test]
    fn a_changed_key_replaces_the_paragraph_where_the_same_key_keeps_it() {
        let mut app = App::new();
        let keyed = |key: &str| {
            Text::new("Hello")
                .key(Rc::new(ValueKey::new(key.to_owned())))
                .into_widget()
        };
        let harness = mount(&mut app, keyed("a"));
        let paragraph = paragraph_under_root(&harness, &app);

        harness.set_child(&mut app, ltr(keyed("a")));
        harness.pump(&mut app);
        assert_eq!(paragraph_under_root(&harness, &app), paragraph);

        harness.set_child(&mut app, ltr(keyed("b")));
        harness.pump(&mut app);
        assert_ne!(paragraph_under_root(&harness, &app), paragraph);
    }

    #[test]
    fn a_media_query_bolds_scales_and_spaces_the_text() {
        let mut app = App::new();
        let data = MediaQueryData::new()
            .bold_text(true)
            .text_scaler(TextScaler::linear(2.0))
            .line_height_scale_factor_override(1.5)
            .letter_spacing_override(3.0)
            .word_spacing_override(4.0);
        let harness = mount(
            &mut app,
            MediaQuery::new(data, Text::new("Hello")).into_widget(),
        );
        let paragraph = paragraph_under_root(&harness, &app);
        let style = style_of(paragraph, &app);
        assert_eq!(style.font_weight, Some(FontWeight::BOLD));
        assert_eq!(style.height, Some(1.5));
        assert_eq!(style.letter_spacing, Some(3.0));
        assert_eq!(style.word_spacing, Some(4.0));
        assert_eq!(paragraph.text_scaler(&app), TextScaler::linear(2.0));
    }
}
