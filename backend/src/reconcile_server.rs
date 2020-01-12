use crate::die_on_error::die_on_error;
use crate::message_hash::message_hash;
use crate::mpmc_manual_reset_event::MPMCManualResetEvent;
use crate::reconcile_capnp::reconcile as Reconcile;
use async_std::task;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::task::LocalSpawn;
use futures::AsyncReadExt;
use futures_intrusive::sync::LocalMutex;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::borrow::Borrow;
use std::convert::TryInto;
use std::include_str;
struct ReconcileRPCServer {
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    reconciliation_intent: std::rc::Rc<LocalMutex<MPMCManualResetEvent>>,
    spawner: futures::executor::LocalSpawner,
}

impl ReconcileRPCServer {
    fn new(
        connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
        reconciliation_intent: std::rc::Rc<LocalMutex<MPMCManualResetEvent>>,
        spawner: futures::executor::LocalSpawner,
    ) -> ReconcileRPCServer {
        ReconcileRPCServer {
            connection,
            reconciliation_intent,
            spawner,
        }
    }
}

impl Reconcile::Server for ReconcileRPCServer {
    fn hashes(
        &mut self,
        _params: Reconcile::HashesParams,
        mut results: Reconcile::HashesResults,
    ) -> Promise<(), Error> {
        let connection = self.connection.clone();
        Promise::from_future(async move {
            let hashes1 = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
            let hashes2 = hashes1.clone();
            task::spawn(async move {
                let connection = die_on_error(connection.get());
                let mut statement = die_on_error(
                    connection.prepare(include_str!("../sql/B. RPC/1. Retrieve hashes.sql")),
                );
                let mut rows = die_on_error(statement.query(params![]));
                while let Some(row) = die_on_error(rows.next()) {
                    die_on_error(hashes1.lock()).push(die_on_error(row.get(0)));
                }
            })
            .await;
            let length: u32 = die_on_error(die_on_error(hashes2.lock()).len().try_into());
            let mut result = results.get().init_hashes(length);

            for i in 0..length {
                let vector_index: usize = die_on_error(i.try_into());
                result.set(i, &die_on_error(hashes2.lock())[vector_index]);
            }
            Ok(())
        })
    }

    fn query(
        &mut self,
        params: Reconcile::QueryParams,
        mut results: Reconcile::QueryResults,
    ) -> Promise<(), Error> {
        let connection = self.connection.clone();
        Promise::from_future(async move {
            let hash = params.get()?.get_hash()?;
            let connection = die_on_error(connection.get());
            let mut statement = die_on_error(
                connection.prepare(include_str!("../sql/B. RPC/2. Retrieve message.sql")),
            );
            let mut rows = die_on_error(statement.query(params![hash]));
            while let Some(row) = die_on_error(rows.next()) {
                let payload: Vec<u8> = die_on_error(row.get(1));
                let nonce: i64 = die_on_error(row.get(2));
                let expiration_time: i64 = die_on_error(row.get(3));
                let mut result = results.get().get_message()?;
                result.set_payload(&payload);
                result.set_nonce(nonce);
                result.set_expiration_time(expiration_time);
                return Ok(());
            }
            return Err(Error {
                description: "message does not exist or has expired".to_string(),
                kind: capnp::ErrorKind::Failed,
            });
        })
    }

    fn submit(
        &mut self,
        params: Reconcile::SubmitParams,
        _results: Reconcile::SubmitResults,
    ) -> Promise<(), Error> {
        let connection1 = self.connection.clone();
        let connection2 = self.connection.clone();
        let reconciliation_intent = self.reconciliation_intent.clone();
        let spawner = self.spawner.clone();
        let message = pry!(pry!(params.get()).get_message());
        let payload = pry!(message.get_payload()).to_vec();
        let nonce = message.get_nonce();
        let expiration_time = message.get_expiration_time();
        Promise::from_future(async move {
            let hash1 = std::sync::Arc::new(message_hash(&payload, expiration_time).to_vec());
            let hash2 = hash1.clone();
            let message_exists = task::spawn(async move {
                let connection = die_on_error(connection1.get());
                let mut statement = die_on_error(
                    connection.prepare(include_str!("../sql/B. RPC/2. Retrieve message.sql")),
                );
                die_on_error(statement.exists(params![hash1.borrow() as &Vec<u8>]))
            })
            .await;

            let proof_of_work_valid =
                crate::proof_of_work::verify(&payload, nonce, expiration_time);

            if !message_exists && proof_of_work_valid {
                task::spawn(async move {
                    let connection = die_on_error(connection2.get());
                    die_on_error(connection.execute(
                        include_str!("../sql/B. RPC/3. Put message.sql"),
                        params![hash2.borrow() as &Vec<u8>, payload, nonce, expiration_time],
                    ));
                })
                .await;
                let cloned = reconciliation_intent.clone();
                die_on_error(
                    spawner.spawn_local_obj(
                        Box::new(async move {
                            cloned.lock().await.broadcast();
                        })
                        .into(),
                    ),
                );
            }
            Ok(())
        })
    }
}

pub async fn init_server(
    stream: async_std::net::TcpStream,
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    spawner: futures::executor::LocalSpawner,
    reconciliation_intent: std::rc::Rc<LocalMutex<MPMCManualResetEvent>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reconcile = Reconcile::ToClient::new(ReconcileRPCServer::new(
        connection,
        reconciliation_intent,
        spawner.clone(),
    ))
    .into_client::<capnp_rpc::Server>();
    stream.set_nodelay(true)?;
    let (reader, writer) = stream.split();
    let network = twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Server,
        Default::default(),
    );
    let rpc_system = RpcSystem::new(Box::new(network), Some(reconcile.clone().client));
    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async move {
                if let Err(error) = rpc_system.await {
                    crate::log::warning(format!("Error occurred while reconciling: {:?}", error));
                }
            })
            .into(),
        ),
    );
    Ok(())
}
