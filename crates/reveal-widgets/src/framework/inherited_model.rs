//! Flutter counterpart: `widgets/inherited_model.dart` (`InheritedModel`,
//! `InheritedModelElement`).
//!
//! [`InheritedModelElement`] is Dart's `InheritedModelElement extends InheritedElement`: the
//! same struct as `InheritedElement` with `update_dependencies` and `notify_dependent`
//! replaced. `InheritedElement` is a concrete struct with no override point, so the shared
//! bodies are repeated here rather than inherited; see `PORTING.md`.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

use reveal_foundation::{App, Handle};

use super::element::{
    AnyElement, BuildContext, Element, ElementData, ElementLifecycle, Slot,
    debug_check_owner_build_target_exists,
};
use super::elements::{ComponentElement, ComponentElementData, ProxyElement};
use super::widget::{
    InheritedWidget, IntoWidget, KeyRef, Widget, WidgetKind, WidgetRef, downcast_widget,
};

/// An [`InheritedWidget`] that's intended to be used as the base class for models whose
/// dependents may only depend on one part or "aspect" of the overall model.
///
/// An inherited widget's dependents are unconditionally rebuilt when the inherited widget
/// changes per [`InheritedWidget::update_should_notify`]. This widget is similar except that
/// dependents aren't rebuilt unconditionally.
///
/// Widgets that depend on an [`InheritedModel`] qualify their dependence with a value that
/// indicates what "aspect" of the model they depend on. When the model is rebuilt,
/// dependents will also be rebuilt, but only if there was a change in the model that
/// corresponds to the aspect they provided.
///
/// The associated type [`Aspect`](Self::Aspect) is the type of the model aspect objects
/// (Dart's type parameter `T`).
///
/// Widgets create a dependency on an [`InheritedModel`] with a static method:
/// [`inherit_from`](Self::inherit_from). This method's `context` parameter defines the
/// subtree that will be rebuilt when the model changes. Typically the `inherit_from` method
/// is called from a model-specific static `maybe_of` or `of` methods, a convention that is
/// present in many Flutter framework classes which look things up. For example:
///
/// ```text
/// impl MyModel {
///     fn maybe_of(app: &mut App, context: BuildContext, aspect: Option<String>) -> Option<MyModel> {
///         MyModel::inherit_from(app, context, aspect).cloned()
///     }
/// }
/// ```
///
/// Calling `MyModel::maybe_of(app, context, Some("foo".into()))` means that `context`
/// should only be rebuilt when the `foo` aspect of `MyModel` changes. If the `aspect` is
/// `None`, then the model supports all aspects.
///
/// When the inherited model is rebuilt the [`update_should_notify`](InheritedWidget::update_should_notify)
/// and [`update_should_notify_dependent`](Self::update_should_notify_dependent) methods are
/// used to decide what should be rebuilt. If `update_should_notify` returns true, then the
/// inherited model's `update_should_notify_dependent` method is tested for each dependent
/// and the set of aspect objects it depends on. The `update_should_notify_dependent` method
/// must compare the set of aspect dependencies with the changes in the model itself.
///
/// The dependencies checked by `update_should_notify_dependent` are just the aspects
/// passed to `depend_on_inherited_widget_of_exact_type_with_aspect`. They're represented as
/// a set because one widget can depend on more than one aspect of the model. If a widget
/// depends on the model but doesn't specify an aspect, then changes in the model will cause
/// the widget to be rebuilt unconditionally.
///
/// A model is both an [`InheritedWidget`] and an [`InheritedModel`], so the kind tag of
/// [`IntoWidget`] cannot be inferred for it: the model type names the conversion itself
/// (`fn into_widget(self) -> WidgetRef { IntoWidget::<InheritedModelKind>::into_widget(self) }`).
///
/// See also:
///
/// * [`InheritedWidget`], an inherited widget that only notifies dependents when its value
///   is different.
/// * `InheritedNotifier`, an inherited widget whose value can be a `Listenable`, and which
///   will notify dependents whenever the value sends notifications.
pub trait InheritedModel: InheritedWidget {
    /// The type of the model aspect objects; Dart's `T`.
    type Aspect: Clone + Eq + Hash + 'static;

    /// Return true if the changes between this model and `old_widget` match any of the
    /// `dependencies`.
    fn update_should_notify_dependent(
        &self,
        old_widget: &Self,
        dependencies: &HashSet<Self::Aspect>,
    ) -> bool;

    /// Returns true if this model supports the given `aspect`.
    ///
    /// Returns true by default: this model supports all aspects.
    ///
    /// Subclasses may override this method to indicate that they do not support all model
    /// aspects. This is typically done when a model can be used to "shadow" some aspects of
    /// an ancestor.
    fn is_supported_aspect(&self, aspect: &Self::Aspect) -> bool {
        let _ = aspect;
        true
    }

    /// Makes `context` dependent on the specified `aspect` of an [`InheritedModel`] of this
    /// type.
    ///
    /// When the given `aspect` of the model changes, the `context` will be rebuilt. The
    /// [`update_should_notify_dependent`](Self::update_should_notify_dependent) method must
    /// determine if a change in the model widget corresponds to an `aspect` value.
    ///
    /// The dependencies created by this method target all [`InheritedModel`] ancestors of
    /// this type up to and including the first one for which
    /// [`is_supported_aspect`](Self::is_supported_aspect) returns true.
    ///
    /// If `aspect` is `None` this method is the same as
    /// `context.depend_on_inherited_widget_of_exact_type::<Self>()`.
    ///
    /// If no ancestor of this type exists, `None` is returned.
    ///
    /// Dart's `InheritedModel.inheritFrom<T>(context, aspect: aspect)` is
    /// `T::inherit_from(app, context, aspect)`.
    fn inherit_from(
        app: &mut App,
        context: BuildContext,
        aspect: Option<Self::Aspect>,
    ) -> Option<&Self>
    where
        Self: Sized,
    {
        let Some(aspect) = aspect else {
            return context.depend_on_inherited_widget_of_exact_type::<Self>(app);
        };

        // Create a dependency on all of the type T ancestor models up until
        // a model is found for which isSupportedAspect(aspect) is true.
        let mut models = Vec::new();
        find_models::<Self>(app, context, &aspect, &mut models);
        let last_model = *models.last()?;
        for model in models {
            let erased: Rc<dyn Any> = Rc::new(aspect.clone());
            context.depend_on_inherited_element(app, model, Some(erased));
            if model == last_model {
                let widget: &WidgetRef = model.widget(app);
                return downcast_widget::<Self>(&**widget);
            }
        }

        unreachable!("the last model is in the list");
    }
}

/// Dart's `InheritedModel._findModels`: `results` will be a list of all of `context`'s type
/// `T` ancestors concluding with the one that supports the specified model `aspect`.
fn find_models<T: InheritedModel>(
    app: &App,
    context: BuildContext,
    aspect: &T::Aspect,
    results: &mut Vec<AnyElement>,
) {
    let Some(model) = context.get_element_for_inherited_widget_of_exact_type::<T>(app) else {
        return;
    };

    results.push(model);

    let model_widget: &T = downcast_widget::<T>(&**model.widget(app))
        .expect("an InheritedModel element holds its InheritedModel");
    if model_widget.is_supported_aspect(aspect) {
        return;
    }

    let mut model_parent = None;
    model.visit_ancestor_elements(app, &mut |ancestor| {
        model_parent = Some(ancestor);
        false
    });
    let Some(model_parent) = model_parent else {
        return;
    };

    find_models::<T>(app, model_parent, aspect, results);
}

// ---------------------------------------------------------------------------------------------
// The erased form

/// The kind tag of [`IntoWidget`] for an [`InheritedModel`].
pub struct InheritedModelKind;

/// The erased form of an [`InheritedModel`].
pub struct InheritedModelWrapper<W: InheritedModel>(pub W);

impl<W: InheritedModel> IntoWidget<InheritedModelKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(InheritedModelWrapper(self))
    }
}

impl<W: InheritedModel> Widget for InheritedModelWrapper<W> {
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        InheritedModelElement::<W>::create(app, this)
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

impl<W: InheritedModel> Debug for InheritedModelWrapper<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------------------------
// InheritedModelElement

/// An [`Element`] that uses an [`InheritedModel`] as its configuration.
///
/// Dart's `_dependents` maps each dependent to `Object?`; here it holds what the model
/// element's casts assert: the set of aspects the dependent depends on, empty when it
/// depends on the whole model.
pub struct InheritedModelElement<W: InheritedModel> {
    element: ElementData,
    component: ComponentElementData,
    dependents: HashMap<AnyElement, HashSet<W::Aspect>>,
    marker: PhantomData<fn() -> W>,
}

impl<W: InheritedModel> InheritedModelElement<W> {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(InheritedModelElement::<W> {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            dependents: HashMap::new(),
            marker: PhantomData,
        })
        .as_element()
    }

    /// The widget, typed.
    pub fn widget(self: Handle<Self>, app: &App) -> &W {
        downcast_widget::<W>(&**self.as_element().widget(app))
            .expect("an InheritedModelElement holds its InheritedModel")
    }

    /// Returns the dependencies value recorded for `dependent` with
    /// [`set_dependencies`](Self::set_dependencies): the aspects it depends on, or `None`
    /// if it is not a dependent.
    pub fn get_dependencies(
        self: Handle<Self>,
        app: &App,
        dependent: AnyElement,
    ) -> Option<&HashSet<W::Aspect>> {
        app.get(self).dependents.get(&dependent)
    }

    /// Sets the value returned by [`get_dependencies`](Self::get_dependencies) value for
    /// `dependent`.
    pub fn set_dependencies(
        self: Handle<Self>,
        app: &mut App,
        dependent: AnyElement,
        value: HashSet<W::Aspect>,
    ) {
        app.get_mut(self).dependents.insert(dependent, value);
    }

    /// Called by [`notify_clients`](ProxyElement::notify_clients) for each dependent.
    ///
    /// Calls `dependent.did_change_dependencies()` when the dependent depends on the whole
    /// model, or when [`InheritedModel::update_should_notify_dependent`] says one of its
    /// aspects changed.
    pub fn notify_dependent(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &WidgetRef,
        dependent: AnyElement,
    ) {
        let Some(dependencies) = self.get_dependencies(app, dependent) else {
            return;
        };
        let should_notify = dependencies.is_empty() || {
            let old =
                downcast_widget::<W>(&**old_widget).expect("the old widget has the same type");
            self.widget(app)
                .update_should_notify_dependent(old, dependencies)
        };
        if should_notify {
            dependent.did_change_dependencies(app);
        }
    }
}

impl<W: InheritedModel> ComponentElement for InheritedModelElement<W> {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        self.proxied_child(app)
    }
}

impl<W: InheritedModel> ProxyElement for InheritedModelElement<W> {
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
        self.widget(app).child().clone()
    }

    fn updated(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        let should_notify = {
            let old =
                &downcast_widget::<W>(&*old_widget).expect("the old widget has the same type");
            self.widget(app).update_should_notify(old)
        };
        if should_notify {
            self.notify_clients(app, old_widget);
        }
    }

    fn notify_clients(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        debug_assert!(debug_check_owner_build_target_exists(
            self.as_element(),
            app,
            "notifyClients"
        ));
        let dependents: Vec<AnyElement> = app.get(self).dependents.keys().copied().collect();
        for dependent in dependents {
            if cfg!(debug_assertions) {
                // check that it really is our descendant
                let mut ancestor = dependent.parent(app);
                while let Some(current) = ancestor
                    && current != self.as_element()
                {
                    ancestor = current.parent(app);
                }
                debug_assert!(ancestor == Some(self.as_element()));
                // check that it really depends on us
                debug_assert!(dependent.does_depend_on_inherited_element(app, self.as_element()));
            }
            self.notify_dependent(app, &old_widget, dependent);
        }
    }
}

impl<W: InheritedModel> Element for InheritedModelElement<W> {
    crate::element_accessors!();

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        ComponentElement::visit_children_component(self, app, visitor);
    }

    fn render_object_attaching_child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        ComponentElement::child(self, app)
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        ComponentElement::forget_child(self, app, child);
    }

    // `debug_doing_build` keeps the trait default: `ComponentElementData` does not expose the
    // flag `ComponentElement::perform_rebuild` sets.

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ProxyElement::update(self, app, new_widget);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }

    fn update_inheritance(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Active);
        let incoming = self
            .as_element()
            .parent(app)
            .and_then(|parent| parent.inherited_elements(app));
        let mut widgets: HashMap<TypeId, AnyElement> = incoming
            .map(|incoming| (*incoming).clone())
            .unwrap_or_default();
        widgets.insert(TypeId::of::<W>(), self.as_element());
        self.as_element()
            .set_inherited_elements(app, Some(Rc::new(widgets)));
    }

    fn debug_deactivated(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).dependents.is_empty());
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Inactive);
    }

    const IS_INHERITED_ELEMENT: bool = true;

    fn update_dependencies(
        self: Handle<Self>,
        app: &mut App,
        dependent: AnyElement,
        aspect: Option<Rc<dyn Any>>,
    ) {
        if self
            .get_dependencies(app, dependent)
            .is_some_and(HashSet::is_empty)
        {
            return;
        }

        match aspect {
            None => self.set_dependencies(app, dependent, HashSet::new()),
            Some(aspect) => {
                let aspect = (*aspect)
                    .downcast_ref::<W::Aspect>()
                    .expect("an InheritedModel dependency names one of the model's aspects")
                    .clone();
                app.get_mut(self)
                    .dependents
                    .entry(dependent)
                    .or_default()
                    .insert(aspect);
            }
        }
    }

    fn remove_dependent(self: Handle<Self>, app: &mut App, dependent: AnyElement) {
        app.get_mut(self).dependents.remove(&dependent);
    }
}
