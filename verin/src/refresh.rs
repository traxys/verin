use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::spawn,
};

pub fn refresh_server(refresh_port: u16, request_port: u16) -> color_eyre::Result<()> {
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
                                    if let Err(e) = ws.send(tungstenite::Message::text("xxx")) {
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

    Ok(())
}

pub fn trigger_refresh(request_port: u16) -> color_eyre::Result<()> {
    let mut stream = TcpStream::connect(("0.0.0.0", request_port))?;
    stream.write_all(b"xxx")?;

    Ok(())
}
