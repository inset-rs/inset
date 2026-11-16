//! flutter_test's `Finder` and `find` (`finders.dart`): a description of which elements to find,
//! evaluated against the tree when a case asks.
//!
//! A [`Finder`] walks the element tree depth first, a parent before its children, from the root's
//! children, and by default skips what is offstage: it walks with
//! [`AnyElement::debug_visit_onstage_children`](crate::AnyElement::debug_visit_onstage_children),
//! as Dart's `collectAllElementsFrom(skipOffstage: true)` does. [`Finder::skip_offstage`] walks
//! every child instead.

use std::fmt;
use std::rc::Rc;

use waft_foundation::App;

use crate::{AnyElement, GlobalKey, IconData, KeyRef, Widget, WidgetRef, WidgetsBinding};

use super::WidgetTester;

/// Which elements a [`Finder`] matches among its candidates.
type ElementMatch = Rc<dyn Fn(&App, AnyElement) -> bool>;

/// Dart's `Finder`: the elements of the tree that match, found each time it is evaluated.
#[derive(Clone)]
pub struct Finder {
    description: Rc<str>,
    candidates: Candidates,
    filter: Filter,
    skip_offstage: bool,
    pick: Option<Pick>,
}

/// Where a finder looks.
#[derive(Clone)]
enum Candidates {
    /// Every element under the root.
    Tree,
    /// The elements under each element `of` finds, and those elements themselves when
    /// `match_root` (Dart's `_DescendantWidgetFinder`).
    DescendantsOf { of: Box<Finder>, match_root: bool },
    /// The ancestors of each element `of` finds, nearest first, and the element itself first
    /// when `match_root` (Dart's `_AncestorWidgetFinder`).
    AncestorsOf { of: Box<Finder>, match_root: bool },
}

/// Which candidates a finder keeps.
#[derive(Clone)]
enum Filter {
    Matches(ElementMatch),
    /// The candidates another finder finds too.
    FoundBy(Box<Finder>),
}

/// Dart's `.first`, `.last` and `.at(index)`.
#[derive(Clone, Copy, Debug)]
enum Pick {
    First,
    Last,
    At(usize),
}

impl Finder {
    fn matching(
        description: String,
        matches: impl Fn(&App, AnyElement) -> bool + 'static,
    ) -> Finder {
        Finder {
            description: description.into(),
            candidates: Candidates::Tree,
            filter: Filter::Matches(Rc::new(matches)),
            skip_offstage: true,
            pick: None,
        }
    }

    /// Dart's `skipOffstage:` argument: `false` finds offstage elements too.
    pub fn skip_offstage(mut self, skip_offstage: bool) -> Finder {
        self.skip_offstage = skip_offstage;
        self
    }

    /// Dart's `finder.first`: the first element found, if any.
    pub fn first(self) -> Finder {
        self.picking(Pick::First, "first")
    }

    /// Dart's `finder.last`: the last element found, if any.
    pub fn last(self) -> Finder {
        self.picking(Pick::Last, "last")
    }

    /// Dart's `finder.at(index)`: the element at `index` of those found, if there is one.
    pub fn at(self, index: usize) -> Finder {
        self.picking(Pick::At(index), &format!("index {index} of"))
    }

    fn picking(self, pick: Pick, name: &str) -> Finder {
        let description = format!("{name} {}", self.description);
        Finder {
            description: description.into(),
            candidates: Candidates::Tree,
            filter: Filter::FoundBy(Box::new(self)),
            skip_offstage: false,
            pick: Some(pick),
        }
    }

    /// What the finder looks for, as Dart's `describeMatch` words it for many matches.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Dart's `finder.evaluate()`: the elements found now, in tree order.
    pub fn evaluate(&self, tester: &WidgetTester) -> Vec<AnyElement> {
        self.evaluate_in(&mut tester.cell.borrow_mut())
    }

    /// [`evaluate`](Self::evaluate) with the app at hand.
    pub fn evaluate_in(&self, app: &mut App) -> Vec<AnyElement> {
        if let (Some(pick), Filter::FoundBy(of)) = (self.pick, &self.filter) {
            let found = of.evaluate_in(app);
            let picked = match pick {
                Pick::First => found.first(),
                Pick::Last => found.last(),
                Pick::At(index) => found.get(index),
            };
            return picked.copied().into_iter().collect();
        }
        let candidates = self.candidates_in(app);
        match &self.filter {
            Filter::Matches(matches) => candidates
                .into_iter()
                .filter(|element| matches(app, *element))
                .collect(),
            Filter::FoundBy(finder) => {
                let found = finder.evaluate_in(app);
                candidates
                    .into_iter()
                    .filter(|element| found.contains(element))
                    .collect()
            }
        }
    }

    fn candidates_in(&self, app: &mut App) -> Vec<AnyElement> {
        match &self.candidates {
            Candidates::Tree => {
                let root = WidgetsBinding::instance(app).root_element(app);
                root.map_or_else(Vec::new, |root| {
                    descendants_of(app, root, self.skip_offstage)
                })
            }
            Candidates::DescendantsOf { of, match_root } => {
                let roots = of.evaluate_in(app);
                let mut candidates: Vec<AnyElement> = Vec::new();
                for root in &roots {
                    for element in descendants_of(app, *root, self.skip_offstage) {
                        if !candidates.contains(&element) {
                            candidates.push(element);
                        }
                    }
                }
                if *match_root {
                    candidates.splice(0..0, roots);
                }
                candidates
            }
            Candidates::AncestorsOf { of, match_root } => {
                let mut candidates = Vec::new();
                for leaf in of.evaluate_in(app) {
                    if *match_root {
                        candidates.push(leaf);
                    }
                    candidates.extend(std::iter::successors(leaf.parent(app), |parent| {
                        parent.parent(app)
                    }));
                }
                candidates
            }
        }
    }
}

impl fmt::Debug for Finder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Finder({})", self.description)
    }
}

/// Every element under `root`, depth first with a parent before its children and siblings in
/// order, without `root` itself: Dart's `collectAllElementsFrom`.
fn descendants_of(app: &App, root: AnyElement, skip_offstage: bool) -> Vec<AnyElement> {
    let children_of = |element: AnyElement| {
        let mut children = Vec::new();
        if skip_offstage {
            element.debug_visit_onstage_children(app, &mut |child| children.push(child));
        } else {
            element.visit_children(app, &mut |child| children.push(child));
        }
        children
    };
    let mut found = Vec::new();
    let mut pending: Vec<AnyElement> = children_of(root).into_iter().rev().collect();
    while let Some(element) = pending.pop() {
        found.push(element);
        pending.extend(children_of(element).into_iter().rev());
    }
    found
}

/// The plain text of a `Text`, its data or its span's.
fn text_of(text: &crate::Text) -> Option<String> {
    match (&text.data, &text.text_span) {
        (Some(data), _) => Some(data.clone()),
        (None, Some(span)) => Some(span.to_plain_text(true, true)),
        (None, None) => None,
    }
}

/// Dart's `_MatchTextFinder.matches`: an `EditableText` by its controller's text, and a `Text`,
/// or with `find_rich_text` a `RichText` (which every `Text` builds), by its plain text.
fn matches_text(
    app: &App,
    element: AnyElement,
    find_rich_text: bool,
    matches: &dyn Fn(&str) -> bool,
) -> bool {
    let widget = element.widget(app);
    if let Some(editable) = crate::downcast_widget::<crate::EditableText>(&**widget) {
        return matches(&editable.controller.value(app).text);
    }
    if find_rich_text {
        return crate::downcast_widget::<crate::RichText>(&**widget)
            .is_some_and(|rich_text| matches(&rich_text.text.to_plain_text(true, true)));
    }
    crate::downcast_widget::<crate::Text>(&**widget)
        .and_then(text_of)
        .is_some_and(|text| matches(&text))
}

/// flutter_test's `find`: the finders a case starts from.
pub mod find {
    use super::*;

    /// Dart's `find.text(text)`: the `Text` widgets, and the `EditableText`s, showing `text`.
    pub fn text(text: &str) -> Finder {
        let expected = text.to_string();
        Finder::matching(format!("text \"{text}\""), move |app, element| {
            matches_text(app, element, false, &|found| found == expected)
        })
    }

    /// Dart's `find.text(text, findRichText: true)`: the `RichText` widgets, and so every
    /// `Text`, and the `EditableText`s, showing `text`.
    pub fn rich_text(text: &str) -> Finder {
        let expected = text.to_string();
        Finder::matching(format!("rich text \"{text}\""), move |app, element| {
            matches_text(app, element, true, &|found| found == expected)
        })
    }

    /// Dart's `find.textContaining(pattern)` for a string pattern: the `Text` widgets, and the
    /// `EditableText`s, whose text contains `pattern`.
    pub fn text_containing(pattern: &str) -> Finder {
        let pattern = pattern.to_string();
        Finder::matching(format!("text containing {pattern}"), move |app, element| {
            matches_text(app, element, false, &|found| {
                found.contains(pattern.as_str())
            })
        })
    }

    /// Dart's `find.byKey(key)`: the widgets with `key`.
    pub fn by_key(key: impl Into<KeyRef>) -> Finder {
        let key: KeyRef = key.into();
        Finder::matching(format!("key {key:?}"), move |app, element| {
            element.widget(app).key() == Some(&key)
        })
    }

    /// Dart's `find.byKey(globalKey)`, taking the key by reference.
    pub fn by_global_key(key: &GlobalKey) -> Finder {
        by_key(key.clone())
    }

    /// Dart's `find.byType(W)`: the widgets whose type is `W`.
    pub fn by_type<W: 'static>() -> Finder {
        Finder::matching(
            format!("type \"{}\"", short_type_name::<W>()),
            |app, element| crate::downcast_widget::<W>(&**element.widget(app)).is_some(),
        )
    }

    /// Dart's `find.byIcon(icon)`: the `Icon` widgets showing `icon`.
    pub fn by_icon(icon: &IconData) -> Finder {
        let icon = icon.clone();
        Finder::matching(format!("icon \"{icon:?}\""), move |app, element| {
            crate::downcast_widget::<crate::Icon>(&**element.widget(app))
                .is_some_and(|widget| widget.icon.as_ref() == Some(&icon))
        })
    }

    /// Dart's `find.byWidget(widget)`: the elements configured by that very widget.
    pub fn by_widget(widget: &WidgetRef) -> Finder {
        let widget = Rc::clone(widget);
        Finder::matching(
            format!("the given widget ({widget:?})"),
            move |app, element| Rc::ptr_eq(element.widget(app), &widget),
        )
    }

    /// Dart's `find.byWidgetPredicate(predicate, description:)`.
    pub fn by_widget_predicate(
        description: &str,
        predicate: impl Fn(&App, &dyn Widget) -> bool + 'static,
    ) -> Finder {
        Finder::matching(description.to_string(), move |app, element| {
            predicate(app, &**element.widget(app))
        })
    }

    /// Dart's `find.byElementPredicate(predicate, description:)`.
    pub fn by_element_predicate(
        description: &str,
        predicate: impl Fn(&App, AnyElement) -> bool + 'static,
    ) -> Finder {
        Finder::matching(description.to_string(), predicate)
    }

    /// Dart's `find.byTooltip(message)` for a string message: the `RawTooltip`s whose semantics
    /// tooltip is `message`. (Material's `Tooltip` gives its `RawTooltip` its message.)
    pub fn by_tooltip(message: &str) -> Finder {
        let message = message.to_string();
        Finder::matching(format!("tooltip \"{message}\""), move |app, element| {
            crate::downcast_widget::<crate::RawTooltip>(&**element.widget(app))
                .is_some_and(|tooltip| tooltip.semantics_tooltip.as_deref() == Some(&*message))
        })
    }

    /// Dart's `find.widgetWithText(W, text)`: the `W` widgets above a `Text` showing `text`.
    pub fn widget_with_text<W: 'static>(text: &str) -> Finder {
        ancestor(self::text(text), by_type::<W>())
    }

    /// Dart's `find.widgetWithIcon(W, icon)`: the `W` widgets above an `Icon` showing `icon`.
    pub fn widget_with_icon<W: 'static>(icon: &IconData) -> Finder {
        ancestor(by_icon(icon), by_type::<W>())
    }

    /// Dart's `find.descendant(of: of, matching: matching)`: what `matching` finds under what
    /// `of` finds.
    pub fn descendant(of: Finder, matching: Finder) -> Finder {
        descendant_with_root(of, matching, false)
    }

    /// Dart's `find.descendant(of:, matching:, matchRoot: true)`: what `of` finds counts too.
    pub fn descendant_or_self(of: Finder, matching: Finder) -> Finder {
        descendant_with_root(of, matching, true)
    }

    fn descendant_with_root(of: Finder, matching: Finder, match_root: bool) -> Finder {
        let description = format!(
            "{} descending from {}{}",
            matching.description,
            of.description,
            if match_root { " inclusive" } else { "" }
        );
        Finder {
            description: description.into(),
            candidates: Candidates::DescendantsOf {
                of: Box::new(of),
                match_root,
            },
            filter: Filter::FoundBy(Box::new(matching)),
            skip_offstage: true,
            pick: None,
        }
    }

    /// Dart's `find.ancestor(of: of, matching: matching)`: what `matching` finds above what
    /// `of` finds, nearest first, offstage or not.
    pub fn ancestor(of: Finder, matching: Finder) -> Finder {
        ancestor_with_root(of, matching, false)
    }

    /// Dart's `find.ancestor(of:, matching:, matchRoot: true)`: what `of` finds counts too.
    pub fn ancestor_or_self(of: Finder, matching: Finder) -> Finder {
        ancestor_with_root(of, matching, true)
    }

    fn ancestor_with_root(of: Finder, matching: Finder, match_root: bool) -> Finder {
        let description = format!(
            "{} which is an ancestor of {}{}",
            matching.description,
            of.description,
            if match_root { " inclusive" } else { "" }
        );
        Finder {
            description: description.into(),
            candidates: Candidates::AncestorsOf {
                of: Box::new(of),
                match_root,
            },
            filter: Filter::FoundBy(Box::new(matching)),
            skip_offstage: false,
            pick: None,
        }
    }

    /// The type's name without its paths, as Dart's `runtimeType` prints it:
    /// `SegmentedButton<i32>` for `waft_material::segmented_button::SegmentedButton<i32>`.
    fn short_type_name<W>() -> String {
        let mut short = String::new();
        let mut segment = String::new();
        for character in std::any::type_name::<W>().chars() {
            if character.is_alphanumeric() || character == '_' || character == ':' {
                segment.push(character);
            } else {
                short.push_str(segment.rsplit("::").next().unwrap_or(&segment));
                segment.clear();
                short.push(character);
            }
        }
        short.push_str(segment.rsplit("::").next().unwrap_or(&segment));
        short
    }
}

/// The one element a tester method acts on: a [`Finder`] that finds exactly one, as Dart's
/// tester methods require of theirs, an element, or a global key's widget.
pub trait FindsOne {
    /// The element, or a panic naming `callee` when a finder finds none or several.
    fn find_one(&self, tester: &WidgetTester, callee: &str) -> AnyElement;
}

impl FindsOne for Finder {
    fn find_one(&self, tester: &WidgetTester, callee: &str) -> AnyElement {
        let found = self.evaluate(tester);
        match found.as_slice() {
            [element] => *element,
            [] => panic!(
                "The finder \"{}\" (used in a call to \"{callee}()\") could not find any \
                 matching widgets.",
                self.description
            ),
            _ => panic!(
                "The finder \"{}\" (used in a call to \"{callee}()\") ambiguously found \
                 multiple matching widgets. The \"{callee}()\" method needs a single target.",
                self.description
            ),
        }
    }
}

impl FindsOne for &Finder {
    fn find_one(&self, tester: &WidgetTester, callee: &str) -> AnyElement {
        (*self).find_one(tester, callee)
    }
}

impl FindsOne for AnyElement {
    fn find_one(&self, _tester: &WidgetTester, _callee: &str) -> AnyElement {
        *self
    }
}

impl FindsOne for &GlobalKey {
    fn find_one(&self, tester: &WidgetTester, callee: &str) -> AnyElement {
        self.current_context(&tester.cell.borrow())
            .unwrap_or_else(|| panic!("the key used in a call to \"{callee}()\" is not mounted"))
    }
}

/// The render box a tester's geometry method measures: the render box of what a [`FindsOne`]
/// finds, or a render box itself.
pub trait RenderTarget {
    /// The render box, or a panic naming `callee` when there is not exactly one.
    fn render_box(&self, tester: &WidgetTester, callee: &str) -> waft_rendering::AnyRenderBox;
}

impl<T: FindsOne> RenderTarget for T {
    fn render_box(&self, tester: &WidgetTester, callee: &str) -> waft_rendering::AnyRenderBox {
        let element = self.find_one(tester, callee);
        let app = tester.cell.borrow();
        element
            .find_render_object(&app)
            .and_then(|render| render.as_box())
            .unwrap_or_else(|| panic!("the widget found for \"{callee}()\" has no render box"))
    }
}

impl RenderTarget for waft_rendering::AnyRenderBox {
    fn render_box(&self, _tester: &WidgetTester, _callee: &str) -> waft_rendering::AnyRenderBox {
        *self
    }
}
