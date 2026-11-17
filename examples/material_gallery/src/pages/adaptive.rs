//! Adaptive layout: Reply, Google's Material 3 mail sample, laid out with the navigation suite
//! and the pane scaffold. Resize the window: the bar becomes a rail and then a drawer, and the
//! inbox shows the open mail beside the list once there is room.

use waft_foundation::App;
use waft_widgets::{BuildContext, IntoWidget, WidgetRef};

use crate::reply::ReplyApp;

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    ReplyApp.into_widget()
}
