//! The gallery's integration tests, one module per subject, in one binary so that the gallery,
//! the kit and their dependencies are linked once rather than once per file, as wasmtime's
//! `tests/all` and Cargo's `tests/testsuite` are. A new subject is a module here, not a file
//! beside `all/`: Cargo builds each of those as a binary of its own. Run one subject with its
//! module path as the filter: `cargo test -p winui_gallery --test all combo_box::`.
//!
//! The tests run alongside each other in one process, each on its own fixture. The framework's
//! debug switches (`set_debug_paint_size_enabled`, `set_debug_disable_shadows` and the like) are
//! the process's, so a test that turns one on changes what every test beside it draws.
//!
//! `anchored_flyout`, `navigation_scroll_viewport` and `scroll_tab_viewer` are binaries of their
//! own beside this one: each compiles a private source file of `waft-winui` as one of its modules,
//! and that file names the kit's items through `crate::`, which only a crate root that imports
//! them, as theirs does, answers.
#![feature(arbitrary_self_types)]

mod common;

mod acrylic;
mod breadcrumb_bar;
mod button_family;
mod check_box;
mod color_picker;
mod combo_box;
mod content_dialog;
mod control_border;
mod drag_drop;
mod event_loop;
mod feedback;
mod flyout;
mod focus_policy;
mod focus_state;
mod gallery;
mod gallery_catalog;
mod gallery_navigation_options;
mod grid_child_size;
mod grid_layout_builder;
mod icon;
mod info_badge;
mod items_view;
mod menu_bar;
mod menu_flyout;
mod navigation_indicator_scheduling;
mod navigation_indicator_transition;
mod navigation_view;
mod navigation_view_chrome;
mod navigation_view_compact_hierarchy;
mod navigation_view_footer;
mod navigation_view_items;
mod navigation_view_top_layout;
mod number_box;
mod page;
mod primitives;
mod progress_ring;
mod radio_button;
mod repeat_button_input;
mod retained_flyout;
mod scroll_viewport;
mod selection_examples;
mod selector_bar;
mod slider;
mod slider_tooltip;
mod split_button;
mod split_view;
mod tab_view;
mod text_input;
mod theme_scope;
mod toggle_switch;
mod toggle_switch_input;
mod tool_tip;
mod tree_view;
mod window_resize;
