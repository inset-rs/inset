//! The inbox's state: Reply's `ReplyHomeViewModel` and `ReplyHomeUIState`, as one entity.
//!
//! The screens read [`ReplyHome`] while they build and call its operations from their callbacks;
//! each operation notifies, so every screen reading it rebuilds.

use std::collections::BTreeSet;

use waft_foundation::{App, Entity};

use crate::reply::data::{Email, all_emails};

/// The mail, which one is open, and which are picked for a bulk action.
#[derive(Debug)]
pub struct ReplyHome {
    pub emails: Vec<Email>,
    /// The mails picked by a long press or a tap on the avatar.
    pub selected_emails: BTreeSet<u64>,
    /// The mail the detail shows; the first one until another is opened.
    pub opened_email: Option<u64>,
    /// Whether the open mail has a page of its own, over the list, as on a phone.
    pub is_detail_only_open: bool,
}

/// One entry of the inbox's back stack, as `ThreePaneNavigator` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InboxEntry {
    List,
    Detail(u64),
}

impl ReplyHome {
    /// The inbox with its first mail open, as Reply starts on a large screen.
    pub fn new() -> ReplyHome {
        let emails = all_emails();
        let opened_email = emails.first().map(|email| email.id);
        ReplyHome {
            emails,
            selected_emails: BTreeSet::new(),
            opened_email,
            is_detail_only_open: false,
        }
    }

    /// The entity the screens share.
    pub fn entity(app: &mut App) -> Entity<ReplyHome> {
        app.new_entity(|_cx| ReplyHome::new())
    }

    pub fn email(&self, id: u64) -> Option<&Email> {
        self.emails.iter().find(|email| email.id == id)
    }

    /// The mail the detail shows: the open one, or the first.
    pub fn opened_email(&self) -> &Email {
        self.opened_email
            .and_then(|id| self.email(id))
            .unwrap_or(&self.emails[0])
    }

    /// The list, then the open mail when it has a page of its own.
    pub fn back_stack(&self) -> Vec<InboxEntry> {
        let mut stack = vec![InboxEntry::List];
        if self.is_detail_only_open
            && let Some(id) = self.opened_email
        {
            stack.push(InboxEntry::Detail(id));
        }
        stack
    }

    /// Opens a mail on a page of its own: `setOpenedEmail(id, SINGLE_PANE)`, which is what
    /// every tap in the sample's list asks for.
    fn set_opened_email(&mut self, id: u64) {
        self.opened_email = Some(id);
        self.is_detail_only_open = true;
    }

    fn toggle_selected_email(&mut self, id: u64) {
        if !self.selected_emails.remove(&id) {
            self.selected_emails.insert(id);
        }
    }

    /// Back from the open mail: the detail shows the first mail again.
    fn close_detail_screen(&mut self) {
        self.is_detail_only_open = false;
        self.opened_email = self.emails.first().map(|email| email.id);
    }
}

impl Default for ReplyHome {
    fn default() -> ReplyHome {
        ReplyHome::new()
    }
}

/// Opens mail `id`.
pub fn open_email(app: &mut App, home: &Entity<ReplyHome>, id: u64) {
    app.update(home, |home, cx| {
        home.set_opened_email(id);
        cx.notify();
    });
}

/// Picks mail `id`, or puts it back.
pub fn toggle_selected_email(app: &mut App, home: &Entity<ReplyHome>, id: u64) {
    app.update(home, |home, cx| {
        home.toggle_selected_email(id);
        cx.notify();
    });
}

/// Closes the open mail's page.
pub fn close_detail_screen(app: &mut App, home: &Entity<ReplyHome>) {
    app.update(home, |home, cx| {
        home.close_detail_screen();
        cx.notify();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_a_mail_gives_it_a_page_and_closing_returns_to_the_first() {
        let mut home = ReplyHome::new();
        assert_eq!(home.back_stack(), vec![InboxEntry::List]);
        assert_eq!(home.opened_email().id, 0);

        home.set_opened_email(3);
        assert_eq!(
            home.back_stack(),
            vec![InboxEntry::List, InboxEntry::Detail(3)]
        );
        assert_eq!(home.opened_email().subject, "High school reunion?");

        home.close_detail_screen();
        assert_eq!(home.back_stack(), vec![InboxEntry::List]);
        assert_eq!(home.opened_email().id, 0);
    }

    #[test]
    fn a_second_toggle_puts_a_mail_back() {
        let mut home = ReplyHome::new();
        home.toggle_selected_email(2);
        assert!(home.selected_emails.contains(&2));
        home.toggle_selected_email(2);
        assert!(home.selected_emails.is_empty());
    }
}
