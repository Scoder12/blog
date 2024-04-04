use std::{ffi::OsStr, fs::File, path::PathBuf};

use color_eyre::eyre::{eyre, Context as EyreContext};
use frontmatter_extension::FrontmatterExtractor;
use pulldown_cmark::{Options, Parser};
use serde::Deserialize;

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
//  empty struct, but this gives room to add state to the handlers in the future.
// We use eyre as the error trait here, because the library separation doesn't need to
//  be *that* complete
#[enum_dispatch::enum_dispatch]
pub trait ProcessFile {
    // caller should remember filename if needed to read()
    fn process_file(
        &self,
        file_path: &PathBuf,
        read: Box<dyn FnOnce() -> color_eyre::Result<Vec<u8>>>,
    ) -> color_eyre::Result<Vec<OutputAction>>;
}

#[derive(Clone, Copy, Debug)]
pub struct CopyHandler;

impl ProcessFile for CopyHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: Box<dyn FnOnce() -> color_eyre::Result<Vec<u8>>>,
    ) -> color_eyre::Result<Vec<OutputAction>> {
        Ok(vec![OutputAction::Copy])
    }
}

fn new_md_parser<'a, 'callback>(input: &'a str) -> Parser<'a, 'callback> {
    let mut md_options = Options::empty();
    md_options.insert(Options::ENABLE_STRIKETHROUGH);
    Parser::new_ext(input.as_ref(), md_options)
}

#[derive(Clone, Copy, Debug)]
pub struct GenericMarkdownHandler;

impl ProcessFile for GenericMarkdownHandler {
    fn process_file(
        &self,
        file_path: &PathBuf,
        read: Box<dyn FnOnce() -> color_eyre::Result<Vec<u8>>>,
    ) -> color_eyre::Result<Vec<OutputAction>> {
        let input = String::from_utf8(read()?).wrap_err_with(|| {
            eyre!(
                "markdown file {} contains invalid utf-8",
                file_path.to_string_lossy()
            )
        })?;
        let parser = new_md_parser(&input);
        let mut html_buf = String::new();
        pulldown_cmark::html::push_html(&mut html_buf, parser);
        Ok(vec![OutputAction::OutputOther {
            file_path: file_path.with_extension("html"),
            contents: html_buf.into(),
        }])
    }
}

#[derive(Debug, Deserialize)]
struct PostFrontmatter {
    title: String,
}

#[derive(Clone, Copy, Debug)]
pub struct PostHandler;

impl PostHandler {
    const TOML_FRONTMATTER_DELIMITER: &'static str = "+++";
}

impl ProcessFile for PostHandler {
    fn process_file(
        &self,
        file_path: &PathBuf,
        read: Box<dyn FnOnce() -> color_eyre::Result<Vec<u8>>>,
    ) -> color_eyre::Result<Vec<OutputAction>> {
        let input = String::from_utf8(read()?).wrap_err_with(|| {
            eyre!(
                "post file {} contains invalid utf-8",
                file_path.to_string_lossy()
            )
        })?;
        let parser = new_md_parser(&input);
        let mut parser = FrontmatterExtractor::new_with_delimiter(
            parser,
            PostHandler::TOML_FRONTMATTER_DELIMITER,
        );
        let mut html_buf = String::new();
        pulldown_cmark::html::push_html(&mut html_buf, &mut parser);

        let frontmatter_str = parser.frontmatter_str().ok_or_else(|| {
            eyre!(
                "expected post file {} to contain frontmatter",
                file_path.to_string_lossy()
            )
        })?;
        let frontmatter: PostFrontmatter =
            toml::from_str(&frontmatter_str).wrap_err_with(|| {
                eyre!(
                    "expected frontmatter of post file {} to parse as TOML",
                    file_path.to_string_lossy()
                )
            })?;
        println!("frontmatter: {:#?}", frontmatter);

        Ok(vec![OutputAction::OutputOther {
            file_path: file_path.with_extension("html"),
            contents: html_buf.into(),
        }])
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
pub struct BlogContext {
    posts_dir: PathBuf,
}

impl BlogContext {
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
