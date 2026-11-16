//! flutter_test's finder matchers (`matchers.dart`): `findsNothing`, `findsOneWidget`,
//! `findsWidgets`, `findsNWidgets` and `findsAtLeastNWidgets`, which [`expect`] checks a
//! [`Finder`] against, failing with Dart's description of the mismatch.

use super::{Finder, WidgetTester};

/// How many elements a [`Finder`] is expected to find: Dart's `_FindsCountMatcher`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FinderMatcher {
    min: Option<usize>,
    max: Option<usize>,
}

/// Dart's `findsNothing`.
pub const fn finds_nothing() -> FinderMatcher {
    FinderMatcher {
        min: None,
        max: Some(0),
    }
}

/// Dart's `findsOneWidget`.
pub const fn finds_one_widget() -> FinderMatcher {
    FinderMatcher {
        min: Some(1),
        max: Some(1),
    }
}

/// Dart's `findsWidgets`: at least one.
pub const fn finds_widgets() -> FinderMatcher {
    FinderMatcher {
        min: Some(1),
        max: None,
    }
}

/// Dart's `findsNWidgets(n)`.
pub const fn finds_n_widgets(n: usize) -> FinderMatcher {
    FinderMatcher {
        min: Some(n),
        max: Some(n),
    }
}

/// Dart's `findsAtLeastNWidgets(n)`.
pub const fn finds_at_least_n_widgets(n: usize) -> FinderMatcher {
    FinderMatcher {
        min: Some(n),
        max: None,
    }
}

impl FinderMatcher {
    /// Whether `count` found elements match.
    pub fn matches(&self, count: usize) -> bool {
        self.min.is_none_or(|min| count >= min) && self.max.is_none_or(|max| count <= max)
    }

    /// Dart's `describe`: what was expected.
    fn describe(&self) -> String {
        match (self.min, self.max) {
            (None, Some(0)) => "no matching candidates".to_string(),
            (Some(1), Some(1)) => "exactly one matching candidate".to_string(),
            (Some(1), None) => "at least one matching candidate".to_string(),
            (Some(min), None) => format!("at least {min} matching candidates"),
            (Some(n), Some(m)) if n == m => format!("exactly {n} matching candidates"),
            (min, max) => format!("between {min:?} and {max:?} matching candidates"),
        }
    }

    /// Dart's `describeMismatch`: why `count` does not match.
    fn describe_mismatch(&self, count: usize) -> String {
        match (self.min, self.max) {
            (None, Some(0)) if count == 1 => {
                "means one was found but none were expected".to_string()
            }
            (None, Some(0)) => "means some were found but none were expected".to_string(),
            (Some(1), _) if count == 0 => {
                if self.max == Some(1) {
                    "means none were found but one was expected".to_string()
                } else {
                    "means none were found but some were expected".to_string()
                }
            }
            (Some(min), _) if count < min => "is not enough".to_string(),
            _ => "is too many".to_string(),
        }
    }
}

/// Dart's `expect(finder, matcher)`: fails the case, with Dart's description, unless `finder`
/// finds as many elements as `matcher` expects.
#[track_caller]
pub fn expect(tester: &WidgetTester, finder: &Finder, matcher: FinderMatcher) {
    let found = finder.evaluate(tester);
    if matcher.matches(found.len()) {
        return;
    }
    let plural = if found.len() == 1 {
        "widget"
    } else {
        "widgets"
    };
    panic!(
        "Expected: {}\n  Actual: Found {} {plural} with {}: {found:?}\n   Which: {}",
        matcher.describe(),
        found.len(),
        finder.description(),
        matcher.describe_mismatch(found.len())
    );
}
