//! [`test_widgets`]: Dart's `testWidgets`, the entry every widget case runs through.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use waft_embedder::TargetPlatform;

use super::WidgetTester;

/// The stack a case runs on: the 8 MiB a platform's main thread gives the framework, and 1 MiB
/// more, which a debug build's theme animation (`ThemeData::lerp` of two 180 KB themes) came to
/// need. A test thread's 2 MiB is too little for a debug build of a theme.
const MAIN_THREAD_STACK_SIZE: usize = 9 * 1024 * 1024;

/// Dart's `testWidgets(description, (tester) async { .. })`: runs `body` with a new
/// [`WidgetTester`] on Android, on a thread with a main thread's stack, then runs the tester's
/// end-of-test checks.
///
/// ```text
/// #[test]
/// fn a_tap_reaches_the_button() {
///     test_widgets(|tester| {
///         tester.pump_widget(button);
///         tester.tap(center, 1);
///         tester.pump();
///     });
/// }
/// ```
pub fn test_widgets(body: impl FnOnce(&mut WidgetTester) + Send) {
    on_main_thread_stack(|| {
        let mut tester = WidgetTester::new();
        body(&mut tester);
    });
}

/// Dart's `testWidgets(.., variant: variant)`: runs `body` once per platform of `variant`, each
/// with a new tester on that platform, which [`WidgetTester::target_platform`] answers. A
/// failing run names its platform.
pub fn test_widgets_variant(
    variant: TargetPlatformVariant,
    body: impl Fn(&mut WidgetTester) + Sync,
) {
    on_main_thread_stack(|| {
        for platform in variant.platforms {
            let run = catch_unwind(AssertUnwindSafe(|| {
                let mut tester = WidgetTester::on(platform);
                body(&mut tester);
            }));
            if let Err(panic) = run {
                eprintln!("the case failed with variant: TargetPlatform.{platform:?}");
                resume_unwind(panic);
            }
        }
    });
}

/// Dart's `TargetPlatformVariant`: the platforms a case runs on, one run each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetPlatformVariant {
    platforms: Vec<TargetPlatform>,
}

impl TargetPlatformVariant {
    /// Dart's `TargetPlatformVariant(values)`.
    pub fn new(platforms: impl IntoIterator<Item = TargetPlatform>) -> TargetPlatformVariant {
        TargetPlatformVariant {
            platforms: platforms.into_iter().collect(),
        }
    }

    /// Dart's `TargetPlatformVariant.only(platform)`.
    pub fn only(platform: TargetPlatform) -> TargetPlatformVariant {
        TargetPlatformVariant::new([platform])
    }

    /// Dart's `TargetPlatformVariant.all()`, in Dart's enum order.
    pub fn all() -> TargetPlatformVariant {
        TargetPlatformVariant::new([
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ])
    }

    /// Dart's `TargetPlatformVariant.all(excluding: excluding)`.
    pub fn all_excluding(excluding: &[TargetPlatform]) -> TargetPlatformVariant {
        TargetPlatformVariant::new(
            TargetPlatformVariant::all()
                .platforms
                .into_iter()
                .filter(|platform| !excluding.contains(platform)),
        )
    }

    /// Dart's `TargetPlatformVariant.desktop()`.
    pub fn desktop() -> TargetPlatformVariant {
        TargetPlatformVariant::new([
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ])
    }

    /// Dart's `TargetPlatformVariant.mobile()`.
    pub fn mobile() -> TargetPlatformVariant {
        TargetPlatformVariant::new([
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
        ])
    }

    /// The platforms, in the order the case runs on them.
    pub fn platforms(&self) -> &[TargetPlatform] {
        &self.platforms
    }
}

/// Runs `body` on a thread with [`MAIN_THREAD_STACK_SIZE`] of stack, re-raising its panic.
fn on_main_thread_stack(body: impl FnOnce() + Send) {
    std::thread::scope(|scope| {
        let joined = std::thread::Builder::new()
            .stack_size(MAIN_THREAD_STACK_SIZE)
            .spawn_scoped(scope, body)
            .expect("a thread for the case")
            .join();
        if let Err(panic) = joined {
            resume_unwind(panic);
        }
    });
}
