use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use futures::stream::Stream;
use self_meter;
use tokio_core::reactor::{Handle, Interval};

/// A wrapper around original ``self_meter::Meter`` that locks internal
/// mutex on most operations and maybe used in multiple threads safely.
#[derive(Clone)]
pub struct Meter(Arc<Mutex<self_meter::Meter>>);


impl Meter {

    /// Create a new meter with specified scan interval
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
}
