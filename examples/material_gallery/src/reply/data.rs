//! Reply's mail and contacts: the sample's `data` package and its local data providers, as plain
//! Rust with no `App`.
//!
//! The accounts keep their names and avatars only; the sample's addresses are left out, since no
//! screen shows them. The sample shuffles each mail's thread at start; here every mail carries the
//! same thread in one order, so the screens are the same on every run.

/// A person mail comes from: Reply's `Account`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Account {
    pub id: u64,
    pub first_name: &'static str,
    pub last_name: &'static str,
    /// The encoded picture shown in the profile image.
    pub avatar: &'static [u8],
}

impl Account {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

/// Which mailbox a mail sits in: Reply's `MailboxType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailboxType {
    Inbox,
    Drafts,
    Sent,
    Spam,
    Trash,
}

/// One mail and the thread under it: Reply's `Email`, without recipients and attachments, which
/// no screen shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Email {
    pub id: u64,
    pub sender: Account,
    pub subject: &'static str,
    pub body: &'static str,
    pub is_important: bool,
    pub is_starred: bool,
    pub mailbox: MailboxType,
    pub created_at: &'static str,
    pub threads: Vec<Email>,
}

impl Email {
    fn new(
        id: u64,
        sender: Account,
        subject: &'static str,
        body: &'static str,
        created_at: &'static str,
    ) -> Email {
        Email {
            id,
            sender,
            subject,
            body,
            is_important: false,
            is_starred: false,
            mailbox: MailboxType::Inbox,
            created_at,
            threads: Vec::new(),
        }
    }

    fn mailbox(mut self, mailbox: MailboxType) -> Email {
        self.mailbox = mailbox;
        self
    }

    fn important(mut self) -> Email {
        self.is_important = true;
        self
    }

    fn starred(mut self) -> Email {
        self.is_starred = true;
        self
    }

    fn threads(mut self, threads: Vec<Email>) -> Email {
        self.threads = threads;
        self
    }
}

const AVATAR_2: &[u8] = include_bytes!("../../assets/reply/avatar_2.jpg");
const AVATAR_3: &[u8] = include_bytes!("../../assets/reply/avatar_3.jpg");
const AVATAR_4: &[u8] = include_bytes!("../../assets/reply/avatar_4.jpg");
const AVATAR_5: &[u8] = include_bytes!("../../assets/reply/avatar_5.jpg");
const AVATAR_6: &[u8] = include_bytes!("../../assets/reply/avatar_6.jpg");
const AVATAR_7: &[u8] = include_bytes!("../../assets/reply/avatar_7.jpg");
const AVATAR_8: &[u8] = include_bytes!("../../assets/reply/avatar_8.jpg");
const AVATAR_EXPRESS: &[u8] = include_bytes!("../../assets/reply/avatar_express.png");

/// The picture the search bar shows for the signed-in user.
pub const PROFILE_AVATAR: &[u8] = AVATAR_6;

/// The contacts mail comes from: `LocalAccountsDataProvider`'s contact accounts that send mail.
fn contact(id: u64) -> Account {
    let (first_name, last_name, avatar) = match id {
        5 => ("Allison", "Trabucco", AVATAR_3),
        6 => ("Ali", "Connors", AVATAR_5),
        8 => ("Kim", "Alen", AVATAR_7),
        9 => ("Google", "Express", AVATAR_EXPRESS),
        10 => ("Sandra", "Adams", AVATAR_2),
        11 => ("Trevor", "Hansen", AVATAR_8),
        13 => ("Frank", "Hawkins", AVATAR_4),
        _ => panic!("no contact {id}"),
    };
    Account {
        id,
        first_name,
        last_name,
        avatar,
    }
}

const PLAY_STORE_BODY: &str = "Your update, 0.1.1, is now live on the Play Store and available for your alpha users to start testing.\n\nYour alpha testers will be automatically notified. If you'd rather send them a link directly, go to your Google Play Console and follow the instructions for obtaining an open alpha testing link.";

const BRUNCH_BODY: &str = "I'll be in your neighborhood doing errands and was hoping to catch you for a coffee this Saturday. If you don't have anything scheduled, it would be great to see you! It feels like its been forever.\n\nIf we do get a chance to get together, remind me to tell you about Kim. She stopped over at the house to say hey to the kids and told me all about her trip to Mexico.\n\nTalk to you soon,\n\nAli";

const RECIPE_BODY: &str = "Raspberry Pie: We should make this pie recipe tonight! The filling is very quick to put together.";

const NO_SUBJECT_BODY: &str = "Hey,\n\nWanted to email and see what you thought of";

/// The thread every mail opens onto: `LocalEmailsDataProvider.threads`.
fn threads() -> Vec<Email> {
    vec![
        Email::new(
            8,
            contact(13),
            "Your update on Google Play Store is live!",
            PLAY_STORE_BODY,
            "3 hours ago",
        )
        .mailbox(MailboxType::Trash),
        Email::new(
            5,
            contact(13),
            "Update to Your Itinerary",
            "",
            "2 hours ago",
        ),
        Email::new(6, contact(10), "Recipe to try", RECIPE_BODY, "2 hours ago")
            .mailbox(MailboxType::Sent),
        Email::new(
            7,
            contact(9),
            "Delivered",
            "Your shoes should be waiting for you at home!",
            "2 hours ago",
        ),
        Email::new(
            9,
            contact(10),
            "(No subject)",
            NO_SUBJECT_BODY,
            "3 hours ago",
        )
        .mailbox(MailboxType::Drafts),
        Email::new(
            1,
            contact(6),
            "Brunch this weekend?",
            BRUNCH_BODY,
            "40 mins ago",
        ),
        Email::new(
            2,
            contact(5),
            "Bonjour from Paris",
            "Here are some great shots from my trip...",
            "1 hour ago",
        )
        .important(),
    ]
}

/// The inbox, newest first: `LocalEmailsDataProvider.allEmails`.
pub fn all_emails() -> Vec<Email> {
    let mails = vec![
        Email::new(
            0,
            contact(9),
            "Package shipped!",
            "Cucumber Mask Facial has shipped.\n\nKeep an eye out for a package to arrive between this Thursday and next Tuesday. If for any reason you don't receive your package before the end of next week, please reach out to us for details on your shipment.\n\nAs always, thank you for shopping with us and we hope you love our specially formulated Cucumber Mask!",
            "20 mins ago",
        )
        .starred(),
        Email::new(1, contact(6), "Brunch this weekend?", BRUNCH_BODY, "40 mins ago"),
        Email::new(
            2,
            contact(5),
            "Bonjour from Paris",
            "Here are some great shots from my trip...",
            "1 hour ago",
        )
        .important(),
        Email::new(
            3,
            contact(8),
            "High school reunion?",
            "Hi friends,\n\nI was at the grocery store on Sunday night.. when I ran into Genie Williams! I almost didn't recognize her afer 20 years!\n\nAnyway, it turns out she is on the organizing committee for the high school reunion this fall. I don't know if you were planning on going or not, but she could definitely use our help in trying to track down lots of missing alums. If you can make it, we're doing a little phone-tree party at her place next Saturday, hoping that if we can find one person, thee more will...",
            "2 hours ago",
        )
        .mailbox(MailboxType::Sent),
        Email::new(
            4,
            contact(11),
            "Brazil trip",
            "Thought we might be able to go over some details about our upcoming vacation.\n\nI've been doing a bit of research and have come across a few paces in Northern Brazil that I think we should check out. One, the north has some of the most predictable wind on the planet. I'd love to get out on the ocean and kitesurf for a couple of days if we're going to be anywhere near or around Taiba. I hear it's beautiful there and if you're up for it, I'd love to go. Other than that, I haven't spent too much time looking into places along our road trip route. I'm assuming we can find places to stay and things to do as we drive and find places we think look interesting. But... I know you're more of a planner, so if you have ideas or places in mind, lets jot some ideas down!\n\nMaybe we can jump on the phone later today if you have a second.",
            "2 hours ago",
        )
        .starred(),
        Email::new(5, contact(13), "Update to Your Itinerary", "", "2 hours ago"),
        Email::new(6, contact(10), "Recipe to try", RECIPE_BODY, "2 hours ago")
            .mailbox(MailboxType::Sent),
        Email::new(
            7,
            contact(9),
            "Delivered",
            "Your shoes should be waiting for you at home!",
            "2 hours ago",
        ),
        Email::new(
            8,
            contact(13),
            "Your update on Google Play Store is live!",
            PLAY_STORE_BODY,
            "3 hours ago",
        )
        .mailbox(MailboxType::Trash),
        Email::new(9, contact(10), "(No subject)", NO_SUBJECT_BODY, "3 hours ago")
            .mailbox(MailboxType::Drafts),
        Email::new(
            10,
            contact(5),
            "Try a free TrailGo account",
            "Looking for the best hiking trails in your area? TrailGo gets you on the path to the outdoors faster than you can pack a sandwich.\n\nWhether you're an experienced hiker or just looking to get outside for the afternoon, there's a segment that suits you.",
            "3 hours ago",
        )
        .mailbox(MailboxType::Trash),
        Email::new(
            11,
            contact(5),
            "Free money",
            "You've been selected as a winner in our latest raffle! To claim your prize, click on the link.",
            "3 hours ago",
        )
        .mailbox(MailboxType::Spam),
    ];
    mails
        .into_iter()
        .map(|mail| mail.threads(threads()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inbox_opens_on_the_shipped_package_and_every_mail_has_its_thread() {
        let emails = all_emails();
        assert_eq!(emails.len(), 12);
        assert_eq!(emails[0].subject, "Package shipped!");
        assert_eq!(emails[0].sender.first_name, "Google");
        assert!(emails.iter().all(|email| email.threads.len() == 7));
    }
}
