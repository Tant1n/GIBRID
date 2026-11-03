# NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


# (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.# GIBRID Native

Unofficial native Rust client targeting **Minecraft Java Edition 26.3**. Not affiliated with Mojang or Microsoft.

This repository is **Phase 0**: workspace, version manifest, coordinates, NBT/region limits, and a process that starts, logs, and exits. It is not a playable game.

Documentation sources live in `C:\Users\user\Desktop\GIBRID` (not copied into this repo wholesale).

## What this is / is not

- **Is:** a from-scratch native core (ADR-0014). wgpu is the planned renderer.
- **Is not:** a Minecraft launcher, Fabric Probe, Java mod loader, or a Horizon fork.
- **Does not** ship game JARs, textures, or worlds. Point GIBRID at a legal local install later.

## Build

Requirements: Rust 1.99.1 (`rust-toolchain.toml`) and the MSVC C++ build tools on Windows.

```text
cargo test --workspace
cargo run -p gibrid-app
```

`gibrid-app` prints the 26.3 manifest. Protocol 777 is **unverified**. Data Version is **unset** until extracted from the installed game.

## Status

See [docs/status.md](docs/status.md). Vocabulary: SPECIFIED, SCAFFOLDED, IMPLEMENTED, TESTED, COMPATIBLE, BENCHMARKED.

A stub is not implemented. A unit test is not parity.

