//! Files dragged over a window, as winit 0.30 reports them: one event per file, no position, and
//! the files of a phase back to back, since the system hands them over in one call. The embedder
//! gathers a turn's events into reports here and sends them when the loop is about to wait.

use std::path::PathBuf;

use inset_embedder::{DropChange, DropData, ViewId};

/// The turn's reports of files dragged over the views, in the order they happened. A file of the
/// same phase over the same view as the report before it joins that report, so each phase of a
/// drag is one report with all its files.
#[derive(Default)]
pub(crate) struct DragReports(Vec<DropData>);

impl DragReports {
    /// One file of a drag that came over `view`, or was let go over it.
    pub(crate) fn add_file(&mut self, view: ViewId, change: DropChange, path: PathBuf) {
        match self.0.last_mut() {
            Some(last) if last.view_id == view && last.change == change => last.paths.push(path),
            _ => self.0.push(report(view, change, vec![path])),
        }
    }

    /// The drag left `view` without dropping.
    pub(crate) fn add_exit(&mut self, view: ViewId) {
        self.0.push(report(view, DropChange::Exited, Vec::new()));
    }

    /// The turn's reports, in the order they happened.
    pub(crate) fn take(&mut self) -> Vec<DropData> {
        std::mem::take(&mut self.0)
    }
}

/// A report with no position: winit gives none.
fn report(view: ViewId, change: DropChange, paths: Vec<PathBuf>) -> DropData {
    DropData {
        view_id: view,
        change,
        physical_position: None,
        paths,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: ViewId = ViewId(1);
    const B: ViewId = ViewId(2);

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_files_of_one_phase_are_one_report() {
        let mut reports = DragReports::default();
        reports.add_file(A, DropChange::Entered, PathBuf::from("/a"));
        reports.add_file(A, DropChange::Entered, PathBuf::from("/b"));
        assert_eq!(
            reports.take(),
            [report(A, DropChange::Entered, paths(&["/a", "/b"]))]
        );
        assert!(reports.take().is_empty(), "reported once");
    }

    #[test]
    fn the_phases_of_a_drag_report_in_order() {
        let mut reports = DragReports::default();
        reports.add_file(A, DropChange::Entered, PathBuf::from("/a"));
        reports.add_file(A, DropChange::Dropped, PathBuf::from("/a"));
        assert_eq!(
            reports.take(),
            [
                report(A, DropChange::Entered, paths(&["/a"])),
                report(A, DropChange::Dropped, paths(&["/a"])),
            ]
        );
    }

    #[test]
    fn a_drag_moving_to_another_window_leaves_before_it_enters() {
        let mut reports = DragReports::default();
        reports.add_exit(A);
        reports.add_file(B, DropChange::Entered, PathBuf::from("/a"));
        assert_eq!(
            reports.take(),
            [
                report(A, DropChange::Exited, Vec::new()),
                report(B, DropChange::Entered, paths(&["/a"])),
            ]
        );
    }

    #[test]
    fn drags_over_two_windows_in_one_turn_are_both_reported() {
        let mut reports = DragReports::default();
        reports.add_file(A, DropChange::Entered, PathBuf::from("/a"));
        reports.add_file(B, DropChange::Entered, PathBuf::from("/b"));
        assert_eq!(
            reports.take(),
            [
                report(A, DropChange::Entered, paths(&["/a"])),
                report(B, DropChange::Entered, paths(&["/b"])),
            ]
        );
    }
}
