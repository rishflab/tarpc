// Copyright 2018 Google LLC
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(
    futures_api,
    pin,
    arbitrary_self_types,
    await_macro,
    async_await
)]

use clap::{App, Arg};
use futures::{
    compat::TokioDefaultSpawner,
    future::{self, Ready},
    prelude::*,
};
use std::{io, net::SocketAddr};
use tarpc::{
    context,
    server::{Handler, Server},
};

// This is the type that implements the generated Service trait. It is the business logic
// and is used to start the server.
#[derive(Clone)]
struct HelloServer;

impl service::Service for HelloServer {
    // Each defined rpc generates two items in the trait, a fn that serves the RPC, and
    // an associated type representing the future output by the fn.

    type HelloFut = Ready<String>;

    fn hello(self, _: context::Context, name: String) -> Self::HelloFut {
        future::ready(format!("Hello, {}!", name))
    }
}

async fn run(server_addr: SocketAddr) -> io::Result<()> {
    // bincode_transport is provided by the associated crate bincode-transport. It makes it easy
    // to start up a serde-powered bincode serialization strategy over TCP.
    let transport = bincode_transport::listen(&server_addr)?;

    // The server is configured with the defaults.
    let server = Server::default()
        // Server can listen on any type that implements the Transport trait.
        .incoming(transport)
        // serve is generated by the service! macro. It takes as input any type implementing
        // the generated Service trait.
        .respond_with(service::serve(HelloServer));

    await!(server);

    Ok(())
}

fn main() {
    let flags = App::new("Hello Server")
        .version("0.1")
        .author("Tim <tikue@google.com>")
        .about("Say hello!")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("NUMBER")
                .help("Sets the port number to listen on")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let port = flags.value_of("port").unwrap();
    let port = port
        .parse()
        .unwrap_or_else(|e| panic!(r#"--port value "{}" invalid: {}"#, port, e));

    tarpc::init(TokioDefaultSpawner);

    tokio::run(
        run(([0, 0, 0, 0], port).into())
            .map_err(|e| eprintln!("Oh no: {}", e))
            .boxed()
            .compat(),
    );
}
