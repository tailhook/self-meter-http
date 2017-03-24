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

use tokio_core::net::{TcpListener};
use futures::{Future, Stream};
use futures::future::{ok};

use tk_http::server::buffered::{BufferedDispatcher};
use tk_http::server::{self, Proto};
use tk_listen::ListenExt;
use tk_easyloop::handle;


fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init().expect("init logging");

    let meter = self_meter_http::Meter::new();
    meter.track_current_thread_by_name();

    tk_easyloop::run_forever(|| -> Result<(), ()> {
        meter.spawn_scanner(&handle());

        let addr = "0.0.0.0:8080".parse().unwrap();
        let scfg = server::Config::new().done();
        let listener = TcpListener::bind(&addr, &handle()).unwrap();

        tk_easyloop::spawn(listener.incoming()
            .sleep_on_error(Duration::from_millis(100), &handle())
            .map(move |(socket, addr)| {
                let meter = meter.clone();
                Proto::new(socket, &scfg,
                    BufferedDispatcher::new(addr, &handle(), move || {
                        let meter = meter.clone();
                        move |_req, e| ok(meter.respond(e))
                    }),
                    &handle())
                .map_err(|e| { debug!("Connection error: {}", e); })
            })
            .listen(10));
        Ok(())
    }).unwrap();
}
