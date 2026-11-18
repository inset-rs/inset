//! waft-winui's integration tests, one module per subject, in one binary so that the kit and its
//! dependencies are linked once rather than once per file. A new subject is a module here, not a
//! file beside `all/`: Cargo builds each of those as a binary of its own.
#![feature(arbitrary_self_types)]

mod button_input;
mod fluent_app;
mod navigation_view_pane_title;
mod selector_bar_icon;
mod table_view_selection;
mod text_box_height;
mod text_box_no_wrap;
