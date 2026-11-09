//! waft-embedder's integration tests, one module per subject, in one binary so that the crate and
//! its dependencies are linked once rather than once per file. A new subject is a module here, not
//! a file beside `all/`: Cargo builds each of those as a binary of its own. A subject that tests
//! what a feature adds is built only with that feature.

#[cfg(feature = "serde")]
mod view_values_come_back_from_json_as_they_went;
