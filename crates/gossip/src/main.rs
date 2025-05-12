use maelstrom::{Result, Runtime};
use std::sync::Arc;

pub mod node;
pub mod request;
use crate::node::GossipNode;

#[tokio::main]
pub(crate) async fn main() -> Result<()> {
    let node = Arc::new(GossipNode::default());
    Runtime::new().with_node(node).run().await
}
