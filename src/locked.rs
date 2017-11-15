use std::fmt;
use std::io::{Write, BufWriter};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use std::thread;

use futures::stream::Stream;
use self_meter;
use serde::{Serializer, Serialize};
use tokio_core::reactor::{Handle, Interval};
use tk_http::{Status};
use tk_http::server::{Encoder, EncoderDone};

use json::{serialize, ReportWrapper, ThreadIter};
use self_meter::Pid;
use routing::{Route, Error as RError};
use data;

/// A wrapper around original ``self_meter::Meter`` that locks internal
/// mutex on most operations and maybe used in multiple threads safely.
#[derive(Clone)]
pub struct Meter(Arc<Mutex<self_meter::Meter>>);

/// A serializable report structure
///
/// Use `Meter::report()` to get instance.
///
/// Currently the structure is fully opaque except serialization
#[derive(Debug)]
pub struct Report<'a>(&'a Meter);

/// A serializable thread report structure
///
/// The structure serializes to a map of thread reports.
/// Use `Meter::thread_report()` to get instance.
///
/// Currently the structure is fully opaque except serialization
#[derive(Debug)]
pub struct ThreadReport<'a>(&'a Meter);

/// A serializable process report structure
///
/// The structure serializes to a structure of whole-process metrics.
/// Use `Meter::process_report()` to get instance.
///
/// Currently the structure is fully opaque except serialization
#[derive(Debug)]
pub struct ProcessReport<'a>(&'a Meter);

impl Meter {

    /// Create a new meter with specified scan iterval of one second
    pub fn new() -> Meter {
        let inner = self_meter::Meter::new(Duration::new(1, 0))
            .expect("self-meter should be created successfully");
        Meter(Arc::new(Mutex::new(inner)))
    }

    /// Adds a scanner coroutine to tokio main loop
    ///
    /// This must be called once per process (not per thread or tokio loop)
    pub fn spawn_scanner(&self, handle: &Handle) {
        let meter = self.clone();
        handle.spawn(
            Interval::new(Duration::new(1, 0), handle)
            .expect("interval should work")
            .map_err(|e| panic!("interval error: {}", e))
            .map(move |()| {
                meter.lock().scan()
                .map_err(|e| error!("Self-meter scan error: {}", e)).ok();
            })
            .for_each(|()| Ok(())
        ));
    }

    fn lock(&self) -> MutexGuard<self_meter::Meter> {
        self.0.lock().expect("meter not poisoned")
    }


    /// Serialize response into JSON
    pub fn serialize<W: Write>(&self, buf: W) {
        serialize(&*self.lock(), buf)
    }

    /// Get serializable report
    pub fn report(&self) -> Report {
        Report(self)
    }

    /// Get serializable report for process data
    ///
    /// This is a part of `report()` / `Report`, and is needed for fine-grained
    /// serialization control.
    pub fn process_report(&self) -> ProcessReport {
        ProcessReport(self)
    }

    /// Get serializable report for thread data
    ///
    /// This is a part of `report()` / `Report`, and is needed for fine-grained
    /// serialization control.
    pub fn thread_report(&self) -> ThreadReport {
        ThreadReport(self)
    }

    /// Same as `serialize` but also adds required HTTP headers
    pub fn respond<S>(&self, mut e: Encoder<S>) -> EncoderDone<S> {
        e.status(Status::Ok);
        // TODO(tailhook) add date
        e.add_header("Server",
            concat!("self-meter-http/", env!("CARGO_PKG_VERSION"))
        ).unwrap();
        e.add_header("Content-Type", "application/json").unwrap();
        e.add_chunked().unwrap();
        if e.done_headers().unwrap() {
            self.serialize(BufWriter::new(&mut e))
        }
        e.done()
    }

    /// Similar to respond, but also serves static files if needed
    pub fn serve<S>(&self, route: Result<Route, RError>, mut e: Encoder<S>)
        -> EncoderDone<S>
    {
        match route {
            Ok(Route::Page) => {
                e.status(Status::Ok);
                // TODO(tailhook) add date
                e.add_header("Server",
                    concat!("self-meter-http/", env!("CARGO_PKG_VERSION"))
                ).unwrap();
                e.add_header("Content-Type", "text/html").unwrap();
                e.add_length(data::HTML.as_bytes().len() as u64).unwrap();
                if e.done_headers().unwrap() {
                    e.write_body(data::HTML.as_bytes());
                }
                e.done()
            }
            Ok(Route::Bundle) => {
                e.status(Status::Ok);
                // TODO(tailhook) add date
                e.add_header("Server",
                    concat!("self-meter-http/", env!("CARGO_PKG_VERSION"))
                ).unwrap();
                e.add_header("Content-Type",
                    "application/javascript").unwrap();
                e.add_length(data::BUNDLE.as_bytes().len() as u64).unwrap();
                if e.done_headers().unwrap() {
                    e.write_body(data::BUNDLE.as_bytes());
                }
                e.done()
            }
            Ok(Route::Stylesheet) => {
                e.status(Status::Ok);
                // TODO(tailhook) add date
                e.add_header("Server",
                    concat!("self-meter-http/", env!("CARGO_PKG_VERSION"))
                ).unwrap();
                e.add_header("Content-Type",
                    "text/stylesheet").unwrap();
                e.add_length(data::CSS.as_bytes().len() as u64).unwrap();
                if e.done_headers().unwrap() {
                    e.write_body(data::CSS.as_bytes());
                }
                e.done()
            }
            Ok(Route::StatusJson) => {
                e.status(Status::Ok);
                // TODO(tailhook) add date
                e.add_header("Server",
                    concat!("self-meter-http/", env!("CARGO_PKG_VERSION"))
                ).unwrap();
                e.add_header("Content-Type", "application/json").unwrap();
                e.add_chunked().unwrap();
                if e.done_headers().unwrap() {
                    self.serialize(BufWriter::new(&mut e))
                }
                e.done()
            }
            Ok(__Nonexhaustive) => unreachable!(),
            Err(err) => {
                e.status(match err {
                    RError::NotFound => Status::NotFound,
                    RError::MethodNotAllowed => Status::MethodNotAllowed,
                    _ => Status::InternalServerError,
                });
                // TODO(tailhook) add date
                e.add_header("Server",
                    concat!("self-meter-http/", env!("CARGO_PKG_VERSION"))
                ).unwrap();
                e.add_header("Content-Type", "application/json").unwrap();
                e.add_chunked().unwrap();
                if e.done_headers().unwrap() {
                    self.serialize(BufWriter::new(&mut e))
                }
                e.done()
            }
        }
    }

    /// Start tracking specified thread
    ///
    /// Note: you must add main thread here manually. Usually you
    /// should use `track_current_thread()` instead.
    pub fn track_thread(&self, tid: Pid, name: &str) {
        self.lock().track_thread(tid, name)
    }
    /// Stop tracking specified thread (for example if it's dead)
    pub fn untrack_thread(&self, tid: Pid) {
        self.lock().untrack_thread(tid)
    }
    /// Add current thread using `track_thread`, returns thread id
    pub fn track_current_thread(&self, name: &str) -> Pid {
        self.lock().track_current_thread(name)
    }
    /// Track current thread by using name from `std::thread::current().name()`
    ///
    /// This may not work if thread has no name, use `track_current_thread`
    /// if thread has no own name or if you're unsure.
    ///
    /// # Panics
    ///
    /// If no thread is set.
    pub fn track_current_thread_by_name(&self) {
        let thread = thread::current();
        let name = thread.name().expect("thread name must be set");
        self.lock().track_current_thread(name);
    }
    /// Remove current thread using `untrack_thread`
    pub fn untrack_current_thread(&self) {
        self.lock().untrack_current_thread();
    }
}

impl fmt::Debug for Meter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Meter")
        .finish()
    }
}

impl<'a> Serialize for Report<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        ReportWrapper { meter: &*self.0.lock() }.serialize(serializer)
    }
}

impl<'a> Serialize for ProcessReport<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.0.lock().report().serialize(serializer)
    }
}

impl<'a> Serialize for ThreadReport<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        ThreadIter(&*self.0.lock()).serialize(serializer)
    }
}
