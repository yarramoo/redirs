use std::io::{self, Write};

use super::serialise_message;

#[derive(Debug, PartialEq)]
pub(crate) enum Message<'a> {
    SimpleString(&'a [u8]),
    Error(&'a [u8]),
    Integer(isize),
    BulkString(Option<&'a [u8]>),
    Array(Option<Vec<Message<'a>>>),
    Null,
    Bool(bool),
    Double(f64),
}

impl<'a> Message<'a> {
    pub fn serialise<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        serialise_message(self, writer)
    }

    pub fn to_string(&self) -> String {
        let mut buf: Vec<u8> = Vec::new(); 
        self.serialise(&mut buf).unwrap();
        String::from_utf8_lossy(buf.as_slice()).into_owned()
    }

    pub fn as_bulk_string(&self) -> Option<&'a [u8]> {
        if let Self::BulkString(Some(string)) = self {
            Some(string)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&[Message<'a>]> {
        if let Self::Array(Some(ref messages)) = self {
            Some(messages)
        } else {
            None
        }
    }
}