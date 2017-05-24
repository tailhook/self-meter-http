use std::io::Write;

use serde_json;
use serde::{Serialize, Serializer};
use serde::ser::{SerializeStruct, SerializeMap};
use self_meter::Meter;


pub struct ReportWrapper<'a> {
    pub meter: &'a Meter,
}

pub struct ThreadIter<'a>(pub &'a Meter);


pub fn serialize<W: Write>(meter: &Meter, mut buf: W) {
    serde_json::to_writer(&mut buf,
        &ReportWrapper {
            meter: meter,
        },
    ).expect("self report is serializable");
}

impl<'a> Serialize for ReportWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut struc = serializer.serialize_struct("SelfMeter",  2)?;
        struc.serialize_field("process", &self.meter.report())?;
        struc.serialize_field("threads", &ThreadIter(self.meter))?;
        struc.end()
    }
}

impl<'a> Serialize for ThreadIter<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        // TODO(tailhook) use fixed size serializer when things are properly
        // exposed in `self-meter`
        let mut map = serializer.serialize_map(None)?;
        if let Some(threads) = self.0.thread_report() {
            for (name, ref report) in threads {
                map.serialize_key(name)?;
                map.serialize_value(report)?;
            }
        }
        map.end()
    }
}
