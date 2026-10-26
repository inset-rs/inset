//! A promised file's bytes, written into a file as its producer makes them: at the receiver's URL
//! on macOS, into a temporary file the item provider hands over on iOS. A file whose producer
//! fails is removed, so a part never passes for the whole.

use std::fs::{self, File};
use std::future::{Future, ready};
use std::io::{self, Write};
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;

use inset_embedder::{FileSink, WriteFuture};

use super::app_futures::LocalFuture;

/// What produces a promised file's bytes, each time someone pastes it.
pub(crate) type FileProducer = Rc<dyn Fn(Box<dyn FileSink>) -> WriteFuture>;

/// The file at `path`, written with what `produce` makes: whole once this resolves `Ok`, and
/// removed when it fails.
pub(crate) fn produce_at(path: PathBuf, produce: &FileProducer) -> LocalFuture<io::Result<()>> {
    let file = match File::create(&path) {
        Ok(file) => file,
        Err(error) => return Box::pin(ready(Err(error))),
    };
    let produced = produce(Box::new(OpenFile(file)));
    Box::pin(async move {
        let produced = produced.await;
        if produced.is_err() {
            let _ = fs::remove_file(&path);
        }
        produced
    })
}

/// The file a promised file's bytes are written into as they come.
struct OpenFile(File);

impl FileSink for OpenFile {
    fn write<'a>(
        &'a mut self,
        bytes: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + 'a>> {
        Box::pin(ready(self.0.write_all(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn now<T>(mut future: LocalFuture<T>) -> T {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("the test's producers answer in the call"),
        }
    }

    fn producer(chunks: &'static [&'static [u8]], fails: bool) -> FileProducer {
        Rc::new(move |mut sink: Box<dyn FileSink>| {
            Box::pin(async move {
                for chunk in chunks {
                    sink.write(chunk).await?;
                }
                if fails {
                    return Err(io::Error::other("the producer broke"));
                }
                Ok(())
            })
        })
    }

    fn folder(test: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "inset-promised-files-{}-{test}",
            std::process::id()
        ));
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_promised_file_is_written_as_its_producer_makes_it() {
        let folder = folder("written");
        let path = folder.join("note.txt");
        now(produce_at(
            path.clone(),
            &producer(&[b"prom", b"ised"], false),
        ))
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"promised");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_promised_file_whose_producer_fails_is_removed() {
        let folder = folder("failed");
        let path = folder.join("half.txt");
        let produced = now(produce_at(path.clone(), &producer(&[b"half"], true)));
        assert!(produced.is_err());
        assert!(!path.exists());
        fs::remove_dir_all(&folder).unwrap();
    }
}
