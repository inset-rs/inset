//! Files dragged over a window, as winit 0.30 reports them: one event per file, no
//! position, and all of a drag's files delivered inside one turn of the loop. The turn's
//! end is the batch's end, so the paths are gathered here and reported once, as one
//! `DropData`, when the loop is about to wait.

use std::path::PathBuf;

use inset_embedder::DropChange;
use winit::window::WindowId;

/// A drag's files gathered over one turn, per window they concern.
#[derive(Default)]
pub(crate) struct Drags {
    hovering: Option<(WindowId, Vec<PathBuf>)>,
    dropped: Option<(WindowId, Vec<PathBuf>)>,
    exited: Option<WindowId>,
}

/// One report due at the turn's end.
#[derive(Debug, PartialEq)]
pub(crate) struct DragReport {
    pub(crate) window: WindowId,
    pub(crate) change: DropChange,
    pub(crate) paths: Vec<PathBuf>,
}

impl Drags {
    /// winit's `HoveredFile`: one of the files a drag brought over `window`.
    pub(crate) fn hovered(&mut self, window: WindowId, path: PathBuf) {
        gather(&mut self.hovering, window, path);
    }

    /// winit's `DroppedFile`: one of the files let go over `window`.
    pub(crate) fn dropped(&mut self, window: WindowId, path: PathBuf) {
        gather(&mut self.dropped, window, path);
    }

    /// winit's `HoveredFileCancelled`: the drag left `window` without dropping.
    pub(crate) fn cancelled(&mut self, window: WindowId) {
        self.exited = Some(window);
    }

    /// The reports the turn's events add up to, in the order they happened, emptying the
    /// gathering.
    pub(crate) fn take_reports(&mut self) -> Vec<DragReport> {
        let mut reports = Vec::new();
        if let Some((window, paths)) = self.hovering.take() {
            reports.push(DragReport {
                window,
                change: DropChange::Entered,
                paths,
            });
        }
        if let Some((window, paths)) = self.dropped.take() {
            reports.push(DragReport {
                window,
                change: DropChange::Dropped,
                paths,
            });
        }
        if let Some(window) = self.exited.take() {
            reports.push(DragReport {
                window,
                change: DropChange::Exited,
                paths: Vec::new(),
            });
        }
        reports
    }
}

/// Adds `path` to the batch for `window`, starting one if the batch is another window's.
fn gather(batch: &mut Option<(WindowId, Vec<PathBuf>)>, window: WindowId, path: PathBuf) {
    match batch {
        Some((gathered_for, paths)) if *gathered_for == window => paths.push(path),
        _ => *batch = Some((window, vec![path])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> WindowId {
        WindowId::from(1u64)
    }

    #[test]
    fn a_turn_of_hovered_files_is_one_entry() {
        let mut drags = Drags::default();
        drags.hovered(window(), PathBuf::from("/a"));
        drags.hovered(window(), PathBuf::from("/b"));
        assert_eq!(
            drags.take_reports(),
            [DragReport {
                window: window(),
                change: DropChange::Entered,
                paths: vec![PathBuf::from("/a"), PathBuf::from("/b")],
            }]
        );
        assert!(drags.take_reports().is_empty(), "reported once");
    }

    #[test]
    fn dropped_files_and_a_cancel_report_in_order() {
        let mut drags = Drags::default();
        drags.dropped(window(), PathBuf::from("/a"));
        drags.cancelled(window());
        let reports = drags.take_reports();
        assert_eq!(
            reports
                .iter()
                .map(|report| report.change)
                .collect::<Vec<_>>(),
            [DropChange::Dropped, DropChange::Exited]
        );
        assert_eq!(reports[0].paths, [PathBuf::from("/a")]);
    }
}
