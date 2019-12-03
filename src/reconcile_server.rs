use crate::die_on_error::die_on_error;
use crate::reconcile_capnp::reconcile as Reconcile;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::pry;
use rusqlite::{params, Connection};
use std::convert::TryInto;
use std::include_str;

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
            description: "message does not exist".to_string(),
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
