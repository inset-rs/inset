//! Flutter counterpart: `cupertino/list_section.dart`.
//!

use std::fmt;

use inset_embedder::{Clip, Color, FontWeight, Radius};
use inset_foundation::App;
use inset_painting::{
    AlignmentDirectional, AnyColor, BorderRadius, BorderSide, BoxDecoration, EdgeInsetsGeometry,
    RoundedSuperellipseBorder, ShapeDecoration, TextStyle,
};
use inset_widgets::{
    Align, BuildContext, ClipRSuperellipse, Column, Container, DecoratedBox, DefaultTextStyle,
    IntoWidget, KeyRef, MediaQuery, Padding, SizedBox, StatelessWidget, WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::theme::CupertinoTheme;

// Margin on top of the list section. This was eyeballed from iOS 14.4 Simulator
// and should be always present on top of the edge-to-edge variant.
const K_MARGIN_TOP: f64 = 22.0;

// Standard header margin, determined from SwiftUI's Forms in iOS 14.2 SDK.
const K_DEFAULT_HEADER_MARGIN: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(20.0, 0.0, 20.0, 6.0);

// Header margin for inset grouped variant, determined from iOS 14.4 Simulator.
const K_INSET_GROUPED_DEFAULT_HEADER_MARGIN: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(20.0, 16.0, 20.0, 6.0);

// Standard footer margin, determined from SwiftUI's Forms in iOS 14.2 SDK.
const K_DEFAULT_FOOTER_MARGIN: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(20.0, 0.0, 20.0, 0.0);

// Footer margin for inset grouped variant, determined from iOS 14.4 Simulator.
const K_INSET_GROUPED_DEFAULT_FOOTER_MARGIN: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(20.0, 0.0, 20.0, 10.0);

// Margin around children in edge-to-edge variant, determined from iOS 14.4
// Simulator.
const K_DEFAULT_ROWS_MARGIN: EdgeInsetsGeometry = EdgeInsetsGeometry::only(0.0, 0.0, 0.0, 8.0);

// Used for iOS "Inset Grouped" margin, determined from SwiftUI's Forms in
// iOS 14.2 SDK.
const K_DEFAULT_INSET_GROUPED_ROWS_MARGIN: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(20.0, 20.0, 20.0, 10.0);

// Used for iOS "Inset Grouped" margin, determined from SwiftUI's Forms in
// iOS 14.2 SDK.
const K_DEFAULT_INSET_GROUPED_ROWS_MARGIN_WITH_HEADER: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(20.0, 0.0, 20.0, 10.0);

// Used for iOS "Inset Grouped" border radius, estimated from SwiftUI's Forms in
// iOS 14.2 SDK.
const K_DEFAULT_INSET_GROUPED_BORDER_RADIUS: BorderRadius =
    BorderRadius::all(Radius::circular(10.0));

// The margin of divider used in base list section. Estimated from iOS 14.4 SDK
// Settings app.
const K_BASE_DIVIDER_MARGIN: f64 = 20.0;

// Additional margin of divider used in base list section with list tiles with
// leading widgets. Estimated from iOS 14.4 SDK Settings app.
const K_BASE_ADDITIONAL_DIVIDER_MARGIN: f64 = 44.0;

// The margin of divider used in inset grouped version of list section.
// Estimated from iOS 14.4 SDK Reminders app.
const K_INSET_DIVIDER_MARGIN: f64 = 14.0;

// Additional margin of divider used in inset grouped version of list section.
// Estimated from iOS 14.4 SDK Reminders app.
const K_INSET_ADDITIONAL_DIVIDER_MARGIN: f64 = 42.0;

// Additional margin of divider used in inset grouped version of list section
// when there is no leading widgets. Estimated from iOS 14.4 SDK Notes app.
const K_INSET_ADDITIONAL_DIVIDER_MARGIN_WITHOUT_LEADING: f64 = 14.0;

// Color of header and footer text in edge-to-edge variant.
const K_HEADER_FOOTER_COLOR_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
    Color::from_rgbo(108, 108, 108, 1.0),
    Color::from_rgbo(142, 142, 146, 1.0),
    Color::from_rgbo(74, 74, 77, 1.0),
    Color::from_rgbo(176, 176, 183, 1.0),
    Color::from_rgbo(108, 108, 108, 1.0),
    Color::from_rgbo(142, 142, 146, 1.0),
    Color::from_rgbo(108, 108, 108, 1.0),
    Color::from_rgbo(142, 142, 146, 1.0),
);
const K_HEADER_FOOTER_COLOR: AnyColor = K_HEADER_FOOTER_COLOR_DYNAMIC.to_any();

/// Denotes what type of the list section a [`CupertinoListSection`] is.
///
/// This is for internal use only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CupertinoListSectionType {
    /// A basic form of [`CupertinoListSection`].
    Base,
    /// An inset-grouped style of [`CupertinoListSection`].
    InsetGrouped,
}

/// An iOS-style list section.
///
/// The [`CupertinoListSection`] is a container for children widgets. These are
/// most often [`CupertinoListTile`](crate::CupertinoListTile)s.
///
/// [`new`](Self::new) constructs an edge-to-edge style section which includes an iOS-style
/// header, the dividers between rows, and borders on top and bottom of the rows. An example
/// of such list section are sections in iOS Settings app.
///
/// [`inset_grouped`](Self::inset_grouped) creates a round-edged and padded section that is
/// seen in iOS Notes and Reminders apps. It creates an iOS-style header, and the dividers
/// between rows. Does not create borders on top and bottom of the rows.
///
/// The section [`header`](Self::header) lies above the [`children`](Self::children) rows,
/// with margins and style that match the iOS style.
///
/// The section [`footer`](Self::footer) lies below the [`children`](Self::children) rows and
/// is used to provide additional information for current list section.
///
/// The [`children`](Self::children) is the list of widgets to be displayed in this list
/// section. Typically, the children are of type
/// [`CupertinoListTile`](crate::CupertinoListTile), however these is not enforced.
///
/// The [`margin`](Self::margin) is used to provide spacing around the content area of the
/// section encapsulating [`children`](Self::children).
///
/// The [`decoration`](Self::decoration) of [`children`](Self::children) specifies how they
/// should be decorated. If it is not provided, the background color of
/// [`children`](Self::children) defaults to
/// [`CupertinoColors::SECONDARY_SYSTEM_GROUPED_BACKGROUND`] and border radius of children
/// group defaults to 10.0 circular radius when constructing with
/// [`inset_grouped`](Self::inset_grouped). Defaults to zero radius for [`new`](Self::new).
///
/// The [`divider_margin`](Self::divider_margin) and
/// [`additional_divider_margin`](Self::additional_divider_margin) specify the starting margin
/// of the divider between list tiles. The [`divider_margin`](Self::divider_margin) is always
/// present, but [`additional_divider_margin`](Self::additional_divider_margin) is only added
/// to the [`divider_margin`](Self::divider_margin) if [`has_leading`](Self::has_leading) is
/// set to true, which is the default value.
///
/// The [`background_color`](Self::background_color) of the section defaults to
/// [`CupertinoColors::SYSTEM_GROUPED_BACKGROUND`].
///
/// See also:
///
///  * [`CupertinoListTile`](crate::CupertinoListTile), an iOS-style list tile, a typical
///    child of [`CupertinoListSection`].
///  * [`CupertinoFormSection`](crate::CupertinoFormSection), an iOS-style form section.
pub struct CupertinoListSection {
    pub key: Option<KeyRef>,
    /// The type of list section, either base or inset grouped.
    ///
    /// This member is public for testing purposes only and cannot be set
    /// manually. Instead, use a corresponding constructors.
    pub r#type: CupertinoListSectionType,
    /// Sets the form section header. The section header lies above the
    /// [`children`](Self::children) rows. Usually a `Text` widget.
    pub header: Option<WidgetRef>,
    /// Sets the form section footer. The section footer lies below the
    /// [`children`](Self::children) rows. Usually a `Text` widget.
    pub footer: Option<WidgetRef>,
    /// Margin around the content area of the section encapsulating
    /// [`children`](Self::children).
    ///
    /// Defaults to zero padding if constructed with [`new`](Self::new). Defaults to the
    /// standard notched-style iOS margin when constructing with
    /// [`inset_grouped`](Self::inset_grouped).
    pub margin: EdgeInsetsGeometry,
    /// The list of rows in the section. Usually a list of
    /// [`CupertinoListTile`](crate::CupertinoListTile)s.
    ///
    /// This takes a list, as opposed to a more efficient builder function that
    /// lazy builds, because such lists are intended to be short in row count.
    pub children: Option<Vec<WidgetRef>>,
    /// Sets the decoration around [`children`](Self::children).
    ///
    /// If `None`, background color defaults to
    /// [`CupertinoColors::SECONDARY_SYSTEM_GROUPED_BACKGROUND`].
    ///
    /// If `None`, border radius defaults to 10.0 circular radius when constructing with
    /// [`inset_grouped`](Self::inset_grouped). Defaults to zero radius for
    /// [`new`](Self::new).
    pub decoration: Option<BoxDecoration>,
    /// Sets the background color behind the section.
    ///
    /// Defaults to [`CupertinoColors::SYSTEM_GROUPED_BACKGROUND`].
    pub background_color: AnyColor,
    /// Whether and how to clip the rows.
    ///
    /// Defaults to [`Clip::HardEdge`] for [`inset_grouped`](Self::inset_grouped) and
    /// [`Clip::None`] for [`new`](Self::new).
    pub clip_behavior: Clip,
    /// The starting offset of a margin between two list tiles.
    pub divider_margin: f64,
    /// Additional starting inset of the divider used between rows. This is used
    /// when adding a leading icon to children and a divider should start at the
    /// text inset instead of the icon.
    pub additional_divider_margin: f64,
    /// Margin above the list section. Only used in edge-to-edge variant and it
    /// matches iOS style by default.
    pub top_margin: Option<f64>,
    /// Sets the color for the dividers between rows, and borders on top and
    /// bottom of the rows.
    ///
    /// If `None`, defaults to [`CupertinoColors::SEPARATOR`].
    pub separator_color: Option<AnyColor>,
    /// Whether [`margin`](Self::margin) still holds the value the constructor picked, so
    /// that [`header`](Self::header) may re-pick it as Dart's initializer list does.
    margin_is_default: bool,
    /// Whether [`additional_divider_margin`](Self::additional_divider_margin) still holds
    /// the value the constructor picked, so that [`has_leading`](Self::has_leading) may
    /// re-pick it as Dart's initializer list does.
    additional_divider_margin_is_default: bool,
}

impl CupertinoListSection {
    /// Creates a section that mimics standard iOS forms.
    ///
    /// This constructs an edge-to-edge style section which includes an iOS-style header, the
    /// dividers between rows, and borders on top and bottom of the rows. An example of such
    /// list section are sections in iOS Settings app.
    pub fn new() -> CupertinoListSection {
        CupertinoListSection {
            key: None,
            r#type: CupertinoListSectionType::Base,
            header: None,
            footer: None,
            margin: K_DEFAULT_ROWS_MARGIN,
            children: None,
            decoration: None,
            background_color: CupertinoColors::SYSTEM_GROUPED_BACKGROUND,
            clip_behavior: Clip::None,
            divider_margin: K_BASE_DIVIDER_MARGIN,
            additional_divider_margin: K_BASE_ADDITIONAL_DIVIDER_MARGIN,
            top_margin: Some(K_MARGIN_TOP),
            separator_color: None,
            margin_is_default: true,
            additional_divider_margin_is_default: true,
        }
    }

    /// Creates a section that mimics standard "Inset Grouped" iOS list section.
    ///
    /// This creates a round-edged and padded section that is seen in iOS Notes and Reminders
    /// apps. It creates an iOS-style header, and the dividers between rows. Does not create
    /// borders on top and bottom of the rows.
    pub fn inset_grouped() -> CupertinoListSection {
        CupertinoListSection {
            r#type: CupertinoListSectionType::InsetGrouped,
            margin: K_DEFAULT_INSET_GROUPED_ROWS_MARGIN,
            clip_behavior: Clip::HardEdge,
            divider_margin: K_INSET_DIVIDER_MARGIN,
            additional_divider_margin: K_INSET_ADDITIONAL_DIVIDER_MARGIN,
            top_margin: None,
            ..CupertinoListSection::new()
        }
    }

    /// Dart `CupertinoListSection(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoListSection {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoListSection(children:)`.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = WidgetRef>,
    ) -> CupertinoListSection {
        self.children = Some(children.into_iter().collect());
        self
    }

    /// Dart `CupertinoListSection(header:)`; an inset-grouped section that still has its
    /// default margin takes the with-header one, as Dart's initializer list does.
    pub fn header<K>(mut self, header: impl IntoWidget<K>) -> CupertinoListSection {
        self.header = Some(header.into_widget());
        if self.margin_is_default && self.r#type == CupertinoListSectionType::InsetGrouped {
            self.margin = K_DEFAULT_INSET_GROUPED_ROWS_MARGIN_WITH_HEADER;
        }
        self
    }

    /// Dart `CupertinoListSection(footer:)`.
    pub fn footer<K>(mut self, footer: impl IntoWidget<K>) -> CupertinoListSection {
        self.footer = Some(footer.into_widget());
        self
    }

    /// Dart `CupertinoListSection(margin:)`.
    pub fn margin(mut self, margin: EdgeInsetsGeometry) -> CupertinoListSection {
        self.margin = margin;
        self.margin_is_default = false;
        self
    }

    /// Dart `CupertinoListSection(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: impl Into<AnyColor>,
    ) -> CupertinoListSection {
        self.background_color = background_color.into();
        self
    }

    /// Dart `CupertinoListSection(decoration:)`.
    pub fn decoration(mut self, decoration: BoxDecoration) -> CupertinoListSection {
        self.decoration = Some(decoration);
        self
    }

    /// Dart `CupertinoListSection(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> CupertinoListSection {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `CupertinoListSection(dividerMargin:)`.
    pub fn divider_margin(mut self, divider_margin: f64) -> CupertinoListSection {
        self.divider_margin = divider_margin;
        self
    }

    /// Dart `CupertinoListSection(additionalDividerMargin:)`.
    pub fn additional_divider_margin(
        mut self,
        additional_divider_margin: f64,
    ) -> CupertinoListSection {
        self.additional_divider_margin = additional_divider_margin;
        self.additional_divider_margin_is_default = false;
        self
    }

    /// Dart `CupertinoListSection(topMargin:)`.
    pub fn top_margin(mut self, top_margin: f64) -> CupertinoListSection {
        self.top_margin = Some(top_margin);
        self
    }

    /// Dart `CupertinoListSection(hasLeading:)`: whether children
    /// [`CupertinoListTile`](crate::CupertinoListTile) widgets contain leading or not. Used
    /// for calculating the correct starting margin for the divider between rows, so it
    /// re-picks [`additional_divider_margin`](Self::additional_divider_margin) while that is
    /// still the constructor's default.
    pub fn has_leading(mut self, has_leading: bool) -> CupertinoListSection {
        if self.additional_divider_margin_is_default {
            self.additional_divider_margin = match (self.r#type, has_leading) {
                (CupertinoListSectionType::Base, true) => K_BASE_ADDITIONAL_DIVIDER_MARGIN,
                (CupertinoListSectionType::Base, false) => 0.0,
                (CupertinoListSectionType::InsetGrouped, true) => K_INSET_ADDITIONAL_DIVIDER_MARGIN,
                (CupertinoListSectionType::InsetGrouped, false) => {
                    K_INSET_ADDITIONAL_DIVIDER_MARGIN_WITHOUT_LEADING
                }
            };
        }
        self
    }

    /// Dart `CupertinoListSection(separatorColor:)`.
    pub fn separator_color(mut self, separator_color: impl Into<AnyColor>) -> CupertinoListSection {
        self.separator_color = Some(separator_color.into());
        self
    }

    fn header_footer_style(&self, app: &mut App, context: BuildContext) -> TextStyle {
        let style = CupertinoTheme::of(app, context).text_theme().text_style();
        match self.r#type {
            CupertinoListSectionType::Base => {
                style.merge(Some(&TextStyle::new().font_size(13.0).color(
                    CupertinoDynamicColor::resolve(&K_HEADER_FOOTER_COLOR, app, context),
                )))
            }
            CupertinoListSectionType::InsetGrouped => style,
        }
    }
}

impl Default for CupertinoListSection {
    fn default() -> CupertinoListSection {
        CupertinoListSection::new()
    }
}

impl fmt::Debug for CupertinoListSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoListSection")
            .field("type", &self.r#type)
            .field("header", &self.header)
            .field("footer", &self.footer)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for CupertinoListSection {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        // Dart's constructor assert; the fluent setters fill both operands, so it holds here.
        debug_assert!(
            self.children
                .as_ref()
                .is_some_and(|children| !children.is_empty())
                || self.header.is_some()
        );

        let divider_color = match &self.separator_color {
            Some(separator_color) => separator_color.clone(),
            None => CupertinoDynamicColor::resolve(&CupertinoColors::SEPARATOR, app, context),
        };
        let divider_height = 1.0 / MediaQuery::device_pixel_ratio_of(app, context);

        // Long divider is used for wrapping the top and bottom of rows.
        // Only used in CupertinoListSectionType::Base mode.
        let long_divider: WidgetRef = Container::new()
            .color(divider_color.clone())
            .height(divider_height)
            .into_widget();

        // Short divider is used between rows.
        let short_divider: WidgetRef = Container::new()
            .margin(EdgeInsetsGeometry::directional(
                self.divider_margin + self.additional_divider_margin,
                0.0,
                0.0,
                0.0,
            ))
            .color(divider_color)
            .height(divider_height)
            .into_widget();

        let style = self.header_footer_style(app, context);
        let header_widget = self.header.as_ref().map(|header| match self.r#type {
            CupertinoListSectionType::Base => {
                DefaultTextStyle::new(style.clone(), header.clone()).into_widget()
            }
            CupertinoListSectionType::InsetGrouped => DefaultTextStyle::new(
                style.merge(Some(
                    &TextStyle::new()
                        .font_size(20.0)
                        .font_weight(FontWeight::BOLD),
                )),
                header.clone(),
            )
            .into_widget(),
        });
        let footer_widget = self
            .footer
            .as_ref()
            .map(|footer| DefaultTextStyle::new(style.clone(), footer.clone()).into_widget());

        let mut decorated_children_group: Option<WidgetRef> = None;
        if let Some(children) = &self.children
            && !children.is_empty()
        {
            // We construct children_with_dividers as follows:
            // Insert a short divider between all rows.
            // If it is a `CupertinoListSectionType::Base` type, add a long divider
            // to the top and bottom of the rows.
            let mut children_with_dividers: Vec<WidgetRef> = Vec::new();

            if self.r#type == CupertinoListSectionType::Base {
                children_with_dividers.push(long_divider.clone());
            }

            for child in &children[..children.len() - 1] {
                children_with_dividers.push(child.clone());
                children_with_dividers.push(short_divider.clone());
            }

            children_with_dividers.push(children[children.len() - 1].clone());
            if self.r#type == CupertinoListSectionType::Base {
                children_with_dividers.push(long_divider);
            }

            let children_group_border_radius = match self.r#type {
                CupertinoListSectionType::InsetGrouped => K_DEFAULT_INSET_GROUPED_BORDER_RADIUS,
                CupertinoListSectionType::Base => BorderRadius::ZERO,
            };

            let rows = Column::new().children(children_with_dividers);
            let group: WidgetRef = match &self.decoration {
                Some(decoration) => DecoratedBox::new(decoration.clone())
                    .child(rows)
                    .into_widget(),
                None => DecoratedBox::new(
                    ShapeDecoration::new(RoundedSuperellipseBorder::new(
                        BorderSide::NONE,
                        Some(children_group_border_radius.into()),
                    ))
                    .color(CupertinoDynamicColor::resolve(
                        &CupertinoColors::SECONDARY_SYSTEM_GROUPED_BACKGROUND,
                        app,
                        context,
                    )),
                )
                .child(rows)
                .into_widget(),
            };

            let clipped: WidgetRef = if self.clip_behavior == Clip::None {
                group
            } else {
                ClipRSuperellipse::new()
                    .border_radius(children_group_border_radius.into())
                    .clip_behavior(self.clip_behavior)
                    .child(group)
                    .into_widget()
            };
            decorated_children_group = Some(Padding::new(self.margin).child(clipped).into_widget());
        }

        let mut column_children: Vec<WidgetRef> = Vec::new();
        if self.r#type == CupertinoListSectionType::Base {
            let mut top_margin = SizedBox::new();
            if let Some(height) = self.top_margin {
                top_margin = top_margin.height(height);
            }
            column_children.push(top_margin.into_widget());
        }
        if let Some(header_widget) = header_widget {
            column_children.push(
                Align::new()
                    .alignment(AlignmentDirectional::CENTER_START.into())
                    .child(
                        Padding::new(match self.r#type {
                            CupertinoListSectionType::Base => K_DEFAULT_HEADER_MARGIN,
                            CupertinoListSectionType::InsetGrouped => {
                                K_INSET_GROUPED_DEFAULT_HEADER_MARGIN
                            }
                        })
                        .child(header_widget),
                    )
                    .into_widget(),
            );
        }
        if let Some(decorated_children_group) = decorated_children_group {
            column_children.push(decorated_children_group);
        }
        if let Some(footer_widget) = footer_widget {
            column_children.push(
                Align::new()
                    .alignment(AlignmentDirectional::CENTER_START.into())
                    .child(
                        Padding::new(match self.r#type {
                            CupertinoListSectionType::Base => K_DEFAULT_FOOTER_MARGIN,
                            CupertinoListSectionType::InsetGrouped => {
                                K_INSET_GROUPED_DEFAULT_FOOTER_MARGIN
                            }
                        })
                        .child(footer_widget),
                    )
                    .into_widget(),
            );
        }

        DecoratedBox::new(BoxDecoration::new().color(CupertinoDynamicColor::resolve(
            &self.background_color,
            app,
            context,
        )))
        .child(Column::new().children(column_children))
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;

    use inset_embedder::{Offset, TextDirection};
    use inset_rendering::{
        AnyRenderObject, RenderBox, RenderClipRSuperellipse, RenderCustomClip, RenderHandle,
        RenderObject,
    };
    use inset_widgets::{Builder, Directionality, GlobalKey};

    use super::*;
    use crate::test_support::{build, test_cell};

    fn key_of(key: &GlobalKey) -> KeyRef {
        Rc::new(key.clone())
    }

    /// A `SizedBox` of `width` x `height`, findable through `key`.
    fn slot(key: &GlobalKey, width: f64, height: f64) -> SizedBox {
        SizedBox::new().key(key_of(key)).width(width).height(height)
    }

    fn offset_of(app: &mut App, key: &GlobalKey) -> Offset {
        key.current_context(app)
            .expect("the slot mounted")
            .find_render_object(app)
            .expect("the slot has been laid out")
            .as_box()
            .expect("a box")
            .local_to_global(app, Offset::ZERO, None)
    }

    /// The first descendant render object of type `T`, from `node` down.
    fn find<T: RenderObject>(app: &App, node: AnyRenderObject) -> Option<RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find::<T>(app, child);
            }
        });
        found
    }

    fn root(app: &mut App, key: &GlobalKey) -> AnyRenderObject {
        key.current_context(app)
            .expect("the section mounted")
            .find_render_object(app)
            .expect("the section has been laid out")
    }

    fn mount<K>(cell: &AppCell, section: impl IntoWidget<K>) {
        build(
            cell,
            Directionality::new(TextDirection::Ltr, section).into_widget(),
        );
    }

    /// Records the text style in force where the section places a slot.
    fn probe(seen: &Rc<RefCell<Vec<TextStyle>>>) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            seen.borrow_mut()
                .push(DefaultTextStyle::of(app, context).style);
            SizedBox::shrink().into_widget()
        })
    }

    #[test]
    fn a_base_section_stacks_its_top_margin_header_dividers_and_footer() {
        let cell = test_cell();
        let (section_key, header, first, second, footer) = (
            GlobalKey::new(),
            GlobalKey::new(),
            GlobalKey::new(),
            GlobalKey::new(),
            GlobalKey::new(),
        );
        mount(
            &cell,
            CupertinoListSection::new()
                .key(key_of(&section_key))
                .header(slot(&header, 50.0, 10.0))
                .footer(slot(&footer, 50.0, 10.0))
                .children([
                    slot(&first, 50.0, 40.0).into_widget(),
                    slot(&second, 50.0, 40.0).into_widget(),
                ]),
        );
        let mut app = cell.borrow_mut();

        assert_eq!(
            offset_of(&mut app, &header),
            Offset::new(20.0, 22.0),
            "the top margin, then the header's start margin"
        );
        assert_eq!(
            offset_of(&mut app, &first),
            Offset::new(175.0, 38.5),
            "the header margin's 6 below it, then the long divider at a 2x device pixel"
        );
        assert_eq!(
            offset_of(&mut app, &second).dy(),
            79.0,
            "the first row and the short divider between them"
        );
        assert_eq!(
            offset_of(&mut app, &footer),
            Offset::new(20.0, 127.5),
            "the closing long divider and the rows' bottom margin"
        );

        let section = root(&mut app, &section_key);
        assert!(
            find::<RenderClipRSuperellipse>(&app, section).is_none(),
            "the base section does not clip"
        );
    }

    #[test]
    fn an_inset_grouped_section_clips_its_rows_and_drops_the_long_dividers() {
        let cell = test_cell();
        let (section_key, first, second) = (GlobalKey::new(), GlobalKey::new(), GlobalKey::new());
        mount(
            &cell,
            CupertinoListSection::inset_grouped()
                .key(key_of(&section_key))
                .children([
                    slot(&first, 50.0, 40.0).into_widget(),
                    slot(&second, 50.0, 40.0).into_widget(),
                ]),
        );
        let mut app = cell.borrow_mut();

        assert_eq!(
            offset_of(&mut app, &first).dy(),
            20.0,
            "no top margin and no long divider, only the rows' own margin"
        );
        assert_eq!(
            offset_of(&mut app, &second).dy(),
            60.5,
            "one short divider between the rows"
        );

        let section = root(&mut app, &section_key);
        let clip = find::<RenderClipRSuperellipse>(&app, section).expect("the rows are clipped");
        assert_eq!(clip.clip_behavior(&app), Clip::HardEdge);
        assert_eq!(clip.size(&app).width(), 360.0, "inset by the rows' margin");
    }

    #[test]
    fn a_base_section_styles_its_header_and_footer_in_the_header_footer_color() {
        let cell = test_cell();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let resolved = Rc::new(RefCell::new(None));
        mount(
            &cell,
            CupertinoListSection::new()
                .header(probe(&seen))
                .footer(probe(&seen))
                .children([Builder::new({
                    let resolved = Rc::clone(&resolved);
                    move |app, context| {
                        *resolved.borrow_mut() = Some(CupertinoDynamicColor::resolve(
                            &K_HEADER_FOOTER_COLOR,
                            app,
                            context,
                        ));
                        SizedBox::shrink().into_widget()
                    }
                })
                .into_widget()]),
        );

        let seen = seen.borrow();
        assert_eq!(seen.len(), 2, "header and footer");
        for style in seen.iter() {
            assert_eq!(style.font_size, Some(13.0));
            assert_eq!(style.color, resolved.borrow().clone());
        }
    }

    #[test]
    fn an_inset_grouped_header_is_bold_and_re_picks_the_rows_margin() {
        let cell = test_cell();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let with_header = CupertinoListSection::inset_grouped().header(probe(&seen));
        assert_eq!(
            with_header.margin,
            K_DEFAULT_INSET_GROUPED_ROWS_MARGIN_WITH_HEADER
        );
        assert_eq!(
            CupertinoListSection::inset_grouped().margin,
            K_DEFAULT_INSET_GROUPED_ROWS_MARGIN,
            "a section without a header keeps the plain margin"
        );

        mount(
            &cell,
            with_header
                .footer(probe(&seen))
                .children([SizedBox::new().width(50.0).height(40.0).into_widget()]),
        );

        let seen = seen.borrow();
        let (header, footer) = (&seen[0], &seen[1]);
        assert_eq!(header.font_size, Some(20.0));
        assert_eq!(header.font_weight, Some(FontWeight::BOLD));
        assert_eq!(
            footer.font_size,
            Some(17.0),
            "the footer keeps the theme's text style"
        );
    }

    #[test]
    fn has_leading_re_picks_the_additional_divider_margin() {
        assert_eq!(
            CupertinoListSection::new().additional_divider_margin,
            K_BASE_ADDITIONAL_DIVIDER_MARGIN
        );
        assert_eq!(
            CupertinoListSection::new()
                .has_leading(false)
                .additional_divider_margin,
            0.0
        );
        assert_eq!(
            CupertinoListSection::inset_grouped()
                .has_leading(false)
                .additional_divider_margin,
            K_INSET_ADDITIONAL_DIVIDER_MARGIN_WITHOUT_LEADING
        );
        assert_eq!(
            CupertinoListSection::inset_grouped()
                .additional_divider_margin(3.0)
                .has_leading(false)
                .additional_divider_margin,
            3.0,
            "an explicit margin wins over the leading flag, whatever the order"
        );
    }
}
