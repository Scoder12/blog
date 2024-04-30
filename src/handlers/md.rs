use std::path::PathBuf;

use color_eyre::eyre::{eyre, Context};

use crate::{markdown::new_md_parser, FsOperation, ProcessFile, ReadFn};

#[derive(Clone, Copy, Debug)]
pub struct GenericMarkdownHandler;

impl ProcessFile for GenericMarkdownHandler {
    fn process_file<'a>(
        &self,
        file_path: &PathBuf,
        read: ReadFn,
    ) -> color_eyre::Result<Vec<FsOperation>> {
        let input = String::from_utf8(read()?).wrap_err_with(|| {
            eyre!(
                "markdown file {} contains invalid utf-8",
                file_path.display()
            )
        })?;
        let parser = new_md_parser(&input);
        let mut html_buf = String::new();
        pulldown_cmark::html::push_html(&mut html_buf, parser);
        Ok(vec![FsOperation::Write {
            path: file_path.with_extension("html"),
            contents: html_buf.into(),
        }])
    }
}
