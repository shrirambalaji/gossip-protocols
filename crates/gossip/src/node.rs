use crate::request::Request;
use async_trait::async_trait;
use log::info;
use maelstrom::protocol::Message;
use maelstrom::{Node as MaelstromNode, Result, Runtime, done};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct GossipNode {
    state: Arc<Mutex<NodeState>>,
}

#[derive(Clone, Default)]
pub struct NodeState {
    seen: HashSet<u64>,
    neighbours: Vec<String>,
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
            Ok(Request::Broadcast { message: element }) => {
                if self.try_add(element) {
                    info!("messages now {}", element);
                    for node in runtime.neighbours() {
                        runtime.execute_rpc(node, Request::Broadcast { message: element });
                    }
                }

                return runtime.reply_ok(req).await;
            }
            Ok(Request::Topology { topology }) => {
                let neighbours = topology.get(runtime.node_id()).unwrap();
                self.state.lock().unwrap().neighbours = neighbours.clone();
                info!("My neighbours are {:?}", neighbours);
                return runtime.reply_ok(req).await;
            }
            _ => done(runtime, req),
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
