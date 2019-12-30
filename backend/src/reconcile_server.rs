use crate::die_on_error::die_on_error;
use crate::message_hash::message_hash;
use crate::reconcile_capnp::reconcile as Reconcile;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::task::LocalSpawn;
use futures::{AsyncReadExt, FutureExt, StreamExt, TryFutureExt};
use rusqlite::{params, Connection};
use std::convert::TryInto;
use std::include_str;
struct ReconcileRPCServer {
    connection: std::sync::Arc<Connection>,
}

impl ReconcileRPCServer {
    fn new(connection: std::sync::Arc<Connection>) -> ReconcileRPCServer {
        ReconcileRPCServer {
            connection: connection,
        }
    }
}

impl Reconcile::Server for ReconcileRPCServer {
    fn hashes(
        &mut self,
        _params: Reconcile::HashesParams,
        mut results: Reconcile::HashesResults,
    ) -> Promise<(), Error> {
        let mut statement = die_on_error(
            self.connection
                .prepare(include_str!("../sql/B. RPC/1. Retrieve hashes.sql")),
        );
        let mut rows = die_on_error(statement.query(params![]));
        let mut hashes = Vec::<Vec<u8>>::new();
        while let Some(row) = die_on_error(rows.next()) {
            hashes.push(die_on_error(row.get(0)));
        }
        let length: u32 = die_on_error(hashes.len().try_into());
        let mut result = results.get().init_hashes(length);

        for i in 0..length {
            let vector_index: usize = die_on_error(i.try_into());
            result.set(i, &hashes[vector_index]);
        }
        Promise::ok(())
    }

    fn query(
        &mut self,
        params: Reconcile::QueryParams,
        mut results: Reconcile::QueryResults,
    ) -> Promise<(), Error> {
        let hash = pry!(pry!(params.get()).get_hash());
        let mut statement = die_on_error(
            self.connection
                .prepare(include_str!("../sql/B. RPC/2. Retrieve message.sql")),
        );
        let mut rows = die_on_error(statement.query(params![hash]));
        while let Some(row) = die_on_error(rows.next()) {
            let payload: Vec<u8> = die_on_error(row.get(1));
            let nonce: i64 = die_on_error(row.get(2));
            let expiration_time: i64 = die_on_error(row.get(3));
            let mut result = pry!(results.get().get_message());
            result.set_payload(&payload);
            result.set_nonce(nonce);
            result.set_expiration_time(expiration_time);
            return Promise::ok(());
        }
        return Promise::err(Error {
            description: "message does not exist or has expired".to_string(),
            kind: capnp::ErrorKind::Failed,
        });
    }

    fn submit(
        &mut self,
        params: Reconcile::SubmitParams,
        _results: Reconcile::SubmitResults,
    ) -> Promise<(), Error> {
        let message = pry!(pry!(params.get()).get_message());
        let payload = pry!(message.get_payload());
        let nonce = message.get_nonce();
        let expiration_time = message.get_expiration_time();
        if crate::proof_of_work::verify(payload, nonce, expiration_time) {
            die_on_error(self.connection.execute(
                include_str!("../sql/B. RPC/3. Put message.sql"),
                params![
                    message_hash(payload, expiration_time).to_vec(),
                    payload,
                    nonce,
                    expiration_time
                ],
            ));
        }
        Promise::ok(())
    }
}

pub async fn init_server(
    address: async_std::net::SocketAddr,
    connection: std::sync::Arc<Connection>,
    spawner: futures::executor::LocalSpawner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = async_std::net::TcpListener::bind(&address).await?;
    let reconcile = Reconcile::ToClient::new(ReconcileRPCServer::new(connection))
        .into_client::<capnp_rpc::Server>();
    let mut incoming = listener.incoming();
    while let Some(socket) = incoming.next().await {
        let socket = socket?;
        socket.set_nodelay(true)?;
        let (reader, writer) = socket.split();
        let network = twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );
        let rpc_system = RpcSystem::new(Box::new(network), Some(reconcile.clone().client));
        match spawner.spawn_local_obj(Box::pin(rpc_system.map_err(|_| ()).map(|_| ())).into()) {
            Ok(_) => {}
            Err(error) => {
                crate::log::warning(format!("Failed to spawn local object: {:?}", error));
            }
        }
    }
    Ok(())
}
