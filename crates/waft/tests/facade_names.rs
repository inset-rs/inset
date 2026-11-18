//! The names an app reaches for are reachable from the facade: one `use waft::…` names the file
//! dialogs a picker answers, the gesture details a callback carries, and the state accessors a
//! stateful widget takes, as it names the widgets.

// Waft's `State` takes `self: Handle<Self>`.
#![feature(arbitrary_self_types)]

use waft::{
    App, AppCell, Body, BuildContext, Canvas, CustomPainter, EdgeInsets, EdgeInsetsDirectional,
    FileDialogError, FileDialogs, FileKind, FileType, Files, Handle, HttpClient, HttpError,
    IntoWidget, OpenMode, Paint, PaintingStyle, Path, PathFillType, PickedFile, PickedFolder,
    RedirectPolicy, SaveOptions, ScaleUpdateDetails, SendOptions, SizedBox, State, StateData,
    StatefulWidget, WidgetRef,
};

#[test]
fn the_facade_names_the_file_dialog_and_gesture_types() {
    let _: Option<&dyn FileDialogs> = None;
    let _ = FileType::default();
    let _ = SaveOptions::default();
    let _ = FileDialogError::Failed("no dialog".to_owned());
    let _: Option<PickedFile> = None;
    let _: Option<PickedFolder> = None;
    let _ = EdgeInsets::all(8.0);
    let _ = EdgeInsetsDirectional::only(8.0, 0.0, 0.0, 0.0);
    let details = ScaleUpdateDetails::default();
    assert_eq!(details.scale, 1.0);
}

/// A stateful widget whose state takes its accessors from the facade's `state_accessors!`, which
/// an app names in `use waft::…`.
#[derive(Debug)]
struct Counter;

impl StatefulWidget for Counter {
    type State = CounterState;

    fn create_state(&self) -> CounterState {
        CounterState {
            state: StateData::new(),
        }
    }
}

struct CounterState {
    state: StateData<Counter>,
}

impl State for CounterState {
    type Widget = Counter;
    waft::state_accessors!();

    fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
        SizedBox::shrink().into_widget()
    }
}

#[test]
fn a_state_takes_its_accessors_from_the_facade() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let kept = app.create(Counter.create_state());
    let handle = kept.handle();
    let bag = CounterState::state_data(handle, &app) as *const _;
    let bag_mut = CounterState::state_data_mut(handle, &mut app) as *const _;
    assert_eq!(bag, bag_mut, "both accessors name the state's bag");
}

#[test]
fn the_facade_names_custom_painting_and_the_host_files_and_http() {
    fn paints(_canvas: &mut Canvas, _paint: &Paint, _path: &Path) {}
    let _: fn(&mut Canvas, &Paint, &Path) = paints;
    let _: Option<&dyn CustomPainter> = None;
    let _ = PathFillType::NonZero;
    let _ = PaintingStyle::Fill;

    let _: Option<&dyn Files> = None;
    let _ = FileKind::File;
    let _ = OpenMode::default();

    let _: Option<&dyn HttpClient> = None;
    let _ = Body::empty();
    let _ = RedirectPolicy::Never;
    let _ = SendOptions::default();
    let _: Option<HttpError> = None;
}

#[test]
fn the_facade_names_the_timers_and_the_awaitable_delay() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();

    let delay = waft::delayed(&mut app, std::time::Duration::from_millis(5));
    drop(delay);
    assert!(
        app.has_pending_timers(),
        "a delay waits on the app's timers"
    );

    let periodic = waft::Timer::periodic(&mut app, std::time::Duration::from_millis(5), |_, _| {});
    periodic.cancel(&mut app);
    assert!(!periodic.is_active(&app));
}

/// The types an app-facing widget setter takes, reached by the glob an app writes: `use
/// waft::*`.
mod glob {
    use std::rc::Rc;

    use waft::*;

    #[test]
    fn use_waft_reaches_what_widget_setters_take() {
        let _: Option<HitTestBehavior> = None;
        let _: Option<Locale> = None;
        let _: Option<PointerDeviceKind> = None;
        let _: Option<TextSelection> = None;
        let _: Option<TextEditingValue> = None;
        let _: Option<TextPosition> = None;
        let _: Option<TextRange> = None;
        let _: Option<TextInputType> = None;
        let _: Option<TextInputAction> = None;
        let _: Option<TextCapitalization> = None;
        let _: Option<SmartDashesType> = None;
        let _: Option<SmartQuotesType> = None;
        let _: Option<BoxHeightStyle> = None;
        let _: Option<BoxWidthStyle> = None;
        let _: Option<ImageFilter> = None;
        let _: Option<FilterQuality> = None;
        let _: Option<PlaceholderAlignment> = None;
        let _: Option<RRect> = None;
        let _: Option<RSuperellipse> = None;
        let _: Option<Rc<dyn CustomClipper<Rect>>> = None;
        let _: Option<BoxConstraintsTransform> = None;
        let _: Option<CacheExtentStyle> = None;
        let _: Option<ScrollCacheExtent> = None;
        let _: Option<ScrollDirection> = None;
        let _: Option<SliverPaintOrder> = None;
        let _: Option<ItemExtentBuilder> = None;
        let _: Option<Rc<dyn SliverGridDelegate>> = None;
        let _: Option<Rc<dyn MultiChildLayoutDelegate>> = None;
        let _: Option<Rc<dyn SingleChildLayoutDelegate>> = None;
        let _: Option<RelativeRect> = None;
        let _: Option<Ref<LayerLink>> = None;
        let _: Option<DecorationPosition> = None;
        let _: Option<OverflowBoxFit> = None;
        let _: Option<WrapAlignment> = None;
        let _: Option<WrapCrossAlignment> = None;
        let _: Option<FloatingHeaderSnapConfiguration> = None;
        let _: Option<OverScrollHeaderStretchConfiguration> = None;
        let _: Option<PersistentHeaderShowOnScreenConfiguration> = None;
        let _: Option<ImageFilterConfig> = None;
        let _: Option<SelectedContent> = None;
        let _: Option<SelectionRegistrarRef> = None;
        let _: Option<TextSelectionPoint> = None;
        let _: Option<AnyBoxConstraints> = None;
        let _: Option<TickerProviderRef> = None;
        let _: Option<ViewRef> = None;
        let _: Option<ViewId> = None;
        let _: Option<ViewFocusEvent> = None;
        let _: Option<Rc<dyn CompositionLayer>> = None;
        let _: Option<CaptionStyle> = None;
        let _: Option<Cue> = None;
        let _: Option<SeekTolerance> = None;
        let _: Option<Rc<dyn VideoFrameReader>> = None;
        let _: Option<VideoOutput> = None;
        let _: Option<Camera> = None;
        let _: Option<Rc<dyn CameraDevice>> = None;
        let _: Option<CameraFormat> = None;
        let _: Option<CameraOutputs> = None;
        let _: Option<CheckState> = None;
        let _: Option<ClipboardEvent> = None;
        let _: Option<EditCommand> = None;
        let _: Option<IconImage> = None;
        let _: Option<MenuItemCheck> = None;
        let _: Option<RestorationData> = None;
        let _: Option<SystemUiOverlayStyle> = None;
        let _: Option<TrayIconEvent> = None;
        let _: Option<TrayIconId> = None;

        assert_eq!(HitTestBehavior::Opaque, HitTestBehavior::Opaque);
    }
}
