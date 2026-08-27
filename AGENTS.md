# reveal-rs

A Flutter port in Rust. The framework matches Flutter closely enough that upstream changes land without worry. Below it, a platform seam (views, frames, pointers, timers) so the same framework runs in tests and on real hosts.

Flutter's source is the spec: `/Users/mac/code/flutter/packages/flutter/lib/src`. Read the Dart. Do not write Flutter from memory. If the checkout is missing, ask.

Prior art, not specs:

- `/Users/mac/code/reveal-rs-experiment` — ownership spike and handle-newtype experiment. Prior art, not a spec: take the code, not the commentary.
- `/Users/mac/code/shaft-rs-next` — earlier port; its `PORTING.md` files record what divergence cost
- `/Users/mac/code/ShaftUI` — `Backend` / `NativeView` split (vocabulary, not API)
- `/Users/mac/code/zed/crates/gpui` — `Entity` as a user-facing store (later; not used to implement Flutter)

## Commands

```bash
cargo test --workspace
cargo clippy --workspace
cargo run -p <example>
```

Run affected tests during ordinary work. Full workspace tests and clippy for a finished cross-cutting batch.

## Two layers

- **Platform** — host-agnostic. Headless for tests, desktop (and later others) for real windows. The framework never depends on an embedder.
- **Framework** — a verbatim Flutter port. Crate order follows Flutter: geometry / foundation → app → gestures / painting → rendering → widgets. Lower crates do not name higher crates.

Current loose target: cupertino widgets, first `CupertinoButton` that presses and fades, sitting on that stack — not a shortcut past it.

## Ownership

One `App`. Every framework callback gets `&mut App` plus a typed handle to itself.

- **`Handle<T>`** — Copy generational id, point access (`app.get` / `app.get_mut`). Exclusivity lasts one field access, never a whole pass. Flutter objects live here: elements, render objects, `AnimationController`, `ScrollController`, `FocusNode`, and the rest. `ChangeNotifier` is mixed in as a `ChangeNotifierState` field (mixin-as-field).
- **`Entity<T>`** — reserved for app-level stores the *user* writes, gpui-shaped. Not used to implement Flutter. Not built; how it is accessed is undecided.

Do not lease a Handle out of the arena for a pass. shaft-rs-next did that; layout then could not re-enter the node it was laying out.

## Porting

**Before writing or changing any Rust that corresponds to a Flutter file, read `.cursor/skills/porting-flutter/SKILL.md`.**

Copy the Dart, then modify. Do not rewrite from understanding — models are bad at repeating a file they have only read.

Port bottom-up so an upper layer never meets a missing dependency — that is when agents tend to invent ad-hoc solutions. While shaping a lower API, read how Flutter's next layer uses it so the interface matches; do not guess.

When something does not fit: stop and ask. If a divergence is needed, use a proved pattern (`Handle`, a mixin-as-field, a `Drop` guard) — not a one-off workaround. A silent "seems compatible" change is a bug.

## Docs

`PORTING.md` in a source folder records functional divergences from Flutter, for a reader who knows Rust and only surface Flutter. Each entry is Change / Reason / Affect. No visible Affect means Identical — omit the entry. Straight transcriptions go under `## Identical`. Empty file: omit. Format is in the porting skill.

Doc comments: the invariant a later editor will break. Inline comments: only what the next line does that the code cannot say. When copying from the experiment, strip its commentary.

Do not hard-wrap Markdown at 80 columns.

## Review

Parity is against the Dart, never against `PORTING.md`. Names (snake_case aside), comments (except symbol paths), member order, where state lives, who decides, when hooks run, which members are override points.

A Flutter-shaped name on a different mechanism is a bug.

## Workflow

- Do not commit unless asked.
- Do not add a rule to this file or to the porting skill without asking.
- Ask rather than invent when a special case appears. The rules may be wrong too.
