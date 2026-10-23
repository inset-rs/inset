//! The producer: a thread of its own that waits for a refresh of its view's screen on the
//! platform's vsync waiter, draws the next frame into a free buffer, and presents it, as a video
//! decoder or a program in a launcher would. It starts with three buffers and never waits for one
//! to come back: the host learns that the compositor has let go of a buffer only at the surface's
//! next present, so when none is free the producer makes another, as Flutter's surface manager
//! does (the macOS host's `os/macos/composition` says why). It counts what happened so a run can
//! be judged from its output: among it, how often the buffer on screen was in use by the system's
//! compositor at a refresh, which it is only while the compositor really shows the surface's
//! layer.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::time::{Duration, Instant};

use inset_embedder::{Buffer, ScreenId, Surface, VsyncObserver, VsyncWaiter};

use crate::iosurface;

/// How many buffers the producer starts with.
const STARTING_BUFFERS: usize = 3;

/// How long to wait for a refresh before counting it as missed: a little more than one refresh
/// at 60 Hz.
const PATIENCE: Duration = Duration::from_millis(20);

/// The screen the surface's view is on, which the app keeps current and the producer paces by.
#[derive(Default)]
pub(crate) struct ViewScreen(Mutex<Option<ScreenId>>);

impl ViewScreen {
    pub(crate) fn set(&self, screen: Option<ScreenId>) {
        *self.0.lock().expect("held briefly") = screen;
    }

    fn get(&self) -> Option<ScreenId> {
        *self.0.lock().expect("held briefly")
    }
}

/// What the producer draws on, and what it paces by.
pub(crate) struct Pacing {
    pub(crate) surface: Arc<dyn Surface>,
    pub(crate) waiter: Arc<dyn VsyncWaiter>,
    pub(crate) screen: Arc<ViewScreen>,
}

pub(crate) fn start(pacing: Pacing, size: [usize; 2], exit_after: Option<Duration>) {
    thread::Builder::new()
        .name("producer".into())
        .spawn(move || Producer::new(pacing, size).run(exit_after))
        .expect("a thread for the producer");
}

struct Producer {
    pacing: Pacing,
    size: [usize; 2],
    buffers: Vec<Buffer>,
    /// Where a buffer's release callback hands its index back, and where the producer takes
    /// the next one to draw into.
    hand_back: Sender<usize>,
    free_buffers: Receiver<usize>,
    vsync: Arc<VsyncSignal>,
    vsyncs: Receiver<Instant>,
    counts: Counts,
    frame: u64,
    /// The buffer presented last, which the surface shows now.
    on_screen: Option<usize>,
}

/// What happened, since the last report and since the start.
#[derive(Clone, Copy, Default)]
struct Counts {
    vsyncs: u64,
    missed_vsyncs: u64,
    presented: u64,
    released: u64,
    /// Buffers made because every one was still the compositor's.
    added: u64,
    /// Vsyncs at which the buffer on screen was in use by the system's compositor.
    on_screen_in_use: u64,
}

impl Producer {
    fn new(pacing: Pacing, size: [usize; 2]) -> Producer {
        let (hand_back, free_buffers) = mpsc::channel();
        let (vsync, vsyncs) = VsyncSignal::new();
        let mut producer = Producer {
            pacing,
            size,
            buffers: Vec::new(),
            hand_back,
            free_buffers,
            vsync,
            vsyncs,
            counts: Counts::default(),
            frame: 0,
            on_screen: None,
        };
        for _ in 0..STARTING_BUFFERS {
            let index = producer.allocate_buffer();
            let _ = producer.hand_back.send(index);
        }
        producer
    }

    fn run(mut self, exit_after: Option<Duration>) {
        let started = Instant::now();
        let mut last_report = (started, self.counts);
        loop {
            if self.wait_for_vsync() {
                self.draw_next_frame();
            }
            if last_report.0.elapsed() >= Duration::from_secs(1) {
                report("last second", self.counts.since(last_report.1));
                last_report = (Instant::now(), self.counts);
            }
            if exit_after.is_some_and(|after| started.elapsed() >= after) {
                report(
                    &format!(
                        "{:.1} s in all, {} buffers",
                        started.elapsed().as_secs_f64(),
                        self.buffers.len()
                    ),
                    self.counts,
                );
                std::process::exit(0);
            }
        }
    }

    /// Asks for the next refresh of the screen the view is on and waits for it; a view on no
    /// screen has none to wait for.
    fn wait_for_vsync(&mut self) -> bool {
        let Some(screen) = self.pacing.screen.get() else {
            thread::sleep(PATIENCE);
            self.counts.missed_vsyncs += 1;
            return false;
        };
        let observer = Arc::downgrade(&self.vsync) as Weak<dyn VsyncObserver>;
        self.pacing.waiter.next_vsync(screen, observer);
        match self.vsyncs.recv_timeout(PATIENCE) {
            Ok(_) => {
                self.counts.vsyncs += 1;
                self.count_on_screen_in_use();
                true
            }
            Err(RecvTimeoutError::Timeout) => {
                self.counts.missed_vsyncs += 1;
                false
            }
            Err(RecvTimeoutError::Disconnected) => unreachable!("the producer holds the signal"),
        }
    }

    /// Counts whether the compositor holds the buffer the surface shows.
    fn count_on_screen_in_use(&mut self) {
        if let Some(on_screen) = self.on_screen
            && self.buffers[on_screen].is_in_use()
        {
            self.counts.on_screen_in_use += 1;
        }
    }

    /// Draws the next frame into a buffer the compositor has let go of, or a new one, and
    /// presents it.
    fn draw_next_frame(&mut self) {
        let index = self.take_free_buffer();
        iosurface::fill(&self.buffers[index], self.frame);
        let hand_back = self.hand_back.clone();
        self.pacing.surface.present(
            self.buffers[index].clone(),
            Box::new(move || {
                let _ = hand_back.send(index);
            }),
        );
        self.counts.presented += 1;
        self.frame += 1;
        self.on_screen = Some(index);
    }

    /// A buffer the compositor has let go of, or a new one when it holds every one.
    fn take_free_buffer(&mut self) -> usize {
        let Ok(index) = self.free_buffers.try_recv() else {
            self.counts.added += 1;
            return self.allocate_buffer();
        };
        // The starting buffers come from the start, not from a release.
        if self.frame >= STARTING_BUFFERS as u64 {
            self.counts.released += 1;
        }
        index
    }

    /// A buffer more, at the surface's size; its index.
    fn allocate_buffer(&mut self) -> usize {
        let [width, height] = self.size;
        self.buffers.push(iosurface::allocate(width, height));
        self.buffers.len() - 1
    }
}

impl Counts {
    fn since(self, earlier: Counts) -> Counts {
        Counts {
            vsyncs: self.vsyncs - earlier.vsyncs,
            missed_vsyncs: self.missed_vsyncs - earlier.missed_vsyncs,
            presented: self.presented - earlier.presented,
            released: self.released - earlier.released,
            added: self.added - earlier.added,
            on_screen_in_use: self.on_screen_in_use - earlier.on_screen_in_use,
        }
    }
}

fn report(period: &str, counts: Counts) {
    println!(
        "{period}: {} vsyncs, {} presented, {} buffers released, {} buffers added, {} vsyncs \
         missed, buffer on screen in use by the compositor at {} vsyncs",
        counts.vsyncs,
        counts.presented,
        counts.released,
        counts.added,
        counts.missed_vsyncs,
        counts.on_screen_in_use
    );
}

/// What the waiter calls at a refresh the producer asked for: one message to the producer.
struct VsyncSignal {
    vsyncs: Sender<Instant>,
}

impl VsyncSignal {
    fn new() -> (Arc<VsyncSignal>, Receiver<Instant>) {
        let (vsyncs, received) = mpsc::channel();
        (Arc::new(VsyncSignal { vsyncs }), received)
    }
}

impl VsyncObserver for VsyncSignal {
    fn vsync(&self, at: Instant) {
        let _ = self.vsyncs.send(at);
    }
}
