//! Flutter counterpart: `widgets/orientation_builder.dart`.

use std::fmt;
use std::rc::Rc;

use waft_foundation::App;
use waft_rendering::BoxConstraints;

use crate::framework::{BuildContext, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::layout_builder::LayoutBuilder;
use crate::widgets::media_query::{MediaQuery, Orientation};

/// Signature for a function that builds a widget given an [`Orientation`]: Dart's
/// `OrientationWidgetBuilder`.
pub type OrientationWidgetBuilder = Rc<dyn Fn(&mut App, BuildContext, Orientation) -> WidgetRef>;

/// Builds a widget tree that can depend on the parent widget's orientation: whether its
/// constraints are wider than they are tall.
///
/// This is not the device's orientation, which [`DeviceOrientationBuilder`] reads from the
/// [`MediaQuery`].
pub struct OrientationBuilder {
    pub key: Option<KeyRef>,
    /// Builds the widgets below this widget given this widget's orientation.
    pub builder: OrientationWidgetBuilder,
}

impl OrientationBuilder {
    /// Creates an orientation builder.
    pub fn new(
        builder: impl Fn(&mut App, BuildContext, Orientation) -> WidgetRef + 'static,
    ) -> OrientationBuilder {
        OrientationBuilder {
            key: None,
            builder: Rc::new(builder),
        }
    }

    /// Dart `OrientationBuilder(key:)`.
    pub fn key(mut self, key: KeyRef) -> OrientationBuilder {
        self.key = Some(key);
        self
    }
}

/// Dart's `_buildWithConstraints`: fully unbounded constraints prefer portrait, since scrolling
/// vertically is the more common.
fn orientation_of_constraints(constraints: BoxConstraints) -> Orientation {
    if constraints.max_width > constraints.max_height {
        Orientation::Landscape
    } else {
        Orientation::Portrait
    }
}

impl StatelessWidget for OrientationBuilder {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&self.builder);
        LayoutBuilder::new(move |app, context, constraints| {
            builder(
                app,
                context,
                orientation_of_constraints(constraints.constraints()),
            )
        })
        .into_widget()
    }
}

impl fmt::Debug for OrientationBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrientationBuilder")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

/// Builds a widget tree that can depend on the device's orientation, as the [`MediaQuery`]
/// reports it.
///
/// This is not the parent widget's orientation, which [`OrientationBuilder`] reads from its
/// constraints.
pub struct DeviceOrientationBuilder {
    pub key: Option<KeyRef>,
    /// Builds the widgets below this widget given the device's orientation.
    pub builder: OrientationWidgetBuilder,
}

impl DeviceOrientationBuilder {
    /// Creates a device orientation builder.
    pub fn new(
        builder: impl Fn(&mut App, BuildContext, Orientation) -> WidgetRef + 'static,
    ) -> DeviceOrientationBuilder {
        DeviceOrientationBuilder {
            key: None,
            builder: Rc::new(builder),
        }
    }

    /// Dart `DeviceOrientationBuilder(key:)`.
    pub fn key(mut self, key: KeyRef) -> DeviceOrientationBuilder {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for DeviceOrientationBuilder {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let orientation = MediaQuery::orientation_of(app, context);
        (self.builder)(app, context, orientation)
    }
}

impl fmt::Debug for DeviceOrientationBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceOrientationBuilder")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use waft_embedder::{Size, TextDirection};

    use super::*;
    use crate::framework::IntoWidget;
    use crate::testing::{WidgetTester, find, test_widgets};
    use crate::widgets::basic::{Center, Directionality, SizedBox};
    use crate::widgets::container::Container;
    use crate::widgets::media_query::MediaQueryData;
    use crate::widgets::text::Text;

    /// Dart's `MediaQuery(data: MediaQueryData(size: media_size), child: Center(child:
    /// SizedBox(width:, height:, child: builder)))`, with `builder` recording what it was told.
    fn pump_sized(
        tester: &mut WidgetTester,
        media_size: Size,
        size: Size,
        device: bool,
    ) -> Option<Orientation> {
        let seen: Rc<Cell<Option<Orientation>>> = Rc::default();
        let record = {
            let seen = Rc::clone(&seen);
            move |_: &mut App, _: BuildContext, orientation: Orientation| {
                seen.set(Some(orientation));
                Container::new().into_widget()
            }
        };
        let builder = if device {
            DeviceOrientationBuilder::new(record).into_widget()
        } else {
            OrientationBuilder::new(record).into_widget()
        };
        tester.pump_widget(
            MediaQuery::new(
                MediaQueryData::new().size(media_size),
                Center::new().child(
                    SizedBox::new()
                        .width(size.width())
                        .height(size.height())
                        .child(builder),
                ),
            )
            .into_widget(),
        );
        seen.get()
    }

    #[test]
    fn orientation_builder_determines_orientation_from_constraints() {
        test_widgets(|tester| {
            // Device orientation is landscape; the widget's constraints are portrait.
            let orientation = pump_sized(
                tester,
                Size::new(800.0, 600.0),
                Size::new(100.0, 200.0),
                false,
            );
            assert_eq!(orientation, Some(Orientation::Portrait));
        });
    }

    #[test]
    fn orientation_builder_reports_landscape_when_width_is_greater_than_height() {
        test_widgets(|tester| {
            let orientation = pump_sized(
                tester,
                Size::new(600.0, 800.0),
                Size::new(200.0, 100.0),
                false,
            );
            assert_eq!(orientation, Some(Orientation::Landscape));
        });
    }

    #[test]
    fn orientation_builder_rebuilds_when_constraints_change() {
        test_widgets(|tester| {
            let media_size = Size::new(800.0, 600.0);
            let portrait = pump_sized(tester, media_size, Size::new(100.0, 200.0), false);
            assert_eq!(portrait, Some(Orientation::Portrait));
            let landscape = pump_sized(tester, media_size, Size::new(200.0, 100.0), false);
            assert_eq!(landscape, Some(Orientation::Landscape));
        });
    }

    #[test]
    fn orientation_builder_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::shrink().child(OrientationBuilder::new(
                    |_, _, _| Text::new("X").into_widget(),
                ))),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<OrientationBuilder>()),
                Size::ZERO
            );
        });
    }

    #[test]
    fn device_orientation_builder_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::shrink().child(DeviceOrientationBuilder::new(
                    |_, _, _| Text::new("X").into_widget(),
                ))),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<DeviceOrientationBuilder>()),
                Size::ZERO
            );
        });
    }

    #[test]
    fn device_orientation_builder_uses_media_query_orientation() {
        test_widgets(|tester| {
            // The widget's constraints are portrait, but the device is landscape.
            let orientation = pump_sized(
                tester,
                Size::new(800.0, 600.0),
                Size::new(100.0, 200.0),
                true,
            );
            assert_eq!(orientation, Some(Orientation::Landscape));
        });
    }

    #[test]
    fn device_orientation_builder_reports_portrait_when_device_is_portrait() {
        test_widgets(|tester| {
            // The widget's constraints are landscape, but the device is portrait.
            let orientation = pump_sized(
                tester,
                Size::new(600.0, 800.0),
                Size::new(200.0, 100.0),
                true,
            );
            assert_eq!(orientation, Some(Orientation::Portrait));
        });
    }

    #[test]
    fn device_orientation_builder_rebuilds_when_media_query_orientation_changes() {
        test_widgets(|tester| {
            let seen: Rc<Cell<Option<Orientation>>> = Rc::default();
            let pump = |tester: &mut WidgetTester, size: Size| {
                let seen = Rc::clone(&seen);
                tester.pump_widget(
                    MediaQuery::new(
                        MediaQueryData::new().size(size),
                        DeviceOrientationBuilder::new(move |_, _, orientation| {
                            seen.set(Some(orientation));
                            Container::new().into_widget()
                        }),
                    )
                    .into_widget(),
                );
            };
            pump(tester, Size::new(600.0, 800.0));
            assert_eq!(seen.get(), Some(Orientation::Portrait));
            pump(tester, Size::new(800.0, 600.0));
            assert_eq!(seen.get(), Some(Orientation::Landscape));
        });
    }

    #[test]
    fn device_orientation_builder_differs_from_orientation_builder() {
        use crate::widgets::basic::{Column, Expanded};

        test_widgets(|tester| {
            let layout: Rc<Cell<Option<Orientation>>> = Rc::default();
            let device: Rc<Cell<Option<Orientation>>> = Rc::default();
            let recording = |seen: &Rc<Cell<Option<Orientation>>>| {
                let seen = Rc::clone(seen);
                move |_: &mut App, _: BuildContext, orientation: Orientation| {
                    seen.set(Some(orientation));
                    Container::new().into_widget()
                }
            };
            // The device is landscape; the widget's constraints are portrait.
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new().size(Size::new(800.0, 600.0)),
                    Center::new().child(SizedBox::new().width(100.0).height(200.0).child(
                        Column::new().children(vec![
                        Expanded::new(OrientationBuilder::new(recording(&layout))).into_widget(),
                        Expanded::new(DeviceOrientationBuilder::new(recording(&device)))
                            .into_widget(),
                    ]),
                    )),
                )
                .into_widget(),
            );
            assert_eq!(
                layout.get(),
                Some(Orientation::Portrait),
                "OrientationBuilder should use widget constraints"
            );
            assert_eq!(
                device.get(),
                Some(Orientation::Landscape),
                "DeviceOrientationBuilder should use MediaQuery orientation"
            );
        });
    }
}
