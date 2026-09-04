//! Flutter counterpart: `cupertino/icon_theme_data.dart`.

use std::rc::Rc;

use reveal_foundation::App;
use reveal_widgets::{BuildContext, IconThemeData};

use crate::colors::CupertinoDynamicColor;

/// An `IconThemeData` subclass that automatically resolves its `color` when retrieved using
/// `IconTheme.of`.
///
/// Dart's `class CupertinoIconThemeData extends IconThemeData` overrides one method; here
/// [`new`](Self::new) returns an [`IconThemeData`] whose `resolve` override is
/// [`CupertinoIconThemeData::resolve`], and `copy_with` keeps it, as the subclass's
/// `copyWith` does.
pub struct CupertinoIconThemeData;

impl CupertinoIconThemeData {
    /// Creates a [`CupertinoIconThemeData`].
    ///
    /// Dart's named arguments are the fluent setters on the result
    /// (`CupertinoIconThemeData::new().color(..).size(..)`).
    #[expect(
        clippy::new_ret_no_self,
        reason = "Dart's subclass constructor; the value is the base type with the override set"
    )]
    pub fn new() -> IconThemeData {
        IconThemeData::new().resolver(Rc::new(CupertinoIconThemeData::resolve))
    }

    /// Called by `IconTheme.of` to resolve `color` against the given `context`.
    pub fn resolve(data: &IconThemeData, app: &mut App, context: BuildContext) -> IconThemeData {
        let resolved_color =
            CupertinoDynamicColor::maybe_resolve(data.color.as_ref(), app, context);
        if resolved_color == data.color {
            return data.clone();
        }
        let copy = data.copy_with();
        match resolved_color {
            Some(color) => copy.color(color),
            None => copy,
        }
    }
}
