extern crate capnp;
extern crate capnp_rpc;
extern crate chrono;
extern crate clap;
extern crate rusqlite;
use clap::{App, Arg};
use rusqlite::{params, Connection};
use std::include_str;
use std::net::SocketAddr;
use std::process::exit;
mod connect_reconcile;
mod die_on_error;
mod log;
use die_on_error::die_on_error;
mod reconcile_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/reconcile_capnp.rs"));
}
fn main() {
    let matches = App::new("Contrasleuth")
        .version("0.1.0")
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
    let address = matches.value_of("address").unwrap();

    match address.parse::<SocketAddr>() {
        Ok(_) => {}
        Err(_) => {
            log::fatal("TCP listen address is invalid");
            exit(1);
        }
    }

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
}
