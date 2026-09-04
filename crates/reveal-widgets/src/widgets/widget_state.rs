//! Flutter counterpart: `widgets/widget_state.dart`.
//!
//! `WidgetStateBorderSide`, `WidgetStateOutlinedBorder`, and `WidgetStateTextStyle` (with
//! `_LerpSides`) wait: painting's `BorderSide` and `TextStyle` are plain values with no
//! extension slot to carry a resolver, and `OutlinedBorder` has no stand-in for Dart's
//! `RoundedRectangleBorder` default. See the crate's PORTING.md.

use std::any::{Any, type_name};
use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::ops::{BitAnd, BitOr, Deref, Not};
use std::rc::Rc;

use reveal_embedder::Color;
use reveal_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB};
use reveal_painting::{AnyColor, ColorExtension};
use reveal_services::{MouseCursor, MouseCursorRef, MouseCursorSession, SystemMouseCursors};

/// Dart's `Set<WidgetState>`: the interactive states a widget is in.
pub type WidgetStates = HashSet<WidgetState>;

/// This mixin allows [`WidgetState`] enum values to be combined
/// using the `&`, `|`, and `~` operators.
///
/// A map with [`WidgetStatesConstraint`] objects as keys can be used
/// in the `<dyn WidgetStateProperty<T>>::from_map` constructor
/// to resolve to one of its values, based on the first key that
/// [`is_satisfied_by`](Self::is_satisfied_by) the current set of states.
///
/// A constraint is stored erased as a [`WidgetStatesConstraintRef`], which the operators
/// produce; `constraint.into()` erases a value by hand.
pub trait WidgetStatesConstraint: Debug + 'static {
    /// Whether the provided `states` satisfy this object's criteria.
    ///
    /// If the constraint is a single [`WidgetState`] object,
    /// it's satisfied by the set if the set contains the object.
    ///
    /// The constraint can also be created using one or more operators, for example:
    ///
    /// ```text
    /// let constraint = WidgetState::Focused | WidgetState::Hovered;
    /// ```
    ///
    /// In the above case, `constraint.is_satisfied_by(&states)` is equivalent to:
    ///
    /// ```text
    /// states.contains(&WidgetState::Focused) || states.contains(&WidgetState::Hovered)
    /// ```
    fn is_satisfied_by(&self, states: &WidgetStates) -> bool;

    /// Downcast support for Dart's `is` checks between constraint classes.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`. Identity unless a class overrides it, as the combinators do.
    fn eq_constraint(&self, other: &dyn WidgetStatesConstraint) -> bool {
        std::ptr::addr_eq(
            self as *const Self,
            other as *const dyn WidgetStatesConstraint,
        )
    }
}

impl PartialEq for dyn WidgetStatesConstraint {
    fn eq(&self, other: &dyn WidgetStatesConstraint) -> bool {
        self.eq_constraint(other)
    }
}

/// A shared, erased [`WidgetStatesConstraint`] (a Dart object reference).
///
/// The `&`, `|`, and `~` operators of the Dart mixin live here and on [`WidgetState`], and
/// produce this type: `WidgetState::Hovered & WidgetState::Focused`, `~WidgetState::Disabled`.
/// It derefs to the constraint for [`is_satisfied_by`](WidgetStatesConstraint::is_satisfied_by).
#[derive(Clone)]
pub struct WidgetStatesConstraintRef(Rc<dyn WidgetStatesConstraint>);

impl WidgetStatesConstraintRef {
    /// Erases a constraint.
    pub fn new(constraint: impl WidgetStatesConstraint) -> WidgetStatesConstraintRef {
        WidgetStatesConstraintRef(Rc::new(constraint))
    }
}

impl<C: WidgetStatesConstraint> From<C> for WidgetStatesConstraintRef {
    fn from(constraint: C) -> WidgetStatesConstraintRef {
        WidgetStatesConstraintRef::new(constraint)
    }
}

impl Deref for WidgetStatesConstraintRef {
    type Target = dyn WidgetStatesConstraint;

    fn deref(&self) -> &dyn WidgetStatesConstraint {
        &*self.0
    }
}

impl PartialEq for WidgetStatesConstraintRef {
    fn eq(&self, other: &WidgetStatesConstraintRef) -> bool {
        self.0.eq_constraint(&*other.0)
    }
}

impl Debug for WidgetStatesConstraintRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The operators of Dart's `WidgetStatesConstraint` mixin, for a type that implements it.
macro_rules! widget_states_constraint_operators {
    ($constraint:ty) => {
        impl<R: Into<WidgetStatesConstraintRef>> BitAnd<R> for $constraint {
            type Output = WidgetStatesConstraintRef;

            /// Combines two [`WidgetStatesConstraint`] values using logical "and".
            fn bitand(self, other: R) -> WidgetStatesConstraintRef {
                WidgetStatesConstraintRef::new(WidgetStateAnd {
                    first: self.into(),
                    second: other.into(),
                })
            }
        }

        impl<R: Into<WidgetStatesConstraintRef>> BitOr<R> for $constraint {
            type Output = WidgetStatesConstraintRef;

            /// Combines two [`WidgetStatesConstraint`] values using logical "or".
            fn bitor(self, other: R) -> WidgetStatesConstraintRef {
                WidgetStatesConstraintRef::new(WidgetStateOr {
                    first: self.into(),
                    second: other.into(),
                })
            }
        }

        impl Not for $constraint {
            type Output = WidgetStatesConstraintRef;

            /// Takes a [`WidgetStatesConstraint`] and applies the logical "not".
            fn not(self) -> WidgetStatesConstraintRef {
                WidgetStatesConstraintRef::new(WidgetStateNot { value: self.into() })
            }
        }
    };
}

widget_states_constraint_operators!(WidgetStatesConstraintRef);

// Dart's `_WidgetStateCombo` is the `first` / `second` pair the two combinators share.

struct WidgetStateAnd {
    first: WidgetStatesConstraintRef,
    second: WidgetStatesConstraintRef,
}

impl WidgetStatesConstraint for WidgetStateAnd {
    fn is_satisfied_by(&self, states: &WidgetStates) -> bool {
        self.first.is_satisfied_by(states) && self.second.is_satisfied_by(states)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_constraint(&self, other: &dyn WidgetStatesConstraint) -> bool {
        other
            .as_any()
            .downcast_ref::<WidgetStateAnd>()
            .is_some_and(|other| other.first == self.first && other.second == self.second)
    }
}

impl Debug for WidgetStateAnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} & {:?})", self.first, self.second)
    }
}

struct WidgetStateOr {
    first: WidgetStatesConstraintRef,
    second: WidgetStatesConstraintRef,
}

impl WidgetStatesConstraint for WidgetStateOr {
    fn is_satisfied_by(&self, states: &WidgetStates) -> bool {
        self.first.is_satisfied_by(states) || self.second.is_satisfied_by(states)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_constraint(&self, other: &dyn WidgetStatesConstraint) -> bool {
        other
            .as_any()
            .downcast_ref::<WidgetStateOr>()
            .is_some_and(|other| other.first == self.first && other.second == self.second)
    }
}

impl Debug for WidgetStateOr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} | {:?})", self.first, self.second)
    }
}

struct WidgetStateNot {
    value: WidgetStatesConstraintRef,
}

impl WidgetStatesConstraint for WidgetStateNot {
    fn is_satisfied_by(&self, states: &WidgetStates) -> bool {
        !self.value.is_satisfied_by(states)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_constraint(&self, other: &dyn WidgetStatesConstraint) -> bool {
        other
            .as_any()
            .downcast_ref::<WidgetStateNot>()
            .is_some_and(|other| other.value == self.value)
    }
}

impl Debug for WidgetStateNot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "~{:?}", self.value)
    }
}

/// A private class, used to create [`WidgetState::any`]. Every instance is equal, as Dart's
/// `const` one is canonical.
struct AnyWidgetStates;

impl WidgetStatesConstraint for AnyWidgetStates {
    fn is_satisfied_by(&self, _states: &WidgetStates) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_constraint(&self, other: &dyn WidgetStatesConstraint) -> bool {
        other.as_any().is::<AnyWidgetStates>()
    }
}

impl Debug for AnyWidgetStates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WidgetState.any")
    }
}

/// Interactive states that some of the widgets can take on when receiving input
/// from the user.
///
/// States are defined by <https://m3.material.io/foundations/interaction/states>,
/// but are not limited to the Material design system or library.
///
/// Some widgets track their current state in a [`WidgetStates`] set.
///
/// See also:
///
///  * `MaterialState`, the Material specific version of `WidgetState`.
///  * [`WidgetStateProperty`], an interface for objects that "resolve" to
///    different values depending on a widget's state.
///  * [`WidgetStateColor`], a `Color` that implements `WidgetStateProperty`
///    which is used in APIs that need to accept either a `Color` or a
///    `WidgetStateProperty<Color>`.
///  * [`WidgetStateMouseCursor`], a `MouseCursor` that implements
///    `WidgetStateProperty` which is used in APIs that need to accept either
///    a `MouseCursor` or a `WidgetStateProperty<MouseCursor>`.
///  * `WidgetStateOutlinedBorder`, an `OutlinedBorder` that implements
///    `WidgetStateProperty` which is used in APIs that need to accept either
///    an `OutlinedBorder` or a `WidgetStateProperty<OutlinedBorder>`.
///  * `WidgetStateBorderSide`, a `BorderSide` that implements
///    `WidgetStateProperty` which is used in APIs that need to accept either
///    a `BorderSide` or a `WidgetStateProperty<BorderSide>`.
///  * `WidgetStateTextStyle`, a `TextStyle` that implements
///    `WidgetStateProperty` which is used in APIs that need to accept either
///    a `TextStyle` or a `WidgetStateProperty<TextStyle>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetState {
    /// The state when the user drags their mouse cursor over the given widget.
    ///
    /// See: <https://material.io/design/interaction/states.html#hover>.
    Hovered,

    /// The state when the user navigates with the keyboard to a given widget.
    ///
    /// This can also sometimes be triggered when a widget is tapped. For example,
    /// when a `TextField` is tapped, it becomes [`Focused`](Self::Focused).
    ///
    /// See: <https://material.io/design/interaction/states.html#focus>.
    Focused,

    /// The state when the user is actively pressing down on the given widget.
    ///
    /// See: <https://material.io/design/interaction/states.html#pressed>.
    Pressed,

    /// The state when this widget is being dragged from one place to another by
    /// the user.
    ///
    /// <https://material.io/design/interaction/states.html#dragged>.
    Dragged,

    /// The state when this item has been selected.
    ///
    /// This applies to things that can be toggled (such as chips and checkboxes)
    /// and things that are selected from a set of options (such as tabs and radio buttons).
    ///
    /// See: <https://material.io/design/interaction/states.html#selected>.
    Selected,

    /// The state when this widget overlaps the content of a scrollable below.
    ///
    /// Used by `AppBar` to indicate that the primary scrollable's
    /// content has scrolled up and behind the app bar.
    ScrolledUnder,

    /// The state when this widget is disabled and cannot be interacted with.
    ///
    /// Disabled widgets should not respond to hover, focus, press, or drag
    /// interactions.
    ///
    /// See: <https://material.io/design/interaction/states.html#disabled>.
    Disabled,

    /// The state when the widget has entered some form of invalid state.
    ///
    /// See <https://material.io/design/interaction/states.html#usage>.
    Error,
}

impl WidgetState {
    /// To prevent a situation where each [`WidgetStatesConstraint`]
    /// isn't satisfied by the given set of states, consider adding
    /// [`WidgetState::any`] as the final [`WidgetStateMap`] key.
    pub fn any() -> WidgetStatesConstraintRef {
        WidgetStatesConstraintRef::new(AnyWidgetStates)
    }
}

impl WidgetStatesConstraint for WidgetState {
    fn is_satisfied_by(&self, states: &WidgetStates) -> bool {
        states.contains(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_constraint(&self, other: &dyn WidgetStatesConstraint) -> bool {
        other.as_any().downcast_ref::<WidgetState>() == Some(self)
    }
}

widget_states_constraint_operators!(WidgetState);

/// Signature for the function that returns a value of type `T` based on a given
/// set of states.
pub type WidgetPropertyResolver<T> = Rc<dyn Fn(&WidgetStates) -> T>;

/// Defines a `Color` that is also a [`WidgetStateProperty`].
///
/// This class exists to enable widgets with `Color` valued properties
/// to also accept `WidgetStateProperty<Color>` values. A widget
/// state color property represents a color which depends on
/// a widget's "interactive state". This state is represented as a
/// set of [`WidgetState`]s, like [`WidgetState::Pressed`],
/// [`WidgetState::Focused`] and [`WidgetState::Hovered`].
///
/// [`WidgetStateColor`] should only be used with widgets that document
/// their support, like `TimePickerThemeData.dayPeriodColor`.
///
/// A [`WidgetStateColor`] can be created in one of the following ways:
///   1. Use [`WidgetStateColor::resolve_with`] and pass in a callback that
///      will be used to resolve the color in the given states.
///   2. Use [`WidgetStateColor::from_map`] to assign a value using a [`WidgetStateMap`].
///
/// It is the `Color` it is in Dart once erased with [`into_any`](Self::into_any) (or
/// `.into()`): an [`AnyColor`] whose value is the color resolved in the default state (the
/// empty set of states), Dart's `super(defaultValue)`, and whose extension is this object,
/// recovered with `color.extension::<WidgetStateColor>()`.
pub struct WidgetStateColor {
    subclass: ColorSubclass,
}

/// Dart's three private subclasses of `WidgetStateColor`.
enum ColorSubclass {
    /// `_WidgetStateColor`.
    ResolveWith(WidgetPropertyResolver<Color>),
    /// `_WidgetStateColorTransparent`.
    Transparent,
    /// `_WidgetStateColorMapper`.
    Mapper(WidgetStateMapper<Color>),
}

impl WidgetStateColor {
    /// Creates a [`WidgetStateColor`] from a [`WidgetPropertyResolver<Color>`]
    /// callback function.
    ///
    /// If used as a regular color, the color resolved in the default state (the
    /// empty set of states) will be used.
    ///
    /// The given callback parameter must return a non-null color in the default
    /// state.
    pub fn resolve_with(callback: impl Fn(&WidgetStates) -> Color + 'static) -> WidgetStateColor {
        WidgetStateColor {
            subclass: ColorSubclass::ResolveWith(Rc::new(callback)),
        }
    }

    /// Creates a [`WidgetStateColor`] from a [`WidgetStateMap<Color>`].
    ///
    /// This constructor's [`resolve`](WidgetStateProperty::resolve) method finds the first
    /// entry whose key is satisfied by the set of states, and returns its associated value.
    /// It should only be used with widgets that document support for
    /// [`WidgetStateColor`] (panics if used as a normal `Color` with no key satisfied by
    /// the empty set of states).
    ///
    /// To prevent a situation where each [`WidgetStatesConstraint`]
    /// isn't satisfied by the given set of states, consider adding
    /// [`WidgetState::any`] as the final [`WidgetStateMap`] key.
    pub fn from_map(map: WidgetStateMap<Color>) -> WidgetStateColor {
        WidgetStateColor {
            subclass: ColorSubclass::Mapper(WidgetStateMapper::new(map)),
        }
    }

    /// A constant whose value is transparent for all states.
    pub fn transparent() -> WidgetStateColor {
        WidgetStateColor {
            subclass: ColorSubclass::Transparent,
        }
    }

    /// The color as the `Color` it is in Dart: the default-state value, carrying this
    /// object as the extension.
    pub fn into_any(self) -> AnyColor {
        let default_value = self.resolve(&WidgetStates::new());
        AnyColor::with_shared(default_value, Rc::new(self))
    }
}

impl WidgetStateProperty<Color> for WidgetStateColor {
    /// Returns a `Color` that's to be used when a component is in the specified
    /// state.
    fn resolve(&self, states: &WidgetStates) -> Color {
        match &self.subclass {
            ColorSubclass::ResolveWith(resolve) => resolve(states),
            ColorSubclass::Transparent => Color::new(0x00000000),
            ColorSubclass::Mapper(mapper) => mapper.resolve(states),
        }
    }
}

impl ColorExtension for WidgetStateColor {
    fn as_any(&self) -> &dyn Any {
        self
    }

    // Dart's `Color.==` compares runtime types (values are already equal here); the mapper
    // subclass compares maps instead.
    fn eq_extension(&self, other: &dyn ColorExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<WidgetStateColor>()
            .is_some_and(|other| match (&self.subclass, &other.subclass) {
                (ColorSubclass::ResolveWith(_), ColorSubclass::ResolveWith(_))
                | (ColorSubclass::Transparent, ColorSubclass::Transparent) => true,
                (ColorSubclass::Mapper(mine), ColorSubclass::Mapper(theirs)) => mine == theirs,
                _ => false,
            })
    }
}

impl Debug for WidgetStateColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.subclass {
            ColorSubclass::ResolveWith(_) => f.write_str("WidgetStateColor.resolveWith"),
            ColorSubclass::Transparent => f.write_str("WidgetStateColor.transparent"),
            ColorSubclass::Mapper(mapper) => mapper.fmt(f),
        }
    }
}

impl From<WidgetStateColor> for AnyColor {
    fn from(color: WidgetStateColor) -> AnyColor {
        color.into_any()
    }
}

impl MaybeWidgetStateProperty<Color> for AnyColor {
    fn as_widget_state_property(&self) -> Option<&dyn WidgetStateProperty<Color>> {
        self.extension::<WidgetStateColor>()
            .map(|color| color as &dyn WidgetStateProperty<Color>)
    }

    fn from_resolved(value: Color) -> AnyColor {
        AnyColor::new(value)
    }
}

/// Defines a `MouseCursor` whose value depends on a set of [`WidgetState`]s which
/// represent the interactive state of a component.
///
/// This kind of `MouseCursor` is useful when the set of interactive
/// actions a widget supports varies with its state. For example, a
/// mouse pointer hovering over a disabled `ListTile` should not
/// display [`SystemMouseCursors::CLICK`], since a disabled list tile
/// doesn't respond to mouse clicks. `ListTile`'s default mouse cursor
/// is a [`WidgetStateMouseCursor::clickable`], which resolves to
/// [`SystemMouseCursors::BASIC`] when the button is disabled.
///
/// This class should only be used for parameters that document their support
/// for [`WidgetStateMouseCursor`].
///
/// A [`WidgetStateMouseCursor`] can be created in one of the following ways:
///   1. Use [`WidgetStateMouseCursor::resolve_with`] and pass in a callback that
///      will be used to resolve the color in the given states.
///   2. Use [`WidgetStateMouseCursor::from_map`] to assign a value using a [`WidgetStateMap`].
///
/// This example defines a mouse cursor that resolves to
/// [`SystemMouseCursors::FORBIDDEN`] when its widget is disabled.
///
/// ```text
/// let cursor = WidgetStateMouseCursor::resolve_with(
///     |states| {
///         if states.contains(&WidgetState::Disabled) {
///             return SystemMouseCursors::FORBIDDEN.into();
///         }
///         SystemMouseCursors::CLICK.into()
///     },
///     None,
/// );
/// ```
///
/// It is the `MouseCursor` it is in Dart once erased with `.into()`: a [`MouseCursorRef`]
/// recovered with `cursor.as_any().downcast_ref::<WidgetStateMouseCursor>()`.
///
/// See also:
///
///  * [`MouseCursor`] for introduction on the mouse cursor system.
///  * [`SystemMouseCursors`], which defines cursors that are supported by
///    native platforms.
pub struct WidgetStateMouseCursor {
    subclass: MouseCursorSubclass,
}

/// Dart's two private subclasses of `WidgetStateMouseCursor`.
enum MouseCursorSubclass {
    /// `_WidgetStateMouseCursor`.
    ResolveWith {
        resolve: WidgetPropertyResolver<MouseCursorRef>,
        debug_description: String,
    },
    /// `_WidgetMouseCursorMapper`.
    Mapper(WidgetStateMapper<MouseCursorRef>),
}

impl WidgetStateMouseCursor {
    /// Creates a [`WidgetStateMouseCursor`] using a [`WidgetPropertyResolver`]
    /// callback.
    ///
    /// A `debug_description` may optionally be provided (Dart's default is
    /// `'WidgetStateMouseCursor()'`).
    ///
    /// If used as a regular `MouseCursor`, the cursor resolved
    /// in the default state (the empty set of states) will be used.
    pub fn resolve_with(
        callback: impl Fn(&WidgetStates) -> MouseCursorRef + 'static,
        debug_description: Option<&str>,
    ) -> WidgetStateMouseCursor {
        WidgetStateMouseCursor {
            subclass: MouseCursorSubclass::ResolveWith {
                resolve: Rc::new(callback),
                debug_description: debug_description
                    .unwrap_or("WidgetStateMouseCursor()")
                    .to_owned(),
            },
        }
    }

    /// Creates a [`WidgetStateMouseCursor`] from a [`WidgetStateMap`].
    ///
    /// This constructor's [`resolve`](WidgetStateProperty::resolve) method finds the first
    /// entry whose key is satisfied by the set of states, and returns its associated value.
    /// It should only be used with classes that document support for
    /// [`WidgetStateMouseCursor`] (panics if used as a regular
    /// `MouseCursor` with no key satisfied by the empty set of states).
    pub fn from_map(map: WidgetStateMap<MouseCursorRef>) -> WidgetStateMouseCursor {
        WidgetStateMouseCursor {
            subclass: MouseCursorSubclass::Mapper(WidgetStateMapper::new(map)),
        }
    }

    /// A mouse cursor for clickable widgets, which resolves differently when the
    /// widget is disabled.
    ///
    /// By default this cursor resolves to [`SystemMouseCursors::CLICK`]. If the widget is
    /// disabled, the cursor resolves to [`SystemMouseCursors::BASIC`].
    pub fn clickable() -> WidgetStateMouseCursor {
        WidgetStateMouseCursor::resolve_with(
            Self::resolve_clickable,
            Some("WidgetStateMouseCursor(clickable)"),
        )
    }

    fn resolve_clickable(states: &WidgetStates) -> MouseCursorRef {
        if states.contains(&WidgetState::Disabled) {
            return SystemMouseCursors::BASIC.into();
        }
        SystemMouseCursors::CLICK.into()
    }

    /// A platform-adaptive mouse cursor for clickable widgets, which resolves
    /// differently based on the widget's state and the platform.
    ///
    /// On web platforms, this cursor resolves to [`SystemMouseCursors::CLICK`] by
    /// default. If the widget is disabled, it resolves to
    /// [`SystemMouseCursors::BASIC`].
    ///
    /// On non-web platforms, this cursor always resolves to
    /// [`SystemMouseCursors::BASIC`].
    ///
    /// This cursor is commonly used for interactive widgets like buttons. The
    /// difference in behavior across platforms reflects native conventions (e.g.,
    /// web uses a hand pointer for buttons, while desktop platforms do not).
    pub fn adaptive_clickable() -> WidgetStateMouseCursor {
        WidgetStateMouseCursor::resolve_with(
            Self::resolve_adaptive_clickable,
            Some("WidgetStateMouseCursor(adaptiveClickable)"),
        )
    }

    fn resolve_adaptive_clickable(states: &WidgetStates) -> MouseCursorRef {
        if states.contains(&WidgetState::Disabled) {
            return SystemMouseCursors::BASIC.into();
        }
        if K_IS_WEB {
            SystemMouseCursors::CLICK.into()
        } else {
            SystemMouseCursors::BASIC.into()
        }
    }

    /// A mouse cursor for widgets related to text, which resolves differently
    /// when the widget is disabled.
    ///
    /// By default this cursor resolves to [`SystemMouseCursors::TEXT`]. If the widget is
    /// disabled, the cursor resolves to [`SystemMouseCursors::BASIC`].
    ///
    /// This cursor is the default for many widgets.
    pub fn textable() -> WidgetStateMouseCursor {
        WidgetStateMouseCursor::resolve_with(
            Self::resolve_textable,
            Some("WidgetStateMouseCursor(textable)"),
        )
    }

    fn resolve_textable(states: &WidgetStates) -> MouseCursorRef {
        if states.contains(&WidgetState::Disabled) {
            return SystemMouseCursors::BASIC.into();
        }
        SystemMouseCursors::TEXT.into()
    }
}

impl MouseCursor for WidgetStateMouseCursor {
    fn create_session(&self, device: i64) -> Box<dyn MouseCursorSession> {
        self.resolve(&WidgetStates::new()).create_session(device)
    }

    fn debug_description(&self) -> String {
        match &self.subclass {
            MouseCursorSubclass::ResolveWith {
                debug_description, ..
            } => debug_description.clone(),
            MouseCursorSubclass::Mapper(mapper) => format!("{mapper:?}"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    // Identity, except that the mapper subclass compares maps (Dart's `WidgetStateMapper.==`).
    fn eq_cursor(&self, other: &dyn MouseCursor) -> bool {
        match (
            &self.subclass,
            other.as_any().downcast_ref::<WidgetStateMouseCursor>(),
        ) {
            (
                MouseCursorSubclass::Mapper(mine),
                Some(WidgetStateMouseCursor {
                    subclass: MouseCursorSubclass::Mapper(theirs),
                }),
            ) => mine == theirs,
            _ => std::ptr::addr_eq(self as *const Self, other as *const dyn MouseCursor),
        }
    }
}

impl WidgetStateProperty<MouseCursorRef> for WidgetStateMouseCursor {
    /// Returns a `MouseCursor` that's to be used when a component is in the
    /// specified state.
    fn resolve(&self, states: &WidgetStates) -> MouseCursorRef {
        match &self.subclass {
            MouseCursorSubclass::ResolveWith { resolve, .. } => resolve(states),
            MouseCursorSubclass::Mapper(mapper) => mapper.resolve(states),
        }
    }
}

impl Debug for WidgetStateMouseCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.debug_description())
    }
}

impl From<WidgetStateMouseCursor> for MouseCursorRef {
    fn from(cursor: WidgetStateMouseCursor) -> MouseCursorRef {
        Rc::new(cursor)
    }
}

impl MaybeWidgetStateProperty<MouseCursorRef> for MouseCursorRef {
    fn as_widget_state_property(&self) -> Option<&dyn WidgetStateProperty<MouseCursorRef>> {
        self.as_any()
            .downcast_ref::<WidgetStateMouseCursor>()
            .map(|cursor| cursor as &dyn WidgetStateProperty<MouseCursorRef>)
    }

    fn from_resolved(value: MouseCursorRef) -> MouseCursorRef {
        value
    }
}

// `WidgetStateBorderSide`, `_LerpSides`, `WidgetStateOutlinedBorder`, and `WidgetStateTextStyle`
// wait; see the crate's PORTING.md.

/// Interface for classes that [`resolve`] to a value of type `T` based
/// on a widget's interactive "state", which is defined as a set
/// of [`WidgetState`]s.
///
/// Widget state properties represent values that depend on a widget's "state".
/// The state is encoded as a set of [`WidgetState`] values, like
/// [`WidgetState::Focused`], [`WidgetState::Hovered`], [`WidgetState::Pressed`]. For
/// example the `InkWell.overlayColor` defines the color that fills the ink well
/// when it's pressed (the "splash color"), focused, or hovered. The `InkWell`
/// uses the overlay color's [`resolve`] method to compute the color for the
/// ink well's current state.
///
/// `ButtonStyle`, which is used to configure the appearance of
/// buttons like `TextButton`, `ElevatedButton`, and `OutlinedButton`,
/// has many material state properties. The button widgets keep track
/// of their current material state and [`resolve`] the button style's
/// material state properties when their value is needed.
///
/// ## Performance Consideration
///
/// In order for constructed [`WidgetStateProperty`] objects to be recognized as
/// equivalent, they need to have overrides for `==`.
///
/// This comes into play when, for instance, two `ThemeData` objects are being
/// compared for equality.
///
/// For a concrete `WidgetStateProperty` object that supports stable
/// equality checks, consider using [`WidgetStateMapper`].
///
/// A property is stored erased as a [`WidgetStatePropertyRef`]; Dart's static members
/// (`from_map`, `resolve_as`, `resolve_with`, `all`, `lerp`) are on
/// `<dyn WidgetStateProperty<T>>`.
///
/// See also:
///
///  * `MaterialStateProperty`, the Material specific version of
///    `WidgetStateProperty`.
///  * [`WidgetStateColor`] and [`WidgetStateMouseCursor`], the value types that
///    implement it.
///
/// [`resolve`]: WidgetStateProperty::resolve
pub trait WidgetStateProperty<T>: Debug {
    /// Returns a value of type `T` that depends on `states`.
    ///
    /// Widgets like `TextButton` and `ElevatedButton` apply this method to their
    /// current [`WidgetState`]s to compute colors and other visual parameters
    /// at build time.
    fn resolve(&self, states: &WidgetStates) -> T;
}

/// A shared, erased [`WidgetStateProperty`] (a Dart object reference).
pub type WidgetStatePropertyRef<T> = Rc<dyn WidgetStateProperty<T>>;

/// A value that may be a [`WidgetStateProperty`] in disguise: Dart's
/// `value is WidgetStateProperty<T>` in `<dyn WidgetStateProperty<T>>::resolve_as`.
///
/// [`AnyColor`] answers through its [`WidgetStateColor`] extension, [`MouseCursorRef`]
/// through `as_any`, and an `Option` of either through its content (Dart's `T?`).
pub trait MaybeWidgetStateProperty<T>: Clone {
    /// The property this value is, if it is one.
    fn as_widget_state_property(&self) -> Option<&dyn WidgetStateProperty<T>>;

    /// A resolved value as this type (Dart's `T` is both).
    fn from_resolved(value: T) -> Self;
}

impl<T, V: MaybeWidgetStateProperty<T>> MaybeWidgetStateProperty<T> for Option<V> {
    fn as_widget_state_property(&self) -> Option<&dyn WidgetStateProperty<T>> {
        self.as_ref().and_then(V::as_widget_state_property)
    }

    fn from_resolved(value: T) -> Option<V> {
        Some(V::from_resolved(value))
    }
}

impl<T: 'static> dyn WidgetStateProperty<T> {
    /// Creates a property that resolves using a [`WidgetStateMap`].
    ///
    /// This constructor's [`resolve`](WidgetStateProperty::resolve) method finds the first
    /// entry whose key is satisfied by the set of states, and returns its associated value.
    ///
    /// Panics if no keys match (see [`WidgetStateMapper`]).
    /// To prevent a situation where each [`WidgetStatesConstraint`]
    /// isn't satisfied by the given set of states, consider adding
    /// [`WidgetState::any`] as the final [`WidgetStateMap`] key.
    pub fn from_map(map: WidgetStateMap<T>) -> WidgetStatePropertyRef<T>
    where
        T: Clone + Debug,
    {
        Rc::new(WidgetStateMapper::new(map))
    }

    /// Resolves the value for the given set of states if `value` is a
    /// [`WidgetStateProperty`], otherwise returns the value itself.
    ///
    /// This is useful for widgets that have parameters which can optionally be a
    /// [`WidgetStateProperty`]. For example, `InkWell.mouseCursor` can be a
    /// `MouseCursor` or a `WidgetStateProperty<MouseCursor>`.
    ///
    /// Dart's `T` is the value's own type here (`AnyColor` for a `Color`,
    /// [`MouseCursorRef`] for a `MouseCursor`, or an `Option` of one):
    /// `<dyn WidgetStateProperty<Color>>::resolve_as(&color, &states)`.
    pub fn resolve_as<V: MaybeWidgetStateProperty<T>>(value: &V, states: &WidgetStates) -> V {
        if let Some(property) = value.as_widget_state_property() {
            return V::from_resolved(property.resolve(states));
        }
        value.clone()
    }

    /// Convenience method for creating a [`WidgetStateProperty`] from a
    /// [`WidgetPropertyResolver`] function alone.
    pub fn resolve_with(
        callback: impl Fn(&WidgetStates) -> T + 'static,
    ) -> WidgetStatePropertyRef<T> {
        Rc::new(WidgetStatePropertyWith {
            resolve: Rc::new(callback),
        })
    }

    /// Convenience method for creating a [`WidgetStateProperty`] that resolves
    /// to a single value for all states.
    ///
    /// Prefer using [`WidgetStatePropertyAll`] directly.
    pub fn all(value: T) -> WidgetStatePropertyRef<T>
    where
        T: Clone + Debug,
    {
        Rc::new(WidgetStatePropertyAll::new(value))
    }

    /// Linearly interpolate between two [`WidgetStateProperty`]s.
    pub fn lerp(
        a: Option<WidgetStatePropertyRef<T>>,
        b: Option<WidgetStatePropertyRef<T>>,
        t: f64,
        lerp_function: impl Fn(Option<T>, Option<T>, f64) -> Option<T> + 'static,
    ) -> Option<WidgetStatePropertyRef<Option<T>>> {
        // Avoid creating a LerpProperties object for a common case.
        if a.is_none() && b.is_none() {
            return None;
        }
        Some(Rc::new(LerpProperties {
            a,
            b,
            t,
            lerp_function: Rc::new(lerp_function),
        }))
    }
}

/// Dart's `T? Function(T?, T?, double)` argument of `WidgetStateProperty.lerp`.
type LerpFunction<T> = Rc<dyn Fn(Option<T>, Option<T>, f64) -> Option<T>>;

struct LerpProperties<T> {
    a: Option<WidgetStatePropertyRef<T>>,
    b: Option<WidgetStatePropertyRef<T>>,
    t: f64,
    lerp_function: LerpFunction<T>,
}

impl<T: 'static> WidgetStateProperty<Option<T>> for LerpProperties<T> {
    fn resolve(&self, states: &WidgetStates) -> Option<T> {
        let resolved_a = self.a.as_ref().map(|a| a.resolve(states));
        let resolved_b = self.b.as_ref().map(|b| b.resolve(states));
        (self.lerp_function)(resolved_a, resolved_b, self.t)
    }
}

impl<T> Debug for LerpProperties<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LerpProperties({:?}, {:?}, {})", self.a, self.b, self.t)
    }
}

struct WidgetStatePropertyWith<T> {
    resolve: WidgetPropertyResolver<T>,
}

impl<T> WidgetStateProperty<T> for WidgetStatePropertyWith<T> {
    fn resolve(&self, states: &WidgetStates) -> T {
        (self.resolve)(states)
    }
}

impl<T> Debug for WidgetStatePropertyWith<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WidgetStateProperty.resolveWith")
    }
}

/// A map used to resolve to a single value of type `T` based on
/// the current set of Widget states.
///
/// Dart's `Map<WidgetStatesConstraint, T>`, kept as its entries in insertion order.
///
/// Example:
///
/// ```text
/// // This WidgetStateMap<Option<Color>> resolves to None if no keys match.
/// <dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
///     (WidgetState::Error.into(), Some(RED)),
///     (WidgetState::Hovered & WidgetState::Focused, Some(BLUE_ACCENT)),
///     (WidgetState::Focused.into(), Some(BLUE)),
///     (~WidgetState::Disabled, Some(BLACK)),
///     (WidgetState::any(), None),
/// ]);
///
/// // The same can be accomplished with a WidgetPropertyResolver,
/// // but it's more verbose:
/// <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
///     if states.contains(&WidgetState::Error) {
///         Some(RED)
///     } else if states.contains(&WidgetState::Hovered) && states.contains(&WidgetState::Focused) {
///         Some(BLUE_ACCENT)
///     } else if states.contains(&WidgetState::Focused) {
///         Some(BLUE)
///     } else if !states.contains(&WidgetState::Disabled) {
///         Some(BLACK)
///     } else {
///         None
///     }
/// });
/// ```
///
/// A widget state combination can be stored in a variable,
/// and [`WidgetState::any`] can be used for non-nullable types to ensure
/// that there's a match:
///
/// ```text
/// let selected_error = WidgetState::Selected & WidgetState::Error;
///
/// let color = <dyn WidgetStateProperty<Color>>::from_map(vec![
///     (selected_error.clone() & WidgetState::Hovered, RED_ACCENT),
///     (selected_error, RED),
///     (WidgetState::any(), BLACK),
/// ]);
///
/// // The (more verbose) WidgetPropertyResolver implementation:
/// let color_resolve_with = <dyn WidgetStateProperty<Color>>::resolve_with(|states| {
///     if states.contains(&WidgetState::Selected) && states.contains(&WidgetState::Error) {
///         if states.contains(&WidgetState::Hovered) {
///             return RED_ACCENT;
///         }
///         return RED;
///     }
///     BLACK
/// });
/// ```
pub type WidgetStateMap<T> = Vec<(WidgetStatesConstraintRef, T)>;

/// Uses a [`WidgetStateMap`] to resolve to a single value of type `T` based on
/// the current set of Widget states.
///
/// Classes that hold a [`WidgetStateMapper`] can implement any other interface,
/// but should only be used for fields that document their support for
/// [`WidgetStateProperty`] objects.
///
/// For example, a [`WidgetStateColor::from_map`] object can be passed anywhere that
/// accepts either a `Color` or a [`WidgetStateProperty`] object.
///
/// [`resolve`](WidgetStateProperty::resolve) panics when no key is satisfied, whatever
/// `T`; a map that should resolve to `None` ends with `(WidgetState::any(), None)`.
#[derive(Clone)]
pub struct WidgetStateMapper<T> {
    map: WidgetStateMap<T>,
}

impl<T> WidgetStateMapper<T> {
    /// Creates a [`WidgetStateProperty`] object that can resolve
    /// to a value of type `T` using the provided `map`.
    pub fn new(map: WidgetStateMap<T>) -> WidgetStateMapper<T> {
        WidgetStateMapper { map }
    }
}

impl<T: Clone + Debug + 'static> WidgetStateProperty<T> for WidgetStateMapper<T> {
    fn resolve(&self, states: &WidgetStates) -> T {
        for (key, value) in &self.map {
            if key.is_satisfied_by(states) {
                return value.clone();
            }
        }

        panic!(
            "The current set of widget states is {states:?}.\n\
             None of the provided map keys matched this set, \
             and the type \"{t}\" is non-nullable.\n\
             Consider using \"WidgetStateMapper<Option<{t}>>\" with a final \
             \"WidgetState::any()\" key mapped to None, \
             or adding the \"WidgetState::any()\" key to this map.",
            t = type_name::<T>()
        );
    }
}

impl<T: PartialEq> PartialEq for WidgetStateMapper<T> {
    // Dart's `mapEquals`: the same entries, looked up by key equality.
    fn eq(&self, other: &WidgetStateMapper<T>) -> bool {
        self.map.len() == other.map.len()
            && self.map.iter().all(|(key, value)| {
                other
                    .map
                    .iter()
                    .any(|(other_key, other_value)| other_key == key && other_value == value)
            })
    }
}

impl<T: Debug> Debug for WidgetStateMapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetStateMapper<{}>({:?})", type_name::<T>(), self.map)
    }
}

/// Convenience class for creating a [`WidgetStateProperty`] that
/// resolves to the given value for all states.
///
/// See also:
///
///  * `MaterialStatePropertyAll`, the Material specific version of
///    `WidgetStatePropertyAll`.
#[derive(Clone, PartialEq)]
pub struct WidgetStatePropertyAll<T> {
    /// The value of the property that will be used for all states.
    pub value: T,
}

impl<T> WidgetStatePropertyAll<T> {
    /// Constructs a [`WidgetStateProperty`] that always resolves to the given
    /// value.
    pub const fn new(value: T) -> WidgetStatePropertyAll<T> {
        WidgetStatePropertyAll { value }
    }
}

impl<T: Clone + Debug> WidgetStateProperty<T> for WidgetStatePropertyAll<T> {
    fn resolve(&self, _states: &WidgetStates) -> T {
        self.value.clone()
    }
}

impl<T: Debug> Debug for WidgetStatePropertyAll<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetStatePropertyAll({:?})", self.value)
    }
}

/// Manages a set of [`WidgetState`]s and notifies listeners of changes.
///
/// Used by widgets that expose their internal state for the sake of
/// extensions that add support for additional states. See
/// `TextButton` for an example.
///
/// The controller's [`value`] is its current set of states. Listeners
/// are notified whenever the [`value`] changes. The [`value`] should only be
/// changed with [`update`]; it should not be modified directly.
///
/// The controller's [`value`] represents the set of states that a
/// widget's visual properties, typically [`WidgetStateProperty`]
/// values, are resolved against. It is _not_ the intrinsic state of
/// the widget. The widget is responsible for ensuring that the
/// controller's [`value`] tracks its intrinsic state. For example one
/// cannot request the keyboard focus for a widget by adding
/// [`WidgetState::Focused`] to its controller. When the widget gains the
/// or loses the focus it will [`update`] its controller's [`value`] and
/// notify listeners of the change.
///
/// When calling `set_state` in a [`WidgetStatesController`] listener, use the
/// `SchedulerBinding::add_post_frame_callback` to delay the call to `set_state` after
/// the frame has been rendered. It's generally prudent to use the
/// `SchedulerBinding::add_post_frame_callback` because some of the widgets that
/// depend on [`WidgetStatesController`] may call [`update`] in their build method.
/// In such cases, listener's that call `set_state` - during the build phase - will cause
/// an error.
///
/// Dart's `ValueNotifier<Set<WidgetState>>` superclass is the `change_notifier` and `value`
/// fields here, with its [`value`] / [`set_value`] / [`dispose`]; the handle is a
/// `Listenable`.
///
/// See also:
///
///  * `MaterialStatesController`, the Material specific version of
///    `WidgetStatesController`.
///
/// [`value`]: WidgetStatesController::value
/// [`set_value`]: WidgetStatesController::set_value
/// [`dispose`]: WidgetStatesController::dispose
/// [`update`]: WidgetStatesController::update
#[derive(Debug)]
pub struct WidgetStatesController {
    change_notifier: ChangeNotifierData,
    value: WidgetStates,
}

impl WidgetStatesController {
    /// Creates a WidgetStatesController.
    pub fn new(app: &mut App, value: Option<WidgetStates>) -> Handle<WidgetStatesController> {
        app.create(WidgetStatesController {
            change_notifier: ChangeNotifierData::new(),
            value: value.unwrap_or_default(),
        })
    }

    /// The current set of states.
    pub fn value(&self) -> &WidgetStates {
        &self.value
    }

    /// Replaces the current set of states, notifying listeners when it differs
    /// (`ValueNotifier`'s setter). Prefer [`update`](Self::update).
    pub fn set_value(self: Handle<Self>, app: &mut App, new_value: WidgetStates) {
        if app.get(self).value == new_value {
            return;
        }
        app.get_mut(self).value = new_value;
        self.notify_listeners(app);
    }

    /// Discards any resources used by the object.
    pub fn dispose(&mut self) {
        self.change_notifier.dispose();
    }

    /// Adds `state` to [`value`](Self::value) if `add` is true, and removes it otherwise,
    /// and notifies listeners if [`value`](Self::value) has changed.
    pub fn update(self: Handle<Self>, app: &mut App, state: WidgetState, add: bool) {
        let value = &mut app.get_mut(self).value;
        let value_changed = if add {
            value.insert(state)
        } else {
            value.remove(&state)
        };
        if value_changed {
            self.notify_listeners(app);
        }
    }
}

impl ChangeNotifier for WidgetStatesController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use reveal_foundation::{Listenable, Listener};
    use reveal_painting::TextStyle;

    use super::*;

    const WHITE: Color = Color::new(0xFFFFFFFF);
    const BLACK: Color = Color::new(0xFF000000);
    const RED: Color = Color::new(0xFFFF0000);

    fn states(states: impl IntoIterator<Item = WidgetState>) -> WidgetStates {
        states.into_iter().collect()
    }

    fn cursor(cursor: MouseCursorRef) -> MouseCursorRef {
        cursor
    }

    // widget_state_property_test.dart 'WidgetStateProperty.resolveWith()'.
    #[test]
    fn resolve_with_resolves_through_the_callback() {
        let value = <dyn WidgetStateProperty<WidgetState>>::resolve_with(|states| {
            states.iter().next().copied().expect("one state")
        });
        for state in [
            WidgetState::Hovered,
            WidgetState::Focused,
            WidgetState::Pressed,
            WidgetState::Dragged,
            WidgetState::Selected,
            WidgetState::Disabled,
            WidgetState::Error,
        ] {
            assert_eq!(value.resolve(&states([state])), state);
        }
    }

    // widget_state_property_test.dart 'WidgetStateProperty.map()'.
    #[test]
    fn a_map_resolves_to_the_first_satisfied_key_through_the_combinators() {
        let active = WidgetState::Hovered | WidgetState::Focused | WidgetState::Pressed;
        let value = <dyn WidgetStateProperty<Option<&str>>>::from_map(vec![
            (active.clone() & WidgetState::Error, Some("active error")),
            (
                WidgetState::Disabled | WidgetState::Error,
                Some("kinda sus"),
            ),
            (
                !(WidgetState::Dragged | WidgetState::Selected) & !active.clone(),
                Some("this is boring"),
            ),
            (active, Some("active")),
            (WidgetState::any(), None),
        ]);
        assert_eq!(
            value.resolve(&states([WidgetState::Focused, WidgetState::Error])),
            Some("active error")
        );
        assert_eq!(
            value.resolve(&states([WidgetState::ScrolledUnder])),
            Some("this is boring")
        );
        assert_eq!(
            value.resolve(&states([WidgetState::Disabled])),
            Some("kinda sus")
        );
        assert_eq!(
            value.resolve(&states([WidgetState::Hovered])),
            Some("active")
        );
        assert_eq!(value.resolve(&states([WidgetState::Dragged])), None);
    }

    #[test]
    fn any_is_satisfied_by_every_set_and_a_constraint_prints_as_dart_does() {
        assert!(WidgetState::any().is_satisfied_by(&WidgetStates::new()));
        assert!(WidgetState::any().is_satisfied_by(&states([WidgetState::Disabled])));
        assert_eq!(WidgetState::any(), WidgetState::any());
        assert_eq!(format!("{:?}", WidgetState::any()), "WidgetState.any");
        assert_eq!(
            format!("{:?}", !(WidgetState::Hovered & WidgetState::Focused)),
            "~(Hovered & Focused)"
        );
        assert_eq!(
            WidgetState::Hovered | WidgetState::Focused,
            WidgetState::Hovered | WidgetState::Focused
        );
        assert_ne!(
            WidgetState::Hovered | WidgetState::Focused,
            WidgetState::Focused | WidgetState::Hovered
        );
        assert_ne!(
            WidgetState::Hovered | WidgetState::Focused,
            WidgetState::Hovered & WidgetState::Focused
        );
    }

    #[test]
    #[should_panic(expected = "None of the provided map keys matched this set")]
    fn a_map_with_no_satisfied_key_panics_with_darts_message() {
        let value = WidgetStateMapper::new(vec![(WidgetState::Hovered.into(), 1)]);
        value.resolve(&states([WidgetState::Focused]));
    }

    // widget_state_property_test.dart 'WidgetStateProperty.all()' and 'WidgetStatePropertyAll'.
    #[test]
    fn all_resolves_to_its_value_for_every_state() {
        let value = <dyn WidgetStateProperty<i32>>::all(123);
        let constant = WidgetStatePropertyAll::new(123);
        assert_eq!(constant.resolve(&WidgetStates::new()), 123);
        for state in [
            WidgetState::Hovered,
            WidgetState::Focused,
            WidgetState::Pressed,
            WidgetState::Dragged,
            WidgetState::Selected,
            WidgetState::Disabled,
            WidgetState::Error,
        ] {
            assert_eq!(value.resolve(&states([state])), 123);
            assert_eq!(constant.resolve(&states([state])), 123);
        }
        assert_eq!(
            format!("{:?}", WidgetStatePropertyAll::new(Some(WHITE))),
            format!("WidgetStatePropertyAll({:?})", Some(WHITE))
        );
        assert_eq!(
            WidgetStatePropertyAll::new(1),
            WidgetStatePropertyAll::new(1)
        );
        assert_ne!(
            WidgetStatePropertyAll::new(1),
            WidgetStatePropertyAll::new(2)
        );
    }

    // widget_state_property_test.dart "Can interpolate between two WidgetStateProperty's".
    #[test]
    fn lerp_interpolates_the_resolved_values() {
        let enabled = WidgetStates::new();
        let text_style_1: WidgetStatePropertyRef<TextStyle> = Rc::new(WidgetStatePropertyAll::new(
            TextStyle::new().font_size(14.0),
        ));
        let text_style_2: WidgetStatePropertyRef<TextStyle> = Rc::new(WidgetStatePropertyAll::new(
            TextStyle::new().font_size(20.0),
        ));
        let lerp = |t: f64| {
            <dyn WidgetStateProperty<TextStyle>>::lerp(
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
        assert!(
            <dyn WidgetStateProperty<TextStyle>>::lerp(None, None, 0.5, |_, _, _| None).is_none()
        );
    }

    #[test]
    fn a_widget_state_color_resolves_per_state_and_travels_through_a_text_style() {
        let color = WidgetStateColor::resolve_with(|states| {
            if states.contains(&WidgetState::Pressed) {
                return RED;
            }
            BLACK
        });
        assert_eq!(color.resolve(&states([WidgetState::Pressed])), RED);
        assert_eq!(color.resolve(&WidgetStates::new()), BLACK);

        let style = TextStyle::new().color(color);
        let color = style.color.clone().expect("set above");
        assert_eq!(color.color(), BLACK, "the default-state value is the Color");
        let property = color
            .extension::<WidgetStateColor>()
            .expect("the extension survives the style");
        assert_eq!(property.resolve(&states([WidgetState::Pressed])), RED);

        assert_eq!(
            <dyn WidgetStateProperty<Color>>::resolve_as(&color, &states([WidgetState::Pressed])),
            AnyColor::new(RED)
        );
        assert_eq!(
            <dyn WidgetStateProperty<Color>>::resolve_as(
                &Some(color),
                &states([WidgetState::Pressed])
            ),
            Some(AnyColor::new(RED))
        );
    }

    #[test]
    fn resolve_as_returns_a_plain_color_unchanged() {
        let plain = AnyColor::new(RED);
        assert_eq!(
            <dyn WidgetStateProperty<Color>>::resolve_as(&plain, &states([WidgetState::Pressed])),
            plain
        );
        assert_eq!(
            <dyn WidgetStateProperty<Color>>::resolve_as(
                &None::<AnyColor>,
                &states([WidgetState::Pressed])
            ),
            None
        );
    }

    #[test]
    fn transparent_resolves_transparent_and_a_from_map_color_resolves_its_first_key() {
        let transparent = WidgetStateColor::transparent().into_any();
        assert_eq!(transparent.color(), Color::new(0x00000000));
        assert_eq!(
            transparent
                .extension::<WidgetStateColor>()
                .expect("a WidgetStateColor")
                .resolve(&states([WidgetState::Pressed])),
            Color::new(0x00000000)
        );
        assert_eq!(transparent, WidgetStateColor::transparent().into_any());

        let color = WidgetStateColor::from_map(vec![
            (WidgetState::Focused | WidgetState::Hovered, WHITE),
            (WidgetState::any(), BLACK),
        ]);
        assert_eq!(color.resolve(&states([WidgetState::Hovered])), WHITE);
        assert_eq!(color.resolve(&states([WidgetState::Pressed])), BLACK);
        assert_eq!(color.into_any().color(), BLACK);
    }

    // widget_state_property_test.dart '.fromMap() constructors perform accurate equality checks'.
    #[test]
    fn from_map_constructors_perform_accurate_equality_checks() {
        let color_1 = WidgetStateColor::from_map(vec![
            (WidgetState::Focused | WidgetState::Hovered, WHITE),
            (WidgetState::any(), BLACK),
        ])
        .into_any();
        let color_2 = WidgetStateColor::from_map(vec![
            (WidgetState::Focused | WidgetState::Hovered, WHITE),
            (WidgetState::any(), BLACK),
        ])
        .into_any();
        let color_3 = WidgetStateColor::from_map(vec![
            (WidgetState::Focused | WidgetState::Hovered, BLACK),
            (WidgetState::any(), WHITE),
        ])
        .into_any();
        assert!(color_1 == color_2);
        assert!(color_1 != color_3);

        let cursor_1 = cursor(
            WidgetStateMouseCursor::from_map(vec![
                (
                    WidgetState::Focused | WidgetState::Hovered,
                    <dyn MouseCursor>::defer(),
                ),
                (WidgetState::any(), <dyn MouseCursor>::uncontrolled()),
            ])
            .into(),
        );
        let cursor_2 = cursor(
            WidgetStateMouseCursor::from_map(vec![
                (
                    WidgetState::Focused | WidgetState::Hovered,
                    <dyn MouseCursor>::defer(),
                ),
                (WidgetState::any(), <dyn MouseCursor>::uncontrolled()),
            ])
            .into(),
        );
        let cursor_3 = cursor(
            WidgetStateMouseCursor::from_map(vec![
                (
                    WidgetState::Focused | WidgetState::Hovered,
                    <dyn MouseCursor>::uncontrolled(),
                ),
                (WidgetState::any(), <dyn MouseCursor>::defer()),
            ])
            .into(),
        );
        assert!(*cursor_1 == *cursor_2);
        assert!(*cursor_1 != *cursor_3);
    }

    #[test]
    fn clickable_resolves_to_basic_when_disabled_and_is_a_mouse_cursor() {
        let clickable = WidgetStateMouseCursor::clickable();
        assert!(
            *clickable.resolve(&WidgetStates::new()) == *cursor(SystemMouseCursors::CLICK.into())
        );
        assert!(
            *clickable.resolve(&states([WidgetState::Disabled]))
                == *cursor(SystemMouseCursors::BASIC.into())
        );
        assert_eq!(
            clickable.debug_description(),
            "WidgetStateMouseCursor(clickable)"
        );

        let textable = WidgetStateMouseCursor::textable();
        assert!(
            *textable.resolve(&WidgetStates::new()) == *cursor(SystemMouseCursors::TEXT.into())
        );
        assert!(
            *textable.resolve(&states([WidgetState::Disabled]))
                == *cursor(SystemMouseCursors::BASIC.into())
        );

        // Used as a plain cursor, the session is the default-state cursor's.
        let session = clickable.create_session(1);
        assert!(*session.cursor() == *cursor(SystemMouseCursors::CLICK.into()));
        assert_eq!(session.device(), 1);

        let erased: MouseCursorRef = clickable.into();
        assert!(erased.as_any().is::<WidgetStateMouseCursor>());
        assert!(
            *<dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                &erased,
                &states([WidgetState::Disabled])
            ) == *cursor(SystemMouseCursors::BASIC.into())
        );
        assert!(
            *<dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                &Some(Rc::clone(&erased)),
                &states([WidgetState::Disabled])
            )
            .expect("some")
                == *cursor(SystemMouseCursors::BASIC.into())
        );
        let plain = cursor(SystemMouseCursors::GRAB.into());
        assert!(Rc::ptr_eq(
            &<dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&plain, &WidgetStates::new()),
            &plain
        ));
    }

    // widget_state_mouse_cursor.0.dart: a cursor that is forbidden when disabled.
    #[test]
    fn a_resolve_with_cursor_can_forbid_when_disabled() {
        let cursor = WidgetStateMouseCursor::resolve_with(
            |states| {
                if states.contains(&WidgetState::Disabled) {
                    return SystemMouseCursors::FORBIDDEN.into();
                }
                SystemMouseCursors::CLICK.into()
            },
            None,
        );
        assert!(
            *cursor.resolve(&states([WidgetState::Disabled]))
                == *MouseCursorRef::from(SystemMouseCursors::FORBIDDEN)
        );
        assert!(
            *cursor.resolve(&WidgetStates::new())
                == *MouseCursorRef::from(SystemMouseCursors::CLICK)
        );
        assert_eq!(cursor.debug_description(), "WidgetStateMouseCursor()");
    }

    // widget_states_controller_test.dart 'WidgetStatesController constructor'.
    #[test]
    fn controller_starts_with_the_given_states() {
        let mut app = App::new();
        let empty = WidgetStatesController::new(&mut app, None);
        assert_eq!(*app.get(empty).value(), WidgetStates::new());
        let selected = WidgetStatesController::new(&mut app, Some(states([WidgetState::Selected])));
        assert_eq!(*app.get(selected).value(), states([WidgetState::Selected]));
    }

    // widget_states_controller_test.dart 'WidgetStatesController update, listener'.
    #[test]
    fn controller_update_notifies_only_on_change() {
        let mut app = App::new();
        let count = Rc::new(Cell::new(0));
        let value_changed = Listener::new({
            let count = Rc::clone(&count);
            move |_app| count.set(count.get() + 1)
        });

        let controller = WidgetStatesController::new(&mut app, None);
        controller.add_listener(&mut app, value_changed.clone());

        let expect = |app: &App, expected: &[WidgetState], expected_count: i32| {
            assert_eq!(
                *app.get(controller).value(),
                states(expected.iter().copied())
            );
            assert_eq!(count.get(), expected_count);
        };

        controller.update(&mut app, WidgetState::Selected, true);
        expect(&app, &[WidgetState::Selected], 1);
        controller.update(&mut app, WidgetState::Selected, true);
        expect(&app, &[WidgetState::Selected], 1);

        controller.update(&mut app, WidgetState::Hovered, false);
        expect(&app, &[WidgetState::Selected], 1);
        controller.update(&mut app, WidgetState::Selected, false);
        expect(&app, &[], 2);

        controller.update(&mut app, WidgetState::Hovered, true);
        expect(&app, &[WidgetState::Hovered], 3);
        controller.update(&mut app, WidgetState::Hovered, true);
        expect(&app, &[WidgetState::Hovered], 3);
        controller.update(&mut app, WidgetState::Pressed, true);
        expect(&app, &[WidgetState::Hovered, WidgetState::Pressed], 4);
        controller.update(&mut app, WidgetState::Selected, true);
        expect(
            &app,
            &[
                WidgetState::Hovered,
                WidgetState::Pressed,
                WidgetState::Selected,
            ],
            5,
        );
        controller.update(&mut app, WidgetState::Selected, false);
        expect(&app, &[WidgetState::Hovered, WidgetState::Pressed], 6);
        controller.update(&mut app, WidgetState::Selected, false);
        expect(&app, &[WidgetState::Hovered, WidgetState::Pressed], 6);
        controller.update(&mut app, WidgetState::Pressed, false);
        expect(&app, &[WidgetState::Hovered], 7);
        controller.update(&mut app, WidgetState::Hovered, false);
        expect(&app, &[], 8);

        controller.remove_listener(&mut app, &value_changed);
        controller.update(&mut app, WidgetState::Selected, true);
        expect(&app, &[WidgetState::Selected], 8);
    }

    // widget_states_controller_test.dart 'WidgetStatesController const initial value'.
    #[test]
    fn controller_with_an_initial_value_notifies_only_on_change() {
        let mut app = App::new();
        let count = Rc::new(Cell::new(0));
        let controller =
            WidgetStatesController::new(&mut app, Some(states([WidgetState::Selected])));
        controller.add_listener(&mut app, {
            let count = Rc::clone(&count);
            Listener::new(move |_app| count.set(count.get() + 1))
        });

        controller.update(&mut app, WidgetState::Selected, true);
        assert_eq!(
            *app.get(controller).value(),
            states([WidgetState::Selected])
        );
        assert_eq!(count.get(), 0);

        controller.update(&mut app, WidgetState::Selected, false);
        assert_eq!(*app.get(controller).value(), WidgetStates::new());
        assert_eq!(count.get(), 1);

        controller.set_value(&mut app, states([WidgetState::Hovered]));
        assert_eq!(count.get(), 2);
        controller.set_value(&mut app, states([WidgetState::Hovered]));
        assert_eq!(count.get(), 2);

        app.get_mut(controller).dispose();
        assert!(!app.get(controller).change_notifier_data().has_listeners());
    }
}
