//! Image decoder composition and the native event-loop wake boundary.

use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Wake, Waker};

use valo::ImageContext;
use valo_codec::{Decoder, ImageLoader};

/// Selects where the built-in image decoders run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecodeExecution {
    /// Polls decoding on the caller's executor and starts no threads.
    #[cfg_attr(
        not(all(feature = "image-worker", not(target_arch = "wasm32"))),
        default
    )]
    Local,

    /// Owns decoder state on one native worker, waking the event loop for results.
    #[cfg(all(feature = "image-worker", not(target_arch = "wasm32")))]
    #[default]
    Worker,
}

/// `create_image_loader` registers the decoders this build carries, in preference order.
///
/// The platform's own codec comes first where it was compiled in: it reads formats no portable
/// decoder carries, and hands back frames the GPU can sample without an upload. The software
/// decoder takes whatever that one declines. The loader behaves the same either side of
/// [`DecodeExecution`].
pub fn create_image_loader(
    images: ImageContext,
    execution: DecodeExecution,
) -> std::io::Result<ImageLoader> {
    let decoders: Vec<Box<dyn Decoder>> = vec![
        #[cfg(all(feature = "image-apple", any(target_os = "macos", target_os = "ios")))]
        Box::new(valo_codec_apple::AppleDecoder::default()),
        #[cfg(feature = "image-software")]
        Box::new(valo_codec_software::SoftwareDecoder),
    ];
    match execution {
        DecodeExecution::Local => Ok(ImageLoader::new(images, decoders)),
        #[cfg(all(feature = "image-worker", not(target_arch = "wasm32")))]
        DecodeExecution::Worker => ImageLoader::with_worker(images, decoders),
    }
}

/// Schedules the caller's continuation before waking the native event loop that polls it.
struct EventLoopWake {
    continuation: Waker,
    wake_host: Arc<dyn Fn() + Send + Sync>,
}

impl Wake for EventLoopWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.continuation.wake_by_ref();
        (self.wake_host)();
    }
}

/// Bridges a host future's wakeups to both the foreground task queue and winit.
///
/// Inset polls its ready queue at client callbacks. Waking only the task would leave
/// its continuation queued indefinitely while a window's event loop is asleep.
pub(crate) fn with_event_loop_wake<'a, T: 'a>(
    future: impl Future<Output = T> + 'a,
    wake_host: impl Fn() + Send + Sync + 'static,
) -> Pin<Box<dyn Future<Output = T> + 'a>> {
    let mut future = Box::pin(future);
    let wake_host: Arc<dyn Fn() + Send + Sync> = Arc::new(wake_host);
    Box::pin(poll_fn(move |context| {
        let waker = Waker::from(Arc::new(EventLoopWake {
            continuation: context.waker().clone(),
            wake_host: wake_host.clone(),
        }));
        future.as_mut().poll(&mut Context::from_waker(&waker))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::task::Poll;
    use std::time::Duration;

    struct ReadyTask(Arc<AtomicBool>);

    impl Wake for ReadyTask {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::Release);
        }
    }

    #[test]
    fn completion_queues_the_continuation_before_waking_an_idle_host() {
        let queued = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let saved_waker = Arc::new(Mutex::new(None::<Waker>));
        let (send, receive) = mpsc::channel();
        let saved = saved_waker.clone();
        let ready = completed.clone();
        let queued_at_wake = queued.clone();
        let mut future = with_event_loop_wake(
            poll_fn(move |context| {
                *saved.lock().unwrap() = Some(context.waker().clone());
                if ready.load(Ordering::Acquire) {
                    Poll::Ready(42)
                } else {
                    Poll::Pending
                }
            }),
            move || {
                assert!(queued_at_wake.load(Ordering::Acquire));
                send.send(()).unwrap();
            },
        );
        let waker = Waker::from(Arc::new(ReadyTask(queued)));
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending()
        );
        let worker = std::thread::spawn(move || {
            completed.store(true, Ordering::Release);
            saved_waker.lock().unwrap().take().unwrap().wake();
        });
        receive
            .recv_timeout(Duration::from_secs(5))
            .expect("completion wakes the host without a redraw timer");
        worker.join().unwrap();
        assert_eq!(
            future.as_mut().poll(&mut Context::from_waker(&waker)),
            Poll::Ready(42)
        );
    }
}
