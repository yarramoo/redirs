use super::serialise_message;

#[derive(Debug, PartialEq)]
pub(crate) enum Message {
    SimpleString(String),
    Error(String),
    Integer(isize),
    BulkString(Option<String>),
    Array(Option<Vec<Message>>),
    Null,
    Bool(bool),
    Double(f64),
}

impl Message {
    pub fn serialise(&self) -> Vec<u8> {
        serialise_message(self)
    }

    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.serialise()).into()
    }

    pub fn as_bulk_string(&self) -> Option<&String> {
        if let Self::BulkString(Some(ref string)) = self {
            Some(string)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&[Message]> {
        if let Self::Array(Some(ref messages)) = self {
            Some(messages)
        } else {
            None
        }
    }
}