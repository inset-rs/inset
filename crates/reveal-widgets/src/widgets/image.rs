//! Flutter counterpart: `widgets/image.dart` (`createLocalImageConfiguration`).
//!
//! `Image`, `precacheImage`, and the image-loading state wait with the image cache.

use reveal_embedder::Size;
use reveal_foundation::App;
use reveal_painting::ImageConfiguration;

use crate::framework::BuildContext;
use crate::widgets::basic::Directionality;
use crate::widgets::media_query::MediaQuery;

/// Creates an [`ImageConfiguration`] based on the given [`BuildContext`] (and optionally
/// size).
///
/// This is the object that must be passed to `BoxPainter.paint` and to
/// `ImageProvider.resolve`.
///
/// If this is not called from a build method, then it should be reevaluated each time the
/// dependencies change, e.g. by calling it from `State::did_change_dependencies`, so that
/// any changes in the environment cause the image to be re-resolved.
///
/// The asset bundle and locale wait with `DefaultAssetBundle` and `Localizations`.
pub fn create_local_image_configuration(
    app: &mut App,
    context: BuildContext,
    size: Option<Size>,
) -> ImageConfiguration {
    ImageConfiguration {
        bundle: None,
        device_pixel_ratio: Some(
            MediaQuery::maybe_device_pixel_ratio_of(app, context).unwrap_or(1.0),
        ),
        text_direction: Directionality::maybe_of(app, context),
        size,
        platform: Some(app.platform().target_platform()),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_embedder::TextDirection;

    use super::*;
    use crate::framework::{IntoWidget, WidgetRef};
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::media_query::MediaQueryData;

    type Seen = Rc<RefCell<Option<ImageConfiguration>>>;

    /// A leaf that records the configuration its context yields.
    fn probe(seen: &Seen, size: Option<Size>) -> WidgetRef {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            *seen.borrow_mut() = Some(create_local_image_configuration(app, context, size));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    #[test]
    fn the_configuration_reads_the_direction_pixel_ratio_size_and_platform() {
        let mut app = App::new();
        let seen: Seen = Rc::default();
        let tree = Directionality::new(
            TextDirection::Rtl,
            MediaQuery::new(
                MediaQueryData::new().device_pixel_ratio(3.0),
                probe(&seen, Some(Size::new(1.0, 2.0))),
            )
            .into_widget(),
        )
        .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);

        let configuration = seen.borrow_mut().take().expect("the builder ran");
        assert_eq!(configuration.text_direction, Some(TextDirection::Rtl));
        assert_eq!(configuration.device_pixel_ratio, Some(3.0));
        assert_eq!(configuration.size, Some(Size::new(1.0, 2.0)));
        assert_eq!(
            configuration.platform,
            Some(app.platform().target_platform())
        );
        assert!(configuration.bundle.is_none());
    }

    #[test]
    fn without_ancestors_the_configuration_falls_back() {
        let mut app = App::new();
        let seen: Seen = Rc::default();
        let harness = Harness::mount(&mut app, probe(&seen, None));
        harness.pump(&mut app);

        let configuration = seen.borrow_mut().take().expect("the builder ran");
        assert_eq!(configuration.text_direction, None);
        assert_eq!(configuration.device_pixel_ratio, Some(1.0));
        assert_eq!(configuration.size, None);
    }
}
