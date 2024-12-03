use std::io::{self, Write};

use super::Message;

const CRLF: &[u8; 2] = b"\r\n";

pub(crate) fn serialise_message<W: Write>(message: &Message, writer: &mut W) -> io::Result<()> {
    match message {
        Message::SimpleString(string) => serialise_simple_string(string, writer),
        Message::Error(error) => serialise_error(error, writer),
        Message::Integer(n) => serialise_integer(*n, writer),
        Message::BulkString(string) => serialise_bulk_string(string, writer),
        Message::Array(array) => serialise_array(array, writer),
        Message::Null => serialise_null(writer),
        Message::Bool(b) => serialise_bool(*b, writer),
        Message::Double(n) => serialise_double(*n, writer),
    }
}

fn serialise_simple_string<W: Write>(string: &[u8], writer: &mut W) -> io::Result<()> {
    writer.write_all(&[b'+'])?;
    writer.write_all(string)?;
    writer.write(CRLF)?;
    Ok(())
}

fn serialise_error<W: Write>(error: &[u8], writer: &mut W) -> io::Result<()> {
    writer.write_all(&[b'-'])?;
    writer.write_all(error)?;
    writer.write_all(CRLF)?;
    Ok(())
}

fn serialise_integer<W: Write>(n: isize, writer: &mut W) -> io::Result<()> {
    writer.write_all(&[b':'])?;
    writer.write_all(n.to_string().as_bytes())?;
    writer.write_all(CRLF)?;
    Ok(())
}

fn serialise_bulk_string<W: Write>(string: &Option<&[u8]>, writer: &mut W) -> io::Result<()> {
    if let Some(string) = string {
        let length = string.len();
        writer.write_all(&[b'$'])?;
        writer.write_all(length.to_string().as_bytes())?;
        writer.write_all(CRLF)?;
        writer.write_all(string)?;
        writer.write_all(CRLF)?;
    } else {
        writer.write( "$-1\r\n".as_bytes())?;
    }
    Ok(())
}

fn serialise_null<W: Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all("_\r\n".as_bytes())
}

fn serialise_bool<W: Write>(b: bool, writer: &mut W) -> io::Result<()> {
    if b {
        writer.write_all("#t\r\n".as_bytes())
    } else {
        writer.write_all("#f\r\n".as_bytes())
    }
}

fn serialise_double<W: Write>(n: f64, writer: &mut W) -> io::Result<()> {
    writer.write_all(",".as_bytes())?;
    writer.write_all(n.to_string().as_bytes())?;
    writer.write_all(CRLF)?;
    Ok(())
}

fn serialise_array<W: Write>(array: &Option<Vec<Message>>, writer: &mut W) -> io::Result<()> {
    if let Some(ref array) = array {
        let length = array.len();
        writer.write_all("*".as_bytes())?;
        writer.write_all(length.to_string().as_bytes())?;
        writer.write_all(CRLF)?;
        for message in array {
            serialise_message(message, writer)?;
        }
    } else {
        writer.write_all("*-1\r\n".as_bytes())?;
    }
    Ok(())
}