//! Flutter counterpart: material_ui's `material_state.dart`.
//!
//! The deprecated Material names of the widgets layer's `widget_state.rs` types, as Dart's
//! typedefs are. `MaterialStateProperty` is the erased property type
//! `dyn WidgetStateProperty<T>`: Rust has no stable alias of a trait, so a type implements
//! [`WidgetStateProperty`] by that name. The input borders that are also properties
//! (`MaterialStateOutlineInputBorder`, `MaterialStateUnderlineInputBorder`,
//! `WidgetStateInputBorder`) implement [`InputBorder`] by delegating to the plain border their
//! Dart class extends.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Canvas, Paint, Path, Rect};
use waft_painting::{BorderSide, EdgeInsetsGeometry, ShapeBorder, ShapeBorderRef, TextDirection};
use waft_widgets::{
    WidgetPropertyResolver, WidgetState, WidgetStateBorderSide, WidgetStateColor, WidgetStateMap,
    WidgetStateMapper, WidgetStateMouseCursor, WidgetStateOutlinedBorder, WidgetStateProperty,
    WidgetStatePropertyAll, WidgetStateTextStyle, WidgetStates, WidgetStatesController,
};

use crate::input_border::{InputBorder, OutlineInputBorder, UnderlineInputBorder};

/// Interactive states that some of the Material widgets can take on when
/// receiving input from the user.
///
/// States are defined by <https://material.io/design/interaction/states.html#usage>.
///
/// Some Material widgets track their current state in a `Set<MaterialState>`.
///
/// See also:
///
///  * [`WidgetState`], a general non-Material version that can be used
///    interchangeably with `MaterialState`. They functionally work the same,
///    except [`WidgetState`] can be used outside of Material.
///  * [`MaterialStateProperty`], an interface for objects that "resolve" to
///    different values depending on a widget's material state.
///  * [`MaterialStateColor`], a `Color` that implements `MaterialStateProperty`
///    which is used in APIs that need to accept either a `Color` or a
///    `MaterialStateProperty<Color>`.
///  * [`MaterialStateMouseCursor`], a `MouseCursor` that implements
///    `MaterialStateProperty` which is used in APIs that need to accept either
///    a `MouseCursor` or a `MaterialStateProperty<MouseCursor>`.
///  * [`MaterialStateOutlinedBorder`], an `OutlinedBorder` that implements
///    `MaterialStateProperty` which is used in APIs that need to accept either
///    an `OutlinedBorder` or a `MaterialStateProperty<OutlinedBorder>`.
///  * `MaterialStateOutlineInputBorder`, an `OutlineInputBorder` that implements
///    `MaterialStateProperty` which is used in APIs that need to accept either
///    an `OutlineInputBorder` or a `MaterialStateProperty<OutlineInputBorder>`.
///  * `MaterialStateUnderlineInputBorder`, an `UnderlineInputBorder` that implements
///    `MaterialStateProperty` which is used in APIs that need to accept either
///    an `UnderlineInputBorder` or a `MaterialStateProperty<UnderlineInputBorder>`.
///  * [`MaterialStateBorderSide`], a `BorderSide` that implements
///    `MaterialStateProperty` which is used in APIs that need to accept either
///    a `BorderSide` or a `MaterialStateProperty<BorderSide>`.
///  * [`MaterialStateTextStyle`], a `TextStyle` that implements
///    `MaterialStateProperty` which is used in APIs that need to accept either
///    a `TextStyle` or a `MaterialStateProperty<TextStyle>`.
#[deprecated(
    note = "Use WidgetState instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialState = WidgetState;

/// Signature for the function that returns a value of type `T` based on a given
/// set of states.
///
/// See also:
///
///  * [`WidgetPropertyResolver`], the non-Material form of `MaterialPropertyResolver`
///    that can be used interchangeably with `MaterialPropertyResolver`.
#[deprecated(
    note = "Use WidgetPropertyResolver instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialPropertyResolver<T> = WidgetPropertyResolver<T>;

/// Defines a `Color` that is also a [`MaterialStateProperty`].
///
/// This class exists to enable widgets with `Color` valued properties
/// to also accept `MaterialStateProperty<Color>` values. A material
/// state color property represents a color which depends on
/// a widget's "interactive state". This state is represented as a
/// set of [`MaterialState`]s, like [`WidgetState::Pressed`],
/// [`WidgetState::Focused`] and [`WidgetState::Hovered`].
///
/// [`MaterialStateColor`] should only be used with widgets that document
/// their support, like `TimePickerThemeData.dayPeriodColor`.
///
/// To use a [`MaterialStateColor`], use [`WidgetStateColor::resolve_with`] and pass in a
/// callback that will be used to resolve the color in the given states.
///
/// If a [`MaterialStateColor`] is used for a property or a parameter that doesn't
/// support resolving `MaterialStateProperty<Color>`s, then its default color
/// value will be used for all states.
///
/// See also
///
///  * [`WidgetStateColor`], the non-Material version that can be used
///    interchangeably with `MaterialStateColor`.
#[deprecated(
    note = "Use WidgetStateColor instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStateColor = WidgetStateColor;

/// Defines a `MouseCursor` whose value depends on a set of [`MaterialState`]s which
/// represent the interactive state of a component.
///
/// This kind of `MouseCursor` is useful when the set of interactive
/// actions a widget supports varies with its state. For example, a
/// mouse pointer hovering over a disabled `ListTile` should not
/// display `SystemMouseCursors.click`, since a disabled list tile
/// doesn't respond to mouse clicks. `ListTile`'s default mouse cursor
/// is a [`WidgetStateMouseCursor::clickable`], which resolves to
/// `SystemMouseCursors.basic` when the button is disabled.
///
/// This class should only be used for parameters which are documented to take
/// [`MaterialStateMouseCursor`], otherwise only the default state will be used.
///
/// See also:
///
///  * [`WidgetStateMouseCursor`], the non-Material version that can be used
///    interchangeably with `MaterialStateMouseCursor`.
///  * `MouseCursor` for introduction on the mouse cursor system.
///  * `SystemMouseCursors`, which defines cursors that are supported by
///    native platforms.
#[deprecated(
    note = "Use WidgetStateMouseCursor instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStateMouseCursor = WidgetStateMouseCursor;

/// Defines a `BorderSide` whose value depends on a set of [`MaterialState`]s
/// which represent the interactive state of a component.
///
/// This class enables existing widget implementations with `BorderSide`
/// properties to be extended to also effectively support `MaterialStateProperty<BorderSide>`
/// property values. [`MaterialStateBorderSide`] should only be used with widgets that document
/// their support, like `ActionChip.side`.
///
/// This class should only be used for parameters which are documented to take
/// [`MaterialStateBorderSide`], otherwise only the default state will be used.
///
/// See also:
///
///  * [`WidgetStateBorderSide`], the non-Material version that can be used
///    interchangeably with `MaterialStateBorderSide`.
#[deprecated(
    note = "Use WidgetStateBorderSide instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStateBorderSide = WidgetStateBorderSide;

/// Defines an `OutlinedBorder` whose value depends on a set of [`MaterialState`]s
/// which represent the interactive state of a component.
///
/// This class should only be used for parameters which are documented to take
/// [`MaterialStateOutlinedBorder`], otherwise only the default state will be used.
///
/// See also:
///
///  * [`WidgetStateOutlinedBorder`], the non-Material version that can be used
///    interchangeably with `MaterialStateOutlinedBorder`.
///  * `ShapeBorder` the base class for shape outlines.
#[deprecated(
    note = "Use WidgetStateOutlinedBorder instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStateOutlinedBorder = WidgetStateOutlinedBorder;

/// Defines a `TextStyle` that is also a [`MaterialStateProperty`].
///
/// This class exists to enable widgets with `TextStyle` valued properties
/// to also accept `MaterialStateProperty<TextStyle>` values. A material
/// state text style property represents a text style which depends on
/// a widget's "interactive state". This state is represented as a
/// set of [`MaterialState`]s, like [`WidgetState::Pressed`],
/// [`WidgetState::Focused`] and [`WidgetState::Hovered`].
///
/// [`MaterialStateTextStyle`] should only be used with widgets that document
/// their support, like `InputDecoration.labelStyle`.
///
/// To use a [`MaterialStateTextStyle`], use [`WidgetStateTextStyle::resolve_with`] and pass
/// in a callback that will be used to resolve the color in the given states.
///
/// If a [`MaterialStateTextStyle`] is used for a property or a parameter that doesn't
/// support resolving `MaterialStateProperty<TextStyle>`s, then its default color
/// value will be used for all states.
///
/// See also:
///
///  * [`WidgetStateTextStyle`], the non-Material version that can be used
///    interchangeably with `MaterialStateTextStyle`.
#[deprecated(
    note = "Use WidgetStateTextStyle instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStateTextStyle = WidgetStateTextStyle;

/// The `ShapeBorder` and `InputBorder` members of an input border that is also a
/// `WidgetStateProperty<InputBorder>`: those of the plain border Dart's class extends,
/// held in `self.border`, with `as_widget_state_property` answering the type itself.
macro_rules! state_input_border {
    ($name:ty, $debug:literal) => {
        #[allow(deprecated)]
        impl ShapeBorder for $name {
            fn dimensions(&self) -> EdgeInsetsGeometry {
                self.border.dimensions()
            }

            fn scale(&self, t: f64) -> ShapeBorderRef {
                ShapeBorder::scale(&self.border, t)
            }

            fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<ShapeBorderRef> {
                self.border.lerp_from(a, t)
            }

            fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<ShapeBorderRef> {
                self.border.lerp_to(b, t)
            }

            fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Path {
                self.border.get_outer_path(rect, text_direction)
            }

            fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Path {
                self.border.get_inner_path(rect, text_direction)
            }

            fn paint_interior(
                &self,
                canvas: &mut Canvas,
                rect: Rect,
                paint: &Paint,
                text_direction: Option<TextDirection>,
            ) {
                self.border
                    .paint_interior(canvas, rect, paint, text_direction);
            }

            fn prefer_paint_interior(&self) -> bool {
                self.border.prefer_paint_interior()
            }

            fn paint(
                &self,
                canvas: &mut Canvas,
                rect: Rect,
                text_direction: Option<TextDirection>,
            ) {
                ShapeBorder::paint(&self.border, canvas, rect, text_direction);
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
                other
                    .as_any()
                    .downcast_ref::<$name>()
                    .is_some_and(|other| self.eq_subclass(other))
            }
        }

        #[allow(deprecated)]
        impl InputBorder for $name {
            fn border_side(&self) -> BorderSide {
                self.border.border_side
            }

            fn copy_with(&self, border_side: Option<BorderSide>) -> Box<dyn InputBorder> {
                InputBorder::copy_with(&self.border, border_side)
            }

            fn is_outline(&self) -> bool {
                self.border.is_outline()
            }

            fn paint(
                &self,
                canvas: &mut Canvas,
                rect: Rect,
                gap_start: Option<f64>,
                gap_extent: f64,
                gap_percentage: f64,
                text_direction: Option<TextDirection>,
            ) {
                InputBorder::paint(
                    &self.border,
                    canvas,
                    rect,
                    gap_start,
                    gap_extent,
                    gap_percentage,
                    text_direction,
                );
            }

            fn clone_input_border(&self) -> Box<dyn InputBorder> {
                Box::new(self.clone())
            }

            fn as_widget_state_property(
                &self,
            ) -> Option<&dyn WidgetStateProperty<Box<dyn InputBorder>>> {
                Some(self)
            }
        }

        #[allow(deprecated)]
        impl Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str($debug)
            }
        }
    };
}

/// Defines a [`OutlineInputBorder`] that is also a [`MaterialStateProperty`].
///
/// This class exists to enable widgets with [`OutlineInputBorder`] valued properties
/// to also accept `MaterialStateProperty<OutlineInputBorder>` values. A material
/// state input border property represents an input border which depends on
/// a widget's "interactive state". This state is represented as a
/// set of [`MaterialState`]s, like [`WidgetState::Pressed`],
/// [`WidgetState::Focused`] and [`WidgetState::Hovered`].
///
/// [`MaterialStateOutlineInputBorder`] should only be used with widgets that document
/// their support, like `InputDecoration.border`.
///
/// To use a [`MaterialStateOutlineInputBorder`], use
/// [`MaterialStateOutlineInputBorder::resolve_with`] and pass in a callback that
/// will be used to resolve the color in the given states.
///
/// If a [`MaterialStateOutlineInputBorder`] is used for a property or a parameter that doesn't
/// support resolving `MaterialStateProperty<OutlineInputBorder>`s, then its default color
/// value will be used for all states.
///
/// It is itself an [`InputBorder`]: the default `OutlineInputBorder()` it extends in Dart.
#[deprecated(
    note = "Use WidgetStateInputBorder instead. Renamed to match other WidgetStateProperty objects. This feature was deprecated after v3.26.0-0.1.pre."
)]
#[derive(Clone)]
pub struct MaterialStateOutlineInputBorder {
    /// Dart's `_MaterialStateOutlineInputBorder._resolve`.
    resolve: WidgetPropertyResolver<Box<dyn InputBorder>>,
    /// Dart's `super`: `MaterialStateOutlineInputBorder extends OutlineInputBorder`.
    border: OutlineInputBorder,
}

#[allow(deprecated)]
impl MaterialStateOutlineInputBorder {
    /// Creates a [`MaterialStateOutlineInputBorder`] from a
    /// `MaterialPropertyResolver<InputBorder>` callback function.
    ///
    /// If used as a regular input border, the border resolved in the default state (the
    /// empty set of states) will be used.
    ///
    /// The given callback parameter must return a non-null text style in the default
    /// state.
    #[deprecated(
        note = "Use WidgetStateInputBorder.resolveWith() instead. Renamed to match other WidgetStateProperty objects. This feature was deprecated after v3.26.0-0.1.pre."
    )]
    pub fn resolve_with(
        callback: impl Fn(&WidgetStates) -> Box<dyn InputBorder> + 'static,
    ) -> MaterialStateOutlineInputBorder {
        MaterialStateOutlineInputBorder {
            resolve: Rc::new(callback),
            border: OutlineInputBorder::new(),
        }
    }

    // Dart's `OutlineInputBorder.==`: the same runtime type and fields.
    fn eq_subclass(&self, other: &MaterialStateOutlineInputBorder) -> bool {
        self.border == other.border
    }
}

/// Returns a [`InputBorder`] that's to be used when a Material component is in the
/// specified state.
#[allow(deprecated)]
impl WidgetStateProperty<Box<dyn InputBorder>> for MaterialStateOutlineInputBorder {
    fn resolve(&self, states: &WidgetStates) -> Box<dyn InputBorder> {
        (self.resolve)(states)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_property(&self, other: &dyn WidgetStateProperty<Box<dyn InputBorder>>) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.eq_subclass(other))
    }
}

state_input_border!(
    MaterialStateOutlineInputBorder,
    "_MaterialStateOutlineInputBorder()"
);

/// Defines a [`UnderlineInputBorder`] that is also a [`MaterialStateProperty`].
///
/// This class exists to enable widgets with [`UnderlineInputBorder`] valued properties
/// to also accept `MaterialStateProperty<UnderlineInputBorder>` values. A material
/// state input border property represents an input border which depends on
/// a widget's "interactive state". This state is represented as a
/// set of [`MaterialState`]s, like [`WidgetState::Pressed`],
/// [`WidgetState::Focused`] and [`WidgetState::Hovered`].
///
/// [`MaterialStateUnderlineInputBorder`] should only be used with widgets that document
/// their support, like `InputDecoration.border`.
///
/// To use a [`MaterialStateUnderlineInputBorder`], use
/// [`MaterialStateUnderlineInputBorder::resolve_with`] and pass in a callback that
/// will be used to resolve the color in the given states.
///
/// If a [`MaterialStateUnderlineInputBorder`] is used for a property or a parameter that doesn't
/// support resolving `MaterialStateProperty<UnderlineInputBorder>`s, then its default color
/// value will be used for all states.
///
/// It is itself an [`InputBorder`]: the default `UnderlineInputBorder()` it extends in Dart.
#[deprecated(
    note = "Use WidgetStateInputBorder instead. Renamed to match other WidgetStateProperty objects. This feature was deprecated after v3.26.0-0.1.pre."
)]
#[derive(Clone)]
pub struct MaterialStateUnderlineInputBorder {
    /// Dart's `_MaterialStateUnderlineInputBorder._resolve`.
    resolve: WidgetPropertyResolver<Box<dyn InputBorder>>,
    /// Dart's `super`: `MaterialStateUnderlineInputBorder extends UnderlineInputBorder`.
    border: UnderlineInputBorder,
}

#[allow(deprecated)]
impl MaterialStateUnderlineInputBorder {
    /// Creates a [`MaterialStateUnderlineInputBorder`] from a
    /// `MaterialPropertyResolver<InputBorder>` callback function.
    ///
    /// If used as a regular input border, the border resolved in the default state (the
    /// empty set of states) will be used.
    ///
    /// The given callback parameter must return a non-null text style in the default
    /// state.
    #[deprecated(
        note = "Use WidgetStateInputBorder.resolveWith() instead. Renamed to match other WidgetStateProperty objects. This feature was deprecated after v3.26.0-0.1.pre."
    )]
    pub fn resolve_with(
        callback: impl Fn(&WidgetStates) -> Box<dyn InputBorder> + 'static,
    ) -> MaterialStateUnderlineInputBorder {
        MaterialStateUnderlineInputBorder {
            resolve: Rc::new(callback),
            border: UnderlineInputBorder::new(),
        }
    }

    // Dart's `UnderlineInputBorder.==`: the same runtime type and fields.
    fn eq_subclass(&self, other: &MaterialStateUnderlineInputBorder) -> bool {
        self.border == other.border
    }
}

/// Returns a [`InputBorder`] that's to be used when a Material component is in the
/// specified state.
#[allow(deprecated)]
impl WidgetStateProperty<Box<dyn InputBorder>> for MaterialStateUnderlineInputBorder {
    fn resolve(&self, states: &WidgetStates) -> Box<dyn InputBorder> {
        (self.resolve)(states)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_property(&self, other: &dyn WidgetStateProperty<Box<dyn InputBorder>>) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.eq_subclass(other))
    }
}

state_input_border!(
    MaterialStateUnderlineInputBorder,
    "_MaterialStateUnderlineInputBorder()"
);

/// Defines an [`InputBorder`] that is also a [`WidgetStateProperty`].
///
/// This class exists to enable widgets with [`InputBorder`] valued properties
/// to also accept [`WidgetStateProperty`] objects.
///
/// [`WidgetStateInputBorder`] should only be used with widgets that document
/// their support, like `InputDecoration.border`.
///
/// A [`WidgetStateInputBorder`] can be created by:
///  1. Using [`WidgetStateInputBorder::resolve_with`] with a callback that
///     resolves the input border in the given states.
///  2. Using [`WidgetStateInputBorder::from_map`] to assign a border with a
///     [`WidgetStateMap`].
///
/// It is itself an [`InputBorder`]: the default `OutlineInputBorder()` that Dart's
/// `_WidgetStateInputBorder` extends, for a map-built one too.
#[derive(Clone)]
pub struct WidgetStateInputBorder {
    subclass: InputBorderSubclass,
    /// Dart's `super`: `_WidgetStateInputBorder extends OutlineInputBorder`.
    border: OutlineInputBorder,
}

/// Dart's two private implementations of `WidgetStateInputBorder`.
#[derive(Clone)]
enum InputBorderSubclass {
    /// `_WidgetStateInputBorder`.
    ResolveWith(WidgetPropertyResolver<Box<dyn InputBorder>>),
    /// `_WidgetInputBorderMapper`.
    Mapper(Rc<WidgetStateMapper<Box<dyn InputBorder>>>),
}

impl WidgetStateInputBorder {
    /// Creates a [`WidgetStateInputBorder`] using a [`WidgetPropertyResolver`]
    /// callback.
    ///
    /// This constructor should only be used for fields that support
    /// [`WidgetStateInputBorder`], such as `InputDecoration.border`
    /// (if used as a regular [`InputBorder`], it acts the same as
    /// an empty `OutlineInputBorder()` constructor).
    pub fn resolve_with(
        callback: impl Fn(&WidgetStates) -> Box<dyn InputBorder> + 'static,
    ) -> WidgetStateInputBorder {
        WidgetStateInputBorder {
            subclass: InputBorderSubclass::ResolveWith(Rc::new(callback)),
            border: OutlineInputBorder::new(),
        }
    }

    /// Creates a [`WidgetStateInputBorder`] from a [`WidgetStateMap`].
    ///
    /// This constructor's [`resolve`](WidgetStateProperty::resolve) method finds the first
    /// entry whose key is satisfied by the set of states, and returns its associated value.
    /// It should only be used for fields that support [`WidgetStateInputBorder`]
    /// objects, such as `InputDecoration.border`.
    pub fn from_map(map: WidgetStateMap<Box<dyn InputBorder>>) -> WidgetStateInputBorder {
        WidgetStateInputBorder {
            subclass: InputBorderSubclass::Mapper(Rc::new(WidgetStateMapper::new(map))),
            border: OutlineInputBorder::new(),
        }
    }

    // `_WidgetStateInputBorder` has `OutlineInputBorder.==` (the same runtime type and
    // fields); the mapper has `WidgetStateMapper.==`, which compares maps.
    fn eq_subclass(&self, other: &WidgetStateInputBorder) -> bool {
        match (&self.subclass, &other.subclass) {
            (InputBorderSubclass::ResolveWith(_), InputBorderSubclass::ResolveWith(_)) => {
                self.border == other.border
            }
            (InputBorderSubclass::Mapper(mine), InputBorderSubclass::Mapper(theirs)) => {
                mine == theirs
            }
            _ => false,
        }
    }
}

impl WidgetStateProperty<Box<dyn InputBorder>> for WidgetStateInputBorder {
    fn resolve(&self, states: &WidgetStates) -> Box<dyn InputBorder> {
        match &self.subclass {
            InputBorderSubclass::ResolveWith(resolve) => resolve(states),
            InputBorderSubclass::Mapper(mapper) => mapper.resolve(states),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mapper(&self) -> Option<&WidgetStateMapper<Box<dyn InputBorder>>> {
        match &self.subclass {
            InputBorderSubclass::Mapper(mapper) => Some(mapper.as_ref()),
            InputBorderSubclass::ResolveWith(_) => None,
        }
    }

    fn eq_property(&self, other: &dyn WidgetStateProperty<Box<dyn InputBorder>>) -> bool {
        if let InputBorderSubclass::Mapper(mapper) = &self.subclass {
            return mapper.eq_property(other);
        }
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.eq_subclass(other))
    }
}

state_input_border!(WidgetStateInputBorder, "_WidgetStateInputBorder()");

/// Interface for classes that [`resolve`](WidgetStateProperty::resolve) to a value of
/// type `T` based on a widget's interactive "state", which is defined as a set
/// of [`MaterialState`]s.
///
/// Material state properties represent values that depend on a widget's material
/// "state". The state is encoded as a set of [`MaterialState`] values, like
/// [`WidgetState::Focused`], [`WidgetState::Hovered`], [`WidgetState::Pressed`]. For
/// example the `InkWell.overlayColor` defines the color that fills the ink well
/// when it's pressed (the "splash color"), focused, or hovered. The `InkWell`
/// uses the overlay color's [`resolve`](WidgetStateProperty::resolve) method to compute
/// the color for the ink well's current state.
///
/// `ButtonStyle`, which is used to configure the appearance of
/// buttons like `TextButton`, `ElevatedButton`, and `OutlinedButton`,
/// has many material state properties. The button widgets keep track
/// of their current material state and [`resolve`](WidgetStateProperty::resolve) the
/// button style's material state properties when their value is needed.
///
/// This is the erased property, `dyn WidgetStateProperty<T>`: Dart's static members are
/// reached through it (`<MaterialStateProperty<Color>>::resolve_with(..)`), and a type
/// implements [`WidgetStateProperty`] itself.
///
/// See also:
///
///  * [`WidgetStateProperty`], the non-Material version that can be used
///    interchangeably with `MaterialStateProperty`.
#[deprecated(
    note = "Use WidgetStateProperty instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStateProperty<T> = dyn WidgetStateProperty<T>;

/// Convenience class for creating a [`MaterialStateProperty`] that
/// resolves to the given value for all states.
///
/// See also:
///
///  * [`WidgetStatePropertyAll`], the non-Material version that can be used
///    interchangeably with `MaterialStatePropertyAll`.
#[deprecated(
    note = "Use WidgetStatePropertyAll instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStatePropertyAll<T> = WidgetStatePropertyAll<T>;

/// Manages a set of [`MaterialState`]s and notifies listeners of changes.
///
/// Used by widgets that expose their internal state for the sake of
/// extensions that add support for additional states. See
/// `TextButton` for an example.
///
/// The controller's [`value`](WidgetStatesController::value) is its current set of
/// states. Listeners are notified whenever the
/// [`value`](WidgetStatesController::value) changes. The
/// [`value`](WidgetStatesController::value) should only be changed with
/// [`update`](WidgetStatesController::update); it should not be modified directly.
///
/// The controller's [`value`](WidgetStatesController::value) represents the set of
/// states that a widget's visual properties, typically [`MaterialStateProperty`]
/// values, are resolved against. It is _not_ the intrinsic state of
/// the widget. The widget is responsible for ensuring that the
/// controller's [`value`](WidgetStatesController::value) tracks its intrinsic state.
/// For example one cannot request the keyboard focus for a widget by adding
/// [`WidgetState::Focused`] to its controller. When the widget gains or loses the
/// focus it will [`update`](WidgetStatesController::update) its controller's
/// [`value`](WidgetStatesController::value) and notify listeners of the change.
///
/// When calling `set_state` in a [`MaterialStatesController`] listener, use the
/// `SchedulerBinding::add_post_frame_callback` to delay the call to `set_state` after
/// the frame has been rendered. It's generally prudent to use the
/// `SchedulerBinding::add_post_frame_callback` because some of the widgets that
/// depend on [`MaterialStatesController`] may call
/// [`update`](WidgetStatesController::update) in their build method.
/// In such cases, listener's that call `set_state` - during the build phase - will cause
/// an error.
///
/// See also:
///
///  * [`WidgetStatesController`], the non-Material version that can be used
///    interchangeably with `MaterialStatesController`.
#[deprecated(
    note = "Use WidgetStatesController instead. Moved to the Widgets layer to make code available outside of Material. This feature was deprecated after v3.19.0-0.3.pre."
)]
pub type MaterialStatesController = WidgetStatesController;

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use waft_embedder::Color;
    use waft_foundation::{AppCell, ChangeNotifier, Listenable, Listener};
    use waft_painting::TextStyle;
    use waft_widgets::WidgetStates;

    use super::*;

    const EVERY_STATE: [MaterialState; 7] = [
        MaterialState::Hovered,
        MaterialState::Focused,
        MaterialState::Pressed,
        MaterialState::Dragged,
        MaterialState::Selected,
        MaterialState::Disabled,
        MaterialState::Error,
    ];

    fn states(states: impl IntoIterator<Item = MaterialState>) -> WidgetStates {
        states.into_iter().collect()
    }

    // material_state_property_test.dart 'MaterialStateProperty.resolveWith()'.
    #[test]
    fn resolve_with_resolves_through_the_callback() {
        let value: Rc<MaterialStateProperty<MaterialState>> =
            <MaterialStateProperty<MaterialState>>::resolve_with(|states| {
                states.iter().next().copied().expect("one state")
            });
        for state in EVERY_STATE {
            assert_eq!(value.resolve(&states([state])), state);
        }
    }

    // material_state_property_test.dart 'MaterialStateProperty.all()'.
    #[test]
    fn all_resolves_to_its_value_for_every_state() {
        let value: Rc<MaterialStateProperty<i32>> = <MaterialStateProperty<i32>>::all(123);
        for state in EVERY_STATE {
            assert_eq!(value.resolve(&states([state])), 123);
        }
    }

    // material_state_property_test.dart 'MaterialStatePropertyAll'.
    #[test]
    fn material_state_property_all_resolves_to_its_value_for_every_state() {
        let value = MaterialStatePropertyAll::new(123);
        assert_eq!(value.resolve(&WidgetStates::new()), 123);
        for state in EVERY_STATE {
            assert_eq!(value.resolve(&states([state])), 123);
        }
    }

    // material_state_property_test.dart 'toString formats correctly'.
    #[test]
    fn a_material_state_property_all_prints_as_a_widget_state_property_all() {
        let color_property = MaterialStatePropertyAll::new(Some(Color::new(0xFFFFFFFF)));
        assert_eq!(
            format!("{color_property:?}"),
            format!("WidgetStatePropertyAll({:?})", Some(Color::new(0xffffffff)))
        );
    }

    // material_state_property_test.dart "Can interpolate between two MaterialStateProperty's".
    #[test]
    fn lerp_interpolates_the_resolved_values() {
        let enabled = WidgetStates::new();
        let text_style_1: Rc<MaterialStateProperty<TextStyle>> = Rc::new(
            MaterialStatePropertyAll::new(TextStyle::new().font_size(14.0)),
        );
        let text_style_2: Rc<MaterialStateProperty<TextStyle>> = Rc::new(
            MaterialStatePropertyAll::new(TextStyle::new().font_size(20.0)),
        );
        let lerp = |t: f64| {
            <MaterialStateProperty<TextStyle>>::lerp(
                Some(Rc::clone(&text_style_1)),
                Some(Rc::clone(&text_style_2)),
                t,
                |a, b, t| TextStyle::lerp(a.as_ref(), b.as_ref(), t),
            )
            .expect("one side")
            .resolve(&enabled)
            .expect("both sides")
            .font_size
        };
        assert_eq!(lerp(0.0), Some(14.0));
        assert_eq!(lerp(0.5), Some(17.0));
        assert_eq!(lerp(1.0), Some(20.0));
    }

    // material_states_controller_test.dart 'MaterialStatesController constructor'.
    #[test]
    fn controller_starts_with_the_given_states() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let none = MaterialStatesController::new(&mut app, None);
        assert_eq!(*app.get(&none).value(), WidgetStates::new());
        let empty = MaterialStatesController::new(&mut app, Some(WidgetStates::new()));
        assert_eq!(*app.get(&empty).value(), WidgetStates::new());
        let selected =
            MaterialStatesController::new(&mut app, Some(states([MaterialState::Selected])));
        assert_eq!(
            *app.get(&selected).value(),
            states([MaterialState::Selected])
        );
    }

    // material_states_controller_test.dart 'MaterialStatesController update, listener'.
    #[test]
    fn controller_update_notifies_only_on_change() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let count = Rc::new(Cell::new(0));
        let value_changed = Listener::new({
            let count = Rc::clone(&count);
            move |_app| count.set(count.get() + 1)
        });

        let controller = MaterialStatesController::new(&mut app, None);
        controller.add_listener(&mut app, value_changed.clone());

        let expect = |app: &waft_foundation::App, expected: &[MaterialState], expected_count| {
            assert_eq!(
                *app.get(&controller).value(),
                states(expected.iter().copied())
            );
            assert_eq!(count.get(), expected_count);
        };

        controller.update(&mut app, MaterialState::Selected, true);
        expect(&app, &[MaterialState::Selected], 1);
        controller.update(&mut app, MaterialState::Selected, true);
        expect(&app, &[MaterialState::Selected], 1);

        controller.update(&mut app, MaterialState::Hovered, false);
        expect(&app, &[MaterialState::Selected], 1);
        controller.update(&mut app, MaterialState::Selected, false);
        expect(&app, &[], 2);

        controller.update(&mut app, MaterialState::Hovered, true);
        expect(&app, &[MaterialState::Hovered], 3);
        controller.update(&mut app, MaterialState::Hovered, true);
        expect(&app, &[MaterialState::Hovered], 3);
        controller.update(&mut app, MaterialState::Pressed, true);
        expect(&app, &[MaterialState::Hovered, MaterialState::Pressed], 4);
        controller.update(&mut app, MaterialState::Selected, true);
        expect(
            &app,
            &[
                MaterialState::Hovered,
                MaterialState::Pressed,
                MaterialState::Selected,
            ],
            5,
        );
        controller.update(&mut app, MaterialState::Selected, false);
        expect(&app, &[MaterialState::Hovered, MaterialState::Pressed], 6);
        controller.update(&mut app, MaterialState::Selected, false);
        expect(&app, &[MaterialState::Hovered, MaterialState::Pressed], 6);
        controller.update(&mut app, MaterialState::Pressed, false);
        expect(&app, &[MaterialState::Hovered], 7);
        controller.update(&mut app, MaterialState::Hovered, false);
        expect(&app, &[], 8);

        controller.remove_listener(&mut app, &value_changed);
        controller.update(&mut app, MaterialState::Selected, true);
        expect(&app, &[MaterialState::Selected], 8);
    }

    // material_states_controller_test.dart 'MaterialStatesController const initial value'.
    #[test]
    fn controller_with_an_initial_value_notifies_only_on_change() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let count = Rc::new(Cell::new(0));
        let controller =
            MaterialStatesController::new(&mut app, Some(states([MaterialState::Selected])));
        controller.add_listener(&mut app, {
            let count = Rc::clone(&count);
            Listener::new(move |_app| count.set(count.get() + 1))
        });

        controller.update(&mut app, MaterialState::Selected, true);
        assert_eq!(
            *app.get(&controller).value(),
            states([MaterialState::Selected])
        );
        assert_eq!(count.get(), 0);

        controller.update(&mut app, MaterialState::Selected, false);
        assert_eq!(*app.get(&controller).value(), WidgetStates::new());
        assert_eq!(count.get(), 1);

        app.get_mut(&controller).dispose();
        assert!(!app.get(&controller).change_notifier_data().has_listeners());
    }
}
