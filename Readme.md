# Gossip Protocols

This is an implementation of a simple Gossip Protocol which works with Maelstrom. 

# Setup 

```sh
cargo run build
```

# Running the Maelstrom Test

```sh
./bin/maelstrom test -w broadcast --bin ./target/debug/gossip --node-count 2 --time-limit 20 --rate 10 --log-stderr
```
