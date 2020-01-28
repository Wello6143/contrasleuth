use clap::{App, Arg};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::include_str;
use std::net::SocketAddr;
use std::process::exit;
mod connect;
mod die_on_error;
mod inventory;
mod log;
mod message_hash;
mod mpmc_manual_reset_event;
mod proof_of_work;
mod reconcile_client;
mod reconcile_server;
use die_on_error::die_on_error;
mod stdio_ipc;
mod reconcile_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/reconcile_capnp.rs"));
}
use async_std::prelude::*;
use async_std::sync::RwLock;
use futures::task::LocalSpawn;

fn main() {
    let matches = App::new("Contrasleuth")
        .version("prerelease")
        .author("Transparent <transparent.cf@gmail.com>")
        .about("A potent communication tool")
        .arg(
            Arg::with_name("database")
                .short("f")
                .long("database")
                .value_name("FILE")
                .help("Sets the SQLite database file")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("address")
                .short("l")
                .long("address")
                .value_name("ADDRESS")
                .help("Sets the TCP listen address")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("reverse client address")
                .short("r")
                .long("reverse-address")
                .value_name("REVERSE_ADDRESS")
                .help("Sets the reverse reconciliation client address")
                .takes_value(true),
        )
        .get_matches();

    let database_path = matches.value_of("database").unwrap();
    let address = matches.value_of("address").unwrap().to_owned();

    let parsed_address = match address.parse::<SocketAddr>() {
        Ok(address) => address,
        Err(_) => {
            log::fatal("TCP listen address is invalid");
            exit(1);
        }
    };

    let reverse_address = match matches.value_of("reverse client address") {
        Some(value) => Some(value.to_owned()),
        None => None,
    };

    let parsed_reverse_address = match reverse_address.to_owned() {
        Some(address) => match address.parse::<SocketAddr>() {
            Ok(address) => Some(address),
            Err(_) => {
                log::fatal("Reverse reconciliation client address is invalid");
                exit(1);
            }
        },
        None => None,
    };

    let manager = SqliteConnectionManager::file(database_path);

    let connection = std::sync::Arc::new(match r2d2::Pool::new(manager) {
        Ok(connection) => connection,
        Err(_) => {
            log::fatal("Unable to open database file");
            exit(1);
        }
    });

    die_on_error(die_on_error(connection.get()).execute(
        include_str!("../sql/A. Schema/1. Initial schema.sql"),
        params![],
    ));

    log::welcome("Welcome to Contrasleuth, a potent communication tool");
    log::welcome("Contrasleuth provides adequate protections for most users. Refer to the guide at https://contrasleuth.cf/warnings to better protect yourself.");
    log::welcome("Standard streams are being used for interprocess communication");
    log::notice(format!(
        "Listening for incoming client connections on {}",
        address
    ));

    if let Some(address) = reverse_address.to_owned() {
        log::notice(format!(
            "Listening for incoming reverse server connections on {}",
            address
        ));
    }

    let mut exec = futures::executor::LocalPool::new();
    let spawner = exec.spawner();

    let spawner_clone = spawner.clone();

    let connection_clone = connection.clone();

    let reconciliation_intent = std::rc::Rc::new(RwLock::new(
        mpmc_manual_reset_event::MPMCManualResetEvent::new(),
    ));

    let reconciliation_intent_clone = reconciliation_intent.clone();
    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async move {
                let listener = match async_std::net::TcpListener::bind(&parsed_address).await {
                    Ok(listener) => listener,
                    Err(error) => {
                        log::fatal(format!(
                            "Failed to bind to {} due to error {:?}",
                            address, error
                        ));
                        exit(1);
                    }
                };
                let mut incoming = listener.incoming();
                let spawner_clone2 = spawner_clone.clone();
                while let Some(socket) = incoming.next().await {
                    match socket {
                        Ok(socket) => {
                            let spawner_clone3 = spawner_clone2.clone();
                            let connection_clone = connection_clone.clone();
                            let reconciliation_intent_clone = reconciliation_intent_clone.clone();
                            die_on_error(
                                spawner_clone2.spawn_local_obj(
                                    Box::new(async move {
                                        if let Err(error) = reconcile_server::init_server(
                                            socket,
                                            connection_clone.clone(),
                                            spawner_clone3.clone(),
                                            reconciliation_intent_clone.clone(),
                                        )
                                        .await
                                        {
                                            log::warning(format!(
                                                "Error occurred while reconciling: {:?}",
                                                error
                                            ));
                                        }
                                    })
                                    .into(),
                                ),
                            );
                        }
                        Err(error) => {
                            log::warning(format!(
                                "Unexpected error while accepting incoming socket: {:?}",
                                error
                            ));
                        }
                    }
                }
            })
            .into(),
        ),
    );

    let spawner_clone = spawner.clone();
    let reconciliation_intent_clone = reconciliation_intent.clone();
    if let Some(address) = parsed_reverse_address {
        let connection_clone = connection.clone();
        die_on_error(
            spawner.spawn_local_obj(
                Box::new(async move {
                    let listener = match async_std::net::TcpListener::bind(&address).await {
                        Ok(listener) => listener,
                        Err(error) => {
                            log::fatal(format!(
                                "Failed to bind to {} due to error {:?}",
                                reverse_address.unwrap(),
                                error
                            ));
                            exit(1);
                        }
                    };
                    let mut incoming = listener.incoming();
                    let spawner_clone2 = spawner_clone.clone();
                    while let Some(socket) = incoming.next().await {
                        match socket {
                            Ok(socket) => {
                                let spawner_clone3 = spawner_clone2.clone();
                                let connection_clone = connection_clone.clone();
                                let reconciliation_intent = reconciliation_intent_clone.clone();
                                die_on_error(
                                    spawner_clone2.spawn_local_obj(
                                        Box::new(async move {
                                            if let Err(error) = reconcile_client::reconcile(
                                                socket,
                                                connection_clone.clone(),
                                                spawner_clone3.clone(),
                                                reconciliation_intent.clone(),
                                            )
                                            .await
                                            {
                                                log::warning(format!(
                                                    "Error occurred while reconciling: {:?}",
                                                    error
                                                ));
                                            }
                                        })
                                        .into(),
                                    ),
                                );
                            }
                            Err(error) => {
                                log::warning(format!(
                                    "Unexpected error while accepting incoming socket: {:?}",
                                    error
                                ));
                            }
                        }
                    }
                })
                .into(),
            ),
        );
    }

    let spawner_clone = spawner.clone();
    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async {
                stdio_ipc::communicate(reconciliation_intent, connection, spawner_clone).await;
            })
            .into(),
        ),
    );
    exec.run();
}
