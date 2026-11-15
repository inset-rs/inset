//! Flutter counterpart: material_ui's `bottom_sheet_theme.dart`.

use waft_embedder::{Clip, Color, Size, lerp_double};
use waft_painting::{AnyColor, ShapeBorder};
use waft_rendering::BoxConstraints;

/// Defines default property values for `BottomSheet`'s `Material`.
///
/// Descendant widgets obtain the current [`BottomSheetThemeData`] object
/// using `Theme::of(app, context).bottom_sheet_theme`. Instances of
/// [`BottomSheetThemeData`] can be customized with
/// [`BottomSheetThemeData::copy_with`].
///
/// Typically a [`BottomSheetThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::bottom_sheet_theme`.
///
/// All [`BottomSheetThemeData`] properties are `null` by default.
/// When null, the `BottomSheet` will provide its own defaults.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BottomSheetThemeData {
    /// Overrides the default value for `BottomSheet::background_color`.
    ///
    /// If null, `BottomSheet` defaults to `Material`'s default.
    pub background_color: Option<Color>,

    /// Overrides the default value for surfaceTintColor.
    ///
    /// If null, `BottomSheet` will not display an overlay color.
    ///
    /// See `Material::surface_tint_color` for more details.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value for `BottomSheet::elevation`.
    ///
    /// {@macro material_ui.material.elevation}
    ///
    /// If null, `BottomSheet` defaults to 0.0.
    pub elevation: Option<f64>,

    /// Value for `BottomSheet::background_color` when the Bottom sheet is presented
    /// as a modal bottom sheet.
    pub modal_background_color: Option<Color>,

    /// Overrides the default value for barrier color when the Bottom sheet is presented as
    /// a modal bottom sheet.
    pub modal_barrier_color: Option<Color>,

    /// Overrides the default value for `BottomSheet::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Value for `BottomSheet::elevation` when the Bottom sheet is presented as a
    /// modal bottom sheet.
    pub modal_elevation: Option<f64>,

    /// Overrides the default value for `BottomSheet::shape`.
    ///
    /// If null, no overriding shape is specified for `BottomSheet`, so the
    /// `BottomSheet` is rectangular.
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// Overrides the default value for `BottomSheet::show_drag_handle`.
    pub show_drag_handle: Option<bool>,

    /// Overrides the default value for `BottomSheet::drag_handle_color`.
    pub drag_handle_color: Option<AnyColor>,

    /// Overrides the default value for `BottomSheet::drag_handle_size`.
    pub drag_handle_size: Option<Size>,

    /// Overrides the default value for `BottomSheet::clip_behavior`.
    ///
    /// If null, `BottomSheet` uses [`Clip::None`].
    pub clip_behavior: Option<Clip>,

    /// Constrains the size of the `BottomSheet`.
    ///
    /// If null, the bottom sheet's size will be unconstrained.
    pub constraints: Option<BoxConstraints>,
}

impl BottomSheetThemeData {
    /// Creates a theme that can be used for `ThemeData::bottom_sheet_theme`; Dart's optional
    /// named arguments are the fluent setters.
    pub fn new() -> BottomSheetThemeData {
        BottomSheetThemeData::default()
    }

    /// Dart `BottomSheetThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> BottomSheetThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `BottomSheetThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> BottomSheetThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `BottomSheetThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> BottomSheetThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `BottomSheetThemeData(modalBackgroundColor:)`.
    pub fn modal_background_color(mut self, modal_background_color: Color) -> BottomSheetThemeData {
        self.modal_background_color = Some(modal_background_color);
        self
    }

    /// Dart `BottomSheetThemeData(modalBarrierColor:)`.
    pub fn modal_barrier_color(mut self, modal_barrier_color: Color) -> BottomSheetThemeData {
        self.modal_barrier_color = Some(modal_barrier_color);
        self
    }

    /// Dart `BottomSheetThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> BottomSheetThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `BottomSheetThemeData(modalElevation:)`.
    pub fn modal_elevation(mut self, modal_elevation: f64) -> BottomSheetThemeData {
        self.modal_elevation = Some(modal_elevation);
        self
    }

    /// Dart `BottomSheetThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> BottomSheetThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `BottomSheetThemeData(showDragHandle:)`.
    pub fn show_drag_handle(mut self, show_drag_handle: bool) -> BottomSheetThemeData {
        self.show_drag_handle = Some(show_drag_handle);
        self
    }

    /// Dart `BottomSheetThemeData(dragHandleColor:)`.
    pub fn drag_handle_color(
        mut self,
        drag_handle_color: impl Into<AnyColor>,
    ) -> BottomSheetThemeData {
        self.drag_handle_color = Some(drag_handle_color.into());
        self
    }

    /// Dart `BottomSheetThemeData(dragHandleSize:)`.
    pub fn drag_handle_size(mut self, drag_handle_size: Size) -> BottomSheetThemeData {
        self.drag_handle_size = Some(drag_handle_size);
        self
    }

    /// Dart `BottomSheetThemeData(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> BottomSheetThemeData {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `BottomSheetThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> BottomSheetThemeData {
        self.constraints = Some(constraints);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(4.0)`).
    pub fn copy_with(&self) -> BottomSheetThemeData {
        self.clone()
    }

    /// Linearly interpolate between two bottom sheet themes.
    ///
    /// If both arguments are null then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&BottomSheetThemeData>,
        b: Option<&BottomSheetThemeData>,
        t: f64,
    ) -> Option<BottomSheetThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(BottomSheetThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            modal_background_color: Color::lerp(
                a.and_then(|a| a.modal_background_color),
                b.and_then(|b| b.modal_background_color),
                t,
            ),
            modal_barrier_color: Color::lerp(
                a.and_then(|a| a.modal_barrier_color),
                b.and_then(|b| b.modal_barrier_color),
                t,
            ),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            modal_elevation: lerp_double(
                a.and_then(|a| a.modal_elevation),
                b.and_then(|b| b.modal_elevation),
                t,
            ),
            shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.shape.as_deref()),
                b.and_then(|b| b.shape.as_deref()),
                t,
            ),
            show_drag_handle: pick(a, b, t, |theme| theme.show_drag_handle),
            drag_handle_color: AnyColor::lerp(
                a.and_then(|a| a.drag_handle_color.as_ref()),
                b.and_then(|b| b.drag_handle_color.as_ref()),
                t,
            )
            .map(AnyColor::from),
            drag_handle_size: Size::lerp(
                a.and_then(|a| a.drag_handle_size),
                b.and_then(|b| b.drag_handle_size),
                t,
            ),
            clip_behavior: pick(a, b, t, |theme| theme.clip_behavior),
            constraints: BoxConstraints::lerp(
                a.and_then(|a| a.constraints),
                b.and_then(|b| b.constraints),
                t,
            ),
        })
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&BottomSheetThemeData>,
    b: Option<&BottomSheetThemeData>,
    t: f64,
    field: impl Fn(&BottomSheetThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};

    use super::*;
    use crate::colors::Colors;
    use crate::test_support::{test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(radius))
    }

    #[test]
    fn bottom_sheet_theme_data_copy_with_equality_basics() {
        assert_eq!(
            BottomSheetThemeData::new(),
            BottomSheetThemeData::new().copy_with()
        );

        let theme = BottomSheetThemeData::new()
            .elevation(4.0)
            .shape(rounded(8.0))
            .drag_handle_size(Size::new(32.0, 4.0));
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(
            theme.copy_with().modal_elevation(9.0).modal_elevation,
            Some(9.0)
        );
        assert_eq!(theme.copy_with().modal_elevation(9.0).elevation, Some(4.0));
        assert_ne!(theme.copy_with().elevation(5.0), theme);
        assert_ne!(theme.copy_with().shape(rounded(9.0)), theme);
    }

    #[test]
    fn bottom_sheet_theme_data_lerp_special_cases() {
        assert_eq!(BottomSheetThemeData::lerp(None, None, 0.0), None);
        let data = BottomSheetThemeData::new();
        assert_eq!(
            BottomSheetThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn bottom_sheet_theme_data_null_fields_by_default() {
        let theme = BottomSheetThemeData::new();
        assert_eq!(theme.background_color, None);
        assert_eq!(theme.surface_tint_color, None);
        assert_eq!(theme.shadow_color, None);
        assert_eq!(theme.elevation, None);
        assert_eq!(theme.modal_background_color, None);
        assert_eq!(theme.modal_barrier_color, None);
        assert_eq!(theme.modal_elevation, None);
        assert!(theme.shape.is_none());
        assert_eq!(theme.show_drag_handle, None);
        assert_eq!(theme.clip_behavior, None);
        assert_eq!(theme.constraints, None);
        assert_eq!(theme.drag_handle_color, None);
        assert_eq!(theme.drag_handle_size, None);
    }

    #[test]
    fn bottom_sheet_theme_data_lerps_each_field() {
        let a = BottomSheetThemeData::new()
            .background_color(Colors::BLACK)
            .elevation(2.0)
            .modal_elevation(4.0)
            .shape(rounded(4.0))
            .show_drag_handle(true)
            .drag_handle_size(Size::new(20.0, 2.0))
            .clip_behavior(Clip::HardEdge)
            .constraints(BoxConstraints::tight(Size::new(100.0, 100.0)));
        let b = BottomSheetThemeData::new()
            .background_color(Colors::WHITE)
            .surface_tint_color(Colors::RED.primary())
            .elevation(6.0)
            .modal_elevation(8.0)
            .shape(rounded(8.0))
            .show_drag_handle(false)
            .drag_handle_size(Size::new(40.0, 6.0))
            .clip_behavior(Clip::AntiAlias)
            .constraints(BoxConstraints::tight(Size::new(300.0, 300.0)));

        let halfway = BottomSheetThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both given");

        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.surface_tint_color,
            Color::lerp(None, Some(Colors::RED.primary()), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(halfway.modal_elevation, Some(6.0));
        assert_eq!(
            halfway.shape.as_deref(),
            Some(&rounded(6.0) as &dyn ShapeBorder)
        );
        assert_eq!(halfway.show_drag_handle, Some(false));
        assert_eq!(halfway.drag_handle_size, Some(Size::new(30.0, 4.0)));
        assert_eq!(halfway.clip_behavior, Some(Clip::AntiAlias));
        assert_eq!(
            halfway.constraints,
            Some(BoxConstraints::tight(Size::new(200.0, 200.0)))
        );

        let early = BottomSheetThemeData::lerp(Some(&a), Some(&b), 0.25).expect("both given");
        assert_eq!(early.show_drag_handle, Some(true));
        assert_eq!(early.clip_behavior, Some(Clip::HardEdge));
    }

    #[test]
    fn theme_data_holds_the_bottom_sheet_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.bottom_sheet_theme, BottomSheetThemeData::new());

            let tall = ThemeData::builder()
                .bottom_sheet_theme(BottomSheetThemeData::new().elevation(8.0))
                .build(&app);
            assert_eq!(tall.bottom_sheet_theme.elevation, Some(8.0));
            assert_ne!(theme, tall);
            assert_eq!(
                theme
                    .copy_with()
                    .bottom_sheet_theme(tall.bottom_sheet_theme.clone())
                    .build(),
                tall
            );

            let flat = ThemeData::builder()
                .bottom_sheet_theme(BottomSheetThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &tall, 0.5)
                    .bottom_sheet_theme
                    .elevation,
                Some(4.0)
            );
        });
    }
}
