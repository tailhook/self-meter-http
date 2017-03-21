extern crate tokio_core;
extern crate futures;
extern crate tk_http;
extern crate tk_listen;
extern crate tk_easyloop;
extern crate self_meter_http;
extern crate env_logger;

#[macro_use] extern crate log;

use std::env;
use std::time::Duration;

use tokio_core::io::Io;
use tokio_core::net::{TcpListener};
use futures::{Future, Stream};
use futures::future::{FutureResult, ok};

use tk_http::{Status};
use tk_http::server::buffered::{Request, BufferedDispatcher};
use tk_http::server::{self, Encoder, EncoderDone, Proto, Error};
use tk_listen::ListenExt;
use tk_easyloop::handle;

const BODY: &'static str = "Hello World!";

fn service<S:Io>(_: Request, mut e: Encoder<S>)
    -> FutureResult<EncoderDone<S>, Error>
{
    e.status(Status::Ok);
    e.add_length(BODY.as_bytes().len() as u64).unwrap();
    e.add_header("Server", "tk-listen/http/example").unwrap();
    if e.done_headers().unwrap() {
        e.write_body(BODY.as_bytes());
    }
    ok(e.done())
}


fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init().expect("init logging");

    let meter = self_meter_http::Meter::new();

    tk_easyloop::run_forever(|| -> Result<(), ()> {
        meter.spawn_scanner(&handle());

        let addr = "0.0.0.0:8080".parse().unwrap();
        let scfg = server::Config::new().done();
        let listener = TcpListener::bind(&addr, &handle()).unwrap();

        tk_easyloop::spawn(listener.incoming()
            .sleep_on_error(Duration::from_millis(100), &handle())
            .map(move |(socket, addr)| {
                Proto::new(socket, &scfg,
                    BufferedDispatcher::new(addr, &handle(), || service),
                    &handle())
                .map_err(|e| { debug!("Connection error: {}", e); })
            })
            .listen(10));
        Ok(())
    }).unwrap();
}
