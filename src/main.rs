
mod message;
mod server;
use dashmap::DashMap;
use server::{listen, handle_client};
mod command;

const DEFAULT_PORT: &str = "6379";



fn main() {
    // let db: HashmapDB = HashmapDB::new();
    let db = DashMap::new();
    let _ = listen("127.0.0.1", DEFAULT_PORT, handle_client, db);
}
