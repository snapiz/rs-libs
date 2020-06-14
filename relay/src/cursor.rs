use base64::DecodeError;
use std::convert::From;
use std::string::FromUtf8Error;

#[derive(Debug, PartialEq)]
pub enum CursorError {
    FromUtf8,
    Decoded(DecodeError),
    InvalidFormat,
}

impl From<DecodeError> for CursorError {
    fn from(e: DecodeError) -> CursorError {
        CursorError::Decoded(e)
    }
}

impl From<FromUtf8Error> for CursorError {
    fn from(_: FromUtf8Error) -> CursorError {
        CursorError::FromUtf8
    }
}

pub type CursorResult<T> = Result<T, CursorError>;

pub fn to_cursor(key: &str, value: &str) -> String {
    base64::encode(format!("{}:{}", key, value))
}

pub fn from_cursor(cursor: &str) -> CursorResult<(String, String)> {
    let cursor = base64::decode(cursor)?;
    let cursor = String::from_utf8(cursor)?;
    let data = cursor.splitn(2, ':').collect::<Vec<_>>();

    match data.len() {
        2 => Ok((data[0].to_owned(), data[1].to_owned())),
        _ => Err(CursorError::InvalidFormat),
    }
}

#[cfg(test)]
mod tests {
    use super::CursorError;

    #[test]
    fn to_from_cursor_succes() {
        assert_eq!(
            super::from_cursor(&super::to_cursor("Tim", "ada")),
            Ok(("Tim".to_owned(), "ada".to_owned()))
        );
    }

    #[test]
    fn from_cursor_invalid_format() {
        assert_eq!(
            super::from_cursor("MV9lZmVm"),
            Err(CursorError::InvalidFormat)
        );
    }

    #[test]
    fn from_cursor_success() {
        assert_eq!(
            super::from_cursor("VXNlcjox"),
            Ok(("User".to_owned(), "1".to_owned()))
        );
    }

    #[test]
    fn from_cursor_success_multiple_separator() {
        assert_eq!(
            super::from_cursor("MToyMDIwLTAxLTAxVDEzOjA0OjAwWg=="),
            Ok(("1".to_owned(), "2020-01-01T13:04:00Z".to_owned()))
        );
    }
}
