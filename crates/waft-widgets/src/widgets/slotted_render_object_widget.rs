//! Flutter counterpart: `widgets/slotted_render_object_widget.dart`.
//!
//! The deprecated `SlottedMultiChildRenderObjectWidgetMixin` is the
//! [`SlottedMultiChildRenderObjectWidget`] trait itself: Dart keeps the mixin only so that
//! older subclasses keep compiling.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::rc::Rc;

use waft_foundation::{App, Handle};
use waft_rendering::{
    AnyRenderObject, ErasedRenderObject, PipelineOwner, RenderHandle, RenderObject,
};

use crate::framework::{
    AnyElement, Element, ElementData, IntoWidget, KeyRef, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget, Slot, Widget,
    WidgetKind, WidgetRef, downcast_widget, short_type_name,
};

/// A value that names one of a [`SlottedMultiChildRenderObjectWidget`]'s slots: Dart's
/// `SlotType`, usually an enum.
pub trait SlotValue: Clone + Eq + Hash + Debug + 'static {}

impl<T: Clone + Eq + Hash + Debug + 'static> SlotValue for T {}

/// A superclass for [`RenderObjectWidget`]s that configure [`RenderObject`] subclasses that
/// organize their children in different slots.
///
/// Implementers of this trait have to provide the list of available slots by overriding
/// [`slots`](Self::slots). The list of slots must never change for a given type.
///
/// The [`child_for_slot`](Self::child_for_slot) method returns the child widget that should
/// be configured for the provided slot. The returned widget may be `None` if no widget should
/// occupy the slot. The `RenderObject` must use the [`SlottedContainerRenderObjectMixin`],
/// whose [`child_for_slot`](SlottedContainerRenderObjectMixin::child_for_slot) returns the
/// render object of the slot's child.
///
/// Like any other [`RenderObjectWidget`], an implementer has to provide
/// [`create_render_object`](RenderObjectWidget::create_render_object) and
/// [`update_render_object`](RenderObjectWidget::update_render_object).
pub trait SlottedMultiChildRenderObjectWidget: RenderObjectWidget
where
    Self::RenderObject: SlottedContainerRenderObjectMixin<Self::Slot>,
{
    /// Dart's `SlotType`.
    type Slot: SlotValue;

    /// This method is called to obtain the list of slots that the widget has.
    ///
    /// The list must never change, and its entries must be unique.
    fn slots(&self) -> Vec<Self::Slot>;

    /// Given a slot this method returns the widget that should occupy that slot.
    ///
    /// The method may return `None` if the slot should be empty.
    fn child_for_slot(&self, slot: &Self::Slot) -> Option<WidgetRef>;
}

/// The kind tag of [`IntoWidget`] for a [`SlottedMultiChildRenderObjectWidget`].
pub struct SlottedMultiChildRenderObjectKind;

/// The erased form of a [`SlottedMultiChildRenderObjectWidget`].
pub struct SlottedMultiChildRenderObject<W: SlottedMultiChildRenderObjectWidget>(pub W)
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>;

impl<W: SlottedMultiChildRenderObjectWidget> IntoWidget<SlottedMultiChildRenderObjectKind> for W
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    fn into_widget(self) -> WidgetRef {
        if cfg!(debug_assertions) {
            RenderObjectWidget::debug_check_constructor(&self);
        }
        Rc::new(SlottedMultiChildRenderObject(self))
    }
}

impl<W: SlottedMultiChildRenderObjectWidget> Widget for SlottedMultiChildRenderObject<W>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        SlottedRenderObjectElement::<W>::new(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn type_name(&self) -> &'static str {
        short_type_name::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: SlottedMultiChildRenderObjectWidget> Debug for SlottedMultiChildRenderObject<W>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The fields of [`SlottedContainerRenderObjectMixin`]: Dart's `_slotToChild` map, kept in
/// insertion order as Dart's `Map` is, which is the order the children are visited in.
pub struct SlottedContainerRenderObjectData<S, C> {
    slot_to_child: Vec<(S, C)>,
}

impl<S, C> SlottedContainerRenderObjectData<S, C> {
    /// The fields of a render object with every slot empty.
    pub fn new() -> SlottedContainerRenderObjectData<S, C> {
        SlottedContainerRenderObjectData {
            slot_to_child: Vec::new(),
        }
    }
}

impl<S, C> Default for SlottedContainerRenderObjectData<S, C> {
    fn default() -> SlottedContainerRenderObjectData<S, C> {
        SlottedContainerRenderObjectData::new()
    }
}

/// Mixin for a [`RenderObject`] configured by a [`SlottedMultiChildRenderObjectWidget`].
///
/// The [`RenderObject`] child currently occupying a given slot can be obtained by calling
/// [`child_for_slot`](Self::child_for_slot).
///
/// Implementers must call [`did_attach`](Self::did_attach), [`did_detach`](Self::did_detach),
/// [`redepth_children`](Self::redepth_children) and [`visit_children`](Self::visit_children)
/// from their [`RenderObject`] overrides of the same names, as Dart's mixin overrides them.
pub trait SlottedContainerRenderObjectMixin<S: SlotValue>: RenderObject {
    /// Dart's `ChildType`.
    type ChildType: ErasedRenderObject;

    /// Mixin field access.
    fn slotted_container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &SlottedContainerRenderObjectData<S, Self::ChildType>;

    /// See [`slotted_container_data`](Self::slotted_container_data).
    fn slotted_container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut SlottedContainerRenderObjectData<S, Self::ChildType>;

    /// Returns the [`RenderObject`] child that is currently occupying the provided `slot`.
    ///
    /// Returns `None` if no [`RenderObject`] is configured for the given slot.
    fn child_for_slot(self: RenderHandle<Self>, app: &App, slot: &S) -> Option<Self::ChildType> {
        self.slotted_container_data(app)
            .slot_to_child
            .iter()
            .find(|(child_slot, _)| child_slot == slot)
            .map(|(_, child)| *child)
    }

    /// Returns an iterable over all non-`None` children.
    ///
    /// The children are returned in the order the slots were filled.
    fn children(self: RenderHandle<Self>, app: &App) -> Vec<Self::ChildType> {
        self.slotted_container_data(app)
            .slot_to_child
            .iter()
            .map(|(_, child)| *child)
            .collect()
    }

    /// Returns the debug name for a given `slot`.
    ///
    /// This method is called by `debugDescribeChildren` for each slot that is currently
    /// occupied by a child to obtain a name for that slot for debug outputs.
    fn debug_name_for_slot(&self, slot: &S) -> String {
        format!("{slot:?}")
    }

    /// Dart's `attach` after `super.attach(owner)`.
    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        for child in self.children(app) {
            child.as_object().attach(app, owner);
        }
    }

    /// Dart's `detach` after `super.detach()`.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        for child in self.children(app) {
            child.as_object().detach(app);
        }
    }

    /// Dart's `redepthChildren`.
    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        for child in self.children(app) {
            self.as_render_object(app)
                .redepth_child(app, child.as_object());
        }
    }

    /// Dart's `visitChildren`.
    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        for child in self.children(app) {
            visitor(child.as_object());
        }
    }

    /// Dart's `_setChild`.
    fn set_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: Option<Self::ChildType>,
        slot: &S,
    ) {
        let old_child = self.child_for_slot(app, slot);
        if let Some(old_child) = old_child {
            self.as_render_object(app)
                .drop_child(app, old_child.as_object());
            self.slotted_container_data_mut(app)
                .slot_to_child
                .retain(|(child_slot, _)| child_slot != slot);
        }
        if let Some(child) = child {
            self.slotted_container_data_mut(app)
                .slot_to_child
                .push((slot.clone(), child));
            self.as_render_object(app)
                .adopt_child(app, child.as_object());
        }
    }

    /// Dart's `_moveChild`.
    fn move_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: Self::ChildType,
        slot: &S,
        old_slot: &S,
    ) {
        debug_assert!(slot != old_slot);
        let old_child = self.child_for_slot(app, old_slot);
        if old_child == Some(child) {
            self.set_child(app, None, old_slot);
        }
        self.set_child(app, Some(child), slot);
    }
}

/// An [`Element`] that uses a [`SlottedMultiChildRenderObjectWidget`] as its configuration.
///
/// The element cooperates with a [`RenderObject`] that uses the
/// [`SlottedContainerRenderObjectMixin`] to manage its children.
///
/// Each child's element [`Slot`] is one `Slot::Custom` per slot value, made once and reused,
/// so that a child keeps an identical slot across rebuilds as Dart's constant slot values do.
pub struct SlottedRenderObjectElement<W: SlottedMultiChildRenderObjectWidget>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    element: ElementData,
    render_object_element: RenderObjectElementData,
    slot_to_child: Vec<(W::Slot, AnyElement)>,
    keyed_children: Vec<(KeyRef, AnyElement)>,
    slot_objects: Vec<(W::Slot, Rc<dyn Any>)>,
    debug_previous_slots: Option<Vec<W::Slot>>,
}

impl<W: SlottedMultiChildRenderObjectWidget> SlottedRenderObjectElement<W>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    /// Creates an element that uses the given widget as its configuration.
    pub fn new(app: &mut App, widget: WidgetRef) -> Handle<Self> {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(SlottedRenderObjectElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            slot_to_child: Vec::new(),
            keyed_children: Vec::new(),
            slot_objects: Vec::new(),
            debug_previous_slots: None,
        })
        .into_tree_node(app)
    }

    /// The element slot of the slot value `slot`, the same object every time.
    fn element_slot(self: Handle<Self>, app: &mut App, slot: &W::Slot) -> Slot {
        if let Some((_, object)) = app
            .get(self)
            .slot_objects
            .iter()
            .find(|(slot_value, _)| slot_value == slot)
        {
            return Slot::Custom(Rc::clone(object));
        }
        let object: Rc<dyn Any> = Rc::new(slot.clone());
        app.get_mut(self)
            .slot_objects
            .push((slot.clone(), Rc::clone(&object)));
        Slot::Custom(object)
    }

    /// Dart's `slot as SlotType`.
    fn slot_value(slot: Option<&Slot>) -> &W::Slot {
        match slot {
            Some(Slot::Custom(object)) => object
                .downcast_ref::<W::Slot>()
                .expect("a slotted element gives its children slots of its slot type"),
            _ => unreachable!("a slotted element gives its children custom slots"),
        }
    }

    fn typed(self: Handle<Self>, app: &App) -> RenderHandle<W::RenderObject> {
        self.typed_render_object(app)
    }

    /// Dart's `_updateChildren`.
    fn update_children(self: Handle<Self>, app: &mut App) {
        let widget = self.as_element().widget(app).clone();
        let slotted = Self::widget_of(&widget);
        let slots = slotted.slots();
        if cfg!(debug_assertions) {
            let previous = app
                .get_mut(self)
                .debug_previous_slots
                .get_or_insert_with(|| slots.clone());
            assert!(
                *previous == slots,
                "{}.slots must not change.",
                std::any::type_name::<W>()
            );
        }
        debug_assert!(
            slots.iter().collect::<std::collections::HashSet<_>>().len() == slots.len(),
            "slots must be unique"
        );

        let old_keyed_elements = std::mem::take(&mut app.get_mut(self).keyed_children);
        let mut old_slot_to_child = std::mem::take(&mut app.get_mut(self).slot_to_child);
        // Dart's `debugDuplicateKeys`: each duplicated key with every element that used it.
        let mut debug_duplicate_keys: Vec<(KeyRef, Vec<AnyElement>)> = Vec::new();

        for slot in &slots {
            let widget = slotted.child_for_slot(slot);
            let new_widget_key = widget.as_ref().and_then(|widget| widget.key().cloned());
            let old_slot_child = old_slot_to_child
                .iter()
                .find(|(old_slot, _)| old_slot == slot)
                .map(|(_, child)| *child);
            let old_key_child = new_widget_key.as_ref().and_then(|key| {
                old_keyed_elements
                    .iter()
                    .find(|(old_key, _)| old_key == key)
                    .map(|(_, child)| *child)
            });
            // Try to find the slot for the correct Element that `widget` should update.
            // If key matching fails, resort to `oldSlotChild` from the same slot.
            let from_element = if let Some(old_key_child) = old_key_child {
                let old_key_slot = Self::slot_value(old_key_child.slot(app).as_ref()).clone();
                take_slot(&mut old_slot_to_child, &old_key_slot)
            } else if old_slot_child.is_none_or(|child| child.widget(app).key().is_none()) {
                take_slot(&mut old_slot_to_child, slot)
            } else {
                // The only case we can't use `oldSlotChild` is when its widget has a key.
                debug_assert!(!crate::framework::keys_equal(
                    old_slot_child.and_then(|child| child.widget(app).key()),
                    new_widget_key.as_ref()
                ));
                None
            };
            let element_slot = self.element_slot(app, slot);
            let new_child =
                self.as_element()
                    .update_child(app, from_element, widget, Some(element_slot));
            if let Some(new_child) = new_child {
                app.get_mut(self)
                    .slot_to_child
                    .push((slot.clone(), new_child));
                if let Some(new_widget_key) = new_widget_key {
                    let keyed_children = &mut app.get_mut(self).keyed_children;
                    let existing = keyed_children
                        .iter()
                        .position(|(key, _)| *key == new_widget_key);
                    if cfg!(debug_assertions)
                        && let Some(index) = existing
                    {
                        let existing_element = keyed_children[index].1;
                        match debug_duplicate_keys
                            .iter_mut()
                            .find(|(key, _)| *key == new_widget_key)
                        {
                            Some((_, elements)) => elements.push(new_child),
                            None => debug_duplicate_keys
                                .push((new_widget_key.clone(), vec![existing_element, new_child])),
                        }
                    }
                    // Dart's map write: the last element with a key wins.
                    match existing {
                        Some(index) => keyed_children[index].1 = new_child,
                        None => keyed_children.push((new_widget_key, new_child)),
                    }
                }
            }
        }
        for (_, child) in old_slot_to_child {
            self.as_element().deactivate_child(app, child);
        }
        Self::debug_duplicate_keys(&debug_duplicate_keys);
        debug_assert!(
            app.get(self).keyed_children.iter().all(|(_, keyed)| app
                .get(self)
                .slot_to_child
                .iter()
                .any(|(_, child)| child == keyed)),
            "_keyedChildren should be a subset of _slotToChild"
        );
    }
}

impl<W: SlottedMultiChildRenderObjectWidget> SlottedRenderObjectElement<W>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    /// Dart's `_debugDuplicateKeys`: panics with the first duplicated key and every element
    /// that used it.
    fn debug_duplicate_keys(debug_duplicate_keys: &[(KeyRef, Vec<AnyElement>)]) {
        if let Some((key, elements)) = debug_duplicate_keys.first() {
            let offending: String = elements
                .iter()
                .map(|element| format!("  - {element:?}\n"))
                .collect();
            panic!(
                "Multiple widgets used the same key in {}.\nThe key {:?} was used by multiple \
                 widgets. The offending widgets were:\n{offending}A key can only be specified on \
                 one widget at a time in the same parent widget.",
                std::any::type_name::<W>(),
                key
            );
        }
    }
}

/// Dart's `oldSlotToChild.remove(slot)`.
fn take_slot<S: PartialEq>(slots: &mut Vec<(S, AnyElement)>, slot: &S) -> Option<AnyElement> {
    let index = slots.iter().position(|(old_slot, _)| old_slot == slot)?;
    Some(slots.remove(index).1)
}

impl<W: SlottedMultiChildRenderObjectWidget> RenderObjectElementWidget
    for SlottedRenderObjectElement<W>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    type Widget = W;

    fn widget_of(widget: &WidgetRef) -> &W {
        downcast_widget::<W>(&**widget)
            .expect("a SlottedRenderObjectElement holds its SlottedMultiChildRenderObjectWidget")
    }
}

impl<W: SlottedMultiChildRenderObjectWidget> RenderObjectElement for SlottedRenderObjectElement<W>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl<W: SlottedMultiChildRenderObjectWidget> Element for SlottedRenderObjectElement<W>
where
    W::RenderObject: SlottedContainerRenderObjectMixin<W::Slot>,
{
    crate::element_accessors!();

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        for (_, child) in &app.get(self).slot_to_child {
            visitor(*child);
        }
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(app.get(self).slot_to_child.iter().any(|(_, c)| *c == child));
        let slot = Self::slot_value(child.slot(app).as_ref()).clone();
        let removed = take_slot(&mut app.get_mut(self).slot_to_child, &slot);
        debug_assert!(removed.is_some());
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
        self.update_children(app);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        RenderObjectElement::update(self, app, new_widget.clone());
        debug_assert!(crate::framework::same_widget(
            self.as_element().widget(app),
            &new_widget
        ));
        self.update_children(app);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let slot = Self::slot_value(slot.as_ref()).clone();
        let render_object = self.typed(app);
        let child = ErasedRenderObject::from_object(child);
        render_object.set_child(app, Some(child), &slot);
        debug_assert!(render_object.child_for_slot(app, &slot) == Some(child));
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        let old_slot = Self::slot_value(old_slot.as_ref()).clone();
        let new_slot = Self::slot_value(new_slot.as_ref()).clone();
        let render_object = self.typed(app);
        render_object.move_child(
            app,
            ErasedRenderObject::from_object(child),
            &new_slot,
            &old_slot,
        );
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let slot = Self::slot_value(slot.as_ref()).clone();
        let render_object = self.typed(app);
        if render_object.child_for_slot(app, &slot) == Some(ErasedRenderObject::from_object(child))
        {
            render_object.set_child(app, None, &slot);
            debug_assert!(render_object.child_for_slot(app, &slot).is_none());
        }
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Offset, Size};
    use waft_foundation::{AppCell, ValueKey};
    use waft_rendering::{
        AnyRenderBox, BoxConstraints, BoxParentData, PaintingContext, RenderBox, RenderBoxData,
        RenderObjectData,
    };

    use super::*;
    use crate::framework::BuildContext;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Center, SizedBox};

    /// `slotted_render_object_widget_test.dart`'s `_DiagonalSlot`, with Dart's `null` slot.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum DiagonalSlot {
        Null,
        TopLeft,
        BottomRight,
    }

    /// `_Diagonal`.
    #[derive(Debug, Default)]
    struct Diagonal {
        top_left: Option<WidgetRef>,
        bottom_right: Option<WidgetRef>,
        null_slot: Option<WidgetRef>,
    }

    impl RenderObjectWidget for Diagonal {
        type RenderObject = RenderDiagonal;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderHandle::new_box(
                app,
                RenderDiagonal {
                    render_object: RenderObjectData::new(),
                    render_box: RenderBoxData::new(),
                    slotted: SlottedContainerRenderObjectData::new(),
                },
            )
            .as_object()
        }
    }

    impl SlottedMultiChildRenderObjectWidget for Diagonal {
        type Slot = DiagonalSlot;

        fn slots(&self) -> Vec<DiagonalSlot> {
            vec![
                DiagonalSlot::Null,
                DiagonalSlot::TopLeft,
                DiagonalSlot::BottomRight,
            ]
        }

        fn child_for_slot(&self, slot: &DiagonalSlot) -> Option<WidgetRef> {
            match slot {
                DiagonalSlot::Null => self.null_slot.clone(),
                DiagonalSlot::TopLeft => self.top_left.clone(),
                DiagonalSlot::BottomRight => self.bottom_right.clone(),
            }
        }
    }

    /// `_RenderDiagonal`.
    struct RenderDiagonal {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        slotted: SlottedContainerRenderObjectData<DiagonalSlot, AnyRenderBox>,
    }

    impl SlottedContainerRenderObjectMixin<DiagonalSlot> for RenderDiagonal {
        type ChildType = AnyRenderBox;

        fn slotted_container_data(
            self: RenderHandle<Self>,
            app: &App,
        ) -> &SlottedContainerRenderObjectData<DiagonalSlot, AnyRenderBox> {
            &self.get(app).slotted
        }

        fn slotted_container_data_mut(
            self: RenderHandle<Self>,
            app: &mut App,
        ) -> &mut SlottedContainerRenderObjectData<DiagonalSlot, AnyRenderBox> {
            &mut self.get_mut(app).slotted
        }
    }

    impl RenderDiagonal {
        fn lay_out(app: &mut App, child: Option<AnyRenderBox>, offset: Offset) -> Size {
            let Some(child) = child else {
                return Size::ZERO;
            };
            child.layout(app, BoxConstraints::new(), true);
            child
                .as_object()
                .parent_data_of_mut::<BoxParentData>(app)
                .offset = offset;
            child.size(app)
        }
    }

    impl RenderObject for RenderDiagonal {
        waft_rendering::render_object_accessors!();

        fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
            SlottedContainerRenderObjectMixin::did_attach(self, app, owner);
        }

        fn did_detach(self: RenderHandle<Self>, app: &mut App) {
            SlottedContainerRenderObjectMixin::did_detach(self, app);
        }

        fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
            SlottedContainerRenderObjectMixin::redepth_children(self, app);
        }

        fn visit_children(
            self: RenderHandle<Self>,
            app: &App,
            visitor: &mut dyn FnMut(AnyRenderObject),
        ) {
            SlottedContainerRenderObjectMixin::visit_children(self, app, visitor);
        }

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let top_left = self.child_for_slot(app, &DiagonalSlot::TopLeft);
            let top_left_size = RenderDiagonal::lay_out(app, top_left, Offset::ZERO);
            let bottom_right = self.child_for_slot(app, &DiagonalSlot::BottomRight);
            let bottom_right_size = RenderDiagonal::lay_out(
                app,
                bottom_right,
                Offset::new(top_left_size.width(), top_left_size.height()),
            );
            let null_slot = self.child_for_slot(app, &DiagonalSlot::Null);
            let null_slot_size = RenderDiagonal::lay_out(app, null_slot, Offset::ZERO);
            let size = self.constraints(app).constrain(Size::new(
                top_left_size.width() + bottom_right_size.width() + null_slot_size.width(),
                top_left_size.height() + bottom_right_size.height() + null_slot_size.height(),
            ));
            self.set_size(app, size);
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
        ) {
            for slot in [DiagonalSlot::TopLeft, DiagonalSlot::BottomRight] {
                if let Some(child) = self.child_for_slot(app, &slot) {
                    let child_offset = child
                        .as_object()
                        .parent_data_of::<BoxParentData>(app)
                        .offset;
                    context.paint_child(app, child.as_object(), child_offset + offset);
                }
            }
        }
    }

    impl RenderBox for RenderDiagonal {
        waft_rendering::render_box_accessors!();

        fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
            if !child.parent_data_is::<BoxParentData>(app) {
                child.set_parent_data(app, BoxParentData::new());
            }
        }
    }

    fn sized(width: f64, height: f64) -> WidgetRef {
        SizedBox::new().width(width).height(height).into_widget()
    }

    fn keyed(key: &str, width: f64, height: f64) -> WidgetRef {
        SizedBox::new()
            .key(ValueKey::new(key.to_owned()).into())
            .width(width)
            .height(height)
            .into_widget()
    }

    fn diagonal(app: &App, harness: &Harness) -> RenderHandle<RenderDiagonal> {
        let center = harness.render_root(app).child(app).expect("the centre");
        let mut found = None;
        center
            .as_object()
            .visit_children(app, &mut |child| found = Some(child));
        found
            .expect("the diagonal")
            .downcast::<RenderDiagonal>(app)
            .expect("a RenderDiagonal")
    }

    fn mount_diagonal(
        app: &mut App,
        harness: Option<&Harness>,
        widget: Diagonal,
    ) -> Option<Harness> {
        let tree = Center::new().child(widget).into_widget();
        match harness {
            Some(harness) => {
                harness.set_child(app, tree);
                harness.pump(app);
                None
            }
            None => {
                let harness = Harness::mount(app, tree);
                harness.pump(app);
                Some(harness)
            }
        }
    }

    /// "SlottedRenderObjectWidget test": the slots lay out, update, take a keyed child, and
    /// empty again.
    #[test]
    fn slotted_children_lay_out_update_and_leave_their_slots() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount_diagonal(
            &mut app,
            None,
            Diagonal {
                top_left: Some(sized(40.0, 50.0)),
                bottom_right: Some(sized(55.0, 60.0)),
                null_slot: None,
            },
        )
        .expect("mounted");
        let render = diagonal(&app, &harness);
        assert_eq!(render.size(&app), Size::new(40.0 + 55.0, 50.0 + 60.0));
        let bottom_right = render
            .child_for_slot(&app, &DiagonalSlot::BottomRight)
            .expect("a bottom right child");
        assert_eq!(
            bottom_right
                .as_object()
                .parent_data_of::<BoxParentData>(&app)
                .offset,
            Offset::new(40.0, 50.0)
        );

        // Two unkeyed children of new sizes update the children in place.
        mount_diagonal(
            &mut app,
            Some(&harness),
            Diagonal {
                top_left: Some(sized(50.0, 60.0)),
                bottom_right: Some(sized(110.0, 70.0)),
                null_slot: None,
            },
        );
        assert_eq!(render.size(&app), Size::new(50.0 + 110.0, 60.0 + 70.0));
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::BottomRight),
            Some(bottom_right)
        );

        mount_diagonal(
            &mut app,
            Some(&harness),
            Diagonal {
                top_left: Some(sized(50.0, 60.0)),
                bottom_right: Some(keyed("new", 110.0, 70.0)),
                null_slot: None,
            },
        );
        assert_eq!(render.size(&app), Size::new(50.0 + 110.0, 60.0 + 70.0));
        assert_ne!(
            render.child_for_slot(&app, &DiagonalSlot::BottomRight),
            Some(bottom_right),
            "a newly keyed child replaces the unkeyed one"
        );

        mount_diagonal(
            &mut app,
            Some(&harness),
            Diagonal {
                top_left: Some(sized(50.0, 60.0)),
                ..Diagonal::default()
            },
        );
        assert_eq!(render.size(&app), Size::new(50.0, 60.0));
        assert_eq!(render.children(&app).len(), 1);

        mount_diagonal(&mut app, Some(&harness), Diagonal::default());
        assert_eq!(render.size(&app), Size::ZERO);
        assert!(render.children(&app).is_empty());
    }

    /// "nameForSlot".
    #[test]
    fn a_slot_is_named_by_its_debug_form() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let render = RenderHandle::new_box(
            &mut app,
            RenderDiagonal {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                slotted: SlottedContainerRenderObjectData::new(),
            },
        );
        let render = render.get(&app);
        assert_eq!(
            render.debug_name_for_slot(&DiagonalSlot::BottomRight),
            "BottomRight"
        );
        assert_eq!(
            render.debug_name_for_slot(&DiagonalSlot::TopLeft),
            "TopLeft"
        );
    }

    /// "key reparenting": keyed children that change slots keep their render objects.
    #[test]
    fn keyed_children_move_between_slots_with_their_render_objects() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (small, big, null) = (
            keyed("smol", 10.0, 10.0),
            keyed("big", 100.0, 100.0),
            keyed("null", 50.0, 50.0),
        );
        let harness = mount_diagonal(
            &mut app,
            None,
            Diagonal {
                top_left: Some(small.clone()),
                bottom_right: Some(big.clone()),
                null_slot: Some(null.clone()),
            },
        )
        .expect("mounted");
        let render = diagonal(&app, &harness);
        let small_box = render
            .child_for_slot(&app, &DiagonalSlot::TopLeft)
            .expect("small");
        let big_box = render
            .child_for_slot(&app, &DiagonalSlot::BottomRight)
            .expect("big");
        let null_box = render
            .child_for_slot(&app, &DiagonalSlot::Null)
            .expect("null");
        assert_eq!(small_box.size(&app), Size::new(10.0, 10.0));
        assert_eq!(render.size(&app), Size::new(160.0, 160.0));

        mount_diagonal(
            &mut app,
            Some(&harness),
            Diagonal {
                top_left: Some(big.clone()),
                bottom_right: Some(small.clone()),
                null_slot: Some(null.clone()),
            },
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::TopLeft),
            Some(big_box)
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::BottomRight),
            Some(small_box)
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::Null),
            Some(null_box)
        );
        assert_eq!(
            small_box
                .as_object()
                .parent_data_of::<BoxParentData>(&app)
                .offset,
            Offset::new(100.0, 100.0)
        );
        assert_eq!(render.size(&app), Size::new(160.0, 160.0));
        let small_element = element_in(&app, &harness, DiagonalSlot::BottomRight);
        let big_element = element_in(&app, &harness, DiagonalSlot::TopLeft);
        let null_element = element_in(&app, &harness, DiagonalSlot::Null);

        // Shifting slots.
        mount_diagonal(
            &mut app,
            Some(&harness),
            Diagonal {
                top_left: Some(null.clone()),
                bottom_right: Some(big.clone()),
                null_slot: Some(small.clone()),
            },
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::TopLeft),
            Some(null_box)
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::BottomRight),
            Some(big_box)
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::Null),
            Some(small_box)
        );
        assert_eq!(
            element_in(&app, &harness, DiagonalSlot::TopLeft),
            null_element
        );
        assert_eq!(
            element_in(&app, &harness, DiagonalSlot::BottomRight),
            big_element
        );
        assert_eq!(
            element_in(&app, &harness, DiagonalSlot::Null),
            small_element
        );

        // Moving + Deleting.
        mount_diagonal(
            &mut app,
            Some(&harness),
            Diagonal {
                bottom_right: Some(big.clone()),
                ..Diagonal::default()
            },
        );
        assert_eq!(render.child_for_slot(&app, &DiagonalSlot::TopLeft), None);
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::BottomRight),
            Some(big_box)
        );
        assert_eq!(render.child_for_slot(&app, &DiagonalSlot::Null), None);
        assert!(is_defunct(&app, small_element));
        assert!(is_defunct(&app, null_element));
        assert_eq!(
            element_in(&app, &harness, DiagonalSlot::BottomRight),
            big_element
        );

        // Moving.
        mount_diagonal(
            &mut app,
            Some(&harness),
            Diagonal {
                top_left: Some(big),
                ..Diagonal::default()
            },
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::TopLeft),
            Some(big_box)
        );
        assert_eq!(
            render.child_for_slot(&app, &DiagonalSlot::BottomRight),
            None
        );
        assert_eq!(
            element_in(&app, &harness, DiagonalSlot::TopLeft),
            big_element
        );
    }

    /// The child element in `slot` of the diagonal under the harness's root.
    fn element_in(app: &App, harness: &Harness, slot: DiagonalSlot) -> AnyElement {
        fn find_diagonal(app: &App, element: AnyElement) -> Option<AnyElement> {
            if element.widget(app).widget_type() == TypeId::of::<Diagonal>() {
                return Some(element);
            }
            let mut found = None;
            element.visit_children(app, &mut |child| {
                if found.is_none() {
                    found = find_diagonal(app, child);
                }
            });
            found
        }
        let diagonal =
            find_diagonal(app, harness.root.as_element()).expect("the diagonal is mounted");
        let mut found = None;
        diagonal.visit_children(app, &mut |child| {
            let child_slot = match child.slot(app) {
                Some(Slot::Custom(object)) => object.downcast_ref::<DiagonalSlot>().copied(),
                _ => None,
            };
            if child_slot == Some(slot) {
                found = Some(child);
            }
        });
        found.expect("a child in the slot")
    }

    /// Dart's `element.debugIsDefunct`, for an element whose slot may be gone.
    fn is_defunct(app: &App, element: AnyElement) -> bool {
        app.is_disposed(element.id()) || element.debug_is_defunct(app)
    }

    /// "duplicated key error message": the panic lists every element that used the key, after
    /// the old children are deactivated.
    #[test]
    #[should_panic(expected = "The offending widgets were:")]
    fn a_duplicated_key_panics_naming_every_element_that_used_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        mount_diagonal(
            &mut app,
            None,
            Diagonal {
                top_left: Some(keyed("widget 1", 10.0, 10.0)),
                bottom_right: Some(keyed("widget 1", 100.0, 100.0)),
                null_slot: Some(keyed("widget 1", 50.0, 50.0)),
            },
        );
    }

    /// "SlottedRenderObjectWidget test"'s paint expectations and its final unmount: each slot
    /// paints its child at the child's offset, nothing paints when empty, and the slotted
    /// element unmounts.
    #[test]
    fn slotted_children_paint_at_their_offsets_and_the_element_unmounts() {
        use crate::widgets::basic::ColoredBox;
        use waft_embedder::Color;
        use waft_embedder::valo::Op;

        let yellow = Color::new(0xFFFFFF00);
        let green = Color::new(0xFF00FF00);
        let colored = |color: Color, width: f64, height: f64| {
            SizedBox::new()
                .width(width)
                .height(height)
                .child(ColoredBox::new(color))
                .into_widget()
        };
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount_diagonal(
            &mut app,
            None,
            Diagonal {
                top_left: Some(colored(yellow, 40.0, 50.0)),
                bottom_right: Some(colored(green, 55.0, 60.0)),
                null_slot: None,
            },
        )
        .expect("mounted");
        let ops = scene_ops(&mut app, &harness);
        let rects: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::DrawRect { rect, paint, .. } => Some((*rect, paint.color)),
                _ => None,
            })
            .collect();
        let origin = diagonal_origin(&app, &harness);
        assert_eq!(
            rects,
            vec![
                (
                    waft_embedder::Rect::from_ltwh(origin.dx(), origin.dy(), 40.0, 50.0).into(),
                    yellow.into()
                ),
                (
                    waft_embedder::Rect::from_ltwh(
                        origin.dx() + 40.0,
                        origin.dy() + 50.0,
                        55.0,
                        60.0
                    )
                    .into(),
                    green.into()
                ),
            ]
        );

        mount_diagonal(&mut app, Some(&harness), Diagonal::default());
        assert!(
            !scene_ops(&mut app, &harness)
                .iter()
                .any(|op| matches!(op, Op::DrawRect { .. })),
            "an empty diagonal paints nothing"
        );

        let element = {
            let render = diagonal(&app, &harness);
            let _ = render;
            element_of_diagonal(&app, &harness)
        };
        harness.set_child(&mut app, SizedBox::shrink().into_widget());
        harness.pump(&mut app);
        assert!(is_defunct(&app, element), "the slotted element unmounted");
    }

    fn element_of_diagonal(app: &App, harness: &Harness) -> AnyElement {
        let mut found = None;
        fn walk(app: &App, element: AnyElement, found: &mut Option<AnyElement>) {
            if element.widget(app).widget_type() == TypeId::of::<Diagonal>() {
                *found = Some(element);
                return;
            }
            element.visit_children(app, &mut |child| {
                if found.is_none() {
                    walk(app, child, found);
                }
            });
        }
        walk(app, harness.root.as_element(), &mut found);
        found.expect("the diagonal is mounted")
    }

    /// Where the diagonal sits in the view.
    fn diagonal_origin(app: &App, harness: &Harness) -> Offset {
        diagonal(app, harness)
            .as_box()
            .local_to_global(app, Offset::ZERO, None)
    }

    /// Every op of the root's scene.
    fn scene_ops(app: &mut App, harness: &Harness) -> Vec<waft_embedder::valo::Op> {
        let layer = harness
            .render_root(app)
            .as_object()
            .debug_layer(app)
            .expect("painted");
        let scene = layer.build_scene(app, waft_embedder::SceneBuilder::new());
        waft_embedder::flattened_ops(&waft_embedder::slices(&scene, |_| None).picture_with_room())
    }

    /// `_Slot` of the test: a slot type that is not an enum, named by its `Debug` form.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TestSlot(u32);

    struct RenderTest {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        slotted: SlottedContainerRenderObjectData<TestSlot, AnyRenderBox>,
    }

    impl SlottedContainerRenderObjectMixin<TestSlot> for RenderTest {
        type ChildType = AnyRenderBox;

        fn slotted_container_data(
            self: RenderHandle<Self>,
            app: &App,
        ) -> &SlottedContainerRenderObjectData<TestSlot, AnyRenderBox> {
            &self.get(app).slotted
        }

        fn slotted_container_data_mut(
            self: RenderHandle<Self>,
            app: &mut App,
        ) -> &mut SlottedContainerRenderObjectData<TestSlot, AnyRenderBox> {
            &mut self.get_mut(app).slotted
        }
    }

    impl RenderObject for RenderTest {
        waft_rendering::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let size = self.constraints(app).smallest();
            self.set_size(app, size);
        }
    }

    impl RenderBox for RenderTest {
        waft_rendering::render_box_accessors!();
    }

    /// "nameForSlot"'s non-enum case.
    #[test]
    fn a_slot_that_is_not_an_enum_is_named_by_its_debug_form() {
        let render = RenderTest {
            render_object: RenderObjectData::new(),
            render_box: RenderBoxData::new(),
            slotted: SlottedContainerRenderObjectData::new(),
        };
        assert_eq!(render.debug_name_for_slot(&TestSlot(3)), "TestSlot(3)");
    }
}
