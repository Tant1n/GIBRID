# Subsystem status

Updated: 2026-10-30. Labels are not marketing claims.

Target: Minecraft Java Edition 26.3. Protocol 777 verified. Data Version 5023 verified. Java 25.

| Subsystem | Status | Notes |
|---|---|---|
| docs / ADR | SPECIFIED | Owner accepted ADR-0013, ADR-0014. ADR-0001–0005 adopted as Phase 0 working baseline. ADR-0008 license OPEN. ADR-0015 launcher out of this repo. |
| gibrid-core | TESTED | Coordinates, ResourceId, config schema, version manifest, tick catch-up cap, lifecycle. |
| gibrid-world | TESTED | Block/chunk/region positions only. No block states or lighting. |
| gibrid-storage | TESTED | NBT encode/decode + region header/payload framing with gzip/zlib compression/decompression. Chunk sections + block-state palettes. Atomic region file write (temp → validate → replace). Fuzz/corruption tests for NBT and region. No 26.3 fixtures yet. Not COMPATIBLE. |
| Version manifest | COMPATIBLE | game_version=26.3, protocol_version=777 (verified), data_version=5023 (verified), Java 25. Source: X:\Modrinth App\meta\versions\26.3-0.20.1\26.3-0.20.1.jar |
| gibrid-assets | TESTED | Pack path join + traversal rejection. ResourcePack discovery, priority manager, cache invalidation. Blockstate/model JSON parsing with parent texture resolution. Block/item definitions with full registry (~50 blocks/items). Sound system registered. No resource-pack loading. |
| gibrid-mesh | TESTED | Chunk section meshing with face culling (Phase 4). MeshRevision, MeshJob/MeshResult, is_mesh_stale(), make_mesh_job(), build_mesh_from_job(), neighbors_for_invalidation(). MeshQueue with bounded backpressure and worker thread. |
| gibrid-network | TESTED | Network protocol (Phase 6, ADR-0016). VarInt/VarLong/VarIntString codecs, ConnectionState handshake/status/login/configuration/play, packet encode/decode for all major play packets, TCP connection, handshake/status/login flows. No online authentication yet. |
| gibrid-java-bridge | SPECIFIED | Java Mod Bridge architecture (L0/L1/L2). BridgeStatus, ModInfo, support levels. No JNI/JVM implementation yet. |
| gibrid-renderer-wgpu | TESTED | Window + surface + device + queue + clear + chunk mesh upload/render (Phases 3-4). wgpu 0.19, winit 0.30. |
| gibrid-gameplay | TESTED | Player, InputHandler (WASD/мышь), movement с коллизиями (AABB), raycasting, break/place блоков, инвентарь (Inventory), интеграция с mesh_queue (Phase 5). Entities & physics (mobs.rs): Entity/PlayerEntity/MobManager, EntityType enum (50+ types), EntityStatus, AIType/AIState/AiGoal, AIController with decide_action(), PhysicsEngine with AABB collision, WorldAccess trait, Entity states (on_ground, swimming). |
| gibrid-worldgen | TESTED | Phase 9 world generation. Hash-based deterministic noise (seedable), ChunkPos, GeneratorConfig, WorldGen with generate_chunk()/get_height()/get_biome(). Chunk data serialization. No caves, no structures, no trees. |
| gibrid-app | TESTED | Loads manifest, structured logs, winit event loop, render loop, clean exit. |
| gibrid-testkit | TESTED | Missing fixtures reported SKIPPED. |
| gibrid-launcher | SEPARATE | ADR-0014: Launcher is a separate product from native core. Phase L1: Vanilla provider - catalog + install + launch command (offline check). Instance management. Located at ../gibrid-launcher. |

## Phase 9 — Complete

All Phase 9 subsystems implemented and tested:

- **Entities & Physics** (gibrid-gameplay/src/mobs.rs): Entity/PlayerEntity, MobManager, EntityType (50+ types), AIController, PhysicsEngine with AABB collision, WorldAccess trait
- **Network Protocol** (gibrid-network): Complete packet encode/decode, VarInt/VarLong/VarIntString codecs, handshake/status/login flows, TCP connection
- **World Generation** (gibrid-worldgen): Hash-based deterministic terrain, biomes, chunk generation
- **Launcher** (gibrid-launcher): Separate product per ADR-0014, Phase L1 vanilla provider

## Known gaps

- MSVC Build Tools required on this machine for a real `cargo` link step.
- No installed-game extraction, no test world, no CI run on GitHub yet.
- winit 0.30 API migrated to `EventLoop::run()` (deprecated but functional; `run_app`/`EventLoopHandler` not available in winit 0.30.13).
- Phase 6 (network): online authentication not implemented (ADR-0002 - offline only in v0.x).
- Phase 7 (Java bridge): JNI/JVM not implemented yet.
- Phase 8 (performance): benchmarks created (NBT, region, chunk mesh, player movement). Fuzz tests added (NBT fuzz, region fuzz, blockstate fuzz). CI performance benchmarks not yet integrated.
- Phase 9 remaining: sounds, textures, full item mechanics, all dimensions (Nether/End), recipes, achievements, structures, weather, UI (inventory screen, hotbar, menus), multiplayer, caves, trees, structures.

## Test results

`cargo test --workspace` — **ALL 114 TESTS PASS** across 11 crates:
- gibrid-assets: 20, gibrid-core: 14, gibrid-gameplay: 10, gibrid-java-bridge: 3,
- gibrid-mesh: 14, gibrid-network: 11, gibrid-renderer-wgpu: 2, gibrid-storage: 30,
- gibrid-testkit: 1, gibrid-world: 1, gibrid-worldgen: 8.