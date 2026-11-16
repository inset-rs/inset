//! Flutter counterpart: `widgets/raw_radio.dart`.
//!
//! [`RawRadioState`] is a [`ToggleableStateMixin`] and a [`RadioClient`]: the toggle's
//! animations and gestures come from the mixin, the selection from the [`RadioGroupRegistry`]
//! it registers with. A design system draws the radio in [`RawRadio::builder`], which receives
//! the state as an [`AnyToggleableState`].

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::TargetPlatform;
use waft_foundation::{App, Handle, Ref, ValueChanged};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::MouseCursorRef;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::Semantics;
use crate::widgets::focus_manager::{AnyFocusNode, FocusNodeRef};
use crate::widgets::localizations::WidgetsLocalizations;
use crate::widgets::radio_group::{AnyRadioGroupRegistry, RadioClient, RadioClientData};
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};
use crate::widgets::toggleable::{
    AnyToggleableState, ToggleableStateMixin, ToggleableStateMixinData,
};
use crate::widgets::widget_state::WidgetStatePropertyRef;

/// Signature for [`RawRadio::builder`].
///
/// The builder can use `state` to determine the state of the radio and build
/// the visual.
///
/// See [`ToggleableStateMixin::build_toggleable_with_child`].
pub type RadioBuilder = Rc<dyn Fn(&mut App, BuildContext, AnyToggleableState) -> WidgetRef>;

/// A Radio button that provides basic radio functionalities.
///
/// Provide the `builder` to draw UI for radio.
///
/// See [`ToggleableStateMixin::build_toggleable_with_child`].
///
/// This widget allows selection between a number of mutually exclusive values.
/// When one radio button in a group is selected, the other radio buttons in the
/// group cease to be selected. The values are of type `T`, the type parameter
/// of the radio class. Enums are commonly used for this purpose.
///
/// The radio relies on [`group_registry`](Self::group_registry) to maintains the state for
/// selection. If use in conjunction with a `RadioGroup` widget, use `RadioGroup::maybe_of` to
/// get the group registry from the context.
///
/// If [`enabled`](Self::enabled) is false, the radio will not be interactive.
///
/// See also:
///
///  * `Radio`, which uses this widget to build a Material styled radio button.
///  * `CupertinoRadio`, which uses this widget to build a Cupertino styled radio button.
#[derive(Clone)]
pub struct RawRadio<T: Clone + PartialEq + 'static> {
    pub key: Option<KeyRef>,
    /// The value represented by this radio button.
    pub value: T,
    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty::resolve` is used for the following `WidgetState`s:
    ///
    ///  * `WidgetState::Selected`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Disabled`.
    pub mouse_cursor: WidgetStatePropertyRef<MouseCursorRef>,
    /// Set to true if this radio button is allowed to be returned to an
    /// indeterminate state by selecting it again when selected.
    ///
    /// To indicate returning to an indeterminate state, `RadioGroup::on_changed`
    /// of the `RadioGroup` above the widget tree will be called with null.
    ///
    /// If true, `RadioGroup::on_changed` is called with [`value`](Self::value) when selected
    /// while `RadioGroup::group_value` != [`value`](Self::value), and with null when selected
    /// again while `RadioGroup::group_value` == [`value`](Self::value).
    ///
    /// If false, `RadioGroup::on_changed` will be called with [`value`](Self::value) when it is
    /// selected while `RadioGroup::group_value` != [`value`](Self::value), and only by
    /// selecting another radio button in the group (i.e. changing the value of
    /// `RadioGroup::group_value`) can this radio button be unselected.
    ///
    /// The default is false.
    pub toggleable: bool,
    /// The focus node of the radio.
    pub focus_node: FocusNodeRef,
    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    pub autofocus: bool,
    /// The builder for the radio button visual.
    ///
    /// Use the input `state` to determine the current state of the radio.
    ///
    /// See [`ToggleableStateMixin::build_toggleable_with_child`].
    pub builder: RadioBuilder,
    /// Whether this widget is enabled.
    pub enabled: bool,
    /// The registry this radio registers to.
    ///
    /// The radio relies on [`group_registry`](Self::group_registry) to maintains the state for
    /// selection. If use in conjunction with a `RadioGroup` widget, use
    /// `RadioGroup::maybe_of` to get the group registry from the context.
    pub group_registry: Option<AnyRadioGroupRegistry<T>>,
}

impl<T: Clone + PartialEq + 'static> RawRadio<T> {
    /// Creates a radio button.
    ///
    /// If [`enabled`](Self::enabled) is true, the [`group_registry`](Self::group_registry) must
    /// not be null.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        value: T,
        mouse_cursor: WidgetStatePropertyRef<MouseCursorRef>,
        toggleable: bool,
        focus_node: FocusNodeRef,
        autofocus: bool,
        group_registry: Option<AnyRadioGroupRegistry<T>>,
        enabled: bool,
        builder: RadioBuilder,
    ) -> RawRadio<T> {
        debug_assert!(
            !enabled || group_registry.is_some(),
            "an enabled raw radio must have a registry"
        );
        RawRadio {
            key: None,
            value,
            mouse_cursor,
            toggleable,
            focus_node,
            autofocus,
            builder,
            enabled,
            group_registry,
        }
    }

    /// Dart `RawRadio(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawRadio<T> {
        self.key = Some(key);
        self
    }
}

impl<T: Clone + PartialEq + 'static> Debug for RawRadio<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawRadio")
            .field("enabled", &self.enabled)
            .field("toggleable", &self.toggleable)
            .field("autofocus", &self.autofocus)
            .field("group_registry", &self.group_registry)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for RawRadio<T> {
    type State = RawRadioState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RawRadioState<T> {
        RawRadioState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            toggleable: ToggleableStateMixinData::new(),
            radio_client: RadioClientData::new(),
        }
    }
}

/// Dart's `_RawRadioState`; public because [`RawRadio`]'s `StatefulWidget::State` names it.
pub struct RawRadioState<T: Clone + PartialEq + 'static> {
    state: StateData<RawRadio<T>>,
    ticker_provider: TickerProviderStateMixinData,
    toggleable: ToggleableStateMixinData,
    radio_client: RadioClientData<T>,
}

impl<T: Clone + PartialEq + 'static> RawRadioState<T> {
    /// Handle selection status changed.
    ///
    /// if `selected` is false, nothing happens.
    ///
    /// if `selected` is true, select this radio. i.e. `Radio::on_changed` is called
    /// with `Radio::value`. This also updates the group value in `RadioGroup` if it
    /// is in use.
    ///
    /// if `selected` is null, unselect this radio. Same as `selected` is true
    /// except group value is set to null.
    fn handle_changed(self: Handle<Self>, app: &mut App, selected: Option<bool>) {
        let registry = RadioClient::registry(self, app);
        debug_assert!(registry.is_some());
        let Some(registry) = registry else {
            return;
        };
        if !selected.unwrap_or(true) {
            return;
        }
        if selected.unwrap_or(false) {
            let value = self.widget(app).value.clone();
            registry.on_changed(app, Some(value));
        } else {
            registry.on_changed(app, None);
        }
    }
}

impl<T: Clone + PartialEq + 'static> TickerProviderStateMixin for RawRadioState<T> {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

/// Dart's `vsync: this`.
impl<T: Clone + PartialEq + 'static> TickerProviderObject for RawRadioState<T> {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl<T: Clone + PartialEq + 'static> RadioClient<T> for RawRadioState<T> {
    crate::radio_client_accessors!(T);

    fn tristate(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).toggleable
    }

    fn radio_value(self: Handle<Self>, app: &App) -> T {
        self.widget(app).value.clone()
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        ToggleableStateMixin::is_interactive(self, app)
    }

    fn focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        self.widget(app).focus_node.handle()
    }
}

impl<T: Clone + PartialEq + 'static> ToggleableStateMixin for RawRadioState<T> {
    fn toggleable_data(self: Handle<Self>, app: &App) -> &ToggleableStateMixinData {
        &app.get(self).toggleable
    }

    fn toggleable_data_mut(self: Handle<Self>, app: &mut App) -> &mut ToggleableStateMixinData {
        &mut app.get_mut(self).toggleable
    }

    fn on_changed(self: Handle<Self>, app: &App) -> Option<ValueChanged<Option<bool>>> {
        RadioClient::registry(self, app)?;
        Some(Rc::new(move |app: &mut App, selected| {
            self.handle_changed(app, selected)
        }))
    }

    fn tristate(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).toggleable
    }

    fn value(self: Handle<Self>, app: &App) -> Option<bool> {
        let registry = RadioClient::registry(self, app);
        let group_value = registry.and_then(|registry| registry.group_value(app));
        Some(group_value.as_ref() == Some(&self.widget(app).value))
    }

    fn is_interactive(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).enabled
    }
}

impl<T: Clone + PartialEq + 'static> State for RawRadioState<T> {
    type Widget = RawRadio<T>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        // This has to be before the init state because the [ToggleableStateMixin]
        // expect the [value] is up-to-date when init its state.
        let registry = self.widget(app).group_registry;
        self.set_registry(app, registry);
        ToggleableStateMixin::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &RawRadio<T>) {
        let registry = self.widget(app).group_registry;
        self.set_registry(app, registry);
        self.animate_to_value(app); // The registry's group value may have changed
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ToggleableStateMixin::dispose(self, app);
        TickerProviderStateMixin::dispose(self, app);
        self.set_registry(app, None);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let accessibility_selected: Option<bool>;
        let mut semantics_hint: Option<String> = None;

        match app.platform().target_platform() {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                accessibility_selected = None;
                semantics_hint = None;
            }
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                accessibility_selected = self.value(app);
                // Only provide hint for unselected radio buttons to avoid duplication
                // of the selected state announcement.
                // Selected state is already announced by iOS via the 'selected' property.
                if !self.value(app).unwrap_or(false) {
                    let localizations = <dyn WidgetsLocalizations>::of(app, context);
                    semantics_hint = Some(localizations.radio_button_unselected_label());
                }
            }
        }

        let widget = self.widget(app);
        let focus_node = widget.focus_node.clone();
        let autofocus = widget.autofocus;
        let mouse_cursor = Rc::clone(&widget.mouse_cursor);
        let builder = Rc::clone(&widget.builder);
        let child = builder(app, context, self.as_toggleable_state());
        let mut semantics = Semantics::new().in_mutually_exclusive_group(true);
        if let Some(checked) = self.value(app) {
            semantics = semantics.checked(checked);
        }
        if let Some(selected) = accessibility_selected {
            semantics = semantics.selected(selected);
        }
        if let Some(hint) = semantics_hint {
            semantics = semantics.hint(hint);
        }
        semantics
            .child(self.build_toggleable_with_child(
                app,
                Some(focus_node),
                None,
                autofocus,
                Some(mouse_cursor),
                child,
            ))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::cell::{Cell, RefCell};
    use std::time::Duration;
    use waft_foundation::Ref;

    use waft_embedder::PointerDeviceKind;
    use waft_embedder::{Canvas, Offset, Size};
    use waft_gestures::{GestureBinding, PointerDownEvent, PointerEvent, PointerUpEvent};
    use waft_rendering::CustomPainter;
    use waft_services::SystemMouseCursors;

    use super::*;
    use crate::framework::IntoWidget;
    use crate::test_harness::{binding_mount, binding_pump};
    use crate::widgets::basic::{Builder, Center, CustomPaint, Directionality, SizedBox};
    use crate::widgets::focus_manager::FocusNode;
    use crate::widgets::radio_group::{AnyRadioClient, RadioGroup, RadioGroupRegistry};
    use crate::widgets::widget_state::WidgetStateProperty;
    use waft_embedder::TextDirection;
    use waft_foundation::AppCell;
    use waft_test::{TestPlatform, TestView};

    /// flutter_test's 800 x 600 view at one physical pixel to the logical one.
    fn binding_cell() -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::Android)
            .with_view(Rc::new(TestView::new(800.0, 600.0)));
        AppCell::with_platform(Rc::new(platform))
    }

    /// Dart's `TestRegistry<int>`: the group value is whatever `on_changed` last set.
    struct TestRegistry {
        group_value: Option<u32>,
        clients: Vec<AnyRadioClient<u32>>,
    }

    impl RadioGroupRegistry<u32> for TestRegistry {
        fn group_value(self: Handle<Self>, app: &App) -> Option<u32> {
            app.get(self).group_value
        }

        fn register_client(self: Handle<Self>, app: &mut App, radio: AnyRadioClient<u32>) {
            app.get_mut(self).clients.push(radio);
        }

        fn unregister_client(self: Handle<Self>, app: &mut App, radio: AnyRadioClient<u32>) {
            app.get_mut(self).clients.retain(|client| *client != radio);
        }

        fn on_changed(self: Handle<Self>, app: &mut App, value: Option<u32>) {
            app.get_mut(self).group_value = value;
        }
    }

    fn registry(app: &mut App) -> Ref<TestRegistry> {
        app.create(TestRegistry {
            group_value: None,
            clients: Vec::new(),
        })
    }

    fn raw_radio(
        node: FocusNodeRef,
        registry: Option<Handle<TestRegistry>>,
        enabled: bool,
        toggleable: bool,
        captured: Rc<Cell<Option<AnyToggleableState>>>,
        child: impl Fn() -> WidgetRef + 'static,
    ) -> RawRadio<u32> {
        RawRadio::new(
            0,
            <dyn WidgetStateProperty<MouseCursorRef>>::all(SystemMouseCursors::CLICK.into()),
            toggleable,
            node,
            false,
            registry.map(|registry| registry.as_radio_group_registry()),
            enabled,
            Rc::new(move |_app, _context, state| {
                captured.set(Some(state));
                child()
            }),
        )
    }

    /// Dart's `TestPainter`: a painter makes the `CustomPaint` hit-testable.
    struct TestPainter;

    impl CustomPainter for TestPainter {
        fn paint(&self, _app: &mut App, _canvas: &mut Canvas, _size: Size) {}

        fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
            true
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn painted_box() -> WidgetRef {
        CustomPaint::new()
            .size(Size::new(40.0, 40.0))
            .painter(TestPainter)
            .into_widget()
    }

    /// The logical view of `binding_cell` is the tester's: the radio fills it, so a tap at
    /// (20, 20) lands on it.
    fn tap(cell: &AppCell) {
        let position = Offset::new(20.0, 20.0);
        let mut app = cell.borrow_mut();
        let binding = GestureBinding::instance(&mut app);
        binding.handle_pointer_event(
            &mut app,
            PointerEvent::Down(PointerDownEvent {
                pointer: 1,
                position,
                kind: PointerDeviceKind::Touch,
                ..PointerDownEvent::default()
            }),
        );
        app.drain_microtasks();
        let binding = GestureBinding::instance(&mut app);
        binding.handle_pointer_event(
            &mut app,
            PointerEvent::Up(PointerUpEvent {
                pointer: 1,
                position,
                kind: PointerDeviceKind::Touch,
                ..PointerUpEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    fn pump(cell: &AppCell, at: Duration) {
        cell.elapse(at);
        binding_pump(&mut cell.borrow_mut(), at);
    }

    fn focus_node(cell: &AppCell) -> FocusNodeRef {
        FocusNode::new(&mut cell.borrow_mut()).erase()
    }

    #[test]
    fn a_tap_selects_the_radio_in_its_registry() {
        let cell = binding_cell();
        let node = focus_node(&cell);
        let registry = registry(&mut cell.borrow_mut());
        let captured = Rc::new(Cell::new(None));
        let build = |captured: &Rc<Cell<Option<AnyToggleableState>>>| {
            raw_radio(
                node.clone(),
                Some(registry.handle()),
                true,
                true,
                Rc::clone(captured),
                painted_box,
            )
            .into_widget()
        };

        binding_mount(&cell, build(&captured));
        let actual_state = captured.get().expect("the builder ran");
        {
            let app = cell.borrow();
            assert!(actual_state.tristate(&app));
            assert_eq!(actual_state.value(&app), Some(false));
            assert_eq!(app.get(&registry).group_value, None);

            let state = app
                .handle::<RawRadioState<u32>>(actual_state.id())
                .expect("the radio's state");
            assert!(
                app.get(&registry)
                    .clients
                    .contains(&state.as_radio_client())
            );
        }

        tap(&cell);
        // Rebuilds with new group value
        binding_mount(&cell, build(&captured));

        let app = cell.borrow();
        assert_eq!(app.get(&registry).group_value, Some(0));
        assert_eq!(actual_state.value(&app), Some(true));
    }

    #[test]
    fn a_disabled_radio_does_not_report_a_tap() {
        let cell = binding_cell();
        let node = focus_node(&cell);
        let registry = registry(&mut cell.borrow_mut());
        let captured = Rc::new(Cell::new(None));

        binding_mount(
            &cell,
            raw_radio(
                node,
                Some(registry.handle()),
                false,
                true,
                captured,
                painted_box,
            )
            .into_widget(),
        );
        tap(&cell);

        // onChanged won't fire
        assert_eq!(cell.borrow().get(&registry).group_value, None);
    }

    #[test]
    #[should_panic(expected = "an enabled raw radio must have a registry")]
    fn an_enabled_radio_without_a_registry_is_rejected() {
        let cell = binding_cell();
        let node = focus_node(&cell);

        raw_radio(
            node,
            None,
            true,
            true,
            Rc::new(Cell::new(None)),
            painted_box,
        );
    }

    /// A `RawRadio<String>` under a `RadioGroup<String>`, as Material's `Radio<String>` builds
    /// one: the radio takes its registry from the group above it.
    fn string_radio_group(
        node: FocusNodeRef,
        group_value: Option<String>,
        changes: Rc<RefCell<Vec<Option<String>>>>,
        captured: Rc<Cell<Option<AnyToggleableState>>>,
    ) -> WidgetRef {
        let mut group = RadioGroup::<String>::new(
            Rc::new(move |_app, value| changes.borrow_mut().push(value)),
            Builder::new(move |app, context| {
                let captured = Rc::clone(&captured);
                RawRadio::new(
                    String::from("lafayette"),
                    <dyn WidgetStateProperty<MouseCursorRef>>::all(
                        SystemMouseCursors::CLICK.into(),
                    ),
                    true,
                    node.clone(),
                    false,
                    RadioGroup::<String>::maybe_of(app, context),
                    true,
                    Rc::new(move |_app, _context, state| {
                        captured.set(Some(state));
                        painted_box()
                    }),
                )
                .into_widget()
            }),
        );
        if let Some(group_value) = group_value {
            group = group.group_value(group_value);
        }
        group.into_widget()
    }

    #[test]
    fn a_radio_takes_a_string_value_compared_with_eq() {
        let cell = binding_cell();
        let node = focus_node(&cell);
        let changes: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
        let captured = Rc::new(Cell::new(None));

        binding_mount(
            &cell,
            string_radio_group(
                node.clone(),
                None,
                Rc::clone(&changes),
                Rc::clone(&captured),
            ),
        );
        let state = captured.get().expect("the builder ran");
        assert_eq!(state.value(&cell.borrow()), Some(false));

        tap(&cell);
        assert_eq!(*changes.borrow(), [Some(String::from("lafayette"))]);

        // An equal string that is another object selects the radio.
        binding_mount(
            &cell,
            string_radio_group(
                node,
                Some(String::from("lafayette")),
                Rc::clone(&changes),
                Rc::clone(&captured),
            ),
        );
        assert_eq!(state.value(&cell.borrow()), Some(true));

        // Selecting a toggleable radio again unselects it.
        tap(&cell);
        assert_eq!(*changes.borrow(), [Some(String::from("lafayette")), None]);
    }

    #[test]
    #[should_panic(expected = "No WidgetsLocalizations found")]
    fn an_unselected_radio_on_ios_reads_the_widgets_localizations() {
        let platform = TestPlatform::new()
            .on(TargetPlatform::IOS)
            .with_view(Rc::new(TestView::new(800.0, 600.0)));
        let cell = AppCell::with_platform(Rc::new(platform));
        let node = focus_node(&cell);
        let registry = registry(&mut cell.borrow_mut());

        binding_mount(
            &cell,
            raw_radio(
                node,
                Some(registry.handle()),
                true,
                false,
                Rc::new(Cell::new(None)),
                painted_box,
            )
            .into_widget(),
        );
    }

    #[test]
    fn a_radio_at_zero_area_does_not_crash() {
        let cell = binding_cell();
        let node = focus_node(&cell);
        let registry = registry(&mut cell.borrow_mut());
        let captured = Rc::new(Cell::new(None));
        let radio = raw_radio(
            node,
            Some(registry.handle()),
            true,
            false,
            captured.clone(),
            || SizedBox::square(Some(24.0)).into_widget(),
        );

        binding_mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::shrink().child(radio)),
            )
            .into_widget(),
        );
        pump(&cell, Duration::from_millis(16));

        let app = cell.borrow();
        let state = captured.get().expect("the builder ran");
        let size = state.context(&app).size(&app);
        assert_eq!(size, Some(Size::ZERO));
    }
}
