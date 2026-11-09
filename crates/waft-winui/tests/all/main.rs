//! waft-winui's integration tests, one module per subject, in one binary so that the kit and its
//! dependencies are linked once rather than once per file. A new subject is a module here, not a
//! file beside `all/`: Cargo builds each of those as a binary of its own.
#![feature(arbitrary_self_types)]

mod button_input;
mod text_box_height;
