#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use gibrid_core::{AppConfig, Lifecycle, VersionManifest};
use gibrid_gameplay::{
    block_interaction::{break_block, place_block, raycast},
    input::InputHandler,
    movement::apply_movement,
    Player,
    WorldContext,
};
use gibrid_mesh::mesh_queue::MeshQueue;
use gibrid_renderer_wgpu::Renderer;
use tracing::{error, info, warn};
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("gibrid: {err}");
            ExitCode::from(1)
        }
    }
}

#[allow(deprecated)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    tracing_subscriber::fmt()
        .with_env_filter(config.log_level.clone())
        .with_target(false)
        .init();

    let mut life = Lifecycle::Created;
    life = life
        .transition(Lifecycle::Initializing)
        .expect("created -> init");

    let (manifest_path, manifest) = load_manifest()?;
    init_logging_fields(&config, &manifest_path, &manifest);

    life = life
        .transition(Lifecycle::Ready)
        .and_then(|s| s.transition(Lifecycle::Running))
        .map_err(|_| "illegal lifecycle transition")?;

    let event_loop = EventLoop::new().map_err(|e| format!("failed to create event loop: {e}"))?;
    let window_attributes = winit::window::Window::default_attributes()
        .with_title("GIBRID Native — Phase 5")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
    let window = event_loop
        .create_window(window_attributes)
        .expect("failed to create window");

    let window_id = window.id();
    let mut renderer = Renderer::new(window).expect("failed to create renderer");

    info!(
        subsystem = "renderer",
        event_id = "renderer_initialized",
        width = renderer.size().width,
        height = renderer.size().height,
        "wgpu renderer phase 5"
    );

    // Игровой мир: игрок, вход, очередь мешинга, мир
    let mut player = Player::new(0.0, 64.0, 0.0);
    let mut input = InputHandler::new();
    let mesh_queue = MeshQueue::new(16, 16);
    let mut world = WorldContext::new(mesh_queue);

    // Ставим несколько блоков для теста
    let stone = gibrid_core::ResourceId::minecraft("stone").unwrap();
    world.set_block(0, 63, 0, Some(stone.clone()));
    world.set_block(1, 63, 0, Some(stone.clone()));
    world.set_block(0, 63, 1, Some(stone.clone()));

    info!(
        subsystem = "gameplay",
        event_id = "world_initialized",
        "gameplay phase 5"
    );

    let mut last_instant = Instant::now();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent {
                ref event,
                window_id: id,
            } if id == window_id => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state,
                            physical_key: PhysicalKey::Code(code),
                            ..
                        },
                    ..
                } => {
                    input.handle_event(event, &mut player);
                    if *state == ElementState::Pressed && *code == KeyCode::Escape {
                        elwt.exit();
                    }
                }
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(*physical_size);
                }
                WindowEvent::CursorMoved { .. } => {
                    input.handle_event(event, &mut player);
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    input.handle_event(event, &mut player);

                    if *state == ElementState::Pressed {
                        if let Some(hit) = raycast(&player, &world, 6.0) {
                            match button {
                                winit::event::MouseButton::Left => {
                                    break_block(&mut world, &hit);
                                }
                                winit::event::MouseButton::Right => {
                                    place_block(
                                        &mut world,
                                        &hit,
                                        gibrid_core::ResourceId::minecraft("stone").unwrap(),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                // Кадр
                let now = Instant::now();
                let dt = now.duration_since(last_instant).as_secs_f32();
                last_instant = now;
                let dt = dt.clamp(0.0, 0.1);

                // Обновляем движение игрока
                apply_movement(&mut player, &input, &world, dt);

                // Загружаем готовые меши из очереди в GPU
                while let Some(result) = world.try_recv_mesh() {
                    renderer.upload_chunk(&result);
                }

                if let Err(err) = renderer.render() {
                    match err {
                        wgpu::SurfaceError::Lost => {
                            let size = renderer.size();
                            renderer.resize(size);
                        }
                        wgpu::SurfaceError::OutOfMemory => elwt.exit(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    })?;

    life = life
        .transition(Lifecycle::ShuttingDown)
        .and_then(|s| s.transition(Lifecycle::Stopped))
        .map_err(|_| "illegal lifecycle transition")?;

    info!(
        subsystem = "app",
        event_id = "shutdown",
        lifecycle = ?life,
        "clean exit"
    );
    Ok(())
}

fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    if let Ok(path) = env::var("GIBRID_CONFIG") {
        let text = std::fs::read_to_string(path)?;
        return Ok(AppConfig::parse(&text)?);
    }
    Ok(AppConfig::default())
}

fn load_manifest() -> Result<(PathBuf, VersionManifest), Box<dyn std::error::Error>> {
    if let Ok(path) = env::var("GIBRID_VERSION_FILE") {
        let path = PathBuf::from(path);
        let manifest = VersionManifest::load_path(&path)?;
        return Ok((path, manifest));
    }
    let cwd = env::current_dir()?;
    Ok(VersionManifest::load_from_dir(cwd)?)
}

fn init_logging_fields(
    config: &AppConfig,
    manifest_path: &std::path::Path,
    manifest: &VersionManifest,
) {
    info!(
        subsystem = "app",
        event_id = "startup",
        crate_version = env!("CARGO_PKG_VERSION"),
        game_version = manifest.game_version.as_str(),
        protocol_version = manifest.protocol_version,
        protocol_verified = manifest.protocol_verified,
        data_version = ?manifest.data_version,
        confidence = manifest.confidence.as_str(),
        schema_version = config.schema_version,
        manifest = %manifest_path.display(),
        "gibrid native phase 5"
    );
    for warning in manifest.warnings() {
        warn!(
            subsystem = "version",
            event_id = "unverified_manifest",
            "{warning}"
        );
    }
    if let Err(err) = config.validate() {
        error!(subsystem = "config", event_id = "invalid_config", "{err}");
    }
}
