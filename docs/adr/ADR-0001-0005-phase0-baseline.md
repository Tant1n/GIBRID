# ADR-0001…0005 — Phase 0 working baseline

Status: ACCEPTED as implementation baseline when this repository was created (2026-10-29), taken from `GIBRID_DECISION_LOG_ADR.md` (previously PROPOSED).

- **0001** First milestone: local single-player client with a test world. Multiplayer, dedicated server, and shared engine are later milestones.
- **0002** Official Mojang/Microsoft auth is unsupported in v0.x of the native core. No credentials requested, stored, or logged.
- **0003** Primary platform: Windows x64. Linux later with CI. macOS not promised. wgpu backend is whatever wgpu selects; log backend/adapter/features/limits when a device exists.
- **0004** Stable Rust, version pinned in `rust-toolchain.toml` (1.99.1 at repo start). `Cargo.lock` is committed. wgpu version will be pinned in Phase B, not now.
- **0005** Main thread: window/input later. Simulation thread: 20 tps / 50 ms with catch-up cap. Worker pool later. Bounded queues. No async runtime in core. Renderer reads immutable snapshots only.

ADR-0008 (project license) remains OPEN; repository is private/`UNLICENSED`.
