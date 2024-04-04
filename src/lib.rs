use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    ffi::OsStr,
    fs::File,
    path::PathBuf,
};

use color_eyre::eyre::{eyre, Context as EyreContext};
use frontmatter_extension::FrontmatterExtractor;
use pulldown_cmark::{Options, Parser};
use serde::Deserialize;

pub mod frontmatter_extension;

// TODO: maybe use a pure path library like typed-path here

#[derive(Clone, Debug, PartialEq)]
pub enum OutputAction {
    Copy,
    Output(Vec<u8>),
    // parent directories should be made if necessary (mkdir -p)
    OutputOther {
        file_path: PathBuf,
        contents: Vec<u8>,
    },
    // no way to output an empty directory but I don't think that will be needed
}

pub type ReadFn = Box<dyn FnOnce() -> color_eyre::Result<Vec<u8>>>;

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
    ) -> color_eyre::Result<Vec<OutputAction>>;
}

#[derive(Clone, Copy, Debug)]
pub struct CopyHandler;

impl ProcessFile for CopyHandler {
    fn process_file(
        &self,
        _file_path: &PathBuf,
        _read: ReadFn,
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
    fn process_file<'a>(
        &self,
        file_path: &PathBuf,
        read: ReadFn,
    ) -> color_eyre::Result<Vec<OutputAction>> {
        let input = String::from_utf8(read()?).wrap_err_with(|| {
            eyre!(
                "markdown file {} contains invalid utf-8",
                file_path.display()
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
        read: ReadFn,
    ) -> color_eyre::Result<Vec<OutputAction>> {
        let input = String::from_utf8(read()?)
            .wrap_err_with(|| eyre!("post file {} contains invalid utf-8", file_path.display()))?;
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
                file_path.display()
            )
        })?;
        let frontmatter: PostFrontmatter =
            toml::from_str(&frontmatter_str).wrap_err_with(|| {
                eyre!(
                    "expected frontmatter of post file {} to parse as TOML",
                    file_path.display()
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

fn to_fs_operation(file_path: &PathBuf, action: OutputAction) -> Option<FsOperation> {
    match action {
        OutputAction::Copy => Some(FsOperation::Copy {
            from: file_path.clone(),
            to: file_path.clone(),
        }),
        OutputAction::Output(contents) => Some(FsOperation::Write {
            path: file_path.clone(),
            contents,
        }),
        OutputAction::OutputOther {
            file_path: output_path,
            contents,
        } => Some(FsOperation::Write {
            path: output_path,
            contents,
        }),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlogContext {
    pub posts_dir: PathBuf,
    // output file => input file
    pub written_files: HashMap<PathBuf, PathBuf>,
}

impl BlogContext {
    pub fn from_default_settings() -> Self {
        Self {
            posts_dir: "posts".into(),
            written_files: HashMap::new(),
        }
    }

    fn get_handler(&self, file_path: &PathBuf) -> FileHandler {
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

    pub fn process_file(
        &mut self,
        file_path: &PathBuf,
        read: ReadFn,
    ) -> color_eyre::Result<Vec<FsOperation>> {
        let handler = self.get_handler(file_path);
        let actions = handler.process_file(file_path, read)?;
        let operations = actions
            .into_iter()
            .filter_map(|action| to_fs_operation(file_path, action))
            .collect::<Vec<_>>();
        for op in operations.iter() {
            match self.written_files.entry(op.output_path().clone()) {
                Entry::Occupied(e) => {
                    let output_path = e.key();
                    let prev_path = e.get();
                    return Err(eyre!(
                        "both {} and {} want to write to output path {}",
                        file_path.display(),
                        prev_path.display(),
                        output_path.display()
                    ));
                }
                Entry::Vacant(e) => {
                    e.insert(file_path.clone());
                }
            }
        }
        Ok(operations)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn duplicate_check() {
        let mut ctx = BlogContext::from_default_settings();
        assert_eq!(
            ctx.process_file(&"a.html".to_owned().into(), Box::new(|| unreachable!()))
                .map_err(|_| ()),
            Ok(vec![FsOperation::Copy {
                from: "a.html".to_owned().into(),
                to: "a.html".to_owned().into()
            }])
        );
        assert_eq!(
            ctx.process_file(&"a.md".to_owned().into(), Box::new(|| Ok("# a".into())))
                .map_err(|e| format!("{}", e)),
            Err("both a.md and a.html want to write to output path a.html".to_owned())
        );
    }
}
