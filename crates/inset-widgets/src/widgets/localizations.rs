//! Flutter `widgets/localizations.dart`.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::ops::Deref;
use std::rc::Rc;

use inset_embedder::{Locale, TextDirection};
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Completer, CompleterFuture, Handle, wait_all,
};
use inset_rendering::RendererBinding;

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};
use crate::framework::{
    BuildContext, GlobalKey, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    WidgetRef,
};
use crate::widgets::app::{
    LocaleListResolutionCallback, LocaleResolutionCallback, basic_locale_list_resolution,
};
use crate::widgets::basic::{Directionality, SizedBox};

/// The resources every delegate loaded, keyed by the delegate's [`type`](AnyLocalizationsDelegate::type).
///
/// Each value holds the `Rc<T>` a [`LocalizationsDelegate<T>`] produced.
type TypeToResources = HashMap<TypeId, Rc<dyn Any>>;

// Used by load_all() to record LocalizationsDelegate.load() futures we're
// waiting for.
struct Pending {
    delegate: LocalizationsDelegateRef,
    future_value: CompleterFuture<Rc<dyn Any>>,
}

// A utility function used by Localizations to generate one future
// that completes when all of the LocalizationsDelegate.load() futures
// complete. The returned map is indexed by each delegate's type.
//
// The input future values must have distinct types.
//
// The returned future will resolve when all of the input map's
// future values have resolved. If all of the input map's values are
// already complete (Dart's SynchronousFutures) then a complete future
// is returned immediately.
//
// This is more complicated than just applying wait_all to input
// because some of the input.values may be already complete. We don't want
// to wait_all for those.
fn load_all(
    app: &mut App,
    locale: &Locale,
    all_delegates: &[LocalizationsDelegateRef],
) -> CompleterFuture<Rc<TypeToResources>> {
    let mut output = TypeToResources::new();
    let mut pending_list: Option<Vec<Pending>> = None;

    // Only load the first delegate for each delegate type that supports
    // locale.languageCode.
    let mut types = HashSet::new();
    let mut delegates = Vec::new();
    for delegate in all_delegates {
        if !types.contains(&delegate.r#type()) && delegate.is_supported(locale) {
            types.insert(delegate.r#type());
            delegates.push(delegate);
        }
    }

    for delegate in delegates {
        let input_value = delegate.load(app, locale);
        if let Some(completed_value) = input_value.peek() {
            // inputValue was a SynchronousFuture
            let r#type = delegate.r#type();
            debug_assert!(!output.contains_key(&r#type));
            output.insert(r#type, completed_value);
        } else {
            pending_list.get_or_insert_default().push(Pending {
                delegate: delegate.clone(),
                future_value: input_value,
            });
        }
    }

    // All of the delegate.load() values were synchronous futures, we're done.
    let Some(pending_list) = pending_list else {
        return CompleterFuture::ready(Rc::new(output));
    };

    // Some of delegate.load() values were asynchronous futures. Wait for them.
    let future_values: Vec<_> = pending_list
        .iter()
        .map(|pending| pending.future_value.clone())
        .collect();
    CompleterFuture::spawn(app, async move {
        let values = wait_all(future_values).await;
        debug_assert_eq!(values.len(), pending_list.len());
        for (pending, value) in pending_list.iter().zip(values) {
            let r#type = pending.delegate.r#type();
            debug_assert!(!output.contains_key(&r#type));
            output.insert(r#type, value);
        }
        Rc::new(output)
    })
}

// ---------------------------------------------------------------------------------------------
// LocalizationsDelegate

/// A factory for a set of localized resources of type `T`, to be loaded by a
/// [`Localizations`] widget.
///
/// Typical applications have one [`Localizations`] widget which is created by the
/// `WidgetsApp` and configured with the app's `localizationsDelegates` parameter (a list
/// of delegates). The delegate's [`type`](AnyLocalizationsDelegate::type) is used to
/// identify the object created by an individual delegate's [`load`](Self::load) method.
///
/// An example of a class used as the value of `T` here would be `MaterialLocalizations`.
pub trait LocalizationsDelegate<T: ?Sized + 'static>: Debug + 'static {
    /// Whether resources for the given locale can be loaded by this delegate.
    ///
    /// Return true if the instance of `T` loaded by this delegate's [`load`](Self::load)
    /// method supports the given `locale`'s language.
    fn is_supported(&self, locale: &Locale) -> bool;

    /// Start loading the resources for `locale`. The returned future completes
    /// when the resources have finished loading.
    ///
    /// It's assumed that this method will return an object that contains a
    /// collection of related resources (typically defined with one method per
    /// resource). The object will be retrieved with [`Localizations::of`].
    ///
    /// A delegate whose resources are at hand returns [`CompleterFuture::ready`], Dart's
    /// `SynchronousFuture`: the [`Localizations`] widget then builds at once instead of
    /// deferring the first frame until the future completes.
    fn load(&self, app: &mut App, locale: &Locale) -> CompleterFuture<Rc<T>>;

    /// Returns true if the resources for this delegate should be loaded
    /// again by calling the [`load`](Self::load) method.
    ///
    /// This method is called whenever its [`Localizations`] widget is
    /// rebuilt. If it returns true then dependent widgets will be rebuilt
    /// after [`load`](Self::load) has completed.
    fn should_reload(&self, old: &Self) -> bool;
}

/// The erased form of a [`LocalizationsDelegate`]: the delegate as a [`Localizations`]
/// widget holds it, without its resource type.
pub trait AnyLocalizationsDelegate: Debug {
    /// See [`LocalizationsDelegate::is_supported`].
    fn is_supported(&self, locale: &Locale) -> bool;

    /// See [`LocalizationsDelegate::load`]; the value holds the `Rc<T>`.
    fn load(&self, app: &mut App, locale: &Locale) -> CompleterFuture<Rc<dyn Any>>;

    /// See [`LocalizationsDelegate::should_reload`]; false when `old` is a delegate of
    /// another type.
    fn should_reload(&self, old: &dyn AnyLocalizationsDelegate) -> bool;

    /// The type of the object returned by the [`load`](Self::load) method, T by default.
    ///
    /// This type is used to retrieve the object "loaded" by this
    /// [`LocalizationsDelegate`] from the [`Localizations`] inherited widget.
    /// For example the object loaded by `LocalizationsDelegate<Foo>` would
    /// be retrieved with:
    ///
    /// ```text
    /// Localizations::of::<Foo>(app, context)
    /// ```
    ///
    /// It's rarely necessary to override this getter.
    fn r#type(&self) -> TypeId;

    /// The delegate's own type, Dart's `runtimeType`.
    fn runtime_type(&self) -> TypeId;

    /// The delegate as `Any`, for [`should_reload`](Self::should_reload).
    fn as_any(&self) -> &dyn Any;
}

/// A shared, erased [`LocalizationsDelegate`].
#[derive(Clone)]
pub struct LocalizationsDelegateRef(Rc<dyn AnyLocalizationsDelegate>);

impl LocalizationsDelegateRef {
    /// Erases `delegate`.
    pub fn new<T: ?Sized + 'static>(delegate: impl LocalizationsDelegate<T>) -> Self {
        LocalizationsDelegateRef(Rc::new(ErasedDelegate {
            delegate,
            resources: std::marker::PhantomData::<fn() -> Rc<T>>,
        }))
    }
}

impl Deref for LocalizationsDelegateRef {
    type Target = dyn AnyLocalizationsDelegate;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Debug for LocalizationsDelegateRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

struct ErasedDelegate<T: ?Sized, D> {
    delegate: D,
    resources: std::marker::PhantomData<fn() -> Rc<T>>,
}

impl<T: ?Sized + 'static, D: LocalizationsDelegate<T>> Debug for ErasedDelegate<T, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.delegate, f)
    }
}

impl<T: ?Sized + 'static, D: LocalizationsDelegate<T>> AnyLocalizationsDelegate
    for ErasedDelegate<T, D>
{
    fn is_supported(&self, locale: &Locale) -> bool {
        self.delegate.is_supported(locale)
    }

    fn load(&self, app: &mut App, locale: &Locale) -> CompleterFuture<Rc<dyn Any>> {
        let resources = self.delegate.load(app, locale);
        // A complete future stays complete, so `load_all` still tells a synchronous delegate
        // apart; a pending one is forwarded when it completes.
        if let Some(value) = resources.peek() {
            return CompleterFuture::ready(erase(value));
        }
        let completer = Completer::new();
        let future = completer.future();
        resources.then(app, move |app, value| completer.complete(app, erase(value)));
        future
    }

    fn should_reload(&self, old: &dyn AnyLocalizationsDelegate) -> bool {
        old.as_any()
            .downcast_ref::<Self>()
            .is_some_and(|old| self.delegate.should_reload(&old.delegate))
    }

    fn r#type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn runtime_type(&self) -> TypeId {
        TypeId::of::<D>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The `Rc<T>` as [`TypeToResources`] stores it.
fn erase<T: ?Sized + 'static>(resources: Rc<T>) -> Rc<dyn Any> {
    Rc::new(resources)
}

// ---------------------------------------------------------------------------------------------
// WidgetsLocalizations

/// Interface for localized resource values for the lowest levels of the Flutter
/// framework.
///
/// This class also maps locales to a specific [`Directionality`] using the
/// [`text_direction`](Self::text_direction) property.
///
/// See also:
///
///  * [`DefaultWidgetsLocalizations`], which implements this interface and
///    supports a variety of locales.
pub trait WidgetsLocalizations: 'static {
    /// The reading direction for text in this locale.
    fn text_direction(&self) -> TextDirection;

    /// The semantics label used for `SliverReorderableList` to reorder an item in the
    /// list to the start of the list.
    fn reorder_item_to_start(&self) -> String;

    /// The semantics label used for `SliverReorderableList` to reorder an item in the
    /// list to the end of the list.
    fn reorder_item_to_end(&self) -> String;

    /// The semantics label used for `SliverReorderableList` to reorder an item in the
    /// list one space up the list.
    fn reorder_item_up(&self) -> String;

    /// The semantics label used for `SliverReorderableList` to reorder an item in the
    /// list one space down the list.
    fn reorder_item_down(&self) -> String;

    /// The semantics label used for `SliverReorderableList` to reorder an item in the
    /// list one space left in the list.
    fn reorder_item_left(&self) -> String;

    /// The semantics label used for `SliverReorderableList` to reorder an item in the
    /// list one space right in the list.
    fn reorder_item_right(&self) -> String;

    /// Label for "search results found" announcement.
    fn search_results_found(&self) -> String {
        String::from("Search results found")
    }

    /// Label for "no results found" announcement.
    fn no_results_found(&self) -> String {
        String::from("No results found")
    }

    /// The label for the copy button on the text selection toolbar.
    fn copy_button_label(&self) -> String;

    /// The label for the cut button on the text selection toolbar.
    fn cut_button_label(&self) -> String;

    /// The label for the paste button on the text selection toolbar.
    fn paste_button_label(&self) -> String;

    /// The label for the select all button on the text selection toolbar.
    fn select_all_button_label(&self) -> String;

    /// The label for the look up button on the text selection toolbar.
    fn look_up_button_label(&self) -> String;

    /// The label for the search web button on the text selection toolbar.
    fn search_web_button_label(&self) -> String;

    /// The label for the share button on the text selection toolbar.
    fn share_button_label(&self) -> String;

    /// The semantics label used for a `RadioGroup` widget when the radio button is
    /// unselected.
    fn radio_button_unselected_label(&self) -> String;
}

impl dyn WidgetsLocalizations {
    /// The `WidgetsLocalizations` from the closest [`Localizations`] instance
    /// that encloses the given context.
    ///
    /// This method is just a convenient shorthand for:
    /// `Localizations::of::<dyn WidgetsLocalizations>(app, context)`.
    ///
    /// References to the localized resources defined by this class are typically
    /// written in terms of this method. For example:
    ///
    /// ```text
    /// <dyn WidgetsLocalizations>::of(app, context).reorder_item_up()
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> Rc<dyn WidgetsLocalizations> {
        Localizations::of::<dyn WidgetsLocalizations>(app, context).expect(
            "No WidgetsLocalizations found. WidgetsLocalizations.of requires a \
             Localizations ancestor with a WidgetsLocalizations delegate.",
        )
    }
}

#[derive(Debug)]
struct WidgetsLocalizationsDelegate;

impl LocalizationsDelegate<dyn WidgetsLocalizations> for WidgetsLocalizationsDelegate {
    fn is_supported(&self, _locale: &Locale) -> bool {
        true
    }

    fn load(
        &self,
        _app: &mut App,
        locale: &Locale,
    ) -> CompleterFuture<Rc<dyn WidgetsLocalizations>> {
        DefaultWidgetsLocalizations::load(locale)
    }

    fn should_reload(&self, _old: &Self) -> bool {
        false
    }
}

/// US English localizations for the widgets library.
///
/// See also:
///
///  * `GlobalWidgetsLocalizations`, which provides widgets localizations for
///    many languages.
///  * `WidgetsApp.localizationsDelegates`, which automatically includes
///    [`DefaultWidgetsLocalizations::delegate`] by default.
#[derive(Debug, Default)]
pub struct DefaultWidgetsLocalizations;

impl DefaultWidgetsLocalizations {
    /// Construct an object that defines the localized values for the widgets
    /// library for US English (only).
    ///
    /// [`delegate`](Self::delegate) is the delegate.
    pub const fn new() -> DefaultWidgetsLocalizations {
        DefaultWidgetsLocalizations
    }

    /// Creates an object that provides US English resource values for the
    /// lowest levels of the widgets library.
    ///
    /// The `locale` parameter is ignored.
    ///
    /// This method is typically used to create a [`LocalizationsDelegate`].
    /// The `WidgetsApp` does so by default.
    pub fn load(_locale: &Locale) -> CompleterFuture<Rc<dyn WidgetsLocalizations>> {
        CompleterFuture::ready(Rc::new(DefaultWidgetsLocalizations::new()))
    }

    /// A [`LocalizationsDelegate`] that uses [`DefaultWidgetsLocalizations::load`]
    /// to create an instance of this class.
    ///
    /// `WidgetsApp` automatically adds this value to `WidgetsApp.localizationsDelegates`.
    pub fn delegate() -> LocalizationsDelegateRef {
        LocalizationsDelegateRef::new(WidgetsLocalizationsDelegate)
    }
}

impl WidgetsLocalizations for DefaultWidgetsLocalizations {
    fn text_direction(&self) -> TextDirection {
        TextDirection::Ltr
    }

    fn reorder_item_to_start(&self) -> String {
        String::from("Move to the start")
    }

    fn reorder_item_to_end(&self) -> String {
        String::from("Move to the end")
    }

    fn reorder_item_up(&self) -> String {
        String::from("Move up")
    }

    fn reorder_item_down(&self) -> String {
        String::from("Move down")
    }

    fn reorder_item_left(&self) -> String {
        String::from("Move left")
    }

    fn reorder_item_right(&self) -> String {
        String::from("Move right")
    }

    fn copy_button_label(&self) -> String {
        String::from("Copy")
    }

    fn cut_button_label(&self) -> String {
        String::from("Cut")
    }

    fn paste_button_label(&self) -> String {
        String::from("Paste")
    }

    fn select_all_button_label(&self) -> String {
        String::from("Select all")
    }

    fn look_up_button_label(&self) -> String {
        String::from("Look Up")
    }

    fn search_web_button_label(&self) -> String {
        String::from("Search Web")
    }

    fn share_button_label(&self) -> String {
        String::from("Share")
    }

    fn radio_button_unselected_label(&self) -> String {
        String::from("Not selected")
    }
}

// ---------------------------------------------------------------------------------------------
// _LocalizationsScope

/// Dart's `_LocalizationsScope`: the inherited widget that [`Localizations`] rebuilds its
/// dependents through.
struct LocalizationsScope {
    key: Option<KeyRef>,
    locale: Locale,
    localizations_state: Handle<LocalizationsState>,
    type_to_resources: Rc<TypeToResources>,
    child: WidgetRef,
}

impl Debug for LocalizationsScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalizationsScope")
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for LocalizationsScope {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &LocalizationsScope) -> bool {
        !Rc::ptr_eq(&self.type_to_resources, &old.type_to_resources)
    }
}

// ---------------------------------------------------------------------------------------------
// Localizations

/// Defines the [`Locale`] for its `child` and the localized resources that the
/// child depends on.
///
/// ## Defining localized resources
///
/// This class is effectively an [`InheritedWidget`]. If it's rebuilt with
/// a new `locale` or a different list of `delegates` or any of its
/// delegates' [`LocalizationsDelegate::should_reload`] methods returns true,
/// then widgets that have created a dependency on this widget will be
/// rebuilt after the resources for the new locale have been loaded.
///
/// The [`Localizations`] widget also instantiates [`Directionality`] in order to
/// support the appropriate [`Directionality::of`] for its localized `child`.
///
/// Each [`LocalizationsDelegate`] in `delegates` produces a collection of localized
/// resources, an object of type `T`, one of which is retrieved with [`Localizations::of`].
///
/// `WidgetsApp` creates a [`Localizations`] widget so most apps will not need to create one.
/// The widget app's [`Localizations`] delegates can be initialized with
/// `WidgetsApp.localizationsDelegates`. The `MaterialApp` class also provides a
/// `localizationsDelegates` parameter that's just passed along to the `WidgetsApp`.
///
/// ## Loading localized resources
///
/// Localized resources are loaded by the list of [`LocalizationsDelegate`]
/// `delegates`. Each delegate is essentially a factory for a collection
/// of localized resources. There are multiple delegates because there are
/// multiple sources for localizations within an app.
///
/// If [`Localizations`] were to be rebuilt with a new `locale` then
/// the widget subtree that corresponds to [`BuildContext`] `context` would
/// be rebuilt after the corresponding resources had been loaded.
///
/// ## Localizations widgets
///
/// The delegates are the primary parameter.
pub struct Localizations {
    key: Option<KeyRef>,
    locale: Locale,
    delegates: Vec<LocalizationsDelegateRef>,
    child: Option<WidgetRef>,
    is_application_level: bool,
}

impl Localizations {
    /// Create a widget from which localizations (like translated strings) can be obtained.
    ///
    /// The delegates must include one for [`WidgetsLocalizations`].
    pub fn new(locale: Locale, delegates: Vec<LocalizationsDelegateRef>) -> Localizations {
        debug_assert!(
            delegates
                .iter()
                .any(|delegate| delegate.r#type() == TypeId::of::<dyn WidgetsLocalizations>()),
            "the delegates must include a LocalizationsDelegate<dyn WidgetsLocalizations>"
        );
        Localizations {
            key: None,
            locale,
            delegates,
            child: None,
            is_application_level: false,
        }
    }

    /// Overrides the inherited [`Locale`] or [`LocalizationsDelegate`]s for `child`.
    ///
    /// This factory constructor is used for the (rare) situation where part of an app
    /// should be localized for a different locale than the one defined for the device,
    /// or if its localizations should come from a different list of
    /// [`LocalizationsDelegate`]s than the list defined by `WidgetsApp.localizationsDelegates`.
    ///
    /// For example you could specify that `my_widget` was only to be localized for
    /// the US English locale:
    ///
    /// ```text
    /// Localizations::r#override(app, context)
    ///     .locale(Locale::new("en").country_code("US"))
    ///     .child(my_widget)
    /// ```
    ///
    /// The `locale` and `delegates` parameters default to the [`Localizations::locale_of`]
    /// and [`Localizations`] delegates values from the enclosing [`Localizations`] widget.
    /// The given `delegates` are consulted before the inherited ones.
    pub fn r#override(app: &mut App, context: BuildContext) -> LocalizationsOverride {
        LocalizationsOverride {
            key: None,
            locale: Localizations::locale_of(app, context),
            delegates: Localizations::delegates_of(app, context),
            child: None,
        }
    }

    /// Dart `Localizations(key:)`.
    pub fn key(mut self, key: KeyRef) -> Localizations {
        self.key = Some(key);
        self
    }

    /// The widget below this widget in the tree.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Localizations {
        self.child = Some(child.into_widget());
        self
    }

    /// Whether this widget is at the application level.
    ///
    /// `WidgetsApp` sets this to true.
    pub fn is_application_level(mut self, is_application_level: bool) -> Localizations {
        self.is_application_level = is_application_level;
        self
    }

    /// The locale of the Localizations widget for the widget tree that
    /// corresponds to [`BuildContext`] `context`.
    ///
    /// If no [`Localizations`] widget is in scope then the [`Localizations::locale_of`]
    /// method will throw an exception.
    pub fn locale_of(app: &mut App, context: BuildContext) -> Locale {
        let state = Self::state_of(app, context).expect(
            "Requested the Locale of a context that does not include a Localizations \
             ancestor.\nTo request the Locale, the context used to retrieve the \
             Localizations widget must be that of a widget that is a descendant of a \
             Localizations widget.",
        );
        app.get(state).locale.clone().expect(
            "Localizations.localeOf found a Localizations widget that had a unexpected null locale.",
        )
    }

    /// The locale of the Localizations widget for the widget tree that
    /// corresponds to [`BuildContext`] `context`.
    ///
    /// If no [`Localizations`] widget is in scope then this function will return
    /// null.
    pub fn maybe_locale_of(app: &mut App, context: BuildContext) -> Option<Locale> {
        let state = Self::state_of(app, context)?;
        app.get(state).locale.clone()
    }

    // There doesn't appear to be a need to make this public. It's only used
    // by Localizations.override.
    fn delegates_of(app: &mut App, context: BuildContext) -> Vec<LocalizationsDelegateRef> {
        let state = Self::state_of(app, context).expect("a Localizations ancestor was not found");
        state.widget(app).delegates.clone()
    }

    /// Returns the localized resources object of the given `type` for the widget
    /// tree that corresponds to the given `context`.
    ///
    /// Returns null if no resources object of the given `type` exists within
    /// the given `context`.
    ///
    /// This method is typically used by a static factory method on the `type`
    /// class. For example Flutter's `MaterialLocalizations` class looks up Material
    /// resources with a method defined like this:
    ///
    /// ```text
    /// pub fn of(app: &mut App, context: BuildContext) -> Rc<dyn MaterialLocalizations> {
    ///     Localizations::of::<dyn MaterialLocalizations>(app, context).expect(..)
    /// }
    /// ```
    pub fn of<T: ?Sized + 'static>(app: &mut App, context: BuildContext) -> Option<Rc<T>> {
        let state = Self::state_of(app, context)?;
        state.resources_for::<T>(app)
    }

    /// The state behind the nearest `_LocalizationsScope`, registering the dependency.
    fn state_of(app: &mut App, context: BuildContext) -> Option<Handle<LocalizationsState>> {
        context
            .depend_on_inherited_widget_of_exact_type::<LocalizationsScope>(app)
            .map(|scope| scope.localizations_state)
    }
}

impl Debug for Localizations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Localizations")
            .field("locale", &self.locale)
            .field("delegates", &self.delegates)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Localizations {
    type State = LocalizationsState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> LocalizationsState {
        LocalizationsState {
            state: StateData::new(),
            localized_resources_scope_key: GlobalKey::new(),
            type_to_resources: Rc::new(TypeToResources::new()),
            locale: None,
        }
    }
}

/// Dart's `Localizations.override` factory, before its optional arguments are known.
pub struct LocalizationsOverride {
    key: Option<KeyRef>,
    locale: Locale,
    delegates: Vec<LocalizationsDelegateRef>,
    child: Option<WidgetRef>,
}

impl LocalizationsOverride {
    /// Dart `Localizations.override(key:)`.
    pub fn key(mut self, key: KeyRef) -> LocalizationsOverride {
        self.key = Some(key);
        self
    }

    /// Dart `Localizations.override(locale:)`.
    pub fn locale(mut self, locale: Locale) -> LocalizationsOverride {
        self.locale = locale;
        self
    }

    /// Dart `Localizations.override(delegates:)`: consulted before the inherited delegates.
    pub fn delegates(mut self, delegates: Vec<LocalizationsDelegateRef>) -> LocalizationsOverride {
        self.delegates.splice(0..0, delegates);
        self
    }

    /// Dart `Localizations.override(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> LocalizationsOverride {
        self.child = Some(child.into_widget());
        self
    }
}

/// The kind tag of [`IntoWidget`] for a [`LocalizationsOverride`].
pub struct LocalizationsOverrideKind;

impl IntoWidget<LocalizationsOverrideKind> for LocalizationsOverride {
    fn into_widget(self) -> WidgetRef {
        Localizations {
            key: self.key,
            locale: self.locale,
            delegates: self.delegates,
            child: self.child,
            is_application_level: false,
        }
        .into_widget()
    }
}

/// Dart's `_LocalizationsState`.
pub struct LocalizationsState {
    state: StateData<Localizations>,
    localized_resources_scope_key: GlobalKey,
    type_to_resources: Rc<TypeToResources>,
    locale: Option<Locale>,
}

impl LocalizationsState {
    /// The locale the resources were loaded for; `None` until the first load.
    pub fn locale(self: Handle<Self>, app: &App) -> Option<&Locale> {
        app.get(self).locale.as_ref()
    }

    fn set_locale(self: Handle<Self>, app: &mut App, locale: Locale) {
        if app.get(self).locale.as_ref() == Some(&locale) {
            return;
        }
        app.platform().set_application_locale(&locale);
        app.get_mut(self).locale = Some(locale);
    }

    fn any_delegates_should_reload(self: Handle<Self>, app: &App, old: &Localizations) -> bool {
        let delegates = &self.widget(app).delegates;
        if delegates.len() != old.delegates.len() {
            return true;
        }
        delegates
            .iter()
            .zip(&old.delegates)
            .any(|(delegate, old_delegate)| {
                delegate.runtime_type() != old_delegate.runtime_type()
                    || delegate.should_reload(&**old_delegate)
            })
    }

    /// Loads the resources for `locale` from the widget's delegates.
    pub fn load(self: Handle<Self>, app: &mut App, locale: Locale) {
        let delegates = self.widget(app).delegates.clone();
        if delegates.is_empty() {
            self.set_locale(app, locale);
            return;
        }

        let type_to_resources_future = load_all(app, &locale, &delegates);

        if let Some(type_to_resources) = type_to_resources_future.peek() {
            // All of the delegates' resources loaded synchronously.
            app.get_mut(self).type_to_resources = type_to_resources;
            self.set_locale(app, locale);
        } else {
            // - Don't rebuild the dependent widgets until the resources for the new locale
            // have finished loading. Until then the old locale will continue to be used.
            // - If we're running at app startup time then defer reporting the first
            // "useful" frame until after the async load has completed.
            RendererBinding::instance(app).defer_first_frame(app);
            type_to_resources_future.then(app, move |app, value| {
                if self.mounted(app) {
                    self.set_state(app, |state| state.type_to_resources = value);
                    self.set_locale(app, locale);
                }
                RendererBinding::instance(app).allow_first_frame(app);
            });
        }
    }

    /// The resources a [`LocalizationsDelegate<T>`] loaded, if one did.
    pub fn resources_for<T: ?Sized + 'static>(self: Handle<Self>, app: &App) -> Option<Rc<T>> {
        app.get(self)
            .type_to_resources
            .get(&TypeId::of::<T>())
            .and_then(|resources| resources.downcast_ref::<Rc<T>>())
            .cloned()
    }

    fn text_direction(self: Handle<Self>, app: &App) -> TextDirection {
        self.resources_for::<dyn WidgetsLocalizations>(app)
            .expect("the delegates include one for WidgetsLocalizations")
            .text_direction()
    }
}

impl State for LocalizationsState {
    type Widget = Localizations;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let locale = self.widget(app).locale.clone();
        self.load(app, locale);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &Localizations) {
        if self.widget(app).locale != old.locale || self.any_delegates_should_reload(app, old) {
            let locale = self.widget(app).locale.clone();
            self.load(app, locale);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let Some(locale) = app.get(self).locale.clone() else {
            return SizedBox::shrink().into_widget();
        };
        let text_direction = self.text_direction(app);
        let child = self
            .widget(app)
            .child
            .clone()
            .expect("a Localizations widget needs a child");
        let state = app.get(self);
        // Dart wraps the scope in `Semantics(localeForSubtree:, container:, textDirection:)`.
        LocalizationsScope {
            key: Some(Rc::new(state.localized_resources_scope_key.clone())),
            locale,
            localizations_state: self,
            type_to_resources: Rc::clone(&state.type_to_resources),
            child: Directionality::new(text_direction, child).into_widget(),
        }
        .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// LocalizationsResolver

/// Resolves the locale for an app based on the locale it is being run in, the locale the
/// app has been configured with, and the locales the app supports.
///
/// Registers itself as a binding observer (Dart's `WidgetsBindingObserver` mixin) so that
/// [`did_change_locales`](WidgetsBindingObserverObject::did_change_locales) re-resolves.
pub struct LocalizationsResolver {
    change_notifier: ChangeNotifierData,
    localizations_delegates: Option<Vec<LocalizationsDelegateRef>>,
    locale_list_resolution_callback: Option<LocaleListResolutionCallback>,
    locale_resolution_callback: Option<LocaleResolutionCallback>,
    supported_locales: Vec<Locale>,
    locale: Option<Locale>,
    resolved_locale: Option<Locale>,
    observer: Option<WidgetsBindingObserverRef>,
}

impl LocalizationsResolver {
    /// Creates a resolver (Dart's constructor: `supportedLocales` is required; the rest are
    /// its optional named arguments, in the order [`update`](Self::update) takes them).
    pub fn new(
        app: &mut App,
        locale: Option<Locale>,
        locale_list_resolution_callback: Option<LocaleListResolutionCallback>,
        locale_resolution_callback: Option<LocaleResolutionCallback>,
        localizations_delegates: Option<Vec<LocalizationsDelegateRef>>,
        supported_locales: Vec<Locale>,
    ) -> Handle<LocalizationsResolver> {
        let this = app.create(LocalizationsResolver {
            change_notifier: ChangeNotifierData::default(),
            localizations_delegates,
            locale_list_resolution_callback,
            locale_resolution_callback,
            supported_locales,
            locale,
            resolved_locale: None,
            observer: None,
        });
        let platform_locales = app.platform().locales();
        let resolved_locale = this.resolve_locales(app, Some(&platform_locales));
        app.get_mut(this).resolved_locale = Some(resolved_locale);
        let observer: WidgetsBindingObserverRef = Rc::new(this);
        WidgetsBinding::instance(app).add_observer(app, Rc::clone(&observer));
        app.get_mut(this).observer = Some(observer);
        this
    }

    /// Unregisters from the binding, disposes the notifier, and frees the slot.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get_mut(self).observer.take() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
        app.get_mut(self).change_notifier.dispose();
        app.destroy(self);
    }

    /// Updates the resolver with the given configuration, re-resolving the locale when
    /// `supported_locales` changed.
    pub fn update(
        self: Handle<Self>,
        app: &mut App,
        locale: Option<Locale>,
        locale_list_resolution_callback: Option<LocaleListResolutionCallback>,
        locale_resolution_callback: Option<LocaleResolutionCallback>,
        localizations_delegates: Option<Vec<LocalizationsDelegateRef>>,
        supported_locales: Vec<Locale>,
    ) {
        let this = app.get_mut(self);
        this.locale = locale;
        this.locale_list_resolution_callback = locale_list_resolution_callback;
        this.locale_resolution_callback = locale_resolution_callback;
        this.localizations_delegates = localizations_delegates;
        if this.supported_locales != supported_locales {
            this.supported_locales = supported_locales;
            let platform_locales = app.platform().locales();
            self.update_resolved_locale(app, Some(&platform_locales));
        }
    }

    /// The currently resolved locale.
    pub fn locale(self: Handle<Self>, app: &mut App) -> Locale {
        match app.get(self).locale.clone() {
            Some(locale) => {
                let supported_locales = app.get(self).supported_locales.clone();
                self.resolve_locales_with(app, Some(&[locale]), &supported_locales)
            }
            None => app
                .get(self)
                .resolved_locale
                .clone()
                .expect("resolved at construction"),
        }
    }

    /// The delegates for the app's localizations, with
    /// [`DefaultWidgetsLocalizations::delegate`] appended.
    pub fn localizations_delegates(self: Handle<Self>, app: &App) -> Vec<LocalizationsDelegateRef> {
        let mut delegates = app
            .get(self)
            .localizations_delegates
            .clone()
            .unwrap_or_default();
        delegates.push(DefaultWidgetsLocalizations::delegate());
        delegates
    }

    /// The callback that resolves from the full list of preferred locales.
    pub fn locale_list_resolution_callback(
        self: Handle<Self>,
        app: &App,
    ) -> Option<LocaleListResolutionCallback> {
        app.get(self).locale_list_resolution_callback.clone()
    }

    /// The callback that resolves from the single default locale.
    pub fn locale_resolution_callback(
        self: Handle<Self>,
        app: &App,
    ) -> Option<LocaleResolutionCallback> {
        app.get(self).locale_resolution_callback.clone()
    }

    /// The locales the app supports.
    pub fn supported_locales(self: Handle<Self>, app: &App) -> &[Locale] {
        &app.get(self).supported_locales
    }

    fn update_resolved_locale(
        self: Handle<Self>,
        app: &mut App,
        preferred_locales: Option<&[Locale]>,
    ) {
        let new_locale = self.resolve_locales(app, preferred_locales);
        if app.get(self).resolved_locale.as_ref() != Some(&new_locale) {
            app.get_mut(self).resolved_locale = Some(new_locale);
            self.notify_listeners(app);
        }
    }

    fn resolve_locales(
        self: Handle<Self>,
        app: &mut App,
        preferred_locales: Option<&[Locale]>,
    ) -> Locale {
        let supported_locales = app.get(self).supported_locales.clone();
        self.resolve_locales_with(app, preferred_locales, &supported_locales)
    }

    fn resolve_locales_with(
        self: Handle<Self>,
        app: &mut App,
        preferred_locales: Option<&[Locale]>,
        supported_locales: &[Locale],
    ) -> Locale {
        // Attempt to use localeListResolutionCallback.
        if let Some(callback) = app.get(self).locale_list_resolution_callback.clone()
            && let Some(locale) = callback(preferred_locales, supported_locales)
        {
            return locale;
        }
        // localeListResolutionCallback failed, falling back to localeResolutionCallback.
        if let Some(callback) = app.get(self).locale_resolution_callback.clone()
            && let Some(locale) = callback(
                preferred_locales.and_then(|locales| locales.first()),
                supported_locales,
            )
        {
            return locale;
        }
        // Both callbacks failed, falling back to default algorithm.
        basic_locale_list_resolution(preferred_locales, supported_locales)
    }
}

impl Debug for LocalizationsResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LocalizationsResolver")
    }
}

impl ChangeNotifier for LocalizationsResolver {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl WidgetsBindingObserverObject for LocalizationsResolver {
    fn did_change_locales(self: Handle<Self>, app: &mut App, locales: Option<&[Locale]>) {
        self.update_resolved_locale(app, locales);
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::Builder;

    fn en_us() -> Locale {
        Locale::new("en").country_code("US")
    }

    /// A resource type of its own, so `Localizations::of` has two types to tell apart.
    trait Greetings: 'static {
        fn hello(&self) -> String;
    }

    struct LocaleGreetings(Locale);

    impl Greetings for LocaleGreetings {
        fn hello(&self) -> String {
            format!("hello in {}", self.0)
        }
    }

    #[derive(Debug)]
    struct GreetingsDelegate {
        loads: Rc<Cell<u32>>,
        reload: bool,
    }

    impl LocalizationsDelegate<dyn Greetings> for GreetingsDelegate {
        fn is_supported(&self, locale: &Locale) -> bool {
            locale.language_code != "xx"
        }

        fn load(&self, _app: &mut App, locale: &Locale) -> CompleterFuture<Rc<dyn Greetings>> {
            self.loads.set(self.loads.get() + 1);
            CompleterFuture::ready(Rc::new(LocaleGreetings(locale.clone())))
        }

        fn should_reload(&self, _old: &Self) -> bool {
            self.reload
        }
    }

    /// Dart's `FakeLocalizationsDelegate`: loads whenever the test completes its completer.
    struct FakeLocalizationsDelegate {
        completer: Completer<Rc<dyn Greetings>>,
    }

    impl Debug for FakeLocalizationsDelegate {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("FakeLocalizationsDelegate")
        }
    }

    impl LocalizationsDelegate<dyn Greetings> for FakeLocalizationsDelegate {
        fn is_supported(&self, _locale: &Locale) -> bool {
            true
        }

        fn load(&self, _app: &mut App, _locale: &Locale) -> CompleterFuture<Rc<dyn Greetings>> {
            self.completer.future()
        }

        fn should_reload(&self, _old: &Self) -> bool {
            false
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Seen {
        locale: Locale,
        copy_label: String,
        greeting: Option<String>,
        text_direction: TextDirection,
    }

    fn probe(seen: &Rc<RefCell<Vec<Seen>>>) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            seen.borrow_mut().push(Seen {
                locale: Localizations::locale_of(app, context),
                copy_label: <dyn WidgetsLocalizations>::of(app, context).copy_button_label(),
                greeting: Localizations::of::<dyn Greetings>(app, context)
                    .map(|greetings| greetings.hello()),
                text_direction: Directionality::of(app, context),
            });
            SizedBox::shrink().into_widget()
        })
    }

    #[test]
    fn localizations_provides_the_locale_the_resources_and_the_directionality() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let harness = Harness::mount(
            &mut app,
            Localizations::new(en_us(), vec![DefaultWidgetsLocalizations::delegate()])
                .child(probe(&seen))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            seen.borrow().as_slice(),
            [Seen {
                locale: en_us(),
                copy_label: String::from("Copy"),
                greeting: None,
                text_direction: TextDirection::Ltr,
            }]
        );
        assert!(
            RendererBinding::instance(&mut app).send_frames_to_engine(&app),
            "a delegate that loads synchronously defers no frame"
        );
    }

    #[test]
    fn locale_is_available_when_localizations_widget_stops_deferring_frames() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let delegate = FakeLocalizationsDelegate {
            completer: Completer::new(),
        };
        let completer = delegate.completer.clone();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let key = GlobalKey::new();
        let harness = Harness::mount(
            &mut app,
            Localizations::new(
                Locale::new("fo"),
                vec![
                    DefaultWidgetsLocalizations::delegate(),
                    LocalizationsDelegateRef::new(delegate),
                ],
            )
            .key(Rc::new(key.clone()))
            .child(probe(&seen))
            .into_widget(),
        );
        harness.pump(&mut app);
        let state = key
            .current_state::<LocalizationsState>(&mut app)
            .expect("the Localizations widget mounted");
        assert_eq!(state.locale(&app), None);
        assert!(seen.borrow().is_empty(), "the child is not built yet");
        let binding = RendererBinding::instance(&mut app);
        assert!(
            !binding.send_frames_to_engine(&app),
            "the first frame is deferred until the load completes"
        );

        completer.complete(&mut app, Rc::new(LocaleGreetings(Locale::new("fo"))));
        assert_eq!(
            state.locale(&app),
            None,
            "the load completes on the microtask queue"
        );
        drop(app);
        cell.checkpoint();
        let mut app = cell.borrow_mut();
        assert_eq!(state.locale(&app), Some(&Locale::new("fo")));
        assert!(binding.send_frames_to_engine(&app));
        harness.pump(&mut app);
        assert_eq!(seen.borrow()[0].locale, Locale::new("fo"));
        assert_eq!(seen.borrow()[0].greeting.as_deref(), Some("hello in fo"));
    }

    #[test]
    fn a_pending_load_keeps_the_old_locale_until_it_completes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        // One probe widget: an identical child is not rebuilt, so only the scope's
        // notification can rebuild it.
        let probe = probe(&seen).into_widget();
        let harness = Harness::mount(
            &mut app,
            Localizations::new(
                en_us(),
                vec![
                    LocalizationsDelegateRef::new(GreetingsDelegate {
                        loads: Rc::new(Cell::new(0)),
                        reload: false,
                    }),
                    DefaultWidgetsLocalizations::delegate(),
                ],
            )
            .child(Rc::clone(&probe))
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(seen.borrow()[0].greeting.as_deref(), Some("hello in en_US"));

        let delegate = FakeLocalizationsDelegate {
            completer: Completer::new(),
        };
        let completer = delegate.completer.clone();
        harness.set_child(
            &mut app,
            Localizations::new(
                Locale::new("fr"),
                vec![
                    LocalizationsDelegateRef::new(delegate),
                    DefaultWidgetsLocalizations::delegate(),
                ],
            )
            .child(Rc::clone(&probe))
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            seen.borrow().len(),
            1,
            "the dependents keep the old resources"
        );
        let binding = RendererBinding::instance(&mut app);
        assert!(!binding.send_frames_to_engine(&app));

        completer.complete(&mut app, Rc::new(LocaleGreetings(Locale::new("fr"))));
        drop(app);
        cell.checkpoint();
        let mut app = cell.borrow_mut();
        harness.pump(&mut app);
        assert!(binding.send_frames_to_engine(&app));
        assert_eq!(
            seen.borrow().len(),
            2,
            "the new resources rebuild the dependents"
        );
        assert_eq!(seen.borrow()[1].locale, Locale::new("fr"));
        assert_eq!(seen.borrow()[1].greeting.as_deref(), Some("hello in fr"));
    }

    #[test]
    fn a_delegate_loads_once_per_locale_and_again_when_it_asks_to_reload() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let loads = Rc::new(Cell::new(0));
        // One probe widget: an identical child is not rebuilt, so only the scope's
        // notification can rebuild it.
        let probe = probe(&seen).into_widget();
        let localized = |locale: Locale, reload: bool| {
            Localizations::new(
                locale,
                vec![
                    LocalizationsDelegateRef::new(GreetingsDelegate {
                        loads: Rc::clone(&loads),
                        reload,
                    }),
                    DefaultWidgetsLocalizations::delegate(),
                ],
            )
            .child(Rc::clone(&probe))
            .into_widget()
        };
        let harness = Harness::mount(&mut app, localized(en_us(), false));
        harness.pump(&mut app);
        assert_eq!(loads.get(), 1);
        assert_eq!(seen.borrow()[0].greeting.as_deref(), Some("hello in en_US"));

        harness.set_child(&mut app, localized(en_us(), false));
        harness.pump(&mut app);
        assert_eq!(loads.get(), 1, "the same locale and delegates load nothing");
        assert_eq!(seen.borrow().len(), 1, "the dependents do not rebuild");

        harness.set_child(&mut app, localized(Locale::new("fr"), false));
        harness.pump(&mut app);
        assert_eq!(loads.get(), 2, "a new locale loads");
        assert_eq!(seen.borrow()[1].greeting.as_deref(), Some("hello in fr"));

        harness.set_child(&mut app, localized(Locale::new("fr"), true));
        harness.pump(&mut app);
        assert_eq!(loads.get(), 3, "a delegate that asks to reload loads");
        assert_eq!(seen.borrow().len(), 3);
    }

    #[test]
    fn an_unsupported_locale_skips_the_delegate() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let harness = Harness::mount(
            &mut app,
            Localizations::new(
                Locale::new("xx"),
                vec![
                    LocalizationsDelegateRef::new(GreetingsDelegate {
                        loads: Rc::new(Cell::new(0)),
                        reload: false,
                    }),
                    DefaultWidgetsLocalizations::delegate(),
                ],
            )
            .child(probe(&seen))
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(seen.borrow()[0].greeting, None);
    }

    #[test]
    fn override_keeps_the_inherited_locale_and_puts_its_delegates_first() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let inner = {
            let seen = Rc::clone(&seen);
            Builder::new(move |app, context| {
                Localizations::r#override(app, context)
                    .delegates(vec![LocalizationsDelegateRef::new(GreetingsDelegate {
                        loads: Rc::new(Cell::new(0)),
                        reload: false,
                    })])
                    .child(probe(&seen))
                    .into_widget()
            })
        };
        let harness = Harness::mount(
            &mut app,
            Localizations::new(en_us(), vec![DefaultWidgetsLocalizations::delegate()])
                .child(inner)
                .into_widget(),
        );
        harness.pump(&mut app);
        let seen = seen.borrow();
        assert_eq!(seen[0].locale, en_us());
        assert_eq!(seen[0].greeting.as_deref(), Some("hello in en_US"));
        assert_eq!(seen[0].copy_label, "Copy");
    }

    #[test]
    fn the_resolver_follows_the_platform_locales_and_notifies_on_a_change() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let resolver = LocalizationsResolver::new(
            &mut app,
            None,
            None,
            None,
            None,
            vec![en_us(), Locale::new("fr").country_code("FR")],
        );
        assert_eq!(
            resolver.locale(&mut app),
            en_us(),
            "no platform locales: the first supported"
        );
        let notified = Rc::new(Cell::new(0));
        app.get_mut(resolver)
            .change_notifier
            .add_listener(inset_foundation::Listener::new({
                let notified = Rc::clone(&notified);
                move |_app| notified.set(notified.get() + 1)
            }));

        let binding = WidgetsBinding::instance(&mut app);
        binding.dispatch_locales_changed(&mut app, Some(&[Locale::new("fr").country_code("CA")]));
        assert_eq!(
            resolver.locale(&mut app),
            Locale::new("fr").country_code("FR")
        );
        assert_eq!(notified.get(), 1);
        binding.dispatch_locales_changed(&mut app, Some(&[Locale::new("fr").country_code("CA")]));
        assert_eq!(notified.get(), 1, "the same resolution does not notify");

        resolver.update(
            &mut app,
            Some(Locale::new("en").country_code("GB")),
            None,
            None,
            None,
            vec![en_us(), Locale::new("fr").country_code("FR")],
        );
        assert_eq!(
            resolver.locale(&mut app),
            en_us(),
            "an app locale resolves against the supported list"
        );
        assert_eq!(resolver.localizations_delegates(&app).len(), 1);

        resolver.dispose(&mut app);
        assert!(!app.contains(resolver));
    }

    #[test]
    fn the_resolver_prefers_its_callbacks_over_the_basic_algorithm() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let list_callback: LocaleListResolutionCallback =
            Rc::new(|_locales, _supported| Some(Locale::new("de")));
        let resolver = LocalizationsResolver::new(
            &mut app,
            None,
            Some(list_callback),
            Some(Rc::new(|_locale, _supported| Some(Locale::new("it")))),
            None,
            vec![en_us()],
        );
        assert_eq!(resolver.locale(&mut app), Locale::new("de"));
        resolver.update(
            &mut app,
            None,
            Some(Rc::new(|_locales, _supported| None)),
            Some(Rc::new(|_locale, _supported| Some(Locale::new("it")))),
            None,
            vec![en_us()],
        );
        assert_eq!(
            resolver.locale(&mut app),
            Locale::new("de"),
            "update re-resolves only when the supported locales change"
        );
        resolver.update(
            &mut app,
            None,
            Some(Rc::new(|_locales, _supported| None)),
            Some(Rc::new(|_locale, _supported| Some(Locale::new("it")))),
            None,
            vec![en_us(), Locale::new("fr")],
        );
        assert_eq!(
            resolver.locale(&mut app),
            Locale::new("it"),
            "the list callback declines"
        );
        resolver.dispose(&mut app);
    }

    #[test]
    fn maybe_locale_of_is_none_without_an_ancestor() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(Cell::new(Some(en_us())));
        let harness = Harness::mount(
            &mut app,
            Builder::new({
                let seen = Rc::clone(&seen);
                move |app, context| {
                    seen.set(Localizations::maybe_locale_of(app, context));
                    SizedBox::shrink().into_widget()
                }
            })
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(seen.take(), None);
    }
}
