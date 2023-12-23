use std::{ffi::OsStr, path::PathBuf};

use color_eyre::eyre::{eyre, Context};
use frontmatter_extension::FrontmatterExtractor;
use pulldown_cmark::{Options, Parser};
use serde::Deserialize;

mod frontmatter_extension;

fn new_md_parser<'a, 'callback>(input: &'a str) -> Parser<'a, 'callback> {
    let mut md_options = Options::empty();
    md_options.insert(Options::ENABLE_STRIKETHROUGH);
    Parser::new_ext(input.as_ref(), md_options)
}

fn render_md(input: String) -> color_eyre::Result<String> {
    let parser = new_md_parser(&input);
    let mut html_buf = String::new();
    pulldown_cmark::html::push_html(&mut html_buf, parser);
    Ok(html_buf)
}

#[derive(Debug, Deserialize)]
struct PostFrontmatter {
    title: String,
}

fn render_post(file_path: &PathBuf, input: String) -> color_eyre::Result<String> {
    let parser = new_md_parser(&input);
    let mut parser = FrontmatterExtractor::new(parser);
    let mut html_buf = String::new();
    pulldown_cmark::html::push_html(&mut html_buf, &mut parser);
    let frontmatter_str = parser.frontmatter_str().ok_or_else(|| {
        eyre!(
            "Did not recognize any frontmatter in post {}",
            file_path.to_string_lossy()
        )
    })?;
    let frontmatter: PostFrontmatter = toml::from_str(&frontmatter_str).wrap_err_with(|| {
        format!(
            "Parsing frontmatter of {} failed",
            file_path.to_string_lossy()
        )
    })?;
    println!("frontmatter: {:#?}", frontmatter);
    Ok(html_buf)
}

fn is_post(input_dir: &PathBuf, file_path: &PathBuf) -> bool {
    let posts_dir = input_dir.join("posts");
    file_path.starts_with(posts_dir)
}

fn process_md_file(
    input_dir: &PathBuf,
    file_path: &PathBuf,
    output_path: &PathBuf,
) -> color_eyre::Result<()> {
    let file_bytes = std::fs::read(file_path.clone())
        .wrap_err_with(|| format!("reading file {} failed", file_path.to_string_lossy()))?;
    let text = String::from_utf8(file_bytes).wrap_err_with(|| {
        format!(
            "file {} contains invalid UTF-8",
            file_path.to_string_lossy()
        )
    })?;
    let html_buf = if is_post(input_dir, file_path) {
        render_post(file_path, text)?
    } else {
        render_md(text)?
    };
    let output_path = output_path.with_extension("html");
    // TODO: check if we are overwriting a non-md HTML file
    std::fs::write(output_path.clone(), html_buf)
        .wrap_err_with(|| format!("writing file {} failed", output_path.to_string_lossy()))?;
    Ok(())
}

fn process_file(
    input_dir: &PathBuf,
    output_dir: &PathBuf,
    file_path: &PathBuf,
) -> color_eyre::Result<()> {
    let relative_path = file_path.strip_prefix(input_dir.clone()).unwrap();
    let output_path = output_dir.join(relative_path);
    let relative_path_str = relative_path.to_string_lossy();

    let output_parent = output_path
        .parent()
        .expect("path created by join will have a parent");
    std::fs::create_dir_all(output_parent).wrap_err_with(|| {
        format!(
            "Creating directory {} failed",
            output_parent.to_string_lossy()
        )
    })?;

    let ext = file_path.extension().and_then(OsStr::to_str);
    if let Some("md") = ext {
        println!("compiling markdown file {}", relative_path_str);
        return process_md_file(input_dir, file_path, &output_path);
    }

    println!("copying file {}", relative_path_str);

    std::fs::copy(file_path.clone(), output_path.clone()).wrap_err_with(|| {
        format!(
            "copying {} to {} failed",
            file_path.to_string_lossy(),
            output_path.to_string_lossy()
        )
    })?;

    Ok(())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Usage: {} <input directory> <output directory>",
            args.into_iter().nth(0).unwrap_or_default(),
        );
        std::process::exit(1);
    }
    let input_dir = std::path::PathBuf::from(args.get(1).unwrap());
    let output_dir = std::path::PathBuf::from(args.get(2).unwrap());

    let mut dirs = vec![input_dir.clone()];
    while !dirs.is_empty() {
        let dir = dirs.remove(0);
        for entry in dir
            .clone()
            .read_dir()
            .wrap_err_with(|| format!("reading directory {} failed", dir.to_string_lossy()))?
        {
            let entry = entry
                .wrap_err_with(|| format!("reading directory {} failed", dir.to_string_lossy()))?;
            let metadata = entry.metadata().wrap_err_with(|| {
                format!("reading metadata of {}", entry.path().to_string_lossy())
            })?;

            let file_path = match metadata {
                metadata if metadata.is_file() => entry.path(),
                metadata if metadata.is_dir() => {
                    dirs.push(entry.path());
                    continue;
                }
                _ => {
                    return Err(eyre!(
                        "cannot process directory entry: {}",
                        entry.path().to_string_lossy()
                    ));
                }
            };

            process_file(&input_dir, &output_dir, &file_path)?;
        }
    }

    Ok(())
}
