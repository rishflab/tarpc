// Copyright 2018 Google LLC
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(
    underscore_imports,
    futures_api,
    pin,
    arbitrary_self_types,
    await_macro,
    async_await,
)]

use futures_legacy::{Stream as _, Sink as _};
use clap::{App, Arg};
use futures::{
    compat::{Stream01CompatExt, TokioDefaultSpawner},
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

    fn hello(self, _: context::Context, first: String, last: String) -> Self::HelloFut {
        future::ready(format!("Hello, {} {}!", first, last))
    }

    type AddFut = Ready<i32>;

    fn add(self, _: context::Context, x: i32, y: i32) -> Self::AddFut {
        future::ready(x + y)
    }
}

async fn run(server_addr: SocketAddr) -> io::Result<()> {
    // bincode_transport is provided by the associated crate bincode-transport. It makes it easy
    // to start up a serde-powered bincode serialization strategy over TCP.
    let io = tokio_tcp::TcpListener::bind(&server_addr)?;
    let transport = io.incoming().and_then(|io| {
        let peer_addr = io.peer_addr()?;
        let local_addr = io.local_addr()?;
        Ok(transport(io, peer_addr, local_addr))
    });

    // The server is configured with the defaults.
    let server = Server::default()
        // Server can listen on any type that implements the Transport trait.
        .incoming(transport.compat())
        // serve is generated by the service! macro. It takes as input any type implementing
        // the generated Service trait.
        .respond_with(service::serve(HelloServer));

    await!(server);

    Ok(())
}

fn transport<Item, SinkItem>(io: impl tokio_io::AsyncRead + tokio_io::AsyncWrite, local_addr: SocketAddr, peer_addr: SocketAddr)
    -> impl tarpc::transport::Transport<Item = Item, SinkItem = SinkItem>
where Item: for <'a> serde::Deserialize<'a>,
      SinkItem: serde::Serialize,
{
    let transport = tokio::codec::Framed::new(io, tokio::codec::LinesCodec::new())
        .and_then(|req| serde_json::from_str(&req).map_err(io_err))
        .with(|val| serde_json::to_string(&val).map_err(io_err));
    tarpc::transport::new(tarpc_compat::Compat::new(transport), local_addr, peer_addr)
}

fn io_err<E>(e: E) -> io::Error where E: std::error::Error + Send + Sync + 'static {
    log::warn!("Error in serialization: {}", e);
    io::Error::new(io::ErrorKind::Other, e)
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

    env_logger::init();
    tarpc::init(TokioDefaultSpawner);

    tokio::run(
        run(([0, 0, 0, 0], port).into())
            .map_err(|e| eprintln!("Oh no: {}", e))
            .boxed()
            .compat(),
    );
}