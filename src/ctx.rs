use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
};

use color_eyre::eyre::eyre;

use crate::{handlers::copy::CopyHandler, FileHandler, FsOperation, ProcessFile, ReadFn};

pub struct BlogContext {
    // -- config --
    pub get_handler: Box<dyn Fn(&PathBuf) -> FileHandler>,
    // -- state --
    // output file => input file
    pub written_files: HashMap<PathBuf, PathBuf>,
}

impl BlogContext {
    pub fn builder() -> BlogContextBuilder {
        BlogContextBuilder::default()
    }

    pub fn process_file<'a>(
        &mut self,
        file_path: &PathBuf,
        read: ReadFn<'a>,
    ) -> color_eyre::Result<Vec<FsOperation>> {
        let handler = (self.get_handler)(file_path);
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

#[derive(Default)]
pub struct BlogContextBuilder {
    get_handler: Option<Box<dyn Fn(&PathBuf) -> FileHandler>>,
}

impl BlogContextBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_get_handler(self, get_handler: Box<dyn Fn(&PathBuf) -> FileHandler>) -> Self {
        Self {
            get_handler: Some(get_handler),
            ..self
        }
    }

    pub fn build(self) -> BlogContext {
        BlogContext {
            get_handler: self
                .get_handler
                .unwrap_or_else(|| Box::new(|_path| FileHandler::CopyHandler(CopyHandler))),
            // -- state --
            written_files: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::FsOperation;

    use super::*;

    #[test]
    fn duplicate_check() {
        let mut ctx = BlogContext::builder().build();
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
