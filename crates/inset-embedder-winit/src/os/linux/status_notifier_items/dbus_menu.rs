//! The `com.canonical.dbusmenu` interface of one icon's menu, which the desktop draws: the
//! framework's value laid out as the DBus menu's tree, each entry with its number
//! (`menus/numbered.rs`) as its id and the root as 0, and the desktop's `clicked` on an id
//! reported as the item chosen. A provided item acts as the system's own where it has one, which
//! for a tray's menu is Edit's commands, sent to the focused view; the rest are left out. A
//! shortcut is shown, and an icon given as the PNG the DBus menu takes.
//!
//! Each change of the menu counts a new revision, which the desktop is told; the invariant: an id
//! is looked up in the menu of the revision the desktop was last told.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use inset_embedder::{
    CheckState, IconImage, MenuEntry, MenuItem, MenuItemCheck, MenuShortcut, ProvidedItem,
    ShortcutModifiers, TrayIconEvent, TrayIconId,
};
use zbus::interface;
use zbus::zvariant::{OwnedValue, StructureBuilder, Value};

use super::locked;
use crate::menus::numbered::{Choice, Numbered, choice, find, numbered};
use crate::menus::{EditAction, MenuEvent, MenuEvents, dbus_menu_shortcut};

/// An entry's properties, by name.
type Properties = HashMap<String, OwnedValue>;
/// An entry of the layout: its id, its properties, and its entries, each one of these in a
/// variant, as the DBus menu's signature `(ia{sv}av)` has it.
type Layout = (i32, Properties, Vec<OwnedValue>);

/// The menu as the app last set it.
#[derive(Default)]
pub(super) struct MenuState {
    entries: Vec<MenuEntry>,
    revision: u32,
}

impl MenuState {
    /// Replaces the menu whole; answers its new revision.
    pub(super) fn replace(&mut self, entries: &[MenuEntry]) -> u32 {
        self.entries = entries.to_vec();
        self.revision = self.revision.wrapping_add(1);
        self.revision
    }
}

/// One icon's menu as the desktop reaches it.
pub(super) struct DbusMenu {
    icon: TrayIconId,
    menu: Arc<Mutex<MenuState>>,
    events: MenuEvents,
}

impl DbusMenu {
    pub(super) fn new(icon: TrayIconId, menu: Arc<Mutex<MenuState>>, events: MenuEvents) -> Self {
        DbusMenu { icon, menu, events }
    }

    /// The user chose the entry `id`.
    fn clicked(&self, id: i32) {
        let Ok(number) = u32::try_from(id) else {
            return;
        };
        match choice(&locked(&self.menu).entries, number) {
            Some(Choice::App(chosen)) => self.events.send(MenuEvent::Tray(
                self.icon,
                TrayIconEvent::MenuSelected(chosen),
            )),
            Some(Choice::Provided(item)) => {
                if let Some(action) = EditAction::of(item) {
                    self.events.send(MenuEvent::Edit(action));
                }
            }
            None => {}
        }
    }
}

#[interface(name = "com.canonical.dbusmenu")]
impl DbusMenu {
    /// The tree under `parent_id`, `recursion_depth` levels deep, or all of it at -1; every
    /// property is given, whichever are asked for.
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> zbus::fdo::Result<(u32, Layout)> {
        let menu = locked(&self.menu);
        let numbered = numbered(&menu.entries);
        let layout = if parent_id == 0 {
            (0, root_properties(), children(&numbered, recursion_depth))
        } else {
            let parent = u32::try_from(parent_id)
                .ok()
                .and_then(|number| find(&numbered, number))
                .ok_or_else(|| no_entry(parent_id))?;
            layout_of(parent, recursion_depth).ok_or_else(|| no_entry(parent_id))?
        };
        Ok((menu.revision, layout))
    }

    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, Properties)> {
        let menu = locked(&self.menu);
        let numbered = numbered(&menu.entries);
        ids.into_iter()
            .filter_map(|id| {
                let entry = find(&numbered, u32::try_from(id).ok()?)?;
                Some((id, properties(entry.entry)?))
            })
            .collect()
    }

    fn get_property(&self, id: i32, name: String) -> zbus::fdo::Result<OwnedValue> {
        let menu = locked(&self.menu);
        let numbered = numbered(&menu.entries);
        u32::try_from(id)
            .ok()
            .and_then(|number| find(&numbered, number))
            .and_then(|entry| properties(entry.entry))
            .and_then(|mut properties| properties.remove(&name))
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("no property {name} of {id}")))
    }

    fn event(&self, id: i32, event_id: &str, _data: OwnedValue, _timestamp: u32) {
        if event_id == "clicked" {
            self.clicked(id);
        }
    }

    /// Answers the ids it found no entry for.
    fn event_group(&self, events: Vec<(i32, String, OwnedValue, u32)>) -> Vec<i32> {
        let menu_size = numbered_count(&locked(&self.menu).entries);
        let mut unknown = Vec::new();
        for (id, event_id, _, _) in events {
            if id <= 0 || id as usize > menu_size {
                unknown.push(id);
            } else if event_id == "clicked" {
                self.clicked(id);
            }
        }
        unknown
    }

    /// The menu is always up to date: nothing to update before it shows.
    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }

    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }

    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }
}

fn no_entry(id: i32) -> zbus::fdo::Error {
    zbus::fdo::Error::InvalidArgs(format!("no entry {id}"))
}

/// How many entries the menu has, at every depth.
fn numbered_count(entries: &[MenuEntry]) -> usize {
    entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Submenu { entries, .. } => 1 + numbered_count(entries),
            MenuEntry::Item { .. } | MenuEntry::Provided { .. } | MenuEntry::Separator => 1,
        })
        .sum()
}

/// `entry`'s layout, `depth` levels of its entries deep; none for an entry the menu leaves out.
fn layout_of(entry: &Numbered<'_>, depth: i32) -> Option<Layout> {
    let id = i32::try_from(entry.number).ok()?;
    Some((
        id,
        properties(entry.entry)?,
        children(&entry.entries, depth),
    ))
}

/// `entries` as a layout's entries, `depth` levels deep, or all of them at a negative depth.
fn children(entries: &[Numbered<'_>], depth: i32) -> Vec<OwnedValue> {
    if depth == 0 {
        return Vec::new();
    }
    let below = if depth < 0 { depth } else { depth - 1 };
    entries
        .iter()
        .filter_map(|entry| layout_value(layout_of(entry, below)?))
        .collect()
}

fn layout_value((id, properties, children): Layout) -> Option<OwnedValue> {
    let structure = StructureBuilder::new()
        .add_field(id)
        .add_field(properties)
        .add_field(children)
        .build()
        .ok()?;
    Value::from(structure).try_into_owned().ok()
}

fn root_properties() -> Properties {
    HashMap::from([property("children-display", "submenu")])
}

/// `entry`'s properties; none for a provided item the menu leaves out.
fn properties(entry: &MenuEntry) -> Option<Properties> {
    Some(match entry {
        MenuEntry::Separator => HashMap::from([property("type", "separator")]),
        MenuEntry::Submenu { label, enabled, .. } => HashMap::from([
            property("label", mnemonic_free(label)),
            property("enabled", *enabled),
            property("children-display", "submenu"),
        ]),
        MenuEntry::Item { item, .. } => item_properties(item),
        MenuEntry::Provided { item, enabled, .. } => {
            let (label, key) = provided(*item)?;
            let mut properties =
                HashMap::from([property("label", label), property("enabled", *enabled)]);
            let shortcut = MenuShortcut {
                key: u64::from(key),
                modifiers: ShortcutModifiers {
                    control: true,
                    ..ShortcutModifiers::default()
                },
            };
            if let Some(names) = dbus_menu_shortcut(&shortcut) {
                properties.extend([property("shortcut", vec![names])]);
            }
            properties
        }
    })
}

fn item_properties(item: &MenuItem) -> Properties {
    let mut properties = HashMap::from([
        property("label", mnemonic_free(&item.label)),
        property("enabled", item.enabled),
    ]);
    let toggle = match item.check {
        MenuItemCheck::Plain => None,
        MenuItemCheck::Checkbox(state) => Some(("checkmark", toggle_state(state))),
        MenuItemCheck::Radio(on) => Some(("radio", i32::from(on))),
    };
    if let Some((toggle_type, state)) = toggle {
        properties.extend([
            property("toggle-type", toggle_type),
            property("toggle-state", state),
        ]);
    }
    if let Some(names) = item.shortcut.as_ref().and_then(dbus_menu_shortcut) {
        properties.extend([property("shortcut", vec![names])]);
    }
    if let Some(png) = item.icon.as_ref().and_then(png) {
        properties.extend([property("icon-data", png)]);
    }
    properties
}

/// A provided item's label and the key that, with Control, is its shortcut, where the tray's
/// menu has it: Edit's commands, for the focused view.
fn provided(item: ProvidedItem) -> Option<(&'static str, char)> {
    Some(match item {
        ProvidedItem::Undo => ("Undo", 'z'),
        ProvidedItem::Redo => ("Redo", 'y'),
        ProvidedItem::Cut => ("Cut", 'x'),
        ProvidedItem::Copy => ("Copy", 'c'),
        ProvidedItem::Paste => ("Paste", 'v'),
        ProvidedItem::SelectAll => ("Select All", 'a'),
        ProvidedItem::Delete
        | ProvidedItem::About
        | ProvidedItem::Services
        | ProvidedItem::Hide
        | ProvidedItem::HideOthers
        | ProvidedItem::ShowAll
        | ProvidedItem::Quit
        | ProvidedItem::Minimize
        | ProvidedItem::Zoom
        | ProvidedItem::ToggleFullScreen
        | ProvidedItem::BringAllToFront => return None,
    })
}

fn toggle_state(state: CheckState) -> i32 {
    match state {
        CheckState::Off => 0,
        CheckState::On => 1,
        CheckState::Mixed => -1,
    }
}

fn property<'v>(name: &str, value: impl Into<Value<'v>>) -> (String, OwnedValue) {
    let value = value
        .into()
        .try_into_owned()
        .expect("a value without file descriptors");
    (name.to_owned(), value)
}

/// `label` with its underscores doubled: the DBus menu reads one as the mark of the next letter's
/// mnemonic.
fn mnemonic_free(label: &str) -> String {
    label.replace('_', "__")
}

/// `image` as a PNG.
fn png(image: &IconImage) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&image.rgba).ok()?;
    writer.finish().ok()?;
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use inset_embedder::MenuItem;

    use super::*;

    fn menu() -> Vec<MenuEntry> {
        vec![
            MenuEntry::Item {
                id: 40,
                item: MenuItem::new("Show_Window").check(MenuItemCheck::Checkbox(CheckState::On)),
            },
            MenuEntry::Submenu {
                id: 41,
                label: "More".to_owned(),
                enabled: true,
                entries: vec![MenuEntry::Item {
                    id: 42,
                    item: MenuItem::new("Inner"),
                }],
            },
            MenuEntry::Provided {
                id: 43,
                item: ProvidedItem::Quit,
                enabled: true,
            },
        ]
    }

    #[test]
    fn the_layout_numbers_every_entry_and_leaves_out_what_the_tray_has_not() {
        let menu = menu();
        let numbered = numbered(&menu);
        let layout = children(&numbered, -1);
        assert_eq!(layout.len(), 2, "Quit has no counterpart in a tray's menu");
        let first = properties(&menu[0]).expect("an item's properties");
        assert_eq!(
            first.get("label").map(|label| label.to_string()),
            Some("\"Show__Window\"".to_owned())
        );
        assert!(first.contains_key("toggle-state"));
        assert_eq!(children(&numbered, 0).len(), 0);
        assert_eq!(numbered_count(&menu), 4);
    }
}
