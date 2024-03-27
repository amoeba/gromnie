use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct ConnectRequestHeader {
    server_time: f64,
    pub cookie: u64,
    net_id: i32,
    outgoing_seed: u32,
    incoming_seed: u32,
    unknown: u32,
}
