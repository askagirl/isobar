extern crate docopt;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use std::env;
use std::process::Command;
use std::path::Path;
use std::error::Error;
use docopt::Docopt;
use std::os::unix::net::UnixStream;
use serde_json::value::Value;
use std::io::Write;

const USAGE: &'static str = "
Isobar

Usage:
  isobar <path>...
  isobar (-h | --help)

Options:
  -h --help      Show this screen.
";

const DEFAULT_SOCKET_PACK: &'static str = "/tmp/isobar.sock";

#[derive(Debug, Deserialize)]
struct Args {
    arg_path: Vec<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let message = json!({
        "type": "OpenWorkspace",
        "paths": args.arg_path
    });

    let socket_path = DEFAULT_SOCKET_PACK;

    if let Ok(mut socket) = UnixStream::connect(socket_path) {
        if let Err(error) = write_to_socket(&mut socket, message) {
            eprintln!(
                "Failed to write to {}: {}",
                socket_path,
                error.description()
            );
        }
        return;
    }

    if let Ok(src_path) = env::var("ISOBAR_SRC_PATH") {
        let src_path = Path::new(&src_path);
        let electron_app_path = src_path.join("isobar_electron");
        let electron_bin_path = electron_app_path.join("node_modules/.bin/electron");
        Command::new(electron_bin_path)
            .arg(electron_app_path)
            .env("ISOBAR_INITIAL_MESSAGE", message.to_string())
            .spawn()
            .expect("Failed to open Isobar app");
    } else {
        eprintln!("Must specify the ISOBAR_APP_PATH environment variable");
    }
}

fn write_to_socket(socket: &mut UnixStream, value: Value) -> Result<(), Box<Error>> {
    let vec = serde_json::to_vec(&value)?;
    socket.write(&vec)?;
    socket.write(b"\n")?;
    Ok(())
}
