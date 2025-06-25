# Gossip Protocols

![image](https://github.com/user-attachments/assets/d4373f50-91d3-40e9-a7c2-74c2af5d032a)


This is an implementation of a simple Gossip Protocol which works with [Maelstrom](https://github.com/jepsen-io/maelstrom), and was part of the demo at my talk at [Rootconf 2025 on Gossip Protocols](https://hasgeek.com/rootconf/2025/schedule/rumor-has-it-understanding-gossip-protocols-for-eventual-consistency-2hXBN5TtaPhwWDNG3gtY2E)

## Features

- Gossip Protocol Implementation that is resilient to network partitions
- [Maelstrom Runtime](https://github.com/shrirambalaji/maelstrom-node) with better error handling and refactoring.

## Setup 

First, initialize and pull in the required submodules:

```sh
git submodule update --init
```

### Prerequisites

**Windows users:** Install Java 11 using winget:
```powershell
winget install Microsoft.OpenJDK.11
```

**Note for Windows:** Maelstrom requires the ability to create symbolic links. Either:
- Run PowerShell as Administrator, OR
- Enable Developer Mode in Windows Settings → Update & Security → For developers

Then build the project:

```sh
cargo run build
```

## Running the Maelstrom Test

**Linux/macOS:**
```sh
./bin/maelstrom test -w broadcast --bin ./target/debug/gossip --node-count 10 --time-limit 20 --rate 10 --log-stderr
```

**Windows (PowerShell):**
```powershell
.\bin\maelstrom.ps1 test -w broadcast --bin .\target\debug\gossip.exe --node-count 10 --time-limit 20 --rate 10 --log-stderr
```

## Simulating a network partition

Maelstrom has a `--nemesis` flag, that allows introducing network partitions. This tests the `retry` workflow in our `GossipNode` implementation:

**Linux/macOS:**
```sh
./bin/maelstrom test -w broadcast --bin ./target/debug/gossip --node-count 10 --time-limit 20 --rate 10 --log-stderr --nemesis partition
```

**Windows (PowerShell):**
```powershell
.\bin\maelstrom.ps1 test -w broadcast --bin .\target\debug\gossip.exe --node-count 10 --time-limit 20 --rate 10 --log-stderr --nemesis partition
```
