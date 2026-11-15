//! Flutter counterpart: material_ui's `action_icons_theme.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_foundation::App;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetBuilder, WidgetRef,
};

use crate::theme::Theme;

/// A [`ActionIconThemeData`] that overrides the default icons of
/// `BackButton`, `CloseButton`, `DrawerButton`, and `EndDrawerButton` with
/// [`ActionIconTheme::of`] or the overall `Theme`'s `ThemeData::action_icon_theme`.
#[derive(Clone, Default)]
pub struct ActionIconThemeData {
    /// Overrides `BackButtonIcon`'s icon.
    ///
    /// If [`back_button_icon_builder`](Self::back_button_icon_builder) is null, then
    /// `BackButtonIcon` fallbacks to the platform's default back button icon.
    pub back_button_icon_builder: Option<WidgetBuilder>,

    /// Overrides `CloseButtonIcon`'s icon.
    ///
    /// If [`close_button_icon_builder`](Self::close_button_icon_builder) is null, then
    /// `CloseButtonIcon` fallbacks to the platform's default close button icon.
    pub close_button_icon_builder: Option<WidgetBuilder>,

    /// Overrides `DrawerButtonIcon`'s icon.
    ///
    /// If [`drawer_button_icon_builder`](Self::drawer_button_icon_builder) is null, then
    /// `DrawerButtonIcon` fallbacks to the platform's default drawer button icon.
    pub drawer_button_icon_builder: Option<WidgetBuilder>,

    /// Overrides `EndDrawerButtonIcon`'s icon.
    ///
    /// If [`end_drawer_button_icon_builder`](Self::end_drawer_button_icon_builder) is null, then
    /// `EndDrawerButtonIcon` fallbacks to the platform's default end drawer button icon.
    pub end_drawer_button_icon_builder: Option<WidgetBuilder>,
}

impl ActionIconThemeData {
    /// Creates an [`ActionIconThemeData`]; Dart's optional named arguments are the fluent
    /// setters.
    ///
    /// The builders [`back_button_icon_builder`](Self::back_button_icon_builder),
    /// [`close_button_icon_builder`](Self::close_button_icon_builder),
    /// [`drawer_button_icon_builder`](Self::drawer_button_icon_builder),
    /// [`end_drawer_button_icon_builder`](Self::end_drawer_button_icon_builder) may be null.
    pub fn new() -> ActionIconThemeData {
        ActionIconThemeData::default()
    }

    /// Dart `ActionIconThemeData(backButtonIconBuilder:)`.
    pub fn back_button_icon_builder(
        mut self,
        back_button_icon_builder: WidgetBuilder,
    ) -> ActionIconThemeData {
        self.back_button_icon_builder = Some(back_button_icon_builder);
        self
    }

    /// Dart `ActionIconThemeData(closeButtonIconBuilder:)`.
    pub fn close_button_icon_builder(
        mut self,
        close_button_icon_builder: WidgetBuilder,
    ) -> ActionIconThemeData {
        self.close_button_icon_builder = Some(close_button_icon_builder);
        self
    }

    /// Dart `ActionIconThemeData(drawerButtonIconBuilder:)`.
    pub fn drawer_button_icon_builder(
        mut self,
        drawer_button_icon_builder: WidgetBuilder,
    ) -> ActionIconThemeData {
        self.drawer_button_icon_builder = Some(drawer_button_icon_builder);
        self
    }

    /// Dart `ActionIconThemeData(endDrawerButtonIconBuilder:)`.
    pub fn end_drawer_button_icon_builder(
        mut self,
        end_drawer_button_icon_builder: WidgetBuilder,
    ) -> ActionIconThemeData {
        self.end_drawer_button_icon_builder = Some(end_drawer_button_icon_builder);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().back_button_icon_builder(builder)`).
    pub fn copy_with(&self) -> ActionIconThemeData {
        self.clone()
    }

    /// Linearly interpolate between two action icon themes.
    pub fn lerp(
        a: Option<&ActionIconThemeData>,
        b: Option<&ActionIconThemeData>,
        t: f64,
    ) -> Option<ActionIconThemeData> {
        if a.is_none() && b.is_none() {
            return None;
        }
        let pick = |field: fn(&ActionIconThemeData) -> &Option<WidgetBuilder>| {
            if t < 0.5 {
                a.and_then(|a| field(a).clone())
            } else {
                b.and_then(|b| field(b).clone())
            }
        };
        Some(ActionIconThemeData {
            back_button_icon_builder: pick(|data| &data.back_button_icon_builder),
            close_button_icon_builder: pick(|data| &data.close_button_icon_builder),
            drawer_button_icon_builder: pick(|data| &data.drawer_button_icon_builder),
            end_drawer_button_icon_builder: pick(|data| &data.end_drawer_button_icon_builder),
        })
    }
}

/// Dart's `==` on a builder: two builders are equal when they are the same function object.
fn same_builder(a: &Option<WidgetBuilder>, b: &Option<WidgetBuilder>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl PartialEq for ActionIconThemeData {
    fn eq(&self, other: &ActionIconThemeData) -> bool {
        std::ptr::eq(self, other)
            || (same_builder(
                &other.back_button_icon_builder,
                &self.back_button_icon_builder,
            ) && same_builder(
                &other.close_button_icon_builder,
                &self.close_button_icon_builder,
            ) && same_builder(
                &other.drawer_button_icon_builder,
                &self.drawer_button_icon_builder,
            ) && same_builder(
                &other.end_drawer_button_icon_builder,
                &self.end_drawer_button_icon_builder,
            ))
    }
}

impl Debug for ActionIconThemeData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let present = |builder: &Option<WidgetBuilder>| builder.as_ref().map(|_| "WidgetBuilder");
        formatter
            .debug_struct("ActionIconThemeData")
            .field(
                "back_button_icon_builder",
                &present(&self.back_button_icon_builder),
            )
            .field(
                "close_button_icon_builder",
                &present(&self.close_button_icon_builder),
            )
            .field(
                "drawer_button_icon_builder",
                &present(&self.drawer_button_icon_builder),
            )
            .field(
                "end_drawer_button_icon_builder",
                &present(&self.end_drawer_button_icon_builder),
            )
            .finish()
    }
}

/// An inherited widget that overrides the default icon of `BackButtonIcon`,
/// `CloseButtonIcon`, `DrawerButtonIcon`, and `EndDrawerButtonIcon` in this
/// widget's subtree.
#[derive(Debug)]
pub struct ActionIconTheme {
    pub key: Option<KeyRef>,
    /// Specifies the default icon overrides for descendant `BackButtonIcon`,
    /// `CloseButtonIcon`, `DrawerButtonIcon`, and `EndDrawerButtonIcon` widgets.
    pub data: ActionIconThemeData,
    pub child: WidgetRef,
}

impl ActionIconTheme {
    /// Creates a theme that overrides the default icon of `BackButtonIcon`,
    /// `CloseButtonIcon`, `DrawerButtonIcon`, and `EndDrawerButtonIcon` in this
    /// widget's subtree.
    pub fn new<K>(data: ActionIconThemeData, child: impl IntoWidget<K>) -> ActionIconTheme {
        ActionIconTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ActionIconTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ActionIconTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`ActionIconThemeData`] from the closest ancestor [`ActionIconTheme`]
    /// widget.
    ///
    /// If there is no enclosing [`ActionIconTheme`] widget, then
    /// `ThemeData::action_icon_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ActionIconTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> Option<ActionIconThemeData> {
        match context.depend_on_inherited_widget_of_exact_type::<ActionIconTheme>(app) {
            Some(action_icon_theme) => Some(action_icon_theme.data.clone()),
            None => Theme::of(app, context).action_icon_theme.clone(),
        }
    }
}

impl InheritedTheme for ActionIconTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ActionIconTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ActionIconTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ActionIconTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Color, TargetPlatform};
    use waft_widgets::{Column, Icon, IconData, RichText, SizedBox};

    use super::*;
    use crate::action_buttons::{BackButton, CloseButton, DrawerButton, EndDrawerButton};
    use crate::material::Material;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    const GREEN: Color = Color::new(0xff00ff00);
    const ICON: IconData = IconData::new(0);

    fn builder(make: impl Fn() -> WidgetRef + 'static) -> WidgetBuilder {
        Rc::new(move |_app, _context| make())
    }

    fn sample_icon_builder() -> WidgetBuilder {
        builder(|| Icon::new(Some(ICON)).size(20.0).color(GREEN).into_widget())
    }

    fn sample_theme() -> ActionIconThemeData {
        let sample = sample_icon_builder();
        ActionIconThemeData::new()
            .back_button_icon_builder(Rc::clone(&sample))
            .close_button_icon_builder(Rc::clone(&sample))
            .drawer_button_icon_builder(Rc::clone(&sample))
            .end_drawer_button_icon_builder(sample)
    }

    fn all_buttons() -> WidgetRef {
        Material::new()
            .child(Column::new().children(vec![
                BackButton::new().into_widget(),
                CloseButton::new().into_widget(),
                DrawerButton::new().into_widget(),
                EndDrawerButton::new().into_widget(),
            ]))
            .into_widget()
    }

    /// The glyphs of the four buttons: each button's `Icon`, with the colour its `RichText` paints.
    fn expect_sample_icons(tester: &WidgetTester) {
        let icons = tester.find_by_type::<Icon>();
        assert_eq!(icons.len(), 4);
        for icon in icons {
            assert!(tester.widget(icon, |icon: &Icon| icon.icon == Some(ICON)));
            let style = tester.glyph_style(icon);
            assert_eq!(style.color.as_ref().map(|color| color.color()), Some(GREEN));
        }
        assert_eq!(tester.find_by_type::<RichText>().len(), 4);
    }

    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_fonts();
        tester
    }

    // 'ActionIconThemeData copyWith, ==, hashCode basics'
    #[test]
    fn an_empty_theme_equals_its_copy() {
        assert_eq!(
            ActionIconThemeData::new(),
            ActionIconThemeData::new().copy_with()
        );
    }

    // 'ActionIconThemeData copyWith overrides all properties'
    #[test]
    fn copy_with_overrides_all_properties() {
        let original_builder = builder(|| SizedBox::new().into_widget());
        let new_builder = sample_icon_builder();
        let original = ActionIconThemeData::new()
            .back_button_icon_builder(Rc::clone(&original_builder))
            .close_button_icon_builder(Rc::clone(&original_builder))
            .drawer_button_icon_builder(Rc::clone(&original_builder))
            .end_drawer_button_icon_builder(Rc::clone(&original_builder));

        // Check if the all properties are copied.
        let copy = original.copy_with();
        for copied in [
            &copy.back_button_icon_builder,
            &copy.close_button_icon_builder,
            &copy.drawer_button_icon_builder,
            &copy.end_drawer_button_icon_builder,
        ] {
            assert!(same_builder(copied, &Some(Rc::clone(&original_builder))));
        }

        // Check if the properties are overridden.
        let overridden = original
            .copy_with()
            .back_button_icon_builder(Rc::clone(&new_builder))
            .close_button_icon_builder(Rc::clone(&new_builder))
            .drawer_button_icon_builder(Rc::clone(&new_builder))
            .end_drawer_button_icon_builder(Rc::clone(&new_builder));
        for replaced in [
            &overridden.back_button_icon_builder,
            &overridden.close_button_icon_builder,
            &overridden.drawer_button_icon_builder,
            &overridden.end_drawer_button_icon_builder,
        ] {
            assert!(same_builder(replaced, &Some(Rc::clone(&new_builder))));
        }
        assert_ne!(original, overridden);
    }

    // 'ActionIconThemeData defaults'
    #[test]
    fn the_defaults_have_no_builders() {
        let theme = ActionIconThemeData::new();
        assert!(theme.back_button_icon_builder.is_none());
        assert!(theme.close_button_icon_builder.is_none());
        assert!(theme.drawer_button_icon_builder.is_none());
        assert!(theme.end_drawer_button_icon_builder.is_none());
    }

    // `ActionIconThemeData.lerp`, which Dart's tests reach through `ThemeData.lerp`.
    #[test]
    fn lerp_switches_at_the_middle() {
        let a = sample_theme();
        let b = ActionIconThemeData::new();
        assert!(ActionIconThemeData::lerp(None, None, 0.5).is_none());
        assert_eq!(
            ActionIconThemeData::lerp(Some(&a), Some(&b), 0.25),
            Some(a.clone())
        );
        assert_eq!(
            ActionIconThemeData::lerp(Some(&a), Some(&b), 0.5),
            Some(b.clone())
        );
        assert_eq!(ActionIconThemeData::lerp(Some(&a), None, 0.75), Some(b));
    }

    // 'Action buttons use ThemeData action icon theme'
    #[test]
    fn action_buttons_use_the_theme_datas_action_icon_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    ThemeData::builder()
                        .action_icon_theme(sample_theme())
                        .build(&app),
                    all_buttons(),
                )
            };
            tester.pump_widget(root);

            expect_sample_icons(&tester);
        });
    }

    // 'Action buttons use ActionIconTheme'
    #[test]
    fn action_buttons_use_the_action_icon_theme_widget() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    ThemeData::new(&app),
                    ActionIconTheme::new(sample_theme(), all_buttons()).into_widget(),
                )
            };
            tester.pump_widget(root);

            expect_sample_icons(&tester);
        });
    }

    // `ActionIconTheme.updateShouldNotify`: a theme with other builders rebuilds the buttons.
    #[test]
    fn a_changed_action_icon_theme_rebuilds_the_buttons() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let cell = Rc::clone(&tester.cell);
            let theme = |data: ActionIconThemeData| {
                let app = cell.borrow();
                material_app(
                    ThemeData::new(&app),
                    ActionIconTheme::new(data, all_buttons()).into_widget(),
                )
            };
            let root = theme(ActionIconThemeData::new());
            tester.pump_widget(root);
            assert!(
                tester
                    .find_by_type::<Icon>()
                    .into_iter()
                    .all(|icon| tester.widget(icon, |icon: &Icon| icon.icon != Some(ICON)))
            );

            let root = theme(sample_theme());
            tester.pump_widget(root);
            expect_sample_icons(&tester);
        });
    }
}
