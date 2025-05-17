use crate::request::Request;
use async_trait::async_trait;
use log::info;
use maelstrom::protocol::Message;
use maelstrom::{Node as MaelstromNode, Result, Runtime};
use rand::rng;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, sleep};

/// The number of random peers to select for gossiping.
const RANDOM_PEER_COUNT: usize = 3;

#[derive(Clone, Default)]
pub struct GossipNode {
    pub state: Arc<Mutex<NodeState>>,
}
impl GossipNode {
    pub fn new(neighbours: Vec<String>) -> Self {
        GossipNode {
            state: Arc::new(Mutex::new(NodeState {
                seen: HashSet::new(),
                neighbours,
                pending: HashMap::new(),
            })),
        }
    }
}

#[derive(Clone, Default)]
pub struct NodeState {
    pub seen: HashSet<u64>,
    pub neighbours: Vec<String>,
    pub pending: HashMap<u64, HashSet<String>>, // NEW: message -> unacked peers
}

#[async_trait]
impl MaelstromNode for GossipNode {
    async fn process(&self, runtime: Runtime, req: Message) -> Result<()> {
        let msg: Result<Request> = req.body.as_obj();
        match msg {
            Ok(Request::Read {}) => {
                let data = self.snapshot();
                let msg = Request::ReadOk { messages: data };
                return runtime.reply(req, msg).await;
            }
            Ok(Request::Broadcast { message }) => {
                let sender = req.src.clone();
                let mut should_retry = false;
                if self.try_add(message) {
                    let mut st = self.state.lock().unwrap();
                    let mut unacked: HashSet<String> = st.neighbours.iter().cloned().collect();
                    unacked.remove(&sender);
                    st.pending.insert(message, unacked.clone());
                    should_retry = true;
                }
                runtime.reply_ok(req).await?;
                if should_retry {
                    self.retry(runtime.clone(), message);
                }
                return Ok(());
            }
            Ok(Request::Topology { topology }) => {
                let neighbours = topology.get(runtime.node_id()).unwrap();
                self.state.lock().unwrap().neighbours = neighbours.clone();
                info!("My neighbours are {:?}", neighbours);
                return runtime.reply_ok(req).await;
            }
            _ => runtime.exit(req),
        }
    }
}

impl GossipNode {
    fn retry(&self, runtime: Runtime, msg: u64) {
        let node = self.clone();
        tokio::spawn(async move {
            loop {
                {
                    let st = node.state.lock().unwrap();
                    if st.pending.get(&msg).map_or(true, |un| un.is_empty()) {
                        break;
                    }
                    let targets: Vec<String> =
                        st.pending.get(&msg).unwrap().iter().cloned().collect();
                    drop(st);
                    for dest in targets {
                        runtime.execute_rpc(dest, Request::Broadcast { message: msg });
                    }
                }
                sleep(Duration::from_secs(1)).await;
            }
            node.state.lock().unwrap().pending.remove(&msg);
        });
    }

    fn snapshot(&self) -> Vec<u64> {
        self.state.lock().unwrap().seen.iter().copied().collect()
    }

    fn try_add(&self, value: u64) -> bool {
        let mut state = self.state.lock().unwrap();
        if !state.seen.contains(&value) {
            state.seen.insert(value);
            return true;
        }
        false
    }
}
