use crate::request::Request;
use async_trait::async_trait;
use log::info;
use maelstrom::protocol::Message;
use maelstrom::{Node as MaelstromNode, Result, Runtime};
use rand::rng;
use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

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
            })),
        }
    }
}

#[derive(Clone, Default)]
pub struct NodeState {
    pub seen: HashSet<u64>,
    pub neighbours: Vec<String>,
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

            // Each request to a `broadcast` is a round.
            // A broadcast typically fans out a request to every N neighbours.
            // Here we are using a random sample of 3 neighbours.
            Ok(Request::Broadcast { message: element }) => {
                if self.try_add(element) {
                    let state = self.state.lock().unwrap();
                    let mut neighbours = state.neighbours.clone();
                    neighbours.shuffle(&mut rng());

                    for node in neighbours.into_iter().take(RANDOM_PEER_COUNT) {
                        runtime.execute_rpc(node, Request::Broadcast { message: element });
                    }
                }

                // We simply return an empty Ok response.
                return runtime.reply_ok(req).await;
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
