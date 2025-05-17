use crate::request::Request;
use async_trait::async_trait;
use log::info;
use maelstrom::protocol::Message;
use maelstrom::{Node as MaelstromNode, Result, Runtime};
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
                unacked: HashMap::new(),
            })),
        }
    }
}

#[derive(Clone, Default)]
pub struct NodeState {
    // a list of all the seen MessageIds
    pub seen: HashSet<u64>,

    // all the neighbours to a node
    pub neighbours: Vec<String>,

    // a map of a messageId, and a unique set of neighbours who have not acknowleded it.
    pub unacked: HashMap<u64, HashSet<String>>,
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
                let sender: String = req.src.clone();
                if self.try_add(message) {
                    let mut state = self.state.lock().unwrap();

                    // before we send a message we move it to unacked
                    let mut neighbours: HashSet<String> =
                        state.neighbours.iter().cloned().collect();
                    neighbours.remove(&sender);

                    // unacked state used to actually send a message to those nodes
                    state.unacked.insert(message, neighbours.clone());
                    self.retry(runtime.clone(), message);
                }
                runtime.reply_ok(req).await?;
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

        // why is the background task necessary?
        // because we need to retry sending the message until all neighbours have acknowledged it.
        // if we don't do this, we will not be able to send the message to all neighbours.
        tokio::spawn(async move {
            loop {
                {
                    let state = node.state.lock().unwrap();
                    if state.unacked.get(&msg).map_or(true, |un| un.is_empty()) {
                        break;
                    }

                    // we have unacked messages, so we will retry
                    let mut neighbours: Vec<String> =
                        state.unacked.get(&msg).unwrap().iter().cloned().collect();

                    // a per-thread random number generator to shuffle the order of neighbours randomly
                    let mut rng = rand::rng();
                    neighbours.shuffle(&mut rng);

                    for neighbour in neighbours.into_iter().take(RANDOM_PEER_COUNT) {
                        runtime.execute_rpc(neighbour, Request::Broadcast { message: msg });
                    }
                }

                // we run this background loop every 1 second.
                sleep(Duration::from_secs(1)).await;
            }
            node.state.lock().unwrap().unacked.remove(&msg);
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
