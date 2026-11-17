//! Material adaptive layout, ported from Jetpack Compose's material3-adaptive.
//!
//! Compose's `material3-adaptive` answers what Flutter's Material has no widget for: a phone
//! layout on a phone, and a list beside its detail on a tablet or desktop. This module keeps its
//! split: the size-class and pane decisions are plain data and functions, and the widgets are
//! [`ThreePaneScaffold`] and [`NavigationSuiteScaffold`].
//!
//! Where a type or rule has no counterpart in Flutter, `PORTING.md` beside this module records
//! it with the reason kind `framework`.

mod adapt_strategy;
mod back_navigation_behavior;
mod navigation_suite_scaffold;
mod pane;
mod pane_adapted_value;
mod pane_motion;
mod pane_scaffold_directive;
mod pane_scaffold_role;
mod three_pane_navigator;
mod three_pane_scaffold;
mod three_pane_scaffold_horizontal_order;
mod three_pane_scaffold_value;
mod three_pane_scene_strategy;
mod window_size_class;

pub use adapt_strategy::*;
pub use back_navigation_behavior::*;
pub use navigation_suite_scaffold::*;
pub use pane::*;
pub use pane_adapted_value::*;
pub use pane_scaffold_directive::*;
pub use pane_scaffold_role::*;
pub use three_pane_navigator::*;
pub use three_pane_scaffold::*;
pub use three_pane_scaffold_horizontal_order::*;
pub use three_pane_scaffold_value::*;
pub use three_pane_scene_strategy::*;
pub use window_size_class::*;
