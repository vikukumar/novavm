# Architecture Overview

## Crate Dependency Graph

```
sdk        (no internal deps — public plugin API)
agent      (no internal deps — guest protocol)
hypervisor (no internal deps — platform trait)
scheduler  (no internal deps)
memory     (no internal deps)
storage    (no internal deps)
network    (no internal deps)
engine     ──► hypervisor, scheduler, memory, storage, network
snapshot   ──► engine, storage
monitor    ──► (none — uses sysinfo directly)
api        ──► engine, hypervisor, scheduler, memory, storage, network, snapshot, monitor
src-tauri  ──► api, engine, monitor, storage, network, snapshot, hypervisor
```

## Engine State Machine

```
Stopped ──► Starting ──► Running ──► Paused
   ▲                        │           │
   └──────────── Stopped ◄──┘           │
   ▲                                    │
   └─────────────────── Stopped ◄───────┘
Crashed ──► Starting (restart)
```

## NovaDisk Format

```
Offset  Size    Field
0       8       Magic: "NOVADISK"
8       4       Format version (u32 LE)
12      8       Header JSON length (u64 LE)
20      var     Header JSON (UTF-8)
var     var     Cluster map (cluster_count × ClusterEntry)
var     var     Data clusters (cluster_size × cluster_count)
```

Each cluster is independently encrypted (AES-256-GCM) and/or compressed (zstd).
The cluster map contains refcounts supporting copy-on-write for snapshots.

## Virtual Network Modes

| Mode      | Internet | Host access | Inter-VM | Physical network |
|-----------|----------|-------------|----------|-----------------|
| NAT       | ✅       | ✅          | ✅       | ❌ (behind NAT) |
| Bridged   | ✅       | ✅          | ✅       | ✅              |
| Host-only | ❌       | ✅          | ✅       | ❌              |
| Internal  | ❌       | ❌          | ✅       | ❌              |
