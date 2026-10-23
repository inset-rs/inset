# Many optional fields

Dart named optional constructor arguments (and `copyWith`) are a fluent chain on the value. `new()` is the all-defaults object. Each setter is named after the Dart field, takes `self` and the inner type (not `Option`), returns `Self`. Omitted stays unset. `copy_with(&self)` clones, then the same chain replaces fields.

Do not add a separate builder type when the chain on the value does the job; a value whose constructor fills fields from other arguments takes one ([derived-fields.md](derived-fields.md)). Do not pass a long `Option` argument list. One or two fields stay a plain `new(a, b)`.

Constructor asserts run on the setter that can violate them (or on a private assemble that lerp / scale use when they must write `Option`s).

```
BoxDecoration::new()
    .color(c)
    .border_radius(r)
    .shape(BoxShape::Circle);

decoration.copy_with().color(c);
```

A field the Dart constructor fills from other arguments is a [derived field](derived-fields.md).

See `box_decoration.rs`.
