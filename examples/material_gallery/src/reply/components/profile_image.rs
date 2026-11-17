//! `ReplyProfileImage.kt`: a contact's picture in a circle.

use waft_painting::BoxFit;
use waft_widgets::{ClipOval, Image, IntoWidget, SizedBox, WidgetRef};

/// The size the sample's profile images take unless it says otherwise.
pub const PROFILE_IMAGE_SIZE: f64 = 40.0;

/// `avatar`, an encoded picture, clipped to a circle `size` across.
pub fn profile_image(avatar: &'static [u8], size: f64) -> WidgetRef {
    SizedBox::new()
        .width(size)
        .height(size)
        .child(ClipOval::new().child(Image::memory_static(avatar).fit(BoxFit::Cover)))
        .into_widget()
}
