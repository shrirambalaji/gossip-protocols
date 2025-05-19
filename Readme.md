# Gossip Protocols

![image](https://github.com/user-attachments/assets/2c20c9d1-cdbe-4472-87c6-df9483c8ff52)


This is an implementation of a simple Gossip Protocol which works with [Maelstrom](https://github.com/jepsen-io/maelstrom), and was part of the demo at my talk at [Rootconf 2025 on Gossip Protocols](https://hasgeek.com/rootconf/2025/schedule/rumor-has-it-understanding-gossip-protocols-for-eventual-consistency-2hXBN5TtaPhwWDNG3gtY2E)

## Features

- Gossip Protocol Implementation that is resilient to network partitions
- [Maelstrom Runtime](https://github.com/shrirambalaji/maelstrom-node) with better error handling and refactoring.

## Setup 

```sh
cargo run build
```

## Running the Maelstrom Test

```sh
./bin/maelstrom test -w broadcast --bin ./target/debug/gossip --node-count 10 --time-limit 20 --rate 10 --log-stderr
```

## Simulating a network partition

Maelstrom has a `--nemesis` flag, that allows introducing network partitions. This tests the `retry` workflow in our `GossipNode` implementation:

```sh
./bin/maelstrom test -w broadcast --bin ./target/debug/gossip --node-count 10 --time-limit 20 --rate 10 --log-stderr --nemesis partition
```
