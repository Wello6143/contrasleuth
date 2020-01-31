use crate::die_on_error::die_on_error;
use crate::inventory;
use crate::message_hash::message_hash;
use crate::mpmc_manual_reset_event::MPMCManualResetEvent;
use crate::reconcile_capnp::reconcile as Reconcile;
use async_std::sync::RwLock;
use async_std::task;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::AsyncReadExt;
use r2d2_sqlite::SqliteConnectionManager;
use std::convert::TryInto;
struct ReconcileRPCServer {
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    reconciliation_intent: std::rc::Rc<RwLock<MPMCManualResetEvent>>,
}

impl ReconcileRPCServer {
    fn new(
        connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
        reconciliation_intent: std::rc::Rc<RwLock<MPMCManualResetEvent>>,
    ) -> ReconcileRPCServer {
        ReconcileRPCServer {
            connection,
            reconciliation_intent,
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
                let channel = inventory::hashes(connection);
                while let Some(hash) = channel.recv().await {
                    die_on_error(hashes1.lock()).push(hash);
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
            let hash = params.get()?.get_hash()?.to_vec();
            let message =
                match task::spawn(async move { inventory::retrieve(connection, &hash) }).await {
                    Some(message) => message,
                    None => {
                        results.get().get_message()?.set_none(());
                        return Ok(());
                    }
                };
            let mut result = results.get().get_message()?.init_some();
            result.set_payload(&message.payload);
            result.set_nonce(message.nonce);
            result.set_expiration_time(message.expiration_time);
            Ok(())
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
        let message = pry!(pry!(params.get()).get_message());
        let payload = pry!(message.get_payload()).to_vec();
        let nonce = message.get_nonce();
        let expiration_time = message.get_expiration_time();
        Promise::from_future(async move {
            let hash1 = std::sync::Arc::new(message_hash(&payload, expiration_time).to_vec());
            let message_exists =
                task::spawn(async move { inventory::exists(connection1, &hash1) }).await;

            let proof_of_work_valid =
                crate::proof_of_work::verify(&payload, nonce, expiration_time);

            if !message_exists && proof_of_work_valid {
                task::spawn(async move {
                    inventory::insert(connection2, &payload, nonce, expiration_time);
                })
                .await;
                let cloned = reconciliation_intent.clone();
                cloned.read().await.broadcast();
            }
            Ok(())
        })
    }
}

pub async fn init_server(
    stream: async_std::net::TcpStream,
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    reconciliation_intent: std::rc::Rc<RwLock<MPMCManualResetEvent>>,
) -> Result<(), capnp::Error> {
    let reconcile =
        Reconcile::ToClient::new(ReconcileRPCServer::new(connection, reconciliation_intent))
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
    rpc_system.await
}
