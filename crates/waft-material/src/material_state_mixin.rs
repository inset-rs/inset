//! Flutter counterpart: material_ui's `material_state_mixin.dart`.
//!
//! [`MaterialStateMixin`] is a mixin with fields ([`MaterialStateMixinData`]) on a [`State`],
//! which holds it under the field `material_state`.

use std::rc::Rc;

use waft_foundation::{App, Handle, ValueChanged};
use waft_widgets::{State, WidgetState, WidgetStates};

/// The fields of Dart's `MaterialStateMixin`, which a state holds under the field
/// `material_state`.
#[derive(Debug, Default)]
pub struct MaterialStateMixinData {
    /// Managed set of active [`WidgetState`] values; designed to be passed to
    /// `WidgetStateProperty::resolve` methods.
    ///
    /// To mutate and have `set_state` called automatically for you, use
    /// [`MaterialStateMixin::set_material_state`], [`MaterialStateMixin::add_material_state`],
    /// or [`MaterialStateMixin::remove_material_state`]. Directly mutating the set is possible,
    /// and may be necessary if you need to alter its list without calling `set_state` (and thus
    /// triggering a re-render).
    pub material_states: WidgetStates,
}

impl MaterialStateMixinData {
    /// The bag of a state with no active [`WidgetState`].
    pub fn new() -> MaterialStateMixinData {
        MaterialStateMixinData::default()
    }
}

/// Mixin for [`State`] classes that require knowledge of changing [`WidgetState`]
/// values for their child widgets.
///
/// This mixin does nothing by mere application to a [`State`] class, but is
/// helpful when writing `build` methods that include child `InkWell`,
/// `GestureDetector`, `MouseRegion`, or `Focus` widgets. Instead of manually
/// creating handlers for each type of user interaction, such [`State`] classes can
/// instead provide a `ValueChanged<bool>` function and allow [`MaterialStateMixin`]
/// to manage the set of active [`WidgetState`]s, and the calling of `set_state`
/// as necessary.
///
/// The state holds a [`MaterialStateMixinData`] under the field `material_state`.
///
/// ```text
/// impl State for MyWidgetState {
///     fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
///         let color = self.widget(app).color.clone();
///         InkWell::new()
///             .on_focus_change(self.update_material_state(WidgetState::Focused, None))
///             .child(ColoredBox::new(color.resolve(self.material_states(app))))
///             .into_widget()
///     }
/// }
/// ```
pub trait MaterialStateMixin: State {
    /// Mixin field access.
    fn material_state_data(self: Handle<Self>, app: &App) -> &MaterialStateMixinData;

    /// See [`material_state_data`](Self::material_state_data).
    fn material_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut MaterialStateMixinData;

    /// Dart's `materialStates`: the set of active [`WidgetState`] values.
    fn material_states(self: Handle<Self>, app: &App) -> &WidgetStates {
        &self.material_state_data(app).material_states
    }

    /// Callback factory which accepts a [`WidgetState`] value and returns a
    /// closure to mutate [`material_states`](Self::material_states) and call `set_state`.
    ///
    /// Accepts an optional second argument, `on_changed`, which allows
    /// arbitrary functionality to be wired through the [`MaterialStateMixin`].
    /// If supplied, the `on_changed` function is only called when child widgets
    /// report events that make changes to the current set of [`WidgetState`]s.
    ///
    /// ```text
    /// InkWell::new().on_highlight_changed(self.update_material_state(
    ///     WidgetState::Pressed,
    ///     Some(Rc::new(move |app, value| {
    ///         if value {
    ///             self.widget(app).on_pressed();
    ///         }
    ///     })),
    /// ))
    /// ```
    fn update_material_state(
        self: Handle<Self>,
        key: WidgetState,
        on_changed: Option<ValueChanged<bool>>,
    ) -> ValueChanged<bool> {
        Rc::new(move |app: &mut App, value: bool| {
            if self.material_states(app).contains(&key) == value {
                return;
            }
            self.set_material_state(app, key, value);
            if let Some(on_changed) = &on_changed {
                on_changed(app, value);
            }
        })
    }

    /// Mutator to mark a [`WidgetState`] value as either active or inactive.
    fn set_material_state(self: Handle<Self>, app: &mut App, state: WidgetState, is_set: bool) {
        if is_set {
            self.add_material_state(app, state)
        } else {
            self.remove_material_state(app, state)
        }
    }

    /// Mutator to mark a [`WidgetState`] value as active.
    fn add_material_state(self: Handle<Self>, app: &mut App, state: WidgetState) {
        if self
            .material_state_data_mut(app)
            .material_states
            .insert(state)
        {
            self.set_state(app, |_| {});
        }
    }

    /// Mutator to mark a [`WidgetState`] value as inactive.
    fn remove_material_state(self: Handle<Self>, app: &mut App, state: WidgetState) {
        if self
            .material_state_data_mut(app)
            .material_states
            .remove(&state)
        {
            self.set_state(app, |_| {});
        }
    }

    /// Getter for whether this class considers [`WidgetState::Disabled`] to be active.
    fn is_disabled(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app).contains(&WidgetState::Disabled)
    }

    /// Getter for whether this class considers [`WidgetState::Dragged`] to be active.
    fn is_dragged(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app).contains(&WidgetState::Dragged)
    }

    /// Getter for whether this class considers [`WidgetState::Error`] to be active.
    fn is_errored(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app).contains(&WidgetState::Error)
    }

    /// Getter for whether this class considers [`WidgetState::Focused`] to be active.
    fn is_focused(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app).contains(&WidgetState::Focused)
    }

    /// Getter for whether this class considers [`WidgetState::Hovered`] to be active.
    fn is_hovered(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app).contains(&WidgetState::Hovered)
    }

    /// Getter for whether this class considers [`WidgetState::Pressed`] to be active.
    fn is_pressed(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app).contains(&WidgetState::Pressed)
    }

    /// Getter for whether this class considers [`WidgetState::ScrolledUnder`] to be active.
    fn is_scrolled_under(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app)
            .contains(&WidgetState::ScrolledUnder)
    }

    /// Getter for whether this class considers [`WidgetState::Selected`] to be active.
    fn is_selected(self: Handle<Self>, app: &App) -> bool {
        self.material_states(app).contains(&WidgetState::Selected)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fmt::{self, Debug};

    use waft_embedder::{Color, TargetPlatform};
    use waft_widgets::{
        BuildContext, ColoredBox, Container, GlobalKey, IntoWidget, StateData, StatefulWidget,
        WidgetRef, downcast_widget,
    };

    use super::*;
    use crate::test_support::{WidgetTester, with_main_thread_stack};

    const TRUE_COLOR: Color = Color::new(0xFFF44336);
    const FALSE_COLOR: Color = Color::new(0xFF4CAF50);

    /// What Dart's `StreamController<bool>` carries: the callback the inner widget got.
    type Controller = Rc<RefCell<Option<ValueChanged<bool>>>>;

    /// Mock widget which plays the role of a button: it hands its `on_value_changed` to the
    /// controller, through which a test emits notifications that [`WidgetState`] values are
    /// now in or out of play (Dart's `_InnerWidget`, whose `initState` listens to the stream).
    struct InnerWidget {
        on_value_changed: ValueChanged<bool>,
        controller: Controller,
    }

    impl Debug for InnerWidget {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("InnerWidget").finish_non_exhaustive()
        }
    }

    struct InnerWidgetState {
        state: StateData<InnerWidget>,
    }

    impl StatefulWidget for InnerWidget {
        type State = InnerWidgetState;

        fn create_state(&self) -> InnerWidgetState {
            InnerWidgetState {
                state: StateData::new(),
            }
        }
    }

    impl State for InnerWidgetState {
        type Widget = InnerWidget;
        waft_widgets::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let widget = self.widget(app);
            *widget.controller.borrow_mut() = Some(Rc::clone(&widget.on_value_changed));
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            Container::new().into_widget()
        }
    }

    /// Dart's `_MyWidget`.
    struct MyWidget {
        /// Wrapper around `MaterialStateMixin.is_pressed/is_hovered/is_focused/etc`.
        evaluator: fn(&App, Handle<MyWidgetState>) -> bool,
        controller: Controller,
        /// The value we're watching in the given test.
        material_state: WidgetState,
        key: GlobalKey,
    }

    impl Debug for MyWidget {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("MyWidget")
                .field("material_state", &self.material_state)
                .finish_non_exhaustive()
        }
    }

    struct MyWidgetState {
        state: StateData<MyWidget>,
        material_state: MaterialStateMixinData,
    }

    impl StatefulWidget for MyWidget {
        type State = MyWidgetState;

        fn create_state(&self) -> MyWidgetState {
            MyWidgetState {
                state: StateData::new(),
                material_state: MaterialStateMixinData::new(),
            }
        }
    }

    impl MaterialStateMixin for MyWidgetState {
        fn material_state_data(self: Handle<Self>, app: &App) -> &MaterialStateMixinData {
            &app.get(self).material_state
        }

        fn material_state_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut MaterialStateMixinData {
            &mut app.get_mut(self).material_state
        }
    }

    impl State for MyWidgetState {
        type Widget = MyWidget;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let widget = self.widget(app);
            let color = if (widget.evaluator)(app, self) {
                TRUE_COLOR
            } else {
                FALSE_COLOR
            };
            let (controller, material_state) =
                (Rc::clone(&widget.controller), widget.material_state);
            ColoredBox::new(color)
                .key(widget.key.clone().into())
                .child(InnerWidget {
                    on_value_changed: self.update_material_state(material_state, None),
                    controller,
                })
                .into_widget()
        }
    }

    fn colored_box_color(tester: &WidgetTester, key: &GlobalKey) -> u32 {
        let widget = key
            .current_widget(&tester.cell.borrow_mut())
            .expect("mounted");
        downcast_widget::<ColoredBox>(&*widget)
            .expect("a ColoredBox")
            .color
            .to_argb32()
    }

    fn verify(evaluator: fn(&App, Handle<MyWidgetState>) -> bool, material_state: WidgetState) {
        with_main_thread_stack(move || {
            let mut tester = WidgetTester::on(TargetPlatform::Android);
            let controller = Controller::default();
            let key = GlobalKey::new();
            tester.pump_widget(
                MyWidget {
                    evaluator,
                    controller: Rc::clone(&controller),
                    material_state,
                    key: key.clone(),
                }
                .into_widget(),
            );
            let emit = |tester: &mut WidgetTester, value: bool| {
                let on_value_changed = controller.borrow().clone().expect("the inner widget ran");
                on_value_changed(&mut tester.cell.borrow_mut(), value);
                tester.pump_and_settle();
            };

            // Set the value to True
            emit(&mut tester, true);
            assert_eq!(colored_box_color(&tester, &key), TRUE_COLOR.to_argb32());

            // Set the value to False
            emit(&mut tester, false);
            assert_eq!(colored_box_color(&tester, &key), FALSE_COLOR.to_argb32());
        });
    }

    #[test]
    fn widget_state_pressed_is_tracked() {
        verify(|app, state| state.is_pressed(app), WidgetState::Pressed);
    }

    #[test]
    fn widget_state_focused_is_tracked() {
        verify(|app, state| state.is_focused(app), WidgetState::Focused);
    }

    #[test]
    fn widget_state_hovered_is_tracked() {
        verify(|app, state| state.is_hovered(app), WidgetState::Hovered);
    }

    #[test]
    fn widget_state_disabled_is_tracked() {
        verify(|app, state| state.is_disabled(app), WidgetState::Disabled);
    }

    #[test]
    fn widget_state_selected_is_tracked() {
        verify(|app, state| state.is_selected(app), WidgetState::Selected);
    }

    #[test]
    fn widget_state_scrolled_under_is_tracked() {
        verify(
            |app, state| state.is_scrolled_under(app),
            WidgetState::ScrolledUnder,
        );
    }

    #[test]
    fn widget_state_dragged_is_tracked() {
        verify(|app, state| state.is_dragged(app), WidgetState::Dragged);
    }

    #[test]
    fn widget_state_error_is_tracked() {
        verify(|app, state| state.is_errored(app), WidgetState::Error);
    }
}
