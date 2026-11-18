//! Flutter counterpart: `cupertino/form_section.dart`.

use waft_embedder::Clip;
use waft_foundation::App;
use waft_painting::{AnyColor, BoxDecoration, EdgeInsetsGeometry, TextStyle};
use waft_widgets::{
    BuildContext, DefaultTextStyle, IntoWidget, KeyRef, StatelessWidget, WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::list_section::{CupertinoListSection, CupertinoListSectionType};

// Used for iOS "Inset Grouped" margin, determined from SwiftUI's Forms in
// iOS 14.2 SDK.
const K_FORM_DEFAULT_INSET_GROUPED_ROWS_MARGIN: EdgeInsetsGeometry =
    EdgeInsetsGeometry::from_steb(20.0, 0.0, 20.0, 10.0);

/// An iOS-style form section.
///
/// [`new`](Self::new) constructs an edge-to-edge style section which includes an iOS-style
/// header, rows, the dividers between rows, and borders on top and bottom of the rows.
///
/// [`inset_grouped`](Self::inset_grouped) creates a round-edged and padded section that is
/// commonly seen in notched-displays like iPhone X and beyond. Creates an iOS-style header,
/// rows, and the dividers between rows. Does not create borders on top and bottom of the
/// rows.
///
/// The [`header`](Self::header) lies above the [`children`](Self::children) rows, with
/// margins that match the iOS style.
///
/// The [`footer`](Self::footer) lies below the [`children`](Self::children) rows.
///
/// The [`children`](Self::children) is required and sets the list of rows shown in the
/// section. It takes a list, as opposed to a more efficient builder function that lazy
/// builds, because forms are intended to be short in row count. It is recommended that only
/// [`CupertinoFormRow`](crate::CupertinoFormRow) widgets be included in the
/// [`children`](Self::children) list in order to retain the iOS look.
///
/// The [`margin`](Self::margin) sets the spacing around the content area of the section
/// encapsulating [`children`](Self::children).
///
/// The [`decoration`](Self::decoration) sets the decoration around
/// [`children`](Self::children). If `None`, defaults to
/// [`CupertinoColors::SECONDARY_SYSTEM_GROUPED_BACKGROUND`], and to 10.0 circular radius when
/// constructing with [`inset_grouped`](Self::inset_grouped). Defaults to zero radius for
/// [`new`](Self::new).
///
/// The [`background_color`](Self::background_color) sets the background color behind the
/// section. Defaults to [`CupertinoColors::SYSTEM_GROUPED_BACKGROUND`].
///
/// See also:
///
///  * [`CupertinoFormRow`](crate::CupertinoFormRow), an iOS-style list tile, a typical child
///    of [`CupertinoFormSection`].
///  * [`CupertinoListSection`], an iOS-style list section.
#[derive(Debug)]
pub struct CupertinoFormSection {
    pub key: Option<KeyRef>,
    r#type: CupertinoListSectionType,
    /// Sets the form section header. The section header lies above the
    /// [`children`](Self::children) rows.
    pub header: Option<WidgetRef>,
    /// Sets the form section footer. The section footer lies below the
    /// [`children`](Self::children) rows.
    pub footer: Option<WidgetRef>,
    /// Margin around the content area of the section encapsulating
    /// [`children`](Self::children).
    ///
    /// Defaults to zero padding if constructed with [`new`](Self::new). Defaults to the
    /// standard notched-style iOS margin when constructing with
    /// [`inset_grouped`](Self::inset_grouped).
    pub margin: EdgeInsetsGeometry,
    /// The list of rows in the section.
    pub children: Vec<WidgetRef>,
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
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,
}

impl CupertinoFormSection {
    /// Creates a section that mimics standard iOS forms.
    ///
    /// This constructs an edge-to-edge style section which includes an iOS-style header,
    /// rows, the dividers between rows, and borders on top and bottom of the rows.
    pub fn new(children: impl IntoIterator<Item = WidgetRef>) -> CupertinoFormSection {
        CupertinoFormSection::of_type(
            children.into_iter().collect(),
            CupertinoListSectionType::Base,
            EdgeInsetsGeometry::ZERO,
        )
    }

    /// Creates a section that mimics standard "Inset Grouped" iOS forms.
    ///
    /// This creates a round-edged and padded section that is commonly seen in
    /// notched-displays like iPhone X and beyond. Creates an iOS-style header, rows, and the
    /// dividers between rows. Does not create borders on top and bottom of the rows.
    pub fn inset_grouped(children: impl IntoIterator<Item = WidgetRef>) -> CupertinoFormSection {
        CupertinoFormSection::of_type(
            children.into_iter().collect(),
            CupertinoListSectionType::InsetGrouped,
            K_FORM_DEFAULT_INSET_GROUPED_ROWS_MARGIN,
        )
    }

    fn of_type(
        children: Vec<WidgetRef>,
        r#type: CupertinoListSectionType,
        margin: EdgeInsetsGeometry,
    ) -> CupertinoFormSection {
        debug_assert!(!children.is_empty());
        CupertinoFormSection {
            key: None,
            r#type,
            header: None,
            footer: None,
            margin,
            children,
            decoration: None,
            background_color: CupertinoColors::SYSTEM_GROUPED_BACKGROUND,
            clip_behavior: Clip::None,
        }
    }

    /// Dart `CupertinoFormSection(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoFormSection {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoFormSection(header:)`.
    pub fn header<K>(mut self, header: impl IntoWidget<K>) -> CupertinoFormSection {
        self.header = Some(header.into_widget());
        self
    }

    /// Dart `CupertinoFormSection(footer:)`.
    pub fn footer<K>(mut self, footer: impl IntoWidget<K>) -> CupertinoFormSection {
        self.footer = Some(footer.into_widget());
        self
    }

    /// Dart `CupertinoFormSection(margin:)`.
    pub fn margin(mut self, margin: impl Into<EdgeInsetsGeometry>) -> CupertinoFormSection {
        let margin = margin.into();
        self.margin = margin;
        self
    }

    /// Dart `CupertinoFormSection(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: impl Into<AnyColor>,
    ) -> CupertinoFormSection {
        self.background_color = background_color.into();
        self
    }

    /// Dart `CupertinoFormSection(decoration:)`.
    pub fn decoration(mut self, decoration: BoxDecoration) -> CupertinoFormSection {
        self.decoration = Some(decoration);
        self
    }

    /// Dart `CupertinoFormSection(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> CupertinoFormSection {
        self.clip_behavior = clip_behavior;
        self
    }
}

impl StatelessWidget for CupertinoFormSection {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let style = TextStyle::new()
            .font_size(13.0)
            .color(CupertinoDynamicColor::resolve(
                &CupertinoColors::SECONDARY_LABEL,
                app,
                context,
            ));
        let header_widget = self
            .header
            .as_ref()
            .map(|header| DefaultTextStyle::new(style.clone(), header.clone()));
        let footer_widget = self
            .footer
            .as_ref()
            .map(|footer| DefaultTextStyle::new(style, footer.clone()));

        let mut section = match self.r#type {
            CupertinoListSectionType::Base => CupertinoListSection::new(),
            CupertinoListSectionType::InsetGrouped => CupertinoListSection::inset_grouped(),
        };
        if let Some(header_widget) = header_widget {
            section = section.header(header_widget);
        }
        if let Some(footer_widget) = footer_widget {
            section = section.footer(footer_widget);
        }
        section = section
            .margin(self.margin)
            .background_color(self.background_color.clone())
            .clip_behavior(self.clip_behavior)
            .has_leading(false)
            .children(self.children.iter().cloned());
        if let Some(decoration) = &self.decoration {
            section = section.decoration(decoration.clone());
        }
        section.into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_embedder::TextDirection;
    use waft_rendering::RenderClipRRect;
    use waft_widgets::{Builder, Directionality, GlobalKey, SizedBox};

    use waft_widgets::testing::{WidgetTester, assert_f32_equals, test_widgets};

    use super::*;

    fn key_of(key: &GlobalKey) -> KeyRef {
        key.clone().into()
    }

    /// A `SizedBox` of `width` x `height`, findable through `key`.
    fn slot(key: &GlobalKey, width: f64, height: f64) -> SizedBox {
        SizedBox::new().key(key_of(key)).width(width).height(height)
    }

    fn mount<K>(tester: &mut WidgetTester, section: impl IntoWidget<K>) {
        tester.pump_widget(Directionality::new(TextDirection::Ltr, section));
    }

    /// Records the text style in force where the section places a slot.
    fn probe(seen: &Rc<RefCell<Vec<waft_painting::TextStyle>>>) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            seen.borrow_mut()
                .push(waft_widgets::DefaultTextStyle::of(app, context).style);
            SizedBox::shrink().into_widget()
        })
    }

    #[test]
    fn a_base_form_section_styles_its_header_and_lays_out_as_a_base_list_section() {
        test_widgets(|tester| {
            let seen = Rc::new(RefCell::new(Vec::new()));
            let secondary = Rc::new(RefCell::new(None));
            let (first, second) = (GlobalKey::new(), GlobalKey::new());
            mount(
                tester,
                CupertinoFormSection::new([
                    Builder::new({
                        let (secondary, first) = (Rc::clone(&secondary), first.clone());
                        move |app, context| {
                            *secondary.borrow_mut() = Some(CupertinoDynamicColor::resolve(
                                &CupertinoColors::SECONDARY_LABEL,
                                app,
                                context,
                            ));
                            slot(&first, 50.0, 40.0).into_widget()
                        }
                    })
                    .into_widget(),
                    slot(&second, 50.0, 40.0).into_widget(),
                ])
                .header(probe(&seen))
                .footer(probe(&seen)),
            );
            let styles = seen.borrow();
            assert_eq!(styles.len(), 2, "header and footer");
            for style in styles.iter() {
                assert_eq!(style.font_size, Some(13.0));
                assert_eq!(style.color, secondary.borrow().clone());
            }

            // The list section's top margin, the header margin, and the long divider, a device
            // pixel at 3.0.
            assert_f32_equals(tester.get_top_left(&first).dy(), 28.0 + 1.0 / 3.0);
            // The short divider between the rows.
            assert_f32_equals(tester.get_top_left(&second).dy(), 68.0 + 2.0 / 3.0);
        });
    }

    #[test]
    fn an_inset_grouped_form_section_keeps_the_form_margin_and_does_not_clip() {
        test_widgets(|tester| {
            let first = GlobalKey::new();
            let section =
                CupertinoFormSection::inset_grouped([slot(&first, 50.0, 40.0).into_widget()]);
            assert_eq!(section.margin, K_FORM_DEFAULT_INSET_GROUPED_ROWS_MARGIN);
            assert_eq!(section.clip_behavior, Clip::None);
            mount(tester, section);
            assert_eq!(
                tester.get_top_left(&first).dy(),
                0.0,
                "no top margin, and the form's own margin has none either"
            );
            assert!(
                tester.all::<RenderClipRRect>().is_empty(),
                "the form section clips nothing"
            );
        });
    }
}
