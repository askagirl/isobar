extern crate docopt;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use docopt::Docopt;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};

const USAGE: &'static str = "
Isobar

Usage:
  isobar [--socket-path=<path>] [--headless] [--listen=<port>] [--connect=<address>] [<path>...]
  isobar (-h | --help)

Options:
  -h --help              Show this screen.
  -H --headless          Start Isobar in headless mode.
  -l --listen=<port>     Listen for TCP connections on specified port.
  -c --connect=<address> Connect to the specific address.
";

type PortNumber = u16;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerRequest {
    StartCLI { headless: bool },
    OpenWorkspace { paths: Vec<PathBuf> },
    ConnectToPeer { address: SocketAddr },
    TcpListen { port: PortNumber },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ServerResponse {
    Ok,
    Error { description: String },
}

#[derive(Debug, Deserialize)]
struct Args {
    flag_socket_path: Option<String>,
    flag_headless: Option<bool>,
    flag_listen: Option<PortNumber>,
    flag_connect: Option<SocketAddr>,
    arg_path: Vec<PathBuf>,
}

fn main() {
    process::exit(match launch() {
        Ok(()) => 0,
        Err(description) => {
            eprintln!("{}", description);
            1
        }
    })
}

fn launch() -> Result<(), String> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let headless = args.flag_headless.unwrap_or(false);

    const DEFAULT_SOCKET_PATH: &'static str = "/tmp/isobar.sock";
    let socket_path = PathBuf::from(
        args.flag_socket_path
            .as_ref()
            .map_or(DEFAULT_SOCKET_PATH, |path| path.as_str()),
    );

    let mut socket = match UnixStream::connect(&socket_path) {
        Ok(socket) => socket,
        Err(_) => {
            let src_path = PathBuf::from(env::var("ISOBAR_SRC_PATH")
                .map_err(|_| "Must specify the ISOBAR_SRC_PATH environment variable")?);

            let server_bin_path;
            let node_env;
            if cfg!(debug_assertions) {
                server_bin_path = src_path.join("target/debug/isobar_server");
                node_env = "development";
            } else {
                server_bin_path = src_path.join("target/release/isobar_server");
                node_env = "production";
            }

            if headless {
                start_headless(&server_bin_path, &socket_path)?
            } else {
                start_electron(&src_path, &server_bin_path, &socket_path, &node_env)?
            }
        }
    };

    send_message(&mut socket, ServerRequest::StartCLI { headless })?;

    if let Some(address) = args.flag_connect {
        send_message(&mut socket, ServerRequest::ConnectToPeer { address })?;
    } else if args.arg_path.len() > 0 {
        let mut paths = Vec::new();
        for path in args.arg_path {
            paths.push(fs::canonicalize(&path)
                .map_err(|error| format!("Invalid path {:?} - {}", path, error))?);
        }
        send_message(&mut socket, ServerRequest::OpenWorkspace { paths })?;
    }

    if let Some(port) = args.flag_listen {
        send_message(&mut socket, ServerRequest::TcpListen { port })?;
    }

    Ok(())
}

fn start_headless(server_bin_path: &Path, socket_path: &Path) -> Result<UnixStream, String> {
    let command = Command::new(server_bin_path)
        .env("ISOBAR_SOCKET_PATH", socket_path)
        .env("ISOBAR_HEADLESS", "1")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to open Isobar app {}", error))?;

    let mut stdout = command.stdout.unwrap();
    let mut reader = BufReader::new(&mut stdout);
    let mut line = String::new();
    while line != "Listening\n" {
        reader
            .read_line(&mut line)
            .map_err(|_| String::from("Error reading app output"))?;
    }
    UnixStream::connect(socket_path).map_err(|_| String::from("Error connecting to socket"))
}

fn start_electron(
    src_path: &Path,
    server_bin_path: &Path,
    socket_path: &Path,
    node_env: &str,
) -> Result<UnixStream, String> {
    let electron_app_path = Path::new(src_path).join("isobar_electron");
    let electron_bin_path = electron_app_path.join("node_modules/.bin/electron");
    let command = Command::new(electron_bin_path)
        .arg(electron_app_path)
        .env("ISOBAR_SERVER_PATH", server_bin_path)
        .env("ISOBAR_SOCKET_PATH", socket_path)
        .env("ISOBAR_HEADLESS", "0")
        .env("NODE_ENV", node_env)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to open Isobar app {}", error))?;

    let mut stdout = command.stdout.unwrap();
    let mut reader = BufReader::new(&mut stdout);
    let mut line = String::new();
    while line != "Listening\n" {
        reader
            .read_line(&mut line)
            .map_err(|_| String::from("Error reading app output"))?;
    }
    UnixStream::connect(socket_path).map_err(|_| String::from("Error connecting to socket"))
}

fn send_message(socket: &mut UnixStream, message: ServerRequest) -> Result<(), String> {
    let bytes = serde_json::to_vec(&message).expect("Error serializing message");
    socket
        .write(&bytes)
        .expect("Error writing to server socket");
    socket.write(b"\n").expect("Error writing to server socket");

    let mut reader = BufReader::new(socket);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("Error reading server response");
    match serde_json::from_str::<ServerResponse>(&line).expect("Error reading server response") {
        ServerResponse::Ok => Ok(()),
        ServerResponse::Error { description } => Err(description),
    }
}
