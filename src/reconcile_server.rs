use crate::die_on_error::die_on_error;
use crate::reconcile_capnp::reconcile as Reconcile;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{Future, Stream};
// https://stackoverflow.com/questions/58611380/is-there-a-way-that-we-can-convert-from-futures-0-1-to-the-standard-library-futu
use futures_03::compat::Future01CompatExt;
use rusqlite::{params, Connection};
use std::convert::TryInto;
use std::include_str;
use std::net::SocketAddr;
use tokio_core::reactor;
use tokio_io::AsyncRead;

struct ReconcileRPCServer {
    connection: Connection,
}

impl ReconcileRPCServer {
    fn new(connection: Connection) -> ReconcileRPCServer {
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

    fn request_reconciliation(
        &mut self,
        params: Reconcile::RequestReconciliationParams,
        _results: Reconcile::RequestReconciliationResults,
    ) -> Promise<(), Error> {
        let hashes = pry!(pry!(params.get()).get_hashes());
        Promise::ok(())
    }
}

pub async fn init_server(address: SocketAddr, connection: Connection) {
    let mut core = die_on_error(reactor::Core::new());
    let handle = core.handle();
    let listener = die_on_error(tokio_core::net::TcpListener::bind(&address, &handle));
    let reconcile = Reconcile::ToClient::new(ReconcileRPCServer::new(connection))
        .into_client::<capnp_rpc::Server>();
    let handle1 = handle.clone();
    let done = listener.incoming().for_each(move |(socket, _)| {
        socket.set_nodelay(true)?;
        let (reader, writer) = socket.split();
        let handle = handle1.clone();
        let network = twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );
        let rpc_system = RpcSystem::new(Box::new(network), Some(reconcile.clone().client));
        handle.spawn(rpc_system.map_err(|_| ()));
        Ok(())
    });
    die_on_error(done.compat().await);
}
