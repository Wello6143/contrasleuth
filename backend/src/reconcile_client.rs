use crate::die_on_error::die_on_error;
use crate::inventory;
use crate::mpmc_manual_reset_event::MPMCManualResetEvent;
use crate::reconcile_capnp::reconcile as Reconcile;
use async_std::sync::RwLock;
use async_std::task;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::task::LocalSpawn;
use futures::AsyncReadExt;
use futures_intrusive::channel::LocalUnbufferedChannel;
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashSet;

pub async fn reconcile(
    stream: async_std::net::TcpStream,
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    spawner: futures::executor::LocalSpawner,
    reconciliation_intent: std::rc::Rc<RwLock<MPMCManualResetEvent>>,
) -> Result<(), capnp::Error> {
    stream.set_nodelay(true)?;
    let (reader, writer) = stream.split();
    let network = twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    );
    let mut rpc_system = RpcSystem::new(Box::new(network), None);
    let reconcile: Reconcile::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    let handle = reconciliation_intent.write().await.get_handle();
    let reconciliation_intent1 = reconciliation_intent.clone();
    let reconciliation_intent2 = reconciliation_intent.clone();

    #[derive(Debug)]
    enum TerminateOrProceed {
        Terminate(Result<(), capnp::Error>),
        Proceed,
    };

    let channel = std::rc::Rc::new(LocalUnbufferedChannel::new());
    let channel1 = channel.clone();
    let channel2 = channel.clone();

    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async move {
                if let Err(error) = rpc_system.await {
                    if let Err(_) = channel1
                        .send(TerminateOrProceed::Terminate(Err(error)))
                        .await
                    {}
                } else {
                    if let Err(_) = channel1.send(TerminateOrProceed::Terminate(Ok(()))).await {}
                }
                reconciliation_intent1.write().await.drop_handle(handle);
            })
            .into(),
        ),
    );

    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async move {
                let event = reconciliation_intent2.read().await.get_event(handle);
                loop {
                    event.wait().await;
                    event.reset();
                    if let Err(_) = channel2.send(TerminateOrProceed::Proceed).await {
                        break;
                    }
                }
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
        for i in 0..their_hashes.len() {
            let hash1 = std::sync::Arc::new(their_hashes.get(i)?.to_vec());
            let hash2 = hash1.clone();
            hash_set.insert(hash1.to_vec());
            let connection1 = connection1.clone();
            let connection2 = connection2.clone();
            if !task::spawn(async move { inventory::exists(connection1, &hash1) }).await {
                let mut query_request = reconcile.query_request();
                query_request.get().set_hash(&hash2);
                let result = query_request.send().promise.await?;
                let message = result.get()?.get_message()?;
                if let crate::reconcile_capnp::maybe_message::Some(message) = message.which()? {
                    let message = message?;
                    let payload = message.get_payload()?.to_vec();
                    let nonce = message.get_nonce();
                    let expiration_time = message.get_expiration_time();
                    if crate::proof_of_work::verify(&payload, nonce, expiration_time) {
                        task::spawn(async move {
                            inventory::insert(connection2, &payload, nonce, expiration_time);
                        })
                        .await;
                        reconciliation_intent
                            .read()
                            .await
                            .broadcast_to_others(handle);
                    }
                }
            }
        }

        {
            let connection = connection.clone();
            let connection1 = connection.clone();
            let channel = inventory::hashes(connection);

            while let Some(hash) = channel.clone().recv().await {
                if !hash_set.contains(&hash) {
                    let connection1 = connection1.clone();
                    let message =
                        match task::spawn(async move { inventory::retrieve(connection1, &hash) })
                            .await
                        {
                            Some(message) => message,
                            None => continue,
                        };
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
        }

        match channel.receive().await {
            Some(terminate_or_proceed) => match terminate_or_proceed {
                TerminateOrProceed::Terminate(result) => return result,
                TerminateOrProceed::Proceed => continue,
            },
            None => return Ok(()),
        }
    }
}
