use std::path::PathBuf;

use color_eyre::eyre::{eyre, Context};
use serde::Deserialize;

use crate::{
    markdown::{new_md_parser, FrontmatterExtractor},
    FsOperation, ProcessFile, ReadFn,
};

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
    ) -> color_eyre::Result<Vec<FsOperation>> {
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

        Ok(vec![FsOperation::Write {
            path: file_path.with_extension("html"),
            contents: html_buf.into(),
        }])
    }
}
