//! The producer: a thread of its own that asks for a vsync, draws the next frame into a free
//! buffer, and presents it, as a video decoder or a program in a launcher would. It keeps
//! three buffers, which is what a surface on macOS needs (`docs/composition.md`), and counts
//! what happened so a run can be judged from its output: among it, how often the buffer on
//! screen was in use by the system's compositor at a vsync, which it is only while the
//! compositor really shows the surface's layer.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::{Duration, Instant};

use inset_embedder::{Buffer, Surface, VsyncObserver};

use crate::iosurface;

/// How many buffers the producer draws into in turn.
const BUFFERS: usize = 3;

/// How long to wait for a vsync or a free buffer before counting it as missed: a little more
/// than one refresh at 60 Hz.
const PATIENCE: Duration = Duration::from_millis(20);

pub(crate) fn start(surface: Arc<dyn Surface>, size: [usize; 2], exit_after: Option<Duration>) {
    thread::Builder::new()
        .name("producer".into())
        .spawn(move || Producer::new(surface, size).run(exit_after))
        .expect("a thread for the producer");
}

struct Producer {
    surface: Arc<dyn Surface>,
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
    /// Frames skipped because every buffer was still the compositor's.
    stalls: u64,
    /// Vsyncs at which the buffer on screen was in use by the system's compositor.
    on_screen_in_use: u64,
}

impl Producer {
    fn new(surface: Arc<dyn Surface>, [width, height]: [usize; 2]) -> Producer {
        let (hand_back, free_buffers) = mpsc::channel();
        let buffers = (0..BUFFERS)
            .map(|index| {
                let _ = hand_back.send(index);
                iosurface::allocate(width, height)
            })
            .collect();
        let (vsync, vsyncs) = VsyncSignal::new();
        Producer {
            surface,
            buffers,
            hand_back,
            free_buffers,
            vsync,
            vsyncs,
            counts: Counts::default(),
            frame: 0,
            on_screen: None,
        }
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
                    &format!("{:.1} s in all", started.elapsed().as_secs_f64()),
                    self.counts,
                );
                std::process::exit(0);
            }
        }
    }

    /// Asks the surface for the next vsync of its display and waits for it.
    fn wait_for_vsync(&mut self) -> bool {
        let observer = Arc::downgrade(&self.vsync) as Weak<dyn VsyncObserver>;
        self.surface.next_vsync(observer);
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

    /// Draws the next frame into a buffer the compositor has let go of, and presents it.
    fn draw_next_frame(&mut self) {
        let Some(index) = self.take_free_buffer() else {
            self.counts.stalls += 1;
            return;
        };
        iosurface::fill(&self.buffers[index], self.frame);
        let hand_back = self.hand_back.clone();
        self.surface.present(
            self.buffers[index].clone(),
            Box::new(move || {
                let _ = hand_back.send(index);
            }),
        );
        self.counts.presented += 1;
        self.frame += 1;
        self.on_screen = Some(index);
    }

    fn take_free_buffer(&mut self) -> Option<usize> {
        let index = self.free_buffers.recv_timeout(PATIENCE).ok()?;
        // The first three come from the start, not from a release.
        if self.frame >= BUFFERS as u64 {
            self.counts.released += 1;
        }
        Some(index)
    }
}

impl Counts {
    fn since(self, earlier: Counts) -> Counts {
        Counts {
            vsyncs: self.vsyncs - earlier.vsyncs,
            missed_vsyncs: self.missed_vsyncs - earlier.missed_vsyncs,
            presented: self.presented - earlier.presented,
            released: self.released - earlier.released,
            stalls: self.stalls - earlier.stalls,
            on_screen_in_use: self.on_screen_in_use - earlier.on_screen_in_use,
        }
    }
}

fn report(period: &str, counts: Counts) {
    println!(
        "{period}: {} vsyncs, {} presented, {} buffers released, {} stalls, {} vsyncs missed, \
         buffer on screen in use by the compositor at {} vsyncs",
        counts.vsyncs,
        counts.presented,
        counts.released,
        counts.stalls,
        counts.missed_vsyncs,
        counts.on_screen_in_use
    );
}

/// What the surface calls at a vsync the producer asked for: one message to the producer.
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
