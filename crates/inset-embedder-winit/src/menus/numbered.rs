//! A menu's entries numbered, for a system that names a menu's items by number: Win32's command
//! ids and the DBus menu's item ids. Every entry has its number, depth first from 1, a submenu's
//! before its own entries, so the number a system reports finds the entry the menu was built from.
//!
//! The invariant: a number is looked up in the menu it was given for; the same numbers in a
//! changed menu are other entries.

use inset_embedder::{MenuEntry, MenuId, ProvidedItem};

/// An entry with its number, and a submenu's entries with theirs.
pub(crate) struct Numbered<'a> {
    pub(crate) number: u32,
    pub(crate) entry: &'a MenuEntry,
    /// A submenu's entries; none for any other entry.
    pub(crate) entries: Vec<Numbered<'a>>,
}

/// What choosing an item does: an app's item reports its id, a provided item acts as the
/// system's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    App(MenuId),
    Provided(ProvidedItem),
}

/// `entries`, numbered depth first from 1.
pub(crate) fn numbered(entries: &[MenuEntry]) -> Vec<Numbered<'_>> {
    let mut next = 1;
    numbered_from(entries, &mut next)
}

fn numbered_from<'a>(entries: &'a [MenuEntry], next: &mut u32) -> Vec<Numbered<'a>> {
    entries
        .iter()
        .map(|entry| {
            let number = *next;
            *next += 1;
            let entries = match entry {
                MenuEntry::Submenu { entries, .. } => numbered_from(entries, next),
                MenuEntry::Item { .. } | MenuEntry::Provided { .. } | MenuEntry::Separator => {
                    Vec::new()
                }
            };
            Numbered {
                number,
                entry,
                entries,
            }
        })
        .collect()
}

/// The entry numbered `number` among `numbered`, at any depth.
pub(crate) fn find<'n, 'a>(numbered: &'n [Numbered<'a>], number: u32) -> Option<&'n Numbered<'a>> {
    numbered.iter().find_map(|entry| {
        if entry.number == number {
            Some(entry)
        } else {
            find(&entry.entries, number)
        }
    })
}

/// What choosing the entry numbered `number` of `entries` does; none for a submenu, a separator
/// or a number the menu does not have.
pub(crate) fn choice(entries: &[MenuEntry], number: u32) -> Option<Choice> {
    match find(&numbered(entries), number)?.entry {
        MenuEntry::Item { id, .. } => Some(Choice::App(*id)),
        MenuEntry::Provided { item, .. } => Some(Choice::Provided(*item)),
        MenuEntry::Submenu { .. } | MenuEntry::Separator => None,
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::MenuItem;

    use super::*;

    fn item(id: MenuId, label: &str) -> MenuEntry {
        MenuEntry::Item {
            id,
            item: MenuItem::new(label),
        }
    }

    fn menu() -> Vec<MenuEntry> {
        vec![
            item(40, "Open"),
            MenuEntry::Submenu {
                id: 41,
                label: "Recent".to_owned(),
                enabled: true,
                entries: vec![item(42, "a.txt"), item(43, "b.txt")],
            },
            MenuEntry::Separator,
            MenuEntry::Provided {
                id: 44,
                item: ProvidedItem::Copy,
                enabled: true,
            },
        ]
    }

    #[test]
    fn entries_are_numbered_depth_first_a_submenu_before_its_entries() {
        let menu = menu();
        let numbered = numbered(&menu);
        let numbers: Vec<u32> = numbered.iter().map(|entry| entry.number).collect();
        assert_eq!(numbers, [1, 2, 5, 6]);
        let inner: Vec<u32> = numbered[1]
            .entries
            .iter()
            .map(|entry| entry.number)
            .collect();
        assert_eq!(inner, [3, 4]);
    }

    #[test]
    fn a_number_finds_what_choosing_its_entry_does() {
        let menu = menu();
        assert_eq!(choice(&menu, 1), Some(Choice::App(40)));
        assert_eq!(choice(&menu, 4), Some(Choice::App(43)));
        assert_eq!(choice(&menu, 6), Some(Choice::Provided(ProvidedItem::Copy)));
        assert_eq!(choice(&menu, 2), None, "a submenu");
        assert_eq!(choice(&menu, 5), None, "a separator");
        assert_eq!(choice(&menu, 7), None, "no such entry");
    }
}
