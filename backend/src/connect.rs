use crate::die_on_error::die_on_error;
use crate::log;
use crate::mpmc_manual_reset_event;
use crate::reconcile_client;
use crate::reconcile_server;
use async_std::sync::RwLock;
use futures::executor::LocalSpawner;
use futures::task::LocalSpawn;
use r2d2_sqlite::SqliteConnectionManager;
pub fn connect<F1, F2>(
    address: String,
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    handle: LocalSpawner,
    reconciliation_intent: std::rc::Rc<RwLock<mpmc_manual_reset_event::MPMCManualResetEvent>>,
    on_connection_failed: F1,
    on_reconcile_failed: F2,
) where
    F1: FnOnce(std::io::Error) -> () + 'static,
    F2: FnOnce(std::boxed::Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>) -> ()
        + 'static,
{
    die_on_error(
        handle.spawn_local_obj(
            Box::new(async move {
                let exec = futures::executor::LocalPool::new();
                let spawner = exec.spawner();
                log::notice(format!("Connecting to {}", address));
                let stream = match async_std::net::TcpStream::connect(&address).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        on_connection_failed(error);
                        return;
                    }
                };
                if let Err(error) = reconcile_client::reconcile(
                    stream,
                    connection.clone(),
                    spawner.clone(),
                    reconciliation_intent,
                )
                .await
                {
                    on_reconcile_failed(error);
                }
            })
            .into(),
        ),
    );
}

pub fn reverse_connect<F1, F2>(
    address: String,
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    handle: LocalSpawner,
    reconciliation_intent: std::rc::Rc<RwLock<mpmc_manual_reset_event::MPMCManualResetEvent>>,
    on_connection_failed: F1,
    on_reconcile_failed: F2,
) where
    F1: FnOnce(std::io::Error) -> () + 'static,
    F2: FnOnce(std::boxed::Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>) -> ()
        + 'static,
{
    die_on_error(
        handle.spawn_local_obj(
            Box::new(async move {
                let exec = futures::executor::LocalPool::new();
                let spawner = exec.spawner();
                log::notice(format!("Connecting to {}", address));
                let stream = match async_std::net::TcpStream::connect(&address).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        on_connection_failed(error);
                        return;
                    }
                };
                if let Err(error) = reconcile_server::init_server(
                    stream,
                    connection.clone(),
                    spawner.clone(),
                    reconciliation_intent,
                )
                .await
                {
                    on_reconcile_failed(error);
                }
            })
            .into(),
        ),
    );
}
