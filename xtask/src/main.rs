use std::{
    ffi::OsString,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{Arc, Mutex},
};

use clap::{CommandFactory, Parser};
use duct::cmd;
use std::thread::spawn;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    task: Option<Task>,
}

#[derive(Parser)]
enum Task {
    Build {
        #[clap(long, short)]
        debug: bool,
    },
    RefreshServer {
        #[clap(long, short, default_value = "4111")]
        refresh_port: u16,
        #[clap(long, short = 'p', default_value = "4112")]
        request_port: u16,
    },
    BuildRefresh {
        #[clap(long, short, default_value = "4112")]
        request_port: u16,
    },
    Watch {
        #[clap(long, short, default_value = "4111")]
        refresh_port: u16,
        #[clap(long, short = 'p', default_value = "4112")]
        request_port: u16,
    },
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .ancestors()
        .nth(1)
        .expect("could not get workspace root");

    let args = Args::from_args();
    match args.task {
        None => Args::command().print_help()?,
        Some(task) => match task {
            Task::Build { debug } => {
                let posts = workspace.join("posts");
                let mode = if debug { "debug" } else { "release" };
                let target = workspace.join(format!("target/{mode}/html"));
                let mut args: Vec<_> = vec![
                    OsString::from("run"),
                    OsString::from("--release"),
                    OsString::from("--"),
                    OsString::from("build"),
                    posts.into_os_string(),
                    target.into_os_string(),
                ];
                if debug {
                    args.push("--debug".into());
                }

                duct::cmd(env!("CARGO"), args).dir(workspace).run()?;
            }
            Task::RefreshServer {
                refresh_port,
                request_port,
            } => {
                let bus = Arc::new(Mutex::new(bus::Bus::new(128)));

                let refresh_listener = TcpListener::bind(("0.0.0.0", refresh_port))?;
                let b = bus.clone();
                spawn(move || {
                    for stream in refresh_listener.incoming() {
                        println!("Websocket request");
                        match stream {
                            Err(e) => eprintln!("Error in refresher: {:?}", e),
                            Ok(s) => {
                                let mut r = b.lock().unwrap().add_rx();
                                match tungstenite::accept(s) {
                                    Err(e) => eprintln!("Error in websocket accept: {e:?}"),
                                    Ok(mut ws) => {
                                        spawn(move || {
                                            while r.recv().is_ok() {
                                                println!("Request taken into account");
                                                if let Err(e) = ws.write_message(
                                                    tungstenite::Message::text("xxx"),
                                                ) {
                                                    println!("WS error: {e:?}");
                                                    return;
                                                };
                                            }
                                            println!("Finished websocket")
                                        });
                                    }
                                }
                            }
                        }
                    }
                });

                println!("Started request_listener");
                let request_listener = TcpListener::bind(("0.0.0.0", request_port))?;

                for stream in request_listener.incoming() {
                    let mut stream = stream?;
                    let buf = &mut [0, 0, 0];
                    stream.read_exact(buf)?;

                    if buf != b"xxx" {
                        eprintln!("Invalid request on request_port");
                    }

                    bus.lock().unwrap().broadcast(());
                    println!("Refresh Requested");
                }
            }
            Task::BuildRefresh { request_port } => {
                cmd!(env!("CARGO"), "xtask", "build", "--debug").run()?;

                let mut stream = TcpStream::connect(("0.0.0.0", request_port))?;
                stream.write_all(b"xxx")?;
            }
            Task::Watch {
                refresh_port,
                request_port,
            } => {
                cmd!(env!("CARGO"), "xtask", "build", "--debug").run()?;

                cmd!(
                    env!("CARGO"),
                    "xtask",
                    "refresh-server",
                    "-r",
                    &refresh_port.to_string(),
                    "-p",
                    &request_port.to_string()
                )
                .start()?;

                cmd!(
                    env!("CARGO"),
                    "watch",
                    "-x",
                    format!("xtask build-refresh -r {request_port}"),
                )
                .dir(workspace)
                .run()?;
            }
        },
    }
    Ok(())
}
