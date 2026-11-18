//! The names an app reaches for are reachable from the facade: one `use waft::…` names the file
//! dialogs a picker answers and the gesture details a callback carries, as it names the widgets.

use waft::{
    EdgeInsets, EdgeInsetsDirectional, FileDialogError, FileDialogs, FileType, PickedFile,
    PickedFolder, SaveOptions, ScaleUpdateDetails,
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
