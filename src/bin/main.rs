//!
//! # WebSocket Serial Server
//!
//! WebSocket Serial Server is a program that allows browsers to access serial ports on localhost.
//!
//! ## Running
//!
//! ```./wsss```
//!
//! For information on configuration please check out the [cfg](../lib/cfg/index.html)
//! package in lib/.
//!
//! You can open/close ports, read and write data, and see the responses. All messages are in JSON
//! format.
//!
//! TODO: Add status message websocket channel to accept broadcasts of any errors / transmission
//! problems.

extern crate actix;
extern crate actix_web;
extern crate argparse;
extern crate base64;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rand;
extern crate serde_json;
extern crate serialport;

extern crate lib;

use actix::prelude::*;
use actix_web::{http, HttpServer, App, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use lib::cfg::*;
use lib::serial_port_arbiter::*;
use lib::websocket_client_actor::*;

/// Max number of failures we allow when trying to send
/// data to client before exiting
/// TODO: Make configurable
pub const MAX_SEND_ERROR_COUNT: u32 = 5;

// HTML Template
const WEBSOCKET_HTML: &str = include_str!("./websockets.html");

fn index_handler(request: &HttpRequest) -> HttpResponse {
  HttpResponse::Ok()
    .content_type("text/html")
    .body(request.state())
}

fn websocket_handler(req: &HttpRequest<>) -> Result<HttpResponse, Error> {
  ws::start(WebsocketClientActor::default(), req, WebsocketClientActor::default())
}

/// Launches wsss
pub fn main() {
  // Init logger
  env_logger::try_init().expect("Initialization of logging system failed!");

  // Grab config
  let cfg = WsssConfig::load();

  info!("Using port {}", cfg.http_port);

  // Start Actix runtime
  let system = actix::System::new("wsss");

  // Start chat server actor in separate thread
  let arbiter = Arbiter::new();
  let serial_port_arbiter_address = SerialPortArbiter::start_in_arbiter(&arbiter, |_ctx: &mut Context<SerialPortArbiter>| SerialPortArbiter::default());

  // Build HTTP Server.
  HttpServer::new(move || {
    vec![
      // Index html
      App::with_state(WebsocketClientState {
        serial_port_arbiter_address: serial_port_arbiter_address.clone(),
      })
      .prefix("/ws")
      .resource("", |r| r.route().f(websocket_handler))
      .resource("/", |r| r.route().f(websocket_handler))
      .boxed(),
      App::with_state(WEBSOCKET_HTML.to_string().clone())
        .prefix("/")
        .resource("", |r| r.method(http::Method::GET).f(index_handler))
        .resource("/", |r| r.method(http::Method::GET).f(index_handler))
        .resource("/index.html", |r| {
          r.method(http::Method::GET).f(index_handler)
        })
        .boxed(),
    ]
  })
  .bind(format!("{}:{}", cfg.bind_address, cfg.http_port))
  .expect(
    format!(
      "Cannot start server on {}:{}",
      cfg.bind_address, cfg.http_port
    )
    .as_str(),
  )
  .shutdown_timeout(15)
  .start();

  let _ = system.run();
}
