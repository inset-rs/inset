# Derived fields

A Dart constructor can fill a field from other arguments: `showCursor = showCursor ?? !readOnly`, or `inputFormatters = maxLines == 1 ? [singleLineFormatter, ...?inputFormatters] : inputFormatters`. Dart sees every argument at once. A fluent chain ([many-optional-fields.md](many-optional-fields.md)) sees them one at a time, so a setter that fills another field from what it has seen so far ends up wrong:

- it overwrites what the caller set earlier: `.keyboard_type(NUMBER).max_lines(None)` lost `NUMBER`;
- or it keeps a guess that a later setter has to undo: `new()` added the single-line formatter for its default `max_lines`, and `.max_lines(None)` could not take it out again.

So the field holds only what the caller gave, and an `effective_*` method works out Dart's expression when it is read:

```rust
pub show_cursor: Option<bool>, // None: not given

/// Dart `EditableText(showCursor:)`.
pub fn show_cursor(mut self, show_cursor: bool) -> Self {
    self.show_cursor = Some(show_cursor);
    self
}

/// Dart's `showCursor` field, as its constructor fills it.
pub fn effective_show_cursor(&self) -> bool {
    self.show_cursor.unwrap_or(!self.read_only)
}
```

- A setter writes its own field only. Constructor asserts still run on the setter that can break them.
- Every reader calls `effective_*`, never the field, including the old-widget comparisons in `did_update_widget`.
- A rule that reads `defaultTargetPlatform` takes `&App` and reads `app.platform().target_platform()`.
- A plain default (`this.maxLines = 1`) is not derived: it stays a value in `new()`.
- `copy_with` keeps the worked-out values, as Dart's `copyWith` passes `this.field` on.

That is for a widget, which the caller hands over as it is. A value that is finished before it is used takes a builder instead: its setters collect what was given, `build()` fills the rest once, and the value keeps plain fields, so its readers need no `effective_*`. `copy_with()` returns a builder holding the value's fields.

```rust
SliverGeometry::builder()
    .paint_extent(p)
    .layout_extent(l)
    .build() // cache extent: l, as in Dart
```

See `editable_text.rs`, and `sliver.rs` and `text_input.rs` (`TextInputConfiguration`) for values.
