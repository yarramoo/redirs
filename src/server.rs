use std::io::{self, Write, Read};
use std::net::{TcpListener, TcpStream};
use std::thread;

use dashmap::DashMap;
use nom::AsBytes;

use crate::command::{handle_command, parse_command};
use crate::message::{Message, parse_message, serialise_message};

const BUFFER_SIZE: usize = 1024;

type DB = DashMap<Vec<u8>, Vec<u8>>;

pub fn listen<F>(
    ip: &str, 
    port: &str, 
    handle_client: F, 
    db: DB
) -> io::Result<()> 
where
    F: Fn(TcpStream, DB) + Send + Copy + 'static,
{
    let listener = TcpListener::bind(format!("{}:{}", &ip, &port))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let db_clone = db.clone();
                thread::spawn(move || { // basic mutlithreaded solution. Maybe do a threadpool
                    handle_client(stream, db_clone);
                });
            },
            Err(e) => {
                eprintln!("Failed to accept client {}", e);
            }
        }
    }

    Ok(())
}

pub fn handle_client(mut stream: TcpStream, mut db: DB) 
where
{
    let mut buffer = [0; BUFFER_SIZE];
    loop {
        // println!("{:?}", String::from_utf8_lossy(buffer.as_slice()));
        match stream.read(&mut buffer) {
            Ok(0) => {
                // client disconnected
                break;
            },
            Ok(_) => {
                let message = parse_message(&buffer[..]);
                // println!("{:?}", message);
                if let Ok((_, message)) = message {
                    handle_message(&message, &mut stream, &mut db);
                    buffer.fill(0);
                }
            }
            Err(_) => todo!(),
        }
    }
}

fn handle_message(message: &Message, stream: &mut TcpStream, db: &mut DB) 
{
    let cmd = parse_command(message).unwrap();
    let response_message = handle_command(&cmd, db);
    let response_serialised = serialise_message(&response_message);
    let _ = stream.write_all(response_serialised.as_bytes());
    // println!("{:?}", response_message);
    // println!("{:?}", String::from_utf8_lossy(response_serialised.as_bytes()));
}