// https://abronan.com/getting-started-with-capnproto-rpc-for-rust/
use clap::{App, Arg};
use rusqlite::{params, Connection};
use std::include_str;
use std::net::SocketAddr;
use std::process::exit;
mod die_on_error;
mod log;
mod reconcile_server;
use die_on_error::die_on_error;
mod reconcile_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/reconcile_capnp.rs"));
}
use async_std::task;
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

    let connection = match Connection::open(database_path) {
        Ok(connection) => connection,
        Err(_) => {
            log::fatal("Unable to open database file");
            exit(1);
        }
    };
    die_on_error(connection.execute(
        include_str!("../sql/A. Schema/1. Initial schema.sql"),
        params![],
    ));

    log::welcome("Welcome to Contrasleuth, a potent communication tool");
    log::welcome("Contrasleuth provides adequate protections for most users. Refer to the guide at https://contrasleuth.cf/warnings to better protect yourself.");

    log::notice(format!(
        "Listening for incoming TCP connections on {}",
        address
    ));
    task::block_on(async {
        reconcile_server::init_server(parsed_address, connection).await;
    });
}
