//! `Widget` and its kinds. A widget is an immutable value; the tree holds it as a shared
//! [`WidgetRef`]. Dart's `class Foo extends StatelessWidget` is `impl StatelessWidget for
//! Foo`, and a value becomes a tree node with [`IntoWidget::into_widget`].

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use reveal_foundation::{App, Key};
use reveal_rendering::{
    AnyRenderObject, ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject,
    RenderObjectWithChildMixin,
};

use super::element::{AnyElement, BuildContext};
use super::elements::{
    InheritedElement, LeafRenderObjectElement, MultiChildRenderObjectElement, ParentDataElement,
    SingleChildRenderObjectElement, StatefulElement, StatelessElement,
};
use super::state::State;

/// A shared key (Dart's `Key?` on a widget).
pub type KeyRef = Rc<dyn Key>;

/// A shared, immutable widget (Dart's `Widget` reference).
pub type WidgetRef = Rc<dyn Widget>;

/// Describes the configuration for an `Element`.
///
/// Widgets are the central class hierarchy in the Flutter framework. A widget is an immutable
/// description of part of a user interface. Widgets can be inflated into elements, which
/// manage the underlying render tree.
///
/// This is the erased object every kind of widget becomes ([`IntoWidget`]); user code
/// implements a kind ([`StatelessWidget`], [`StatefulWidget`], …) rather than this trait.
pub trait Widget: Any + Debug {
    /// Controls how one widget replaces another widget in the tree.
    ///
    /// If the [`key`](Self::key) of the widget is the same as the key of the widget it is
    /// replacing (and the types are the same), the framework updates the existing element
    /// instead of creating a new one.
    fn key(&self) -> Option<&KeyRef>;

    /// Inflates this configuration to a concrete instance. `this` is this widget's shared
    /// reference, which the element keeps.
    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement;

    /// The concrete widget, for Dart's `widget as Foo`.
    fn as_any(&self) -> &dyn Any;

    /// The concrete widget's type, Dart's `runtimeType`.
    fn widget_type(&self) -> TypeId;

    /// The kind of element this widget inflates to, for `Element.updateChild`'s superclass
    /// check.
    fn kind(&self) -> WidgetKind;
}

/// Dart's `Widget._debugConcreteSubtype` classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetKind {
    Stateful,
    Stateless,
    Other,
}

/// Whether the `new_widget` can be used to update an `Element` that currently has the
/// `old_widget` as its configuration.
///
/// An element that uses a given widget as its configuration can be updated to use another
/// widget as its configuration if, and only if, the two widgets have `runtimeType` and
/// [`Widget::key`] properties that are equal.
///
/// If the widgets have no key (their key is null), then they are considered a match if they
/// have the same type, even if their children are completely different.
pub fn can_update(old_widget: &dyn Widget, new_widget: &dyn Widget) -> bool {
    old_widget.widget_type() == new_widget.widget_type()
        && keys_equal(old_widget.key(), new_widget.key())
}

/// Dart's `oldWidget.key == newWidget.key`.
pub fn keys_equal(a: Option<&KeyRef>, b: Option<&KeyRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => **a == **b,
        _ => false,
    }
}

/// Dart's `identical(a, b)` on two widget references: `Widget.==` is identity.
pub fn same_widget(a: &WidgetRef, b: &WidgetRef) -> bool {
    Rc::ptr_eq(a, b)
}

/// Dart's `widget as W`: the concrete widget behind a [`WidgetRef`], when it is a `W`.
pub fn downcast_widget<W: 'static>(widget: &dyn Widget) -> Option<&W> {
    widget.as_any().downcast_ref::<W>()
}

/// The conversion of a widget value into a tree node.
///
/// Every widget kind gets a blanket implementation, told apart by the phantom `Kind` so the
/// implementations do not overlap; a call site infers it. A [`WidgetRef`] converts to itself.
pub trait IntoWidget<Kind> {
    fn into_widget(self) -> WidgetRef;
}

/// The kind tag of [`IntoWidget`] for a [`WidgetRef`].
pub struct WidgetRefKind;

impl IntoWidget<WidgetRefKind> for WidgetRef {
    fn into_widget(self) -> WidgetRef {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// StatelessWidget

/// A widget that does not require mutable state.
///
/// A stateless widget is a widget that describes part of the user interface by building a
/// constellation of other widgets that describe the user interface more concretely. The
/// building process continues recursively until the description of the user interface is
/// fully concrete (e.g., consists entirely of `RenderObjectWidget`s, which describe concrete
/// `RenderObject`s).
pub trait StatelessWidget: Debug + 'static {
    /// See [`Widget::key`].
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// Describes the part of the user interface represented by this widget.
    ///
    /// The framework calls this method when this widget is inserted into the tree in a given
    /// [`BuildContext`] and when the dependencies of this widget change (e.g., an
    /// `InheritedWidget` referenced by this widget changes). This method can potentially be
    /// called in every frame and should not have any side effects beyond building a widget.
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef;
}

/// The kind tag of [`IntoWidget`] for a [`StatelessWidget`].
pub struct StatelessKind;

/// The erased form of a [`StatelessWidget`].
pub struct Stateless<W: StatelessWidget>(pub W);

impl<W: StatelessWidget> IntoWidget<StatelessKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(Stateless(self))
    }
}

impl<W: StatelessWidget> Widget for Stateless<W> {
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        StatelessElement::<W>::create(app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Stateless
    }
}

impl<W: StatelessWidget> Debug for Stateless<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------------------------
// StatefulWidget

/// A widget that has mutable state.
///
/// State is information that (1) can be read synchronously when the widget is built and (2)
/// might change during the lifetime of the widget. It is the responsibility of the widget
/// implementer to ensure that the [`State`] is promptly notified when such state changes,
/// using `State::set_state`.
///
/// A stateful widget is a widget that describes part of the user interface by building a
/// constellation of other widgets that describe the user interface more concretely.
///
/// Dart's `createState` returns any `State<Foo>`; here the state class is the associated
/// type [`State`](Self::State), which is what `State<T>`'s type argument already fixed.
pub trait StatefulWidget: Debug + 'static {
    /// The state this widget creates.
    type State: State<Widget = Self>;

    /// See [`Widget::key`].
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// Creates the mutable state for this widget at a given location in the tree.
    ///
    /// The framework can call this method multiple times over the lifetime of a
    /// [`StatefulWidget`]. For example, if the widget is inserted into the tree in multiple
    /// locations, the framework will create a separate [`State`] object for each location.
    /// Similarly, if the widget is removed from the tree and later inserted in to the tree
    /// again, the framework will call [`create_state`](Self::create_state) again to create a
    /// fresh [`State`] object, simplifying the lifecycle of [`State`] objects.
    fn create_state(&self) -> Self::State;
}

/// The kind tag of [`IntoWidget`] for a [`StatefulWidget`].
pub struct StatefulKind;

/// The erased form of a [`StatefulWidget`].
pub struct Stateful<W: StatefulWidget>(pub W);

impl<W: StatefulWidget> IntoWidget<StatefulKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(Stateful(self))
    }
}

impl<W: StatefulWidget> Widget for Stateful<W> {
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        StatefulElement::<W>::create(app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Stateful
    }
}

impl<W: StatefulWidget> Debug for Stateful<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------------------------
// ProxyWidget: ParentDataWidget, InheritedWidget

/// Base class for widgets that efficiently propagate information down the tree.
///
/// To obtain the nearest instance of a particular type of inherited widget from a build
/// context, use `BuildContext::depend_on_inherited_widget_of_exact_type`.
///
/// Inherited widgets, when referenced in this way, will cause the consumer to rebuild when
/// the inherited widget itself changes state.
pub trait InheritedWidget: Debug + 'static {
    /// See [`Widget::key`].
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// The widget below this widget in the tree.
    fn child(&self) -> &WidgetRef;

    /// Whether the framework should notify widgets that inherit from this widget.
    ///
    /// When this widget is rebuilt, sometimes we need to rebuild the widgets that inherit
    /// from this widget but sometimes we do not. For example, if the data held by this widget
    /// is the same as the data held by `old_widget`, then we do not need to rebuild the
    /// widgets that inherited the data held by `old_widget`.
    ///
    /// The framework distinguishes these cases by calling this function with the widget that
    /// previously occupied this location in the tree as an argument. The given widget is
    /// guaranteed to have the same `runtimeType` as this object.
    fn update_should_notify(&self, old_widget: &Self) -> bool;
}

/// The kind tag of [`IntoWidget`] for an [`InheritedWidget`].
pub struct InheritedKind;

/// The erased form of an [`InheritedWidget`].
pub struct Inherited<W: InheritedWidget>(pub W);

impl<W: InheritedWidget> IntoWidget<InheritedKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(Inherited(self))
    }
}

impl<W: InheritedWidget> Widget for Inherited<W> {
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        InheritedElement::<W>::create(app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: InheritedWidget> Debug for Inherited<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Base class for widgets that hook `ParentData` information for children of
/// `RenderObjectWidget`s.
///
/// This can be used to provide per-child configuration for `RenderObjectWidget`s with more
/// than one child. For example, `Stack` uses the `Positioned` parent data widget to position
/// each child.
pub trait ParentDataWidget: Debug + 'static {
    /// The parent data this widget writes; Dart's `T extends ParentData`.
    type ParentData: ParentData + 'static;

    /// See [`Widget::key`].
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// The widget below this widget in the tree.
    fn child(&self) -> &WidgetRef;

    /// Write the data from this widget into the given render object's parent data.
    ///
    /// The framework calls this function whenever it detects that the `RenderObject`
    /// associated with the [`child`](Self::child) has outdated `parent_data`. The parent data
    /// is the data that the parent of the render object uses when laying out and painting the
    /// render object.
    ///
    /// This function is responsible for calling `mark_needs_layout` on the render object as
    /// appropriate.
    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject);

    /// Dart's `debugIsValidRenderObject`: whether `render_object`'s parent data is the one
    /// this widget writes.
    ///
    /// The default answers `render_object.parent_data_is::<Self::ParentData>(app)`. A widget
    /// whose Dart counterpart also accepts a subclass of its `ParentData` — `Positioned` on a
    /// `_TheaterParentData` — overrides this to accept that type too.
    fn debug_is_valid_render_object(
        &self,
        app: &App,
        render_object: ::reveal_rendering::AnyRenderObject,
    ) -> bool {
        render_object.parent_data_is::<Self::ParentData>(app)
    }

    /// Whether the `ParentDataElement::apply_widget_out_of_turn` method will be allowed
    /// with this widget.
    ///
    /// This should only return true if this widget represents a `ParentData` configuration
    /// that will have no impact on the layout or paint phase.
    fn debug_can_apply_out_of_turn(&self) -> bool {
        false
    }
}

/// The kind tag of [`IntoWidget`] for a [`ParentDataWidget`].
pub struct ParentDataKind;

/// The erased form of a [`ParentDataWidget`].
pub struct ParentDataWrapper<W: ParentDataWidget>(pub W);

impl<W: ParentDataWidget> IntoWidget<ParentDataKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(ParentDataWrapper(self))
    }
}

impl<W: ParentDataWidget> Widget for ParentDataWrapper<W> {
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        ParentDataElement::<W>::create(app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: ParentDataWidget> Debug for ParentDataWrapper<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------------------------
// RenderObjectWidget

/// The parts of a `RenderObjectWidget` every kind shares: the render object it creates and
/// updates. Dart's `covariant RenderObject renderObject` parameters are typed through
/// [`RenderObject`](Self::RenderObject).
pub trait RenderObjectWidget: Debug + 'static {
    /// The render object this widget creates; Dart's `covariant RenderObject` parameter type.
    type RenderObject: RenderObject;

    /// See [`Widget::key`].
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// Creates an instance of the `RenderObject` class that this widget represents, using the
    /// configuration described by this widget.
    ///
    /// This method should not do anything with the children of the render object. That
    /// should instead be handled by the method that overrides `RenderObjectElement.mount`
    /// in the object rendered by this object's `create_element` method.
    ///
    /// Returns the type-erased handle (`handle.as_object()`), which is what the element tree holds;
    /// [`update_render_object`](Self::update_render_object) gets the typed handle back.
    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject;

    /// Copies the configuration described by this widget to the given render object, which
    /// will be of the same type as returned by this object's
    /// [`create_render_object`](Self::create_render_object).
    ///
    /// This method should not do anything to update the children of the render object. That
    /// should instead be handled by the method that overrides `RenderObjectElement.update`
    /// in the object rendered by this object's `create_element` method.
    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<Self::RenderObject>,
    ) {
        let _ = (app, context, render_object);
    }

    /// A render object previously associated with this widget has been removed from the tree.
    /// The given `RenderObject` will be of the same type as returned by this object's
    /// [`create_render_object`](Self::create_render_object).
    fn did_unmount_render_object(
        &self,
        app: &mut App,
        render_object: RenderHandle<Self::RenderObject>,
    ) {
        let _ = (app, render_object);
    }
}

/// A superclass for `RenderObjectWidget`s that configure `RenderObject` subclasses that have
/// no children.
pub trait LeafRenderObjectWidget: RenderObjectWidget {}

/// The kind tag of [`IntoWidget`] for a [`LeafRenderObjectWidget`].
pub struct LeafRenderObjectKind;

/// The erased form of a [`LeafRenderObjectWidget`].
pub struct LeafRenderObject<W: LeafRenderObjectWidget>(pub W);

impl<W: LeafRenderObjectWidget> IntoWidget<LeafRenderObjectKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(LeafRenderObject(self))
    }
}

impl<W: LeafRenderObjectWidget> Widget for LeafRenderObject<W> {
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        LeafRenderObjectElement::<W>::create(app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: LeafRenderObjectWidget> Debug for LeafRenderObject<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A superclass for `RenderObjectWidget`s that configure `RenderObject` subclasses that have
/// a single child slot.
///
/// The render object assigned to this widget should make use of
/// `RenderObjectWithChildMixin` to implement a single child model.
pub trait SingleChildRenderObjectWidget: RenderObjectWidget
where
    Self::RenderObject: RenderObjectWithChildMixin,
{
    /// The widget below this widget in the tree.
    fn child(&self) -> Option<&WidgetRef>;

    /// Creates the element that manages this widget's child.
    ///
    /// Dart's `createElement`; a widget whose element has bodies of its own
    /// (`SingleChildScrollView`'s viewport) overrides it with an element that implements
    /// `SingleChildRenderObjectElementBase`.
    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement
    where
        Self: Sized,
    {
        SingleChildRenderObjectElement::<Self>::create(app, this)
    }
}

/// The kind tag of [`IntoWidget`] for a [`SingleChildRenderObjectWidget`].
pub struct SingleChildRenderObjectKind;

/// The erased form of a [`SingleChildRenderObjectWidget`].
pub struct SingleChildRenderObject<W: SingleChildRenderObjectWidget>(pub W)
where
    W::RenderObject: RenderObjectWithChildMixin;

impl<W: SingleChildRenderObjectWidget> IntoWidget<SingleChildRenderObjectKind> for W
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    fn into_widget(self) -> WidgetRef {
        Rc::new(SingleChildRenderObject(self))
    }
}

impl<W: SingleChildRenderObjectWidget> Widget for SingleChildRenderObject<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        SingleChildRenderObjectWidget::create_element(&self.0, app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: SingleChildRenderObjectWidget> Debug for SingleChildRenderObject<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A superclass for `RenderObjectWidget`s that configure `RenderObject` subclasses that have a
/// single list of children. (This superclass only provides the storage for that child list, it
/// doesn't actually provide the updating logic.)
///
/// The render object assigned to this widget must use `ContainerRenderObjectMixin`, which
/// provides the necessary functionality to visit the children of the container render object
/// (the render object belonging to the [`children`](Self::children) widgets).
///
/// See also:
///
///  * `Stack`, which uses [`MultiChildRenderObjectWidget`].
///  * `RenderStack`, for an example implementation of the associated render object.
pub trait MultiChildRenderObjectWidget: RenderObjectWidget
where
    Self::RenderObject: ContainerRenderObjectMixin,
{
    /// The widgets below this widget in the tree.
    ///
    /// If this list is going to be mutated, it is usually wise to put a [`Key`] on each of the
    /// child widgets, so that the framework can match old configurations to new configurations
    /// and maintain the underlying render objects.
    ///
    /// Also, a widget is immutable, so directly modifying the [`children`](Self::children) of a
    /// widget already in the tree results in incorrect behaviour. Whenever the children list is
    /// modified, a new list should be provided.
    fn children(&self) -> &[WidgetRef];

    /// Creates the element that manages this widget's children.
    ///
    /// Dart's `createElement`; a widget whose element has bodies of its own (`Viewport`)
    /// overrides it with an element that implements `MultiChildRenderObjectElementBase`.
    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement
    where
        Self: Sized,
    {
        MultiChildRenderObjectElement::<Self>::create(app, this)
    }
}

/// The kind tag of [`IntoWidget`] for a [`MultiChildRenderObjectWidget`].
pub struct MultiChildRenderObjectKind;

/// The erased form of a [`MultiChildRenderObjectWidget`].
pub struct MultiChildRenderObject<W: MultiChildRenderObjectWidget>(pub W)
where
    W::RenderObject: ContainerRenderObjectMixin;

impl<W: MultiChildRenderObjectWidget> IntoWidget<MultiChildRenderObjectKind> for W
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    fn into_widget(self) -> WidgetRef {
        Rc::new(MultiChildRenderObject(self))
    }
}

impl<W: MultiChildRenderObjectWidget> Widget for MultiChildRenderObject<W>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        MultiChildRenderObjectWidget::create_element(&self.0, app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: MultiChildRenderObjectWidget> Debug for MultiChildRenderObject<W>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------------------------
// GlobalKey

/// A key that is unique across the entire app.
///
/// Global keys uniquely identify elements. Global keys provide access to other objects that
/// are associated with those elements, such as `BuildContext`. For [`StatefulWidget`]s,
/// global keys also provide access to [`State`].
///
/// Widgets that have global keys reparent their subtrees when they are moved from one
/// location in the tree to another location in the tree. In order to reparent its subtree, a
/// widget must arrive at its new location in the tree in the same animation frame in which
/// it was removed from its old location in the tree.
///
/// Reparenting an `Element` using a global key is relatively expensive, as this operation
/// will trigger a call to `State::deactivate` on the associated [`State`] and all of its
/// descendants; then force all widgets that depends on an `InheritedWidget` to rebuild.
///
/// Two global keys are equal only when they are the same key (Dart's `LabeledGlobalKey`).
/// Dart's type argument names the state; here [`current_state`](Self::current_state) does.
#[derive(Clone)]
pub struct GlobalKey {
    id: Rc<GlobalKeyIdentity>,
    debug_label: Option<String>,
}

/// The identity a [`GlobalKey`] compares by; one allocation per key.
struct GlobalKeyIdentity;

impl GlobalKey {
    /// Creates a global key.
    #[expect(
        clippy::new_without_default,
        reason = "each call creates a distinct key; Default would suggest a neutral value"
    )]
    pub fn new() -> GlobalKey {
        GlobalKey {
            id: Rc::new(GlobalKeyIdentity),
            debug_label: None,
        }
    }

    /// Creates a global key with a debugging label (Dart's `LabeledGlobalKey`).
    ///
    /// The label does not affect the key's identity.
    pub fn labeled(debug_label: impl Into<String>) -> GlobalKey {
        GlobalKey {
            id: Rc::new(GlobalKeyIdentity),
            debug_label: Some(debug_label.into()),
        }
    }

    /// The registry identity every global key compares by.
    pub fn identity(&self) -> GlobalKeyId {
        GlobalKeyId(Rc::as_ptr(&self.id) as usize)
    }

    /// The build context in which the widget with this key builds.
    ///
    /// The current context is `None` if there is no widget in the tree that matches this
    /// global key.
    pub fn current_context(&self, app: &mut App) -> Option<BuildContext> {
        self.current_element(app)
    }

    /// The widget in the tree that currently has this global key.
    ///
    /// The current widget is `None` if there is no widget in the tree that matches this
    /// global key.
    pub fn current_widget(&self, app: &mut App) -> Option<WidgetRef> {
        let element = self.current_element(app)?;
        Some(element.widget(app).clone())
    }

    /// The [`State`] for the widget in the tree that currently has this global key.
    ///
    /// The current state is `None` if (1) there is no widget in the tree that matches this
    /// global key, (2) that widget is not a [`StatefulWidget`], or the associated [`State`]
    /// object is not an `S`.
    pub fn current_state<S: State>(&self, app: &mut App) -> Option<reveal_foundation::Handle<S>> {
        let element = self.current_element(app)?;
        element.state_handle::<S>(app)
    }

    fn current_element(&self, app: &mut App) -> Option<AnyElement> {
        let owner = crate::binding::current_build_owner(app)?;
        owner.global_key_element(app, self.identity())
    }
}

impl Key for GlobalKey {
    fn eq_key(&self, other: &dyn Key) -> bool {
        (other as &dyn Any)
            .downcast_ref::<GlobalKey>()
            .is_some_and(|other| Rc::ptr_eq(&self.id, &other.id))
    }

    fn hash_key(&self, mut state: &mut dyn Hasher) {
        self.identity().hash(&mut state);
    }
}

impl Debug for GlobalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.debug_label {
            Some(label) => write!(
                f,
                "[GlobalKey#{:05x} {label}]",
                self.identity().0 & 0xF_FFFF
            ),
            None => write!(f, "[GlobalKey#{:05x}]", self.identity().0 & 0xF_FFFF),
        }
    }
}

/// The identity of a [`GlobalKey`], as the `BuildOwner` registers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlobalKeyId(usize);

/// Dart's `key is GlobalKey`: the registry identity when a key is a global key.
pub fn global_key_id(key: &KeyRef) -> Option<GlobalKeyId> {
    ((&**key) as &dyn Any)
        .downcast_ref::<GlobalKey>()
        .map(GlobalKey::identity)
}
