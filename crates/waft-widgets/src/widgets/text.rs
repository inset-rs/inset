//! Flutter counterpart: `widgets/text.dart`, plus the public `RichText` from `basic.dart`.
//!
//! [`Text`] builds [`RichText`]. Under a [`SelectionContainer`], as under a `SelectableRegion`,
//! it builds a container of its own: a `SelectableTextContainer` whose delegate handles the
//! paragraph's selection, over the foundation in `selectable_region.rs`. `Locale`, `StrutStyle`
//! and `Semantics` wait; see `PORTING.md`.

use std::cmp::Ordering;

use waft_embedder::{Color, FontWeight, TextAlign, TextDirection, TextHeightBehavior};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Ref};
use waft_painting::{
    InlineSpanRef, TextOverflow, TextScaler, TextSpan, TextStyle, TextWidthBasis, transform_rect,
};
use waft_rendering::{
    AnyRenderObject, AnySelectable, RenderBox, RenderHandle, RenderParagraph, SelectedContentRange,
    SelectionEvent, SelectionGeometry, SelectionHandler, SelectionRegistrar, SelectionRegistrarRef,
    SelectionResult,
};
use waft_services::SystemMouseCursors;

use crate::framework::{
    BuildContext, GlobalKey, InheritedWidget, IntoWidget, KeyRef, MultiChildRenderObjectWidget,
    RenderObjectWidget, State, StateData, StatefulWidget, StatelessWidget, WidgetRef,
};
use crate::view::View;
use crate::widgets::basic::{Builder, Directionality, ExcludeSemantics, MouseRegion, Semantics};
use crate::widgets::default_selection_style::DefaultSelectionStyle;
use crate::widgets::inherited_theme::InheritedTheme;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::selectable_region::{
    MultiSelectableSelectionContainerDelegate, MultiSelectableSelectionContainerDelegateBase,
    MultiSelectableSelectionContainerDelegateData, StaticSelectionContainerDelegate,
    StaticSelectionContainerDelegateBase, StaticSelectionContainerDelegateData,
    compare_horizontally, compare_vertically,
};
use crate::widgets::selection_container::{
    SelectionContainer, SelectionContainerDelegate, SelectionContainerDelegateData,
};
use crate::widgets::widget_span::WidgetSpan;

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

    /// Creates a default text style that overrides the text styles in scope at this point in
    /// the widget tree.
    ///
    /// The given `style` is merged with the `style` from the default text style for the
    /// [`BuildContext`] where the widget is inserted, and any of the other arguments that are
    /// not `None` replace the corresponding properties on that same default text style.
    ///
    /// This constructor cannot be used to override the `max_lines` property of the ancestor
    /// with the value `None`, since `None` here is used to mean "defer to ancestor". To
    /// replace a non-`None` `max_lines` from an ancestor with the `None` value (to remove the
    /// restriction on number of lines), manually obtain the ambient [`DefaultTextStyle`]
    /// using [`of`](Self::of), then create a new [`DefaultTextStyle`] using
    /// [`new`](Self::new) directly.
    ///
    /// If a `text_height_behavior` is provided, the existing configuration will be replaced
    /// completely. To retain part of the original `text_height_behavior`, manually obtain the
    /// ambient [`DefaultTextStyle`] using [`of`](Self::of).
    ///
    /// Dart's optional named arguments are the setters of the [`DefaultTextStyleMerge`] this
    /// returns.
    pub fn merge<K>(child: impl IntoWidget<K>) -> DefaultTextStyleMerge {
        DefaultTextStyleMerge {
            key: None,
            style: None,
            text_align: None,
            soft_wrap: None,
            overflow: None,
            max_lines: None,
            text_width_basis: None,
            text_height_behavior: None,
            child: child.into_widget(),
        }
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

/// Dart's `DefaultTextStyle.merge(..)` before its optional arguments are known: a `Builder`
/// that merges them into the ambient [`DefaultTextStyle`].
#[derive(Debug)]
pub struct DefaultTextStyleMerge {
    key: Option<KeyRef>,
    style: Option<TextStyle>,
    text_align: Option<TextAlign>,
    soft_wrap: Option<bool>,
    overflow: Option<TextOverflow>,
    max_lines: Option<i32>,
    text_width_basis: Option<TextWidthBasis>,
    text_height_behavior: Option<TextHeightBehavior>,
    child: WidgetRef,
}

impl DefaultTextStyleMerge {
    /// Dart `DefaultTextStyle.merge(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultTextStyleMerge {
        self.key = Some(key);
        self
    }

    /// Dart `DefaultTextStyle.merge(style:)`.
    pub fn style(mut self, style: TextStyle) -> DefaultTextStyleMerge {
        self.style = Some(style);
        self
    }

    /// Dart `DefaultTextStyle.merge(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> DefaultTextStyleMerge {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `DefaultTextStyle.merge(softWrap:)`.
    pub fn soft_wrap(mut self, soft_wrap: bool) -> DefaultTextStyleMerge {
        self.soft_wrap = Some(soft_wrap);
        self
    }

    /// Dart `DefaultTextStyle.merge(overflow:)`.
    pub fn overflow(mut self, overflow: TextOverflow) -> DefaultTextStyleMerge {
        self.overflow = Some(overflow);
        self
    }

    /// Dart `DefaultTextStyle.merge(maxLines:)`. Must be greater than zero.
    pub fn max_lines(mut self, max_lines: i32) -> DefaultTextStyleMerge {
        debug_assert!(max_lines > 0);
        self.max_lines = Some(max_lines);
        self
    }

    /// Dart `DefaultTextStyle.merge(textWidthBasis:)`.
    pub fn text_width_basis(mut self, text_width_basis: TextWidthBasis) -> DefaultTextStyleMerge {
        self.text_width_basis = Some(text_width_basis);
        self
    }

    /// Dart `DefaultTextStyle.merge(textHeightBehavior:)`.
    pub fn text_height_behavior(
        mut self,
        text_height_behavior: TextHeightBehavior,
    ) -> DefaultTextStyleMerge {
        self.text_height_behavior = Some(text_height_behavior);
        self
    }
}

/// The kind tag of [`IntoWidget`] for a [`DefaultTextStyleMerge`].
pub struct DefaultTextStyleMergeKind;

impl IntoWidget<DefaultTextStyleMergeKind> for DefaultTextStyleMerge {
    fn into_widget(self) -> WidgetRef {
        Builder::new(move |app, context| {
            let parent = DefaultTextStyle::of(app, context);
            DefaultTextStyle {
                key: self.key.clone(),
                style: parent.style.merge(self.style.as_ref()),
                text_align: self.text_align.or(parent.text_align),
                soft_wrap: self.soft_wrap.unwrap_or(parent.soft_wrap),
                overflow: self.overflow.unwrap_or(parent.overflow),
                max_lines: self.max_lines.or(parent.max_lines),
                text_width_basis: self.text_width_basis.unwrap_or(parent.text_width_basis),
                text_height_behavior: self.text_height_behavior.or(parent.text_height_behavior),
                child: self.child.clone(),
            }
            .into_widget()
        })
        .into_widget()
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

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

impl InheritedTheme for DefaultTextStyle {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DefaultTextStyle {
            key: None,
            style: self.style.clone(),
            text_align: self.text_align,
            soft_wrap: self.soft_wrap,
            overflow: self.overflow,
            max_lines: self.max_lines,
            text_width_basis: self.text_width_basis,
            text_height_behavior: self.text_height_behavior,
            child,
        }
        .into_widget()
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

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

impl InheritedTheme for DefaultTextHeightBehavior {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DefaultTextHeightBehavior::new(self.text_height_behavior, child).into_widget()
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
    /// The text to display as an [`InlineSpan`](waft_painting::InlineSpan).
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
    /// An alternative semantics label for this text.
    ///
    /// If present, the semantics of this widget will contain this value instead
    /// of the actual text. This will overwrite any of the semantics labels applied
    /// directly to the [`TextSpan`]s.
    ///
    /// This is useful for replacing abbreviations or shorthands with the full
    /// text value: `Text::new("$$").semantics_label("Double dollars")`.
    pub semantics_label: Option<String>,
    /// A unique identifier for the semantics node for this widget.
    ///
    /// This is useful for cases where the text widget needs to have a uniquely
    /// identifiable ID that is recognized through the automation tools without
    /// having a dependency on the actual content of the text that can possibly be
    /// dynamic in nature.
    pub semantics_identifier: Option<String>,
    /// Defines how to measure the width of the rendered text.
    pub text_width_basis: Option<TextWidthBasis>,
    /// Defines how to apply `TextStyle.height` over and under text.
    pub text_height_behavior: Option<TextHeightBehavior>,
    /// The color to use when painting the selection.
    ///
    /// This is ignored if [`SelectionContainer::maybe_of`] returns `None` in the
    /// [`BuildContext`] of this widget.
    ///
    /// If `None`, the ambient [`DefaultSelectionStyle`] is used (if any); failing that, the
    /// selection color defaults to [`DefaultSelectionStyle::DEFAULT_COLOR`].
    pub selection_color: Option<Color>,
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

    /// Creates a text widget with an [`InlineSpan`](waft_painting::InlineSpan).
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
            semantics_label: None,
            semantics_identifier: None,
            text_width_basis: None,
            text_height_behavior: None,
            selection_color: None,
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

    /// Dart `Text(semanticsLabel:)`.
    pub fn semantics_label(mut self, semantics_label: impl Into<String>) -> Text {
        self.semantics_label = Some(semantics_label.into());
        self
    }

    /// Dart `Text(semanticsIdentifier:)`.
    pub fn semantics_identifier(mut self, semantics_identifier: impl Into<String>) -> Text {
        self.semantics_identifier = Some(semantics_identifier.into());
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

    /// Dart `Text(selectionColor:)`.
    pub fn selection_color(mut self, selection_color: Color) -> Text {
        self.selection_color = Some(selection_color);
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
                ..TextSpan::default()
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
        let text_align = self
            .text_align
            .or(default_text_style.text_align)
            .unwrap_or(TextAlign::Start);
        let soft_wrap = self.soft_wrap.unwrap_or(default_text_style.soft_wrap);
        let overflow = self
            .overflow
            .or(effective_text_style
                .as_ref()
                .and_then(|style| style.overflow))
            .unwrap_or(default_text_style.overflow);
        let text_width_basis = self
            .text_width_basis
            .unwrap_or(default_text_style.text_width_basis);
        let max_lines = self.max_lines.or(default_text_style.max_lines);
        let default_selection_style = DefaultSelectionStyle::of(app, context);
        let selection_color = self
            .selection_color
            .or_else(|| default_selection_style.selection_color.as_deref().copied())
            .unwrap_or(DefaultSelectionStyle::DEFAULT_COLOR);
        let registrar = SelectionContainer::maybe_of(app, context);
        let mut result = if registrar.is_some() {
            // A mouse hovering the selectable text shows the text cursor, as `SelectableRegion`'s
            // text does.
            let cursor = default_selection_style
                .mouse_cursor
                .clone()
                .unwrap_or_else(|| SystemMouseCursors::TEXT.into());
            MouseRegion::new()
                .cursor(cursor)
                .child(SelectableTextContainer {
                    text: effective_text_span,
                    text_align,
                    text_direction: self.text_direction,
                    soft_wrap,
                    overflow,
                    text_scaler,
                    max_lines,
                    text_width_basis,
                    text_height_behavior,
                    selection_color,
                })
                .into_widget()
        } else {
            let mut rich_text = RichText::new(effective_text_span.into_span())
                .text_align(text_align)
                .soft_wrap(soft_wrap)
                .overflow(overflow)
                .text_scaler(text_scaler)
                .text_width_basis(text_width_basis)
                .selection_color(selection_color);
            // RichText uses Directionality.of to obtain a default if this is None.
            if let Some(text_direction) = self.text_direction {
                rich_text = rich_text.text_direction(text_direction);
            }
            if let Some(max_lines) = max_lines {
                rich_text = rich_text.max_lines(max_lines);
            }
            if let Some(text_height_behavior) = text_height_behavior {
                rich_text = rich_text.text_height_behavior(text_height_behavior);
            }
            rich_text.into_widget()
        };
        if self.semantics_label.is_some() || self.semantics_identifier.is_some() {
            let mut semantics = Semantics::new();
            if let Some(text_direction) = self.text_direction {
                semantics = semantics.text_direction(text_direction);
            }
            if let Some(semantics_label) = &self.semantics_label {
                semantics = semantics.label(semantics_label.clone());
            }
            if let Some(semantics_identifier) = &self.semantics_identifier {
                semantics = semantics.identifier(semantics_identifier.clone());
            }
            result = semantics
                .child(
                    ExcludeSemantics::new()
                        .excluding(self.semantics_label.is_some())
                        .child(result),
                )
                .into_widget();
        }
        result
    }
}

// ---------------------------------------------------------------------------------------------
// SelectableTextContainer

/// Dart's `_SelectableTextContainer`: the [`SelectionContainer`] a selectable [`Text`] builds, so
/// that its paragraph's selectables register with it and its delegate handles the paragraph's
/// selection.
#[derive(Debug)]
struct SelectableTextContainer {
    text: TextSpan,
    text_align: TextAlign,
    text_direction: Option<TextDirection>,
    soft_wrap: bool,
    overflow: TextOverflow,
    text_scaler: TextScaler,
    max_lines: Option<i32>,
    text_width_basis: TextWidthBasis,
    text_height_behavior: Option<TextHeightBehavior>,
    selection_color: Color,
}

impl StatefulWidget for SelectableTextContainer {
    type State = SelectableTextContainerState;

    fn create_state(&self) -> SelectableTextContainerState {
        SelectableTextContainerState {
            state: StateData::new(),
            selection_delegate: None,
            text_key: GlobalKey::new(),
        }
    }
}

/// The state of a [`SelectableTextContainer`]: Dart's `_SelectableTextContainerState`.
struct SelectableTextContainerState {
    state: StateData<SelectableTextContainer>,
    selection_delegate: Option<Ref<SelectableTextContainerDelegate>>,
    /// The key of the paragraph below, which the delegate finds its `RenderParagraph` by.
    text_key: GlobalKey,
}

impl SelectableTextContainerState {
    fn selection_delegate(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<SelectableTextContainerDelegate> {
        app.get(self)
            .selection_delegate
            .as_ref()
            .expect("the delegate is created in init_state")
            .handle()
    }
}

impl State for SelectableTextContainerState {
    type Widget = SelectableTextContainer;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let text_key = app.get(self).text_key.clone();
        let delegate = SelectableTextContainerDelegate::new(app, text_key);
        app.get_mut(self).selection_delegate = Some(delegate);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let delegate = self.selection_delegate(app);
        MultiSelectableSelectionContainerDelegate::dispose(delegate, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let delegate = self.selection_delegate(app).keep(app).into_erased();
        let widget = self.widget(app);
        SelectionContainer::new(
            delegate,
            SelectableRichText {
                text_key: app.get(self).text_key.clone(),
                text: widget.text.clone(),
                text_align: widget.text_align,
                text_direction: widget.text_direction,
                soft_wrap: widget.soft_wrap,
                overflow: widget.overflow,
                text_scaler: widget.text_scaler.clone(),
                max_lines: widget.max_lines,
                text_width_basis: widget.text_width_basis,
                text_height_behavior: widget.text_height_behavior,
                selection_color: widget.selection_color,
            },
        )
        .into_widget()
    }
}

/// Dart's `_RichText`: the [`RichText`] under the container, which reads the container's
/// registrar and registers the paragraph's selectables with it.
#[derive(Debug)]
struct SelectableRichText {
    text_key: GlobalKey,
    text: TextSpan,
    text_align: TextAlign,
    text_direction: Option<TextDirection>,
    soft_wrap: bool,
    overflow: TextOverflow,
    text_scaler: TextScaler,
    max_lines: Option<i32>,
    text_width_basis: TextWidthBasis,
    text_height_behavior: Option<TextHeightBehavior>,
    selection_color: Color,
}

impl StatelessWidget for SelectableRichText {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let registrar = SelectionContainer::maybe_of(app, context)
            .expect("the text's own selection container hosts a registrar");
        let mut rich_text = RichText::new(self.text.clone().into_span())
            .key(KeyRef::Global(self.text_key.clone()))
            .text_align(self.text_align)
            .soft_wrap(self.soft_wrap)
            .overflow(self.overflow)
            .text_scaler(self.text_scaler.clone())
            .text_width_basis(self.text_width_basis)
            .selection_registrar(registrar)
            .selection_color(self.selection_color);
        if let Some(text_direction) = self.text_direction {
            rich_text = rich_text.text_direction(text_direction);
        }
        if let Some(max_lines) = self.max_lines {
            rich_text = rich_text.max_lines(max_lines);
        }
        if let Some(text_height_behavior) = self.text_height_behavior {
            rich_text = rich_text.text_height_behavior(text_height_behavior);
        }
        rich_text.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// SelectableTextContainerDelegate

/// Dart's `_SelectableTextContainerDelegate`: the delegate of a [`Text`]'s selection container.
///
/// It handles the paragraph's selection across the fragments a `WidgetSpan` splits it into, and
/// finds its `RenderParagraph` through the container's global key.
struct SelectableTextContainerDelegate {
    selection_container_delegate: SelectionContainerDelegateData,
    multi_selectable_selection_container_delegate: MultiSelectableSelectionContainerDelegateData,
    static_selection_container_delegate: StaticSelectionContainerDelegateData,
    /// The key of the paragraph this delegate handles the selection of.
    text_key: GlobalKey,
    /// Where each fragment of the paragraph after its first starts in it, recorded when the
    /// selectables change: reading it needs `&mut App`, and `selection` has `&App` (see
    /// `PORTING.md`).
    selectable_content_starts: Vec<(AnySelectable, i32)>,
}

impl SelectableTextContainerDelegate {
    fn new(app: &mut App, text_key: GlobalKey) -> Ref<SelectableTextContainerDelegate> {
        app.create(SelectableTextContainerDelegate {
            selection_container_delegate: SelectionContainerDelegateData::new(),
            multi_selectable_selection_container_delegate:
                MultiSelectableSelectionContainerDelegateData::new(),
            static_selection_container_delegate: StaticSelectionContainerDelegateData::new(),
            text_key,
            selectable_content_starts: Vec::new(),
        })
    }

    /// Dart's `paragraph` getter.
    fn paragraph(self: Handle<Self>, app: &App) -> RenderHandle<RenderParagraph> {
        let context = app
            .get(self)
            .text_key
            .current_context(app)
            .expect("the text's paragraph is in the tree");
        context
            .find_render_object(app)
            .expect("the text's paragraph has a render object")
            .downcast::<RenderParagraph>(app)
            .expect("the text's render object is a RenderParagraph")
    }

    /// Where `selectable` starts in the paragraph, as recorded by
    /// [`record_selectable_content_starts`](Self::record_selectable_content_starts).
    fn selectable_content_start(
        self: Handle<Self>,
        app: &App,
        selectable: AnySelectable,
    ) -> Option<i32> {
        app.get(self)
            .selectable_content_starts
            .iter()
            .find_map(|(held, start)| (*held == selectable).then_some(*start))
    }

    /// Records where each fragment after the paragraph's first starts in it: Flutter reads it
    /// from the fragment's first box when it computes the range, which needs `&mut App` there.
    fn record_selectable_content_starts(self: Handle<Self>, app: &mut App) {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let paragraph = self.paragraph(app);
        let mut starts = Vec::new();
        for (index, selectable) in selectables.iter().enumerate() {
            if index == 0 || !paragraph.selectable_belongs_to_paragraph(app, *selectable) {
                continue;
            }
            let Some(first_box) = selectable.bounding_boxes(app).into_iter().next() else {
                continue;
            };
            let position = paragraph.get_position_for_offset(app, first_box.center_left());
            starts.push((*selectable, position.offset));
        }
        app.get_mut(self).selectable_content_starts = starts;
    }

    /// Dart's `compareOrder` for a text paragraph: sorts the selectables by their first
    /// bounding box's top left, where the multi-selectable delegate sorts by their union.
    fn compare_top_left_screen_order(
        self: Handle<Self>,
        app: &mut App,
        a: AnySelectable,
        b: AnySelectable,
    ) -> Ordering {
        let rect_a = transform_rect(
            &a.get_transform_to(app, None),
            a.bounding_boxes(app)
                .into_iter()
                .next()
                .expect("a selectable has a bounding box"),
        );
        let rect_b = transform_rect(
            &b.get_transform_to(app, None),
            b.bounding_boxes(app)
                .into_iter()
                .next()
                .expect("a selectable has a bounding box"),
        );
        let result = compare_vertically(rect_a, rect_b);
        if result != Ordering::Equal {
            return result;
        }
        compare_horizontally(rect_a, rect_b)
    }

    /// Dart's `handleSelectParagraph`, whose own body is `_handleSelectParagraph`.
    fn handle_select_paragraph_body(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let SelectionEvent::SelectParagraph {
            global_position,
            absorb,
        } = event
        else {
            panic!("a paragraph selection event selects a paragraph");
        };
        let global_position = *global_position;
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        if *absorb {
            for selectable in &selectables {
                MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                    self,
                    app,
                    *selectable,
                    event,
                );
            }
            let data = self.multi_selectable_selection_container_delegate_data_mut(app);
            data.current_selection_start_index = 0;
            data.current_selection_end_index = selectables.len() as i32 - 1;
            return SelectionResult::Next;
        }

        // First pass: if the position is on a placeholder, dispatch the selection event to the
        // selectable at the location and terminate.
        let paragraph = self.paragraph(app);
        for (index, selectable) in selectables.iter().enumerate() {
            let selectable_is_placeholder =
                !paragraph.selectable_belongs_to_paragraph(app, *selectable);
            if selectable_is_placeholder {
                let boxes = selectable.bounding_boxes(app);
                if !boxes.is_empty() {
                    let transform = selectable.get_transform_to(app, None);
                    for rect in boxes {
                        let global_rect = transform_rect(&transform, rect);
                        if global_rect.contains(global_position) {
                            let data =
                                self.multi_selectable_selection_container_delegate_data_mut(app);
                            data.current_selection_start_index = index as i32;
                            data.current_selection_end_index = index as i32;
                            return MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                                self, app, *selectable, event,
                            );
                        }
                    }
                }
            }
        }

        let mut last_selection_result: Option<SelectionResult> = None;
        let mut found_start = false;
        let mut last_next_index: Option<i32> = None;
        for (index, selectable) in selectables.iter().enumerate() {
            if !paragraph.selectable_belongs_to_paragraph(app, *selectable) {
                if found_start {
                    let synthesized_event = SelectionEvent::SelectParagraph {
                        global_position,
                        absorb: true,
                    };
                    let result =
                        MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                            self,
                            app,
                            *selectable,
                            &synthesized_event,
                        );
                    if selectables.len() - 1 == index {
                        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                        data.current_selection_end_index = index as i32;
                        MultiSelectableSelectionContainerDelegateBase::flush_inactive_selections(
                            self, app,
                        );
                        return result;
                    }
                }
                continue;
            }
            let existing_geometry = selectable.value(app);
            last_selection_result = Some(
                MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                    self,
                    app,
                    *selectable,
                    event,
                ),
            );
            let result = last_selection_result.expect("assigned just now");
            if index == selectables.len() - 1 && result == SelectionResult::Next {
                let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                if found_start {
                    data.current_selection_end_index = index as i32;
                } else {
                    data.current_selection_start_index = index as i32;
                    data.current_selection_end_index = index as i32;
                }
                return SelectionResult::Next;
            }
            if result == SelectionResult::Next {
                if selectable.value(app) == existing_geometry && !found_start {
                    last_next_index = Some(index as i32);
                }
                if selectable.value(app) != existing_geometry && !found_start {
                    let boxes = selectable.bounding_boxes(app);
                    debug_assert!(!boxes.is_empty());
                    let selection_rects = selectable.value(app).selection_rects;
                    debug_assert!(!selection_rects.is_empty());
                    let selection_at_start_of_selectable = boxes[0].overlaps(selection_rects[0]);
                    let start_index = if let Some(last_next_index) = last_next_index
                        && selection_at_start_of_selectable
                    {
                        last_next_index + 1
                    } else if last_next_index.is_none() && selection_at_start_of_selectable {
                        0
                    } else {
                        index as i32
                    };
                    for selectable in &selectables[start_index as usize..index] {
                        let synthesized_event = SelectionEvent::SelectParagraph {
                            global_position,
                            absorb: true,
                        };
                        MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                            self,
                            app,
                            *selectable,
                            &synthesized_event,
                        );
                    }
                    let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                    data.current_selection_start_index = start_index;
                    found_start = true;
                }
                continue;
            }
            if index == 0 && result == SelectionResult::Previous {
                return SelectionResult::Previous;
            }
            if selectable.value(app) != existing_geometry {
                if !found_start && last_next_index.is_none() {
                    let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                    data.current_selection_start_index = 0;
                    for selectable in &selectables[..index] {
                        let synthesized_event = SelectionEvent::SelectParagraph {
                            global_position,
                            absorb: true,
                        };
                        MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                            self,
                            app,
                            *selectable,
                            &synthesized_event,
                        );
                    }
                }
                let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                data.current_selection_end_index = index as i32;
                // Geometry changed as a result of selecting a paragraph; clear the other
                // selectables to keep the selection in sync.
                MultiSelectableSelectionContainerDelegateBase::flush_inactive_selections(self, app);
            }
            return SelectionResult::End;
        }
        debug_assert!(last_selection_result.is_none());
        SelectionResult::End
    }

    /// Dart's `_adjustSelection`: [`MultiSelectableSelectionContainerDelegateBase::adjust_selection`]
    /// with the crossing of the opposite edge that a paragraph selection needs.
    fn adjust_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
        is_end: bool,
    ) -> SelectionResult {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let value = SelectionHandler::value(self, app);
        let is_current_edge_within_viewport = if is_end {
            value.end_selection_point.is_some()
        } else {
            value.start_selection_point.is_some()
        };
        let is_opposite_edge_within_viewport = if is_end {
            value.start_selection_point.is_some()
        } else {
            value.end_selection_point.is_some()
        };
        let data = self.multi_selectable_selection_container_delegate_data(app);
        let mut new_index: i32 = if is_end {
            if is_current_edge_within_viewport {
                data.current_selection_end_index
            } else if is_opposite_edge_within_viewport {
                data.current_selection_start_index
            } else {
                0
            }
        } else if is_current_edge_within_viewport {
            data.current_selection_start_index
        } else if is_opposite_edge_within_viewport {
            data.current_selection_end_index
        } else {
            0
        };
        debug_assert!(new_index < selectables.len() as i32 && new_index >= 0);
        let mut forward: Option<bool> = None;
        let mut final_result: Option<SelectionResult> = None;
        while (new_index as usize) < selectables.len() && new_index >= 0 && final_result.is_none() {
            let current_selectable_result =
                MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                    self,
                    app,
                    selectables[new_index as usize],
                    event,
                );
            match current_selectable_result {
                SelectionResult::End | SelectionResult::Pending | SelectionResult::None => {
                    final_result = Some(current_selectable_result);
                }
                SelectionResult::Next => {
                    if forward == Some(false) {
                        new_index += 1;
                        final_result = Some(SelectionResult::End);
                    } else if new_index as usize == selectables.len() - 1 {
                        final_result = Some(current_selectable_result);
                    } else {
                        forward = Some(true);
                        new_index += 1;
                    }
                }
                SelectionResult::Previous => {
                    if forward == Some(true) {
                        new_index -= 1;
                        final_result = Some(SelectionResult::End);
                    } else if new_index == 0 {
                        final_result = Some(current_selectable_result);
                    } else {
                        forward = Some(false);
                        new_index -= 1;
                    }
                }
            }
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        if is_end {
            let forward_selection =
                data.current_selection_end_index >= data.current_selection_start_index;
            if let Some(forward) = forward
                && ((!forward_selection
                    && forward
                    && new_index >= data.current_selection_start_index)
                    || (forward_selection
                        && !forward
                        && new_index <= data.current_selection_start_index))
            {
                data.current_selection_start_index = data.current_selection_end_index;
            }
            data.current_selection_end_index = new_index;
        } else {
            let forward_selection =
                data.current_selection_end_index >= data.current_selection_start_index;
            if let Some(forward) = forward
                && ((!forward_selection
                    && !forward
                    && new_index <= data.current_selection_end_index)
                    || (forward_selection
                        && forward
                        && new_index >= data.current_selection_end_index))
            {
                data.current_selection_end_index = data.current_selection_start_index;
            }
            data.current_selection_start_index = new_index;
        }
        MultiSelectableSelectionContainerDelegateBase::flush_inactive_selections(self, app);
        final_result.expect("the loop answered a result")
    }

    /// Dart's `_calculateLocalRange`: the selectables' ranges, with a fragment that is not the
    /// paragraph's first offset by where it starts in the paragraph.
    fn calculate_local_range(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        if data.current_selection_start_index == -1 || data.current_selection_end_index == -1 {
            return None;
        }
        let selectables = data.selectables.clone();
        let mut start_offset = 0i32;
        let mut end_offset = 0i32;
        let mut found_start = false;
        let mut forward_selection =
            data.current_selection_end_index >= data.current_selection_start_index;
        if data.current_selection_end_index == data.current_selection_start_index {
            // Determining the direction is inaccurate when both edges share a selectable; the
            // range within it is the source of truth.
            let range = selectables[data.current_selection_start_index as usize]
                .selection(app)
                .expect("the selectable holding both edges has a selection");
            forward_selection = range.end_offset >= range.start_offset;
        }
        let paragraph = self.paragraph(app);
        for (index, selectable) in selectables.iter().enumerate() {
            let Some(range) = selectable.selection(app) else {
                if found_start {
                    return Some(SelectedContentRange::new(
                        if forward_selection {
                            start_offset
                        } else {
                            end_offset
                        },
                        if forward_selection {
                            end_offset
                        } else {
                            start_offset
                        },
                    ));
                }
                start_offset += selectable.content_length(app) as i32;
                end_offset = start_offset;
                continue;
            };
            let selection_start_normalized = range.start_offset.min(range.end_offset);
            let selection_end_normalized = range.start_offset.max(range.end_offset);
            if !found_start {
                // A `RenderParagraph` may split its content into several selectables, so a
                // fragment that is not the paragraph's first starts partway through it.
                let should_consider_content_start =
                    index > 0 && paragraph.selectable_belongs_to_paragraph(app, *selectable);
                let selectable_content_start = if should_consider_content_start {
                    self.selectable_content_start(app, *selectable).unwrap_or(0)
                } else {
                    0
                };
                start_offset += (selection_start_normalized - selectable_content_start).abs();
                end_offset =
                    start_offset + (selection_end_normalized - selection_start_normalized).abs();
                found_start = true;
            } else {
                end_offset += (selection_end_normalized - selection_start_normalized).abs();
            }
        }
        debug_assert!(
            found_start,
            "The start of the selection has not been found despite this selection delegate \
             having an existing currentSelectionStartIndex and currentSelectionEndIndex."
        );
        Some(SelectedContentRange::new(
            if forward_selection {
                start_offset
            } else {
                end_offset
            },
            if forward_selection {
                end_offset
            } else {
                start_offset
            },
        ))
    }
}

impl SelectionContainerDelegate for SelectableTextContainerDelegate {
    fn selection_container_delegate_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SelectionContainerDelegateData {
        &app.get(self).selection_container_delegate
    }

    fn selection_container_delegate_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SelectionContainerDelegateData {
        &mut app.get_mut(self).selection_container_delegate
    }
}

impl MultiSelectableSelectionContainerDelegate for SelectableTextContainerDelegate {
    crate::multi_selectable_selection_container_delegate_accessors!();

    fn did_change_selectables(self: Handle<Self>, app: &mut App) {
        self.record_selectable_content_starts(app);
        StaticSelectionContainerDelegateBase::did_change_selectables(self, app);
    }

    fn compare_order(
        self: Handle<Self>,
        app: &mut App,
        a: AnySelectable,
        b: AnySelectable,
    ) -> Ordering {
        self.compare_top_left_screen_order(app, a, b)
    }

    fn handle_select_all(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        StaticSelectionContainerDelegateBase::handle_select_all(self, app, event)
    }

    fn handle_select_word(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        StaticSelectionContainerDelegateBase::handle_select_word(self, app, event)
    }

    fn handle_select_paragraph(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let result = self.handle_select_paragraph_body(app, event);
        StaticSelectionContainerDelegate::did_receive_selection_boundary_events(self, app);
        result
    }

    fn handle_clear_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        StaticSelectionContainerDelegateBase::handle_clear_selection(self, app, event)
    }

    fn handle_selection_edge_update(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let (global_position, for_end, granularity) = match event {
            SelectionEvent::StartEdgeUpdate {
                global_position,
                granularity,
            } => (*global_position, false, *granularity),
            SelectionEvent::EndEdgeUpdate {
                global_position,
                granularity,
            } => (*global_position, true, *granularity),
            _ => panic!("a selection edge update names an edge"),
        };
        if granularity != waft_rendering::TextGranularity::Paragraph {
            return StaticSelectionContainerDelegateBase::handle_selection_edge_update(
                self, app, event,
            );
        }
        StaticSelectionContainerDelegate::update_last_selection_edge_location(
            self,
            app,
            global_position,
            for_end,
        );
        let selection_index = {
            let data = self.multi_selectable_selection_container_delegate_data(app);
            if for_end {
                data.current_selection_end_index
            } else {
                data.current_selection_start_index
            }
        };
        if selection_index == -1 {
            return StaticSelectionContainerDelegateBase::handle_selection_edge_update(
                self, app, event,
            );
        }
        self.adjust_selection(app, event, for_end)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        StaticSelectionContainerDelegateBase::dispose(self, app);
    }

    fn ensure_child_updated(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        StaticSelectionContainerDelegateBase::ensure_child_updated(self, app, selectable);
    }

    fn dispatch_selection_event_to_child(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        event: &SelectionEvent,
    ) -> SelectionResult {
        StaticSelectionContainerDelegateBase::dispatch_selection_event_to_child(
            self, app, selectable, event,
        )
    }
}

impl StaticSelectionContainerDelegate for SelectableTextContainerDelegate {
    crate::static_selection_container_delegate_accessors!();
}

impl SelectionHandler for SelectableTextContainerDelegate {
    fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
        StaticSelectionContainerDelegateBase::value(self, app)
    }

    fn push_handle_layers(
        self: Handle<Self>,
        app: &mut App,
        start_handle: Option<Ref<waft_rendering::LayerLink>>,
        end_handle: Option<Ref<waft_rendering::LayerLink>>,
    ) {
        StaticSelectionContainerDelegateBase::push_handle_layers(
            self,
            app,
            start_handle,
            end_handle,
        );
    }

    fn selected_content(self: Handle<Self>, app: &App) -> Option<waft_rendering::SelectedContent> {
        StaticSelectionContainerDelegateBase::selected_content(self, app)
    }

    fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
        self.calculate_local_range(app)
    }

    fn dispatch_selection_event(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        StaticSelectionContainerDelegateBase::dispatch_selection_event(self, app, event)
    }

    fn content_length(self: Handle<Self>, app: &App) -> usize {
        StaticSelectionContainerDelegateBase::content_length(self, app)
    }
}

impl SelectionRegistrar for SelectableTextContainerDelegate {
    crate::static_selection_container_delegate_selection_registrar!();
}

impl ChangeNotifier for SelectableTextContainerDelegate {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self
            .multi_selectable_selection_container_delegate
            .change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self
            .multi_selectable_selection_container_delegate
            .change_notifier
    }
}

// ---------------------------------------------------------------------------------------------
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
        recognizer: text_span.recognizer.clone(),
        mouse_cursor: text_span.mouse_cursor.clone(),
        on_enter: text_span.on_enter.clone(),
        on_exit: text_span.on_exit.clone(),
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
    /// The inline widgets of [`text`](Self::text), its `WidgetSpan`s at
    /// [`text_scaler`](Self::text_scaler), as Dart's constructor extracts them.
    children: Vec<WidgetRef>,
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
    /// The [`SelectionRegistrar`](waft_rendering::SelectionRegistrar) this rich text is
    /// subscribed to.
    ///
    /// If this is set, [`selection_color`](Self::selection_color) must be non-null.
    pub selection_registrar: Option<SelectionRegistrarRef>,
    /// The color to use when painting the selection.
    pub selection_color: Option<Color>,
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
            children: WidgetSpan::extract_from_inline_span(&*text, &TextScaler::NO_SCALING),
            text,
            text_align: TextAlign::Start,
            text_direction: None,
            soft_wrap: true,
            overflow: TextOverflow::Clip,
            text_scaler: TextScaler::NO_SCALING,
            max_lines: None,
            text_width_basis: TextWidthBasis::Parent,
            text_height_behavior: None,
            selection_registrar: None,
            selection_color: None,
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
        self.children = WidgetSpan::extract_from_inline_span(&*self.text, &text_scaler);
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

    /// Dart `RichText(selectionRegistrar:)`.
    pub fn selection_registrar(mut self, registrar: SelectionRegistrarRef) -> RichText {
        self.selection_registrar = Some(registrar);
        self
    }

    /// Dart `RichText(selectionColor:)`.
    pub fn selection_color(mut self, color: Color) -> RichText {
        self.selection_color = Some(color);
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

    fn debug_check_constructor(&self) {
        debug_assert!(self.selection_registrar.is_none() || self.selection_color.is_some());
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
        render_object.set_registrar(app, self.selection_registrar.clone());
        render_object.set_selection_color(app, self.selection_color);
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
        render_object.set_registrar(app, self.selection_registrar.clone());
        render_object.set_selection_color(app, self.selection_color);
    }
}

impl MultiChildRenderObjectWidget for RichText {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

#[cfg(test)]
mod tests {
    use waft_foundation::AppCell;

    use waft_embedder::{Color, Offset, Rect, TextLeadingDistribution, TextPosition};
    use waft_painting::{PaintingBinding, transform_point};

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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let span = TextSpan::new().text("Hello").into_span();
        let harness = mount(&mut app, Text::rich(span).into_widget());
        let paragraph = paragraph_under_root(&harness, &app);
        assert_eq!(plain_text_of(paragraph, &app), "Hello");
        assert!(paragraph.size(&app).width() > 0.0);
    }

    #[test]
    fn a_default_text_style_styles_the_text_and_notifies_on_change() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyed = |key: &str| {
            Text::new("Hello")
                .key(ValueKey::new(key.to_owned()).into())
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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

    // ---- selection ----

    use waft_foundation::ValueKey;
    use waft_rendering::{SelectionEvent, SelectionStatus};

    use waft_scheduler::{FrameTarget, SchedulerBinding};

    use crate::testing::{WidgetTester, find, test_widgets};
    use crate::widgets::basic::{Column, SizedBox};
    use crate::widgets::selectable_region::PlainStaticSelectionContainerDelegate;
    use crate::widgets::selection_container::SelectionContainer;

    /// Pumps `texts` in a column under a selection container, as under a `SelectableRegion`, and
    /// answers the container's delegate.
    fn pump_selectable_texts(
        tester: &mut WidgetTester,
        texts: Vec<WidgetRef>,
    ) -> Ref<PlainStaticSelectionContainerDelegate> {
        let delegate = tester.update(PlainStaticSelectionContainerDelegate::new);
        tester.pump_widget(Directionality::new(
            TextDirection::Ltr,
            SelectionContainer::new(delegate.erase(), Column::new().children(texts)),
        ));
        tester.pump_and_settle();
        // The outer container registers itself from a microtask after the frame, so its
        // selectable reaches the delegate at the next frame.
        tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::EveryView));
        tester.pump();
        tester.pump_and_settle();
        delegate
    }

    /// The global position of the caret at `offset` in `paragraph`, five pixels down from the
    /// top of the line.
    fn position_of(
        tester: &mut WidgetTester,
        paragraph: RenderHandle<RenderParagraph>,
        offset: i32,
    ) -> Offset {
        tester.update(|app| {
            let local = paragraph.get_offset_for_caret(app, TextPosition::new(offset), Rect::ZERO)
                + Offset::new(0.0, 5.0);
            transform_point(&paragraph.as_object().get_transform_to(app, None), local)
        })
    }

    /// Sends `event` to `handle`.
    fn dispatch<D: SelectionHandler>(
        tester: &mut WidgetTester,
        handle: Handle<D>,
        event: SelectionEvent,
    ) {
        tester.update(|app| {
            SelectionHandler::dispatch_selection_event(handle, app, &event);
        });
    }

    /// The selected range and status of `handle`.
    fn assert_selection<D: SelectionHandler>(
        tester: &WidgetTester,
        handle: Handle<D>,
        expected_range: Option<(i32, i32)>,
        expected_status: SelectionStatus,
    ) {
        tester.update(|app| {
            let range = SelectionHandler::selection(handle, app);
            assert_eq!(
                range.map(|range| (range.start_offset, range.end_offset)),
                expected_range
            );
            assert_eq!(SelectionHandler::value(handle, app).status, expected_status);
        });
    }

    /// The plain text `handle` has selected, or `None`.
    fn selected_text<D: SelectionHandler>(
        tester: &WidgetTester,
        handle: Handle<D>,
    ) -> Option<String> {
        tester.update(|app| {
            SelectionHandler::selected_content(handle, app).map(|content| content.plain_text)
        })
    }

    /// The paragraph showing `text`.
    fn paragraph_showing(tester: &WidgetTester, text: &str) -> RenderHandle<RenderParagraph> {
        let paragraphs = tester.all::<RenderParagraph>();
        tester.update(|app| {
            *paragraphs
                .iter()
                .find(|paragraph| paragraph.text(app).to_plain_text(true, true) == text)
                .unwrap_or_else(|| panic!("a paragraph showing {text}"))
        })
    }

    /// `paragraph`'s one selection, as its start and end.
    fn paragraph_selection(
        tester: &WidgetTester,
        paragraph: RenderHandle<RenderParagraph>,
    ) -> (i32, i32) {
        tester.update(|app| {
            let selections = paragraph.selections(app);
            assert_eq!(selections.len(), 1, "one selection");
            (selections[0].start(), selections[0].end())
        })
    }

    // selectable_region_test.dart 'onSelectionChanged SelectedContentRange is accurate', now
    // through `Text`, which builds its own selection container.
    #[test]
    fn the_selection_range_covers_the_texts_between_the_edges() {
        test_widgets(|tester| {
            let delegate = pump_selectable_texts(
                tester,
                vec![
                    Text::new("How are you?").into_widget(),
                    Text::new("Good, and you?").into_widget(),
                    Text::new("Fine, thank you.").into_widget(),
                ],
            );
            let paragraphs = tester.all::<RenderParagraph>();
            assert_eq!(paragraphs.len(), 3);
            let start = position_of(tester, paragraphs[0], 4);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );

            let end = position_of(tester, paragraphs[0], 7);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 7)),
                SelectionStatus::Uncollapsed,
            );

            let end = position_of(tester, paragraphs[0], 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 10)),
                SelectionStatus::Uncollapsed,
            );

            let end = position_of(tester, paragraphs[1], 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 22)),
                SelectionStatus::Uncollapsed,
            );

            let end = position_of(tester, paragraphs[2], 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 36)),
                SelectionStatus::Uncollapsed,
            );

            // A collapsed selection, as a click makes one.
            dispatch(tester, delegate.handle(), SelectionEvent::Clear);
            let point = position_of(tester, paragraphs[0], 3);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(point, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(point, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((3, 3)),
                SelectionStatus::Collapsed,
            );

            // A backward selection.
            dispatch(tester, delegate.handle(), SelectionEvent::Clear);
            let start = position_of(tester, paragraphs[2], 4);
            let end = position_of(tester, paragraphs[0], 0);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((30, 0)),
                SelectionStatus::Uncollapsed,
            );
        });
    }

    // selectable_region_test.dart 'can select word when selectable is broken up by an
    // unselectable WidgetSpan': the fragment that starts after the span is selected, and the
    // range counts from the paragraph's start.
    #[test]
    fn a_word_after_a_widget_span_is_selected_and_ranged_from_the_paragraph_start() {
        const BEFORE: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";
        // The 123 characters, then the placeholder, then "Hello, world.": the word is at 124.
        assert_eq!(BEFORE.len(), 123, "as in the test");
        test_widgets(|tester| {
            let span = TextSpan::new()
                .children(vec![
                    TextSpan::new().text(BEFORE).into_span(),
                    WidgetSpan::new(SizedBox::shrink()).into_span(),
                    TextSpan::new().text("Hello, world.").into_span(),
                ])
                .into_span();
            let delegate = pump_selectable_texts(tester, vec![Text::rich(span).into_widget()]);
            let paragraphs = tester.all::<RenderParagraph>();
            assert_eq!(paragraphs.len(), 1);
            let position = position_of(tester, paragraphs[0], 125);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::SelectWord {
                    global_position: position,
                },
            );

            assert_eq!(paragraph_selection(tester, paragraphs[0]), (124, 129));
            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("Hello".to_string())
            );
            // Dart's range counts the first fragment's whole content and then the distance to
            // where the selected fragment starts in the paragraph, so the placeholder the
            // unselected fragment ends at is not counted.
            assert_selection(
                tester,
                delegate.handle(),
                Some((123, 128)),
                SelectionStatus::Uncollapsed,
            );
        });
    }

    // selectable_region_test.dart 'selects backwards across multiple Text widgets and
    // WidgetSpans via mouse drag': every inner and outer text is fully selected.
    #[test]
    fn a_backward_drag_selects_every_inner_and_outer_text() {
        test_widgets(|tester| {
            let texts = (0..5)
                .map(|index| {
                    let number = index + 1;
                    Text::rich(
                        TextSpan::new()
                            .children(vec![
                                WidgetSpan::new(Text::new(format!("{number}. "))).into_span(),
                                TextSpan::new().text(format!("Item {number}")).into_span(),
                            ])
                            .into_span(),
                    )
                    .key(ValueKey::new(index as usize).into())
                    .into_widget()
                })
                .collect();
            let delegate = pump_selectable_texts(tester, texts);
            let drag_start = tester.get_bottom_right(find::by_key(ValueKey::new(4usize)));
            let drag_end = tester.get_top_left(find::by_key(ValueKey::new(0usize)));
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(drag_start, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(drag_end, None),
            );

            for index in 0..5 {
                let number = index + 1;
                let inner = paragraph_showing(tester, &format!("{number}. "));
                let length =
                    tester.update(|app| inner.text(app).to_plain_text(true, true).len() as i32);
                assert_eq!(paragraph_selection(tester, inner), (0, length));

                let outer_text = format!("\u{FFFC}Item {number}");
                let outer = paragraph_showing(tester, &outer_text);
                assert_eq!(
                    paragraph_selection(tester, outer),
                    (1, outer_text.chars().count() as i32)
                );
            }
        });
    }

    // selectable_region_test.dart 'mouse can select multiple widgets on triple click drag when
    // selecting inside a WidgetSpan': a paragraph extension drag selects every line it crosses,
    // and the range counts each inner text's content.
    #[test]
    fn a_paragraph_extension_selects_the_lines_it_crosses() {
        test_widgets(|tester| {
            let texts: Vec<WidgetRef> = (0..5)
                .map(|index| {
                    Text::new(format!("Text widget {}.", (b'A' + index) as char)).into_widget()
                })
                .collect();
            let span = WidgetSpan::new(Column::new().children(texts)).into_span();
            let delegate = pump_selectable_texts(tester, vec![Text::rich(span).into_widget()]);
            let paragraph_c = paragraph_showing(tester, "Text widget C.");
            let paragraph_e = paragraph_showing(tester, "Text widget E.");
            let point_c = position_of(tester, paragraph_c, 2);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::SelectParagraph {
                    global_position: point_c,
                    absorb: false,
                },
            );
            assert_eq!(paragraph_selection(tester, paragraph_c), (0, 14));

            let point_e = position_of(tester, paragraph_e, 5);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(point_e, Some(waft_rendering::TextGranularity::Paragraph)),
            );
            assert_eq!(paragraph_selection(tester, paragraph_c), (0, 14));
            assert_eq!(
                paragraph_selection(tester, paragraph_showing(tester, "Text widget D.")),
                (0, 14)
            );
            assert_eq!(paragraph_selection(tester, paragraph_e), (0, 14));
            // A, B, C, D and E are 14 characters each; C through E is 28..70.
            assert_selection(
                tester,
                delegate.handle(),
                Some((28, 70)),
                SelectionStatus::Uncollapsed,
            );
        });
    }
}
