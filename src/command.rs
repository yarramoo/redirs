use std::io::{self, Read, Write};

use dashmap::DashMap;
use thiserror::Error;

use crate::message::{serialise_message, Message};
type DB = DashMap<Vec<u8>, Vec<u8>>;

const PONG: &[u8] = b"PONG";
const OK: &[u8] = b"OK";

pub(crate) enum Command<'a> {
    PING,
    ECHO(&'a [u8]),
    SET(&'a [u8], &'a [u8]),
    GET(&'a [u8]),
}

#[derive(Debug, Error)]
pub(crate) enum CommandParseError {
    #[error("The message format is invalid: {0}")]
    InvalidMessageFormat(String),

    #[error("Unkown command: {0}")]
    InvalidCommand(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
}

pub(crate) fn parse_command<'a>(message: &Message<'a>) -> Result<Command<'a>, CommandParseError> {
    // A lot of error handling to do here...
    let messages = message
        .as_array()
        .ok_or(CommandParseError::InvalidMessageFormat(message.to_string()))?;

    let command = messages
        .first()
        .ok_or(CommandParseError::InvalidMessageFormat(message.to_string()))?
        .as_bulk_string()
        .ok_or(CommandParseError::InvalidCommand(message.to_string()))?
        .to_vec()
        .to_ascii_lowercase();

    let arguments = &messages[1..];
    match command.as_slice() {
        b"ping" => parse_ping(arguments),
        b"echo" => parse_echo(arguments),
        b"set" => parse_set(arguments),
        b"get" => parse_get(arguments),
        unknown_cmd => Err(CommandParseError::InvalidCommand(String::from_utf8_lossy(&unknown_cmd).to_string())),
    }
}

macro_rules! check_arg_len {
    ($args:expr, $expected_num:expr, $cmd_name:expr) => {
        if $args.len() != $expected_num {
            return Err(CommandParseError::InvalidArguments(
                format!("Wrong number of arguments for the {} command", $cmd_name)
            ));
        }
    };
}

macro_rules! unwrap_bulk_string {
    ($message:expr) => {
        $message.as_bulk_string().ok_or(CommandParseError::InvalidArguments(
            "Argument not a BulkString".to_string()
        ))
    };
}

fn parse_ping<'a>(arguments: &[Message<'a>]) -> Result<Command<'a>, CommandParseError> {
    check_arg_len!(arguments, 0, "ECHO");
    Ok(Command::PING)
}

fn parse_echo<'a>(arguments: &[Message<'a>]) -> Result<Command<'a>, CommandParseError> {
    check_arg_len!(arguments, 1, "ECHO");
    let echo_string = unwrap_bulk_string!(&arguments[0])?;
    Ok(Command::ECHO(echo_string))
}

fn parse_set<'a>(arguments: &[Message<'a>]) -> Result<Command<'a>, CommandParseError> {
    check_arg_len!(arguments, 2, "SET");
    let key = unwrap_bulk_string!(&arguments[0])?;
    let value = unwrap_bulk_string!(&arguments[1])?;
    Ok(Command::SET(key, value))
}

fn parse_get<'a>(arguments: &[Message<'a>]) -> Result<Command<'a>, CommandParseError> {
    check_arg_len!(arguments, 1, "GET");
    let key = unwrap_bulk_string!(&arguments[0])?;
    Ok(Command::GET(key))
}

pub(crate) fn handle_command<W: Write>(command: &Command, db: &mut DB, writer: &mut W) -> io::Result<()>
{   
    match command {
        Command::PING => serialise_message(&Message::BulkString(Some(PONG)), writer),
        Command::ECHO(string) => serialise_message(&Message::BulkString(Some(string)), writer),
        Command::SET(key, value) => {
            db.insert((*key).into(), (*value).into());
            serialise_message(&Message::BulkString(Some(OK)), writer)
        },
        Command::GET(key) => {
            match db.get::<[u8]>(key) {
                Some(value) => 
                    serialise_message(&Message::BulkString(Some(value.as_ref())), writer),
                None => serialise_message(&Message::BulkString(None), writer),
            }
        }
    }
}