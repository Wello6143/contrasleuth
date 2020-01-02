use crate::die_on_error::die_on_error;
use crate::message_hash::message_hash;
use crate::reconcile_capnp::reconcile as Reconcile;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::task::LocalSpawn;
use futures::{AsyncReadExt, FutureExt};
use rusqlite::params;
use std::collections::HashSet;
use std::include_str;

#[allow(dead_code)]
pub async fn reconcile(
    stream: async_std::net::TcpStream,
    connection: std::sync::Arc<rusqlite::Connection>,
    spawner: futures::executor::LocalSpawner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    stream.set_nodelay(true)?;
    let (reader, writer) = stream.split();
    let network = Box::new(twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let reconcile: Reconcile::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    die_on_error(spawner.spawn_local_obj(Box::pin(rpc_system.map(|_| ())).into()));
    let request = reconcile.hashes_request();
    let result = request.send().promise.await?;
    let their_hashes = result.get()?.get_hashes()?;

    let mut statement =
        die_on_error(connection.prepare(include_str!("../sql/B. RPC/2. Retrieve message.sql")));
    let mut hash_set = HashSet::<Vec<u8>>::new();
    for i in 0..their_hashes.len() - 1 {
        let hash = their_hashes.get(i)?;
        hash_set.insert(hash.to_vec());
        if !statement.exists(params![hash])? {
            let mut query_request = reconcile.query_request();
            query_request.get().set_hash(hash);
            let result = query_request.send().promise.await?;
            let message = result.get()?.get_message()?;
            let payload = message.get_payload()?;
            let nonce = message.get_nonce();
            let expiration_time = message.get_expiration_time();
            if crate::proof_of_work::verify(payload, nonce, expiration_time) {
                die_on_error(connection.execute(
                    include_str!("../sql/B. RPC/3. Put message.sql"),
                    params![
                        message_hash(payload, expiration_time).to_vec(),
                        payload,
                        nonce,
                        expiration_time
                    ],
                ));
            }
        }
    }
    let mut statement = die_on_error(connection.prepare(include_str!(
        "../sql/B. RPC/4. Retrieve each and every message.sql"
    )));
    let mut rows = die_on_error(statement.query(params![]));
    while let Some(row) = die_on_error(rows.next()) {
        let hash: Vec<u8> = row.get(0)?;
        if !hash_set.contains(&hash) {
            let mut submit_request = reconcile.submit_request();
            let payload: Vec<u8> = row.get(1)?;
            let nonce = row.get(2)?;
            let expiration_time = row.get(3)?;
            submit_request.get().get_message()?.set_payload(&payload);
            submit_request.get().get_message()?.set_nonce(nonce);
            submit_request
                .get()
                .get_message()?
                .set_expiration_time(expiration_time);
        }
    }
    Ok(())
}
