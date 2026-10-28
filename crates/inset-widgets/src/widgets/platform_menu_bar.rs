//! Flutter counterpart: `widgets/platform_menu_bar.dart`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{
    IconImage, MenuEntry, MenuId, MenuItem, MenuItemCheck, MenuShortcut, ProvidedItem,
    ShortcutModifiers, TargetPlatform,
};
use inset_foundation::{App, Handle, VoidCallback};
use inset_services::LogicalKeyboardKey;

use crate::WidgetsBinding;
use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::actions::{Actions, IntentRef};
use crate::widgets::basic::SizedBox;
use crate::widgets::focus_manager::FocusManager;
use crate::widgets::shortcuts::{CharacterActivator, SingleActivator};

/// A class used by [`MenuSerializableShortcut`] to describe the shortcut for serialization to
/// send to the platform for rendering a [`PlatformMenuBar`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutSerialization {
    trigger: Option<LogicalKeyboardKey>,
    character: Option<String>,
    alt: Option<bool>,
    control: Option<bool>,
    meta: Option<bool>,
    shift: Option<bool>,
    internal: MenuShortcut,
}

impl ShortcutSerialization {
    /// Creates a [`ShortcutSerialization`] representing a single character.
    ///
    /// This is used by a [`CharacterActivator`] to serialize itself.
    pub fn character(
        character: impl Into<String>,
        alt: bool,
        control: bool,
        meta: bool,
    ) -> ShortcutSerialization {
        let character = character.into();
        debug_assert_eq!(character.chars().count(), 1);
        let key = character.chars().next().map_or(0, u64::from);
        ShortcutSerialization {
            internal: MenuShortcut {
                key,
                modifiers: ShortcutModifiers {
                    meta,
                    control,
                    alt,
                    shift: false,
                },
            },
            character: Some(character),
            trigger: None,
            alt: Some(alt),
            control: Some(control),
            meta: Some(meta),
            shift: None,
        }
    }

    /// Creates a [`ShortcutSerialization`] representing a specific [`LogicalKeyboardKey`] and
    /// modifiers.
    ///
    /// This is used by a [`SingleActivator`] to serialize itself.
    pub fn modifier(
        trigger: LogicalKeyboardKey,
        alt: bool,
        control: bool,
        meta: bool,
        shift: bool,
    ) -> ShortcutSerialization {
        debug_assert!(
            ![
                LogicalKeyboardKey::ALT,
                LogicalKeyboardKey::ALT_LEFT,
                LogicalKeyboardKey::ALT_RIGHT,
                LogicalKeyboardKey::CONTROL,
                LogicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::META,
                LogicalKeyboardKey::META_LEFT,
                LogicalKeyboardKey::META_RIGHT,
                LogicalKeyboardKey::SHIFT,
                LogicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_RIGHT,
            ]
            .contains(&trigger),
            "Specifying a modifier key as a trigger is not allowed. Use provided boolean \
             parameters instead."
        );
        ShortcutSerialization {
            internal: MenuShortcut {
                key: trigger.key_id,
                modifiers: ShortcutModifiers {
                    meta,
                    control,
                    alt,
                    shift,
                },
            },
            trigger: Some(trigger),
            character: None,
            alt: Some(alt),
            control: Some(control),
            meta: Some(meta),
            shift: Some(shift),
        }
    }

    /// The keyboard key that triggers this shortcut, if any.
    pub fn trigger(&self) -> Option<LogicalKeyboardKey> {
        self.trigger
    }

    /// The character that triggers this shortcut, if any.
    pub fn character_value(&self) -> Option<&str> {
        self.character.as_deref()
    }

    /// If this shortcut has a [`trigger`](Self::trigger), this indicates whether or not the alt
    /// modifier needs to be down or not.
    pub fn alt(&self) -> Option<bool> {
        self.alt
    }

    /// If this shortcut has a [`trigger`](Self::trigger), this indicates whether or not the
    /// control modifier needs to be down or not.
    pub fn control(&self) -> Option<bool> {
        self.control
    }

    /// If this shortcut has a [`trigger`](Self::trigger), this indicates whether or not the meta
    /// (also known as the Windows or Command key) modifier needs to be down or not.
    pub fn meta(&self) -> Option<bool> {
        self.meta
    }

    /// If this shortcut has a [`trigger`](Self::trigger), this indicates whether or not the shift
    /// modifier needs to be down or not.
    pub fn shift(&self) -> Option<bool> {
        self.shift
    }

    /// Converts the internal representation to the shortcut a menu item carries to the platform.
    pub fn to_menu_shortcut(&self) -> MenuShortcut {
        self.internal
    }
}

/// A mixin allowing a `ShortcutActivator` to provide data for serialization of the shortcut
/// when sending to the platform.
///
/// This is meant for those who have written their own `ShortcutActivator` implementor, and
/// would like to have it work for menus in a [`PlatformMenuBar`] as well.
///
/// Keep in mind that there are limits to the capabilities of the platform APIs, and not all
/// kinds of `ShortcutActivator`s will work with them.
pub trait MenuSerializableShortcut: Debug {
    /// Implement this in a `ShortcutActivator` implementor to allow it to be serialized for use
    /// in a [`PlatformMenuBar`].
    fn serialize_for_menu(&self) -> ShortcutSerialization;
}

pub type MenuSerializableShortcutRef = Rc<dyn MenuSerializableShortcut>;

impl MenuSerializableShortcut for SingleActivator {
    fn serialize_for_menu(&self) -> ShortcutSerialization {
        ShortcutSerialization::modifier(self.trigger, self.alt, self.control, self.meta, self.shift)
    }
}

impl MenuSerializableShortcut for CharacterActivator {
    fn serialize_for_menu(&self) -> ShortcutSerialization {
        ShortcutSerialization::character(self.character.clone(), self.alt, self.control, self.meta)
    }
}

/// An abstract delegate that can be used to set [`WidgetsBinding::set_platform_menu_delegate`]
/// to provide for managing platform menus.
///
/// This can be implemented to provide a different menu plugin than the default system-provided
/// plugin for managing [`PlatformMenuBar`] menus.
///
/// The [`set_menus`](Self::set_menus) method allows for setting of the menu hierarchy when the
/// [`PlatformMenuBar`] menu hierarchy changes.
///
/// This delegate doesn't handle the results of clicking on a menu item, which is left to the
/// implementor to handle for their implementation.
pub trait PlatformMenuDelegate {
    /// Sets the entire menu hierarchy for a platform-rendered menu bar.
    ///
    /// The `top_level_menus` argument is the list of menus that appear in the menu bar, which
    /// themselves can have children.
    ///
    /// To update the menu hierarchy or menu item state, call [`set_menus`](Self::set_menus) with
    /// the modified hierarchy, and it will overwrite the previous menu state.
    fn set_menus(&self, app: &mut App, top_level_menus: &[PlatformMenuItemRef]);

    /// Clears any existing platform-rendered menus and leaves the application with no menus.
    ///
    /// It is not necessary to call this before updating the menu with
    /// [`set_menus`](Self::set_menus).
    fn clear_menus(&self, app: &mut App);

    /// This is called by [`PlatformMenuBar`] when it is initialized, to be sure that only one is
    /// active at a time.
    ///
    /// If the lock is successfully acquired, it will return true.
    fn debug_lock_delegate(&self, context: BuildContext) -> bool;

    /// This is called by [`PlatformMenuBar`] when it is disposed, so that another one can take
    /// over.
    ///
    /// If it successfully unlocks the delegate, it will return true.
    fn debug_unlock_delegate(&self, context: BuildContext) -> bool;
}

pub type PlatformMenuDelegateRef = Rc<dyn PlatformMenuDelegate>;

/// The signature for a function that generates unique menu item IDs for serialization of a
/// [`PlatformMenuItem`].
pub type MenuItemSerializableIdGenerator<'a> = dyn FnMut(PlatformMenuItemRef) -> MenuId + 'a;

/// The platform menu delegate that hands the menus to the host's menu bar
/// (`Platform::menu_bar`), and dispatches a choice the host reports
/// (`EmbedderClient::menu_selected`) to the item chosen.
///
/// An instance of this is set on [`WidgetsBinding::platform_menu_delegate`] by default when the
/// [`WidgetsBinding`] is initialized.
pub struct DefaultPlatformMenuDelegate {
    // Map of distributed IDs to menu items.
    id_map: RefCell<HashMap<MenuId, PlatformMenuItemRef>>,
    // An ever increasing value used to dole out IDs.
    serial: Cell<MenuId>,
    // The context used to "lock" this delegate to a specific instance of PlatformMenuBar to
    // make sure there is only one.
    locked_context: Cell<Option<BuildContext>>,
}

impl DefaultPlatformMenuDelegate {
    /// Creates a [`DefaultPlatformMenuDelegate`], which hears the choices the host reports from
    /// now on.
    pub fn new(app: &mut App) -> Rc<DefaultPlatformMenuDelegate> {
        let delegate = Rc::new(DefaultPlatformMenuDelegate {
            id_map: RefCell::default(),
            serial: Cell::new(0),
            locked_context: Cell::new(None),
        });
        let handler = Rc::downgrade(&delegate);
        app.platform_callbacks_mut().on_menu_selected = Some(Rc::new(move |app, id| {
            if let Some(delegate) = handler.upgrade() {
                delegate.method_call_handler(app, id);
            }
        }));
        delegate
    }

    /// Get the next serialization ID.
    ///
    /// This is called by each item when serializing a new object so that it has a unique ID.
    fn get_id(&self, item: PlatformMenuItemRef) -> MenuId {
        self.serial.set(self.serial.get() + 1);
        self.id_map.borrow_mut().insert(self.serial.get(), item);
        self.serial.get()
    }

    // Handles the choice the host reports, to forward to the selection callbacks.
    fn method_call_handler(&self, app: &mut App, id: MenuId) {
        let item = self.id_map.borrow().get(&id).cloned();
        debug_assert!(
            item.is_some(),
            "Received a menu choice for a menu item with an ID that was not recognized: {id}"
        );
        let Some(item) = item else {
            return;
        };
        select_menu_item(app, &item);
    }
}

/// The body of Dart's `_methodCallHandler` for an item chosen: its `onSelected`, or its intent
/// invoked on the primary focus. A [`PlatformTrayIcon`](super::platform_tray_icon::PlatformTrayIcon)
/// dispatches its menu's choices the same way.
pub(crate) fn select_menu_item(app: &mut App, item: &PlatformMenuItemRef) {
    debug_assert!(
        item.on_selected().is_none() || item.on_selected_intent().is_none(),
        "Only one of PlatformMenuItem.onSelected or PlatformMenuItem.onSelectedIntent may be \
         specified"
    );
    if let Some(on_selected) = item.on_selected().cloned() {
        on_selected(app);
    }
    if let Some(intent) = item.on_selected_intent().cloned() {
        let context = FocusManager::instance(app)
            .primary_focus(app)
            .and_then(|focus| focus.context(app))
            .expect("a menu item's intent is invoked on the primary focus");
        Actions::maybe_invoke(app, context, &*intent);
    }
}

impl PlatformMenuDelegate for DefaultPlatformMenuDelegate {
    fn clear_menus(&self, app: &mut App) {
        self.set_menus(app, &[]);
    }

    fn set_menus(&self, app: &mut App, top_level_menus: &[PlatformMenuItemRef]) {
        self.id_map.borrow_mut().clear();
        let mut representation = Vec::new();
        for child_item in top_level_menus {
            representation
                .extend(Rc::clone(child_item).to_menu_entries(self, &mut |item| self.get_id(item)));
        }
        let platform = app.platform();
        if let Some(menu_bar) = platform.menu_bar() {
            menu_bar.set_menus(&representation);
        }
    }

    fn debug_lock_delegate(&self, context: BuildContext) -> bool {
        if cfg!(debug_assertions) {
            // It's OK to lock if the lock isn't set, but not OK if a different context is
            // locking it.
            if self
                .locked_context
                .get()
                .is_some_and(|locked| locked != context)
            {
                return false;
            }
            self.locked_context.set(Some(context));
        }
        true
    }

    fn debug_unlock_delegate(&self, context: BuildContext) -> bool {
        if cfg!(debug_assertions) {
            // It's OK to unlock if the lock isn't set, but not OK if a different context is
            // unlocking it.
            if self
                .locked_context
                .get()
                .is_some_and(|locked| locked != context)
            {
                return false;
            }
            self.locked_context.set(None);
        }
        true
    }
}

/// A menu bar that uses the platform's native APIs to construct and render a menu described by
/// a [`PlatformMenu`]/[`PlatformMenuItem`] hierarchy.
///
/// This widget is especially useful on macOS, where a system menu is a required part of every
/// application. Where the host has no menu bar (`Platform::menu_bar` is `None`), the menus show
/// nowhere, and the app draws its own.
///
/// As far as the framework is concerned, this widget has no visual representation, and
/// intercepts no events: it just returns the `child` from its build function. This is because
/// all of the rendering, shortcuts, and event handling for the menu is handled by the host. It
/// is only part of the widget tree to provide a convenient refresh mechanism for the menu data.
///
/// There can only be one [`PlatformMenuBar`] at a time using the same
/// [`PlatformMenuDelegate`]. It will assert if more than one is detected.
pub struct PlatformMenuBar {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
    /// The list of menu items that are the top level children of the [`PlatformMenuBar`].
    ///
    /// Whenever the menus list is modified, a new list should be provided.
    pub menus: Vec<PlatformMenuItemRef>,
}

impl PlatformMenuBar {
    /// Creates a [`PlatformMenuBar`] of `menus`.
    pub fn new(menus: Vec<PlatformMenuItemRef>) -> PlatformMenuBar {
        PlatformMenuBar {
            key: None,
            child: None,
            menus,
        }
    }

    pub fn key(mut self, key: KeyRef) -> PlatformMenuBar {
        self.key = Some(key);
        self
    }

    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> PlatformMenuBar {
        self.child = Some(child.into_widget());
        self
    }
}

impl Debug for PlatformMenuBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformMenuBar")
            .field("menus", &self.menus)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for PlatformMenuBar {
    type State = PlatformMenuBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> PlatformMenuBarState {
        PlatformMenuBarState {
            state: StateData::new(),
            descendants: Vec::new(),
        }
    }
}

pub struct PlatformMenuBarState {
    state: StateData<PlatformMenuBar>,
    descendants: Vec<PlatformMenuItemRef>,
}

impl PlatformMenuBarState {
    // Updates the data structures for the menu and send them to the platform plugin.
    fn update_menu(self: Handle<Self>, app: &mut App) {
        let menus = self.widget(app).menus.clone();
        let delegate = WidgetsBinding::instance(app).platform_menu_delegate(app);
        delegate.set_menus(app, &menus);
    }
}

impl State for PlatformMenuBarState {
    type Widget = PlatformMenuBar;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let delegate = WidgetsBinding::instance(app).platform_menu_delegate(app);
        let context = self.context(app);
        debug_assert!(
            delegate.debug_lock_delegate(context),
            "More than one active PlatformMenuBar detected. Only one active platform-rendered \
             menu bar is allowed at a time."
        );
        delegate.clear_menus(app);
        self.update_menu(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let delegate = WidgetsBinding::instance(app).platform_menu_delegate(app);
        let context = self.context(app);
        debug_assert!(
            delegate.debug_unlock_delegate(context),
            "tried to unlock the DefaultPlatformMenuDelegate more than once with context \
             {context:?}."
        );
        delegate.clear_menus(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &PlatformMenuBar) {
        let mut new_descendants = Vec::new();
        for item in &self.widget(app).menus {
            new_descendants.push(Rc::clone(item));
            new_descendants.extend(item.descendants());
        }
        if !same_items(&new_descendants, &app.get(self).descendants) {
            app.get_mut(self).descendants = new_descendants;
            self.update_menu(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        // PlatformMenuBar is really about managing the platform menu bar, and doesn't do any
        // rendering or event handling in the framework.
        self.widget(app)
            .child
            .clone()
            .unwrap_or_else(|| SizedBox::new().into_widget())
    }
}

/// Dart's `listEquals` of menu items, which compares them by identity.
pub(crate) fn same_items(a: &[PlatformMenuItemRef], b: &[PlatformMenuItemRef]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Rc::ptr_eq(a, b))
}

pub type PlatformMenuItemRef = Rc<dyn PlatformMenuItem>;

/// An item in a [`PlatformMenu`]: Dart's `PlatformMenuItem` as the base every menu entry of a
/// [`PlatformMenuBar`] extends. A plain item is a [`PlainPlatformMenuItem`].
///
/// A [`PlatformMenuItem`] may be used to define the leaves of a menu hierarchy, its
/// [`PlatformMenu`]s the branches, and [`PlatformMenuItemGroup`]s the groups divided from the
/// rest.
pub trait PlatformMenuItem: Debug {
    /// The required label used for rendering the menu item.
    fn label(&self) -> &str;

    /// The optional tooltip text that is displayed when the user hovers over the menu item.
    fn tooltip(&self) -> Option<&str> {
        None
    }

    /// The optional shortcut that selects this [`PlatformMenuItem`].
    ///
    /// This shortcut is only enabled when [`on_selected`](Self::on_selected) is set.
    fn shortcut(&self) -> Option<&MenuSerializableShortcutRef> {
        None
    }

    /// An optional callback that is called when this [`PlatformMenuItem`] is selected.
    ///
    /// At most one of [`on_selected`](Self::on_selected) and
    /// [`on_selected_intent`](Self::on_selected_intent) may be set. If neither field is set,
    /// this menu item will be disabled.
    fn on_selected(&self) -> Option<&VoidCallback> {
        None
    }

    /// Returns a callback, if any, to be invoked if the platform menu receives a "Menu.opened"
    /// method call from the platform for this item.
    ///
    /// Only items that have submenus will have this callback invoked.
    fn on_open(&self) -> Option<&VoidCallback> {
        None
    }

    /// Returns a callback, if any, to be invoked if the platform menu receives a "Menu.closed"
    /// method call from the platform for this item.
    ///
    /// Only items that have submenus will have this callback invoked.
    fn on_close(&self) -> Option<&VoidCallback> {
        None
    }

    /// An optional intent that is invoked when this [`PlatformMenuItem`] is selected.
    ///
    /// At most one of [`on_selected`](Self::on_selected) and
    /// [`on_selected_intent`](Self::on_selected_intent) may be set. If neither field is set,
    /// this menu item will be disabled.
    fn on_selected_intent(&self) -> Option<&IntentRef> {
        None
    }

    /// The check the item shows, and which.
    fn check(&self) -> MenuItemCheck {
        MenuItemCheck::Plain
    }

    /// The image the item shows beside its label.
    fn icon(&self) -> Option<&IconImage> {
        None
    }

    /// Returns all descendant [`PlatformMenuItem`]s of this item.
    ///
    /// Returns an empty list if this type of menu item doesn't have descendants.
    fn descendants(&self) -> Vec<PlatformMenuItemRef> {
        Vec::new()
    }

    /// Returns the list of group members if this menu item is a "grouping" menu item, such as
    /// [`PlatformMenuItemGroup`].
    ///
    /// Defaults to an empty list.
    fn members(&self) -> Vec<PlatformMenuItemRef> {
        Vec::new()
    }

    /// Converts the representation of this item into the entries the host's menu takes.
    fn to_menu_entries(
        self: Rc<Self>,
        delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> Vec<MenuEntry>;
}

/// The bodies of Dart's `PlatformMenuItem` base class, which its implementors reach as `super`.
pub struct PlatformMenuItemBase;

impl PlatformMenuItemBase {
    /// Converts the given [`PlatformMenuItem`] into the entry the host's menu takes.
    pub fn serialize(
        item: PlatformMenuItemRef,
        _delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> MenuEntry {
        let shortcut = item
            .shortcut()
            .map(|shortcut| shortcut.serialize_for_menu().to_menu_shortcut());
        let menu_item = MenuItem {
            label: item.label().to_owned(),
            enabled: item.on_selected().is_some() || item.on_selected_intent().is_some(),
            check: item.check(),
            shortcut,
            icon: item.icon().cloned(),
        };
        MenuEntry::Item {
            id: get_id(item),
            item: menu_item,
        }
    }
}

/// A [`PlatformMenuItem`] that is a leaf of a menu hierarchy: Dart's `PlatformMenuItem` itself.
pub struct PlainPlatformMenuItem {
    pub label: String,
    pub tooltip: Option<String>,
    pub shortcut: Option<MenuSerializableShortcutRef>,
    pub on_selected: Option<VoidCallback>,
    pub on_selected_intent: Option<IntentRef>,
    pub check: MenuItemCheck,
    pub icon: Option<IconImage>,
}

impl PlainPlatformMenuItem {
    /// Creates a [`PlainPlatformMenuItem`] labelled `label`, disabled until it has an
    /// [`on_selected`](Self::on_selected) or an [`on_selected_intent`](Self::on_selected_intent).
    pub fn new(label: impl Into<String>) -> PlainPlatformMenuItem {
        PlainPlatformMenuItem {
            label: label.into(),
            tooltip: None,
            shortcut: None,
            on_selected: None,
            on_selected_intent: None,
            check: MenuItemCheck::Plain,
            icon: None,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> PlainPlatformMenuItem {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn shortcut(mut self, shortcut: MenuSerializableShortcutRef) -> PlainPlatformMenuItem {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn on_selected(mut self, on_selected: VoidCallback) -> PlainPlatformMenuItem {
        debug_assert!(
            self.on_selected_intent.is_none(),
            "Only one of onSelected or onSelectedIntent may be specified"
        );
        self.on_selected = Some(on_selected);
        self
    }

    pub fn on_selected_intent(mut self, intent: IntentRef) -> PlainPlatformMenuItem {
        debug_assert!(
            self.on_selected.is_none(),
            "Only one of onSelected or onSelectedIntent may be specified"
        );
        self.on_selected_intent = Some(intent);
        self
    }

    pub fn check(mut self, check: MenuItemCheck) -> PlainPlatformMenuItem {
        self.check = check;
        self
    }

    pub fn icon(mut self, icon: IconImage) -> PlainPlatformMenuItem {
        self.icon = Some(icon);
        self
    }
}

impl Debug for PlainPlatformMenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformMenuItem")
            .field("label", &self.label)
            .field("tooltip", &self.tooltip)
            .field("shortcut", &self.shortcut)
            .field("enabled", &self.on_selected.is_some())
            .finish()
    }
}

impl PlatformMenuItem for PlainPlatformMenuItem {
    fn label(&self) -> &str {
        &self.label
    }

    fn tooltip(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }

    fn shortcut(&self) -> Option<&MenuSerializableShortcutRef> {
        self.shortcut.as_ref()
    }

    fn on_selected(&self) -> Option<&VoidCallback> {
        self.on_selected.as_ref()
    }

    fn on_selected_intent(&self) -> Option<&IntentRef> {
        self.on_selected_intent.as_ref()
    }

    fn check(&self) -> MenuItemCheck {
        self.check
    }

    fn icon(&self) -> Option<&IconImage> {
        self.icon.as_ref()
    }

    fn to_menu_entries(
        self: Rc<Self>,
        delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> Vec<MenuEntry> {
        vec![PlatformMenuItemBase::serialize(self, delegate, get_id)]
    }
}

/// A class for representing menu items that have child submenus.
pub struct PlatformMenu {
    pub label: String,
    pub tooltip: Option<String>,
    /// A callback that is called when this menu is opened.
    pub on_open: Option<VoidCallback>,
    /// A callback that is called when this menu is closed.
    pub on_close: Option<VoidCallback>,
    /// The menu items in the submenu opened by this menu item.
    ///
    /// If this is an empty list, this [`PlatformMenu`] will be disabled.
    pub menus: Vec<PlatformMenuItemRef>,
}

impl PlatformMenu {
    /// Creates a [`PlatformMenu`] labelled `label`, of `menus`.
    pub fn new(label: impl Into<String>, menus: Vec<PlatformMenuItemRef>) -> PlatformMenu {
        PlatformMenu {
            label: label.into(),
            tooltip: None,
            on_open: None,
            on_close: None,
            menus,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> PlatformMenu {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn on_open(mut self, on_open: VoidCallback) -> PlatformMenu {
        self.on_open = Some(on_open);
        self
    }

    pub fn on_close(mut self, on_close: VoidCallback) -> PlatformMenu {
        self.on_close = Some(on_close);
        self
    }

    /// Returns all descendant [`PlatformMenuItem`]s of this item.
    pub fn get_descendants(item: &PlatformMenu) -> Vec<PlatformMenuItemRef> {
        let mut descendants = Vec::new();
        for child in &item.menus {
            descendants.push(Rc::clone(child));
            descendants.extend(child.descendants());
        }
        descendants
    }

    /// Converts the supplied object to the entry the host's menu takes.
    pub fn serialize(
        item: Rc<PlatformMenu>,
        delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> MenuEntry {
        let mut result: Vec<MenuEntry> = Vec::new();
        for child_item in &item.menus {
            result.extend(Rc::clone(child_item).to_menu_entries(delegate, get_id));
        }
        strip_dividers(&mut result);
        let enabled = !item.menus.is_empty();
        MenuEntry::Submenu {
            label: item.label.clone(),
            id: get_id(item),
            enabled,
            entries: result,
        }
    }
}

/// The part of Dart's `PlatformMenu.serialize` that strips dividers, which a
/// [`PlatformTrayIcon`](super::platform_tray_icon::PlatformTrayIcon)'s menu, a menu of no
/// [`PlatformMenu`], strips too.
pub(crate) fn strip_dividers(result: &mut Vec<MenuEntry>) {
    // To avoid doing type checking for groups, just filter out when there are multiple
    // sequential dividers, or when they are first or last, since groups may be interleaved
    // with non-groups, and non-groups may also add dividers.
    let mut previous_is_divider: Option<bool> = None;
    result.retain(|entry| {
        let is_divider = matches!(entry, MenuEntry::Separator);
        if previous_is_divider.is_none() && is_divider {
            // Strip any leading dividers.
            return false;
        }
        if previous_is_divider == Some(true) && is_divider {
            // Strip any duplicate dividers.
            return false;
        }
        previous_is_divider = Some(is_divider);
        true
    });
    if matches!(result.last(), Some(MenuEntry::Separator)) {
        result.pop();
    }
}

impl Debug for PlatformMenu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformMenu")
            .field("label", &self.label)
            .field("enabled", &!self.menus.is_empty())
            .field("menus", &self.menus)
            .finish()
    }
}

impl PlatformMenuItem for PlatformMenu {
    fn label(&self) -> &str {
        &self.label
    }

    fn tooltip(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }

    fn on_open(&self) -> Option<&VoidCallback> {
        self.on_open.as_ref()
    }

    fn on_close(&self) -> Option<&VoidCallback> {
        self.on_close.as_ref()
    }

    fn descendants(&self) -> Vec<PlatformMenuItemRef> {
        PlatformMenu::get_descendants(self)
    }

    fn to_menu_entries(
        self: Rc<Self>,
        delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> Vec<MenuEntry> {
        vec![PlatformMenu::serialize(self, delegate, get_id)]
    }
}

/// A class that groups other menu items into sections delineated by dividers.
///
/// Visual dividers will be added before and after this group if other menu items appear in the
/// [`PlatformMenu`], and the leading one omitted if it is first and the trailing one omitted if
/// it is last in the menu.
pub struct PlatformMenuItemGroup {
    /// The [`PlatformMenuItem`]s that are members of this menu item group.
    ///
    /// An assertion will be thrown if there isn't at least one member of the group.
    pub members: Vec<PlatformMenuItemRef>,
}

impl PlatformMenuItemGroup {
    /// Creates a [`PlatformMenuItemGroup`] of `members`.
    pub fn new(members: Vec<PlatformMenuItemRef>) -> PlatformMenuItemGroup {
        PlatformMenuItemGroup { members }
    }

    /// Converts the supplied object to the entries the host's menu takes.
    pub fn serialize(
        group: &dyn PlatformMenuItem,
        delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> Vec<MenuEntry> {
        let mut entries = vec![MenuEntry::Separator];
        for item in group.members() {
            entries.extend(item.to_menu_entries(delegate, get_id));
        }
        entries.push(MenuEntry::Separator);
        entries
    }
}

impl Debug for PlatformMenuItemGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformMenuItemGroup")
            .field("members", &self.members)
            .finish()
    }
}

impl PlatformMenuItem for PlatformMenuItemGroup {
    fn label(&self) -> &str {
        ""
    }

    fn members(&self) -> Vec<PlatformMenuItemRef> {
        self.members.clone()
    }

    fn to_menu_entries(
        self: Rc<Self>,
        delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> Vec<MenuEntry> {
        debug_assert!(
            !self.members.is_empty(),
            "There must be at least one member in a PlatformMenuItemGroup"
        );
        PlatformMenuItemGroup::serialize(&*self, delegate, get_id)
    }
}

/// A class for [`PlatformMenuItem`]s that do not have handlers, but are provided by the
/// platform.
///
/// These are typically standard menu items like "Quit" or "Hide", or Edit's commands, which the
/// host sends to the focused view.
///
/// Check [`PlatformProvidedMenuItem::has_menu`] to see if the platform provides the given type.
pub struct PlatformProvidedMenuItem {
    /// The type of default menu this is.
    pub type_: PlatformProvidedMenuItemType,
    /// True if this [`PlatformProvidedMenuItem`] should be enabled or not.
    pub enabled: bool,
}

impl PlatformProvidedMenuItem {
    /// Creates a [`PlatformProvidedMenuItem`] of `type_`, enabled.
    pub fn new(type_: PlatformProvidedMenuItemType) -> PlatformProvidedMenuItem {
        PlatformProvidedMenuItem {
            type_,
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> PlatformProvidedMenuItem {
        self.enabled = enabled;
        self
    }

    /// Checks to see if the given default menu type is supported on this platform.
    pub fn has_menu(app: &App, menu: PlatformProvidedMenuItemType) -> bool {
        match app.platform().target_platform() {
            TargetPlatform::Android
            | TargetPlatform::IOS
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => false,
            TargetPlatform::MacOS => menu.provided_item().is_some(),
        }
    }
}

impl Debug for PlatformProvidedMenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformProvidedMenuItem")
            .field("type", &self.type_)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl PlatformMenuItem for PlatformProvidedMenuItem {
    fn label(&self) -> &str {
        // The label is ignored for platform provided menus.
        ""
    }

    fn to_menu_entries(
        self: Rc<Self>,
        _delegate: &dyn PlatformMenuDelegate,
        get_id: &mut MenuItemSerializableIdGenerator<'_>,
    ) -> Vec<MenuEntry> {
        let Some(item) = self.type_.provided_item() else {
            debug_assert!(
                false,
                "No platform provides a menu for {:?}. Call PlatformProvidedMenuItem::has_menu \
                 to determine this before instantiating one.",
                self.type_
            );
            return Vec::new();
        };
        let enabled = self.enabled;
        vec![MenuEntry::Provided {
            id: get_id(self),
            item,
            enabled,
        }]
    }
}

/// The list of possible platform provided, prebuilt menus for use in a [`PlatformMenuBar`].
///
/// These are menus that the platform typically provides that cannot be reproduced in the
/// framework without platform access, or the platform's own commands for the focused view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlatformProvidedMenuItemType {
    /// The system provided "About" menu item.
    About,
    /// The system provided "Quit" menu item.
    Quit,
    /// The system provided "Services" submenu.
    ServicesSubmenu,
    /// The system provided "Hide" menu item.
    Hide,
    /// The system provided "Hide Others" menu item.
    HideOtherApplications,
    /// The system provided "Show All" menu item.
    ShowAllApplications,
    /// The system provided "Start Dictation..." menu item.
    StartSpeaking,
    /// The system provided "Stop Dictation..." menu item.
    StopSpeaking,
    /// The system provided "Enter Full Screen" menu item.
    ToggleFullScreen,
    /// The system provided "Minimize" menu item.
    MinimizeWindow,
    /// The system provided "Zoom" menu item.
    ZoomWindow,
    /// The system provided "Bring To Front" menu item.
    ArrangeWindowsInFront,
    /// The system provided "Undo" menu item, sent to the focused view.
    Undo,
    /// The system provided "Redo" menu item, sent to the focused view.
    Redo,
    /// The system provided "Cut" menu item, sent to the focused view.
    Cut,
    /// The system provided "Copy" menu item, sent to the focused view.
    Copy,
    /// The system provided "Paste" menu item, sent to the focused view.
    Paste,
    /// The system provided "Select All" menu item, sent to the focused view.
    SelectAll,
    /// The system provided "Delete" menu item, sent to the focused view.
    Delete,
}

impl PlatformProvidedMenuItemType {
    /// The item the host provides for this type; none for one no host provides.
    fn provided_item(self) -> Option<ProvidedItem> {
        Some(match self {
            PlatformProvidedMenuItemType::About => ProvidedItem::About,
            PlatformProvidedMenuItemType::Quit => ProvidedItem::Quit,
            PlatformProvidedMenuItemType::ServicesSubmenu => ProvidedItem::Services,
            PlatformProvidedMenuItemType::Hide => ProvidedItem::Hide,
            PlatformProvidedMenuItemType::HideOtherApplications => ProvidedItem::HideOthers,
            PlatformProvidedMenuItemType::ShowAllApplications => ProvidedItem::ShowAll,
            PlatformProvidedMenuItemType::StartSpeaking
            | PlatformProvidedMenuItemType::StopSpeaking => return None,
            PlatformProvidedMenuItemType::ToggleFullScreen => ProvidedItem::ToggleFullScreen,
            PlatformProvidedMenuItemType::MinimizeWindow => ProvidedItem::Minimize,
            PlatformProvidedMenuItemType::ZoomWindow => ProvidedItem::Zoom,
            PlatformProvidedMenuItemType::ArrangeWindowsInFront => ProvidedItem::BringAllToFront,
            PlatformProvidedMenuItemType::Undo => ProvidedItem::Undo,
            PlatformProvidedMenuItemType::Redo => ProvidedItem::Redo,
            PlatformProvidedMenuItemType::Cut => ProvidedItem::Cut,
            PlatformProvidedMenuItemType::Copy => ProvidedItem::Copy,
            PlatformProvidedMenuItemType::Paste => ProvidedItem::Paste,
            PlatformProvidedMenuItemType::SelectAll => ProvidedItem::SelectAll,
            PlatformProvidedMenuItemType::Delete => ProvidedItem::Delete,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use inset_foundation::AppCell;
    use inset_services::LogicalKeyboardKey;
    use inset_test::{MockMenuBar, TestPlatform, TestView};

    use super::*;
    use crate::binding::run_app;
    use crate::framework::IntoWidget;
    use crate::test_harness::pump_every_view;

    /// An app on a macOS host whose menu bar is `menu_bar`.
    fn app_with_menu_bar(menu_bar: &MockMenuBar) -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::new(TestView::new(800.0, 600.0)))
            .with_menu_bar(Rc::new(menu_bar.clone()));
        AppCell::with_platform(Rc::new(platform))
    }

    fn mount(cell: &AppCell, widget: WidgetRef) {
        run_app(&mut cell.borrow_mut(), widget);
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
    }

    /// A File menu with Open, which records its choice into `chosen`, a group holding Save,
    /// disabled, and the system's Quit.
    fn menus(chosen: &Rc<RefCell<Vec<&'static str>>>) -> Vec<PlatformMenuItemRef> {
        let open = Rc::clone(chosen);
        vec![Rc::new(PlatformMenu::new(
            "File",
            vec![
                Rc::new(
                    PlainPlatformMenuItem::new("Open")
                        .shortcut(Rc::new(
                            SingleActivator::new(LogicalKeyboardKey::KEY_O).meta(true),
                        ))
                        .on_selected(Rc::new(move |_app| open.borrow_mut().push("Open"))),
                ),
                Rc::new(PlatformMenuItemGroup::new(vec![Rc::new(
                    PlainPlatformMenuItem::new("Save"),
                )])),
                Rc::new(PlatformProvidedMenuItem::new(
                    PlatformProvidedMenuItemType::Quit,
                )),
            ],
        ))]
    }

    #[test]
    fn the_menu_bar_reaches_the_host_and_a_choice_reaches_its_item() {
        let menu_bar = MockMenuBar::new();
        let cell = app_with_menu_bar(&menu_bar);
        let chosen = Rc::new(RefCell::new(Vec::new()));
        mount(&cell, PlatformMenuBar::new(menus(&chosen)).into_widget());

        let set = menu_bar.menus();
        let [
            MenuEntry::Submenu {
                label,
                enabled,
                entries,
                ..
            },
        ] = &set[..]
        else {
            panic!("one top-level menu: {set:?}");
        };
        assert_eq!(label, "File");
        assert!(enabled);
        let [
            MenuEntry::Item { item: open, .. },
            MenuEntry::Separator,
            MenuEntry::Item { item: save, .. },
            MenuEntry::Separator,
            MenuEntry::Provided { item, .. },
        ] = &entries[..]
        else {
            panic!("Open, the group between separators, and Quit: {entries:?}");
        };
        assert!(open.enabled);
        assert_eq!(
            open.shortcut,
            Some(MenuShortcut {
                key: LogicalKeyboardKey::KEY_O.key_id,
                modifiers: ShortcutModifiers {
                    meta: true,
                    ..ShortcutModifiers::default()
                },
            })
        );
        assert!(!save.enabled, "an item with no callback is disabled");
        assert_eq!(*item, ProvidedItem::Quit);

        let open_id = menu_bar.id_of(&["File", "Open"]).expect("Open has an id");
        choose(&cell, open_id);
        assert_eq!(*chosen.borrow(), ["Open"]);
    }

    /// Reports the choice of `id` as the host does, through the platform callback the delegate
    /// set.
    fn choose(cell: &AppCell, id: MenuId) {
        let mut app = cell.borrow_mut();
        let callback = app
            .platform_callbacks()
            .on_menu_selected
            .clone()
            .expect("the default delegate hears choices");
        callback(&mut app, id);
    }

    #[test]
    fn leading_trailing_and_doubled_dividers_are_stripped() {
        let menu_bar = MockMenuBar::new();
        let cell = app_with_menu_bar(&menu_bar);
        let item = || -> PlatformMenuItemRef { Rc::new(PlainPlatformMenuItem::new("Item")) };
        let group =
            |members| -> PlatformMenuItemRef { Rc::new(PlatformMenuItemGroup::new(members)) };
        mount(
            &cell,
            PlatformMenuBar::new(vec![Rc::new(PlatformMenu::new(
                "Menu",
                vec![
                    group(vec![item()]),
                    group(vec![item()]),
                    group(vec![item()]),
                ],
            ))])
            .into_widget(),
        );
        let [MenuEntry::Submenu { entries, .. }] = &menu_bar.menus()[..] else {
            panic!("one menu");
        };
        let shape: Vec<bool> = entries
            .iter()
            .map(|entry| matches!(entry, MenuEntry::Separator))
            .collect();
        assert_eq!(shape, [false, true, false, true, false]);
    }

    #[test]
    fn a_platform_without_a_menu_bar_shows_the_menus_nowhere() {
        let platform = TestPlatform::new()
            .on(TargetPlatform::Windows)
            .with_view(Rc::new(TestView::new(800.0, 600.0)));
        let cell = AppCell::with_platform(Rc::new(platform));
        let chosen = Rc::new(RefCell::new(Vec::new()));
        mount(&cell, PlatformMenuBar::new(menus(&chosen)).into_widget());
        assert!(
            !PlatformProvidedMenuItem::has_menu(&cell.borrow(), PlatformProvidedMenuItemType::Quit),
            "Windows provides no Quit item"
        );
    }
}
