use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("$insert payload must be an array of [position, value] pairs")]
    MalformedInsert,
    #[error("$delete payload must be an array")]
    MalformedDelete,
    #[error("$add/$discard payload must be an array")]
    MalformedSet,
    #[error("position {0} is not representable as usize")]
    PositionOutOfRange(i64),
}

#[derive(thiserror::Error, Debug)]
pub enum PatchError {
    #[error("list patch key {0:?} is not a valid usize position")]
    InvalidListPosition(Arc<str>),
    #[error("list index {0} out of range for list of length {1}")]
    ListIndexOutOfRange(usize, usize),
    #[error("delete key {0:?} not present in object")]
    DictDeleteMissingKey(Arc<str>),
    #[error("$delete element on a dict source must be a string key")]
    DictDeleteNotString,
    #[error("$delete element on a list source must be a usize position")]
    ListDeleteNotInteger,
    #[error("scalar source cannot be patched with an object diff")]
    ScalarSourceWithObjectDiff,
    #[error("set source is not supported in this implementation")]
    SetSourceUnsupported,
}
