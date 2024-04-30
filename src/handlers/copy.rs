use std::path::PathBuf;

use crate::{FsOperation, ProcessFile, ReadFn};

#[derive(Clone, Copy, Debug)]
pub struct CopyHandler;

impl ProcessFile for CopyHandler {
    fn process_file(
        &self,
        file_path: &PathBuf,
        _read: ReadFn,
    ) -> color_eyre::Result<Vec<FsOperation>> {
        Ok(vec![FsOperation::Copy {
            from: file_path.to_owned(),
            to: file_path.to_owned(),
        }])
    }
}
