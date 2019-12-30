use clap::{App, Arg};
use rusqlite::{params, Connection};
use std::include_str;
use std::net::SocketAddr;
use std::process::exit;
mod die_on_error;
mod log;
mod message_hash;
mod proof_of_work;
mod reconcile_client;
mod reconcile_server;
use die_on_error::die_on_error;
mod reconcile_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/reconcile_capnp.rs"));
}
use async_std::io;
use futures::executor::LocalSpawner;
use futures::task::LocalSpawn;

fn connect(
    address: String,
    connection: std::sync::Arc<rusqlite::Connection>,
    handle: LocalSpawner,
) {
    die_on_error(
        handle.spawn_local_obj(
            Box::new(async move {
                loop {
                    let exec = futures::executor::LocalPool::new();
                    let spawner = exec.spawner();
                    log::notice(format!("Connecting to {}", address));
                    match reconcile_client::reconcile(
                        address.to_owned(),
                        connection.clone(),
                        spawner,
                    )
                    .await
                    {
                        Ok(_) => {
                            log::warning(format!(
                                "Connection to {} completed, reconnecting",
                                address
                            ));
                        }
                        Err(error) => {
                            log::warning(format!(
                                "Connection to {} failed: {:?}, reconnecting",
                                address, error
                            ));
                        }
                    }
                }
            })
            .into(),
        ),
    );
}

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

    let connection = std::sync::Arc::new(match Connection::open(database_path) {
        Ok(connection) => connection,
        Err(_) => {
            log::fatal("Unable to open database file");
            exit(1);
        }
    });
    die_on_error(connection.execute(
        include_str!("../sql/A. Schema/1. Initial schema.sql"),
        params![],
    ));

    log::welcome("Welcome to Contrasleuth, a potent communication tool");
    log::welcome("Contrasleuth provides adequate protections for most users. Refer to the guide at https://contrasleuth.cf/warnings to better protect yourself.");
    log::welcome("Tip: Input a socket address through STDIN makes Contrasleuth connect to it");
    log::notice(format!(
        "Listening for incoming TCP connections on {}",
        address
    ));
    let mut exec = futures::executor::LocalPool::new();
    let spawner = exec.spawner();

    let server_handle = spawner.clone();

    let connection_clone_1 = connection.clone();
    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async move {
                match reconcile_server::init_server(
                    parsed_address,
                    connection_clone_1.clone(),
                    server_handle,
                )
                .await
                {
                    Ok(_) => {}
                    Err(error) => {
                        log::fatal(format!(
                            "Failed to bind to {} due to error {:?}",
                            address, error
                        ));
                        exit(1);
                    }
                }
            })
            .into(),
        ),
    );

    let client_handle = spawner.clone();
    let connection_clone_2 = connection.clone();
    die_on_error(
        spawner.spawn_local_obj(
            Box::new(async move {
                loop {
                    let mut address = String::new();
                    match io::stdin().read_line(&mut address).await {
                        Ok(_) => {
                            connect(
                                address.trim().to_owned(),
                                connection_clone_2.clone(),
                                client_handle.clone(),
                            );
                        }
                        Err(error) => {
                            log::warning(format!("Unexpected STDIN error: {:?}", error));
                        }
                    }
                }
            })
            .into(),
        ),
    );
    exec.run();
}
