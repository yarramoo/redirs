use dashmap::DashMap;
use thiserror::Error;

use crate::message::Message;
type DB = DashMap<Vec<u8>, Vec<u8>>;

pub(crate) enum Command<'a> {
    PING,
    ECHO(&'a str),
    SET(&'a str, &'a str),
    GET(&'a str),
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

pub(crate) fn parse_command(message: &Message) -> Result<Command, CommandParseError> {
    // A lot of error handling to do here...
    let messages = message
        .as_array()
        .ok_or(CommandParseError::InvalidMessageFormat(message.to_string()))?;

    let command = messages
        .first()
        .ok_or(CommandParseError::InvalidMessageFormat(message.to_string()))?
        .as_bulk_string()
        .ok_or(CommandParseError::InvalidCommand(message.to_string()))?;

    let arguments = &messages[1..];
    match command.to_lowercase().as_str() {
        "ping" => parse_ping(arguments),
        "echo" => parse_echo(arguments),
        "set" => parse_set(arguments),
        "get" => parse_get(arguments),
        unknown_cmd => Err(CommandParseError::InvalidCommand(unknown_cmd.to_string())),
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

fn parse_ping(arguments: &[Message]) -> Result<Command, CommandParseError> {
    check_arg_len!(arguments, 0, "ECHO");
    Ok(Command::PING)
}

fn parse_echo(arguments: &[Message]) -> Result<Command, CommandParseError> {
    check_arg_len!(arguments, 1, "ECHO");
    let echo_string = unwrap_bulk_string!(&arguments[0])?;
    Ok(Command::ECHO(echo_string))
}

fn parse_set(arguments: &[Message]) -> Result<Command, CommandParseError> {
    check_arg_len!(arguments, 2, "SET");
    let key = unwrap_bulk_string!(&arguments[0])?;
    let value = unwrap_bulk_string!(&arguments[1])?;
    Ok(Command::SET(key, value))
}

fn parse_get(arguments: &[Message]) -> Result<Command, CommandParseError> {
    check_arg_len!(arguments, 1, "GET");
    let key = unwrap_bulk_string!(&arguments[0])?;
    Ok(Command::GET(key))
}

pub(crate) fn handle_command(command: &Command, db: &mut DB) -> Message {
    match command {
        Command::PING => Message::BulkString(Some("PONG".to_string())),
        Command::ECHO(string) => Message::BulkString(Some(string.to_string())),
        Command::SET(key, value) => {
            db.insert((*key).into(), (*value).into());
            Message::BulkString(Some("OK".to_string()))
        },
        Command::GET(key) => {
            match db.get::<[u8]>(key.as_bytes()) {
                Some(value) => Message::BulkString(Some(String::from_utf8_lossy(&value).into())),
                None => Message::BulkString(None),
            }
        }
    }
}