use async_graphql::ID;
use blob_uuid::ConvertError;
use std::convert::From;
use uuid::Uuid;

use super::cursor;
use super::cursor::CursorError;

#[derive(Debug, PartialEq)]
pub enum UuidError {
    Cusor(CursorError),
    Convert,
}

impl From<CursorError> for UuidError {
    fn from(e: CursorError) -> UuidError {
        UuidError::Cusor(e)
    }
}

impl From<ConvertError> for UuidError {
    fn from(_: ConvertError) -> UuidError {
        UuidError::Convert
    }
}

pub type UuidResult<T> = Result<T, UuidError>;

pub fn to_id(type_name: &str, id: &Uuid) -> ID {
    let id = blob_uuid::to_blob(id);
    ID::from(cursor::to_cursor(type_name, &id))
}

pub fn from_id(id: &ID) -> UuidResult<(String, Uuid)> {
    let (type_name, id) = cursor::from_cursor(id.as_str())?;
    let id = blob_uuid::to_uuid(&id)?;

    Ok((type_name, id))
}
