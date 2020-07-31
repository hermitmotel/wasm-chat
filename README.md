# Chat Demo

## Build and start demo

- Install Rust toolchain
- Install Node.js
- Install [wasm-pack](https://github.com/rustwasm/wasm-pack)
- Start chat server in directory `chat-server`
  - `cargo run`
- Start web server in directory `chat-frontend`
  - `npm start`
- Open `http://localhost:8080/` in a browser 

## Building plugin

The server loads a plugin as WebAssembly.
The binary is already stored in chat-server.
To build the plugin, switch to the directory `plugin` and start the build process with the command `cargo build`.
