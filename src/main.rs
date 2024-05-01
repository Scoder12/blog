use std::{ffi::OsStr, path::PathBuf};

use color_eyre::eyre::{eyre, Context};
use palm::{
    ctx::BlogContext,
    handlers::{copy::CopyHandler, md::GenericMarkdownHandler, post::PostHandler},
    FileHandler, FsOperation,
};

fn perform_fs_op(
    input_dir: &PathBuf,
    output_dir: &PathBuf,
    op: FsOperation,
) -> color_eyre::Result<()> {
    match op.clone() {
        FsOperation::Copy { from, to } => {
            let abs_from = input_dir.join(&from);
            let abs_to = output_dir.join(&to);
            let parent_dir = abs_to.parent().expect("output_dir should not be empty");
            std::fs::create_dir_all(parent_dir)
                .wrap_err_with(|| eyre!("creating directory {} failed", parent_dir.display()))?;

            println!("copy {} to {}", abs_from.display(), abs_to.display());
            Ok(std::fs::copy(&abs_from, &abs_to)
                .wrap_err_with(|| {
                    eyre!(
                        "copy from {} to {} failed",
                        abs_from.display(),
                        abs_to.display()
                    )
                })
                .map(|_| ())?)
        }
        FsOperation::Write { path, contents } => {
            let abs_path = output_dir.join(&path);
            let parent_dir = abs_path.parent().expect("output_dir should not be empty");
            std::fs::create_dir_all(parent_dir)
                .wrap_err_with(|| eyre!("creating directory {} failed", parent_dir.display()))?;
            std::fs::write(&abs_path, contents)
                .wrap_err_with(|| eyre!("writing to {} failed", abs_path.display()))
                .map_err(|e| e.into())
        }
    }
}

fn get_handler(file_path: &PathBuf) -> FileHandler {
    if file_path.starts_with("posts/") {
        return FileHandler::PostHandler(PostHandler);
    }

    let extension = file_path.extension().and_then(OsStr::to_str);
    match extension {
        Some("md") => FileHandler::GenericMarkdownHandler(GenericMarkdownHandler),
        _ => FileHandler::CopyHandler(CopyHandler),
    }
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

    let mut blog_ctx = BlogContext::builder()
        .with_get_handler(Box::new(get_handler))
        .build();
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

            let rel_path: PathBuf = file_path.strip_prefix(&input_dir)?.into();
            let ops = blog_ctx.process_file(
                &rel_path,
                Box::new(|| {
                    std::fs::read(&file_path)
                        .wrap_err_with(|| eyre!("failed to read {}", file_path.display()))
                }),
            )?;
            for op in ops.into_iter() {
                perform_fs_op(&input_dir, &output_dir, op)?;
            }
        }
    }

    Ok(())
}
