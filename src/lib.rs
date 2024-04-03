use std::{ffi::OsStr, fs::File, path::PathBuf};

use pulldown_cmark::{Options, Parser};

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
#[enum_dispatch::enum_dispatch]
pub trait ProcessFile<E> {
    // caller should remember filename if needed to read()
    fn process_file(
        &self,
        file_path: &PathBuf,
        read: Box<dyn FnOnce() -> Result<Vec<u8>, E>>,
    ) -> Result<Vec<OutputAction>, E>;
}

#[derive(Clone, Copy, Debug)]
pub struct CopyHandler;

impl<E> ProcessFile<E> for CopyHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: Box<dyn FnOnce() -> Result<Vec<u8>, E>>,
    ) -> Result<Vec<OutputAction>, E> {
        Ok(vec![OutputAction::Copy])
    }
}

fn new_md_parser<'a, 'callback>(input: &'a str) -> Parser<'a, 'callback> {
    let mut md_options = Options::empty();
    md_options.insert(Options::ENABLE_STRIKETHROUGH);
    Parser::new_ext(input.as_ref(), md_options)
}

#[derive(Clone, Copy, Debug)]
pub struct PostHandler;

impl<E> ProcessFile<E> for PostHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: Box<dyn FnOnce() -> Result<Vec<u8>, E>>,
    ) -> Result<Vec<OutputAction>, E> {
        todo!()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GenericMarkdownHandler;

impl<E> ProcessFile<E> for GenericMarkdownHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: Box<dyn FnOnce() -> Result<Vec<u8>, E>>,
    ) -> Result<Vec<OutputAction>, E> {
        todo!()
    }
}

// Copy for now, might delete later
#[enum_dispatch::enum_dispatch(HandleFile)]
#[derive(Clone, Copy, Debug)]
pub enum FileHandler {
    CopyHandler,
    PostHandler,
    GenericMarkdownHandler,
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

    pub fn get_handler<E>(&self, file_path: &PathBuf) -> FileHandler {
        // None can mean either no extension or a garbage extension.
        let ext = file_path.extension().and_then(OsStr::to_str);

        match ext {
            Some("md") if file_path.starts_with(&self.posts_dir) => {
                FileHandler::PostHandler(PostHandler)
            }
            Some("md") => FileHandler::GenericMarkdownHandler(GenericMarkdownHandler),

            // if we don't recognize the file, copy it over as is.
            _ => FileHandler::CopyHandler(CopyHandler),
        }
    }
}
