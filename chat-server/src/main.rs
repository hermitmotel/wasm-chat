#[macro_use]
extern crate log;
extern crate wasi_common;
extern crate wasmtime;
extern crate wasmtime_wasi;
extern crate websocket;

use std::io::Cursor;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::vec::Vec;
use wasi_common::virtfs::pipe::{ReadPipe, WritePipe};
use wasmtime::*;
use wasmtime_wasi::{Wasi, WasiCtxBuilder};
use websocket::sync::Server;
use websocket::OwnedMessage;

fn main() {
    env_logger::init();

    let (dispatcher_tx, dispatcher_rx) = mpsc::channel::<String>();
    let client_senders: Arc<Mutex<Vec<mpsc::Sender<websocket::OwnedMessage>>>> =
        Arc::new(Mutex::new(vec![]));

    // bind to the server
    let server = Server::bind("127.0.0.1:2794").unwrap();

    // Dispatcher thread
    {
        let client_senders = client_senders.clone();
        thread::spawn(move || {
            let pipe_stdout = Arc::new(RwLock::new(Cursor::new(vec![])));
            let pipe_stdin = Arc::new(RwLock::new(Cursor::new(vec![])));

            let store = Store::default();
            let mut linker = Linker::new(&store);
            let wasi_ctx = WasiCtxBuilder::new()
                .inherit_args()
                .inherit_env()
                .stdin(ReadPipe::from_shared(pipe_stdin.clone()))
                .stdout(WritePipe::from_shared(pipe_stdout.clone()))
                .inherit_stderr()
                .build()
                .expect("Unable to build WasiCtx");
            let wasi = Wasi::new(&store, wasi_ctx);
            wasi.add_to_linker(&mut linker)
                .expect("Unable to add linker");

            // Instantiate our module with the imports we've created, and run it.
            let module = Module::from_file(store.engine(), "plugin.wasm")
                .expect("Unable to load plugin.wasm");
            linker.module("", &module).unwrap();
            let add_emoji = linker
                .get_one_by_name("", "add_emoji")
                .expect("Unable to find symbol")
                .into_func()
                .expect("Unable to convert into a function")
                .get0::<()>()
                .expect("Unable to specify the signature");

            while let Ok(msg) = dispatcher_rx.recv() {
                debug!("Dispatcher thread receive message: {}", msg);

                {
                    let mut guard = pipe_stdin.write().expect("Unable to get lock");

                    guard.get_mut().clear();
                    guard.get_mut().extend_from_slice(msg.as_bytes());
                    guard.set_position(0);
                }

                add_emoji().expect("Unable to call add_emoji");

                let new_msg = {
                    let mut guard = pipe_stdout.write().expect("Unable to get lock");

                    let msg = String::from_utf8(guard.get_ref().to_vec())
                        .expect("Unable to create a string");
                    guard.get_mut().clear();
                    guard.set_position(0);

                    msg
                };

                for sender in client_senders.lock().expect("Unable to get lock").iter() {
                    let message = OwnedMessage::Text(new_msg.clone());
                    let _ = sender.send(message);
                }
            }
        });
    }

    for request in server.filter_map(Result::ok) {
        let dispatcher = dispatcher_tx.clone();
        let (client_tx, client_rx) = mpsc::channel();
        client_senders.lock().unwrap().push(client_tx.clone());

        // Spawn a new thread for each connection.
        thread::spawn(move || {
            if !request.protocols().contains(&"rust-websocket".to_string()) {
                request.reject().unwrap();
                return;
            }

            let client = request.use_protocol("rust-websocket").accept().unwrap();

            let ip = client.peer_addr().unwrap();

            debug!("Establish connection to {}", ip);

            let (mut receiver, mut sender) = client.split().unwrap();

            // thread to handle all outgoing messages
            thread::spawn(move || {
                while let Ok(msg) = client_rx.recv() {
                    if sender.send_message(&msg).is_err() {
                        return;
                    }
                }
            });

            for message in receiver.incoming_messages() {
                let message = message.unwrap();

                match message {
                    OwnedMessage::Close(_) => {
                        let message = OwnedMessage::Close(None);
                        client_tx.send(message).unwrap();
                        debug!("Client {} disconnected", ip);
                        return;
                    }
                    OwnedMessage::Ping(ping) => {
                        let message = OwnedMessage::Pong(ping);
                        client_tx.send(message).unwrap();
                    }
                    OwnedMessage::Text(msg) => {
                        dispatcher.send(msg).unwrap();
                    }
                    _ => {}
                }
            }
        });
    }
}
