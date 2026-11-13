//! The window: a layer in the middle, the framework's label above its bottom edge, both shown
//! as the command line says.

use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;

use waft_embedder::{Color, CompositionLayer, PlatformRef, TextDirection, ViewRef, WindowConfig};
use waft_embedder_winit::{LinuxSettings, WinitEmbedder};
use waft_foundation::{App, Handle};
use waft_painting::{BorderRadiusGeometry, BoxDecoration, EdgeInsetsGeometry, TextStyle};
use waft_scheduler::{PersistentFrameCallback, SchedulerBinding};
use waft_shell::Shell;
use waft_widgets::{
    BuildContext, Center, ClipRRect, CompositionLayerView, DecoratedBox, Directionality,
    IntoWidget, Opacity, Padding, Positioned, SizedBox, Stack, StackFit, State, StateData,
    StatefulWidget, Text, Transform, View, WidgetRef, run_app_with,
};

use crate::producer::{self, Pacing, ViewScreen};

/// The layer's size in logical pixels.
const LAYER_SIZE: [f64; 2] = [400.0, 300.0];
const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const LABEL_BACKGROUND: Color = Color::from_argb(160, 0, 0, 0);
const LABEL: Color = Color::from_argb(255, 255, 255, 255);

/// How the layer and its label are shown, together.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Presentation {
    /// What they are scaled by, about their middle.
    pub(crate) scale: f64,
    pub(crate) opacity: f64,
    /// How far they are turned about their middle, clockwise.
    pub(crate) turn_degrees: f64,
    /// The radius their corners are cut round by.
    pub(crate) corner_radius: f64,
}

/// Runs the window, the layer and its label shown as `presentation` says; closing the window
/// quits, as an app's last window does.
pub(crate) fn run(exit_after: Option<Duration>, presentation: Presentation) {
    WinitEmbedder::default()
        .linux(LinuxSettings {
            app_id: Some(waft_macros::linux_app_id!().to_owned()),
        })
        .run(move |platform| {
            Shell::new(platform, move |app| {
                let config = WindowConfig {
                    title: "Waft — a composition layer".to_owned(),
                    size: [640.0, 480.0],
                    ..WindowConfig::default()
                };
                let home = Home {
                    exit_after,
                    presentation,
                };
                run_app_with(app, home.into_widget(), config);
            })
        });
}

/// The window's content: the layer and its label, the layer's producer started once the view the
/// content is in has a size.
#[derive(Debug)]
struct Home {
    exit_after: Option<Duration>,
    presentation: Presentation,
}

struct HomeState {
    state: StateData<Home>,
    layer: Option<Arc<dyn CompositionLayer>>,
    /// Whether the producer was started for the view the content is in.
    producing: bool,
}

impl StatefulWidget for Home {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        HomeState {
            state: StateData::new(),
            layer: None,
            producing: false,
        }
    }
}

impl State for HomeState {
    type Widget = Home;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let layer = make_layer(&app.platform());
        app.get_mut(self).layer = Some(layer);
    }

    /// Starts the producer for the view the content is in, once.
    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        if std::mem::replace(&mut app.get_mut(self).producing, true) {
            return;
        }
        let view = View::of(app, self.context(app));
        let layer = Arc::clone(app.get(self).layer.as_ref().expect("made in init_state"));
        let exit_after = self.widget(app).exit_after;
        start_producer_once_sized(app, view, layer, exit_after);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let layer = Arc::clone(app.get(self).layer.as_ref().expect("made in init_state"));
        home(layer, self.widget(app).presentation)
    }
}

/// Starts the producer at the first frame in which `view` has a size: only then does the view
/// have the display's pixel ratio the producer draws at. Each frame tells it the screen the view
/// is on, whose refreshes it paces by.
fn start_producer_once_sized(
    app: &mut App,
    view: ViewRef,
    layer: Arc<dyn CompositionLayer>,
    exit_after: Option<Duration>,
) {
    let waiter = app
        .platform()
        .vsync_waiter()
        .expect("the macOS host has a vsync waiter");
    let screen = Arc::new(ViewScreen::default());
    let waiting = Cell::new(Some(Pacing {
        layer,
        waiter,
        screen: Arc::clone(&screen),
    }));
    SchedulerBinding::add_persistent_frame_callback(
        app,
        PersistentFrameCallback::new(move |_app, _views, _elapsed| {
            let metrics = view.metrics();
            screen.set(metrics.screen);
            if metrics.physical_size == [0.0, 0.0] {
                return;
            }
            if let Some(pacing) = waiting.take() {
                producer::start(pacing, pixel_size(metrics.device_pixel_ratio), exit_after);
            }
        }),
    );
}

/// A composition layer from the host's compositor, which the window's frames place.
fn make_layer(platform: &PlatformRef) -> Arc<dyn CompositionLayer> {
    let compositor = platform
        .compositor()
        .expect("the macOS host has a compositor");
    compositor.make_layer(true)
}

/// The layer's size in the display's pixels, which is what the producer draws.
fn pixel_size(device_pixel_ratio: f64) -> [usize; 2] {
    LAYER_SIZE.map(|side| (side * device_pixel_ratio).round() as usize)
}

fn home(layer: Arc<dyn CompositionLayer>, presentation: Presentation) -> WidgetRef {
    let rounded = ClipRRect::new()
        .border_radius(BorderRadiusGeometry::circular(presentation.corner_radius))
        .child(framed_layer(layer));
    let faded = Opacity::new(presentation.opacity).child(rounded);
    let turned = Transform::rotate(presentation.turn_degrees.to_radians()).child(faded);
    let scaled = Transform::scale(Some(presentation.scale), None, None).child(turned);
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
            .child(Center::new().child(scaled)),
    )
    .into_widget()
}

/// The layer at its size, with the label over its bottom edge.
fn framed_layer(layer: Arc<dyn CompositionLayer>) -> WidgetRef {
    SizedBox::new()
        .width(LAYER_SIZE[0])
        .height(LAYER_SIZE[1])
        .child(
            Stack::new().fit(StackFit::Expand).children([
                CompositionLayerView::new(layer).into_widget(),
                Positioned::new(label())
                    .left(0.0)
                    .right(0.0)
                    .bottom(0.0)
                    .into_widget(),
            ]),
        )
        .into_widget()
}

fn label() -> WidgetRef {
    DecoratedBox::new(BoxDecoration::new().color(LABEL_BACKGROUND))
        .child(
            Padding::new(EdgeInsetsGeometry::all(8.0)).child(
                Text::new("Waft paints this label above the layer")
                    .style(TextStyle::new().font_size(15.0).color(LABEL)),
            ),
        )
        .into_widget()
}
