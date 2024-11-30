use super::Message;

const CRLF: &[u8; 2] = b"\r\n";

pub(crate) fn serialise_message(message: &Message) -> Vec<u8> {
    match message {
        Message::SimpleString(string) => serialise_simple_string(string),
        Message::Error(error) => serialise_error(error),
        Message::Integer(n) => serialise_integer(*n),
        Message::BulkString(string) => serialise_bulk_string(string),
        Message::Array(array) => serialise_array(array),
        Message::Null => serialise_null(),
        Message::Bool(b) => serialise_bool(*b),
        Message::Double(n) => serialise_double(*n),
    }
}

fn serialise_simple_string(string: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(string.len() + 3);
    buf.push(b'+');
    buf.extend_from_slice(string.as_bytes());
    buf.extend_from_slice(CRLF);
    buf
}

fn serialise_error(error: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(error.len() + 3);
    buf.push(b'+');
    buf.extend_from_slice(error.as_bytes());
    buf.extend_from_slice(CRLF);
    buf
}

fn serialise_integer(n: isize) -> Vec<u8> {
    let n_str = n.to_string();
    let mut buf = Vec::with_capacity(n_str.len() + 3);
    buf.push(b':');
    buf.extend_from_slice(n_str.as_bytes());
    buf.extend_from_slice(CRLF);
    buf
}

fn serialise_bulk_string(string: &Option<String>) -> Vec<u8> {
    if let Some(ref string) = string {
        let len = string.len();
        let len_str = len.to_string();
        let mut buf = Vec::with_capacity(len + len_str.len() + 5);
        buf.push(b'$');
        buf.extend_from_slice(len_str.as_bytes());
        buf.extend_from_slice(CRLF);
        buf.extend_from_slice(string.as_bytes());
        buf.extend_from_slice(CRLF);
        buf
    } else {
        "$-1\r\n".into()
    }
}

fn serialise_null() -> Vec<u8> {
    b"_\r\n".into()
}

fn serialise_bool(b: bool) -> Vec<u8> {
    if b {
        "#t\r\n".into()
    } else {
        "#f\r\n".into()
    }
}

fn serialise_double(n: f64) -> Vec<u8> {
    let n_str = n.to_string();
    let mut buf = Vec::with_capacity(n_str.len() + 3);
    buf.push(b',');
    buf.extend_from_slice(n_str.as_bytes());
    buf.extend_from_slice(CRLF);
    buf
}

fn serialise_array(array: &Option<Vec<Message>>) -> Vec<u8> {
    if let Some(ref array) = array {
        let mut buf = Vec::new();
        let len_str = array.len().to_string();
        buf.push(b'*');
        buf.extend_from_slice(len_str.as_bytes());
        buf.extend_from_slice(CRLF);
        for message in array {
            buf.extend_from_slice(&serialise_message(message));
        }
        buf
    } else {
        "*-1\r\n".into()
    }
}