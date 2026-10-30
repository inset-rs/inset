//! Files dragged over a window, as winit reports a drag: as it enters a window, at each move over
//! it, as it is dropped and as it leaves, by the id of its data transfer. The files come apart
//! from those events: the embedder fetches the drag's list of URIs as it enters, and winit answers
//! the fetch later, with an event of its own. So the app hears a drag enter once its files are
//! known, its moves after that, and its drop with those files, which waits for them where the
//! drop comes first. A drag whose list names no file is refused, and the app hears nothing of it.

use std::collections::HashMap;
use std::path::PathBuf;

use inset_embedder::{DropChange, DropData, Offset, ViewId};
use percent_encoding::percent_decode_str;
use winit::data_transfer::DataTransferId;
use winit::event_loop::AsyncRequestSerial;

/// The drags over the views, by their transfer, and the fetches of their files winit has yet to
/// answer.
#[derive(Default)]
pub(crate) struct Drags {
    over: HashMap<DataTransferId, Drag>,
    fetching: HashMap<AsyncRequestSerial, DataTransferId>,
}

/// One drag over a view.
struct Drag {
    view: ViewId,
    /// Where it was last reported, in the view's physical pixels.
    position: Option<Offset>,
    /// Its files, once winit has answered the fetch; until then the app has heard nothing of it.
    files: Option<Vec<PathBuf>>,
    /// Whether it was dropped before its files were known.
    dropped: bool,
}

impl Drags {
    /// The drag `transfer` came over `view` at `position`, and `fetch` asks for its files.
    pub(crate) fn entered(
        &mut self,
        transfer: DataTransferId,
        view: ViewId,
        position: Option<Offset>,
        fetch: AsyncRequestSerial,
    ) {
        self.over.insert(
            transfer,
            Drag {
                view,
                position,
                files: None,
                dropped: false,
            },
        );
        self.fetching.insert(fetch, transfer);
    }

    /// The files `fetch` asked for: the drag enters with them, and is dropped with them when the
    /// drop came first. `Err` with the drag's transfer when they name no file, so the drag is
    /// refused; it is forgotten.
    pub(crate) fn files_arrived(
        &mut self,
        fetch: AsyncRequestSerial,
        files: Vec<PathBuf>,
    ) -> Result<Vec<DropData>, DataTransferId> {
        let Some(transfer) = self.fetching.remove(&fetch) else {
            return Ok(Vec::new());
        };
        if files.is_empty() {
            self.over.remove(&transfer);
            return Err(transfer);
        }
        let Some(drag) = self.over.get_mut(&transfer) else {
            return Ok(Vec::new());
        };
        let mut reports = vec![drag.report(DropChange::Entered, files.clone())];
        if drag.dropped {
            reports.push(drag.report(DropChange::Dropped, files));
            self.over.remove(&transfer);
        } else {
            drag.files = Some(files);
        }
        Ok(reports)
    }

    /// The drag `transfer` moved to `position`: the app hears it once it has heard the drag enter.
    pub(crate) fn moved(&mut self, transfer: DataTransferId, position: Offset) -> Option<DropData> {
        let drag = self.over.get_mut(&transfer)?;
        drag.position = Some(position);
        drag.files.as_ref()?;
        Some(drag.report(DropChange::Moved, Vec::new()))
    }

    /// The drag `transfer` was let go: the app hears it with its files, now, or once they are
    /// known.
    pub(crate) fn dropped(&mut self, transfer: DataTransferId) -> Option<DropData> {
        let drag = self.over.get_mut(&transfer)?;
        let Some(files) = drag.files.take() else {
            drag.dropped = true;
            return None;
        };
        let report = drag.report(DropChange::Dropped, files);
        self.over.remove(&transfer);
        Some(report)
    }

    /// The drag `transfer` left without dropping: the app hears it if it heard the drag enter.
    pub(crate) fn left(&mut self, transfer: DataTransferId) -> Option<DropData> {
        let drag = self.over.remove(&transfer)?;
        self.fetching.retain(|_, fetched| *fetched != transfer);
        drag.files.as_ref()?;
        Some(DropData {
            view_id: drag.view,
            change: DropChange::Exited,
            physical_position: None,
            paths: Vec::new(),
        })
    }
}

impl Drag {
    fn report(&self, change: DropChange, paths: Vec<PathBuf>) -> DropData {
        DropData {
            view_id: self.view,
            change,
            physical_position: self.position,
            paths,
        }
    }
}

/// The local file a `file:` URI names (RFC 8089), as winit lists a drag's files: its path, decoded;
/// on Windows a drive's path or, for another host, a share's. `None` for another scheme, a file on
/// another host elsewhere, and a path the system cannot name.
pub(crate) fn file_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let (host, path) = rest.split_at(rest.find('/')?);
    let path = percent_decode_str(path);
    let local = host.is_empty() || host.eq_ignore_ascii_case("localhost");
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let bytes: Vec<u8> = path.collect();
        local.then(|| PathBuf::from(OsStr::from_bytes(&bytes)))
    }
    #[cfg(windows)]
    {
        let path = path.decode_utf8().ok()?.replace('/', "\\");
        if local {
            // `\C:\dir` is the drive's path `C:\dir`.
            return Some(PathBuf::from(path.strip_prefix('\\').unwrap_or(&path)));
        }
        Some(PathBuf::from(format!("\\\\{host}{path}")))
    }
    #[cfg(not(any(unix, windows)))]
    {
        local.then(|| PathBuf::from(path.decode_utf8().ok()?.into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: ViewId = ViewId(1);
    const DRAG: DataTransferId = DataTransferId::from_raw(7);

    fn files(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    fn changes(reports: &[DropData]) -> Vec<DropChange> {
        reports.iter().map(|report| report.change).collect()
    }

    #[test]
    fn a_drag_enters_once_its_files_are_known_and_drops_with_them() {
        let mut drags = Drags::default();
        let fetch = AsyncRequestSerial::get();
        drags.entered(DRAG, A, Some(Offset::new(1.0, 2.0)), fetch);
        assert!(
            drags.moved(DRAG, Offset::new(3.0, 4.0)).is_none(),
            "no move before the enter"
        );
        let entered = drags.files_arrived(fetch, files(&["/a", "/b"])).unwrap();
        assert_eq!(changes(&entered), [DropChange::Entered]);
        assert_eq!(entered[0].paths, files(&["/a", "/b"]));
        assert_eq!(entered[0].physical_position, Some(Offset::new(3.0, 4.0)));
        let moved = drags.moved(DRAG, Offset::new(5.0, 6.0)).unwrap();
        assert_eq!(moved.change, DropChange::Moved);
        let dropped = drags.dropped(DRAG).unwrap();
        assert_eq!(dropped.change, DropChange::Dropped);
        assert_eq!(dropped.paths, files(&["/a", "/b"]));
        assert_eq!(dropped.physical_position, Some(Offset::new(5.0, 6.0)));
        assert!(drags.left(DRAG).is_none(), "a dropped drag is gone");
    }

    #[test]
    fn a_drop_before_the_files_are_known_waits_for_them() {
        let mut drags = Drags::default();
        let fetch = AsyncRequestSerial::get();
        drags.entered(DRAG, A, None, fetch);
        assert!(drags.dropped(DRAG).is_none());
        let reports = drags.files_arrived(fetch, files(&["/a"])).unwrap();
        assert_eq!(
            changes(&reports),
            [DropChange::Entered, DropChange::Dropped]
        );
        assert_eq!(reports[1].paths, files(&["/a"]));
    }

    #[test]
    fn a_drag_that_leaves_is_reported_only_if_it_entered() {
        let mut drags = Drags::default();
        let fetch = AsyncRequestSerial::get();
        drags.entered(DRAG, A, None, fetch);
        assert!(drags.left(DRAG).is_none(), "the app never heard it enter");
        assert!(
            drags
                .files_arrived(fetch, files(&["/a"]))
                .unwrap()
                .is_empty()
        );

        let fetch = AsyncRequestSerial::get();
        drags.entered(DRAG, A, None, fetch);
        drags.files_arrived(fetch, files(&["/a"])).unwrap();
        let left = drags.left(DRAG).unwrap();
        assert_eq!(left.change, DropChange::Exited);
        assert_eq!(left.physical_position, None);
    }

    #[test]
    fn a_drag_without_files_is_refused_and_forgotten() {
        let mut drags = Drags::default();
        let fetch = AsyncRequestSerial::get();
        drags.entered(DRAG, A, None, fetch);
        assert_eq!(drags.files_arrived(fetch, Vec::new()).err(), Some(DRAG));
        assert!(drags.moved(DRAG, Offset::ZERO).is_none());
        assert!(drags.dropped(DRAG).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_file_uri_is_its_decoded_path() {
        assert_eq!(
            file_path("file:///Users/me/My%20File.txt"),
            Some(PathBuf::from("/Users/me/My File.txt"))
        );
        assert_eq!(
            file_path("file://localhost/tmp/a"),
            Some(PathBuf::from("/tmp/a"))
        );
        assert_eq!(file_path("file://server/share/a"), None);
        assert_eq!(file_path("https://example.com/a"), None);
    }
}
