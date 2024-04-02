use std::{ffi::OsStr, path::PathBuf};

pub mod frontmatter_extension;

// TODO: maybe use a pure path library like typed-path here

#[derive(Clone, Debug, PartialEq)]
pub enum OutputAction {
    None,
    Copy,
    Output(String),
    // parent directories should be made if necessary (mkdir -p)
    OutputOther {
        file_path: PathBuf,
        contents: Vec<u8>,
    },
    // no way to output an empty directory but I don't think that will be needed
}

// this *could* be a single function pointer instead of a trait implemented on an
//  empty struct, but this gives room to add state to the handlers in the future
pub trait FileHandler {
    // caller should remember filename if needed to read()
    fn process_file(
        &self,
        file_path: &PathBuf,
        read: Box<dyn FnOnce() -> Vec<u8>>,
    ) -> Vec<OutputAction>;
}

#[derive(Clone, Copy, Debug)]
struct CopyHandler;

impl FileHandler for CopyHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: Box<dyn FnOnce() -> Vec<u8>>,
    ) -> Vec<OutputAction> {
        vec![OutputAction::Copy]
    }
}

#[derive(Clone, Copy, Debug)]
struct PostHandler;

impl FileHandler for PostHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: Box<dyn FnOnce() -> Vec<u8>>,
    ) -> Vec<OutputAction> {
        todo!()
    }
}

#[derive(Clone, Copy, Debug)]
struct GenericMarkdownHandler;

impl FileHandler for GenericMarkdownHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: Box<dyn FnOnce() -> Vec<u8>>,
    ) -> Vec<OutputAction> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    posts_dir: PathBuf,
}

impl Context {
    pub fn from_default_settings() -> Self {
        Self {
            posts_dir: "posts".into(),
        }
    }

    pub fn get_handler(&self, file_path: &PathBuf) -> Box<dyn FileHandler> {
        // None can mean either no extension or a garbage extension.
        let ext = file_path.extension().and_then(OsStr::to_str);

        match ext {
            Some("md") if file_path.starts_with(&self.posts_dir) => Box::new(PostHandler),
            Some("md") => Box::new(GenericMarkdownHandler),

            // if we don't recognize the file, copy it over as is.
            _ => Box::new(CopyHandler),
        }
    }
}
