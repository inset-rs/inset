//! A native surface's producer gets vsyncs while its window is visible, none while it is not, and
//! vsyncs again once it is visible again, through each way a window stops being visible and
//! comes back: the embedder hiding and showing it, another window covering and uncovering it, the
//! Dock taking it and giving it back, and the application hidden and shown again.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens two windows for about fifteen seconds, hides the application for a while, and needs a
//! login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_surfaces_vsyncs_follow_its_window_into_and_out_of_sight";

/// Long enough for every step, twice over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(40);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the steps did not end in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::VisibilityClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Weak};
    use std::time::{Duration, Instant};

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerDataPacket, Rect, RequestId,
        SceneBuilder, Surface, ViewId, VsyncObserver, WindowConfig, WindowRef,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    /// How long a transition is given to settle before the vsyncs are counted: the Dock's
    /// animations take about half of it.
    const SETTLE: Duration = Duration::from_millis(1000);
    /// How long the vsyncs are counted in each state.
    const COUNTED_FOR: Duration = Duration::from_millis(500);
    /// The fewest vsyncs a visible window's producer gets in `COUNTED_FOR`: a 60 Hz display gives
    /// about 30.
    const FEWEST_VSYNCS_SEEN: u32 = 10;

    const SURFACE_WINDOW: &str = "a surface's window";

    /// A state the surface's window is brought into, and whether it is visible there.
    struct Step {
        name: &'static str,
        bring: fn(&VisibilityClient),
        visible: bool,
    }

    const STEPS: [Step; 9] = [
        Step {
            name: "shown",
            bring: |_| {},
            visible: true,
        },
        Step {
            name: "hidden by the host",
            bring: |client| client.windows[0].hide(),
            visible: false,
        },
        Step {
            name: "shown by the host",
            bring: |client| client.windows[0].show(),
            visible: true,
        },
        Step {
            name: "covered by another window",
            bring: |client| client.windows[1].show(),
            visible: false,
        },
        Step {
            name: "uncovered",
            bring: |client| client.windows[1].hide(),
            visible: true,
        },
        Step {
            name: "minimized",
            bring: |_| appkit::minimize(SURFACE_WINDOW),
            visible: false,
        },
        Step {
            name: "restored from the Dock",
            bring: |_| appkit::restore(SURFACE_WINDOW),
            visible: true,
        },
        Step {
            name: "application hidden",
            bring: |_| appkit::hide_application(),
            visible: false,
        },
        Step {
            name: "application shown again",
            bring: |_| {
                appkit::unhide_application();
                // Unhiding does not activate the application, so its window can come back
                // behind other applications' windows, where a covered window is not seen.
                appkit::order_in_front_of_everything(SURFACE_WINDOW);
            },
            visible: true,
        },
    ];

    /// Opens a window with a native surface in it and a larger one to cover it, then brings the
    /// first through each step, counting its producer's vsyncs in each. Wakes, which come whether
    /// a window is visible or not, time the steps.
    pub(crate) struct VisibilityClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        windows: Vec<WindowRef>,
        producer: Option<Arc<CountingProducer>>,
        step: usize,
        step_began: Instant,
        counted_from: Option<u32>,
        failures: Vec<String>,
    }

    impl VisibilityClient {
        pub(crate) fn new(platform: PlatformRef) -> VisibilityClient {
            let requested = vec![
                support::request_window(
                    &platform,
                    WindowConfig {
                        title: SURFACE_WINDOW.into(),
                        size: [400.0, 300.0],
                        position: Some([300.0, 300.0]),
                        ..WindowConfig::default()
                    },
                ),
                support::request_window(
                    &platform,
                    WindowConfig {
                        title: "a window covering it".into(),
                        size: [800.0, 700.0],
                        position: Some([100.0, 100.0]),
                        visible: false,
                        ..WindowConfig::default()
                    },
                ),
            ];
            VisibilityClient {
                platform,
                requested,
                windows: Vec::new(),
                producer: None,
                step: 0,
                step_began: Instant::now(),
                counted_from: None,
                failures: Vec::new(),
            }
        }

        /// Places a surface in the first window once both are made, with a producer that asks
        /// for a vsync at every vsync.
        fn place_a_surface_once_both_windows_are_made(&mut self) {
            if self.producer.is_some() || self.windows.len() < 2 {
                return;
            }
            let view = self.windows[0].view();
            let compositor = self
                .platform
                .compositor()
                .expect("the macOS host composites native surfaces");
            let surface = compositor.open_surface(view.id(), true);
            let mut scene = SceneBuilder::new();
            scene.add_surface(
                Arc::clone(&surface),
                Rect::from_ltwh(0.0, 0.0, 200.0, 200.0),
            );
            view.present(Arc::from(scene.build()));
            self.producer = Some(CountingProducer::start(surface));
            self.step_began = Instant::now();
            support::wake_after_a_tick(&self.platform);
        }

        /// Counts the current step's vsyncs once it has settled, and brings the next step when
        /// the count is done. Answers whether every step is done.
        fn go_on(&mut self) -> bool {
            let Some(producer) = &self.producer else {
                return false;
            };
            let vsyncs = producer.vsyncs.load(Ordering::SeqCst);
            let in_step = self.step_began.elapsed();
            if in_step < SETTLE {
                return false;
            }
            let Some(counted_from) = self.counted_from else {
                self.counted_from = Some(vsyncs);
                return false;
            };
            if in_step < SETTLE + COUNTED_FOR {
                return false;
            }
            self.judge(vsyncs - counted_from);
            self.step += 1;
            self.counted_from = None;
            self.step_began = Instant::now();
            let Some(next) = STEPS.get(self.step) else {
                return true;
            };
            (next.bring)(self);
            false
        }

        fn judge(&mut self, vsyncs: u32) {
            let step = &STEPS[self.step];
            let as_expected = if step.visible {
                vsyncs >= FEWEST_VSYNCS_SEEN
            } else {
                vsyncs == 0
            };
            println!(
                "{:<28} {:>4} vsyncs  {}",
                step.name,
                vsyncs,
                if as_expected { "ok" } else { "FAILED" }
            );
            if !as_expected {
                self.failures.push(step.name.to_owned());
            }
        }
    }

    impl EmbedderClient for VisibilityClient {
        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}

        fn wake(&mut self, _elapsed: Duration) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            self.place_a_surface_once_both_windows_are_made();
            if self.producer.is_none() {
                return;
            }
            if !self.go_on() {
                support::wake_after_a_tick(&self.platform);
                return;
            }
            assert!(
                self.failures.is_empty(),
                "vsyncs not as the window's visibility says in: {:?}",
                self.failures
            );
            support::pass(TEST);
        }

        fn view_added(&mut self, _id: ViewId) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            self.place_a_surface_once_both_windows_are_made();
        }
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }

    /// A producer that asks for the next vsync at every vsync, as one that draws continuously
    /// does, and counts them.
    struct CountingProducer {
        surface: Arc<dyn Surface>,
        vsyncs: AtomicU32,
        this: Weak<CountingProducer>,
    }

    impl CountingProducer {
        fn start(surface: Arc<dyn Surface>) -> Arc<CountingProducer> {
            let producer = Arc::new_cyclic(|this| CountingProducer {
                surface,
                vsyncs: AtomicU32::new(0),
                this: this.clone(),
            });
            producer.ask();
            producer
        }

        fn ask(&self) {
            let this: Weak<dyn VsyncObserver> = self.this.clone();
            self.surface.next_vsync(this);
        }
    }

    impl VsyncObserver for CountingProducer {
        fn vsync(&self, _at: Instant) {
            self.vsyncs.fetch_add(1, Ordering::SeqCst);
            self.ask();
        }
    }
}
