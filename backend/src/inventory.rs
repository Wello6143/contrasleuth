use crate::die_on_error::die_on_error;
use crate::message_hash::message_hash;
use async_std::sync::channel;
use async_std::task;
use rusqlite::params;
use serde::{Deserialize, Serialize};

type Pool = std::sync::Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>;

pub fn exists(pool: Pool, hash: &[u8]) -> bool {
    let connection = die_on_error(pool.get());
    let mut statement =
        die_on_error(connection.prepare(include_str!("../sql/B. RPC/2. Retrieve message.sql")));
    die_on_error(statement.exists(params![hash]))
}

pub fn insert(pool: Pool, payload: &[u8], nonce: i64, expiration_time: i64) {
    die_on_error(die_on_error(pool.get()).execute(
        include_str!("../sql/B. RPC/4. Purge expired messages.sql"),
        params![],
    ));
    die_on_error(die_on_error(pool.get()).execute(
        include_str!("../sql/B. RPC/3. Put message.sql"),
        params![
            message_hash(&payload, expiration_time).to_vec(),
            payload,
            nonce,
            expiration_time
        ],
    ));
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub payload: Vec<u8>,
    pub nonce: i64,
    pub expiration_time: i64,
}

pub fn retrieve(pool: Pool, hash: &[u8]) -> Option<Message> {
    let connection = die_on_error(pool.get());
    let mut statement =
        die_on_error(connection.prepare(include_str!("../sql/B. RPC/2. Retrieve message.sql")));
    let mut rows = die_on_error(statement.query(params![hash]));
    while let Some(row) = die_on_error(rows.next()) {
        let payload: Vec<u8> = die_on_error(row.get(0));
        let nonce: i64 = die_on_error(row.get(1));
        let expiration_time: i64 = die_on_error(row.get(2));
        return Some(Message {
            payload,
            nonce,
            expiration_time,
        });
    }
    None
}

pub fn hashes(pool: Pool) -> async_std::sync::Receiver<Vec<u8>> {
    let (tx, rx) = channel(1);
    task::spawn(async move {
        let connection = die_on_error(pool.get());
        let mut statement =
            die_on_error(connection.prepare(include_str!("../sql/B. RPC/1. Retrieve hashes.sql")));
        let mut rows = die_on_error(statement.query(params![]));
        let tx = std::sync::Arc::new(tx);
        while let Some(row) = die_on_error(rows.next()) {
            let stuff: Vec<u8> = die_on_error(row.get(0));
            let tx = tx.clone();
            task::spawn(async move {
                tx.send(stuff).await;
            });
        }
    });
    rx
}
