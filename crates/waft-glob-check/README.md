# waft-glob-check

A compile-time check that an app can write `use waft::*;` beside a kit's `use waft_winui::*;`
(or Material's, or Cupertino's) without a name becoming ambiguous.

An app globs the facade and a kit together. Rust does not merge two globs silently: a name both
export makes every use of it an error. `src/lib.rs` re-exports each pair of globs under
`#![deny(ambiguous_glob_reexports)]`, so adding a name to the facade and a kit with the same name
fails `cargo test --workspace` and names the item.

Not published, and not depended on by anything: `publish = false` keeps it out of
`cargo publish --workspace`, so the published crates never depend on it or on each other through
it.
