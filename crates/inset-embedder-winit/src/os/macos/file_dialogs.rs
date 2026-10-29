//! The file dialogs: AppKit's `NSOpenPanel` and `NSSavePanel`, a sheet on the parent view's
//! window where there is one and a panel of their own otherwise, each answering paths once the
//! user closes it. Both begin without blocking, so the app goes on while they show.
//!
//! A type's extensions and MIME types become the uniform types the panel enables; AppKit shows no
//! type's label without an accessory view, which these have none of. Every pick a panel answers
//! may be read and written, as AppKit grants a sandboxed app both, so `writable` asks nothing more
//! here.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use block2::RcBlock;
use inset_embedder::{
    FileDialogError, FileDialogFuture, FileDialogs, FileType, FolderOptions, OpenOptions,
    PickedFile, PickedFolder, SaveOptions, ViewId,
};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSModalResponse, NSModalResponseCancel, NSModalResponseOK, NSOpenPanel, NSSavePanel,
};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;

use super::{AppKit, ns_window};
use crate::file_dialogs::{DialogAnswer, ParentWindows, dialog_answer};
use crate::os::SystemFileDialogs;

impl SystemFileDialogs for AppKit {
    fn file_dialogs(windows: ParentWindows) -> Option<Rc<dyn FileDialogs>> {
        let main_thread = MainThreadMarker::new()?;
        Some(Rc::new(Panels {
            main_thread,
            windows,
        }))
    }
}

/// AppKit's open and save panels.
struct Panels {
    main_thread: MainThreadMarker,
    windows: ParentWindows,
}

/// What an open panel lets the user choose.
#[derive(Clone, Copy)]
enum Choose {
    Files { multiple: bool },
    Folder,
}

impl FileDialogs for Panels {
    fn open_file(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let chosen = self.open(
            parent,
            &options.types,
            &options.initial_folder,
            &options.confirm_label,
            Choose::Files { multiple: false },
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFile::Path)) })
    }

    fn open_files(
        &self,
        parent: Option<ViewId>,
        options: OpenOptions,
    ) -> FileDialogFuture<Vec<PickedFile>> {
        let chosen = self.open(
            parent,
            &options.types,
            &options.initial_folder,
            &options.confirm_label,
            Choose::Files { multiple: true },
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().map(PickedFile::Path).collect()) })
    }

    fn open_folder(
        &self,
        parent: Option<ViewId>,
        options: FolderOptions,
    ) -> FileDialogFuture<Option<PickedFolder>> {
        let chosen = self.open(
            parent,
            &[],
            &options.initial_folder,
            &options.confirm_label,
            Choose::Folder,
        );
        Box::pin(async move { Ok(chosen.await?.into_iter().next().map(PickedFolder::Path)) })
    }

    fn save_file(
        &self,
        parent: Option<ViewId>,
        options: SaveOptions,
    ) -> FileDialogFuture<Option<PickedFile>> {
        let panel = NSSavePanel::savePanel(self.main_thread);
        panel.setCanCreateDirectories(true);
        if let Some(name) = &options.suggested_name {
            panel.setNameFieldStringValue(&NSString::from_str(name));
        }
        configure(
            &panel,
            &options.types,
            &options.initial_folder,
            &options.confirm_label,
        );
        let chosen = panel.clone();
        let answer = self.present(&panel, parent, move || {
            chosen
                .URL()
                .and_then(|url| url.to_file_path())
                .into_iter()
                .collect()
        });
        Box::pin(async move { Ok(answer.await?.into_iter().next().map(PickedFile::Path)) })
    }
}

impl Panels {
    fn open(
        &self,
        parent: Option<ViewId>,
        types: &[FileType],
        initial_folder: &Option<PathBuf>,
        confirm_label: &Option<String>,
        choose: Choose,
    ) -> FileDialogFuture<Vec<PathBuf>> {
        let panel = NSOpenPanel::openPanel(self.main_thread);
        match choose {
            Choose::Files { multiple } => {
                panel.setCanChooseFiles(true);
                panel.setCanChooseDirectories(false);
                panel.setAllowsMultipleSelection(multiple);
            }
            Choose::Folder => {
                panel.setCanChooseFiles(false);
                panel.setCanChooseDirectories(true);
                panel.setCanCreateDirectories(true);
                panel.setAllowsMultipleSelection(false);
            }
        }
        configure(&panel, types, initial_folder, confirm_label);
        let chosen = panel.clone();
        self.present(&panel, parent, move || paths(&chosen.URLs()))
    }

    /// Shows `panel`, as a sheet on `parent`'s window where it has one, and answers what
    /// `chosen` reads from it once the user confirms, or nothing once they cancel.
    fn present(
        &self,
        panel: &NSSavePanel,
        parent: Option<ViewId>,
        chosen: impl Fn() -> Vec<PathBuf> + 'static,
    ) -> FileDialogFuture<Vec<PathBuf>> {
        let (answer, answered) = dialog_answer();
        let answer = Cell::new(Some(answer));
        let handler = RcBlock::new(move |response: NSModalResponse| {
            if let Some(answer) = answer.take() {
                give(answer, response, &chosen);
            }
        });
        match self
            .windows
            .window(parent)
            .and_then(|window| ns_window(&*window))
        {
            Some(window) => panel.beginSheetModalForWindow_completionHandler(&window, &handler),
            None => panel.beginWithCompletionHandler(&handler),
        }
        answered
    }
}

/// Answers the panel's `response`: what was chosen, nothing for a cancel, and a failure for a
/// panel AppKit could not show.
fn give(
    answer: DialogAnswer<Vec<PathBuf>>,
    response: NSModalResponse,
    chosen: &dyn Fn() -> Vec<PathBuf>,
) {
    let answered = if response == NSModalResponseOK {
        Ok(chosen())
    } else if response == NSModalResponseCancel {
        Ok(Vec::new())
    } else {
        Err(FileDialogError::Failed(format!(
            "AppKit could not show the panel ({response})"
        )))
    };
    answer.give(answered);
}

/// What every panel takes from its options: the types it enables, the folder it starts in, and
/// its confirming button's label.
fn configure(
    panel: &NSSavePanel,
    types: &[FileType],
    initial_folder: &Option<PathBuf>,
    confirm_label: &Option<String>,
) {
    if !types.is_empty() {
        panel.setAllowedContentTypes(&content_types(types));
    }
    if let Some(folder) = initial_folder.as_ref().and_then(NSURL::from_directory_path) {
        panel.setDirectoryURL(Some(&folder));
    }
    if let Some(label) = confirm_label {
        panel.setPrompt(Some(&NSString::from_str(label)));
    }
}

/// The uniform types `types` name, by extension and by MIME type; one the system does not know
/// is left out.
fn content_types(types: &[FileType]) -> Retained<NSArray<UTType>> {
    let by_extension = types
        .iter()
        .flat_map(|kind| &kind.extensions)
        .filter_map(|extension| {
            UTType::typeWithFilenameExtension(&NSString::from_str(
                extension.trim_start_matches('.'),
            ))
        });
    let by_mime_type = types
        .iter()
        .flat_map(|kind| &kind.mime_types)
        .filter_map(|mime| UTType::typeWithMIMEType(&NSString::from_str(mime)));
    let content_types: Vec<Retained<UTType>> = by_extension.chain(by_mime_type).collect();
    NSArray::from_retained_slice(&content_types)
}

fn paths(urls: &NSArray<NSURL>) -> Vec<PathBuf> {
    urls.iter().filter_map(|url| url.to_file_path()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_become_the_uniform_types_they_name() {
        let types = [FileType {
            label: "Text and pictures".to_owned(),
            extensions: vec!["txt".to_owned(), ".md".to_owned()],
            mime_types: vec!["image/png".to_owned()],
        }];
        let identifiers: Vec<String> = content_types(&types)
            .iter()
            .map(|kind| kind.identifier().to_string())
            .collect();
        assert_eq!(identifiers[0], "public.plain-text");
        assert_eq!(identifiers[2], "public.png");
        assert_eq!(identifiers.len(), 3, "{identifiers:?}");
    }

    #[test]
    fn urls_become_their_paths() {
        let url = NSURL::from_file_path("/tmp/My Notes/café.txt").expect("a file URL");
        let urls = NSArray::from_retained_slice(&[url]);
        assert_eq!(paths(&urls), [PathBuf::from("/tmp/My Notes/café.txt")]);
    }
}
