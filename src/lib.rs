use std::path::PathBuf;

pub mod ctx;
pub mod handlers;
pub mod markdown;

// TODO: maybe use a pure path library like typed-path here

#[derive(Clone, Debug, PartialEq)]
pub enum FsOperation {
    Copy { from: PathBuf, to: PathBuf },
    Write { path: PathBuf, contents: Vec<u8> },
}

impl FsOperation {
    fn output_path(&self) -> &PathBuf {
        match self {
            Self::Copy { from: _, to } => to,
            Self::Write { path, contents: _ } => path,
        }
    }
}

use handlers::copy::CopyHandler;
use handlers::md::GenericMarkdownHandler;
use handlers::post::PostHandler;

// Copy for now, might delete later
#[enum_dispatch::enum_dispatch(HandleFile)]
#[derive(Clone, Copy, Debug)]
pub enum FileHandler {
    CopyHandler,
    PostHandler,
    GenericMarkdownHandler,
}

// Passing a fallible read callback instead of the file contents allows us to skip
//  reading the file for no-op or copy only operations.
pub type ReadFn<'a> = Box<dyn FnOnce() -> color_eyre::Result<Vec<u8>> + 'a>;

// this *could* be a single function pointer instead of a trait implemented on an
//  empty struct, but this gives room to add state to the handlers in the future.
// We use eyre as the error trait here, because the library separation doesn't need to
//  be *that* complete
#[enum_dispatch::enum_dispatch(FileHandler)]
pub trait ProcessFile {
    // caller should remember filename if needed to read()
    fn process_file(
        &self,
        file_path: &PathBuf,
        read: ReadFn,
    ) -> color_eyre::Result<Vec<FsOperation>>;
}
