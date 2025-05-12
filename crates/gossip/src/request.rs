use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Request {
    Init {},
    Read {},
    ReadOk {
        messages: Vec<u64>,
    },
    Broadcast {
        message: u64,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
}
