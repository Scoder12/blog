use std::ffi::OsStr;

use color_eyre::eyre::{eyre, Context};
use pulldown_cmark::{Options, Parser};

fn render_md(input: String) -> color_eyre::Result<String> {
    let mut md_options = Options::empty();
    md_options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&input, md_options);
    let mut html_buf = String::new();
    pulldown_cmark::html::push_html(&mut html_buf, parser);
    Ok(html_buf)
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

            let relative_path = file_path.strip_prefix(input_dir.clone()).unwrap();
            let output_path = output_dir.join(relative_path);
            let relative_path_str = relative_path.to_string_lossy();
            match file_path.extension().and_then(OsStr::to_str) {
                Some("md") => {
                    println!("compiling markdown file {}", relative_path_str);

                    let file_bytes = std::fs::read(file_path.clone()).wrap_err_with(|| {
                        format!("reading file {} failed", file_path.to_string_lossy())
                    })?;
                    let text = String::from_utf8(file_bytes).wrap_err_with(|| {
                        format!(
                            "file {} contains invalid UTF-8",
                            file_path.to_string_lossy()
                        )
                    })?;
                    let html_buf = render_md(text)?;
                    let output_path = output_path.with_extension("html");
                    std::fs::write(output_path.clone(), html_buf).wrap_err_with(|| {
                        format!("writing file {} failed", output_path.to_string_lossy())
                    })?;
                    continue;
                }
                _ => {
                    println!("copying file {}", relative_path_str);

                    std::fs::copy(file_path.clone(), output_path.clone()).wrap_err_with(|| {
                        format!(
                            "copying {} to {} failed",
                            file_path.to_string_lossy(),
                            output_path.to_string_lossy()
                        )
                    })?;
                }
            }
        }
    }

    Ok(())
}
