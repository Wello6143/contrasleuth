use crate::connect::{connect, reverse_connect};
use crate::die_on_error::die_on_error;
use crate::inventory;
use crate::log;
use crate::mpmc_manual_reset_event::MPMCManualResetEvent;
use async_std::sync::RwLock;
use async_std::{io, task};
use futures::executor::LocalSpawner;
use futures::task::LocalSpawn;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::exit;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Using the same operation_id for two or more operations is undefined
/// behavior.
#[derive(Serialize, Deserialize, Debug)]
enum Operation {
    Submit {
        payload: Vec<u8>,
        expiration_time: i64,
        operation_id: String,
    },
    Query {
        hash: Vec<u8>,
        operation_id: String,
    },
    CancelSubmitOperation {
        to_be_cancelled: String,
    },
    EstablishConnection {
        address: String,
        operation_id: String,
    },
    EstablishReverseConnection {
        address: String,
        operation_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message<'a> {
    Inventory(Vec<Vec<u8>>),
    Message {
        in_reply_to: &'a str,
        message: Option<inventory::Message>,
    },
    ProofOfWorkCancelled {
        in_reply_to: &'a str,
    },
    ProofOfWorkCompleted {
        in_reply_to: &'a str,
    },
    ConnectionEstablishmentFailure {
        in_reply_to: &'a str,
    },
    ReconcileFailure {
        in_reply_to: &'a str,
    },
    ServerListenAddress {
        address: &'a str,
    },
    ClientListenAddress {
        address: &'a str,
    },
}

pub fn format_struct<T: Serialize>(value: &T) -> String {
    base64::encode(&die_on_error(serde_json::to_string(value)))
}

pub async fn communicate(
    reconciliation_intent: std::rc::Rc<RwLock<MPMCManualResetEvent>>,
    connection: std::sync::Arc<r2d2::Pool<SqliteConnectionManager>>,
    spawner: LocalSpawner,
) {
    let atomic_cancel_flags: Rc<RwLock<HashMap<String, Arc<AtomicBool>>>> =
        Rc::new(RwLock::new(HashMap::new()));
    {
        let connection = connection.clone();
        let reconciliation_intent = reconciliation_intent.clone();
        die_on_error(
            spawner.spawn_local_obj(
                Box::new(async move {
                    let handle = reconciliation_intent.write().await.get_handle();
                    loop {
                        let channel = inventory::hashes(connection.clone());
                        let mut hashes = Vec::new();
                        while let Some(hash) = channel.receive().await {
                            hashes.push(hash);
                        }
                        log::ipc(format_struct(&Message::Inventory(hashes)));
                        let event = reconciliation_intent.read().await.get_event(handle);
                        event.wait().await;
                        event.reset();
                    }
                })
                .into(),
            ),
        );
    }
    loop {
        let mut line = String::new();
        match io::stdin().read_line(&mut line).await {
            Ok(_) => {
                let operation_result =
                    serde_json::from_slice::<Operation>(&match base64::decode(&line.trim()) {
                        Ok(value) => value,
                        Err(_) => {
                            log::fatal(format!(
                                "Received RPC command is not valid base64. Offending command: {}",
                                line.trim()
                            ));
                            exit(1);
                        }
                    });
                let operation = match operation_result {
                    Ok(operation) => operation,
                    Err(_) => {
                        log::fatal(format!(
                            "Received RPC command can't be parsed. Offending command: {}",
                            line.trim()
                        ));
                        exit(1);
                    }
                };
                match operation {
                    Operation::Submit {
                        payload,
                        expiration_time,
                        operation_id,
                    } => {
                        log::notice(
                            "A task has been spawned to calculate the proof of work. Hang tight.",
                        );
                        let atomic_cancel_flags = atomic_cancel_flags.clone();
                        let reconciliation_intent = reconciliation_intent.clone();
                        let connection = connection.clone();
                        die_on_error(
                            spawner.spawn_local_obj(
                                Box::new(async move {
                                    use crate::proof_of_work::{get_expected_target2, prove};
                                    let target = get_expected_target2(&payload, expiration_time);
                                    let cancelled = Arc::new(AtomicBool::new(false));
                                    let cancelled2 = cancelled.clone();
                                    atomic_cancel_flags
                                        .write()
                                        .await
                                        .insert(operation_id.to_owned(), cancelled);
                                    let nonce = prove(
                                        &payload,
                                        match target {
                                            Some(target) => target,
                                            None => {
                                                log::fatal(format!(
                                                    "Expiration time is in the past. Offending command: {}",
                                                    line.trim()
                                                ));
                                                atomic_cancel_flags
                                                    .write()
                                                    .await
                                                    .remove(&operation_id);
                                                exit(1);
                                            }
                                        },
                                        cancelled2,
                                    )
                                    .await;
                                    let nonce = match nonce {
                                        Some(nonce) => nonce,
                                        None => {
                                            log::ipc(format_struct(
                                                &Message::ProofOfWorkCancelled {
                                                    in_reply_to: &operation_id,
                                                },
                                            ));
                                            atomic_cancel_flags
                                                .write()
                                                .await
                                                .remove(&operation_id);
                                            log::notice("Proof of work cancelled");
                                            return;
                                        }
                                    };
                                    inventory::insert(connection, &payload, nonce, expiration_time);
                                    reconciliation_intent.read().await.broadcast();
                                    log::ipc(format_struct(&Message::ProofOfWorkCompleted {
                                        in_reply_to: &operation_id,
                                    }));
                                    atomic_cancel_flags
                                        .write()
                                        .await
                                        .remove(&operation_id);
                                    log::notice("Message submitted successfully");
                                })
                                .into(),
                            ),
                        );
                    }
                    Operation::Query { hash, operation_id } => {
                        let connection = connection.clone();
                        task::spawn(async move {
                            log::ipc(format_struct(&Message::Message {
                                in_reply_to: &operation_id,
                                message: inventory::retrieve(connection, &hash),
                            }));
                        });
                    }
                    Operation::CancelSubmitOperation { to_be_cancelled } => {
                        let locked = atomic_cancel_flags.read().await;
                        let flag = match locked.get(&to_be_cancelled) {
                            Some(flag) => flag,
                            None => {
                                log::warning(format!(
                                    "Submit operation doesn't exist. Offending command: {}",
                                    line.trim()
                                ));
                                continue;
                            }
                        };
                        flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    Operation::EstablishConnection {
                        address,
                        operation_id,
                    } => {
                        let operation_id1 = std::rc::Rc::new(operation_id);
                        let operation_id2 = operation_id1.clone();
                        let socket_address1 = std::rc::Rc::new(address.clone());
                        let socket_address2 = socket_address1.clone();
                        connect(
                            address,
                            connection.clone(),
                            spawner.clone(),
                            reconciliation_intent.clone(),
                            move |error| {
                                log::warning(format!(
                                    "Can't connect to {} due to error {:?}",
                                    socket_address1, error
                                ));
                                log::ipc(format_struct(&Message::ConnectionEstablishmentFailure {
                                    in_reply_to: &operation_id1,
                                }));
                            },
                            move |error| {
                                log::warning(format!(
                                    "Error occurred while reconciling with {} due to error {:?}",
                                    socket_address2, error
                                ));
                                log::ipc(format_struct(&Message::ReconcileFailure {
                                    in_reply_to: &operation_id2,
                                }));
                            },
                        );
                    }
                    Operation::EstablishReverseConnection {
                        address,
                        operation_id,
                    } => {
                        let operation_id1 = std::rc::Rc::new(operation_id);
                        let operation_id2 = operation_id1.clone();
                        let socket_address1 = std::rc::Rc::new(address.clone());
                        let socket_address2 = socket_address1.clone();
                        reverse_connect(
                            address,
                            connection.clone(),
                            spawner.clone(),
                            reconciliation_intent.clone(),
                            move |error| {
                                log::warning(format!(
                                    "Can't connect to {} due to error {:?}",
                                    socket_address1, error
                                ));
                                log::ipc(format_struct(&Message::ConnectionEstablishmentFailure {
                                    in_reply_to: &operation_id1,
                                }));
                            },
                            move |error| {
                                log::warning(format!(
                                    "Error occurred while reconciling with {} due to error {:?}",
                                    socket_address2, error
                                ));
                                log::ipc(format_struct(&Message::ReconcileFailure {
                                    in_reply_to: &operation_id2,
                                }));
                            },
                        );
                    }
                }
            }
            Err(error) => {
                log::warning(format!("Unexpected STDIN error: {:?}", error));
            }
        }
    }
}
