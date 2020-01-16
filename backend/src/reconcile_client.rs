use crate::die_on_error::die_on_error;
use crate::message_hash::message_hash;
use crate::mpmc_manual_reset_event::MPMCManualResetEvent;
use crate::reconcile_capnp::reconcile as Reconcile;
use async_std::task;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::task::LocalSpawn;
use futures::AsyncReadExt;
use futures_intrusive::channel::UnbufferedChannel;
use futures_intrusive::sync::LocalMutex;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::include_str;

#[allow(dead_code)]
pub async fn reconcile(
    stream: async_std::net::TcpStream,
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    spawner: futures::executor::LocalSpawner,
    reconciliation_intent: std::rc::Rc<LocalMutex<MPMCManualResetEvent>>,
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

    let handle = reconciliation_intent.lock().await.get_handle();
    let cloned = reconciliation_intent.clone();
    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async move {
                if let Err(error) = rpc_system.await {
                    crate::log::warning(format!("Error occurred while reconciling: {:?}", error));
                }
                cloned.lock().await.drop_handle(handle);
            })
            .into(),
        ),
    );

    loop {
        let request = reconcile.hashes_request();
        let result = request.send().promise.await?;
        let their_hashes = result.get()?.get_hashes()?;

        let mut hash_set = HashSet::<Vec<u8>>::new();
        let connection1 = connection.clone();
        let connection2 = connection.clone();
        for i in 0..their_hashes.len() - 1 {
            let hash1 = std::sync::Arc::new(their_hashes.get(i)?.to_vec());
            let hash2 = hash1.clone();
            hash_set.insert((hash1.borrow() as &Vec<u8>).to_vec());
            let connection1 = connection1.clone();
            let connection2 = connection2.clone();
            if !task::spawn(async move {
                let connection = die_on_error(connection1.get());
                let mut statement = die_on_error(
                    connection.prepare(include_str!("../sql/B. RPC/2. Retrieve message.sql")),
                );
                statement.exists(params![hash1.borrow() as &Vec<u8>])
            })
            .await?
            {
                let mut query_request = reconcile.query_request();
                query_request.get().set_hash(&hash2);
                let result = query_request.send().promise.await?;
                let message = result.get()?.get_message()?;
                let payload = message.get_payload()?.to_vec();
                let nonce = message.get_nonce();
                let expiration_time = message.get_expiration_time();
                if crate::proof_of_work::verify(&payload, nonce, expiration_time) {
                    task::spawn(async move {
                        die_on_error(die_on_error(connection2.get()).execute(
                            include_str!("../sql/B. RPC/3. Put message.sql"),
                            params![
                                message_hash(&payload, expiration_time).to_vec(),
                                payload,
                                nonce,
                                expiration_time
                            ],
                        ));
                    });
                    reconciliation_intent
                        .lock()
                        .await
                        .broadcast_to_others(handle);
                }
            }
        }

        #[derive(Debug)]
        struct Message {
            hash: Vec<u8>,
            payload: Vec<u8>,
            nonce: i64,
            expiration_time: i64,
        };

        let channel1 = std::sync::Arc::new(UnbufferedChannel::<Message>::new());
        let channel2 = channel1.clone();
        let connection = connection.clone();
        task::spawn(async move {
            let connection = die_on_error(connection.get());
            let mut statement = die_on_error(connection.prepare(include_str!(
                "../sql/B. RPC/4. Retrieve each and every message.sql"
            )));
            let mut rows = die_on_error(statement.query(params![]));
            while let Some(row) = die_on_error(rows.next()) {
                let hash = die_on_error(row.get(0));
                let payload = die_on_error(row.get(1));
                let nonce = die_on_error(row.get(2));
                let expiration_time = die_on_error(row.get(3));
                let channel1 = channel1.clone();
                let channel2 = channel1.clone();
                task::spawn(async move {
                    die_on_error(
                        channel1
                            .send(Message {
                                hash,
                                payload,
                                nonce,
                                expiration_time,
                            })
                            .await,
                    );
                });
                channel2.close();
            }
        });

        while let Some(message) = channel2.clone().receive().await {
            if !hash_set.contains(&message.hash) {
                let mut submit_request = reconcile.submit_request();
                submit_request
                    .get()
                    .get_message()?
                    .set_payload(&message.payload);
                submit_request.get().get_message()?.set_nonce(message.nonce);
                submit_request
                    .get()
                    .get_message()?
                    .set_expiration_time(message.expiration_time);
                submit_request.send().promise.await?;
            }
        }
        reconciliation_intent.lock().await.block(handle).await;
    }
}
