//! The getters a Dart theme-data class declares and its `_XDefaultsM3(context)` subclasses
//! override, as a trait (`.agents/skills/porting-flutter/patterns/lazy-defaults.md`).
//!
//! A theme-data struct answers each getter with its field; a defaults struct answers with the
//! fields of the `super(...)` call it keeps, except where it computes a value, and computes it
//! only when asked. A caller that reads the defaults goes through the trait and reads only
//! what Dart's `widget ?? theme ?? defaults` reaches.

use std::cell::OnceCell;
use std::rc::Rc;

use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::color_scheme::ColorScheme;
use crate::text_theme::TextTheme;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

/// Declares `pub trait $getters` with one method per field of `$data`, each answering a clone
/// of the field of the struct `$fields` returns, and implements it for `$data` itself.
macro_rules! theme_data_getters {
    (
        $(#[$meta:meta])*
        pub trait $getters:ident for $data:ident, fields: $fields:ident {
            $( $field:ident : $ty:ty ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        pub trait $getters {
            /// The fields Dart's constructor sets: the struct itself, or the `super(...)`
            /// arguments a defaults struct keeps.
            fn $fields(&self) -> &$data;

            $(
                #[doc = concat!("Dart's `", stringify!($field), "` getter.")]
                #[allow(deprecated)]
                fn $field(&self, app: &mut ::waft_foundation::App) -> $ty {
                    let _ = app;
                    self.$fields().$field.clone()
                }
            )*

            /// Dart's `copyWith()` with no arguments: a plain value holding what every getter
            /// answers, for the caller's setters to change.
            #[allow(deprecated)]
            fn copy_with(&self, app: &mut ::waft_foundation::App) -> $data {
                $data {
                    $( $field: self.$field(app), )*
                }
            }
        }

        impl $getters for $data {
            fn $fields(&self) -> &$data {
                self
            }
        }
    };
}

pub(crate) use theme_data_getters;

/// The context a defaults class keeps, with the `late final` theme fields Dart's generated
/// defaults declare: `_theme = Theme.of(context)`, `_colors = _theme.colorScheme` and
/// `_textTheme = _theme.textTheme`, each looked up on first use.
pub(crate) struct DefaultsContext {
    context: BuildContext,
    theme: OnceCell<Rc<ThemeData>>,
}

impl DefaultsContext {
    pub(crate) fn new(context: BuildContext) -> DefaultsContext {
        DefaultsContext {
            context,
            theme: OnceCell::new(),
        }
    }

    /// The context of a defaults class whose constructor reads the theme: Dart's
    /// `_theme = Theme.of(context)` in its initializer list.
    pub(crate) fn with_theme(app: &mut App, context: BuildContext) -> DefaultsContext {
        let defaults_context = DefaultsContext::new(context);
        defaults_context.theme(app);
        defaults_context
    }

    /// Dart's `context`.
    pub(crate) fn context(&self) -> BuildContext {
        self.context
    }

    /// Dart's `_theme`.
    pub(crate) fn theme(&self, app: &mut App) -> &ThemeData {
        self.theme.get_or_init(|| Theme::of(app, self.context))
    }

    /// Dart's `_colors`.
    pub(crate) fn colors(&self, app: &mut App) -> &ColorScheme {
        &self.theme(app).color_scheme
    }

    /// Dart's `_textTheme`.
    pub(crate) fn text_theme(&self, app: &mut App) -> &TextTheme {
        &self.theme(app).text_theme
    }
}
