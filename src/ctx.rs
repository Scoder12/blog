use std::{
    collections::{hash_map::Entry, HashMap},
    ffi::OsStr,
    path::PathBuf,
};

use color_eyre::eyre::eyre;

use crate::{
    handlers::{copy::CopyHandler, md::GenericMarkdownHandler, post::PostHandler},
    FileHandler, FsOperation, ProcessFile, ReadFn,
};

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

    pub fn process_file<'a>(
        &mut self,
        file_path: &PathBuf,
        read: ReadFn<'a>,
    ) -> color_eyre::Result<Vec<FsOperation>> {
        let handler = self.get_handler(file_path);
        let operations = handler.process_file(file_path, read)?;
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
    use crate::FsOperation;

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
